//! The shop page: the sample-shop notice, the crafting jobs, the wares and
//! the order slip, then Request a craft and Collection, and the footer, with
//! Back to top in the corner.
//!
//! Once opened, the shop page stays in the site, hidden while another page
//! shows, so the search and sort are as you left them when you come back.
//! That also keeps the Eorzea Database tooltips working: Square Enix's
//! script attaches to the item links once, when it loads, so the links have
//! to stay. (The route for its address in src/app.rs draws nothing; this
//! page shows itself.)

use super::SHOP;
use crate::browser;
use crate::components::{BackToTop, ContactCard, Crafters, FindMe, Footer, OrderSlip, Wares};
use crate::model::{EDB_TOOLTIP_SCRIPT, Shop};
use crate::order::Order;
use leptos::prelude::*;
use leptos_router::hooks::use_location;

/// The ${SHOP_…} values that fell back to their examples because .env didn't
/// set them, such as "SHOP_DISCORD,SHOP_EMAIL" (build.rs works this out).
const EXAMPLES_IN_USE: &str = env!("EXAMPLES_IN_USE");

#[component]
pub fn ShopPage(shop: &'static Shop, order: Order) -> impl IntoView {
    let location = use_location();
    let showing = Memo::new(move |_| location.pathname.with(|path| path == SHOP));
    // Once it has been shown, it stays.
    let opened =
        Memo::new(move |before: Option<&bool>| before.copied().unwrap_or(false) || showing.get());

    move || {
        opened.get().then(|| {
            if shop.shop.tooltips {
                // Square Enix's script looks for database links when it
                // loads, so it goes in after the item list is on the page.
                request_animation_frame(|| browser::load_script(EDB_TOOLTIP_SCRIPT));
            }
            view! {
                <div class="shop-page" hidden=move || !showing.get()>
                    <div class="page">
                        <h1 class="sr-only">"Shop"</h1>
                        {shop
                            .shop
                            .demo
                            .then(|| {
                                view! {
                                    <p class="demo-notice" role="note">
                                        <strong>"Sample shop."</strong>
                                        {sample_notice(EXAMPLES_IN_USE)}
                                    </p>
                                }
                            })}
                        <main class="market">
                            // The crafting jobs sit just above the wares window.
                            <Crafters shop />
                            <Wares shop order />
                            <OrderSlip shop order />
                        </main>
                        // Request a craft comes before Collection: ordering, then collecting.
                        <div class="visit">
                            <ContactCard shop />
                            <FindMe shop />
                        </div>
                        <Footer shop />
                    </div>
                    <BackToTop />
                </div>
            }
        })
    }
}

/// What the sample-shop notice says after "Sample shop.". The contact details
/// are only called examples while they are: once .env sets them, they're real.
fn sample_notice(examples_in_use: &str) -> String {
    let example = |name: &str| examples_in_use.split(',').any(|used| used == name);
    let contact = match (example("SHOP_DISCORD"), example("SHOP_EMAIL")) {
        (true, true) => Some("contact details"),
        (true, false) => Some("Discord name"),
        (false, true) => Some("email address"),
        (false, false) => None,
    };
    match contact {
        Some(contact) => format!(
            " The listings, prices and {contact} here are examples. Edit shop.toml and .env to make them yours, then set demo = false."
        ),
        None => " The listings and prices here are examples. Edit shop.toml to make them yours, then set demo = false."
            .to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::sample_notice;

    #[test]
    fn the_notice_calls_only_the_examples_examples() {
        // .env set both contacts, so only the listings and prices are examples.
        assert_eq!(
            sample_notice(""),
            " The listings and prices here are examples. Edit shop.toml to make them yours, then set demo = false."
        );
        assert_eq!(
            sample_notice("SHOP_DISCORD,SHOP_EMAIL"),
            " The listings, prices and contact details here are examples. Edit shop.toml and .env to make them yours, then set demo = false."
        );
        assert!(sample_notice("SHOP_DISCORD").contains(" prices and Discord name here "));
        assert!(sample_notice("SHOP_EMAIL").contains(" prices and email address here "));
        // A value of your own that's still an example doesn't touch the contacts.
        assert_eq!(sample_notice("SHOP_LINKSHELL"), sample_notice(""));
        assert_eq!(sample_notice("SHOP_EMAILS"), sample_notice(""));
    }
}
