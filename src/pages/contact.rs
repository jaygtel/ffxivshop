//! The contact page: a form that sends the shop a message by email. The
//! orders service on the server (orders/) emails it to the shop, with
//! Reply-To set to the visitor's own address, so replying answers them. The
//! visitor gets no copy: the page tells them the message was sent.
//!
//! What the visitor types lasts the whole visit (src/app.rs keeps it), so
//! going to look something up in the shop loses nothing. It isn't saved in
//! the browser.

use super::SHOP;
use crate::browser;
use crate::components::Footer;
use crate::message::{
    ContactMessage, MESSAGE_LIMIT, NAME_LIMIT, Part, Reply, SEND_ADDRESS, SUBJECT_LIMIT,
};
use crate::model::Shop;
use leptos::ev::SubmitEvent;
use leptos::prelude::*;
use leptos::task::spawn_local;

/// How long the page waits for the orders service to answer.
const SEND_WAIT_MS: i32 = 30_000;

/// The message box says how many characters are left once there are this
/// many or fewer.
const SHOW_LEFT_FROM: usize = 200;

/// What the visitor has typed. src/app.rs keeps it for the whole visit.
#[derive(Clone, Copy)]
pub struct MessageDraft {
    name: RwSignal<String>,
    email: RwSignal<String>,
    subject: RwSignal<String>,
    message: RwSignal<String>,
}

impl MessageDraft {
    pub fn new() -> Self {
        MessageDraft {
            name: RwSignal::new(String::new()),
            email: RwSignal::new(String::new()),
            subject: RwSignal::new(String::new()),
            message: RwSignal::new(String::new()),
        }
    }

    /// The message as the orders service takes it, tidied. `website` is the
    /// hidden box's value.
    fn to_send(self, website: String) -> ContactMessage {
        ContactMessage {
            name: self.name.get_untracked(),
            email: self.email.get_untracked(),
            subject: self.subject.get_untracked(),
            message: self.message.get_untracked(),
            website,
        }
        .tidied()
    }

    /// Once the message has gone: the subject and message go too, and the
    /// name and email stay, ready for another message.
    fn sent(self) {
        self.subject.set(String::new());
        self.message.set(String::new());
    }
}

/// Why the message wasn't sent, and the box to put right, if it's one box.
#[derive(Clone)]
struct Problem {
    part: Option<Part>,
    words: String,
}

/// A message that has just been sent.
#[derive(Clone, Default)]
struct Sent {
    name: String,
    /// Where the reply will go.
    email: String,
}

/// The id of each box on the page.
fn box_id(part: Part) -> &'static str {
    match part {
        Part::Name => "contact-name",
        Part::Email => "contact-email",
        Part::Subject => "contact-subject",
        Part::Message => "contact-message",
    }
}

#[component]
pub fn ContactPage(shop: &'static Shop, draft: MessageDraft) -> impl IntoView {
    let sending = RwSignal::new(false);
    let problem = RwSignal::new(None::<Problem>);
    let sent = RwSignal::new(None::<Sent>);
    // The hidden box (see `trap` below).
    let trap = RwSignal::new(String::new());

    let send = move |event: SubmitEvent| {
        event.prevent_default();
        if sending.get_untracked() {
            return;
        }
        let message = draft.to_send(trap.get_untracked());
        if let Some((part, why)) = message.problem() {
            problem.set(Some(Problem {
                part: Some(part),
                words: why.to_string(),
            }));
            browser::focus_quietly(box_id(part));
            return;
        }
        problem.set(None);
        sending.set(true);
        spawn_local(async move {
            let json = serde_json::to_string(&message).unwrap_or_default();
            let answer = browser::post_json(SEND_ADDRESS, &json, SEND_WAIT_MS)
                .await
                .map(|(status, text)| (status, serde_json::from_str::<Reply>(&text).ok()));
            sending.set(false);
            let words = match answer {
                Some((200, Some(Reply { sent: true, .. }))) => {
                    draft.sent();
                    sent.set(Some(Sent {
                        name: message.name,
                        email: message.email,
                    }));
                    // The form has gone, so keyboard focus moves to the
                    // words that say the message was sent.
                    request_animation_frame(|| browser::focus_quietly("message-sent"));
                    return;
                }
                // Turned away, or not sent this time: the service says why.
                Some((
                    status,
                    Some(Reply {
                        problem: Some(why), ..
                    }),
                )) if status < 500 && status != 429 => why,
                Some((
                    _,
                    Some(Reply {
                        problem: Some(why), ..
                    }),
                )) => format!("{why}{}", other_ways(shop)),
                // The service couldn't be reached, or gave no reason.
                _ => not_sent(shop),
            };
            problem.set(Some(Problem { part: None, words }));
        });
    };
    // Changing a box puts away the last problem: it may be fixed now.
    let edited = move || {
        if problem.get_untracked().is_some() {
            problem.set(None);
        }
    };
    let write_another = move |_| {
        sent.set(None);
        request_animation_frame(|| browser::focus_quietly(box_id(Part::Subject)));
    };

    // Marks the box with the problem, for screen readers, and points them at
    // the words that say what's wrong.
    let invalid = move |part: Part| {
        problem.with(|problem| {
            problem
                .as_ref()
                .is_some_and(|problem| problem.part == Some(part))
        })
    };
    let invalid_attr = move |part: Part| invalid(part).then_some("true");
    let described_by = move |part: Part| invalid(part).then_some("contact-problem");

    let characters_left = move || {
        let left =
            MESSAGE_LIMIT.saturating_sub(draft.message.with(|message| message.chars().count()));
        match left {
            _ if left > SHOW_LEFT_FROM => None,
            0 => Some("No characters left.".to_string()),
            1 => Some("1 character left.".to_string()),
            left => Some(format!("{left} characters left.")),
        }
    };

    let form = move || {
        view! {
            <form class="message-fields" novalidate on:submit=send>
                <div class="field-pair">
                    <div class="field">
                        <label for="contact-name">"Your name"</label>
                        <input
                            id="contact-name"
                            name="name"
                            type="text"
                            maxlength=NAME_LIMIT
                            placeholder="Your name or your character's name"
                            autocomplete="name"
                            aria-invalid=move || invalid_attr(Part::Name)
                            aria-describedby=move || described_by(Part::Name)
                            prop:value=move || draft.name.get()
                            on:input=move |ev| {
                                draft.name.set(event_target_value(&ev));
                                edited();
                            }
                        />
                    </div>
                    <div class="field">
                        <label for="contact-email">"Your email"</label>
                        <input
                            id="contact-email"
                            name="email"
                            type="email"
                            maxlength="254"
                            placeholder="name@example.com"
                            autocomplete="email"
                            aria-invalid=move || invalid_attr(Part::Email)
                            aria-describedby=move || described_by(Part::Email)
                            prop:value=move || draft.email.get()
                            on:input=move |ev| {
                                draft.email.set(event_target_value(&ev));
                                edited();
                            }
                        />
                    </div>
                </div>
                <div class="field">
                    <label for="contact-subject">"Subject"</label>
                    <input
                        id="contact-subject"
                        name="subject"
                        type="text"
                        maxlength=SUBJECT_LIMIT
                        placeholder="Question about my order"
                        autocomplete="off"
                        aria-invalid=move || invalid_attr(Part::Subject)
                        aria-describedby=move || described_by(Part::Subject)
                        prop:value=move || draft.subject.get()
                        on:input=move |ev| {
                            draft.subject.set(event_target_value(&ev));
                            edited();
                        }
                    />
                </div>
                <div class="field">
                    <label for="contact-message">"Your message"</label>
                    <textarea
                        id="contact-message"
                        name="message"
                        rows="8"
                        maxlength=MESSAGE_LIMIT
                        aria-invalid=move || invalid_attr(Part::Message)
                        aria-describedby=move || described_by(Part::Message)
                        prop:value=move || draft.message.get()
                        on:input=move |ev| {
                            draft.message.set(event_target_value(&ev));
                            edited();
                        }
                    ></textarea>
                    <p class="small message-left" aria-live="polite">
                        {characters_left}
                    </p>
                </div>
                // A box people never see: it's off the screen, hidden from
                // screen readers and skipped by the keyboard. Bots that fill
                // in every box fill this one in too, and the orders service
                // turns their messages away.
                <div class="trap" aria-hidden="true">
                    <label for="contact-website">"Leave this empty"</label>
                    <input
                        id="contact-website"
                        type="text"
                        name="website"
                        tabindex="-1"
                        autocomplete="off"
                        prop:value=move || trap.get()
                        on:input=move |ev| trap.set(event_target_value(&ev))
                    />
                </div>
                {move || {
                    problem
                        .get()
                        .map(|problem| {
                            view! {
                                <p class="message-problem" id="contact-problem" role="alert">
                                    {problem.words}
                                </p>
                            }
                        })
                }}
                <div class="message-actions">
                    <button
                        type="submit"
                        class="btn btn-primary"
                        aria-disabled=move || sending.get().then_some("true")
                    >
                        {move || if sending.get() { "Sending…" } else { "Send message" }}
                    </button>
                </div>
                <p class="small">
                    "Your message comes to me by email, and my reply goes to the address you give."
                </p>
            </form>
        }
    };

    view! {
        <div class="page contact-page">
            <main>
                <h1 class="sr-only">"Contact"</h1>
                <section class="window contact-window" aria-labelledby="message-title">
                    <header class="window-title">
                        <h2 id="message-title">"Send me a message"</h2>
                    </header>
                    <div class="window-body">
                        <p class="lede">
                            "Have a question about an order, or something you'd like me to craft? Send me a message and I'll reply by email."
                        </p>
                        {shop
                            .contact
                            .note
                            .clone()
                            .map(|note| view! { <p class="small">{note}</p> })}
                        <p class="small">
                            "Ready to order? Add what you'd like in the " <a href=SHOP>"shop"</a>
                            ", then send your order from there."
                        </p>
                        <Show
                            when=move || sent.get().is_none()
                            fallback=move || {
                                let Sent { name, email } = sent.get().unwrap_or_default();
                                view! {
                                    <div
                                        class="message-sent"
                                        id="message-sent"
                                        tabindex="-1"
                                        role="status"
                                    >
                                        <p>
                                            <strong>"Message sent."</strong>
                                            " Thank you, "
                                            {name}
                                            "."
                                        </p>
                                        <p class="small">
                                            "I'll reply to " <strong class="message-to">{email}</strong>
                                            "."
                                        </p>
                                        <button type="button" class="btn" on:click=write_another>
                                            "Write another message"
                                        </button>
                                    </div>
                                }
                            }
                        >
                            {form()}
                        </Show>
                    </div>
                </section>
            </main>
            <Footer shop />
        </div>
    }
}

/// The other ways to reach the shop, for when a message can't be sent.
fn other_ways(shop: &Shop) -> String {
    match (&shop.contact.discord, &shop.contact.email) {
        (Some(discord), Some(email)) => {
            format!(" You can also message me on Discord ({discord}) or email me at {email}.")
        }
        (Some(discord), None) => format!(" You can also message me on Discord ({discord})."),
        (None, Some(email)) => format!(" You can also email me at {email}."),
        (None, None) => String::new(),
    }
}

/// What the page says when the orders service couldn't be reached, or gave
/// no reason.
fn not_sent(shop: &Shop) -> String {
    format!(
        "Your message could not be sent just now. Please try again in a few minutes.{}",
        other_ways(shop)
    )
}

#[cfg(test)]
mod tests {
    use super::{not_sent, other_ways};
    use crate::model::Shop;

    #[test]
    fn the_pages_own_words_are_plain_english() {
        let mut shop = Shop::from_toml(include_str!("../../shop.toml")).unwrap();
        shop.contact.discord = Some("mira.crafts".into());
        shop.contact.email = Some("mira@example.com".into());
        let words = not_sent(&shop);
        assert!(!words.contains("n't"), "{words}");
        assert_eq!(
            words,
            "Your message could not be sent just now. Please try again in a few minutes. You can also message me on Discord (mira.crafts) or email me at mira@example.com."
        );
        shop.contact.discord = None;
        assert_eq!(
            other_ways(&shop),
            " You can also email me at mira@example.com."
        );
        shop.contact.email = None;
        assert_eq!(other_ways(&shop), "");
    }
}
