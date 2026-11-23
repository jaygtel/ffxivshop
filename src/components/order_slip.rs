//! The visitor's order, and the two ways to send it: by email, through the
//! orders service on the server, or copied for a Discord message.

use super::glyphs::GilIcon;
use crate::basket::{Answer, MATS_RULE, NOTE_LIMIT, SEND_ADDRESS};
use crate::browser;
use crate::model::{Shop, gil};
use crate::order::Order;
use leptos::prelude::*;
use leptos::task::spawn_local;
use std::time::Duration;

/// How long the panel waits for the orders service to answer.
const SEND_WAIT_MS: i32 = 30_000;

/// How long the note that an order was sent stays, before the panel goes
/// back to the empty order.
const SENT_SHOWN: Duration = Duration::from_secs(10);

/// An order that has just been emailed.
#[derive(Clone)]
struct Sent {
    number: String,
    /// Where the copy went.
    email: String,
}

#[component]
pub fn OrderSlip(shop: &'static Shop, order: Order) -> impl IntoView {
    let contact = &shop.contact;
    let by_email = contact.send_by_email;
    let copied = RwSignal::new(false);
    // If the browser blocks the clipboard, the message is shown for copying by hand.
    let by_hand = RwSignal::new(None::<String>);
    let sending = RwSignal::new(false);
    // Why the order wasn't sent, for the buyer to put right.
    let problem = RwSignal::new(None::<String>);
    let sent = RwSignal::new(None::<Sent>);
    // The hidden box (see `trap` below).
    let trap = RwSignal::new(String::new());

    // Adding to a new order takes away the note that the last one was sent.
    Effect::new(move |_| {
        if !order.is_empty() {
            sent.set(None);
        }
    });

    let copy = move |text: String| {
        spawn_local(async move {
            if browser::copy(&text).await {
                by_hand.set(None);
                copied.set(true);
                set_timeout(move || copied.set(false), Duration::from_millis(2200));
            } else {
                by_hand.set(Some(text));
            }
        });
    };

    let send = move |_| {
        if sending.get_untracked() {
            return;
        }
        if let Some(why) = order.problem_before_emailing() {
            problem.set(Some(why.to_string()));
            return;
        }
        problem.set(None);
        by_hand.set(None);
        sending.set(true);
        let request = order.for_email(trap.get_untracked());
        let email = request.email.clone();
        spawn_local(async move {
            let json = serde_json::to_string(&request).unwrap_or_default();
            let answer = browser::post_json(SEND_ADDRESS, &json, SEND_WAIT_MS)
                .await
                .map(|(status, text)| (status, serde_json::from_str::<Answer>(&text).ok()));
            sending.set(false);
            match answer {
                Some((
                    200,
                    Some(Answer {
                        number: Some(number),
                        ..
                    }),
                )) => {
                    order.sent();
                    sent.set(Some(Sent {
                        number: number.clone(),
                        email,
                    }));
                    // Then back to the empty order, unless another order has
                    // been sent since (or the panel has gone).
                    set_timeout(
                        move || {
                            let still_showing = sent
                                .try_get_untracked()
                                .flatten()
                                .is_some_and(|shown| shown.number == number);
                            if still_showing {
                                sent.set(None);
                            }
                        },
                        SENT_SHOWN,
                    );
                }
                Some((
                    _,
                    Some(Answer {
                        problem: Some(why), ..
                    }),
                )) => problem.set(Some(why)),
                _ => problem.set(Some(not_sent(shop))),
            }
        });
    };
    // Changing a box puts away the last problem: it may be fixed now.
    let edited = move || {
        if problem.get_untracked().is_some() {
            problem.set(None);
        }
    };

    let written_label = if contact.discord.is_some() {
        "Copy for Discord"
    } else {
        "Copy as a message"
    };
    let how_to_send = match (by_email, &contact.discord, &contact.email) {
        (true, Some(discord), _) => format!(
            "Send by email sends your order to me, with a copy to you. Or copy it, then paste it into a Discord message to {discord}."
        ),
        (true, None, _) => "Send by email sends your order to me, with a copy to you. Or copy it to send it yourself.".to_string(),
        (false, Some(discord), _) => format!("Paste it into a Discord message to {discord}."),
        (false, None, Some(email)) => format!("Paste it into an email to {email}."),
        (false, None, None) => "Paste it into a message to me.".to_string(),
    };
    // With Send by email, copying is the second choice.
    let copy_class = if by_email { "btn" } else { "btn btn-primary" };
    let ready_to = match (by_email, contact.discord.is_some()) {
        (true, true) => "ready to send by email or on Discord",
        (true, false) => "ready to send by email",
        (false, true) => "ready to copy for Discord",
        (false, false) => "ready to copy as a message",
    };

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
                    fallback=move || {
                        match sent.get() {
                            Some(Sent { number, email }) => {
                                view! {
                                    <div class="order-sent" role="status">
                                        <p>
                                            <strong>"Order sent."</strong>
                                            " Your order number is "
                                            <strong class="order-number">{number}</strong>
                                            "."
                                        </p>
                                        <p class="order-small">
                                            "A copy is on its way to " {email}
                                            ". I'll reply to confirm the price and when it'll be ready."
                                        </p>
                                    </div>
                                }
                                    .into_any()
                            }
                            None => {
                                view! {
                                    <p class="order-empty">
                                        "Nothing yet. Choose Add next to anything in the shop and it will collect here, "
                                        {ready_to} "."
                                    </p>
                                }
                                    .into_any()
                            }
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
                    // The rule for bringing materials, while the box is ticked.
                    // The copied order and the receipt say the same.
                    <p
                        class="mats-rule"
                        hidden=move || !(order.own_mats.get() && order.mats_option())
                    >
                        {MATS_RULE}
                    </p>

                    <div class="field">
                        <label for="order-note">"Anything else?"</label>
                        <input
                            id="order-note"
                            type="text"
                            maxlength=NOTE_LIMIT
                            placeholder="Dye, materia, how many in HQ"
                            prop:value=move || order.note.get()
                            on:input=move |ev| {
                                order.note.set(event_target_value(&ev));
                                edited();
                            }
                        />
                    </div>
                    <div class="field-pair">
                        <div class="field">
                            <label for="order-me">"Your character"</label>
                            <input
                                id="order-me"
                                type="text"
                                maxlength="30"
                                placeholder="First and last name"
                                autocomplete="off"
                                prop:value=move || order.character.get()
                                on:input=move |ev| {
                                    order.character.set(event_target_value(&ev));
                                    edited();
                                }
                            />
                        </div>
                        <div class="field">
                            <label for="order-world">"Your world"</label>
                            <input
                                id="order-world"
                                type="text"
                                maxlength="20"
                                placeholder="Cerberus"
                                autocomplete="off"
                                prop:value=move || order.world.get()
                                on:input=move |ev| {
                                    order.world.set(event_target_value(&ev));
                                    edited();
                                }
                            />
                        </div>
                    </div>
                    {by_email
                        .then(|| {
                            view! {
                                <div class="field">
                                    <label for="order-email">"Your email, for your copy"</label>
                                    <input
                                        id="order-email"
                                        type="email"
                                        maxlength="254"
                                        placeholder="name@example.com"
                                        autocomplete="email"
                                        prop:value=move || order.email.get()
                                        on:input=move |ev| {
                                            order.email.set(event_target_value(&ev));
                                            edited();
                                        }
                                    />
                                </div>
                                // A box people never see: it's off the screen, hidden from
                                // screen readers and skipped by the keyboard. Bots that
                                // fill in every box fill this one in too, and the orders
                                // service turns their orders away.
                                <div class="trap" aria-hidden="true">
                                    <label for="order-website">"Leave this empty"</label>
                                    <input
                                        id="order-website"
                                        type="text"
                                        name="website"
                                        tabindex="-1"
                                        autocomplete="off"
                                        prop:value=move || trap.get()
                                        on:input=move |ev| trap.set(event_target_value(&ev))
                                    />
                                </div>
                            }
                        })}

                    {move || {
                        problem
                            .get()
                            .map(|why| {
                                view! {
                                    <p class="order-problem" role="alert">
                                        {why}
                                    </p>
                                }
                            })
                    }}
                    <div class="order-actions">
                        {by_email
                            .then(|| {
                                view! {
                                    <button
                                        type="button"
                                        class="btn btn-primary"
                                        aria-disabled=move || sending.get().then_some("true")
                                        on:click=send
                                    >
                                        {move || {
                                            if sending.get() { "Sending…" } else { "Send by email" }
                                        }}
                                    </button>
                                }
                            })}
                        <button
                            type="button"
                            class=copy_class
                            on:click=move |_| copy(order.written_message())
                        >
                            {move || if copied.get() { "Copied" } else { written_label }}
                        </button>
                    </div>
                    <p class="order-small">{how_to_send.clone()}</p>
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

/// What the panel says when the orders service couldn't be reached, or gave
/// no reason.
fn not_sent(shop: &Shop) -> String {
    let mut words =
        "Your order could not be sent just now. Please try again in a few minutes.".to_string();
    if shop.contact.discord.is_some() {
        words.push_str(" Or copy it for Discord.");
    }
    words
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

#[cfg(test)]
mod tests {
    use super::not_sent;
    use crate::model::Shop;

    #[test]
    fn the_panels_own_words_are_plain_english() {
        let shop = Shop::from_toml(include_str!("../../shop.toml")).unwrap();
        let words = not_sent(&shop);
        assert!(!words.contains("n't"), "{words}");
        assert!(words.ends_with("Or copy it for Discord."));
    }
}
