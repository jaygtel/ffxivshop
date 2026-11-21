//! The navigation bar on every page but the homepage. It sits at the very top
//! of the page and scrolls away with it: dark glass with a very slight
//! shadow, and a gold line along its foot that runs into the emblem's gold
//! ring on each side. The hero's emblem, with all its animation, sits in the
//! centre, hanging below the bar.
//!
//! From 961px wide, the shop's name, set like the hero's, sits on the left
//! and the pages on the right, which glow very slightly under the pointer.
//! The name and the emblem both lead to the homepage.
//!
//! Narrower than that, the bar holds only the emblem, which opens and closes
//! the menu: the hero's panel, frame and all, filling the screen below the
//! bar and holding the name, the divider and the pages. The top edge of its
//! frame takes the place of the bar's gold line while it's open, with the
//! emblem over it. Choosing a page or pressing Escape also closes it. While it's
//! open, the page behind it can't scroll or be reached (see src/app.rs).
//!
//! The homepage has no bar: its hero already carries the emblem and the
//! name, and Order Now leads to the shop.

use super::hero::{CORNERS, divider_art, emblem_art, name_words};
use crate::browser;
use crate::model::Shop;
use crate::pages;
use leptos::ev;
use leptos::prelude::*;
use leptos_router::components::A;

/// Whether the menu is open. src/app.rs owns it, as the rest of the page
/// can't be reached while it's open.
#[derive(Clone, Copy)]
pub struct MenuOpen(pub RwSignal<bool>);

/// The width from which the whole bar shows and the menu isn't used (the
/// 961px in main.css).
const WIDE_FROM: f64 = 961.0;

#[component]
pub fn SiteNav(shop: &'static Shop) -> impl IntoView {
    let MenuOpen(open) = expect_context::<MenuOpen>();
    let button = NodeRef::<leptos::html::Button>::new();

    let focus_button = move || {
        if let Some(button) = button.get_untracked() {
            let _ = button.focus();
        }
    };
    let toggle = move |_| {
        let opening = !open.get_untracked();
        if opening {
            // The menu opens from the bar's gold line, so the bar must be
            // where it is at the top of the page.
            browser::jump_to_top();
        }
        open.set(opening);
        if opening {
            // Into the menu once it shows.
            request_animation_frame(|| browser::focus_first("#site-menu a"));
        }
    };

    // Escape closes the menu and returns to the emblem.
    let on_key = window_event_listener(ev::keydown, move |event| {
        if event.key() == "Escape" && open.get_untracked() {
            open.set(false);
            focus_button();
        }
    });
    // Widening the window to the whole bar closes the menu, which isn't
    // shown there.
    let on_resize = window_event_listener(ev::resize, move |_| {
        if open.get_untracked() && browser::window_width() >= WIDE_FROM {
            open.set(false);
        }
    });
    on_cleanup(move || {
        on_key.remove();
        on_resize.remove();
    });

    let links = move |in_menu: bool| {
        // Choosing a page in the menu closes it; the router then opens the
        // page (or, for the page you're on, goes to its top).
        let chosen = move |_| {
            if in_menu {
                open.set(false);
                request_animation_frame(|| browser::focus_quietly("top"));
            }
        };
        pages::PAGES
            .into_iter()
            .map(|(page, address)| {
                view! {
                    <li on:click=chosen>
                        <A href=address exact=true>
                            {page}
                        </A>
                    </li>
                }
            })
            .collect_view()
    };

    view! {
        // Back to top and every move to another page bring keyboard focus
        // here, so Tab carries on from the top.
        <nav class="site-nav" id="top" tabindex="-1" aria-label="Site">
            <div class="nav-bar">
                <div class="nav-inner">
                    <a class="site-name" href=pages::HOME>
                        {name_words(&shop.shop.name)}
                    </a>
                    <ul class="nav-links">{links(false)}</ul>
                </div>
            </div>
            // Outside the bar, so the emblem can sit above the menu while the
            // menu's frame covers the bar's gold line.
            <span class="emblem" aria-hidden="true" inner_html=emblem_art("nav-")></span>
            // The emblem leads home where the whole bar shows, and opens the
            // menu where it doesn't (main.css shows one or the other).
            <a class="emblem-home" href=pages::HOME aria-label="Go to the homepage"></a>
            <button
                type="button"
                class="emblem-menu"
                node_ref=button
                aria-controls="site-menu"
                aria-expanded=move || if open.get() { "true" } else { "false" }
                aria-label=move || if open.get() { "Close the menu" } else { "Open the menu" }
                on:click=toggle
            ></button>
            <div class="nav-drawer" id="site-menu" class:open=move || open.get()>
                <div class="menu-panel">
                    <span class="corners" aria-hidden="true" inner_html=CORNERS></span>
                    <p class="menu-name">{name_words(&shop.shop.name)}</p>
                    <span
                        class="divider-line"
                        aria-hidden="true"
                        inner_html=divider_art("menu-")
                    ></span>
                    <ul class="menu-links">{links(true)}</ul>
                </div>
            </div>
        </nav>
        // Keeps the page clear of the emblem hanging below the bar.
        <div class="nav-space" aria-hidden="true"></div>
    }
}
