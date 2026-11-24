//! An order's sums and checks, without the page: what each item costs, the
//! total, the longest wait, and whether the details a buyer types in look
//! right. The order panel (src/order.rs and src/components/order_slip.rs)
//! and the orders service (orders/, which emails the receipt) both compile
//! this file, so the receipt always says what the panel said. Keep it free
//! of browser-only code.

use crate::model::{Item, Ready, Shop, gil};
use serde::{Deserialize, Serialize};

/// Where the order panel sends an order to be emailed: the orders service,
/// behind the site's own web server (see deploy/nginx.conf).
pub const SEND_ADDRESS: &str = "/api/order";

/// The rule for bringing materials, in the shop's words: under the tick in
/// the order panel, and on the receipt.
pub const MATS_RULE: &str = "You must bring all the materials, not just some. If you cannot, I will use my own and charge the standard price.";

/// The most characters the order panel's "Anything else?" box takes.
pub const NOTE_LIMIT: usize = 200;

/// The most of one item an order can hold, as the order panel allows.
pub const MOST_OF_ONE: u32 = 999;

/// Whether the item has a price for when the buyer brings the materials,
/// even if that price is "Ask".
pub fn has_own_mats_price(item: &Item) -> bool {
    item.price_own_mats.is_some() || item.own_mats_ask
}

/// The price of one, with or without the buyer's materials. None is "Ask".
pub fn unit_price(item: &Item, own_mats: bool) -> Option<u64> {
    if !own_mats {
        item.price
    } else if item.own_mats_ask {
        None
    } else {
        item.price_own_mats.or(item.price)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Totals {
    /// What the items with a price come to.
    pub gil: u64,
    /// Lines with a price.
    pub priced: usize,
    /// Lines with no price yet ("Ask").
    pub unpriced: usize,
}

impl Totals {
    /// Adds up an order's lines: (item index, quantity) pairs.
    pub fn of(shop: &Shop, lines: &[(usize, u32)], own_mats: bool) -> Totals {
        let mut totals = Totals {
            gil: 0,
            priced: 0,
            unpriced: 0,
        };
        for &(index, qty) in lines {
            match unit_price(&shop.items[index], own_mats) {
                Some(price) => {
                    totals.gil += price * u64::from(qty);
                    totals.priced += 1;
                }
                None => totals.unpriced += 1,
            }
        }
        totals
    }

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

/// The longest wait among an order's lines.
pub fn longest_wait(shop: &Shop, lines: &[(usize, u32)]) -> Ready {
    lines
        .iter()
        .map(|&(index, _)| shop.ready_of(&shop.items[index]))
        .max()
        .unwrap_or(Ready::NOW)
}

// ---------------------------------------------------------------------------
// Sending an order by email
// ---------------------------------------------------------------------------

/// An order to email, as the order panel sends it to the orders service.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct EmailOrder {
    /// What's in the order, by the items' names in shop.toml, the way the
    /// order panel remembers them.
    pub items: Vec<Ordered>,
    /// The buyer ticked "I'll bring the materials".
    #[serde(default)]
    pub own_mats: bool,
    /// What they wrote under "Anything else?".
    #[serde(default)]
    pub note: String,
    pub character: String,
    pub world: String,
    /// Where their copy of the receipt goes.
    pub email: String,
    /// A box that's hidden on the page, so people never fill it in. Bots
    /// that fill in every box do.
    #[serde(default)]
    pub website: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Ordered {
    pub name: String,
    pub quantity: u32,
}

/// The orders service's answer.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Answer {
    /// The order number, once the order is sent.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub number: Option<String>,
    /// What went wrong, in words the order panel shows the buyer.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// Text as the buyer meant it: no spaces at the ends, one space between
/// words, and no line breaks or tabs.
pub fn tidy(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

/// "cerberus" or "CERBERUS" becomes "Cerberus", as the game writes worlds.
pub fn tidy_world(world: &str) -> String {
    let world = tidy(world);
    let mut letters = world.chars();
    match letters.next() {
        Some(first) => first
            .to_uppercase()
            .chain(letters.flat_map(char::to_lowercase))
            .collect(),
        None => world,
    }
}

/// What's wrong with a character's name, if anything. The game gives every
/// character a first and a last name, each 2 to 15 letters (apostrophes and
/// hyphens allowed), with no more than 20 between them.
pub fn character_problem(name: &str) -> Option<&'static str> {
    let name = tidy(name);
    if name.is_empty() {
        return Some("Enter your character's name.");
    }
    let names: Vec<&str> = name.split(' ').collect();
    if names.len() != 2 {
        return Some("Enter your character's first and last name, like Mira Ashwood.");
    }
    if !names.iter().all(|part| {
        part.chars()
            .all(|c| c.is_alphabetic() || c == '\'' || c == '-')
    }) {
        return Some("A character's name has only letters, apostrophes and hyphens.");
    }
    let lengths: Vec<usize> = names.iter().map(|part| part.chars().count()).collect();
    if lengths.iter().any(|n| !(2..=15).contains(n)) || lengths.iter().sum::<usize>() > 20 {
        return Some(
            "Check your character's name. Each name has 2 to 15 letters, and 20 or fewer between them.",
        );
    }
    None
}

/// What's wrong with a world's name, if anything.
pub fn world_problem(world: &str) -> Option<&'static str> {
    let world = tidy(world);
    if world.is_empty() {
        return Some("Enter your world, like Cerberus.");
    }
    if !(3..=20).contains(&world.chars().count()) || !world.chars().all(char::is_alphabetic) {
        return Some("Enter only the name of your world, like Cerberus.");
    }
    None
}

/// What's wrong with an email address, if anything. It only checks the
/// address looks like one: a mistyped address can still look right.
pub fn email_problem(email: &str) -> Option<&'static str> {
    let email = email.trim();
    if email.is_empty() {
        return Some("Enter your email address, so I can send you a copy.");
    }
    (!looks_like_an_email(email))
        .then_some("Check your email address. It should look like name@example.com.")
}

/// Whether an email address looks like one, with nothing else around it.
/// The contact page's checks (src/message.rs) use it too.
pub fn looks_like_an_email(email: &str) -> bool {
    let Some((local, domain)) = email.split_once('@') else {
        return false;
    };
    let fine = |c: char| !(c.is_whitespace() || c.is_control() || "<>()[]\\,;:\"@".contains(c));
    let labels: Vec<&str> = domain.split('.').collect();
    email.len() <= 254
        && (1..=64).contains(&local.len())
        && local.chars().all(fine)
        && !local.starts_with('.')
        && !local.ends_with('.')
        && !local.contains("..")
        && labels.len() >= 2
        && labels.iter().all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label.chars().all(|c| c.is_alphanumeric() || c == '-')
        })
        && labels
            .last()
            .is_some_and(|top| top.chars().count() >= 2 && top.chars().all(char::is_alphabetic))
}

/// What's wrong with the note, if anything. It goes in the copy emailed to
/// whatever address the buyer gives, so it can't carry web links: otherwise
/// anyone could send links to strangers from the shop's address.
pub fn note_problem(note: &str) -> Option<&'static str> {
    let note = tidy(note);
    if note.chars().count() > NOTE_LIMIT {
        return Some("Keep \"Anything else?\" to 200 characters or fewer.");
    }
    let lower = note.to_lowercase();
    (lower.contains("://") || lower.contains("www."))
        .then_some("Leave web addresses out of \"Anything else?\".")
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn prices_with_and_without_the_buyers_materials() {
        let shop = Shop::from_toml(include_str!("../shop.toml")).unwrap();
        let index = |name: &str| {
            shop.items
                .iter()
                .position(|item| item.name == name)
                .unwrap()
        };
        let rapier = index("Courtly Lover's Rapier");
        let stage = index("Semicircular Outdoor Stage");
        let hatchet = index("Gold Thumb's Hatchet");
        assert_eq!(unit_price(&shop.items[rapier], false), Some(380_000));
        assert_eq!(unit_price(&shop.items[rapier], true), Some(90_000));
        // No price of its own with materials: the standard price.
        assert_eq!(unit_price(&shop.items[hatchet], true), Some(95_000));
        assert!(!has_own_mats_price(&shop.items[hatchet]));
        assert_eq!(unit_price(&shop.items[stage], true), None);

        let lines = [(rapier, 2), (stage, 1)];
        let sum = Totals::of(&shop, &lines, true);
        assert_eq!((sum.gil, sum.priced, sum.unpriced), (180_000, 1, 1));
        assert_eq!(longest_wait(&shop, &lines).label(), "1 week");
        assert_eq!(longest_wait(&shop, &[]), Ready::NOW);

        let mut asks = shop.items[rapier].clone();
        asks.own_mats_ask = true;
        asks.price_own_mats = None;
        assert_eq!(unit_price(&asks, true), None);
        assert!(has_own_mats_price(&asks));
    }

    #[test]
    fn character_names_follow_the_games_rules() {
        assert_eq!(character_problem("Mira Ashwood"), None);
        assert_eq!(character_problem("  Mira   Ashwood "), None);
        assert_eq!(character_problem("Y'shtola Rhul"), None);
        assert_eq!(character_problem("Ul-dah Born"), None);
        assert!(character_problem("").is_some());
        assert!(character_problem("Mira").is_some());
        assert!(character_problem("Mira Ashwood Smith").is_some());
        assert!(character_problem("Mira@Cerberus Ashwood").is_some());
        assert!(character_problem("M Ashwood").is_some());
        assert!(character_problem("Abcdefghijklmnop Ashwood").is_some());
        // 11 + 10 letters is over 20.
        assert!(character_problem("Abcdefghijk Abcdefghij").is_some());
        assert_eq!(character_problem("Abcdefghij Abcdefghij"), None);
    }

    #[test]
    fn worlds_are_one_word() {
        assert_eq!(world_problem("Cerberus"), None);
        assert_eq!(world_problem(" sagittarius "), None);
        assert!(world_problem("").is_some());
        assert!(world_problem("Cerberus (Chaos)").is_some());
        assert!(world_problem("Ce").is_some());
        assert_eq!(tidy_world("  sAGITTARIUS "), "Sagittarius");
        assert_eq!(tidy_world("cerberus"), "Cerberus");
        assert_eq!(tidy_world(""), "");
    }

    #[test]
    fn email_addresses_must_look_like_one() {
        for good in [
            "mira@example.com",
            "mira.ashwood+shop@mail.example.co.uk",
            " mira@example.com ",
            "o'neill@example.ie",
        ] {
            assert_eq!(email_problem(good), None, "{good}");
        }
        for bad in [
            "",
            "mira",
            "mira@",
            "@example.com",
            "mira@example",
            "mira@example.c",
            "mira@@example.com",
            "mira ashwood@example.com",
            "mira@exa mple.com",
            "mira@-example.com",
            "mira@example..com",
            ".mira@example.com",
            "mira..ashwood@example.com",
            "\"mira\"@example.com",
            "mira@example.com, other@example.com",
            "mira@example.com\r\nBcc: other@example.com",
            "<mira@example.com>",
        ] {
            assert!(email_problem(bad).is_some(), "{bad:?}");
        }
    }

    #[test]
    fn notes_are_tidied_and_kept_short() {
        assert_eq!(
            tidy(" Jet\tBlack dye,\r\n please "),
            "Jet Black dye, please"
        );
        assert_eq!(note_problem(&"a".repeat(NOTE_LIMIT)), None);
        assert!(note_problem(&"a".repeat(NOTE_LIMIT + 1)).is_some());
        assert_eq!(note_problem("Jet Black dye, two in HQ. Thanks!"), None);
        assert!(note_problem("See https://example.com").is_some());
        assert!(note_problem("go to WWW.example.com").is_some());
    }

    #[test]
    fn the_words_here_are_plain_english() {
        let messages = [
            MATS_RULE,
            character_problem("").unwrap(),
            character_problem("Mira").unwrap(),
            character_problem("Mira 4shwood").unwrap(),
            character_problem("M Ashwood").unwrap(),
            world_problem("").unwrap(),
            world_problem("Cerberus (Chaos)").unwrap(),
            email_problem("").unwrap(),
            email_problem("mira").unwrap(),
            note_problem(&"a".repeat(NOTE_LIMIT + 1)).unwrap(),
            note_problem("www.example.com").unwrap(),
        ];
        for line in messages {
            assert!(!line.contains("n't"), "no negative contractions: {line}");
        }
    }

    #[test]
    fn orders_travel_as_json() {
        let order = EmailOrder {
            items: vec![Ordered {
                name: "Courtly Lover's Rapier".into(),
                quantity: 2,
            }],
            own_mats: true,
            note: "Jet Black".into(),
            character: "Mira Ashwood".into(),
            world: "Cerberus".into(),
            email: "mira@example.com".into(),
            website: String::new(),
        };
        let text = serde_json::to_string(&order).unwrap();
        assert_eq!(serde_json::from_str::<EmailOrder>(&text).unwrap(), order);
        // The hidden box and the optional parts can be left out.
        let short: EmailOrder = serde_json::from_str(
            r#"{"items":[],"character":"Mira Ashwood","world":"Cerberus","email":"mira@example.com"}"#,
        )
        .unwrap();
        assert!(!short.own_mats && short.website.is_empty());
        let sent = Answer {
            number: Some("AA-2611-7K3Q".into()),
            problem: None,
        };
        assert_eq!(
            serde_json::to_string(&sent).unwrap(),
            r#"{"number":"AA-2611-7K3Q"}"#
        );
    }
}
