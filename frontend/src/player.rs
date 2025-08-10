use crate::AppContext;
use common::*;
use dioxus::prelude::*;

#[component]
pub fn PlayerView() -> Element {
    let app_ctx = use_context::<AppContext>();
    // Get the current player's ID once.
    let my_id = *app_ctx.player_id.read();

    let on_buzz = move |_| {
        // We no longer need to manage a local lock. Just send the message.
        // The UI will update automatically when the server confirms the buzz.
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
        let code = app_ctx.game_code.read().clone().unwrap();
        rsx! {
            div {
                class: "game-info-container",
                p { class: "game-info", "Game Code: {code}" }
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
