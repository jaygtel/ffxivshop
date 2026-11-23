//! The shop window: category tabs, search, sorting and the item list.
//!
//! Every item stays on the page; filtering only hides rows. That keeps the
//! Eorzea Database tooltips working, because Square Enix's script attaches
//! to the links once when it loads.
//!
//! In `view!` attributes, avoid `<` and `>` comparisons (`n > 0`): the macro
//! reads `>` as the end of the tag, and leptosfmt drops the braces that would
//! protect it. Write `n != 0` instead.

use super::glyphs::{ClockIcon, GilIcon, GlyphIcon, SearchIcon};
use crate::browser;
use crate::model::{Shop, Soon, gil, icon_url};
use crate::order::Order;
use leptos::ev;
use leptos::prelude::*;
use leptos_router::hooks::use_location;
use std::cmp::Reverse;

#[derive(Clone, Copy, PartialEq, Eq)]
enum SortBy {
    Featured,
    PriceLow,
    PriceHigh,
    Soonest,
    ItemLevel,
    Name,
}

impl SortBy {
    const ALL: [SortBy; 6] = [
        SortBy::Featured,
        SortBy::PriceLow,
        SortBy::PriceHigh,
        SortBy::Soonest,
        SortBy::ItemLevel,
        SortBy::Name,
    ];

    fn key(self) -> &'static str {
        match self {
            SortBy::Featured => "featured",
            SortBy::PriceLow => "price-low",
            SortBy::PriceHigh => "price-high",
            SortBy::Soonest => "soonest",
            SortBy::ItemLevel => "ilvl",
            SortBy::Name => "name",
        }
    }

    fn label(self) -> &'static str {
        match self {
            SortBy::Featured => "Shop order",
            SortBy::PriceLow => "Price, low to high",
            SortBy::PriceHigh => "Price, high to low",
            SortBy::Soonest => "Ready soonest",
            SortBy::ItemLevel => "Item level",
            SortBy::Name => "Name",
        }
    }

    fn from_key(key: &str) -> SortBy {
        SortBy::ALL
            .into_iter()
            .find(|s| s.key() == key)
            .unwrap_or(SortBy::Featured)
    }
}

#[component]
pub fn Wares(shop: &'static Shop, order: Order) -> impl IntoView {
    let items = &shop.items;
    let known = move |key: String| {
        if shop.category(&key).is_some() {
            key
        } else {
            String::new()
        }
    };

    // "" means All. The tab follows the address, so /shop#housing links work:
    // when the part after # changes (Back and Forward, say), and whenever the
    // site moves to another address, such as the navigation's Shop link,
    // which has no # (and the router changes it without a hashchange).
    let tab = RwSignal::new(known(browser::hash()));
    let listener = window_event_listener(ev::hashchange, move |_| tab.set(known(browser::hash())));
    on_cleanup(move || listener.remove());
    let location = use_location();
    Effect::new(move |_| {
        location.pathname.track();
        location.hash.track();
        let now = known(browser::hash());
        if tab.with_untracked(|t| *t != now) {
            tab.set(now);
        }
    });

    let query = RwSignal::new(String::new());
    let sort = RwSignal::new(SortBy::Featured);

    let matches = move |index: usize| {
        let item = &items[index];
        let in_tab = tab.with(|t| t.is_empty() || *t == item.category);
        in_tab
            && query.with(|q| {
                let q = q.trim().to_lowercase().replace('\u{2019}', "'");
                q.is_empty()
                    || item.name.to_lowercase().contains(&q)
                    || item
                        .jobs
                        .as_deref()
                        .is_some_and(|j| j.to_lowercase().contains(&q))
                    || shop
                        .category(&item.category)
                        .is_some_and(|c| c.label.to_lowercase().contains(&q))
            })
    };
    let shown = Memo::new(move |_| (0..items.len()).filter(|i| matches(*i)).count());

    let sorted = Memo::new(move |_| {
        let mut order: Vec<usize> = (0..items.len()).collect();
        // Stable sorts, so ties keep the order from shop.toml.
        match sort.get() {
            SortBy::Featured => {}
            SortBy::PriceLow => {
                order.sort_by_key(|i| (items[*i].price.is_none(), items[*i].price.unwrap_or(0)))
            }
            SortBy::PriceHigh => order.sort_by_key(|i| {
                (
                    items[*i].price.is_none(),
                    Reverse(items[*i].price.unwrap_or(0)),
                )
            }),
            SortBy::Soonest => order.sort_by_key(|i| shop.ready_of(&items[*i])),
            SortBy::ItemLevel => order.sort_by_key(|i| Reverse(items[*i].ilvl.unwrap_or(0))),
            SortBy::Name => order.sort_by(|a, b| items[*a].name.cmp(&items[*b].name)),
        }
        order
    });

    let tabs: Vec<(String, String, usize)> =
        std::iter::once((String::new(), "All".to_string(), items.len()))
            .chain(shop.categories.iter().filter_map(|c| {
                let n = items.iter().filter(|item| item.category == c.key).count();
                (n > 0).then(|| (c.key.clone(), c.label.clone(), n))
            }))
            .collect();

    view! {
        <section class="window wares" aria-labelledby="wares-title">
            <header class="window-title">
                <h2 id="wares-title">"Wares"</h2>
                <span class="count" aria-live="polite">
                    {move || {
                        let n = shown.get();
                        if n == items.len() {
                            format!("{n} items")
                        } else {
                            format!("{n} of {} items", items.len())
                        }
                    }}
                </span>
            </header>

            <nav class="tabs" aria-label="Categories">
                {tabs
                    .into_iter()
                    .map(|(key, label, n)| {
                        let selected = {
                            let key = key.clone();
                            move || tab.with(|t| *t == key)
                        };
                        let pick = {
                            let key = key.clone();
                            move |_| {
                                tab.set(key.clone());
                                browser::set_hash(&key);
                            }
                        };
                        view! {
                            <button
                                type="button"
                                class="tab"
                                aria-pressed=move || if selected() { "true" } else { "false" }
                                on:click=pick
                            >
                                {label}
                                <span class="n">{n}</span>
                            </button>
                        }
                    })
                    .collect_view()}
            </nav>

            <div class="toolbar">
                <div class="search">
                    <label class="sr-only" for="ware-search">
                        "Search wares"
                    </label>
                    <SearchIcon />
                    <input
                        id="ware-search"
                        type="search"
                        placeholder="Search by name or job"
                        autocomplete="off"
                        prop:value=move || query.get()
                        on:input=move |ev| query.set(event_target_value(&ev))
                    />
                </div>
                <div class="sort">
                    <label for="ware-sort">"Sort by"</label>
                    <select
                        id="ware-sort"
                        prop:value=move || sort.get().key()
                        on:change=move |ev| sort.set(SortBy::from_key(&event_target_value(&ev)))
                    >
                        {SortBy::ALL
                            .into_iter()
                            .map(|s| view! { <option value=s.key()>{s.label()}</option> })
                            .collect_view()}
                    </select>
                </div>
            </div>

            <div class="ware-head" aria-hidden="true">
                <span></span>
                <span>"Item"</span>
                <span>"Ready in"</span>
                <span class="right">"Price"</span>
                <span></span>
            </div>
            <ul class="ware-list">
                <For
                    each=move || sorted.get()
                    key=|index| *index
                    children=move |index| {
                        view! {
                            <WareRow
                                shop
                                order
                                index
                                hidden=Signal::derive(move || !matches(index))
                            />
                        }
                    }
                />
            </ul>
            <p class="empty" hidden=move || shown.get() != 0>
                "Nothing here matches \""
                {move || query.get()}
                "\". "
                <button
                    type="button"
                    class="linkish"
                    on:click=move |_| {
                        query.set(String::new());
                        tab.set(String::new());
                        browser::set_hash("");
                    }
                >
                    "Show everything"
                </button>
            </p>
        </section>
    }
}

#[component]
fn WareRow(shop: &'static Shop, order: Order, index: usize, hidden: Signal<bool>) -> impl IntoView {
    let item = &shop.items[index];
    let glyph = shop
        .category(&item.category)
        .map(|c| c.glyph)
        .unwrap_or_default();
    let ready = shop.ready_of(item);
    let soon = match ready.soon() {
        Soon::Now => "now",
        Soon::WithinADay => "day",
        Soon::Later => "later",
    };
    let icon = item.icon.filter(|_| shop.shop.icons).map(icon_url);
    let icon_failed = RwSignal::new(false);
    let in_order = move || order.quantity(index);

    let has_gear_line = item.ilvl.is_some() || item.level.is_some() || item.jobs.is_some();
    let kind = shop
        .category(&item.category)
        .map(|c| c.label.clone())
        .unwrap_or_default();

    view! {
        <li class="ware" hidden=hidden data-item-id=item.id.map(|id| id.to_string())>
            <div class="slot" aria-hidden="true">
                <GlyphIcon glyph />
                {icon
                    .map(|src| {
                        view! {
                            <img
                                src=src
                                alt=""
                                width="40"
                                height="40"
                                loading="lazy"
                                decoding="async"
                                class:gone=move || icon_failed.get()
                                on:error=move |_| icon_failed.set(true)
                            />
                        }
                    })}
            </div>
            <div class="ware-main">
                <p class="ware-name">
                    <a
                        class="eorzeadb_link"
                        href=shop.edb_url(item)
                        target="_blank"
                        rel="noopener"
                        title="Open in the Eorzea Database"
                    >
                        {item.name.clone()}
                    </a>
                    {item
                        .hq
                        .then(|| {
                            view! {
                                <span class="hq" title="Made in high quality">
                                    "HQ"
                                </span>
                            }
                        })}
                </p>
                <p class="ware-meta">
                    {item.ilvl.map(|v| view! { <span title="Item level">"i" {v}</span> })}
                    {item.level.map(|v| view! { <span>"Lv. " {v}</span> })}
                    {item.jobs.clone().map(|v| view! { <span class="jobs">{v}</span> })}
                    {(!has_gear_line).then(|| view! { <span>{kind}</span> })}
                </p>
                {item.note.clone().map(|note| view! { <p class="ware-note">{note}</p> })}
            </div>
            <p class="ware-ready" data-soon=soon>
                <ClockIcon />
                <span class="sr-only">"Ready in "</span>
                {ready.label()}
            </p>
            <div class="ware-price">
                {match item.price {
                    Some(price) => {
                        view! {
                            <p class="price">
                                <GilIcon />
                                {gil(price)}
                                <span class="sr-only">" gil"</span>
                            </p>
                        }
                            .into_any()
                    }
                    None => view! { <p class="price ask">"Ask"</p> }.into_any(),
                }} {item.per.clone().map(|per| view! { <p class="per">{per}</p> })}
                {match (item.price_own_mats, item.own_mats_ask) {
                    (Some(p), _) => {
                        Some(
                            view! {
                                <p class="mats" title="Price if you bring the materials">
                                    "Your mats: "
                                    {gil(p)}
                                </p>
                            }
                                .into_any(),
                        )
                    }
                    (None, true) => {
                        Some(
                            view! {
                                <p class="mats" title="Price if you bring the materials">
                                    "Your mats: Ask"
                                </p>
                            }
                                .into_any(),
                        )
                    }
                    (None, false) => None,
                }}
            </div>
            <div class="ware-act">
                <button
                    type="button"
                    class="btn add"
                    aria-label=format!("Add {} to your order", item.name)
                    on:click=move |_| order.add(index)
                >
                    "Add"
                </button>
                <span class="in-order" class:shown=move || in_order() != 0>
                    {move || format!("{} in order", in_order())}
                </span>
            </div>
        </li>
    }
}
