//! Who owns what: the site design, the shop in game, and the game itself.
//! Also a link to the contact page, the one way to it from the homepage,
//! which has no navigation bar.

use crate::model::Shop;
use crate::pages;
use leptos::prelude::*;

#[component]
pub fn Footer(shop: &'static Shop) -> impl IntoView {
    let this_year = js_sys::Date::new_0().get_full_year() as u16;
    let site_address = shop.site.address.clone();
    view! {
        <footer class="footer">
            <p class="footer-copyright">
                "Website design " {shop.copyright_notice(this_year)} ". All rights reserved."
            </p>
            <p>
                {shop.shop.name.clone()} " is a player-run shop in FINAL FANTASY XIV, "
                {shop.shop.owner_role.to_lowercase()} " " {shop.shop.owner.clone()} "."
            </p>
            <p>
                "FINAL FANTASY XIV \u{a9} SQUARE ENIX CO., LTD. All Rights Reserved. "
                "FINAL FANTASY is a registered trademark of Square Enix Holdings Co., Ltd. "
                "This is an unofficial fan site and isn't affiliated with or endorsed by Square Enix. "
                "Item links open The Lodestone's Eorzea Database; item icons are served by XIVAPI."
            </p>
            <p class="footer-small">
                {site_address
                    .map(|address| {
                        let label = address
                            .trim_start_matches("https://")
                            .trim_start_matches("http://")
                            .trim_end_matches('/')
                            .to_string();
                        view! {
                            <a href=address>{label}</a>
                            ". "
                        }
                    })} <a href=pages::CONTACT>"Contact"</a> ". Built with Rust and WebAssembly."
            </p>
        </footer>
    }
}
