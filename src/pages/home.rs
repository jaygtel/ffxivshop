//! The homepage: the hero, a full screen tall, with the footer out of sight
//! below it. It has no navigation bar: the hero already carries the emblem
//! and the name, and Order Now leads to the shop, so nothing distracts from
//! the hero's opening.

use crate::components::{Footer, Hero};
use crate::model::Shop;
use leptos::prelude::*;

#[component]
pub fn HomePage(shop: &'static Shop) -> impl IntoView {
    view! {
        <main class="home">
            <Hero shop />
        </main>
        <div class="page home-footer">
            <Footer shop />
        </div>
    }
}
