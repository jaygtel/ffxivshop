//! The opening screen: the hero drawing (assets/hero.svg: the crystal emblem,
//! «title», name and tagline), an Order Now link down to the shop and whether
//! orders are open. Centred and a full screen tall. (The crafting jobs sit
//! just above the wares: see crafters.rs.)

use crate::model::Shop;
use leptos::prelude::*;

/// The same file that's published as /hero.svg, so the two never drift apart.
/// It picks its own layout for the screen, and takes its night-sky colours
/// from the page's dark colour scheme. Its words are part of the drawing, so
/// screen readers get the name, title and tagline from shop.toml instead.
const HERO: &str = include_str!("../../assets/hero.svg");

#[component]
pub fn Hero(shop: &'static Shop) -> impl IntoView {
    let info = &shop.shop;

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
        </header>
    }
}
