use crate::{AppContext, NavBar, timer::Timer};
use common::*;
use log::warn;
use dioxus::{document::eval, prelude::*};

#[component]
pub fn PlayerBuzzOrderList(children: Element) -> Element {
    let app_ctx = use_context::<AppContext>();
    let game_state_guard = app_ctx.game_state.read();
    let players_data = if let Some(game) = game_state_guard.as_ref() {
        game.buzzer_order.iter().cloned().collect()
    } else {
        vec![]
    };
    rsx! {
        div {
            class: "buzzed-header",
            div {
                h3 { "Buzzed" }
            }
            // div {
            //     { children }
            // }
            // div {
            //     button {
            //         class: "control-button",
            //         onclick: move |_| {
            //             if let Some(code) = *app_ctx.game_code.read() {
            //                 app_ctx.send(ClientToServer::StartCountdown {
            //                     game_code: code,
            //                     time_limit: 10,
            //                 });
            //             }
            //         },
            //         "Start Timer"
            //     }
            // }
        }
        if !players_data.is_empty() {
            ol { class: "player-list buzzed-order-list",
                for (_, player_name) in players_data {
                    li {
                        "{player_name}"
                    }
                }
            }
        }
    }
}

#[component]
pub fn PlayerView() -> Element {
    let app_ctx = use_context::<AppContext>();
    let my_id = *app_ctx.player_id.read();

    // --- Focus the main div on mount ---
    use_future(move || async move {
        let eval = eval(
            r#"
            setTimeout(() => document.getElementById('player_view_wrapper')?.focus(), 50);
            "#,
        );
        if let Err(e) = eval.await {
            warn!("Couldn't focus: {e}");
        }
    });

    // This is the core logic, now without any arguments.
    let on_buzz = move || {
        if let Some(ref id) = my_id {
            if let Some(game_state) = app_ctx.game_state.read().as_ref() {
                let i_have_buzzed = my_id.map_or(false, |id| {
                    game_state.buzzer_order.iter().any(|(player_id, _)| *player_id == id)
                });
                if !game_state.globally_locked && !i_have_buzzed {
                    app_ctx.send(ClientToServer::Buzz {
                        game_code: app_ctx.game_code.read().clone().unwrap(),
                        player_id: *id,
                    });
                }
            }
        }
    };

    let game_state_guard = app_ctx.game_state.read();
    if let Some(game) = game_state_guard.as_ref() {
        let i_have_buzzed = my_id.map_or(false, |id| {
            game.buzzer_order
                .iter()
                .any(|(player_id, _)| *player_id == id)
        });
        let locked = game.globally_locked || i_have_buzzed;
        let buzzer_text = if locked { "Locked" } else { "BUZZ!" };
        let code_display = app_ctx.game_code.read().map_or_else(
            || "....".to_string(),
            |c| c.to_string()
        );
        let my_name = if let Some(name) = app_ctx.player_name.read().as_ref() {
            name.clone()
        } else {
            "".to_string()
        };

        let on_keydown = move |evt: KeyboardEvent| {
            if evt.key() == Key::Character(" ".to_owned()) {
                evt.prevent_default();
                on_buzz(); // Call the logic
            }
        };

        rsx! {
            div {
                class: "player-view-wrapper",
                id: "player_view_wrapper",
                tabindex: "0",
                onkeydown: on_keydown,
                div {
                    class: "player-view-container",
                    div {
                        class: "player-info-stack",
                        div {
                            class: "game-info-container",
                            p { class: "game-info", "Game Code: {code_display}" }
                        }
                        div {
                            class: "game-info-container",
                            p { class: "game-info", "Your name: {my_name}" }
                        }
                    }
                    div {
                        class: "buzzer-container",
                        button {
                            class: "buzzer",
                            disabled: locked,
                            onclick: move |_| on_buzz(), // Create a new closure for the event
                            "{buzzer_text}"
                        }
                    }
                }
                div {
                    class: "player-lists-wrapper",
                    PlayerBuzzOrderList {
                        // if let Some(time_limit) = app_ctx.time_limit.read().clone() {
                        //     Timer { time_limit: time_limit }
                        // }
                    },
                    PlayerList {}
                }
            }
        }
    } else {
        rsx! {
            div {
                class: "buzzer-container",
                button {
                    class: "buzzer",
                    disabled: true,
                    "Loading..."
                }
            }
        }
    }
}

#[component]
pub fn PlayerList() -> Element {
    let app_ctx = use_context::<AppContext>();
    let players_data = use_memo(move || {
        if let Some(game) = app_ctx.game_state.read().as_ref() {
            let mut players: Vec<_> = game
                .player_join_order
                .iter()
                .filter_map(|player_id| {
                    game.players
                        .get(player_id)
                        .map(|player| (player_id, player))
                })
                .filter(|(_, player)| player.name() != HOST)
                .map(|(player_id, player)| {
                    (
                        player.name().to_string(),
                        *game.scores.get(player_id).unwrap_or(&0),
                    )
                })
                .collect();
            players.sort_by(|p1, p2| p2.1.cmp(&p1.1));
            players
        } else {
            vec![]
        }
    });

    rsx! {
        h3 { "Players" }
        ul { class: "player-list",
            for (player_name, score) in players_data.read().iter() {
                li {
                    class: "player-list-item",
                    span { class: "player-name", "{player_name}" }
                    span { class: "score-display", " {score}" }
                }
            }
        }
    }
}
