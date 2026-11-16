//! The top of the page, styled after the nameplate that floats over an NPC:
//! a «title» above the name, with the monogram beside it like a company crest.

use crate::model::{CRAFTERS, Shop};
use leptos::prelude::*;

/// The same file that's published as /logo.svg, so the two never drift apart.
/// The page sets its colours through the --aa-* custom properties.
const MONOGRAM: &str = include_str!("../../assets/logo.svg");

#[component]
pub fn Nameplate(shop: &'static Shop) -> impl IntoView {
    let info = &shop.shop;
    let location = &shop.location;
    let crafters: Vec<(&str, &str, u16)> = CRAFTERS
        .iter()
        .filter_map(|(abbr, job)| info.crafters.get(*abbr).map(|level| (*abbr, *job, *level)))
        .collect();

    view! {
        <header class="nameplate">
            <div class="np-crest" aria-hidden="true" inner_html=MONOGRAM></div>
            <div class="np-heading">
                {info
                    .title
                    .clone()
                    .map(|title| view! { <p class="np-title">"«" {title} "»"</p> })}
                <h1 class="np-name">{info.name.clone()}</h1>
                {info.tagline.clone().map(|line| view! { <p class="np-tagline">{line}</p> })}
            </div>
            <div class="np-facts">
                <p class=format!("status {}", info.status.class())>
                    <span class="status-dot" aria-hidden="true"></span>
                    <span class="status-label">{info.status.label()}</span>
                </p>
                <p class="np-who">
                    {info.owner_role.clone()} " " <strong>{info.owner.clone()}</strong> " on "
                    {location.world.clone()} " (" {location.data_center.clone()}
                    " data center). Retainer " <strong>{location.retainer.clone()}</strong>
                    " is in " {location.district.name()} ", " {location.ward_and_plot()} "."
                </p>
            </div>
            {info.status_note.clone().map(|note| view! { <p class="np-note">{note}</p> })}
            {(!crafters.is_empty())
                .then(|| {
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
                })}
        </header>
    }
}
