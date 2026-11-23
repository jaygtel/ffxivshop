//! `prices`: sets each item's prices from the market board on your world.
//!
//! For every [[item]] in shop.toml with an `id`, it asks Universalis for the
//! cheapest listing on the world named in [location]: HQ listings for items
//! with hq = true, NQ listings for the rest. `price` becomes that, less the
//! [prices] `undercut` percentage, rounded up to the next whole gil, and never
//! less than the item's `min_price`.
//!
//! `price_own_mats` is that price less what the materials cost: the item's
//! recipe (from FFXIV Teamcraft), with each material at its cheapest listing
//! of any quality, crystals included. A recipe that makes 3 at once takes a
//! third of its materials off each one. The result is never less than the
//! item's `min_price_own_mats`. When the materials cost as much as the price or
//! more, and there's no `min_price_own_mats`, the price with your own materials
//! is "Ask". When a material has no listings, it's the `price_own_mats`
//! written in shop.toml, or "Ask".
//!
//! An item sold by the stack (per = "stack of 99") is priced for the whole
//! stack. Items with auto_price = false are left alone, and so is any item with
//! no listings.
//!
//! The prices go either into shop.toml (its comments and layout are kept) or,
//! with `--json`, into a prices file the site reads when it opens. With
//! `--every-days`, it keeps running and does it again every so many days, as
//! the prices service on the server does. Nothing is written unless every
//! lookup has worked.

use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use toml_edit::{DocumentMut, Item as TomlItem, Table, Value};

/// Universalis's market board API. One request covers up to 100 items.
const UNIVERSALIS: &str = "https://universalis.app/api/v2/";
const ITEMS_PER_REQUEST: usize = 100;

/// Market data older than this gets a reminder to check the item in game.
const OLD_AFTER_DAYS: u64 = 7;

/// After a run that fails, how long `--every-days` waits before trying again.
const RETRY_AFTER: Duration = Duration::from_secs(60 * 60);

const DAY_MS: u64 = 24 * 60 * 60 * 1000;

const NO_PRICES: &str = "\
shop.toml has no [prices] section, so shop-tools doesn't know how to set
your prices. Add one above the items, for example:

[prices]
undercut = 0.5                           # percent below the cheapest listing";

/// What the command was asked to do.
#[derive(Debug)]
pub struct Options {
    pub shop_toml: PathBuf,
    /// Write the prices to this file for the site, rather than into shop.toml.
    pub json: Option<PathBuf>,
    /// Keep running, and update the prices again every so many days.
    pub every_days: Option<u64>,
    /// Download Teamcraft's recipes and item names again, rather than using
    /// the copies saved last time.
    pub refresh: bool,
}

impl Options {
    /// Reads the words after `prices`.
    pub fn from_args(args: &[String], refresh: bool) -> Result<Options, String> {
        let mut shop_toml = None;
        let mut json = None;
        let mut every_days = None;
        let mut words = args.iter();
        while let Some(word) = words.next() {
            match word.as_str() {
                "--json" => {
                    let file = words.next().ok_or("--json needs a file name after it")?;
                    json = Some(PathBuf::from(file));
                }
                "--every-days" => {
                    let days = words
                        .next()
                        .and_then(|days| days.parse::<u64>().ok())
                        .filter(|days| *days >= 1)
                        .ok_or(
                            "--every-days needs a number of days after it, like --every-days 7",
                        )?;
                    every_days = Some(days);
                }
                flag if flag.starts_with("--") => {
                    return Err(format!("prices doesn't know {flag}"));
                }
                path if shop_toml.is_none() => shop_toml = Some(PathBuf::from(path)),
                extra => return Err(format!("prices takes one shop.toml, not \"{extra}\" too")),
            }
        }
        if every_days.is_some() && json.is_none() {
            return Err("--every-days only works with --json".to_string());
        }
        Ok(Options {
            shop_toml: shop_toml.unwrap_or_else(crate::default_shop_toml),
            json,
            every_days,
            refresh,
        })
    }
}

pub fn run(options: &Options) -> Result<(), String> {
    let Some(days) = options.every_days else {
        return run_once(options);
    };
    loop {
        let wait = match run_once(options) {
            Ok(()) => {
                println!(
                    "Next update in {days} day{}.\n",
                    if days == 1 { "" } else { "s" }
                );
                Duration::from_millis(days * DAY_MS)
            }
            Err(message) => {
                eprintln!("{message}\nTrying again in an hour.\n");
                RETRY_AFTER
            }
        };
        thread::sleep(wait);
    }
}

fn run_once(options: &Options) -> Result<(), String> {
    let path = &options.shop_toml;
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
        if let Some(json) = &options.json {
            write_json(json, &rule, &[], now_ms())?;
        }
        return Ok(());
    }

    let agent = crate::http_agent();
    let ids: HashSet<u32> = wanted.iter().map(|w| w.id).collect();
    let teamcraft = Teamcraft::load(&agent, options.refresh, &ids)?;

    let mut lookup: Vec<u32> = ids.iter().copied().collect();
    for recipe in teamcraft.recipes.values() {
        lookup.extend(recipe.materials.iter().map(|(id, _)| *id));
    }
    lookup.sort_unstable();
    lookup.dedup();
    eprintln!(
        "Looking up {} items and materials on Universalis...",
        lookup.len()
    );
    let market = fetch(&agent, &rule.world, &lookup)?;

    let now = now_ms();
    let priced = price_all(&rule, &wanted, &teamcraft, &market, now, &mut kept);
    match &options.json {
        Some(json) => write_json(json, &rule, &priced, now)?,
        None if !priced.is_empty() => {
            apply(&mut doc, &priced);
            fs::write(path, doc.to_string())
                .map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
        }
        None => {}
    }
    report(&rule, &priced, &kept, &teamcraft, &market, now);
    match &options.json {
        Some(json) => println!(
            "\nWrote prices for {} item(s) to {} at {}.",
            priced.len(),
            json.display(),
            iso_time(now)
        ),
        None if priced.is_empty() => {
            println!("\nNo prices changed, so {} is as it was.", path.display())
        }
        None => println!(
            "\nWrote new prices for {} item(s) to {}. To see the changes:\n  git diff {}",
            priced.len(),
            path.display(),
            path.display()
        ),
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// The rule
// ---------------------------------------------------------------------------

/// Where to look and how much to take off, from shop.toml.
struct Rule {
    world: String,
    undercut: Percent,
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
        })
    }
}

/// A percentage, kept in millionths so the sums are exact: 0.5% is 5_000.
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
    min_price: Option<u64>,
    min_price_own_mats: Option<u64>,
    /// The `price_own_mats` written in shop.toml, used when a material has
    /// no listings.
    written_own_mats: Option<u64>,
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
        let gil = |key: &str| {
            table
                .get(key)
                .and_then(TomlItem::as_integer)
                .and_then(|n| u64::try_from(n).ok())
        };
        wanted.push(Wanted {
            index,
            name: name.clone(),
            id,
            hq: table.get("hq").and_then(TomlItem::as_bool) == Some(true),
            units,
            per,
            min_price: gil("min_price"),
            min_price_own_mats: gil("min_price_own_mats"),
            written_own_mats: gil("price_own_mats"),
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
// Recipes (FFXIV Teamcraft)
// ---------------------------------------------------------------------------

/// What one craft needs and makes.
#[derive(Debug, Clone, PartialEq)]
struct Recipe {
    /// How many one craft makes.
    yields: u64,
    /// Each material's item ID and how many one craft needs, crystals
    /// included.
    materials: Vec<(u32, u64)>,
}

/// The recipes for the shop's items, and every item's English name.
struct Teamcraft {
    recipes: HashMap<u32, Recipe>,
    names: HashMap<u32, String>,
}

impl Teamcraft {
    fn load(
        agent: &ureq::Agent,
        refresh: bool,
        wanted: &HashSet<u32>,
    ) -> Result<Teamcraft, String> {
        // IDs are signed because Island Sanctuary's buildings are in the
        // file with negative ones. Only real items are kept.
        #[derive(Deserialize)]
        struct Row {
            result: i64,
            #[serde(default = "one")]
            yields: u64,
            #[serde(default)]
            ingredients: Vec<Ingredient>,
        }
        #[derive(Deserialize)]
        struct Ingredient {
            id: i64,
            amount: u64,
        }
        fn one() -> u64 {
            1
        }

        let get = |file: &str| {
            crate::fetch_cached(agent, file, &format!("{}{file}", crate::TEAMCRAFT), refresh)
        };
        let rows: Vec<Row> = crate::parse("recipes.json", &get("recipes.json")?)?;
        let mut recipes = HashMap::new();
        for row in rows {
            let Ok(result) = u32::try_from(row.result) else {
                continue;
            };
            // Every item in the shop so far has one recipe. If one ever has
            // more, the first is used.
            if wanted.contains(&result) && !recipes.contains_key(&result) {
                let materials = row
                    .ingredients
                    .iter()
                    .filter(|ingredient| ingredient.amount > 0)
                    .filter_map(|ingredient| {
                        u32::try_from(ingredient.id)
                            .ok()
                            .map(|id| (id, ingredient.amount))
                    })
                    .collect();
                recipes.insert(
                    result,
                    Recipe {
                        yields: row.yields.max(1),
                        materials,
                    },
                );
            }
        }
        let names: HashMap<String, crate::Names> = crate::parse("items.json", &get("items.json")?)?;
        let names = names
            .into_iter()
            .filter_map(|(id, names)| id.parse().ok().map(|id| (id, names.en)))
            .collect();
        Ok(Teamcraft { recipes, names })
    }

    fn name(&self, id: u32) -> String {
        self.names
            .get(&id)
            .filter(|name| !name.is_empty())
            .cloned()
            .unwrap_or_else(|| format!("item {id}"))
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
    #[serde(rename = "minPrice", default)]
    cheapest: u64,
    #[serde(rename = "minPriceNQ", default)]
    cheapest_nq: u64,
    #[serde(rename = "minPriceHQ", default)]
    cheapest_hq: u64,
}

impl Market {
    /// The cheapest listing of any quality, if Universalis has seen any.
    fn any(&self) -> Option<u64> {
        (self.updated_ms > 0 && self.cheapest > 0).then_some(self.cheapest)
    }
}

fn fetch(agent: &ureq::Agent, world: &str, ids: &[u32]) -> Result<HashMap<u32, Market>, String> {
    let mut found = HashMap::new();
    for batch in ids.chunks(ITEMS_PER_REQUEST) {
        let list: Vec<String> = batch.iter().map(u32::to_string).collect();
        // With more than one item the answer lists them under "items".
        let fields = if batch.len() == 1 {
            "lastUploadTime,minPrice,minPriceNQ,minPriceHQ"
        } else {
            "items.lastUploadTime,items.minPrice,items.minPriceNQ,items.minPriceHQ"
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
                    "Couldn't get prices from Universalis: {e}\nNothing has been changed."
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
            "Universalis's answer isn't in the expected format ({e}). Nothing has been changed."
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
// Working out the prices
// ---------------------------------------------------------------------------

/// The price when the buyer brings the materials.
#[derive(Debug, Clone, Copy, PartialEq)]
enum OwnMats {
    Gil(u64),
    /// Shown as "Ask": I'll tell them the price.
    Ask,
    /// No recipe, so no price with your own materials is worked out.
    NoRecipe,
}

/// What the materials for one priced item came to.
#[derive(Debug, Clone, PartialEq)]
enum Materials {
    /// Gil for everything one price buys (a recipe that makes 3 at once
    /// counts a third of its materials for each one).
    Cost(u64),
    /// These materials have no listings.
    Unlisted(Vec<u32>),
    NoRecipe,
}

/// The new prices for one item.
struct Priced {
    index: usize,
    label: String,
    id: u32,
    hq: bool,
    /// The cheapest listing, per unit.
    cheapest: u64,
    price: u64,
    materials: Materials,
    own_mats: OwnMats,
    /// A recipe that makes more than one at once.
    yields: u64,
    age_ms: u64,
}

fn price_all(
    rule: &Rule,
    wanted: &[Wanted],
    teamcraft: &Teamcraft,
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
        let price = rule
            .undercut
            .off(cheapest * want.units)
            .max(want.min_price.unwrap_or(0));
        let recipe = teamcraft.recipes.get(&want.id);
        let materials = match recipe {
            Some(recipe) => materials_cost(recipe, market, want.units),
            None => Materials::NoRecipe,
        };
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
            id: want.id,
            hq: want.hq,
            cheapest,
            price,
            own_mats: own_mats(price, &materials, want),
            materials,
            yields: recipe.map_or(1, |recipe| recipe.yields),
            age_ms: now_ms.saturating_sub(found.updated_ms),
        });
    }
    priced
}

/// What the materials for `units` of an item cost: each one at its cheapest
/// listing of any quality, times how many the recipe needs, for as many
/// crafts as `units` takes (a recipe that makes 3 at once needs a third of a
/// craft for each one). Rounded down, so the price that's left rounds up.
fn materials_cost(recipe: &Recipe, market: &HashMap<u32, Market>, units: u64) -> Materials {
    let mut per_craft: u64 = 0;
    let mut unlisted = Vec::new();
    for &(id, amount) in &recipe.materials {
        match market.get(&id).and_then(Market::any) {
            Some(gil) => per_craft += gil * amount,
            None => unlisted.push(id),
        }
    }
    if !unlisted.is_empty() {
        return Materials::Unlisted(unlisted);
    }
    Materials::Cost(per_craft * units / recipe.yields)
}

/// The price when the buyer brings the materials: the price less the
/// materials, never under the item's `min_price_own_mats` or over its price.
fn own_mats(price: u64, materials: &Materials, want: &Wanted) -> OwnMats {
    let floor = want.min_price_own_mats.map(|min| min.min(price));
    match materials {
        Materials::NoRecipe => OwnMats::NoRecipe,
        Materials::Unlisted(_) => want
            .written_own_mats
            .map_or(OwnMats::Ask, |gil| OwnMats::Gil(gil.min(price))),
        Materials::Cost(cost) => match (price.saturating_sub(*cost), floor) {
            (0, None) => OwnMats::Ask,
            (left, floor) => OwnMats::Gil(left.max(floor.unwrap_or(0))),
        },
    }
}

// ---------------------------------------------------------------------------
// Writing the prices
// ---------------------------------------------------------------------------

/// The file the site reads (src/prices.rs reads the same shape).
#[derive(Serialize)]
struct PricesFile<'a> {
    /// When the prices were worked out, in UTC.
    updated: String,
    world: &'a str,
    items: Vec<FileItem>,
}

#[derive(Serialize)]
struct FileItem {
    id: u32,
    hq: bool,
    price: u64,
    /// Left out when there's no recipe, so the site keeps shop.toml's.
    #[serde(skip_serializing_if = "Option::is_none")]
    price_own_mats: Option<FileOwnMats>,
}

#[derive(Serialize)]
#[serde(untagged)]
enum FileOwnMats {
    Gil(u64),
    /// Always "ask".
    Word(&'static str),
}

fn json_text(rule: &Rule, priced: &[Priced], now_ms: u64) -> String {
    let file = PricesFile {
        updated: iso_time(now_ms),
        world: &rule.world,
        items: priced
            .iter()
            .map(|p| FileItem {
                id: p.id,
                hq: p.hq,
                price: p.price,
                price_own_mats: match p.own_mats {
                    OwnMats::Gil(gil) => Some(FileOwnMats::Gil(gil)),
                    OwnMats::Ask => Some(FileOwnMats::Word("ask")),
                    OwnMats::NoRecipe => None,
                },
            })
            .collect(),
    };
    let mut text = serde_json::to_string_pretty(&file).expect("prices always turn into JSON");
    text.push('\n');
    text
}

/// Writes the whole file under a temporary name, then swaps it in, so the
/// site never reads half a file.
fn write_json(path: &Path, rule: &Rule, priced: &[Priced], now_ms: u64) -> Result<(), String> {
    let mut temporary = path.as_os_str().to_owned();
    temporary.push(".new");
    let temporary = PathBuf::from(temporary);
    fs::write(&temporary, json_text(rule, priced, now_ms))
        .map_err(|e| format!("Couldn't write {}: {e}", temporary.display()))?;
    fs::rename(&temporary, path).map_err(|e| format!("Couldn't write {}: {e}", path.display()))
}

/// Writes the prices into shop.toml. A price with your own materials that's
/// "Ask", or has no recipe, leaves what's there.
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
            if let OwnMats::Gil(gil) = p.own_mats {
                set_gil(table, "price_own_mats", gil, "price");
            }
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

/// ms since 1970 → "2026-11-23T10:26:49Z"
fn iso_time(ms: u64) -> String {
    let seconds = ms / 1000;
    let (days, rest) = (seconds / 86_400, seconds % 86_400);
    // Days since 1970 to a date (Howard Hinnant's civil_from_days).
    let z = days as i64 + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    let year = year_of_era + era * 400 + i64::from(month <= 2);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        rest / 3600,
        rest % 3600 / 60,
        rest % 60
    )
}

// ---------------------------------------------------------------------------
// What happened
// ---------------------------------------------------------------------------

fn report(
    rule: &Rule,
    priced: &[Priced],
    kept: &[(String, String)],
    teamcraft: &Teamcraft,
    market: &HashMap<u32, Market>,
    now_ms: u64,
) {
    println!(
        "Prices from the {} market board, {} under the cheapest listing. With your",
        rule.world,
        rule.undercut.label()
    );
    println!("own materials: the price less what the materials cost there. Market data from");
    println!("Universalis; recipes from FFXIV Teamcraft.");

    if !priced.is_empty() {
        let gil = |amount: u64| grouped(amount, ',');
        let rows: Vec<[String; 6]> = priced
            .iter()
            .map(|p| {
                [
                    p.label.clone(),
                    gil(p.cheapest),
                    gil(p.price),
                    match &p.materials {
                        Materials::Cost(cost) => gil(*cost),
                        Materials::Unlisted(_) => "not listed".to_string(),
                        Materials::NoRecipe => "no recipe".to_string(),
                    },
                    match p.own_mats {
                        OwnMats::Gil(own) => gil(own),
                        OwnMats::Ask => "Ask".to_string(),
                        OwnMats::NoRecipe => "-".to_string(),
                    },
                    ago(p.age_ms),
                ]
            })
            .collect();
        let head = [
            "Item",
            "Cheapest",
            "Price",
            "Materials",
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

        let batches: Vec<String> = priced
            .iter()
            .filter(|p| p.yields > 1 && matches!(p.materials, Materials::Cost(_)))
            .map(|p| format!("{} (makes {})", p.label, p.yields))
            .collect();
        if !batches.is_empty() {
            println!(
                "\nThese recipes make more than one at once, so each one counts its share of\nthe materials: {}.",
                batches.join(", ")
            );
        }

        let costly: Vec<&str> = priced
            .iter()
            .filter(|p| p.own_mats == OwnMats::Ask && matches!(p.materials, Materials::Cost(_)))
            .map(|p| p.label.as_str())
            .collect();
        if !costly.is_empty() {
            println!(
                "\nThe materials cost as much as the price or more, so the price with your own\nmaterials is Ask (set min_price_own_mats to give it a price): {}.",
                costly.join(", ")
            );
        }

        let unlisted: Vec<String> = priced
            .iter()
            .filter_map(|p| match &p.materials {
                Materials::Unlisted(ids) => Some(format!(
                    "  {}: no listings for {}; {}",
                    p.label,
                    ids.iter()
                        .map(|id| teamcraft.name(*id))
                        .collect::<Vec<_>>()
                        .join(", "),
                    match p.own_mats {
                        OwnMats::Ask => "Ask".to_string(),
                        _ => "used price_own_mats from shop.toml".to_string(),
                    }
                )),
                _ => None,
            })
            .collect();
        if !unlisted.is_empty() {
            println!("\nSome materials have no listings right now:");
            for line in unlisted {
                println!("{line}");
            }
        }

        let old_after = OLD_AFTER_DAYS * DAY_MS;
        let mut old: Vec<String> = priced
            .iter()
            .filter(|p| p.age_ms > old_after)
            .map(|p| format!("  {}, last checked {}", p.label, ago(p.age_ms)))
            .collect();
        let mut old_materials: Vec<(u32, u64)> = priced
            .iter()
            .filter_map(|p| teamcraft.recipes.get(&p.id))
            .flat_map(|recipe| recipe.materials.iter().map(|(id, _)| *id))
            .collect::<HashSet<u32>>()
            .into_iter()
            .filter_map(|id| {
                let seen = market.get(&id)?.updated_ms;
                let age = now_ms.saturating_sub(seen);
                (seen > 0 && age > old_after).then_some((id, age))
            })
            .collect();
        old_materials.sort_unstable();
        old.extend(old_materials.into_iter().map(|(id, age)| {
            format!(
                "  {} (a material), last checked {}",
                teamcraft.name(id),
                ago(age)
            )
        }));
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

    fn args(words: &str) -> Result<Options, String> {
        let words: Vec<String> = words.split_whitespace().map(str::to_string).collect();
        Options::from_args(&words, false)
    }

    fn listing(updated_ms: u64, cheapest: u64, nq: u64, hq: u64) -> Market {
        Market {
            updated_ms,
            cheapest,
            cheapest_nq: nq,
            cheapest_hq: hq,
        }
    }

    fn wanted(min_price_own_mats: Option<u64>, written_own_mats: Option<u64>) -> Wanted {
        Wanted {
            index: 0,
            name: "Thing".to_string(),
            id: 1,
            hq: true,
            units: 1,
            per: None,
            min_price: None,
            min_price_own_mats,
            written_own_mats,
        }
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
    }

    #[test]
    fn prices_round_up_to_whole_gil() {
        let undercut = Percent(5_000);
        // 89,852 less 0.5% is 89,402.74.
        assert_eq!(undercut.off(89_852), 89_403);
        assert_eq!(undercut.off(400_000), 398_000);
        assert_eq!(undercut.off(440), 438);
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
    fn command_words() {
        let plain = args("").unwrap();
        assert!(plain.json.is_none() && plain.every_days.is_none());
        let server = args("--json /data/prices.json --every-days 7 /app/shop.toml").unwrap();
        assert_eq!(server.json, Some(PathBuf::from("/data/prices.json")));
        assert_eq!(server.every_days, Some(7));
        assert_eq!(server.shop_toml, PathBuf::from("/app/shop.toml"));
        assert!(
            args("--every-days 7")
                .unwrap_err()
                .contains("only works with --json")
        );
        assert!(args("--json").is_err());
        assert!(args("--json a.json --every-days 0").is_err());
        assert!(args("--json a.json --every-days soon").is_err());
        assert!(args("--weekly").unwrap_err().contains("--weekly"));
        assert!(args("a.toml b.toml").is_err());
    }

    #[test]
    fn reads_universalis_answers() {
        // Real answers for Sagittarius, 21 and 23 Nov 2026 (shortened).
        let many = r#"{"items":{"49263":{"lastUploadTime":1795264906381,"minPrice":89886,"minPriceNQ":0,"minPriceHQ":89886},"51255":{"lastUploadTime":1795268751728,"minPrice":74900,"minPriceNQ":74900,"minPriceHQ":0}}}"#;
        let found = parse_market(many, &[49263, 51255, 51271]).unwrap();
        assert_eq!(found[&49263].cheapest_hq, 89_886);
        assert_eq!(found[&51255].cheapest_nq, 74_900);
        assert_eq!(found[&51255].any(), Some(74_900));
        assert!(!found.contains_key(&51271));

        let one =
            r#"{"lastUploadTime":1795035128363,"minPrice":440,"minPriceNQ":0,"minPriceHQ":440}"#;
        let found = parse_market(one, &[49248]).unwrap();
        assert_eq!(found[&49248].cheapest_hq, 440);
        assert_eq!(found[&49248].updated_ms, 1_795_035_128_363);

        assert_eq!(listing(0, 500, 500, 0).any(), None);
        assert_eq!(listing(1, 0, 0, 0).any(), None);
        assert!(parse_market("<html>", &[1, 2]).is_err());
        assert_eq!(encode("Sagittarius"), "Sagittarius");
        assert_eq!(encode("North America"), "North%20America");
    }

    /// The Courtly Lover's Rapier's recipe, with Sagittarius's prices on
    /// 23 Nov 2026.
    fn rapier() -> (Recipe, HashMap<u32, Market>) {
        let recipe = Recipe {
            yields: 1,
            materials: vec![
                (49215, 3), // Rose Garnet
                (49214, 2), // Double Duraluminum Ingot
                (44001, 1), // Ra'Kaznar Ingot
                (49221, 2), // Grade 4 Gemsap of Intelligence
                (46246, 2), // Levinchrome Aethersand
                (16, 3),    // Wind Cluster
                (14, 3),    // Fire Cluster
            ],
        };
        let market = HashMap::from([
            (49215, listing(1, 15_998, 15_998, 0)),
            (49214, listing(1, 9_992, 9_992, 0)),
            (44001, listing(1, 699, 699, 0)),
            (49221, listing(1, 1_732, 1_732, 0)),
            (46246, listing(1, 1_888, 1_888, 0)),
            (16, listing(1, 76, 76, 0)),
            (14, listing(1, 30, 30, 0)),
        ]);
        (recipe, market)
    }

    #[test]
    fn materials_come_off_the_price() {
        let (recipe, market) = rapier();
        let materials = materials_cost(&recipe, &market, 1);
        assert_eq!(materials, Materials::Cost(76_235));
        assert_eq!(
            own_mats(89_403, &materials, &wanted(None, None)),
            OwnMats::Gil(13_168)
        );
        // Two of them: the site multiplies, 2 × 89,403 − 2 × 76,235.
        assert_eq!(2 * 89_403 - 2 * 76_235, 2 * 13_168);
    }

    #[test]
    fn a_recipe_that_makes_three_counts_a_third() {
        let recipe = Recipe {
            yields: 3,
            materials: vec![(1, 1), (2, 2)],
        };
        let market = HashMap::from([(1, listing(1, 1_000, 1_000, 0)), (2, listing(1, 50, 0, 50))]);
        // One craft costs 1,100 and makes 3: 366.67 each, rounded down.
        assert_eq!(materials_cost(&recipe, &market, 1), Materials::Cost(366));
        // A stack of 99 is 33 crafts.
        assert_eq!(
            materials_cost(&recipe, &market, 99),
            Materials::Cost(36_300)
        );
    }

    #[test]
    fn prices_with_your_own_materials() {
        let costs = |gil| Materials::Cost(gil);
        // Materials cost more than the price: Ask, or the minimum.
        assert_eq!(
            own_mats(1_000, &costs(1_500), &wanted(None, None)),
            OwnMats::Ask
        );
        assert_eq!(
            own_mats(1_000, &costs(1_000), &wanted(None, None)),
            OwnMats::Ask
        );
        assert_eq!(
            own_mats(1_000, &costs(1_500), &wanted(Some(200), None)),
            OwnMats::Gil(200)
        );
        // The minimum also lifts a small difference, but never past the price.
        assert_eq!(
            own_mats(1_000, &costs(950), &wanted(Some(200), None)),
            OwnMats::Gil(200)
        );
        assert_eq!(
            own_mats(1_000, &costs(500), &wanted(Some(200), None)),
            OwnMats::Gil(500)
        );
        assert_eq!(
            own_mats(1_000, &costs(1_500), &wanted(Some(5_000), None)),
            OwnMats::Gil(1_000)
        );
        // A material with no listings: shop.toml's price, or Ask.
        let unlisted = Materials::Unlisted(vec![7]);
        assert_eq!(
            own_mats(1_000, &unlisted, &wanted(None, None)),
            OwnMats::Ask
        );
        assert_eq!(
            own_mats(1_000, &unlisted, &wanted(None, Some(300))),
            OwnMats::Gil(300)
        );
        assert_eq!(
            own_mats(1_000, &Materials::NoRecipe, &wanted(None, None)),
            OwnMats::NoRecipe
        );

        let (recipe, mut market) = rapier();
        market.remove(&46246);
        assert_eq!(
            materials_cost(&recipe, &market, 1),
            Materials::Unlisted(vec![46246])
        );
        market.insert(46246, listing(1, 0, 0, 0));
        assert_eq!(
            materials_cost(&recipe, &market, 1),
            Materials::Unlisted(vec![46246])
        );
    }

    #[test]
    fn times_in_utc() {
        assert_eq!(iso_time(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_time(951_782_400_000), "2000-02-29T00:00:00Z");
        assert_eq!(iso_time(1_795_429_609_978), "2026-11-23T10:26:49Z");
        assert_eq!(iso_time(4_102_444_799_000), "2099-12-31T23:59:59Z");
    }

    const SHOP: &str = r#"[location]
world = "Sagittarius"

[prices]
undercut = 0.5

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
min_price = 80_000
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

    fn shop_run(doc: &DocumentMut) -> (Rule, Vec<Priced>, Vec<(String, String)>, Teamcraft) {
        let rule = Rule::from_doc(doc).unwrap();
        let (wanted, mut kept) = wanted_items(doc);
        let (rapier_recipe, mut market) = rapier();
        let teamcraft = Teamcraft {
            recipes: HashMap::from([
                (49263, rapier_recipe),
                // The sofa's materials come to more than its price.
                (
                    51256,
                    Recipe {
                        yields: 1,
                        materials: vec![(49215, 6)],
                    },
                ),
                (
                    49234,
                    Recipe {
                        yields: 3,
                        materials: vec![(14, 3), (16, 3)],
                    },
                ),
            ]),
            names: HashMap::from([(49215, "Rose Garnet".to_string())]),
        };
        let now = 1_795_429_609_978;
        market.insert(49263, listing(now - 3 * HOUR, 89_852, 0, 89_852));
        market.insert(51256, listing(now - 10 * 24 * HOUR, 73_446, 73_446, 0));
        market.insert(49234, listing(now, 2_496, 0, 2_496));
        // Only NQ listed, but the shop sells it HQ.
        market.insert(49248, listing(now, 300, 300, 0));
        let priced = price_all(&rule, &wanted, &teamcraft, &market, now, &mut kept);
        (rule, priced, kept, teamcraft)
    }

    #[test]
    fn works_out_a_whole_shop() {
        let doc: DocumentMut = SHOP.parse().unwrap();
        let (_, priced, kept, _) = shop_run(&doc);
        assert_eq!(priced.len(), 3);

        assert_eq!(priced[0].label, "Courtly Lover's Rapier (HQ)");
        assert_eq!(priced[0].price, 89_403);
        assert_eq!(priced[0].own_mats, OwnMats::Gil(13_168));
        assert_eq!(ago(priced[0].age_ms), "3 hours ago");

        // The sofa's market price is under its min_price, and its materials
        // cost more than that.
        assert_eq!(priced[1].price, 80_000);
        assert_eq!(priced[1].materials, Materials::Cost(95_988));
        assert_eq!(priced[1].own_mats, OwnMats::Ask);
        assert_eq!(ago(priced[1].age_ms), "10 days ago");

        // A stack of 99 at 2,496 each is 247,104; less 0.5% is 245,868.48.
        // Its recipe makes 3, so the stack takes 33 crafts of 318 gil.
        assert_eq!(
            priced[2].label,
            "Grade 4 Gemdraught of Strength (HQ, stack of 99)"
        );
        assert_eq!(priced[2].price, 245_869);
        assert_eq!(priced[2].materials, Materials::Cost(10_494));
        assert_eq!(priced[2].own_mats, OwnMats::Gil(235_375));

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
    }

    #[test]
    fn writes_prices_into_shop_toml() {
        let mut doc: DocumentMut = SHOP.parse().unwrap();
        let (_, priced, _, _) = shop_run(&doc);
        apply(&mut doc, &priced);
        // The sofa's own-materials price is Ask, which shop.toml can't hold,
        // so it gets none.
        let expected = SHOP
            .replace(
                "price = 380_000            # was set by hand\nprice_own_mats = 90_000",
                "price = 89_403            # was set by hand\nprice_own_mats = 13_168",
            )
            .replace(
                "category = \"housing\"\nready = \"3d\"",
                "category = \"housing\"\nprice = 80_000\nready = \"3d\"",
            )
            .replace(
                "price = 4_500\nper",
                "price = 245_869\nprice_own_mats = 235_375\nper",
            );
        assert_eq!(doc.to_string(), expected);
    }

    #[test]
    fn writes_the_prices_file() {
        let doc: DocumentMut = SHOP.parse().unwrap();
        let (rule, priced, _, _) = shop_run(&doc);
        // The same text as the site's test (src/prices.rs) reads.
        let expected = r#"{
  "updated": "2026-11-23T10:26:49Z",
  "world": "Sagittarius",
  "items": [
    {
      "id": 49263,
      "hq": true,
      "price": 89403,
      "price_own_mats": 13168
    },
    {
      "id": 51256,
      "hq": false,
      "price": 80000,
      "price_own_mats": "ask"
    },
    {
      "id": 49234,
      "hq": true,
      "price": 245869,
      "price_own_mats": 235375
    }
  ]
}
"#;
        assert_eq!(json_text(&rule, &priced, 1_795_429_609_978), expected);

        let dir = std::env::temp_dir().join(format!("shop-tools-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("prices.json");
        fs::write(&path, "old").unwrap();
        write_json(&path, &rule, &priced, 1_795_429_609_978).unwrap();
        assert_eq!(fs::read_to_string(&path).unwrap(), expected);
        assert!(!dir.join("prices.json.new").exists());
        fs::remove_dir_all(&dir).unwrap();
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
        let no_world: DocumentMut = "[prices]\nundercut = 0.5\n".parse().unwrap();
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
