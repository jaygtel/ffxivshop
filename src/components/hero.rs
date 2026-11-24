//! The opening screen, built like the Aether & Allure coming-soon page: the
//! crystal emblem floating over a panel of dark glass framed in gold, holding
//! «title», the shop's name, the divider, whether orders are open, the
//! tagline and status note, and an Order Now link to the shop page. Centred
//! and a full screen tall: the homepage is this hero. When the homepage
//! opens, assets/js/intro.js writes the name in aether fire and the rest
//! gathers around it, as on the coming-soon page (not with reduced motion).
//! The crafting jobs sit just above the wares: see crafters.rs.

use crate::model::Shop;
use crate::pages;
use leptos::prelude::*;

// EMBLEM, CORNERS and DIVIDER are Aether & Allure artwork, not code: all
// rights reserved, and not covered by the BSD licence (see docs/NOTICE.md).
// If you reuse this file, draw your own in their place. The navigation and
// its menu (site_nav.rs) show them too, through the functions below.

/// The emblem, as on the coming-soon page: the night disc, the turning gold
/// and aether rings, and the floating crystal with its glint.
const EMBLEM: &str = r##"<div class="disc"></div><svg class="ring" viewBox="0 0 200 200" fill="none"><circle cx="100" cy="100" r="97" stroke="#d9b45a" stroke-opacity=".45" stroke-width=".8"/><circle cx="100" cy="100" r="90" stroke="#d9b45a" stroke-opacity=".55" stroke-width=".7" stroke-dasharray="1.2 5.8"/><g fill="#f0d68c"><path d="M100 0 l4 4 l-4 4 l-4 -4 z"/><path d="M200 100 l-4 4 l-4 -4 l4 -4 z"/><path d="M100 200 l-4 -4 l4 -4 l4 4 z"/><path d="M0 100 l4 -4 l4 4 l-4 4 z"/></g><g fill="#d9b45a" fill-opacity=".8"><circle cx="168.6" cy="31.4" r="1.4"/><circle cx="168.6" cy="168.6" r="1.4"/><circle cx="31.4" cy="168.6" r="1.4"/><circle cx="31.4" cy="31.4" r="1.4"/></g></svg><svg class="ring inner" viewBox="0 0 200 200" fill="none"><circle cx="100" cy="100" r="96" stroke="#7fd8ff" stroke-opacity=".28" stroke-width="1" stroke-dasharray="26 10 3 10"/></svg><svg class="crystal" viewBox="0 0 100 170"><defs><linearGradient id="fTL" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#f2fbff"/><stop offset="1" stop-color="#8fd4ff"/></linearGradient><linearGradient id="fTR" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#c2eaff"/><stop offset="1" stop-color="#3e94e2"/></linearGradient><linearGradient id="fBL" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#63b8f2"/><stop offset="1" stop-color="#1c4fa0"/></linearGradient><linearGradient id="fBR" x1="0" y1="0" x2="1" y2="1"><stop offset="0" stop-color="#2f84d4"/><stop offset="1" stop-color="#0f2a69"/></linearGradient></defs><path d="M50 2 L12 68 L50 84 Z" fill="url(#fTL)"/><path d="M50 2 L50 84 L88 68 Z" fill="url(#fTR)"/><path d="M12 68 L50 84 L50 168 Z" fill="url(#fBL)"/><path d="M50 84 L88 68 L50 168 Z" fill="url(#fBR)"/><path d="M50 2 L50 168 M12 68 L50 84 L88 68" stroke="#ffffff" stroke-opacity=".55" stroke-width=".8" fill="none"/><path d="M50 2 L12 68 L50 168 L88 68 Z" stroke="#e6f6ff" stroke-opacity=".7" stroke-width="1" fill="none" stroke-linejoin="round"/><path d="M36 30 l3 -7 l3 7 l-3 7 z" fill="#ffffff" fill-opacity=".85"/><path class="glint" d="M50 -6 l2 6 l6 2 l-6 2 l-2 6 l-2 -6 l-6 -2 l6 -2 z" fill="#ffffff"/></svg>"##;

/// The panel's gold corner pieces.
pub(super) const CORNERS: &str = r##"<svg class="corner tl" viewBox="0 0 30 30" fill="none"><path d="M1.5 29 V9.5 L9.5 1.5 H29" stroke="currentColor" stroke-width="1.3"/><path d="M5.5 5.5 l3 -3 l3 3 l-3 3 z" fill="currentColor"/></svg><svg class="corner tr" viewBox="0 0 30 30" fill="none"><path d="M1.5 29 V9.5 L9.5 1.5 H29" stroke="currentColor" stroke-width="1.3"/><path d="M5.5 5.5 l3 -3 l3 3 l-3 3 z" fill="currentColor"/></svg><svg class="corner bl" viewBox="0 0 30 30" fill="none"><path d="M1.5 29 V9.5 L9.5 1.5 H29" stroke="currentColor" stroke-width="1.3"/><path d="M5.5 5.5 l3 -3 l3 3 l-3 3 z" fill="currentColor"/></svg><svg class="corner br" viewBox="0 0 30 30" fill="none"><path d="M1.5 29 V9.5 L9.5 1.5 H29" stroke="currentColor" stroke-width="1.3"/><path d="M5.5 5.5 l3 -3 l3 3 l-3 3 z" fill="currentColor"/></svg>"##;

/// The divider under the name.
const DIVIDER: &str = r##"<svg class="divider" viewBox="0 0 300 22"><defs><linearGradient id="dl" gradientUnits="userSpaceOnUse" x1="0" y1="11" x2="124" y2="11"><stop offset="0" stop-color="#d9b45a" stop-opacity="0"/><stop offset="1" stop-color="#d9b45a"/></linearGradient><linearGradient id="dr" gradientUnits="userSpaceOnUse" x1="300" y1="11" x2="176" y2="11"><stop offset="0" stop-color="#d9b45a" stop-opacity="0"/><stop offset="1" stop-color="#d9b45a"/></linearGradient></defs><path d="M0 11 H124" stroke="url(#dl)" stroke-width="1"/><path d="M176 11 H300" stroke="url(#dr)" stroke-width="1"/><circle cx="130" cy="11" r="1.6" fill="#d9b45a"/><circle cx="170" cy="11" r="1.6" fill="#d9b45a"/><path d="M150 1 L158 11 L150 21 L142 11 Z" fill="none" stroke="#f0d68c" stroke-width="1.1"/><path d="M150 6 L154 11 L150 16 L146 11 Z" fill="#7fd8ff"/><path d="M139 11 L143 9 L143 13 Z" fill="#d9b45a"/><path d="M161 11 L157 9 L157 13 Z" fill="#d9b45a"/></svg>"##;

/// The emblem, for another copy on the same page (the navigation's), with
/// `prefix` put before each of its gradients' ids, so the copies don't share
/// ids.
pub(super) fn emblem_art(prefix: &str) -> String {
    EMBLEM
        .replace("id=\"f", &format!("id=\"{prefix}f"))
        .replace("url(#f", &format!("url(#{prefix}f"))
}

/// The divider, for another copy on the same page (the menu's), renamed in
/// the same way.
pub(super) fn divider_art(prefix: &str) -> String {
    DIVIDER
        .replace("id=\"d", &format!("id=\"{prefix}d"))
        .replace("url(#d", &format!("url(#{prefix}d"))
}

/// The shop's name, set like the coming-soon title: the words in Cinzel, an
/// ampersand in italic Cormorant. Each word is its own span; intro.js and the
/// phone layout (a word per line) both rely on that. The navigation and its
/// menu set the name the same way.
pub(super) fn name_words(name: &str) -> impl IntoView + use<> {
    name.split_whitespace()
        .enumerate()
        .map(|(i, word)| {
            let class = if word == "&" { "amp" } else { "w" };
            view! {
                {(i != 0).then_some(" ")}
                <span class=class>{word.to_string()}</span>
            }
        })
        .collect_view()
}

#[component]
pub fn Hero(shop: &'static Shop) -> impl IntoView {
    let info = &shop.shop;
    let name = name_words(&info.name);
    let has_lore = info.tagline.is_some() || info.status_note.is_some();

    view! {
        <header class="hero">
            <div class="stage">
                <div class="emblem" aria-hidden="true" inner_html=EMBLEM></div>
                <div class="panel">
                    <span class="corners" aria-hidden="true" inner_html=CORNERS></span>
                    {info.title.clone().map(|title| view! { <p class="overline">{title}</p> })}
                    <h1 id="title">{name}</h1>
                    <span class="divider-line" aria-hidden="true" inner_html=DIVIDER></span>
                    // Whether orders are open, where the coming-soon page says
                    // "Coming soon".
                    <p class=format!("soon {}", info.status.class())>
                        <span class="status-dot" aria-hidden="true"></span>
                        {info.status.label()}
                    </p>
                    {has_lore
                        .then(|| {
                            view! {
                                <p class="lore">
                                    {info.tagline.clone()}
                                    {info
                                        .status_note
                                        .clone()
                                        .map(|note| view! { <span class="call">{note}</span> })}
                                </p>
                            }
                        })}
                    // A plain link to the shop page. The router opens it
                    // without reloading the site, so the hero's opening
                    // doesn't play again on the way back.
                    <a class="btn btn-primary hero-cta" href=pages::SHOP>
                        "Order Now"
                    </a>
                </div>
            </div>
        </header>
    }
}

#[cfg(test)]
mod tests {
    use super::{DIVIDER, EMBLEM, divider_art, emblem_art};

    #[test]
    fn the_copies_have_their_own_ids() {
        let emblem = emblem_art("nav-");
        for id in ["fTL", "fTR", "fBL", "fBR"] {
            assert!(EMBLEM.contains(&format!("id=\"{id}\"")));
            assert!(emblem.contains(&format!("id=\"nav-{id}\"")));
            assert!(emblem.contains(&format!("url(#nav-{id})")));
            assert!(!emblem.contains(&format!("\"{id}\"")));
            assert!(!emblem.contains(&format!("#{id})")));
        }
        let divider = divider_art("menu-");
        for id in ["dl", "dr"] {
            assert!(DIVIDER.contains(&format!("id=\"{id}\"")));
            assert!(divider.contains(&format!("id=\"menu-{id}\"")));
            assert!(divider.contains(&format!("url(#menu-{id})")));
        }
        // Nothing else changes.
        assert_eq!(emblem.replace("nav-", ""), EMBLEM);
        assert_eq!(divider.replace("menu-", ""), DIVIDER);
    }
}
