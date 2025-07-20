use axum::{
    extract::{
        ws::{Message, WebSocket},
        State, WebSocketUpgrade,
    },
    response::Response,
    routing::get,
    Router,
};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use rand::Rng;
use std::{collections::{HashMap, VecDeque}, net::SocketAddr, sync::Arc};
use tokio::sync::mpsc;
use tower_http::{cors::{Any, CorsLayer}, trace::DefaultMakeSpan};
use tower_http::trace::TraceLayer;
use tracing::{info, warn};
use uuid::Uuid;
use common::*;


// Holds all game states and player connections
#[derive(Default)]
struct AppState {
    games: DashMap<String, GameState>,
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
        .with_state(state)
        .layer(
            TraceLayer::new_for_http()
                .make_span_with(DefaultMakeSpan::default().include_headers(true)),
        )
        .layer(cors);

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));
    info!("Server listening on {}", addr);
    axum::serve(tokio::net::TcpListener::bind(addr).await.unwrap(), app)
        .await
        .unwrap();
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<SharedState>,
) -> Response {
    ws.on_upgrade(|socket| handle_socket(socket, state))
}

async fn handle_socket(socket: WebSocket, state: SharedState) {
    let player_id = Uuid::new_v4();
    let (mut ws_sender, mut ws_receiver) = socket.split();

    // Create a channel to send messages to this specific client's WebSocket
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

    let recv_state = state.clone();

    // This task handles incoming messages from the WebSocket client
    let mut recv_task = tokio::spawn(async move {
        while let Some(Ok(Message::Text(text))) = ws_receiver.next().await {
            match serde_json::from_str::<ClientToServer>(&text) {
                // Use the cloned state here
                Ok(msg) => handle_c2s_message(msg, player_id, recv_state.clone()).await,
                Err(e) => warn!("Failed to parse C2S message: {}", e),
            }
        }
    });

    // Wait for either task to finish (which means the connection is closed)
    tokio::select! {
        _ = (&mut send_task) => recv_task.abort(),
        _ = (&mut recv_task) => send_task.abort(),
    };

    info!("Player {} disconnected", player_id);
    // Cleanup: remove player connection and from any game they were in
    // This `state` is now valid because we only moved the clone.
    state.connections.remove(&player_id);
    for mut game in state.games.iter_mut() {
        if game.players.remove(&player_id).is_some() {
            info!("Removed player {} from game {}", player_id, game.key());
            broadcast_state_update(game.key(), &state).await;
            break;
        }
    }
}

async fn handle_c2s_message(msg: ClientToServer, player_id: Uuid, state: SharedState) {
    match msg {
        ClientToServer::CreateGame  => {
            let game_code = generate_game_code(&state);

            // Create a map and add the host to it immediately
            let mut players = HashMap::new();
            // The `ClientToServer` protocol should ideally be changed to include the host's name
            // For now, we'll use a placeholder name.
            players.insert(player_id, Player { name: "Host".to_string() });

            let game_state = GameState {
                host_id: player_id,
                locked: true,
                buzzer_order: VecDeque::new(),
                players,
            };

            info!("Game created: {} by player {}", game_code, player_id);

            let response = ServerToClient::GameCreated {
                game_code: game_code.clone(),
                player_id,
                game_state: game_state.to_json(),
            };
            state.games.insert(game_code, game_state.clone());
            send_to_player(player_id, &response, &state).await;
        }
        ClientToServer::JoinGame { game_code, player_name } => {
            if let Some(mut game) = state.games.get_mut(&game_code) {
                info!("Insert player {}", player_id);
                game.players.insert(player_id, Player { name: player_name });
                let response = ServerToClient::GameJoined { player_id, game_state: game.to_json() };
                info!("Response to player {}", player_id);
                send_to_player(player_id, &response, &state).await;
                info!("Broadcast state update, game code {}", game_code);
                broadcast_state_update(&game_code, &state).await;
            } else {
                let err = ServerToClient::Error { message: format!("Game '{}' not found.", game_code) };
                send_to_player(player_id, &err, &state).await;
            }
        }
        ClientToServer::Buzz { game_code, player_id } => {
            if let Some(mut game) = state.games.get_mut(&game_code) {
                if !game.locked {
                    info!("Player {} buzzed in game {}", player_id, game_code);
                    game.buzzer_order.push_back(player_id);
                    broadcast_state_update(&game_code, &state).await;
                }
            }
        }
        ClientToServer::Lock { ref game_code } | ClientToServer::Unlock { ref game_code } => {
            if let Some(mut game) = state.games.get_mut(game_code) {
                if game.host_id == player_id { // Only host can lock/unlock
                    game.locked = matches!(msg, ClientToServer::Lock { .. });
                    broadcast_state_update(&game_code, &state).await;
                }
            }
        }
        ClientToServer::Clear { game_code } => {
             if let Some(mut game) = state.games.get_mut(&game_code) {
                if game.host_id == player_id { // Only host can clear
                    game.buzzer_order = VecDeque::new();
                    broadcast_state_update(&game_code, &state).await;
                }
            }
        }
    }
}

/// Helper to serialize a message and send it to a single player
async fn send_to_player(player_id: Uuid, message: &ServerToClient, state: &SharedState) {
    if let Some(tx) = state.connections.get(&player_id) {
        let json_msg = serde_json::to_string(message).unwrap();
        info!("Sending message");
        if tx.send(Message::Text(json_msg)).is_err() {
            warn!("Failed to send message to player {}", player_id);
        }
    }
}

/// Helper to broadcast the current game state to all players in a game
async fn broadcast_state_update(game_code: &str, state: &SharedState) {
    if let Some(game) = state.games.get(game_code) {
        info!("Broadcast game: {}", game_code);
        let update_msg = ServerToClient::GameStateUpdate { game_state: game.to_json() };
        for player_ref in game.players.iter() {
            let player_id = *player_ref.0;
            info!("Send message to player {}", player_id);
            send_to_player(player_id, &update_msg, state).await;
        }
    }
}

fn generate_game_code(state: &SharedState) -> String {
    loop {
        let mut rng = rand::rng();
        let code = rng.random_range(10000..99999).to_string();
        if !state.games.contains_key(&code) {
            return code;
        }
    }
}
