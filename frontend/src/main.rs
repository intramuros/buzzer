#![allow(non_snake_case)]
use common::*;
use dioxus::prelude::*;
use futures_util::{stream::SplitSink, SinkExt, StreamExt};
use gloo_net::websocket::{futures::WebSocket, Message};
use host::*;
use log::{error, info};
use player::PlayerView;
use std::fmt;
use uuid::Uuid;
use web_sys::HtmlAudioElement;

mod host;
mod player;
mod timer;

static CSS: Asset = asset!("/assets/main.css");
const SOUND_OPTIONS: [(&'static str, &'static str); 4] = [
    ("Default Buzzer", "../assets/sounds/ding-101492.mp3"),
    ("New notification", "../assets/sounds/new-notification-09-352705.mp3"),
    ("Pop", "../assets/sounds/pop-39222.mp3"),
    ("Bubble pop", "../assets/sounds/bubble-pop-389501.mp3"),
];
const FAVICON: Asset = asset!("/assets/favicon.svg");

#[derive(Clone, Copy)]
struct AppContext {
    ws_tx: Signal<Option<SplitSink<WebSocket, Message>>>,
    game_state: Signal<Option<GameState>>,
    player_id: Signal<Option<Uuid>>,
    player_name: Signal<Option<String>>,
    game_code: Signal<Option<usize>>,
    error_message: Signal<Option<String>>,
    locally_locked: Signal<bool>,
    buzzer_sound: Signal<String>,
    is_host: Signal<bool>,
    time_limit: Signal<Option<u32>>,
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
    fn send(&self, msg: ClientToServer) {
        let mut ws_tx_signal = self.ws_tx;
        spawn(async move {
            let json_msg = serde_json::to_string(&msg).unwrap();
            // 1. Lock the signal and TAKE the sender, leaving `None` behind.
            //    This gives this task full ownership of the sender.
            let sender = ws_tx_signal.write().take();

            if let Some(mut sender) = sender {
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
    rsx! {
        Router::<Route> {}
    }
}

#[component]
fn AppLayout() -> Element {
    // Centralized state management using signals
    let ws_tx = use_signal::<Option<SplitSink<WebSocket, Message>>>(|| None);
    let game_state = use_signal::<Option<GameState>>(|| None);
    let player_id = use_signal::<Option<Uuid>>(|| None);
    let player_name = use_signal::<Option<String>>(|| None);
    let game_code = use_signal::<Option<usize>>(|| None);
    let error_message = use_signal::<Option<String>>(|| None);
    let locally_locked = use_signal::<bool>(|| false);
    let buzzer_sound = use_signal(|| "../assets/ding-101492.mp3".to_string());
    let is_host = use_signal(|| false);
    let time_limit = use_signal::<Option<u32>>(|| Some(5));

    // Provide the context to all child components
    let mut app_ctx = use_context_provider(|| AppContext {
        ws_tx,
        game_state,
        player_id,
        player_name,
        game_code,
        error_message,
        locally_locked,
        buzzer_sound,
        is_host,
        time_limit,
    });

    let nav = navigator();
    // Effect to establish and manage WebSocket connection
    use_effect(move || {
        spawn(async move {
            let ws_url = {
                #[cfg(feature = "dev")]
                {
                    "ws://localhost:3001/ws"
                }
                #[cfg(not(feature = "dev"))]
                {
                    let window = web_sys::window().expect("no global `window` exists");
                    let location = window.location();
                    let host = location.host().expect("should have a host");
                    let protocol = location.protocol().expect("should have a protocol");
                    let ws_protocol = if protocol == "https:" { "wss:" } else { "ws:" };
                    format!("{}//{}/ws", ws_protocol, host)
                }
            };
            let ws = WebSocket::open(&ws_url).expect("Failed to open WebSocket");
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
                            ServerToClient::GameCreated {
                                game_code: code,
                                player_id: id,
                                game_state: state,
                            } => {
                                *app_ctx.game_code.write() = Some(code.clone());
                                *app_ctx.player_id.write() = Some(id);
                                *app_ctx.game_state.write() = Some(state.into());

                                nav.push(Route::GameRoom { code });
                            }
                            ServerToClient::GameJoined {
                                player_id: id,
                                player_name,
                                game_state: state,
                            } => {
                                info!("ServerToClient: game joined");
                                *app_ctx.player_id.write() = Some(id);
                                *app_ctx.player_name.write() = Some(player_name);
                                *app_ctx.game_state.write() = Some(state.into());
                                if let Some(code) = app_ctx.game_code.read().clone() {
                                    info!("Navigate to GameRoom");
                                    nav.push(Route::GameRoom { code });
                                }
                            }
                            ServerToClient::GameStateUpdate { game_state: state } => {
                                let state: GameState = state.into();
                                if state.globally_locked != *app_ctx.locally_locked.read() {
                                    *app_ctx.locally_locked.write() = state.globally_locked;
                                }
                                *app_ctx.game_state.write() = Some(state);
                            }
                            ServerToClient::PlayerBuzzed {
                                player_id: _,
                                player_name,
                            } => {
                                if *app_ctx.is_host.read() {
                                    let is_first_buzz = app_ctx.game_state.read()
                                        .as_ref()
                                        .map_or(false, |gs| gs.buzzer_order.is_empty());

                                    if is_first_buzz {
                                        log::info!(
                                            "First player '{}' buzzed! Playing sound for host.",
                                            player_name
                                        );

                                        // Get the sound selected in the settings menu
                                        let sound_src = app_ctx.buzzer_sound.read().clone();
                                        if let Ok(audio) = HtmlAudioElement::new_with_src(&sound_src) {
                                            let _ = audio.play();
                                        }
                                    } else {
                                         log::info!(
                                            "Player '{}' buzzed, but was not first. Sound suppressed.",
                                            player_name
                                        );
                                    }
                                }
                            }
                            ServerToClient::CountdownStarted { time_limit }  => {
                                // start counting down in the timer component
                            }
                            ServerToClient::TimerPaused { paused } => {}
                            ServerToClient::Error { message } => {
                                *app_ctx.error_message.write() = Some(message);
                            }
                        }
                    }
                    Err(e) => error!("Failed to parse S2C message: {}", e),
                }
            }
            info!("WebSocket connection closed!");
        });
    });

    rsx! {
        document::Link { rel: "icon", href: FAVICON }
        document::Stylesheet { href: CSS }
        div {
            class: "app-container",
            Outlet::<Route> {}
        }
    }
}

#[derive(Routable, Clone, PartialEq)]
#[rustfmt::skip]
enum Route {
    #[layout(NavBar)]
    #[layout(AppLayout)]
        #[route("/")]
        Home {},
        #[route("/game/:code")]
        GameRoom { code: usize },
    #[end_layout]
    // PageNotFound is a catch all route that will match any route
    #[route("/:..route")]
    PageNotFound { route: Vec<String> },
}

#[component]
pub fn NavBar() -> Element {
    rsx! {
        div { id: "title",
            h1 { "🚨 Buzzer! " }
        }
        Outlet::<Route> {}
    }
}

#[component]
fn Home() -> Element {
    let mut app_ctx = use_context::<AppContext>();
    let mut player_name = use_signal(String::new);
    let mut join_code = use_signal(|| String::default());

    let on_create_game = move |_| {
        info!("Creating game");
        app_ctx.send(ClientToServer::CreateGame);
    };

    let on_join_submit = move |_| {
        let name = player_name.read().trim().to_string();
        if name.is_empty() || name.len() > 12 {
            *app_ctx.error_message.write() =
                Some("Name must be between 1 and 12 characters.".to_string());
            return;
        }

        if name.to_lowercase() == "host" {
            *app_ctx.error_message.write() =
                Some("The name 'Host' is reserved.".to_string());
            return;
        }

        if let Ok(code) = join_code.read().parse::<usize>() {
            *app_ctx.game_code.write() = Some(code);
            app_ctx.send(ClientToServer::JoinGame {
                game_code: code,
                player_name: name,
            });
        }
    };

    rsx! {
         div {
            class: "home-page",
            if let Some(err) = (app_ctx.error_message)() {
                p { class: "error", "{err}" }
            }
            div {
                class: "home-section",
                h2 { "Create a New Game" }
                button {
                    class: "control-button",
                    onclick: on_create_game,
                    "Create Game"
                }
            }
            hr{}

            div {
                class: "home-section",
                h2 { "Join Game" }
                // 1. Wrap your inputs and button in a form tag
                form {
                    onsubmit: on_join_submit,
                    div { class: "form-field",
                        label { r#for: "game_code", "Game Code" }
                        input {
                            id: "game_code",
                            name: "game_code",
                            required: true,
                            value: "{join_code}",
                            oninput: move |evt| join_code.set(evt.value()),
                        }
                    }
                    div { class: "form-field",
                        label { r#for: "player_name", "Your Name" }
                        input {
                            id: "player_name",
                            name: "player_name",
                            required: true,
                            maxlength: 16,
                            value: "{player_name}",
                            oninput: move |evt| player_name.set(evt.value()),
                        }
                    }
                    div { class: "form-button-container",
                        button {
                            r#type: "submit",
                            class: "control-button",
                            "Join Game"
                        }
                    }
                }
            }
        }
    }
}

#[component]
pub fn GameRoom(code: usize) -> Element {
    let mut app_ctx = use_context::<AppContext>();
    let nav = navigator();
    let file_url = use_signal::<Option<String>>(|| None); // Unconditional hook

    // Effect to handle loading/redirect if game state isn't loaded promptly
    use_effect(move || {
        if app_ctx.game_state.read().is_none() {
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(200).await;
                if app_ctx.game_state.read().is_none() {
                    nav.push(Route::Home {});
                }
            });
        }
    });

    use_effect(move || {
        if let Some(game) = app_ctx.game_state.read().as_ref() {
            let my_id = *app_ctx.player_id.read();
            let is_host = my_id.is_some() && my_id == Some(game.host_id);
            app_ctx.is_host.set(is_host);
        }
    });

    if app_ctx.game_state.read().is_some() {
        if *app_ctx.is_host.read() {
            rsx! { HostView { file_url } }
        } else {
            rsx! { PlayerView {} }
        }
    } else {
        rsx! {
            div {
                class: "loading-page",
                h1 { "Joining game..." }
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
