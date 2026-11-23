//! The visitor's order: what they've picked, how many, and the messages they
//! can copy to send you. It's saved in their browser so it survives a reload,
//! and cleared once it has gone 12 hours without a change.

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
/// bring all of them. The /tell has to fit on one chat line, so it's shorter.
/// The order panel says the same, under the tick (order_slip.rs).
const MATS_TELL: &str =
    "I will bring all the materials. If I cannot, I will pay the standard price.";
const MATS_WRITTEN: &str = "I will bring all the materials. If I cannot bring them all, you will craft my order at the standard price, with your own materials. I know you will not take only some of the materials.";
const KEEP_FOR_SECONDS: u32 = 12 * 60 * 60;

#[derive(Clone, Copy)]
pub struct Order {
    shop: &'static Shop,
    /// (item index, quantity), in the order they were added.
    lines: RwSignal<Vec<(usize, u32)>>,
    /// The buyer will bring the materials.
    pub own_mats: RwSignal<bool>,
    /// The buyer's character, for out-of-game messages.
    pub character: RwSignal<String>,
    pub note: RwSignal<String>,
}

pub struct Totals {
    /// What the items with a price come to.
    pub gil: u64,
    /// Lines with a price.
    pub priced: usize,
    /// Lines with no price yet ("Ask").
    pub unpriced: usize,
}

impl Totals {
    /// The total for a message: "12,000 gil", or "12,000 gil, plus the items
    /// with no price yet". None when nothing has a price yet, because a total
    /// of 0 gil would look free.
    pub fn in_words(&self) -> Option<String> {
        if self.priced == 0 {
            return None;
        }
        let mut words = format!("{} gil", gil(self.gil));
        if self.unpriced > 0 {
            words.push_str(", plus the items with no price yet");
        }
        Some(words)
    }
}

impl Order {
    pub fn new(shop: &'static Shop) -> Self {
        let mut lines = Vec::new();
        let mut own_mats = false;
        let mut character = String::new();
        let mut note = String::new();
        for line in browser::load(STORAGE_KEY).unwrap_or_default().lines() {
            let Some((key, rest)) = line.split_once('\t') else {
                continue;
            };
            match key {
                "mats" => own_mats = rest == "1",
                "me" => character = rest.to_string(),
                "note" => note = rest.to_string(),
                qty => {
                    // Items are remembered by name, so editing shop.toml
                    // doesn't scramble anyone's saved order.
                    let index = shop.items.iter().position(|item| item.name == rest);
                    if let (Ok(qty), Some(index)) = (qty.parse::<u32>(), index)
                        && qty > 0
                        && !lines.iter().any(|(i, _)| *i == index)
                    {
                        lines.push((index, qty.min(999)));
                    }
                }
            }
        }

        let order = Order {
            shop,
            lines: RwSignal::new(lines),
            own_mats: RwSignal::new(own_mats),
            character: RwSignal::new(character),
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

    /// Clears the whole order (items, note, materials tick and character)
    /// once it has gone 12 hours without a change, which is when the browser
    /// deletes its cookie. Where cookies are turned off, there's no cookie to
    /// go by, so the order is kept.
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
            self.note.set(String::new());
        }
    }

    /// Whether there's anything to save: items, the materials tick, a
    /// character name or a note.
    fn holds_anything(&self) -> bool {
        self.lines.with_untracked(|lines| !lines.is_empty())
            || self.own_mats.get_untracked()
            || self.character.with_untracked(|me| !me.is_empty())
            || self.note.with_untracked(|note| !note.is_empty())
    }

    fn saved_text(&self) -> String {
        let mut text = String::new();
        for (index, qty) in self.lines.get() {
            text.push_str(&format!("{qty}\t{}\n", self.shop.items[index].name));
        }
        let clean = |s: String| s.replace(['\t', '\n'], " ");
        text.push_str(&format!("mats\t{}\n", u8::from(self.own_mats.get())));
        text.push_str(&format!("me\t{}\n", clean(self.character.get())));
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
                Some((_, qty)) => *qty = (*qty + 1).min(999),
                None => lines.push((index, 1)),
            });
    }

    /// Sets a quantity; zero removes the line.
    pub fn set(&self, index: usize, qty: u32) {
        self.lines.update(|lines| {
            if qty == 0 {
                lines.retain(|(i, _)| *i != index);
            } else if let Some(line) = lines.iter_mut().find(|(i, _)| *i == index) {
                line.1 = qty.min(999);
            }
        });
    }

    pub fn clear(&self) {
        self.lines.set(Vec::new());
    }

    /// Whether any item in the order has a bring-your-own-materials price,
    /// even if that price is "Ask".
    pub fn mats_option(&self) -> bool {
        self.lines.with(|lines| {
            lines.iter().any(|(i, _)| {
                let item = &self.shop.items[*i];
                item.price_own_mats.is_some() || item.own_mats_ask
            })
        })
    }

    fn using_own_mats(&self) -> bool {
        self.own_mats.get() && self.mats_option()
    }

    /// The price of one, taking "I'll bring the materials" into account.
    /// None is "Ask".
    pub fn unit_price(&self, index: usize) -> Option<u64> {
        let item = &self.shop.items[index];
        if !self.own_mats.get() {
            item.price
        } else if item.own_mats_ask {
            None
        } else {
            item.price_own_mats.or(item.price)
        }
    }

    pub fn totals(&self) -> Totals {
        let mut totals = Totals {
            gil: 0,
            priced: 0,
            unpriced: 0,
        };
        for (index, qty) in self.lines.get() {
            match self.unit_price(index) {
                Some(price) => {
                    totals.gil += price * u64::from(qty);
                    totals.priced += 1;
                }
                None => totals.unpriced += 1,
            }
        }
        totals
    }

    /// The longest wait in the order.
    pub fn ready(&self) -> Ready {
        self.lines.with(|lines| {
            lines
                .iter()
                .map(|(i, _)| self.shop.ready_of(&self.shop.items[*i]))
                .max()
                .unwrap_or(Ready::NOW)
        })
    }

    /// One line to paste into the game's chat box.
    pub fn tell_message(&self) -> String {
        let list: Vec<String> = self
            .lines
            .get()
            .iter()
            .map(|(i, qty)| format!("{qty}x {}", self.shop.items[*i].name))
            .collect();
        let mut text = format!(
            "{} Hi! I'd like to order: {}.",
            self.shop.tell_command(),
            list.join(", ")
        );
        let totals = self.totals();
        match totals.in_words() {
            Some(total) => text.push_str(&format!(" That comes to {total}.")),
            None if totals.unpriced > 0 => text.push_str(" Please tell me the price."),
            None => {}
        }
        if self.using_own_mats() {
            text.push(' ');
            text.push_str(MATS_TELL);
        }
        let note = self.note.get();
        if !note.trim().is_empty() {
            text.push(' ');
            text.push_str(note.trim());
        }
        text
    }

    /// A message for Discord or email.
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
        let me = self.character.get();
        if !me.trim().is_empty() {
            text.push_str(&format!("My character: {}\n", me.trim()));
        }
        text.trim_end().to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::Totals;

    fn totals(gil: u64, priced: usize, unpriced: usize) -> Totals {
        Totals {
            gil,
            priced,
            unpriced,
        }
    }

    #[test]
    fn a_priced_order_gives_its_total() {
        assert_eq!(
            totals(12_000, 2, 0).in_words().as_deref(),
            Some("12,000 gil")
        );
    }

    #[test]
    fn items_with_no_price_are_named_after_the_total() {
        assert_eq!(
            totals(12_000, 1, 1).in_words().as_deref(),
            Some("12,000 gil, plus the items with no price yet")
        );
    }

    #[test]
    fn an_order_with_no_prices_has_no_total_rather_than_zero() {
        assert_eq!(totals(0, 0, 2).in_words(), None);
    }

    #[test]
    fn the_materials_rule_is_in_plain_english() {
        for line in [super::MATS_TELL, super::MATS_WRITTEN] {
            assert!(!line.contains("n't"), "no negative contractions: {line}");
            assert!(line.starts_with("I will bring all the materials. If I cannot"));
            assert!(line.contains("the standard price"));
        }
        // The /tell's one chat line also carries the order itself.
        assert!(super::MATS_TELL.len() < 100);
    }
}
