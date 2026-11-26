//! An order as the service accepts it: checked, and priced from the shop's
//! own prices (shop.toml and this week's market prices), never from anything
//! the browser sent. The checks are the order panel's own (src/basket.rs),
//! made again here because anyone can send anything to the service.

use crate::basket::{self, EmailOrder, Totals};
use crate::model::{Ready, Shop};

/// One line of the order.
#[derive(Debug, Clone, PartialEq)]
pub struct Line {
    /// The item's place in shop.toml's list.
    pub index: usize,
    pub quantity: u32,
    /// The price of one. None is "Ask".
    pub each: Option<u64>,
}

impl Line {
    /// What the line comes to. None is "Ask".
    pub fn amount(&self) -> Option<u64> {
        self.each.map(|each| each * u64::from(self.quantity))
    }
}

#[derive(Debug, Clone)]
pub struct Order {
    pub lines: Vec<Line>,
    /// Some item in the order has a price for when the buyer brings the
    /// materials, so the panel offered the tick.
    pub mats_offered: bool,
    /// The buyer will bring the materials.
    pub own_mats: bool,
    pub note: String,
    pub character: String,
    pub world: String,
    pub email: String,
    pub totals: Totals,
    pub ready: Ready,
}

impl Order {
    /// How many things are in the order, as the order panel counts them.
    pub fn count(&self) -> u32 {
        self.lines.iter().map(|line| line.quantity).sum()
    }
}

/// Checks an order from the panel against the shop. The error is what to
/// tell the buyer.
pub fn check(shop: &Shop, sent: &EmailOrder) -> Result<Order, String> {
    if sent.items.is_empty() {
        return Err("Your order is empty. Add something to it first.".into());
    }
    if sent.items.len() > shop.items.len() {
        return Err(not_in_the_shop());
    }
    let mut lines: Vec<(usize, u32)> = Vec::new();
    for ordered in &sent.items {
        let Some(index) = shop.items.iter().position(|item| item.name == ordered.name) else {
            return Err(not_in_the_shop());
        };
        if !(1..=basket::MOST_OF_ONE).contains(&ordered.quantity) {
            return Err(format!(
                "You can order 1 to {} of each item.",
                basket::MOST_OF_ONE
            ));
        }
        // The panel never sends an item twice, but if something else does,
        // the two are added together.
        match lines.iter_mut().find(|(i, _)| *i == index) {
            Some((_, qty)) => *qty = (*qty + ordered.quantity).min(basket::MOST_OF_ONE),
            None => lines.push((index, ordered.quantity)),
        }
    }
    if let Some(problem) = basket::note_problem(&sent.note)
        .or_else(|| basket::character_problem(&sent.character))
        .or_else(|| basket::world_problem(&sent.world))
        .or_else(|| basket::email_problem(&sent.email))
    {
        return Err(problem.to_string());
    }

    let mats_offered = lines
        .iter()
        .any(|&(index, _)| basket::has_own_mats_price(&shop.items[index]));
    let own_mats = sent.own_mats && mats_offered;
    Ok(Order {
        lines: lines
            .iter()
            .map(|&(index, quantity)| Line {
                index,
                quantity,
                each: basket::unit_price(&shop.items[index], own_mats),
            })
            .collect(),
        mats_offered,
        own_mats,
        note: basket::tidy(&sent.note),
        character: basket::tidy(&sent.character),
        world: basket::tidy_world(&sent.world),
        email: sent.email.trim().to_string(),
        totals: Totals::of(shop, &lines, own_mats),
        ready: basket::longest_wait(shop, &lines),
    })
}

fn not_in_the_shop() -> String {
    "Something in your order is no longer in the shop. Reload the page, then check your order and send it again.".into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basket::Ordered;

    fn shop() -> Shop {
        Shop::from_toml(include_str!("../../shop.toml")).unwrap()
    }

    fn order(items: &[(&str, u32)]) -> EmailOrder {
        EmailOrder {
            items: items
                .iter()
                .map(|(name, quantity)| Ordered {
                    name: name.to_string(),
                    quantity: *quantity,
                })
                .collect(),
            own_mats: false,
            note: " Jet Black,\n please ".into(),
            character: " Mira   Ashwood ".into(),
            world: "cerberus".into(),
            email: " mira@example.com ".into(),
            website: String::new(),
        }
    }

    #[test]
    fn an_order_is_priced_from_the_shop() {
        let shop = shop();
        let checked = check(
            &shop,
            &order(&[
                ("Courtly Lover's Rapier", 1),
                ("Grade 4 Gemdraught of Strength", 10),
                ("Semicircular Outdoor Stage", 1),
            ]),
        )
        .unwrap();
        assert_eq!(checked.count(), 12);
        let amounts: Vec<Option<u64>> = checked.lines.iter().map(Line::amount).collect();
        assert_eq!(amounts, [Some(380_000), Some(45_000), None]);
        assert_eq!(checked.totals.gil, 425_000);
        assert_eq!(checked.totals.unpriced, 1);
        assert_eq!(checked.ready.label(), "1 week");
        assert!(checked.mats_offered && !checked.own_mats);
        // Tidied as the panel tidies them.
        assert_eq!(checked.character, "Mira Ashwood");
        assert_eq!(checked.world, "Cerberus");
        assert_eq!(checked.email, "mira@example.com");
        assert_eq!(checked.note, "Jet Black, please");
    }

    #[test]
    fn bringing_the_materials_uses_the_lower_prices() {
        let shop = shop();
        let mut sent = order(&[("Courtly Lover's Rapier", 2), ("Gold Thumb's Hatchet", 1)]);
        sent.own_mats = true;
        let checked = check(&shop, &sent).unwrap();
        assert!(checked.own_mats);
        assert_eq!(checked.lines[0].each, Some(90_000));
        // No price of its own with materials: the standard one.
        assert_eq!(checked.lines[1].each, Some(129_800));
        assert_eq!(checked.totals.gil, 309_800);

        // Ticked, but nothing in the order has a price for it.
        let mut sent = order(&[("Gold Thumb's Hatchet", 1)]);
        sent.own_mats = true;
        let checked = check(&shop, &sent).unwrap();
        assert!(!checked.mats_offered && !checked.own_mats);
    }

    #[test]
    fn the_same_item_twice_is_added_together() {
        let checked = check(
            &shop(),
            &order(&[
                ("Courtly Lover's Rapier", 998),
                ("Courtly Lover's Rapier", 5),
            ]),
        )
        .unwrap();
        assert_eq!(checked.lines.len(), 1);
        assert_eq!(checked.lines[0].quantity, 999);
    }

    #[test]
    fn orders_that_cannot_be_right_are_turned_away() {
        let shop = shop();
        assert!(check(&shop, &order(&[])).unwrap_err().contains("empty"));
        assert!(
            check(&shop, &order(&[("Excalibur", 1)]))
                .unwrap_err()
                .contains("no longer in the shop")
        );
        assert!(check(&shop, &order(&[("Courtly Lover's Rapier", 0)])).is_err());
        assert!(check(&shop, &order(&[("Courtly Lover's Rapier", 1000)])).is_err());
        let too_many: Vec<(&str, u32)> = (0..=shop.items.len())
            .map(|_| ("Courtly Lover's Rapier", 1))
            .collect();
        assert!(check(&shop, &order(&too_many)).is_err());

        for (field, value) in [
            ("character", "Mira"),
            ("world", "Cerberus (Chaos)"),
            ("email", "mira@example"),
            ("note", "https://example.com"),
        ] {
            let mut sent = order(&[("Courtly Lover's Rapier", 1)]);
            match field {
                "character" => sent.character = value.into(),
                "world" => sent.world = value.into(),
                "email" => sent.email = value.into(),
                _ => sent.note = value.into(),
            }
            assert!(check(&shop, &sent).is_err(), "{field}");
        }
    }

    #[test]
    fn its_words_are_plain_english() {
        for words in [
            not_in_the_shop(),
            check(&shop(), &order(&[])).unwrap_err(),
            check(&shop(), &order(&[("Courtly Lover's Rapier", 0)])).unwrap_err(),
        ] {
            assert!(!words.contains("n't"), "{words}");
        }
    }
}
