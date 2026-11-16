//! Aether & Allure: an FFXIV crafting shop, written in Rust and compiled to
//! WebAssembly with Leptos. Everything it shows comes from shop.toml.

mod app;
mod browser;
mod components;
mod model;
mod order;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
