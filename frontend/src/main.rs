#![allow(non_snake_case)]
use dioxus::prelude::*;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use log::{error, info};
use uuid::Uuid;
use common::*;
use host::*;
use std::fmt;

mod host;

static CSS: Asset = asset!("/assets/main.css");


#[derive(Clone, Copy)]
struct AppContext {
    ws_tx: Signal<Option<SplitSink<WebSocket, Message>>>,
    game_state: Signal<Option<GameState>>,
    player_id: Signal<Option<Uuid>>,
    game_code: Signal<Option<String>>,
    error_message: Signal<Option<String>>,
}

impl fmt::Debug for AppContext {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppContext")
         .field("game_state", &self.game_state)
         .field("game_code", &self.game_code)
         .field("player_id", &self.player_id)
         .finish()
    }
}

impl AppContext {
    // Helper to send a message to the server
    fn send(&self, msg: ClientToServer) {
        let mut ws_tx_signal = self.ws_tx;
        spawn(async move {
            let json_msg = serde_json::to_string(&msg).unwrap();
            // 1. Lock the signal and TAKE the sender, leaving `None` behind.
            //    This gives this task full ownership of the sender.
            let sender = ws_tx_signal.write().take();

            if let Some(mut sender) = sender {
                // 2. The sender was available. Send the message.
                if sender.send(Message::Text(json_msg)).await.is_ok() {
                    // 3. If sending succeeded, put the sender back for the next message.
                    *ws_tx_signal.write() = Some(sender);
                } else {
                    // 4. If sending failed, the connection is dead. Don't put it back.
                    error!("WebSocket send failed. Connection is likely closed.");
                }
            } else {
                // This can happen if another message is already being sent.
                // In a real app, you might want a queue here, but for this
                // simple case, logging an error is fine.
                error!("WebSocket sender was not available (already in use).");
            }
        });
    }
}


fn main() {
    wasm_logger::init(wasm_logger::Config::default());
    launch(App);
}

#[component]
fn App() -> Element {
    // Centralized state management using signals
    let ws_tx = use_signal::<Option<SplitSink<WebSocket, Message>>>(|| None);
    let game_state = use_signal::<Option<GameState>>(|| None);
    let player_id = use_signal::<Option<Uuid>>(|| None);
    let game_code = use_signal::<Option<String>>(|| None);
    let error_message = use_signal::<Option<String>>(|| None);

    // Provide the context to all child components
    use_context_provider(|| AppContext {
        ws_tx,
        game_state,
        player_id,
        game_code,
        error_message,
    });

    rsx! {
        document::Stylesheet { href: CSS }
        Router::<Route> {}
    }
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/game/:code")]
    GameRoom { code: String },
    // PageNotFound is a catch all route that will match any route and placing the matched segments in the route field
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
fn Home() -> Element {
    let mut app_ctx = use_context::<AppContext>();
    let mut name = use_signal(String::new);
    let mut join_code = use_signal(|| String::new());
    let nav = navigator();

    // Effect to establish and manage WebSocket connection
    use_effect(move || {
        spawn(async move {
            let ws = WebSocket::open("ws://127.0.0.1:3001/ws").expect("Failed to open WebSocket");
            info!("WebSocket connection opened");
            let (tx, mut rx) = ws.split();
            *app_ctx.ws_tx.write() = Some(tx);
            // This loop listens for messages from the server
            while let Some(Ok(Message::Text(text))) = rx.next().await {
                match serde_json::from_str::<ServerToClient>(&text) {
                    Ok(msg) => {
                        info!("Received message: {:?}", msg);
                        // Clear previous error on new message
                        *app_ctx.error_message.write() = None;

                        match msg {
                            ServerToClient::GameCreated { game_code: code, player_id: id, game_state: state } => {
                                *app_ctx.game_code.write() = Some(code.clone());
                                *app_ctx.player_id.write() = Some(id);
                                *app_ctx.game_state.write() = Some(state.into());

                                nav.push(Route::GameRoom { code });
                            }
                            ServerToClient::GameJoined { player_id: id, game_state: state } => {
                                info!("ServerToClient: game joined");
                                *app_ctx.player_id.write() = Some(id);
                                *app_ctx.game_state.write() = Some(state.into());
                                if let Some(code) = app_ctx.game_code.read().clone() {
                                    info!("Navigate to GameRoom");
                                    nav.push(Route::GameRoom { code });
                                }
                            }
                             ServerToClient::GameStateUpdate { game_state: state } => {
                                *app_ctx.game_state.write() = Some(state.into());
                            }
                            ServerToClient::Error { message } => {
                                *app_ctx.error_message.write() = Some(message);
                            }
                        }
                    }
                    Err(e) => error!("Failed to parse S2C message: {}", e),
                }
            }
            info!("WebSocket connection closed");
        });
    });

    let on_create_game = move |_| {
        info!("Creating game");
        app_ctx.send(ClientToServer::CreateGame);
    };

    let mut app_ctx = use_context::<AppContext>();
    let on_join_game = move |_| {
        let code = join_code();
        info!("Joining game with code: {code}");
        if !name().is_empty() && !code.is_empty() {
            info!("Send info to server");
            app_ctx.send(ClientToServer::JoinGame {
                game_code: join_code(),
                player_name: name(),
            });
            info!("Write game code");
            *app_ctx.game_code.write() = Some(code.clone());
            // nav.push(Route::GameRoom { code });
        }
    };

    let app_ctx = use_context::<AppContext>();
    rsx! {
        h1 { "Quiz Button" }
        if let Some(err) = (app_ctx.error_message)() {
            p { class: "error", "{err}" }
        }
        div {
            h2 { "Create a New Game" }
            button { onclick: on_create_game, "Create Game" }
        }

        hr{}

        div {
            h1 { "Join game" }

            // Input for game code
            h3 {"Game code"}
            input {
                placeholder: "Game code",
                value: "{join_code}",
                oninput: move |evt| join_code.set(evt.value())
            }

            // Input for player name
            h3 {"Your name"}
            input {
                placeholder: "Player's name",
                value: "{name}",
                oninput: move |evt| name.set(evt.value())
            }

            // Wrap the button in its own div to ensure it's on a new line
            div {
                margin_top: "10px", // Optional: Adds some space above the button
                button { 
                    onclick: on_join_game, 
                    "Join Game" 
                }
            }
        }
    }
}

#[component]
fn PageNotFound(route: Vec<String>) -> Element {
    rsx! {
        h1 { "Page not found" }
        p { "We are terribly sorry, but the page you requested doesn't exist." }
        pre { color: "red", "log:\nattemped to navigate to: {route:?}" }
    }
}
