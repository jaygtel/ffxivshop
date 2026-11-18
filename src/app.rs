use crate::browser;
use crate::components::{
    AetherSky, BackToTop, ContactCard, FindMe, Footer, Hero, OrderSlip, Wares,
};
use crate::model::{EDB_TOOLTIP_SCRIPT, Shop};
use crate::order::Order;
use leptos::prelude::*;

/// shop.toml is built into the site. build.rs has already checked it.
const SHOP_TOML: &str = include_str!("../shop.toml");

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
                            " The listings, prices and contact details here are examples. Edit shop.toml to make them yours, then set demo = false."
                        </p>
                    }
                })}
            <main class="market">
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
