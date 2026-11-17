//! A button in the bottom-right corner that goes back to the top of the page.
//! It appears once the hero has scrolled out of view, and only when the margin
//! beside the page content is wide enough to hold it without covering
//! anything. That rules it out whenever the page is stacked on narrow screens,
//! where the content runs to the edge of the window.

use super::glyphs::ArrowUpIcon;
use crate::browser;
use leptos::ev;
use leptos::prelude::*;

/// The least space kept between the button and the page content, in pixels.
const CLEARANCE: f64 = 16.0;

#[component]
pub fn BackToTop() -> impl IntoView {
    let button = NodeRef::<leptos::html::Button>::new();
    let shown = RwSignal::new(false);

    let update = move || {
        let Some(button) = button.get_untracked() else {
            return;
        };
        // The hero is the first thing on the page; "past" means none of it is
        // left on screen.
        let past_hero = browser::rect(".hero").is_some_and(|hero| hero.bottom() < 1.0);
        // The button keeps its place while hidden, so its left edge shows how
        // much room the corner has. The page content's right edge is the shop's.
        let room = browser::rect(".market").is_some_and(|content| {
            button.get_bounding_client_rect().left() - CLEARANCE >= content.right()
        });
        let show = past_hero && room;
        if shown.get_untracked() != show {
            shown.set(show);
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
            node_ref=button
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
