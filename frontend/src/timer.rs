use dioxus::prelude::*;
use gloo_timers::future::TimeoutFuture;

#[component]
pub fn Timer(time_limit: u32) -> Element {
    let mut remaining_time = use_signal(|| time_limit);

    use_future(move || async move {
        while *remaining_time.read() > 0 {
            TimeoutFuture::new(1_000).await;
            let current_time = *remaining_time.read();
            remaining_time.set(current_time - 1);
        }
    });

    let timer_display = if *remaining_time.read() > 0 {
        format!("{}", remaining_time)
    } else {
        "Time's up!".to_string()
    };

    let timer_class = if *remaining_time.read() > 0 {
        "timer-running"
    } else {
        "timer-finished"
    };

    rsx! {
        div {
            class: "timer-container",
            span {
                class: "{timer_class}",
                "{timer_display}"
            }
        }
    }
}
