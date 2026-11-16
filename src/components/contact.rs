//! Ways to reach the shopkeeper, in and out of the game.

use super::copy_button::CopyButton;
use crate::model::Shop;
use leptos::prelude::*;

const DEFAULT_STEPS: [&str; 4] = [
    "Add what you'd like to your order, then copy it.",
    "Send it to me in game or by message.",
    "I'll confirm the price and when it'll be ready.",
    "Collect it with a trade, or from my retainer.",
];

#[component]
pub fn ContactCard(shop: &'static Shop) -> impl IntoView {
    let c = &shop.contact;
    let owner = shop.shop.owner.clone();
    let world = shop.location.world.clone();

    let in_game = view! {
        {c
            .tell
            .then(|| {
                view! {
                    <li class="channel">
                        <span class="ch-label">"Send a tell"</span>
                        <code class="ch-value">{shop.tell_command()}</code>
                        <CopyButton
                            text=shop.tell_command()
                            describe="Copy the /tell command".to_string()
                        />
                    </li>
                }
            })}
        {c
            .friend_request
            .then(|| {
                view! {
                    <li class="channel">
                        <span class="ch-label">"Friend request"</span>
                        <span class="ch-value">{owner.clone()} " on " {world.clone()}</span>
                    </li>
                }
            })}
        {c
            .message_book
            .then(|| {
                view! {
                    <li class="channel">
                        <span class="ch-label">"Message book"</span>
                        <span class="ch-value">
                            "Leave a note in the message book at my estate."
                        </span>
                    </li>
                }
            })}
        {c
            .linkshell
            .clone()
            .map(|ls| {
                view! {
                    <li class="channel">
                        <span class="ch-label">"Linkshell"</span>
                        <span class="ch-value">{ls}</span>
                    </li>
                }
            })}
    };

    let has_out_of_game = c.discord.is_some()
        || c.discord_server.is_some()
        || c.email.is_some()
        || !c.links.is_empty();
    let out_of_game = view! {
        {c
            .discord
            .clone()
            .map(|name| {
                view! {
                    <li class="channel">
                        <span class="ch-label">"Discord"</span>
                        <code class="ch-value">{name.clone()}</code>
                        <CopyButton text=name describe="Copy my Discord username".to_string() />
                    </li>
                }
            })}
        {c
            .discord_server
            .clone()
            .map(|url| {
                view! {
                    <li class="channel">
                        <span class="ch-label">"Discord server"</span>
                        <a class="ch-value" href=url target="_blank" rel="noopener">
                            "Join the server"
                        </a>
                    </li>
                }
            })}
        {c
            .email
            .clone()
            .map(|email| {
                view! {
                    <li class="channel">
                        <span class="ch-label">"Email"</span>
                        <a class="ch-value" href=format!("mailto:{email}")>
                            {email.clone()}
                        </a>
                        <CopyButton text=email describe="Copy my email address".to_string() />
                    </li>
                }
            })}
        {c
            .links
            .iter()
            .map(|link| {
                view! {
                    <li class="channel">
                        <span class="ch-label">{link.label.clone()}</span>
                        <a class="ch-value" href=link.url.clone() target="_blank" rel="noopener">
                            {link
                                .url
                                .trim_start_matches("https://")
                                .trim_start_matches("http://")
                                .to_string()}
                        </a>
                    </li>
                }
            })
            .collect_view()}
    };

    let steps: Vec<String> = if shop.shop.how_to_order.is_empty() {
        DEFAULT_STEPS.iter().map(|s| s.to_string()).collect()
    } else {
        shop.shop.how_to_order.clone()
    };

    view! {
        <section class="window contact" aria-labelledby="contact-title">
            <header class="window-title">
                <h2 id="contact-title">"Request a craft"</h2>
            </header>
            <div class="window-body">
                {c.note.clone().map(|note| view! { <p class="lede">{note}</p> })}
                <div class="block">
                    <h3>"In game"</h3>
                    <ul class="channels">{in_game}</ul>
                </div>
                {has_out_of_game
                    .then(|| {
                        view! {
                            <div class="block">
                                <h3>"Out of game"</h3>
                                <ul class="channels">{out_of_game}</ul>
                            </div>
                        }
                    })} <div class="block">
                    <h3>"How ordering works"</h3>
                    <ol class="steps">
                        {steps.into_iter().map(|step| view! { <li>{step}</li> }).collect_view()}
                    </ol>
                </div>
            </div>
        </section>
    }
}
