use crate::{AppContext, Route};
use common::*;
use dioxus::prelude::*;
use log::info;

#[component]
pub fn GameRoom(code: String) -> Element {
    let app_ctx = use_context::<AppContext>();
    let my_id = app_ctx.player_id.read();
    let nav = navigator();
    // If we land here but have no state, redirect home
    info!("Game state: {:?}", &app_ctx.game_state);
    if app_ctx.game_state.read().is_none() {
        nav.push(Route::Home {});
    }

    let host_id = app_ctx.game_state.read().as_ref().unwrap().host_id;
    let is_host = *my_id == Some(host_id);

    rsx! {
        p { class: "game-info", "Game Code: {code}" }
        if is_host {
            HostView {}
        } else {
            PlayerView {}
        }
        PlayerBuzzOrderList {}
        PlayerList {}
    }
}

#[component]
pub fn HostView() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game = app_ctx.game_state.read().clone().unwrap();

    let on_lock = move |_| {
        app_ctx.send(ClientToServer::Lock {
            game_code: app_ctx.game_code.clone().unwrap(),
        })
    };

    let on_unlock = move |_| {
        app_ctx.send(ClientToServer::Unlock {
            game_code: app_ctx.game_code.clone().unwrap(),
        });
    };

    let on_clear = move |_| {
        app_ctx.send(ClientToServer::Clear {
            game_code: app_ctx.game_code.clone().unwrap(),
        });
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
pub fn PlayerView() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game = app_ctx.game_state.read().clone().unwrap();

    // let someone_buzzed = !game.buzzer_order.is_empty();
    let is_locked = game.locked;

    let on_buzz = move |_| {
        // *app_ctx.game_state.write() = Some(true);
        app_ctx.send(ClientToServer::Buzz {
            game_code: app_ctx.game_code.read().clone().unwrap(),
            player_id: app_ctx.player_id.read().unwrap(),
        })
    };

    let buzzer_text = if is_locked {
        "Locked".to_string()
    } else {
        "BUZZ!".to_string()
    };

    rsx! {
        button {
            class: "buzzer",
            disabled: is_locked,
            onclick: on_buzz,
            "{buzzer_text}"
        }
    }
}

#[component]
pub fn PlayerList() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game = app_ctx.game_state.read().clone().unwrap();

    info!("Show game: {:?}", game);
    rsx! {
        h3 { "Players" }
        ul { class: "player-list",
            for (_, player) in game.players {
                li {
                    "{player.name()}"
                }
            }
        }
    }
}

#[component]
pub fn PlayerBuzzOrderList() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game = app_ctx.game_state.read().clone().unwrap();
    rsx! {
        h3 { "Buzzed" }
        ol { class: "player-list",
            for (_, player_name) in game. buzzer_order.iter() {
                li {
                    // class: if Some(*id) == winner_id { "winner" } else { "" },
                    "{player_name}"
                }
            }
        }
    }
}
