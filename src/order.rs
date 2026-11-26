//! The visitor's order: what they've picked, how many, their details, and the
//! messages they can send you. It's saved in their browser so it survives a
//! reload, and cleared once it has gone 12 hours without a change. The sums
//! and checks behind it are in src/basket.rs, which the orders service
//! shares.

use crate::basket::{self, EmailOrder, Ordered, Totals};
use crate::browser;
use crate::model::{Ready, Shop, gil};
use leptos::prelude::*;
use std::time::Duration;

const STORAGE_KEY: &str = "ffxiv-shop.order";

/// The cookie that keeps a saved order. Each change to the order sets it
/// again for 12 hours; when the browser deletes it, the order is cleared.
/// Its value is the time it runs out, in milliseconds since 1970.
const KEEP_COOKIE: &str = "ffxiv-shop.order-expires";

/// In the copied order when the buyer will bring the materials: they must
/// bring all of them. It's in the buyer's words, as the message is theirs.
/// The order panel says the same in the shop's words, under the tick
/// (basket::MATS_RULE).
const MATS_WRITTEN: &str = "I will bring all the materials. If I cannot bring them all, you will craft my order at the standard price, with your own materials. I know you will not take only some of the materials.";
const KEEP_FOR_SECONDS: u32 = 12 * 60 * 60;

#[derive(Clone, Copy)]
pub struct Order {
    shop: &'static Shop,
    /// (item index, quantity), in the order they were added.
    lines: RwSignal<Vec<(usize, u32)>>,
    /// The buyer will bring the materials.
    pub own_mats: RwSignal<bool>,
    /// The buyer's character's first and last name.
    pub character: RwSignal<String>,
    /// The buyer's world.
    pub world: RwSignal<String>,
    /// Where replies to an emailed order go.
    pub email: RwSignal<String>,
    pub note: RwSignal<String>,
}

impl Order {
    pub fn new(shop: &'static Shop) -> Self {
        let mut lines = Vec::new();
        let mut own_mats = false;
        let mut character = String::new();
        let mut world = String::new();
        let mut email = String::new();
        let mut note = String::new();
        for line in browser::load(STORAGE_KEY).unwrap_or_default().lines() {
            let Some((key, rest)) = line.split_once('\t') else {
                continue;
            };
            match key {
                "mats" => own_mats = rest == "1",
                "me" => character = rest.to_string(),
                "world" => world = rest.to_string(),
                "email" => email = rest.to_string(),
                "note" => note = rest.to_string(),
                qty => {
                    // Items are remembered by name, so editing shop.toml
                    // doesn't scramble anyone's saved order.
                    let index = shop.items.iter().position(|item| item.name == rest);
                    if let (Ok(qty), Some(index)) = (qty.parse::<u32>(), index)
                        && qty > 0
                        && !lines.iter().any(|(i, _)| *i == index)
                    {
                        lines.push((index, qty.min(basket::MOST_OF_ONE)));
                    }
                }
            }
        }
        // Saved before the name and world had a box each: "Name @ World".
        if world.is_empty()
            && let Some((name, saved_world)) = split_name_and_world(&character)
        {
            character = name;
            world = saved_world;
        }

        let order = Order {
            shop,
            lines: RwSignal::new(lines),
            own_mats: RwSignal::new(own_mats),
            character: RwSignal::new(character),
            world: RwSignal::new(world),
            email: RwSignal::new(email),
            note: RwSignal::new(note),
        };
        // An order left for 12 hours is gone before the page shows it.
        order.forget_if_stale();

        Effect::new(move |before: Option<String>| {
            let text = order.saved_text();
            browser::save(STORAGE_KEY, &text);
            if !order.holds_anything() {
                // Nothing left to keep.
                browser::set_cookie(KEEP_COOKIE, "", 0);
            } else if before.is_some_and(|before| before != text) {
                // A change, so the 12 hours start again. Opening the page
                // isn't a change: the order keeps the time it had.
                let until = browser::now() + f64::from(KEEP_FOR_SECONDS) * 1000.0;
                browser::set_cookie(KEEP_COOKIE, &format!("{until:.0}"), KEEP_FOR_SECONDS);
            }
            text
        });

        // While the page is open, it checks once a minute, and straight away
        // when the visitor comes back to the tab or returns to the page with
        // Back, so an order left open overnight goes too.
        let check = move || order.forget_if_stale();
        let every_minute = set_interval_with_handle(check, Duration::from_secs(60)).ok();
        let on_return = window_event_listener_untyped("visibilitychange", move |_| check());
        let on_back = window_event_listener_untyped("pageshow", move |_| check());
        on_cleanup(move || {
            if let Some(timer) = every_minute {
                timer.clear();
            }
            on_return.remove();
            on_back.remove();
        });
        order
    }

    /// Clears the whole order (items, note, materials tick and the buyer's
    /// details) once it has gone 12 hours without a change, which is when
    /// the browser deletes its cookie. Where cookies are turned off, there's
    /// no cookie to go by, so the order is kept.
    fn forget_if_stale(&self) {
        if !self.holds_anything() || !browser::cookies_enabled() {
            return;
        }
        let fresh = browser::cookie(KEEP_COOKIE)
            .and_then(|until| until.parse::<f64>().ok())
            .is_some_and(|until| browser::now() < until);
        if !fresh {
            self.lines.set(Vec::new());
            self.own_mats.set(false);
            self.character.set(String::new());
            self.world.set(String::new());
            self.email.set(String::new());
            self.note.set(String::new());
        }
    }

    /// Whether there's anything to save: items, the materials tick, the
    /// buyer's details or a note.
    fn holds_anything(&self) -> bool {
        self.lines.with_untracked(|lines| !lines.is_empty())
            || self.own_mats.get_untracked()
            || [self.character, self.world, self.email, self.note]
                .iter()
                .any(|text| text.with_untracked(|text| !text.is_empty()))
    }

    fn saved_text(&self) -> String {
        let mut text = String::new();
        for (index, qty) in self.lines.get() {
            text.push_str(&format!("{qty}\t{}\n", self.shop.items[index].name));
        }
        let clean = |s: String| s.replace(['\t', '\n'], " ");
        text.push_str(&format!("mats\t{}\n", u8::from(self.own_mats.get())));
        text.push_str(&format!("me\t{}\n", clean(self.character.get())));
        text.push_str(&format!("world\t{}\n", clean(self.world.get())));
        text.push_str(&format!("email\t{}\n", clean(self.email.get())));
        text.push_str(&format!("note\t{}\n", clean(self.note.get())));
        text
    }

    pub fn lines(&self) -> Vec<(usize, u32)> {
        self.lines.get()
    }

    pub fn is_empty(&self) -> bool {
        self.lines.with(Vec::is_empty)
    }

    /// Total number of things ordered.
    pub fn count(&self) -> u32 {
        self.lines
            .with(|lines| lines.iter().map(|(_, qty)| qty).sum())
    }

    pub fn quantity(&self, index: usize) -> u32 {
        self.lines.with(|lines| {
            lines
                .iter()
                .find(|(i, _)| *i == index)
                .map_or(0, |(_, qty)| *qty)
        })
    }

    pub fn add(&self, index: usize) {
        self.lines
            .update(|lines| match lines.iter_mut().find(|(i, _)| *i == index) {
                Some((_, qty)) => *qty = (*qty + 1).min(basket::MOST_OF_ONE),
                None => lines.push((index, 1)),
            });
    }

    /// Sets a quantity; zero removes the line.
    pub fn set(&self, index: usize, qty: u32) {
        self.lines.update(|lines| {
            if qty == 0 {
                lines.retain(|(i, _)| *i != index);
            } else if let Some(line) = lines.iter_mut().find(|(i, _)| *i == index) {
                line.1 = qty.min(basket::MOST_OF_ONE);
            }
        });
    }

    pub fn clear(&self) {
        self.lines.set(Vec::new());
    }

    /// Once the order has been emailed: the items, note and materials tick
    /// go, ready for the next order. The buyer's details stay.
    pub fn sent(&self) {
        self.lines.set(Vec::new());
        self.own_mats.set(false);
        self.note.set(String::new());
    }

    /// Whether any item in the order has a bring-your-own-materials price,
    /// even if that price is "Ask".
    pub fn mats_option(&self) -> bool {
        self.lines.with(|lines| {
            lines
                .iter()
                .any(|(i, _)| basket::has_own_mats_price(&self.shop.items[*i]))
        })
    }

    fn using_own_mats(&self) -> bool {
        self.own_mats.get() && self.mats_option()
    }

    /// The price of one, taking "I'll bring the materials" into account.
    /// None is "Ask".
    pub fn unit_price(&self, index: usize) -> Option<u64> {
        basket::unit_price(&self.shop.items[index], self.own_mats.get())
    }

    pub fn totals(&self) -> Totals {
        self.lines
            .with(|lines| Totals::of(self.shop, lines, self.own_mats.get()))
    }

    /// The longest wait in the order.
    pub fn ready(&self) -> Ready {
        self.lines
            .with(|lines| basket::longest_wait(self.shop, lines))
    }

    /// What's wrong with the buyer's details for sending the order by email,
    /// if anything, in the order the boxes come.
    pub fn problem_before_emailing(&self) -> Option<&'static str> {
        basket::note_problem(&self.note.get())
            .or_else(|| basket::character_problem(&self.character.get()))
            .or_else(|| basket::world_problem(&self.world.get()))
            .or_else(|| basket::email_problem(&self.email.get()))
    }

    /// The order as the orders service takes it, with the buyer's details
    /// tidied. `website` is the hidden box's value.
    pub fn for_email(&self, website: String) -> EmailOrder {
        EmailOrder {
            items: self
                .lines
                .get()
                .into_iter()
                .map(|(index, quantity)| Ordered {
                    name: self.shop.items[index].name.clone(),
                    quantity,
                })
                .collect(),
            own_mats: self.using_own_mats(),
            note: basket::tidy(&self.note.get()),
            character: basket::tidy(&self.character.get()),
            world: basket::tidy_world(&self.world.get()),
            email: self.email.get().trim().to_string(),
            website,
        }
    }

    /// A message to paste into Discord.
    pub fn written_message(&self) -> String {
        let mut text = format!("Hi! I'd like to order from {}:\n", self.shop.shop.name);
        for (index, qty) in self.lines.get() {
            let name = &self.shop.items[index].name;
            match self.unit_price(index) {
                Some(price) => text.push_str(&format!(
                    "- {qty}x {name} ({} gil)\n",
                    gil(price * u64::from(qty))
                )),
                None => text.push_str(&format!("- {qty}x {name} (no price yet)\n")),
            }
        }
        let totals = self.totals();
        match totals.in_words() {
            Some(total) => text.push_str(&format!("Total: {total}\n")),
            None if totals.unpriced > 0 => text.push_str("Please tell me the price.\n"),
            None => {}
        }
        if self.using_own_mats() {
            text.push_str(MATS_WRITTEN);
            text.push('\n');
        }
        let note = self.note.get();
        if !note.trim().is_empty() {
            text.push_str(note.trim());
            text.push('\n');
        }
        let me = character_line(&self.character.get(), &self.world.get());
        if !me.is_empty() {
            text.push_str(&format!("My character: {me}\n"));
        }
        text.trim_end().to_string()
    }
}

/// "Mira Ashwood @ Cerberus", or as much of it as the buyer gave.
fn character_line(name: &str, world: &str) -> String {
    let (name, world) = (basket::tidy(name), basket::tidy_world(world));
    match (name.is_empty(), world.is_empty()) {
        (false, false) => format!("{name} @ {world}"),
        (false, true) => name,
        (true, false) => format!("@ {world}"),
        (true, true) => String::new(),
    }
}

/// An order saved before the name and world had a box each held them
/// together, as "Name @ World" or "Name@World".
fn split_name_and_world(saved: &str) -> Option<(String, String)> {
    let (name, world) = saved.split_once('@')?;
    let (name, world) = (basket::tidy(name), basket::tidy(world));
    (!name.is_empty() && !world.is_empty()).then_some((name, world))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_materials_rule_is_in_plain_english() {
        for line in [MATS_WRITTEN, basket::MATS_RULE] {
            assert!(!line.contains("n't"), "no negative contractions: {line}");
            assert!(line.contains("the standard price"));
        }
        assert!(MATS_WRITTEN.starts_with("I will bring all the materials. If I cannot"));
    }

    #[test]
    fn a_saved_name_and_world_are_split_into_their_boxes() {
        assert_eq!(
            split_name_and_world("Mira Ashwood @ Cerberus"),
            Some(("Mira Ashwood".into(), "Cerberus".into()))
        );
        assert_eq!(
            split_name_and_world("Mira Ashwood@Cerberus"),
            Some(("Mira Ashwood".into(), "Cerberus".into()))
        );
        assert_eq!(split_name_and_world("Mira Ashwood"), None);
        assert_eq!(split_name_and_world("Mira Ashwood @ "), None);
    }

    #[test]
    fn the_character_line_uses_what_was_given() {
        assert_eq!(
            character_line(" Mira  Ashwood ", "cerberus"),
            "Mira Ashwood @ Cerberus"
        );
        assert_eq!(character_line("Mira Ashwood", ""), "Mira Ashwood");
        assert_eq!(character_line("", "Cerberus"), "@ Cerberus");
        assert_eq!(character_line(" ", " "), "");
    }
}
