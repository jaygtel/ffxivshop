//! Aether & Allure: an FFXIV crafting shop, written in Rust and compiled to
//! WebAssembly with Leptos. Everything it shows comes from shop.toml.

mod app;
mod browser;
mod components;
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
        let prices = prices::fetch().await;
        leptos::mount::mount_to_body(move || view! { <App prices /> });
    });
}
