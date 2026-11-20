//! A button in the bottom-right corner that goes back to the top of the page.
//! It never shows while the hero is on screen. After that:
//!
//! - where the margin beside the page content has room for it (wide windows),
//!   it shows as soon as the hero has scrolled away;
//! - where it doesn't, it would cover the page as it scrolls past, so it waits
//!   until the page is scrolled all the way down, then shows in the empty space
//!   below the footer. If the page then grows (another tab of wares, say),
//!   that's no longer the bottom, so it hides again.
//!
//! The stacked layout (narrow screens) hides it altogether: see the 960px
//! block in main.css.

use super::glyphs::ArrowUpIcon;
use crate::browser;
use leptos::ev;
use leptos::prelude::*;

/// The least space kept between the button and the page content, in pixels.
const CLEARANCE: f64 = 16.0;
/// How close to the very bottom counts as scrolled all the way down, in pixels.
const BOTTOM_SLACK: f64 = 2.0;

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
        // much room the corner has beside the page content (the shop's width).
        let room = browser::rect(".market").is_some_and(|content| {
            button.get_bounding_client_rect().left() - CLEARANCE >= content.right()
        });
        let show = past_hero && (room || browser::at_bottom(BOTTOM_SLACK));
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
    // The page can also change height without scrolling, when a tab or a
    // search changes the list of wares, say, which moves the bottom. This
    // also checks once the page is first laid out. It's stored with the
    // button, so it keeps watching until the button goes.
    StoredValue::new_local(browser::watch_size("body", update));

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
