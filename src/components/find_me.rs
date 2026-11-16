//! Where to find the retainer in game, with directions.

use super::copy_button::CopyButton;
use crate::model::Shop;
use leptos::prelude::*;

#[component]
pub fn FindMe(shop: &'static Shop) -> impl IntoView {
    let loc = &shop.location;
    let district = loc.district;
    let spot = loc
        .spot
        .clone()
        .unwrap_or_else(|| "on the plot".to_string());

    view! {
        <section class="window find" aria-labelledby="find-title">
            <header class="window-title">
                <h2 id="find-title">"Find my retainer"</h2>
            </header>
            <div class="window-body">
                <div class="plaque">
                    <p class="plaque-district">{district.name()}</p>
                    <p class="plaque-plot">
                        "Ward " <b>{loc.ward}</b> ", Plot " <b>{loc.plot}</b>
                        {loc.subdivision().then_some(" (Subdivision)")}
                    </p>
                    <p class="plaque-world">
                        {loc.world.clone()} ", " {loc.data_center.clone()} " data center"
                    </p>
                </div>

                <dl class="facts">
                    <dt>"Retainer"</dt>
                    <dd>
                        <strong>{loc.retainer.clone()}</strong>
                        ", "
                        {spot.clone()}
                    </dd>
                    {loc
                        .size
                        .clone()
                        .map(|size| {
                            view! {
                                <dt>"Estate"</dt>
                                <dd>{size} " house"</dd>
                            }
                        })}
                    <dt>"Shopkeeper"</dt>
                    <dd>{shop.shop.owner.clone()}</dd>
                </dl>

                <div class="block">
                    <h3>"Getting there"</h3>
                    <ol class="steps">
                        <li>
                            "On another world? Use " <em>"Travel to Another World"</em>
                            " at any main aetheryte to visit " {loc.world.clone()} "."
                        </li>
                        <li>"Teleport to " {district.aetheryte()} "."</li>
                        <li>
                            "At the aetheryte, choose " <em>"Residential District Aethernet"</em>
                            ", then " <em>"Go to specified ward"</em> ", and pick Ward " {loc.ward}
                            "."
                        </li>
                        {loc
                            .subdivision()
                            .then(|| {
                                view! {
                                    <li>
                                        "You arrive in the main half of the ward. Use an aethernet shard there to cross to the subdivision."
                                    </li>
                                }
                            })}
                        <li>
                            "Head to plot " {loc.plot} ". " {loc.retainer.clone()} " is " {spot} "."
                        </li>
                    </ol>
                </div>
                {loc.note.clone().map(|note| view! { <p class="small">{note}</p> })}
                <div>
                    <CopyButton text=shop.address() label="Copy address" />
                </div>
            </div>
        </section>
    }
}
