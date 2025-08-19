use crate::AppContext;
use common::*;
use dioxus::prelude::*;
use log::info;
use web_sys::window;

#[derive(Clone, PartialEq)]
struct HostContext {
    pub copied: Signal<bool>,
    pub score_delta: Signal<i32>,
}

#[component]
pub fn HostView(file_url: Signal<Option<String>>) -> Element {
    let app_ctx = use_context::<AppContext>();

    // --- Create the Host-specific state ---
    let copied = use_signal(|| false);
    let score_delta = use_signal(|| 10_i32);
    let mut show_settings = use_signal(|| false);

    // --- Provide the new, scoped HostContext ---
    use_context_provider(|| HostContext {
        copied,
        score_delta,
    });

    let on_lock = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            app_ctx.send(ClientToServer::Lock { game_code: code });
        } else {
            log::error!("Cannot lock: game_code is not set.");
        }
    };
    let on_unlock = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            app_ctx.send(ClientToServer::Unlock { game_code: code });
        } else {
            log::error!("Cannot unlock: game_code is not set.");
        }
    };
    let on_clear = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            app_ctx.send(ClientToServer::Clear { game_code: code });
        } else {
            log::error!("Cannot clear: game_code is not set.");
        }
    };

    // --- Prepare data *before* rendering ---
    let game_state_guard = app_ctx.game_state.read();
    let players_data = if let Some(game) = game_state_guard.as_ref() {
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
                    *player_id,
                    player.name().to_string(),
                    *game.scores.get(player_id).unwrap_or(&0),
                )
            })
            .collect::<Vec<_>>()
    } else {
        vec![]
    };
    let game_code = (*app_ctx.game_code.read())
        .map(|c| c.to_string())
        .unwrap_or_default();

    rsx! {
        div {
            class: "host-view-container",
            // --- Left Column ---
            if file_url.read().is_some() {
                div { class: "file-viewer-column",
                    FileViewer { file_url }
                }
            }
            // --- Right Column ---
            div { class: "host-controls-column",
                if let Some(game) = app_ctx.game_state.read().as_ref() {
                    div {
                        class: "game-info-container",
                        p { class: "game-info", "Game Code: {game_code}" }
                        CopyButton {}
                    }
                    div {
                        class: "host-controls",
                        if game.globally_locked {
                            button { class: "unlock-button", onclick: on_unlock, "Unlock Buzzers" }
                        } else {
                            button { class: "lock-button", onclick: on_lock, "Lock Buzzers" }
                        }
                        button { class: "control-button", onclick: on_clear, "Clear Buzzer" }
                        button {
                            "aria-label": "Open settings",
                            class: "control-button settings-button",
                            onclick: move |_| show_settings.set(true),
                            svg {
                                xmlns: "http://www.w3.org/2000/svg",
                                view_box: "0 0 24 24",
                                fill: "currentColor",
                                path {
                                    d: "M19.14,12.94c0.04-0.3,0.06-0.61,0.06-0.94c0-0.32-0.02-0.64-0.07-0.94l2.03-1.58c0.18-0.14,0.23-0.41,0.12-0.61 \
                                        l-1.92-3.32c-0.12-0.22-0.37-0.29-0.59-0.22l-2.39,0.96c-0.5-0.38-1.03-0.7-1.62-0.94L14.4,2.81\
                                        C14.33,2.59,14.12,2.4,13.86,2.4h-3.72c-0.26,0-0.47,0.19-0.54,0.41L9.2,5.27\
                                        C8.61,5.51,8.08,5.83,7.58,6.21L5.19,5.25C4.97,5.18,4.72,5.25,4.6,5.47L2.68,8.79
                                        c-0.11,0.2-0.06,0.47,0.12,0.61l2.03,1.58C4.78,11.36,4.76,11.68,4.76,12s0.02,0.64,0.07,0.94l-2.03,1.58
                                        c-0.18,0.14-0.23,0.41-0.12,0.61l1.92,3.32c0.12,0.22,0.37,0.29,0.59,0.22l2.39-0.96c0.5,0.38,1.03,0.7,1.62,0.94
                                        l0.4,2.46c0.07,0.22,0.28,0.41,0.54,0.41h3.72c0.26,0,0.47-0.19,0.54-0.41l0.4-2.46c0.59-0.24,1.12-0.56,1.62-0.94
                                        l2.39,0.96c0.22,0.07,0.47,0,0.59-0.22l1.92-3.32c0.11-0.20,0.06-0.47-0.12-0.61L19.14,12.94z
                                        M12,15.6 c-1.98,0-3.6-1.62-3.6-3.6s1.62-3.6,3.6-3.6s3.6,1.62,3.6,3.6S13.98,15.6,12,15.6z"
                                }
                            }
                        }
                        // MOVED: The FileUploader component is no longer here
                    }
                    if show_settings() {
                        // MODIFIED: Pass the file_url signal to the settings menu
                        SettingsMenu { is_open: show_settings, file_url }
                    }
                    PlayerBuzzOrderList {}
                    div {
                        class: "player-list-container",
                        h3 { "Players & Scores" }
                        ul {
                            class: "player-list",
                            for (player_id, player_name, score) in players_data {
                                li {
                                    class: "player-list-item",
                                    span { class: "player-name", "{player_name}" }
                                    span { class: "score-display", "{score}" }
                                    div {
                                        class: "score-buttons-container",
                                        button {
                                            class: "score-button",
                                            onclick: move |_| {
                                                if let Some(code) = *app_ctx.game_code.read() {
                                                    app_ctx.send(ClientToServer::UpdateScore {
                                                        game_code: code,
                                                        player_id,
                                                        delta: *score_delta.read(),
                                                    });
                                                }
                                            },
                                            "+"
                                        }
                                        button {
                                            class: "score-button",
                                            onclick: move |_| {
                                                if let Some(code) = *app_ctx.game_code.read() {
                                                    app_ctx.send(ClientToServer::UpdateScore {
                                                        game_code: code,
                                                        player_id,
                                                        delta: -(*score_delta.read()),
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
            }
        }
    }
}

#[component]
pub fn PlayerBuzzOrderList() -> Element {
    let app_ctx = use_context::<AppContext>();
    let game_state_guard = app_ctx.game_state.read();
    let players_data = if let Some(game) = game_state_guard.as_ref() {
        game.buzzer_order.iter().collect()
    } else {
        vec![]
    };
    rsx! {
        h3 { "Buzzed" }
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
fn CopyButton() -> Element {
    let app_ctx = use_context::<AppContext>();
    let mut host_ctx = use_context::<HostContext>();

    let copy_to_clipboard = move |_| {
        if let Some(code) = *app_ctx.game_code.read() {
            if let Some(window) = window() {
                let clipboard = window.navigator().clipboard();
                let _ = clipboard.write_text(&code.to_string());
                host_ctx.copied.set(true);
                spawn(async move {
                    gloo_timers::future::TimeoutFuture::new(2000).await;
                    host_ctx.copied.set(false);
                });
            } else {
                info!("Window not available");
            }
        }
    };

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

// MODIFIED: Component now accepts the file_url signal
#[component]
fn SettingsMenu(is_open: Signal<bool>, file_url: Signal<Option<String>>) -> Element {
    let mut app_ctx = use_context::<AppContext>();
    let mut host_ctx = use_context::<HostContext>();

    let sound_options = vec![
        ("Default Buzzer", "../assets/ding-101492.mp3"),
        ("Ding", "sounds/ding.mp3"),
        ("Boop", "sounds/boop.mp3"),
    ];

    rsx! {
        div {
            class: "settings-backdrop",
            onclick: move |_| is_open.set(false),
        }
        div {
            class: "settings-menu",
            h2 { "Settings" }
            div {
                class: "setting-item",
                label { r#for: "delta-input", "Score Increment:" }
                input {
                    r#type: "number",
                    id: "delta-input",
                    min: "1",
                    value: "{host_ctx.score_delta}",
                    oninput: move |evt| {
                        if let Ok(val) = evt.value().parse::<i32>() {
                            host_ctx.score_delta.set(val.max(1));
                        }
                    }
                }
            }
            div {
                class: "setting-item",
                label { r#for: "sound-select", "Buzzer Sound:" }
                select {
                    id: "sound-select",
                    onchange: move |evt| {
                        app_ctx.buzzer_sound.set(evt.value());
                    },
                    for (name, path) in sound_options.iter() {
                        option {
                            value: *path,
                            selected: *app_ctx.buzzer_sound.read() == *path,
                            "{name}"
                        }
                    }
                }
            }
            // ADDED: File uploader is now a setting
            div {
                class: "setting-item",
                label { r#for: "pdf-upload", "Upload PDF:" }
                FileUploader { file_url }
            }
            div {
                class: "settings-footer",
                button {
                    class: "control-button",
                    onclick: move |_| is_open.set(false),
                    "Close"
                }
            }
        }
    }
}

// MODIFIED: Added a close button to the viewer
#[component]
pub fn FileViewer(file_url: Signal<Option<String>>) -> Element {
    rsx! {
        div {
            class: "pdf-viewer-container",
            if let Some(url) = file_url() {
                // ADDED: This button will set the file_url to None, effectively closing the viewer
                button {
                    class: "pdf-close-button",
                    "aria-label": "Close PDF Viewer",
                    onclick: move |_| file_url.set(None),
                    "×" // A nice 'times' character for the X
                }
                iframe {
                    src: "{url}",
                    class: "pdf-iframe",
                    title: "PDF Viewer",
                }
            } else {
                div {
                    class: "pdf-placeholder",
                    "No PDF uploaded. Select a PDF file to view."
                }
            }
        }
    }
}

#[component]
fn FileUploader(file_url: Signal<Option<String>>) -> Element {
    let mut error_message = use_signal(|| None::<String>);

    let on_file_change = move |event: Event<FormData>| {
        spawn(async move {
            if let Some(file_engine) = event.files() {
                let files = file_engine.files();
                if let Some(file_name) = files.first() {
                    if let Some(bytes) = file_engine.read_file(file_name).await {
                        let uint8_array =
                            web_sys::js_sys::Uint8Array::new_with_length(bytes.len() as u32);
                        for (i, &byte) in bytes.iter().enumerate() {
                            uint8_array.set_index(i as u32, byte);
                        }
                        let array = web_sys::js_sys::Array::new();
                        array.push(&uint8_array.buffer());
                        let init = web_sys::BlobPropertyBag::new();
                        init.set_type("application/pdf");
                        if let Ok(blob) = web_sys::Blob::new_with_buffer_source_sequence_and_options(
                            &array, &init,
                        ) {
                            if let Ok(url) = web_sys::Url::create_object_url_with_blob(&blob) {
                                file_url.set(Some(url));
                                error_message.set(None);
                            } else {
                                error_message.set(Some("Failed to create URL".to_string()));
                            }
                        } else {
                            error_message.set(Some("Failed to create Blob".to_string()));
                        }
                    } else {
                        error_message.set(Some("Failed to read file".to_string()));
                    }
                } else {
                    error_message.set(Some("No file selected.".to_string()));
                }
            } else {
                error_message.set(Some("No files provided.".to_string()));
            }
        });
    };

    rsx! {
        div {
            class: "file-uploader-container",
            input {
                r#type: "file",
                accept: ".pdf",
                id: "pdf-upload",
                onchange: on_file_change,
                class: "file-input"
            }
            if let Some(err) = error_message() {
                p { class: "error-message", "{err}" }
            }
        }
    }
}
