//! The crafting and gathering jobs with their levels, shown just above the
//! wares window.

use crate::model::{CRAFTERS, Shop};
use leptos::prelude::*;

#[component]
pub fn Crafters(shop: &'static Shop) -> impl IntoView {
    let crafters: Vec<(&str, &str, u16)> = CRAFTERS
        .iter()
        .filter_map(|(abbr, job)| {
            shop.shop
                .crafters
                .get(*abbr)
                .map(|level| (*abbr, *job, *level))
        })
        .collect();

    (!crafters.is_empty()).then(|| {
        view! {
            <ul class="crafters" aria-label="Crafting and gathering jobs">
                {crafters
                    .into_iter()
                    .map(|(abbr, job, level)| {
                        view! {
                            <li title=format!("{job}, level {level}")>
                                <abbr title=job>{abbr}</abbr>
                                <span class="lvl">{level}</span>
                            </li>
                        }
                    })
                    .collect_view()}
            </ul>
        }
    })
}
