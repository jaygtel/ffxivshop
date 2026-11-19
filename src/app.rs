use crate::browser;
use crate::components::{
    AetherSky, BackToTop, ContactCard, Crafters, FindMe, Footer, Hero, OrderSlip, Wares,
};
use crate::model::{EDB_TOOLTIP_SCRIPT, Shop};
use crate::order::Order;
use leptos::prelude::*;

/// shop.toml is built into the site, with its ${SHOP_…} values filled in from
/// .env by build.rs, which has already checked it.
const SHOP_TOML: &str = include_str!(concat!(env!("OUT_DIR"), "/shop.toml"));

/// The ${SHOP_…} values that fell back to their examples because .env didn't
/// set them, such as "SHOP_DISCORD,SHOP_EMAIL" (build.rs works this out).
const EXAMPLES_IN_USE: &str = env!("EXAMPLES_IN_USE");

#[component]
pub fn App() -> impl IntoView {
    match Shop::from_toml(SHOP_TOML) {
        Ok(shop) => {
            // The shop's details never change while the page is open, so they
            // live for the whole visit and every part of the page can borrow them.
            let shop: &'static Shop = Box::leak(Box::new(shop));
            view! { <Storefront shop /> }.into_any()
        }
        Err(problems) => view! { <SetupProblem problems /> }.into_any(),
    }
}

#[component]
fn Storefront(shop: &'static Shop) -> impl IntoView {
    browser::set_title(&shop.shop.name);
    let order = Order::new(shop);

    if shop.shop.tooltips {
        // Square Enix's script looks for database links when it loads, so it
        // goes in after the item list is on the page.
        request_animation_frame(|| browser::load_script(EDB_TOOLTIP_SCRIPT));
    }

    view! {
        <AetherSky />
        <Hero shop />
        // The hero's Order Now link scrolls here.
        <div class="page" id="shop">
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

/// Only seen if shop.toml is broken in a way build.rs couldn't catch.
#[component]
fn SetupProblem(problems: String) -> impl IntoView {
    view! {
        <main class="page">
            <section class="window">
                <header class="window-title">
                    <h1>"shop.toml needs fixing"</h1>
                </header>
                <div class="window-body">
                    <pre class="problems">{problems}</pre>
                </div>
            </section>
        </main>
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
