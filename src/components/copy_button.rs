use crate::browser;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum State {
    Ready,
    Copied,
    Blocked,
}

/// Copies `text` and says so. If the browser blocks the clipboard it asks the
/// visitor to select the text instead (it's always shown next to the button).
#[component]
pub fn CopyButton(
    text: String,
    #[prop(default = "Copy")] label: &'static str,
    /// Read out to screen readers, e.g. "Copy the /tell command".
    #[prop(optional)]
    describe: Option<String>,
) -> impl IntoView {
    let state = RwSignal::new(State::Ready);
    let click = move |_| {
        let text = text.clone();
        spawn_local(async move {
            let next = if browser::copy(&text).await {
                State::Copied
            } else {
                State::Blocked
            };
            state.set(next);
            set_timeout(move || state.set(State::Ready), Duration::from_millis(2400));
        });
    };
    view! {
        <button type="button" class="btn copy" aria-label=describe on:click=click>
            {move || match state.get() {
                State::Ready => label,
                State::Copied => "Copied",
                State::Blocked => "Select to copy",
            }}
        </button>
    }
}
