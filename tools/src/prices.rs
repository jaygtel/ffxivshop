//! `prices`: sets each item's prices from the market board on your world.
//!
//! For every [[item]] in shop.toml with an `id`, it asks Universalis for the
//! cheapest listing on the world named in [location]: HQ listings for items
//! with hq = true, NQ listings for the rest. `price` becomes that, less the
//! [prices] `undercut` percentage, rounded up to the next whole gil.
//! `price_own_mats` is that price less the [prices] `own_mats` percentage,
//! rounded up too. An item sold by the stack (per = "stack of 99") is priced
//! for the whole stack.
//!
//! Items with auto_price = false are left alone, and so is any item with no
//! listings. shop.toml is only written once every lookup has worked, and its
//! comments and layout are kept.

use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};
use toml_edit::{DocumentMut, Item as TomlItem, Table, Value};

/// Universalis's market board API. One request covers up to 100 items.
const UNIVERSALIS: &str = "https://universalis.app/api/v2/";
const ITEMS_PER_REQUEST: usize = 100;

/// Market data older than this gets a reminder to check the item in game.
const OLD_AFTER_DAYS: u64 = 7;

const NO_PRICES: &str = "\
shop.toml has no [prices] section, so shop-tools doesn't know how to set
your prices. Add one above the items, for example:

[prices]
undercut = 0.5                           # percent below the cheapest listing
own_mats = 2                             # percent off that price for buyers who bring the materials";

pub fn run(path: &Path) -> Result<(), String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| format!("{} isn't valid TOML:\n{e}", path.display()))?;

    let rule = Rule::from_doc(&doc)?;
    let (wanted, mut kept) = wanted_items(&doc);
    if wanted.is_empty() {
        println!("No items in {} to price.", path.display());
        print_kept(&kept);
        return Ok(());
    }

    let mut ids: Vec<u32> = wanted.iter().map(|w| w.id).collect();
    ids.sort_unstable();
    ids.dedup();
    eprintln!("Looking up {} item(s) on Universalis...", ids.len());
    let market = fetch(&crate::http_agent(), &rule.world, &ids)?;

    let priced = price_all(&rule, &wanted, &market, now_ms(), &mut kept);
    if !priced.is_empty() {
        apply(&mut doc, &priced);
        fs::write(path, doc.to_string())
            .map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
    }
    report(&rule, &priced, &kept, path);
    Ok(())
}

// ---------------------------------------------------------------------------
// The rule
// ---------------------------------------------------------------------------

/// Where to look and how much to take off, from shop.toml.
struct Rule {
    world: String,
    undercut: Percent,
    own_mats: Percent,
}

impl Rule {
    fn from_doc(doc: &DocumentMut) -> Result<Rule, String> {
        let world = doc
            .get("location")
            .and_then(|location| location.get("world"))
            .and_then(TomlItem::as_str)
            .map(str::trim)
            .filter(|world| !world.is_empty())
            .ok_or("shop.toml needs your world under [location], like world = \"Sagittarius\"")?
            .to_string();
        let prices = doc.get("prices").ok_or(NO_PRICES)?;
        Ok(Rule {
            world,
            undercut: Percent::read("undercut", prices.get("undercut"))?,
            own_mats: Percent::read("own_mats", prices.get("own_mats"))?,
        })
    }
}

/// A percentage, kept in millionths so the sums are exact: 0.5% is 5_000 and
/// 2% is 20_000.
#[derive(Clone, Copy, Debug, PartialEq)]
struct Percent(u64);

impl Percent {
    fn read(key: &str, item: Option<&TomlItem>) -> Result<Percent, String> {
        let number = match item.and_then(TomlItem::as_value) {
            Some(Value::Integer(n)) => *n.value() as f64,
            Some(Value::Float(f)) => *f.value(),
            Some(_) => {
                return Err(format!(
                    "[prices] {key} should be a number, like {key} = 0.5"
                ));
            }
            None => {
                return Err(format!(
                    "[prices] needs {key}, a percentage like {key} = 0.5"
                ));
            }
        };
        // The same check as the site's build (src/model.rs).
        if !(0.0..100.0).contains(&number) {
            return Err(format!(
                "[prices] {key} should be a percentage from 0 to under 100 (it's {number})"
            ));
        }
        // To the nearest 0.0001%, which is finer than a gil ever needs.
        Ok(Percent((number * 10_000.0).round().min(999_999.0) as u64))
    }

    /// `gil` less this percentage, rounded up to the next whole gil.
    fn off(self, gil: u64) -> u64 {
        let kept = u128::from(1_000_000 - self.0);
        let exact = (u128::from(gil) * kept).div_ceil(1_000_000);
        u64::try_from(exact).expect("taking a percentage off never makes a price bigger")
    }

    /// 5_000 → "0.5%"
    fn label(self) -> String {
        let text = format!("{}.{:04}", self.0 / 10_000, self.0 % 10_000);
        format!("{}%", text.trim_end_matches('0').trim_end_matches('.'))
    }
}

// ---------------------------------------------------------------------------
// Which items to price
// ---------------------------------------------------------------------------

/// An [[item]] to look up.
struct Wanted {
    /// Its place among the [[item]] tables.
    index: usize,
    name: String,
    id: u32,
    hq: bool,
    /// How many one price buys: 99 for per = "stack of 99".
    units: u64,
    per: Option<String>,
    was: Option<u64>,
}

/// The items to price, and the ones left alone with the reason why.
fn wanted_items(doc: &DocumentMut) -> (Vec<Wanted>, Vec<(String, String)>) {
    let (mut wanted, mut kept) = (Vec::new(), Vec::new());
    let Some(items) = doc.get("item").and_then(TomlItem::as_array_of_tables) else {
        return (wanted, kept);
    };
    for (index, table) in items.iter().enumerate() {
        let name = table
            .get("name")
            .and_then(TomlItem::as_str)
            .unwrap_or("(no name)")
            .to_string();
        let mut keep = |reason: &str| kept.push((name.clone(), reason.to_string()));

        if table.get("auto_price").and_then(TomlItem::as_bool) == Some(false) {
            keep("auto_price = false");
            continue;
        }
        let Some(id) = table.get("id").and_then(TomlItem::as_integer) else {
            keep("no id yet; run `cargo run -p shop-tools -- fill` first");
            continue;
        };
        let Ok(id) = u32::try_from(id) else {
            keep("its id isn't an item ID");
            continue;
        };
        let per = table
            .get("per")
            .and_then(TomlItem::as_str)
            .map(str::to_string);
        let Some(units) = units_in(per.as_deref()) else {
            keep(&format!(
                "can't tell how many per = \"{}\" is; price it yourself and set auto_price = false",
                per.unwrap_or_default()
            ));
            continue;
        };
        wanted.push(Wanted {
            index,
            name: name.clone(),
            id,
            hq: table.get("hq").and_then(TomlItem::as_bool) == Some(true),
            units,
            per,
            was: table
                .get("price")
                .and_then(TomlItem::as_integer)
                .and_then(|p| u64::try_from(p).ok()),
        });
    }
    (wanted, kept)
}

/// How many one price buys: nothing or "each" is 1, "stack of 99" is 99.
/// None when it can't be told, as with "pair" or "2 stacks of 99".
fn units_in(per: Option<&str>) -> Option<u64> {
    let Some(per) = per.map(str::trim) else {
        return Some(1);
    };
    if per.eq_ignore_ascii_case("each") {
        return Some(1);
    }
    let numbers: Vec<&str> = per
        .split(|c: char| !c.is_ascii_digit())
        .filter(|run| !run.is_empty())
        .collect();
    match numbers.as_slice() {
        [n] => n.parse().ok().filter(|n| (1..=9999).contains(n)),
        _ => None,
    }
}

// ---------------------------------------------------------------------------
// Universalis
// ---------------------------------------------------------------------------

/// The cheapest listings for one item, as Universalis reports them.
#[derive(Debug, Default, Clone, Copy, Deserialize)]
struct Market {
    /// When the item's market board was last seen, in ms since 1970 (0 if
    /// never).
    #[serde(rename = "lastUploadTime", default)]
    updated_ms: u64,
    /// Cheapest price per unit, or 0 when nothing of that quality is listed.
    #[serde(rename = "minPriceNQ", default)]
    cheapest_nq: u64,
    #[serde(rename = "minPriceHQ", default)]
    cheapest_hq: u64,
}

fn fetch(agent: &ureq::Agent, world: &str, ids: &[u32]) -> Result<HashMap<u32, Market>, String> {
    let mut found = HashMap::new();
    for batch in ids.chunks(ITEMS_PER_REQUEST) {
        let list: Vec<String> = batch.iter().map(u32::to_string).collect();
        // With more than one item the answer lists them under "items".
        let fields = if batch.len() == 1 {
            "lastUploadTime,minPriceNQ,minPriceHQ"
        } else {
            "items.lastUploadTime,items.minPriceNQ,items.minPriceHQ"
        };
        let url = format!(
            "{UNIVERSALIS}{}/{}?listings=0&entries=0&fields={fields}",
            encode(world),
            list.join(",")
        );
        let body = match agent.get(&url).call() {
            Ok(mut response) => response
                .body_mut()
                .read_to_string()
                .map_err(|e| format!("Couldn't read Universalis's answer: {e}"))?,
            // One item: it isn't sold on the market board (or the world is
            // wrong). More than one: the world is wrong.
            Err(ureq::Error::StatusCode(404)) if batch.len() == 1 => continue,
            Err(ureq::Error::StatusCode(404)) => {
                return Err(format!(
                    "Universalis doesn't know the world \"{world}\". Check [location] world in shop.toml."
                ));
            }
            Err(e) => {
                return Err(format!(
                    "Couldn't get prices from Universalis: {e}\nshop.toml hasn't been changed. Try again in a minute."
                ));
            }
        };
        found.extend(parse_market(&body, batch)?);
    }
    Ok(found)
}

/// Reads Universalis's answer for `ids`. Items it has no data for are left
/// out.
fn parse_market(body: &str, ids: &[u32]) -> Result<HashMap<u32, Market>, String> {
    #[derive(Deserialize)]
    struct Many {
        #[serde(default)]
        items: HashMap<String, Market>,
    }
    let unexpected = |e: serde_json::Error| {
        format!(
            "Universalis's answer isn't in the expected format ({e}). shop.toml hasn't been changed."
        )
    };
    if let [id] = ids {
        let one: Market = serde_json::from_str(body).map_err(unexpected)?;
        return Ok(HashMap::from([(*id, one)]));
    }
    let many: Many = serde_json::from_str(body).map_err(unexpected)?;
    Ok(many
        .items
        .into_iter()
        .filter_map(|(id, market)| id.parse().ok().map(|id| (id, market)))
        .collect())
}

/// Percent-encodes a world name for the address.
fn encode(text: &str) -> String {
    text.bytes()
        .map(|byte| match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                (byte as char).to_string()
            }
            _ => format!("%{byte:02X}"),
        })
        .collect()
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_millis() as u64)
}

// ---------------------------------------------------------------------------
// Working out and writing the prices
// ---------------------------------------------------------------------------

/// The new prices for one item.
struct Priced {
    index: usize,
    label: String,
    /// The cheapest listing, per unit.
    cheapest: u64,
    was: Option<u64>,
    price: u64,
    own_mats: u64,
    age_ms: u64,
}

fn price_all(
    rule: &Rule,
    wanted: &[Wanted],
    market: &HashMap<u32, Market>,
    now_ms: u64,
    kept: &mut Vec<(String, String)>,
) -> Vec<Priced> {
    let mut priced = Vec::new();
    for want in wanted {
        let Some(found) = market.get(&want.id).filter(|m| m.updated_ms > 0) else {
            kept.push((
                want.name.clone(),
                format!(
                    "Universalis has no market board data for it on {}",
                    rule.world
                ),
            ));
            continue;
        };
        let (cheapest, quality) = if want.hq {
            (found.cheapest_hq, "HQ")
        } else {
            (found.cheapest_nq, "NQ")
        };
        if cheapest == 0 {
            kept.push((
                want.name.clone(),
                format!("no {quality} listings on {} right now", rule.world),
            ));
            continue;
        }
        let price = rule.undercut.off(cheapest * want.units);
        let mut notes: Vec<&str> = Vec::new();
        if want.hq {
            notes.push("HQ");
        }
        if want.units > 1 {
            notes.extend(want.per.as_deref());
        }
        let label = if notes.is_empty() {
            want.name.clone()
        } else {
            format!("{} ({})", want.name, notes.join(", "))
        };
        priced.push(Priced {
            index: want.index,
            label,
            cheapest,
            was: want.was,
            price,
            own_mats: rule.own_mats.off(price),
            age_ms: now_ms.saturating_sub(found.updated_ms),
        });
    }
    priced
}

fn apply(doc: &mut DocumentMut, priced: &[Priced]) {
    let Some(items) = doc
        .get_mut("item")
        .and_then(TomlItem::as_array_of_tables_mut)
    else {
        return;
    };
    for p in priced {
        if let Some(table) = items.get_mut(p.index) {
            let after = if table.contains_key("category") {
                "category"
            } else {
                "name"
            };
            set_gil(table, "price", p.price, after);
            set_gil(table, "price_own_mats", p.own_mats, "price");
        }
    }
}

/// Writes `key = gil` the way shop.toml writes prices (89_437). A comment
/// after the old value stays. A new key goes straight after `after`.
fn set_gil(table: &mut Table, key: &str, gil: u64, after: &str) {
    let mut value: Value = grouped(gil, '_')
        .parse()
        .expect("digits and underscores are a valid TOML number");
    if let Some(old) = table.get_mut(key) {
        if let Some(old) = old.as_value() {
            *value.decor_mut() = old.decor().clone();
        }
        *old = TomlItem::Value(value);
        return;
    }
    let keys: Vec<String> = table.iter().map(|(k, _)| k.to_string()).collect();
    let at = keys
        .iter()
        .position(|k| k == after)
        .map_or(keys.len(), |i| i + 1);
    let moved: Vec<_> = keys[at..]
        .iter()
        .filter_map(|k| table.remove_entry(k))
        .collect();
    table.insert(key, TomlItem::Value(value));
    for (k, item) in moved {
        table.insert_formatted(&k, item);
    }
}

/// 89437 → "89_437" with '_', or "89,437" with ','.
fn grouped(gil: u64, separator: char) -> String {
    let digits = gil.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(separator);
        }
        out.push(c);
    }
    out
}

// ---------------------------------------------------------------------------
// What happened
// ---------------------------------------------------------------------------

fn report(rule: &Rule, priced: &[Priced], kept: &[(String, String)], path: &Path) {
    println!(
        "Prices from the {} market board, {} under the cheapest listing;",
        rule.world,
        rule.undercut.label()
    );
    println!(
        "{} less again for buyers who bring the materials. Market data from Universalis.",
        rule.own_mats.label()
    );

    if !priced.is_empty() {
        let gil = |amount: u64| grouped(amount, ',');
        let rows: Vec<[String; 6]> = priced
            .iter()
            .map(|p| {
                [
                    p.label.clone(),
                    gil(p.cheapest),
                    p.was.map_or_else(|| "Ask".to_string(), gil),
                    gil(p.price),
                    gil(p.own_mats),
                    ago(p.age_ms),
                ]
            })
            .collect();
        let head = [
            "Item",
            "Cheapest",
            "Was",
            "Price",
            "Your mats",
            "Last checked",
        ];
        let width = |col: usize| {
            rows.iter()
                .map(|row| row[col].chars().count())
                .chain([head[col].len()])
                .max()
                .unwrap_or(0)
        };
        let widths: Vec<usize> = (0..6).map(width).collect();
        let line = |cells: [&str; 6]| {
            let mut text = format!("  {:<w$}", cells[0], w = widths[0]);
            for col in 1..5 {
                text.push_str(&format!("  {:>w$}", cells[col], w = widths[col]));
            }
            text.push_str(&format!("  {}", cells[5]));
            println!("{}", text.trim_end());
        };
        println!();
        line(head);
        for row in &rows {
            line([&row[0], &row[1], &row[2], &row[3], &row[4], &row[5]].map(String::as_str));
        }

        let old: Vec<String> = priced
            .iter()
            .filter(|p| p.age_ms > OLD_AFTER_DAYS * 24 * 60 * 60 * 1000)
            .map(|p| format!("  {}, last checked {}", p.label, ago(p.age_ms)))
            .collect();
        if !old.is_empty() {
            println!(
                "\nUniversalis's data for these is over a week old, so the listings may\nhave changed. Check them in game:"
            );
            for line in old {
                println!("{line}");
            }
        }
    }

    print_kept(kept);
    if priced.is_empty() {
        println!("\nNo prices changed, so {} is as it was.", path.display());
    } else {
        println!(
            "\nWrote new prices for {} item(s) to {}. To see the changes:\n  git diff {}",
            priced.len(),
            path.display(),
            path.display()
        );
    }
}

fn print_kept(kept: &[(String, String)]) {
    if kept.is_empty() {
        return;
    }
    println!("\nLeft as they were:");
    for (name, reason) in kept {
        println!("  {name}: {reason}");
    }
}

/// How long ago, in words: "just now", "5 minutes ago", "3 hours ago",
/// "13 days ago".
fn ago(ms: u64) -> String {
    let minutes = ms / 60_000;
    let (n, unit) = match minutes {
        0 => return "just now".to_string(),
        1..=59 => (minutes, "minute"),
        60..=2879 => (minutes / 60, "hour"),
        _ => (minutes / 1440, "day"),
    };
    let s = if n == 1 { "" } else { "s" };
    format!("{n} {unit}{s} ago")
}

#[cfg(test)]
mod tests {
    use super::*;

    const HOUR: u64 = 60 * 60 * 1000;

    fn percent(text: &str) -> Result<Percent, String> {
        let doc: DocumentMut = format!("x = {text}").parse().unwrap();
        Percent::read("x", doc.get("x"))
    }

    #[test]
    fn percentages() {
        assert_eq!(percent("0.5"), Ok(Percent(5_000)));
        assert_eq!(percent("2"), Ok(Percent(20_000)));
        assert_eq!(percent("0"), Ok(Percent(0)));
        assert_eq!(percent("0.125").unwrap().label(), "0.125%");
        assert_eq!(Percent(5_000).label(), "0.5%");
        assert_eq!(Percent(20_000).label(), "2%");
        assert_eq!(percent("99.99999"), Ok(Percent(999_999)));
        for bad in ["100", "-1", "nan", "inf", "\"2\""] {
            assert!(percent(bad).is_err(), "{bad}");
        }
        let empty: DocumentMut = "".parse().unwrap();
        assert!(
            Percent::read("own_mats", empty.get("own_mats"))
                .unwrap_err()
                .contains("needs own_mats")
        );
    }

    #[test]
    fn prices_round_up_to_whole_gil() {
        let (undercut, own_mats) = (Percent(5_000), Percent(20_000));
        // 89,886 less 0.5% is 89,436.57; 89,437 less 2% is 87,648.26.
        assert_eq!(undercut.off(89_886), 89_437);
        assert_eq!(own_mats.off(89_437), 87_649);
        assert_eq!(undercut.off(400_000), 398_000);
        assert_eq!(own_mats.off(398_000), 390_040);
        assert_eq!(undercut.off(440), 438);
        assert_eq!(undercut.off(999), 995);
        // Under 200 gil, 0.5% is less than 1 gil, so rounding up keeps it.
        assert_eq!(undercut.off(199), 199);
        assert_eq!(undercut.off(200), 199);
        assert_eq!(Percent(0).off(12_345), 12_345);
        assert_eq!(undercut.off(999_999_999 * 9_999), 9_949_004_990_051);
    }

    #[test]
    fn units_per_price() {
        assert_eq!(units_in(None), Some(1));
        assert_eq!(units_in(Some("each")), Some(1));
        assert_eq!(units_in(Some(" Each ")), Some(1));
        assert_eq!(units_in(Some("stack of 99")), Some(99));
        assert_eq!(units_in(Some("2 stacks of 99")), None);
        assert_eq!(units_in(Some("pair")), None);
        assert_eq!(units_in(Some("stack of 0")), None);
    }

    #[test]
    fn reads_universalis_answers() {
        // Real answers for Sagittarius (shortened), 21 Nov 2026.
        let many = r#"{"items":{"49263":{"lastUploadTime":1795264906381,"minPriceNQ":0,"minPriceHQ":89886},"51255":{"lastUploadTime":1795268751728,"minPriceNQ":74900,"minPriceHQ":0}}}"#;
        let found = parse_market(many, &[49263, 51255, 51271]).unwrap();
        assert_eq!(found[&49263].cheapest_hq, 89_886);
        assert_eq!(found[&51255].cheapest_nq, 74_900);
        assert!(!found.contains_key(&51271));

        let one = r#"{"lastUploadTime":1795035128363,"minPriceNQ":0,"minPriceHQ":440}"#;
        let found = parse_market(one, &[49248]).unwrap();
        assert_eq!(found[&49248].cheapest_hq, 440);
        assert_eq!(found[&49248].updated_ms, 1_795_035_128_363);

        assert!(parse_market("<html>", &[1, 2]).is_err());
        assert_eq!(encode("Sagittarius"), "Sagittarius");
        assert_eq!(encode("North America"), "North%20America");
    }

    const SHOP: &str = r#"[location]
world = "Sagittarius"

[prices]
undercut = 0.5
own_mats = 2

[[item]]
name = "Courtly Lover's Rapier"
category = "arms"
price = 380_000            # was set by hand
price_own_mats = 90_000
ready = "2d"
hq = true
id = 49263

[[item]]
name = "Everkeep Sofa"
category = "housing"
ready = "3d"
id = 51256

[[item]]
name = "Grade 4 Gemdraught of Strength"
category = "consumables"
price = 4_500
per = "stack of 99"
hq = true
id = 49234

[[item]]
name = "Semicircular Outdoor Stage"
category = "housing"
auto_price = false
id = 51271

[[item]]
name = "Something New"
category = "housing"

[[item]]
name = "Quahog Chowder"
category = "consumables"
price = 2_500
hq = true
id = 49248
"#;

    #[test]
    fn writes_prices_into_shop_toml() {
        let mut doc: DocumentMut = SHOP.parse().unwrap();
        let rule = Rule::from_doc(&doc).unwrap();
        assert_eq!(rule.world, "Sagittarius");

        let (wanted, mut kept) = wanted_items(&doc);
        let ids: Vec<u32> = wanted.iter().map(|w| w.id).collect();
        assert_eq!(ids, [49263, 51256, 49234, 49248]);
        assert_eq!(wanted[2].units, 99);

        let now = 1_795_275_000_000;
        let market = HashMap::from([
            (
                49263,
                Market {
                    updated_ms: now - 3 * HOUR,
                    cheapest_nq: 0,
                    cheapest_hq: 89_886,
                },
            ),
            (
                51256,
                Market {
                    updated_ms: now - 10 * 24 * HOUR,
                    cheapest_nq: 73_446,
                    cheapest_hq: 0,
                },
            ),
            (
                49234,
                Market {
                    updated_ms: now,
                    cheapest_nq: 0,
                    cheapest_hq: 2_496,
                },
            ),
            // Only NQ listed, but the shop sells it HQ.
            (
                49248,
                Market {
                    updated_ms: now,
                    cheapest_nq: 300,
                    cheapest_hq: 0,
                },
            ),
        ]);
        let priced = price_all(&rule, &wanted, &market, now, &mut kept);
        assert_eq!(priced.len(), 3);
        assert_eq!(priced[0].label, "Courtly Lover's Rapier (HQ)");
        assert_eq!(
            priced[2].label,
            "Grade 4 Gemdraught of Strength (HQ, stack of 99)"
        );
        // A stack of 99 at 2,496 each is 247,104; less 0.5% is 245,868.48.
        assert_eq!(priced[2].price, 245_869);
        assert_eq!(ago(priced[0].age_ms), "3 hours ago");
        assert_eq!(ago(priced[1].age_ms), "10 days ago");
        assert_eq!(
            kept,
            [
                (
                    "Semicircular Outdoor Stage".to_string(),
                    "auto_price = false".to_string()
                ),
                (
                    "Something New".to_string(),
                    "no id yet; run `cargo run -p shop-tools -- fill` first".to_string()
                ),
                (
                    "Quahog Chowder".to_string(),
                    "no HQ listings on Sagittarius right now".to_string()
                ),
            ]
        );

        apply(&mut doc, &priced);
        let expected = SHOP
            .replace(
                "price = 380_000            # was set by hand\nprice_own_mats = 90_000",
                "price = 89_437            # was set by hand\nprice_own_mats = 87_649",
            )
            .replace(
                "category = \"housing\"\nready = \"3d\"",
                "category = \"housing\"\nprice = 73_079\nprice_own_mats = 71_618\nready = \"3d\"",
            )
            .replace(
                "price = 4_500\nper",
                "price = 245_869\nprice_own_mats = 240_952\nper",
            );
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn says_what_is_missing() {
        let no_prices: DocumentMut = "[location]\nworld = \"Sagittarius\"\n".parse().unwrap();
        assert!(
            Rule::from_doc(&no_prices)
                .err()
                .unwrap()
                .contains("no [prices] section")
        );
        let no_world: DocumentMut = "[prices]\nundercut = 0.5\nown_mats = 2\n".parse().unwrap();
        assert!(
            Rule::from_doc(&no_world)
                .err()
                .unwrap()
                .contains("[location]")
        );
    }

    #[test]
    fn ages_in_words() {
        assert_eq!(ago(0), "just now");
        assert_eq!(ago(60_000), "1 minute ago");
        assert_eq!(ago(45 * 60_000), "45 minutes ago");
        assert_eq!(ago(HOUR), "1 hour ago");
        assert_eq!(ago(47 * HOUR), "47 hours ago");
        assert_eq!(ago(48 * HOUR), "2 days ago");
        assert_eq!(grouped(89_437, ','), "89,437");
        assert_eq!(grouped(995, '_'), "995");
        assert_eq!(grouped(4_500, '_'), "4_500");
    }
}
