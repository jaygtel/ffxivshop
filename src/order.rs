//! The visitor's order: what they've picked, how many, and the messages they
//! can copy to send you. It's saved in their browser so it survives a reload.

use crate::browser;
use crate::model::{Ready, Shop, gil};
use leptos::prelude::*;

const STORAGE_KEY: &str = "ffxiv-shop.order";

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
    pub gil: u64,
    /// Lines without a price ("Ask").
    pub unpriced: usize,
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
        Effect::new(move |_| browser::save(STORAGE_KEY, &order.saved_text()));
        order
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

    /// Whether any item in the order has a bring-your-own-materials price.
    pub fn mats_option(&self) -> bool {
        self.lines.with(|lines| {
            lines
                .iter()
                .any(|(i, _)| self.shop.items[*i].price_own_mats.is_some())
        })
    }

    fn using_own_mats(&self) -> bool {
        self.own_mats.get() && self.mats_option()
    }

    /// The price of one, taking "I'll bring the materials" into account.
    pub fn unit_price(&self, index: usize) -> Option<u64> {
        let item = &self.shop.items[index];
        if self.own_mats.get() {
            item.price_own_mats.or(item.price)
        } else {
            item.price
        }
    }

    pub fn totals(&self) -> Totals {
        let mut totals = Totals {
            gil: 0,
            unpriced: 0,
        };
        for (index, qty) in self.lines.get() {
            match self.unit_price(index) {
                Some(price) => totals.gil += price * u64::from(qty),
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
        if totals.gil > 0 {
            text.push_str(&format!(" That comes to {} gil", gil(totals.gil)));
            if totals.unpriced > 0 {
                text.push_str(" plus the items priced on request");
            }
            text.push('.');
        }
        if self.using_own_mats() {
            text.push_str(" I'll bring the materials.");
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
                None => text.push_str(&format!("- {qty}x {name} (price on request)\n")),
            }
        }
        let totals = self.totals();
        text.push_str(&format!("Total: {} gil", gil(totals.gil)));
        if totals.unpriced > 0 {
            text.push_str(" plus the items priced on request");
        }
        text.push('\n');
        if self.using_own_mats() {
            text.push_str("I'll bring the materials.\n");
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
