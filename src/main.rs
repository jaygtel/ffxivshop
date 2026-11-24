//! Aether & Allure: an FFXIV crafting shop, written in Rust and compiled to
//! WebAssembly with Leptos. Everything it shows comes from shop.toml.

mod app;
mod basket;
mod browser;
mod components;
mod message;
mod model;
mod order;
mod pages;
mod prices;
// Fills in shop.toml's ${SHOP_…} values. build.rs uses it; it's compiled here
// only so its tests run with `cargo test`.
#[cfg(test)]
mod vars;

use app::App;
use leptos::prelude::*;

fn main() {
    console_error_panic_hook::set_once();
    // The market prices come first, so the page never shows one price and
    // then another. Without them, shop.toml's prices are used.
    wasm_bindgen_futures::spawn_local(async {
        let prices = match browser::fetch_text(prices::ADDRESS, prices::WAIT_MS)
            .await
            .map(|text| prices::read(&text))
        {
            Some(Ok(prices)) => prices,
            Some(Err(problem)) => {
                leptos::logging::warn!(
                    "{} isn't readable ({problem}); using shop.toml's prices",
                    prices::ADDRESS
                );
                None
            }
            None => None,
        };
        leptos::mount::mount_to_body(move || view! { <App prices /> });
    });
}
