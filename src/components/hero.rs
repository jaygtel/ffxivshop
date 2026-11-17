//! The opening screen: the hero drawing (assets/hero.svg: the monogram,
//! «title», name and tagline), an Order Now link down to the shop, whether
//! orders are open, and the crafting jobs. Centred and a full screen tall.

use crate::model::{CRAFTERS, Shop};
use leptos::prelude::*;

/// The same file that's published as /hero.svg, so the two never drift apart.
/// It picks its own layout for the screen, and the page sets its colours
/// through the --aa-* custom properties. Its words are part of the drawing, so
/// screen readers get the name, title and tagline from shop.toml instead.
const HERO: &str = include_str!("../../assets/hero.svg");

#[component]
pub fn Hero(shop: &'static Shop) -> impl IntoView {
    let info = &shop.shop;
    let crafters: Vec<(&str, &str, u16)> = CRAFTERS
        .iter()
        .filter_map(|(abbr, job)| info.crafters.get(*abbr).map(|level| (*abbr, *job, *level)))
        .collect();

    view! {
        // Back to top scrolls here and moves keyboard focus here.
        <header class="hero" id="top" tabindex="-1">
            <h1 class="hero-title">
                <span class="sr-only">{info.name.clone()}</span>
                <span class="hero-art" aria-hidden="true" inner_html=HERO></span>
            </h1>
            {(info.title.is_some() || info.tagline.is_some())
                .then(|| {
                    view! {
                        <p class="sr-only">
                            {info.title.clone().map(|title| format!("«{title}» "))}
                            {info.tagline.clone()}
                        </p>
                    }
                })}
            <div class="hero-actions">
                // A plain link to the shop below, so it also works as a link
                // (and the wares tabs treat the #shop address as "All").
                <a class="btn btn-primary hero-cta" href="#shop">
                    "Order Now"
                </a>
                <p class=format!("status {}", info.status.class())>
                    <span class="status-dot" aria-hidden="true"></span>
                    <span class="status-label">{info.status.label()}</span>
                </p>
            </div>
            {info.status_note.clone().map(|note| view! { <p class="hero-note">{note}</p> })}
            {(!crafters.is_empty())
                .then(|| {
                    view! {
                        <ul class="crafters" aria-label="Crafting and gathering jobs">
                            {crafters
                                .into_iter()
                                .map(|(abbr, job, level)| {
                                    view! {
                                        <li title=format!("{job}, level {level}")>
                                            <abbr title=job>{abbr}</abbr>
                                            <span class="lvl">{level}</span>
                                        </li>
                                    }
                                })
                                .collect_view()}
                        </ul>
                    }
                })}
        </header>
    }
}
