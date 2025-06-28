#![allow(non_snake_case)]
use dioxus::prelude::*;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use log::{error, info};
use uuid::Uuid;
use common::*;


#[derive(Clone, Copy)]
struct AppContext {
    ws_tx: Signal<Option<SplitSink<WebSocket, Message>>>,
    game_state: Signal<Option<GameState>>,
    player_id: Signal<Option<Uuid>>,
    game_code: Signal<Option<String>>,
    error_message: Signal<Option<String>>,
}

impl AppContext {
    // Helper to send a message to the server
    fn send(&self, msg: C2S) {
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
    let mut ws_tx = use_signal::<Option<SplitSink<WebSocket, Message>>>(|| None);
    let mut game_state = use_signal::<Option<GameState>>(|| None);
    let mut player_id = use_signal::<Option<Uuid>>(|| None);
    let mut game_code = use_signal::<Option<String>>(|| None);
    let mut error_message = use_signal::<Option<String>>(|| None);

    // Provide the context to all child components
    use_context_provider(|| AppContext {
        ws_tx,
        game_state,
        player_id,
        game_code,
        error_message,
    });
    
    // Effect to establish and manage WebSocket connection
    use_effect(move || {
        spawn(async move {
            let ws = WebSocket::open("ws://127.0.0.1:3001/ws").expect("Failed to open WebSocket");
            info!("WebSocket connection opened");
            let (tx, mut rx) = ws.split();
            *ws_tx.write() = Some(tx);
            
            // This loop listens for messages from the server
            while let Some(Ok(Message::Text(text))) = rx.next().await {
                match serde_json::from_str::<S2C>(&text) {
                    Ok(msg) => {
                        info!("Received message: {:?}", msg);
                        // Clear previous error on new message
                        *error_message.write() = None;

                        match msg {
                            S2C::GameCreated { game_code: code, player_id: id, game_state: state } => {
                                *game_code.write() = Some(code.clone());
                                *player_id.write() = Some(id);
                                *game_state.write() = Some(state.into());

                                let route: NavigationTarget::<Route> = format!("/game/{}", code).into();
                                dioxus_router::router().push(route);
                            }
                            S2C::GameJoined { player_id: id, game_state: state } => {
                                *player_id.write() = Some(id);
                                *game_state.write() = Some(state.into());
                            }
                             S2C::GameStateUpdate { game_state: state } => {
                                *game_state.write() = Some(state.into());
                            }
                            S2C::Error { message } => {
                                *error_message.write() = Some(message);
                            }
                        }
                    }
                    Err(e) => error!("Failed to parse S2C message: {}", e),
                }
            }
            info!("WebSocket connection closed");
        });
    });

    rsx! {
        Router::<Route> {}
    }
}

#[derive(Routable, Clone)]
#[rustfmt::skip]
enum Route {
    #[route("/")]
    Home {},
    #[route("/game/:code")]
    GameRoom { code: String },
}

#[component]
fn Home() -> Element {
    let mut app_ctx = use_context::<AppContext>();
    let mut name = use_signal(String::new);
    let mut join_code = use_signal(|| String::new());

    let on_create_game = move |_| {
        if !name().is_empty() {
            app_ctx.send(C2S::CreateGame { host_name: name() });
        }
    };

    let mut app_ctx = use_context::<AppContext>();
    let on_join_game = move |_| {
        if !name().is_empty() && !join_code().is_empty() {
            app_ctx.send(C2S::JoinGame {
                game_code: join_code(),
                player_name: name(),
            });
            let code = join_code();
            *app_ctx.game_code.write() = Some(code.clone());
            dioxus_router::router().push(NavigationTarget::Internal( Route::GameRoom { code }));
        }
    };

    let app_ctx = use_context::<AppContext>();
    rsx! {
        h1 { "Dioxus Buzz-in" }
        if let Some(err) = (app_ctx.error_message)() {
            p { class: "error", "{err}" }
        }
        
        div {
            h2 { "Your Name" }
            input {
                placeholder: "Enter your name",
                value: "{name}",
                oninput: move |evt| name.set(evt.value())
            }
        }
        
        div {
            h2 { "Create a New Game" }
            button { onclick: on_create_game, "Create Game" }
        }

        hr{}

        div {
            h2 { "Join an Existing Game" }
            input {
                placeholder: "Enter game code",
                value: "{join_code}",
                oninput: move |evt| join_code.set(evt.value())
            }
            button { onclick: on_join_game, "Join Game" }
        }
    }
}

#[component]
fn GameRoom(code: String) -> Element {
    let app_ctx = use_context::<AppContext>();
    let my_id = app_ctx.player_id.read();
    
    // If we land here but have no state, redirect home
    if app_ctx.game_state.read().is_none() {
        return rsx! {
            p { "Joining game..."}
        };
    }

    let game = (app_ctx.game_state).unwrap();
    let is_host = *my_id == Some(game.host_id);

    rsx! {
        p { class: "game-info", "Game Code: {code}" }
        if is_host {
            HostView {}
        } else {
            PlayerView {}
        }
        PlayerList {}
    }
}

#[component]
fn HostView() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game_code = app_ctx.game_code.read().clone().unwrap();
    let game = app_ctx.game_state.read().clone().unwrap();

    let on_lock_ctx = app_ctx.clone();
    let on_lock_code = game_code.clone();
    let on_lock = move |_| on_lock_ctx.send(C2S::Lock { game_code: on_lock_code.clone() });

    let on_unlock_ctx = app_ctx.clone();
    let on_unlock_code = game_code.clone();
    let on_unlock = move |_| {
        on_unlock_ctx.send(C2S::Unlock { game_code: on_unlock_code.clone() });
    };

    let on_clear_ctx = app_ctx.clone();
    let on_clear_code = game_code.clone();
    let on_clear = move |_| {
        on_clear_ctx.send(C2S::Clear { game_code: on_clear_code.clone() });
    };

    rsx! {
        h2 { "Host Controls" }
        div {
            if game.locked {
                button { onclick: on_unlock, "Unlock Buzzers" }
            } else {
                button { onclick: on_lock, "Lock Buzzers" }
            }
            button { onclick: on_clear, "Clear Buzzer" }
        }
    }
}

#[component]
fn PlayerView() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game_code = app_ctx.game_code.read().clone().unwrap();
    let my_id = app_ctx.player_id.read().unwrap();
    let game = app_ctx.game_state.read().clone().unwrap();

    // let i_buzzed = game.buzzer_order == Some(my_id);
    let someone_buzzed = !game.buzzer_order.is_empty();
    let is_locked = game.locked;

    let on_buzz = move |_| {
        app_ctx.send(C2S::Buzz {
            game_code: game_code.clone(),
            player_id: my_id,
        })
    };

    let buzzer_text = if someone_buzzed {
        let mut buzz_order = Vec::new();
        for id in game.buzzer_order {
            buzz_order.push(game.players.get(&id).unwrap())
        }
        // let winner_name = game.players.get(&winner_id).map(|p| p.name.as_str()).unwrap_or("Someone");
        format!("{:?} Buzzed!", buzz_order)
    } else if is_locked {
        "Locked".to_string()
    } else {
        "BUZZ!".to_string()
    };

    rsx! {
        button {
            class: "buzzer",
            disabled: is_locked || someone_buzzed,
            onclick: on_buzz,
            "{buzzer_text}"
        }
    }
}

#[component]
fn PlayerList() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game = app_ctx.game_state.read().clone().unwrap();
    // let winner_id = game.buzzer_order;

    rsx! {
        h3 { "Players ({game.players.len()})" }
        ul { class: "player-list",
            for (id, player) in game.players.iter() {
                li {
                    // class: if Some(*id) == winner_id { "winner" } else { "" },
                    "{player.name}"
                }
            }
        }
    }
}
