use crate::{player::PlayerView, AppContext, PlayerBuzzOrderList, Route};
use common::*;
use dioxus::prelude::*;
use log::info;
use web_sys::window;

#[derive(Clone, PartialEq)]
struct HostContext {
    pub copied: Signal<bool>,
}

#[component]
pub fn HostView() -> Element {
    let app_ctx = use_context::<AppContext>();

    // --- Create the Host-specific state ---
    let copied = use_signal(|| false);

    // --- Provide the new, scoped HostContext ---
    // Any component rendered as a child of HostView can now access this.
    use_context_provider(|| HostContext { copied });

    let on_lock = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            app_ctx.send(ClientToServer::Lock { game_code: code });
        } else {
            log::error!("Cannot clear: game_code is not set.");
        }
    };
    let on_unlock = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            app_ctx.send(ClientToServer::Unlock { game_code: code });
        } else {
            log::error!("Cannot clear: game_code is not set.");
        }
    };
    let on_clear = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            app_ctx.send(ClientToServer::Clear { game_code: code });
        } else {
            log::error!("Cannot clear: game_code is not set.");
        }
    };

    // --- NEW: Prepare data *before* rendering ---
    let game_state_guard = app_ctx.game_state.read();
    let players_data = if let Some(game) = game_state_guard.as_ref() {
        // 1. Collect the data into a new Vec of simple, owned types.
        game.players
            .iter()
            .filter(|p| p.name() != HOST)
            .map(|p| {
                (
                    p.id(),                                  // Uuid is `Copy`
                    p.name().to_string(),                    // String is `Owned`
                    *game.scores.get(&p.id()).unwrap_or(&0), // i32 is `Copy`
                )
            })
            .collect::<Vec<_>>()
    } else {
        // If no game state, render an empty list.
        vec![]
    };
    let game_code = (*app_ctx.game_code.read())
        .map(|c| c.to_string())
        .unwrap_or_default();
    rsx! {
            if let Some(game) = app_ctx.game_state.read().as_ref() {
                h2 { class: "host-controls-title", "Host Controls" }
                div {
                    class: "game-info-container",
                    p { class: "game-info", "Game Code: {game_code}" }
                    CopyButton {}
                }
                div {
                    class: "host-controls",
                    if game.globally_locked {
                        button { class: "control-button", onclick: on_unlock, "Unlock Buzzers" }
                    } else {
                        button { class: "control-button", onclick: on_lock, "Lock Buzzer" }
                    }
                    button { class: "control-button", onclick: on_clear, "Clear Buzzer" }
                }
                div {
                    class: "score-controls",
                    h3 { "Score Controls" }
                    for (player_id, player_name, score) in players_data {
                        div {
                            class: "score-control",
                            span { "{player_name}" }
                            span { class: "score-display", " {score}" }
                            button {
                                class: "score-button",
                                onclick: move |_| {
                                    // Read the game_code directly from the context here.
                                    if let Some(code) = *app_ctx.game_code.read() {
                                        app_ctx.send(ClientToServer::UpdateScore {
                                            game_code: code,
                                            player_id,
                                            delta: 1,
                                        });
                                    }
                                },
                                "+"
                            }
                            button {
                                class: "score-button",
                                onclick: move |_| {
                                    // Read it again here. This is perfectly fine and safe.
                                    if let Some(code) = *app_ctx.game_code.read() {
                                        app_ctx.send(ClientToServer::UpdateScore {
                                            game_code: code,
                                            player_id,
                                            delta: -1,
                                        });
                                    }
                                },
                                "-"
                            }
                        }
                    }
                }
        }
    }
}

#[component]
pub fn PlayerList() -> Element {
    let app_ctx = use_context::<AppContext>();

    rsx! {
        // Read the signal HERE. This makes the component reactive.
        if let Some(game) = app_ctx.game_state.read().as_ref() {
            h3 { "Players" }
            ul { class: "player-list",
                // You can now safely iterate.
                for player in game.players.iter().filter(|p| p.name() != HOST) {
                    li {
                        span { "{player.name()}" }
                        span { class: "score-display", " ({game.scores.get(&player.id()).unwrap_or(&0)})" }
                    }
                }
            }
        }
    }
}

#[component]
fn CopyButton() -> Element {
    // This component consumes BOTH contexts!
    let app_ctx = use_context::<AppContext>(); // Global
    let mut host_ctx = use_context::<HostContext>(); // Host-specific

    let copy_to_clipboard = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            if let Some(window) = window() {
                let clipboard = window.navigator().clipboard();
                let _ = clipboard.write_text(&code.to_string());
                host_ctx.copied.set(true);
                info!("Copied game code: {}", code);
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    host_ctx.copied.set(false);
                });
            } else {
                info!("Window not available");
            }
            // When copy is successful, update the signal from the HostContext
            host_ctx.copied.set(true);
            spawn(async move {
                gloo_timers::future::TimeoutFuture::new(2000).await;
                host_ctx.copied.set(false);
            });
        }
    };

    // Read the `copied` signal from the host context for display
    let button_text = if *host_ctx.copied.read() {
        "Copied!"
    } else {
        "Copy"
    };

    rsx! {
        button {
            class: "copy-button",
            onclick: copy_to_clipboard,
            aria_label: "Copy game code",
            "{button_text}"
        }
    }
}
