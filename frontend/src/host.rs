use dioxus::prelude::*;
use log::info;
use common::*;
use crate::{AppContext, Route};

#[component]
pub fn GameRoom(code: String) -> Element {
    let app_ctx = use_context::<AppContext>();
    let my_id = app_ctx.player_id.read();
    let nav = navigator();
    // If we land here but have no state, wait briefly then check again
    //     use_effect(move || {
    //     spawn(async move {
    //         tokio::time::sleep(Duration::from_secs(3)).await;
    //         if app_ctx.game_state.read().is_none() {
    //             nav.push(Route::Home {});
    //         }
    //     });
    // });
    // If we land here but have no state, redirect home
    info!("Game state: {:?}", &app_ctx.game_state);
    if app_ctx.game_state.read().is_none() {
        nav.push(Route::Home {});
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
pub fn HostView() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game_code = app_ctx.game_code.read().clone().unwrap();
    let game = app_ctx.game_state.read().clone().unwrap();

    let on_lock_ctx = app_ctx.clone();
    let on_lock_code = game_code.clone();
    let on_lock = move |_| on_lock_ctx.send(ClientToServer::Lock { game_code: on_lock_code.clone() });

    let on_unlock_ctx = app_ctx.clone();
    let on_unlock_code = game_code.clone();
    let on_unlock = move |_| {
        on_unlock_ctx.send(ClientToServer::Unlock { game_code: on_unlock_code.clone() });
    };

    let on_clear_ctx = app_ctx.clone();
    let on_clear_code = game_code.clone();
    let on_clear = move |_| {
        on_clear_ctx.send(ClientToServer::Clear { game_code: on_clear_code.clone() });
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
    let game_code = app_ctx.game_code.read().clone().unwrap();
    let my_id = app_ctx.player_id.read().unwrap();
    let game = app_ctx.game_state.read().clone().unwrap();

    let someone_buzzed = !game.buzzer_order.is_empty();
    let is_locked = game.locked;

    let on_buzz = move |_| {
        app_ctx.send(ClientToServer::Buzz {
            game_code: game_code.clone(),
            player_id: my_id,
        })
    };

    let buzzer_text = if someone_buzzed {
        let mut buzz_order = Vec::new();
        for id in game.buzzer_order {
            buzz_order.push(game.players.get(&id).unwrap())
        }
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
pub fn PlayerList() -> Element {
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
