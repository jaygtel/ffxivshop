//! A button in the bottom-right corner that goes back to the top of the page.
//! It appears once the hero has scrolled out of view. The stacked layout (on
//! narrow screens) hides it altogether: see the 960px block in main.css.

use super::glyphs::ArrowUpIcon;
use crate::browser;
use leptos::ev;
use leptos::prelude::*;

#[component]
pub fn BackToTop() -> impl IntoView {
    let shown = RwSignal::new(false);

    // The hero is the first thing on the page; "past" means none of it is left
    // on screen.
    let update = move || {
        let past_hero = browser::rect(".hero").is_some_and(|hero| hero.bottom() < 1.0);
        if shown.get_untracked() != past_hero {
            shown.set(past_hero);
        }
    };

    let on_scroll = window_event_listener(ev::scroll, move |_| update());
    let on_resize = window_event_listener(ev::resize, move |_| update());
    on_cleanup(move || {
        on_scroll.remove();
        on_resize.remove();
    });
    // Once the page is laid out, and again if the browser restores a scroll
    // position (that fires a scroll event).
    request_animation_frame(update);

    view! {
        <button
            type="button"
            class="to-top"
            class:shown=move || shown.get()
            aria-label="Back to top"
            title="Back to top"
            on:click=move |_| {
                browser::scroll_to("top");
                browser::focus_quietly("top");
            }
        >
            <ArrowUpIcon />
        </button>
    }
}
