//! Small line drawings used around the shop. They're drawn here rather than
//! loaded, so they always appear.

use crate::model::Glyph;
use leptos::prelude::*;

/// Shown in an item's slot until (or instead of) its game icon.
#[component]
pub fn GlyphIcon(glyph: Glyph) -> impl IntoView {
    let paths: &[&str] = match glyph {
        Glyph::Sword => &[
            "M19.5 4.5 L19.5 8 L10.5 17 L7 13.5 L16 4.5 Z",
            "M5.5 12 L12 18.5",
            "M8.7 15.3 L4.8 19.2",
            "M3.8 18.6 a1.1 1.1 0 1 0 1.6 1.6 a1.1 1.1 0 1 0 -1.6 -1.6",
        ],
        Glyph::Shield => &[
            "M12 3 L19 5.8 V11 C19 15.6 16 19 12 21 C8 19 5 15.6 5 11 V5.8 Z",
            "M12 6.5 V17.5 M8.2 10.5 H15.8",
        ],
        Glyph::Armor => &[
            "M8.5 4 L4 6.8 L5.7 11.2 L7 10.6 V20 H17 V10.6 L18.3 11.2 L20 6.8 L15.5 4 C14.8 5.8 13.5 6.8 12 6.8 C10.5 6.8 9.2 5.8 8.5 4 Z",
            "M9.5 13.5 H14.5",
        ],
        Glyph::Ring => &[
            "M6 14.5 a6 6 0 1 0 12 0 a6 6 0 1 0 -12 0",
            "M9.4 5.6 L12 3 L14.6 5.6 L12 8.6 Z",
        ],
        Glyph::Tool => &[
            "M10.7 6.6 L14.4 2.9 L21.1 9.6 L17.4 13.3 Z",
            "M12.9 10.8 L4.2 19.5",
        ],
        Glyph::Potion => &[
            "M9.5 3 H14.5",
            "M10.5 3 V8.8 L6.2 16.6 C5.4 18.3 6.5 20.5 8.4 20.5 H15.6 C17.5 20.5 18.6 18.3 17.8 16.6 L13.5 8.8 V3",
            "M7.8 14.2 H16.2",
        ],
        Glyph::Food => &[
            "M4 11.5 H20 C20 15.9 16.4 19 12 19 C7.6 19 4 15.9 4 11.5 Z",
            "M9 8.5 C9 7 10 7 10 5.5 M13.5 8.5 C13.5 7 14.5 7 14.5 5.5",
            "M9 21 H15",
        ],
        Glyph::House => &[
            "M3.5 11 L12 4 L20.5 11",
            "M6 9.3 V20 H18 V9.3",
            "M10 20 V15 H14 V20",
        ],
        Glyph::Gem => &[
            "M7 4.5 H17 L21 9.5 L12 20 L3 9.5 Z",
            "M3 9.5 H21",
            "M9.5 4.5 L8 9.5 L12 20 L16 9.5 L14.5 4.5",
        ],
        Glyph::Crate => &[
            "M4 7.2 L12 3.4 L20 7.2 V16.8 L12 20.6 L4 16.8 Z",
            "M4 7.2 L12 11 L20 7.2",
            "M12 11 V20.6",
        ],
    };
    view! {
        <svg
            class="glyph"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            {paths.iter().map(|d| view! { <path d=*d></path> }).collect_view()}
        </svg>
    }
}

/// The gil coin.
#[component]
pub fn GilIcon() -> impl IntoView {
    view! {
        <svg class="gil" viewBox="0 0 16 16" aria-hidden="true">
            <circle class="gil-rim" cx="8" cy="8" r="7.2"></circle>
            <circle class="gil-face" cx="8" cy="8" r="5.5"></circle>
            <path class="gil-shine" d="M5.2 6.4 A3.3 3.3 0 0 1 9.2 4.8"></path>
        </svg>
    }
}

#[component]
pub fn ClockIcon() -> impl IntoView {
    view! {
        <svg
            class="clock"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.4"
            stroke-linecap="round"
            aria-hidden="true"
        >
            <circle cx="8" cy="8" r="6.3"></circle>
            <path d="M8 4.6 V8 L10.4 9.6"></path>
        </svg>
    }
}

#[component]
pub fn SearchIcon() -> impl IntoView {
    view! {
        <svg
            class="search-icon"
            viewBox="0 0 16 16"
            fill="none"
            stroke="currentColor"
            stroke-width="1.5"
            stroke-linecap="round"
            aria-hidden="true"
        >
            <circle cx="7" cy="7" r="4.6"></circle>
            <path d="M10.4 10.4 L14 14"></path>
        </svg>
    }
}

/// The arrow on the Back to top button.
#[component]
pub fn ArrowUpIcon() -> impl IntoView {
    view! {
        <svg
            class="arrow-up"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.8"
            stroke-linecap="round"
            stroke-linejoin="round"
            aria-hidden="true"
        >
            <path d="M12 19 V5.5"></path>
            <path d="M6.5 11 L12 5.5 L17.5 11"></path>
        </svg>
    }
}
