//! The visitor's order and the messages they copy to send it.

use super::glyphs::GilIcon;
use crate::browser;
use crate::model::{Shop, gil};
use crate::order::Order;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
enum Copied {
    Nothing,
    Tell,
    Written,
}

#[component]
pub fn OrderSlip(shop: &'static Shop, order: Order) -> impl IntoView {
    let contact = &shop.contact;
    let copied = RwSignal::new(Copied::Nothing);
    // If the browser blocks the clipboard, the message is shown for copying by hand.
    let by_hand = RwSignal::new(None::<String>);

    let copy = move |which: Copied, text: String| {
        spawn_local(async move {
            if browser::copy(&text).await {
                by_hand.set(None);
                copied.set(which);
                set_timeout(
                    move || {
                        if copied.get_untracked() == which {
                            copied.set(Copied::Nothing);
                        }
                    },
                    Duration::from_millis(2200),
                );
            } else {
                by_hand.set(Some(text));
            }
        });
    };

    let written_label = if contact.discord.is_some() {
        "Copy for Discord"
    } else {
        "Copy as a message"
    };
    let where_to_send = match (&contact.discord, &contact.email) {
        (Some(discord), _) => format!(" or into a Discord message to {discord}"),
        (None, Some(email)) => format!(" or into an email to {email}"),
        (None, None) => String::new(),
    };
    let show_tell = contact.tell;

    view! {
        <section class="window order" id="order" aria-labelledby="order-title">
            <header class="window-title">
                <h2 id="order-title">"Your order"</h2>
                <span class="count">
                    {move || match order.count() {
                        0 => String::new(),
                        1 => "1 item".to_string(),
                        n => format!("{n} items"),
                    }}
                </span>
            </header>
            <div class="window-body">
                <Show
                    when=move || !order.is_empty()
                    fallback=|| {
                        view! {
                            <p class="order-empty">
                                "Nothing yet. Choose Add next to anything in the shop and it will collect here, ready to send by /tell or message."
                            </p>
                        }
                    }
                >
                    <ul class="order-lines">
                        <For
                            each=move || order.lines()
                            key=|line| line.0
                            children=move |(index, _)| view! { <OrderLine shop order index /> }
                        />
                    </ul>

                    <div class="order-total">
                        <span>"Total"</span>
                        <strong>
                            <TotalAmount order />
                        </strong>
                    </div>
                    <p class="order-small" hidden=move || order.totals().unpriced == 0>
                        "Items marked " <em>"Ask"</em>
                        " have no price yet. I'll tell you the price when you send me your order."
                    </p>
                    <p class="order-small">
                        {move || {
                            let ready = order.ready();
                            if ready.minutes == 0 {
                                "Everything is in stock. I'll confirm when you get in touch."
                                    .to_string()
                            } else {
                                format!(
                                    "Ready in about {}, the longest wait in your order. I'll confirm when you get in touch.",
                                    ready.label().to_lowercase(),
                                )
                            }
                        }}
                    </p>

                    <label class="check" hidden=move || !order.mats_option()>
                        <input
                            type="checkbox"
                            id="order-mats"
                            prop:checked=move || order.own_mats.get()
                            on:change=move |ev| order.own_mats.set(event_target_checked(&ev))
                        />
                        "I'll bring the materials (uses the lower prices)"
                    </label>

                    <div class="field">
                        <label for="order-note">"Anything else?"</label>
                        <input
                            id="order-note"
                            type="text"
                            maxlength="200"
                            placeholder="Dye, materia, how many in HQ"
                            prop:value=move || order.note.get()
                            on:input=move |ev| order.note.set(event_target_value(&ev))
                        />
                    </div>
                    <div class="field">
                        <label for="order-me">"Your character, for messages"</label>
                        <input
                            id="order-me"
                            type="text"
                            maxlength="60"
                            placeholder="Name @ World"
                            autocomplete="off"
                            prop:value=move || order.character.get()
                            on:input=move |ev| order.character.set(event_target_value(&ev))
                        />
                    </div>

                    <div class="order-actions">
                        {show_tell
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        class="btn btn-primary"
                                        on:click=move |_| copy(Copied::Tell, order.tell_message())
                                    >
                                        {move || {
                                            if copied.get() == Copied::Tell {
                                                "Copied"
                                            } else {
                                                "Copy as /tell"
                                            }
                                        }}
                                    </button>
                                }
                            })}
                        <button
                            type="button"
                            class="btn"
                            on:click=move |_| copy(Copied::Written, order.written_message())
                        >
                            {move || {
                                if copied.get() == Copied::Written {
                                    "Copied"
                                } else {
                                    written_label
                                }
                            }}
                        </button>
                    </div>
                    <p class="order-small">
                        {if show_tell {
                            format!("Paste it into your chat box in game{where_to_send}.")
                        } else {
                            format!("Paste it{where_to_send}.")
                        }}
                    </p>
                    {move || {
                        by_hand
                            .get()
                            .map(|text| {
                                view! {
                                    <div class="by-hand">
                                        <label for="order-copy">
                                            "Your browser blocked copying. Select this text and copy it:"
                                        </label>
                                        <textarea
                                            id="order-copy"
                                            readonly
                                            rows="5"
                                            prop:value=text
                                        ></textarea>
                                    </div>
                                }
                            })
                    }}
                    <button type="button" class="linkish" on:click=move |_| order.clear()>
                        "Clear order"
                    </button>
                </Show>
            </div>
        </section>
        // On small screens a bar keeps the order in reach while browsing.
        <div class="order-peek" hidden=move || order.is_empty()>
            <span>
                {move || {
                    let n = order.count();
                    format!("{n} {}", if n == 1 { "item" } else { "items" })
                }}
            </span>
            <strong>
                <TotalAmount order />
            </strong>
            <button
                type="button"
                class="btn btn-primary"
                on:click=move |_| browser::scroll_to("order")
            >
                "View order"
            </button>
        </div>
    }
}

/// The order's total, worded like the shop list so it never shows a 0 that
/// looks free: "Ask" when nothing in the order has a price yet, and "+ Ask"
/// after the gil when only some of it does.
#[component]
fn TotalAmount(order: Order) -> impl IntoView {
    move || {
        let totals = order.totals();
        let ask = totals.unpriced > 0;
        if totals.priced == 0 && ask {
            view! { <em class="ask">"Ask"</em> }.into_any()
        } else {
            view! {
                <GilIcon />
                {gil(totals.gil)}
                <span class="sr-only">" gil"</span>
                {ask.then(|| view! { <span class="ask">" + " <em>"Ask"</em></span> })}
            }
            .into_any()
        }
    }
}

#[component]
fn OrderLine(shop: &'static Shop, order: Order, index: usize) -> impl IntoView {
    let item = &shop.items[index];
    let qty = move || order.quantity(index);
    let line_total = move || {
        order
            .unit_price(index)
            .map(|price| gil(price * u64::from(qty())))
    };
    view! {
        <li class="order-line">
            <span class="ol-name">{item.name.clone()}</span>
            <span class="stepper" role="group" aria-label=format!("How many {}", item.name)>
                <button
                    type="button"
                    aria-label="One fewer"
                    on:click=move |_| order.set(index, qty().saturating_sub(1))
                >
                    "−"
                </button>
                <output aria-live="polite">{qty}</output>
                <button
                    type="button"
                    aria-label="One more"
                    on:click=move |_| order.set(index, qty() + 1)
                >
                    "+"
                </button>
            </span>
            // "Ask" looks just like a price here: coin first, same size and colour.
            <span class="ol-price">
                <GilIcon />
                {move || line_total().unwrap_or_else(|| "Ask".to_string())}
            </span>
        </li>
    }
}
