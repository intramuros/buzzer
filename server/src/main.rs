use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use tower_http::services::ServeDir;
use common::*;
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use std::{
    collections::{HashMap, VecDeque},
    env,
    net::SocketAddr,
    sync::Arc,
};
use tokio::sync::mpsc;
use tower_http::trace::TraceLayer;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::DefaultMakeSpan,
};
use tracing::{info, warn};
use uuid::Uuid;

// Holds all game states and player connections
#[derive(Default)]
struct AppState {
    games: DashMap<usize, GameState>,
    // Maps a player's unique ID to their WebSocket sender
    connections: DashMap<Uuid, mpsc::UnboundedSender<Message>>,
}

type SharedState = Arc<AppState>;

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let state = SharedState::new(AppState::default());

    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any);

    let app = Router::new()
        .route("/ws", get(ws_handler))
        .fallback_service(ServeDir::new("./dist"))
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(cors);

    let host = std::env::var("HOST").unwrap_or_else(|_| "127.0.0.1".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "3001".to_string());
    let addr_str = format!("{}:{}", host, port);
    let addr: SocketAddr = addr_str.parse().expect("Invalid address format");

    info!("Server listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn ws_handler(ws: WebSocketUpgrade, State(state): State<SharedState>) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: SharedState) {
    let player_id = Uuid::new_v4();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    let (tx, mut rx) = mpsc::unbounded_channel();
    state.connections.insert(player_id, tx);

    // This task forwards messages from our application logic to the actual WebSocket
    let mut send_task = tokio::spawn(async move {
        while let Some(msg) = rx.recv().await {
            if ws_sender.send(msg).await.is_err() {
                break;
            }
        }
    });

    // --- Heartbeat and Message Receiving Task ---
    let recv_state = state.clone();
    let mut recv_task = tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            tokio::select! {
                // Handle incoming messages from the client
                Some(Ok(msg)) = ws_receiver.next() => {
                    if let Message::Text(text) = msg {
                        match serde_json::from_str::<ClientToServer>(&text) {
                            Ok(c2s_msg) => handle_c2s_message(c2s_msg, player_id, recv_state.clone()).await,
                            Err(e) => warn!("Failed to parse C2S message: {}", e),
                        }
                    } else if let Message::Close(_) = msg {
                        // Client sent a close frame
                        break;
                    }
                },
                // Send a ping message on a fixed interval
                _ = interval.tick() => {
                    let sender = recv_state.connections.get(&player_id);
                    if let Some(sender) = sender {
                        if sender.send(Message::Ping(vec![].into())).is_err() {
                            // If sending fails, the connection is likely closed
                            break;
                        }
                    } else {
                        break;
                    }
                }
            }
        }
    });

    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("Player {} disconnected", player_id);
    state.connections.remove(&player_id);

    let mut game_to_cleanup: Option<usize> = None;
    let mut game_to_update: Option<usize> = None;

    if let Some(game_ref) = state.games.iter().find(|g| g.players.contains_key(&player_id)) {
        let game_code = *game_ref.key();
        if let Some(mut game) = state.games.get_mut(&game_code) {
            // Remove the player
            game.players.remove(&player_id);
            game.player_join_order.retain(|id| id != &player_id);
            info!("Removed player {} from game {}", player_id, game_code);

            // Decide whether to clean up the game or just update it
            if game.host_id == player_id || game.players.is_empty() {
                game_to_cleanup = Some(game_code);
            } else {
                game_to_update = Some(game_code);
            }
        }
    }

    if let Some(game_code) = game_to_cleanup {
        info!("Game {} is empty or host left, removing.", game_code);
        state.games.remove(&game_code);
    } else if let Some(game_code) = game_to_update {
        if let Some(game) = state.games.get(&game_code) {
            broadcast_state_update(&game, &state).await;
        }
    }
}

async fn handle_c2s_message(msg: ClientToServer, sender_id: Uuid, state: SharedState) {
    match msg {
        ClientToServer::CreateGame => {
            let game_code = generate_game_code(&state);

            // Create a map and add the host to it immediately
            let players = DashMap::new();
            players.insert(sender_id, Actor::Host { id: sender_id });

            let game_state = GameState {
                host_id: sender_id,
                globally_locked: false,
                buzzer_order: VecDeque::new(),
                players,
                scores: HashMap::new(),
                player_join_order: vec![sender_id],
            };

            info!("Game created: {} by player {}", game_code, sender_id);

            let response = ServerToClient::GameCreated {
                game_code: game_code.clone(),
                player_id: sender_id,
                game_state: game_state.to_json(),
            };
            state.games.insert(game_code, game_state.clone());
            send_to_player(sender_id, &response, &state).await;
        }
        ClientToServer::JoinGame {
            game_code,
            mut player_name,
        } => {
            // Trim whitespace from the name
            player_name = player_name.trim().to_string();

            if player_name.is_empty() {
                let error_msg = ServerToClient::Error {
                    message: "Player name cannot be empty.".to_string(),
                };
                send_to_player(sender_id, &error_msg, &state).await;
                return;
            }


            if let Some(mut game) = state.games.get_mut(&game_code) {
                // --- Check for duplicate names ---
                let name_exists = game.players.iter().any(|p| p.name() == player_name);
                if name_exists {
                    let error_msg = ServerToClient::Error {
                        message: format!("Player name '{}' is already taken.", player_name),
                    };
                    send_to_player(sender_id, &error_msg, &state).await;
                    return;
                }

                game.players.insert(
                    sender_id,
                    Actor::Player {
                        id: sender_id,
                        name: player_name.clone(),
                    },
                );
                game.scores.insert(sender_id, 0);
                game.player_join_order.push(sender_id);
                let response = ServerToClient::GameJoined {
                    player_id: sender_id,
                    player_name,
                    game_state: game.to_json(),
                };
                send_to_player(sender_id, &response, &state).await;
                broadcast_state_update(&game, &state).await;
            } else {
                let err = ServerToClient::Error {
                    message: format!("Game '{}' not found.", game_code),
                };
                send_to_player(sender_id, &err, &state).await;
            }
        }
        ClientToServer::Buzz {
            game_code,
            player_id,
        } => {
            if let Some(mut game) = state.games.get_mut(&game_code) {
                if !game.globally_locked {
                    let Some(player_name) =
                        game.players.get(&player_id).map(|p| p.name().to_owned())
                    else {
                        return;
                    };
                    info!("Player {} buzzed in game {}", player_name, game_code);
                    game.buzzer_order
                        .push_back((player_id, player_name.clone()));
                    let buzz_msg = ServerToClient::PlayerBuzzed {
                        player_id,
                        player_name,
                    };
                    // Use your existing `send_to_player` helper to target the host
                    send_to_player(game.host_id, &buzz_msg, &state).await;
                    broadcast_state_update(&game, &state).await;
                }
            }
        }
        ClientToServer::Lock { ref game_code } | ClientToServer::Unlock { ref game_code } => {
            if let Some(mut game) = state.games.get_mut(game_code) {
                if game.host_id == sender_id {
                    // Only host can lock/unlock
                    game.globally_locked = matches!(msg, ClientToServer::Lock { .. });
                    broadcast_state_update(&game, &state).await;
                }
            }
        }
        ClientToServer::Clear { game_code } => {
            if let Some(mut game) = state.games.get_mut(&game_code) {
                if game.host_id == sender_id {
                    // Only host can clear
                    game.buzzer_order = VecDeque::new();
                    broadcast_state_update(&game, &state).await;
                }
            }
        }
        ClientToServer::UpdateScore {
            game_code,
            player_id,
            delta,
        } => {
            if let Some(mut game) = state.games.get_mut(&game_code) {
                if game.host_id == sender_id {
                    let score = game.scores.entry(player_id).or_insert(0);
                    *score += delta; 
                    info!(
                        "Host {} updated score for player {} to {}",
                        sender_id, player_id, *score
                    );
                    broadcast_state_update(&game, &state).await;
                }
            }
        }
    }
}

/// Helper to serialize a message and send it to a single player
async fn send_to_player(player_id: Uuid, message: &ServerToClient, state: &SharedState) {
    if let Some(tx) = state.connections.get(&player_id) {
        let json_msg = serde_json::to_string(message).unwrap();
        if tx.send(Message::Text(json_msg.into())).is_err() {
            warn!("Failed to send message to player {}", player_id);
        }
    }
}

/// Helper to broadcast the current game state to all players in a game
async fn broadcast_state_update(game: &GameState, state: &SharedState) {
    let update_msg = ServerToClient::GameStateUpdate {
        game_state: game.to_json(),
    };
    for player_ref in game.players.iter() {
        let player_id = player_ref.id();
        send_to_player(player_id, &update_msg, state).await;
    }
}

fn generate_game_code(state: &SharedState) -> usize {
    loop {
        let mut rng = rand::rng();
        let code = rng.random_range(10000..99999);
        if !state.games.contains_key(&code) {
            return code;
        }
    }
}
