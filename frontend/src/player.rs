use crate::{host::PlayerBuzzOrderList, AppContext};
use common::*;
use dioxus::prelude::*;

#[component]
pub fn PlayerView() -> Element {
    let app_ctx = use_context::<AppContext>();
    let my_id = *app_ctx.player_id.read();

    let on_buzz = move |_| {
        if let Some(ref id) = my_id {
            app_ctx.send(ClientToServer::Buzz {
                game_code: app_ctx.game_code.read().clone().unwrap(),
                player_id: *id,
            });
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
        rsx! {
            div {
                class: "player-view-wrapper",
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
                            onclick: on_buzz,
                            "{buzzer_text}"
                        }
                    }
                }
                div {
                    class: "player-lists-wrapper",
                    PlayerBuzzOrderList {},
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
    let game_state_guard = app_ctx.game_state.read();
    let mut players_data = if let Some(game) = game_state_guard.as_ref() {
        game.player_join_order
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
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    players_data.sort_by(|p1, p2| p2.1.cmp(&p1.1));

    rsx! {
        h3 { "Players" }
        ul { class: "player-list",
            // You can now safely iterate.
            for (player_name, score) in players_data {
                li {
                    class: "player-list-item",
                    span { class: "player-name", "{player_name}" }
                    span { class: "score-display", " {score}" }
                }
            }
        }
    }
}
