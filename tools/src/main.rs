//! shop-tools: looks up FFXIV item details and prices for shop.toml.
//!
//!     cargo run -p shop-tools -- find courtly lover's rapier
//!     cargo run -p shop-tools -- fill
//!     cargo run -p shop-tools -- fill path/to/shop.toml
//!     cargo run -p shop-tools -- prices
//!
//! `find` prints matching items with a ready-to-paste `[[item]]` entry.
//! `fill` completes every `[[item]]` in shop.toml that has a `name`: it adds
//! the in-game `id`, the Eorzea Database `lodestone` ID, the `icon`, and for
//! gear the `ilvl`, `level` and `jobs`. Values you've already set are kept,
//! and so are your comments and layout.
//!
//! Add `--refresh` after a game patch to download fresh data. Data comes from
//! FFXIV Teamcraft's game-data files and Asvel's Eorzea Database ID list, and
//! is cached in shop-tools-cache inside the Cargo target directory.
//!
//! `prices` sets each item's `price` and `price_own_mats` from the cheapest
//! listing on your world's market board, following [prices] in shop.toml. It
//! asks Universalis every time it runs and keeps nothing (see prices.rs).

mod prices;

use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Duration;
use std::{env, fs, process};
use toml_edit::{DocumentMut, Item as TomlItem, Table, value};

const TEAMCRAFT: &str = "https://raw.githubusercontent.com/ffxiv-teamcraft/ffxiv-teamcraft/staging/libs/data/src/lib/json/";
const LODESTONE_IDS: &str =
    "https://raw.githubusercontent.com/Asvel/ffxiv-lodestone-item-id/master/lodestone-item-id.txt";

const USAGE: &str = "\
Usage:
  cargo run -p shop-tools -- find <item name words>
  cargo run -p shop-tools -- fill [path/to/shop.toml]
  cargo run -p shop-tools -- prices [path/to/shop.toml]

Options:
  --refresh   download fresh item data for find and fill (do this after a game patch)";

fn main() {
    let mut refresh = false;
    let mut args = Vec::new();
    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--refresh" => refresh = true,
            "-h" | "--help" => {
                println!("{USAGE}");
                return;
            }
            _ => args.push(arg),
        }
    }

    let result = match args.first().map(String::as_str) {
        Some("find") if args.len() > 1 => {
            Db::load(refresh).map(|db| find(&db, &args[1..].join(" ")))
        }
        Some("fill") => {
            let path = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(default_shop_toml);
            Db::load(refresh).and_then(|db| fill(&db, &path))
        }
        Some("prices") => {
            let path = args
                .get(1)
                .map(PathBuf::from)
                .unwrap_or_else(default_shop_toml);
            prices::run(&path)
        }
        _ => Err(USAGE.to_string()),
    };
    if let Err(message) = result {
        eprintln!("{message}");
        process::exit(1);
    }
}

fn default_shop_toml() -> PathBuf {
    let here = PathBuf::from("shop.toml");
    if here.exists() {
        here
    } else {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../shop.toml")
    }
}

// ---------------------------------------------------------------------------
// Item data
// ---------------------------------------------------------------------------

#[derive(Deserialize)]
struct Names {
    #[serde(default)]
    en: String,
}

#[derive(Deserialize)]
struct Equip {
    level: u16,
    #[serde(default)]
    jobs: Vec<String>,
}

struct Db {
    names: HashMap<u32, String>,
    by_name: HashMap<String, Vec<u32>>,
    lodestone: Vec<String>,
    icons: HashMap<u32, u32>,
    ilvls: HashMap<u32, u16>,
    equipment: HashMap<u32, Equip>,
    ui_category: HashMap<u32, u32>,
    ui_names: HashMap<u32, String>,
}

/// Everything we know about one item, ready to write into shop.toml.
struct Details {
    id: u32,
    name: String,
    lodestone: Option<String>,
    icon: Option<u32>,
    ilvl: Option<u16>,
    level: Option<u16>,
    jobs: Option<String>,
    kind: String,
    suggested_category: &'static str,
}

impl Db {
    fn load(refresh: bool) -> Result<Db, String> {
        let agent = http_agent();
        let get = |file: &str, url: &str| fetch_cached(&agent, file, url, refresh);
        let teamcraft = |file: &str| get(file, &format!("{TEAMCRAFT}{file}"));

        let names: HashMap<String, Names> = parse("items.json", &teamcraft("items.json")?)?;
        let icons: HashMap<String, String> =
            parse("item-icons.json", &teamcraft("item-icons.json")?)?;
        let ilvls: HashMap<String, u16> = parse("ilvls.json", &teamcraft("ilvls.json")?)?;
        let equipment: HashMap<String, Equip> =
            parse("equipment.json", &teamcraft("equipment.json")?)?;
        let ui_category: HashMap<String, u32> =
            parse("ui-categories.json", &teamcraft("ui-categories.json")?)?;
        let ui_names: HashMap<String, Names> =
            parse("item-category.json", &teamcraft("item-category.json")?)?;
        let lodestone = get("lodestone-item-id.txt", LODESTONE_IDS)?;

        fn ids<V>(map: HashMap<String, V>) -> HashMap<u32, V> {
            map.into_iter()
                .filter_map(|(k, v)| k.parse().ok().map(|id| (id, v)))
                .collect()
        }

        let names: HashMap<u32, String> = ids(names)
            .into_iter()
            .map(|(id, n): (u32, Names)| (id, n.en))
            .filter(|(_, name)| !name.trim().is_empty())
            .collect();
        let mut by_name: HashMap<String, Vec<u32>> = HashMap::new();
        for (id, name) in &names {
            by_name.entry(normalise(name)).or_default().push(*id);
        }

        Ok(Db {
            names,
            by_name,
            lodestone: lodestone.lines().map(|l| l.trim().to_string()).collect(),
            icons: ids(icons)
                .into_iter()
                .filter_map(|(id, path): (u32, String)| icon_number(&path).map(|icon| (id, icon)))
                .collect(),
            ilvls: ids(ilvls),
            equipment: ids(equipment),
            ui_category: ids(ui_category),
            ui_names: ids(ui_names)
                .into_iter()
                .map(|(id, n): (u32, Names)| (id, n.en))
                .collect(),
        })
    }

    fn lodestone_id(&self, id: u32) -> Option<String> {
        let line = self.lodestone.get(id.checked_sub(1)? as usize)?;
        (line.len() == 11).then(|| line.clone())
    }

    /// The best item for a name: prefer ones listed in the Eorzea Database,
    /// then the newest.
    fn by_exact_name(&self, name: &str) -> Option<(u32, usize)> {
        let ids = self.by_name.get(&normalise(name))?;
        let best = ids
            .iter()
            .copied()
            .max_by_key(|id| (self.lodestone_id(*id).is_some(), *id))?;
        Some((best, ids.len()))
    }

    fn search(&self, words: &str) -> Vec<u32> {
        let wanted: Vec<String> = normalise(words)
            .split_whitespace()
            .map(String::from)
            .collect();
        let mut hits: Vec<u32> = self
            .names
            .iter()
            .filter(|(_, name)| {
                let name = normalise(name);
                wanted.iter().all(|w| name.contains(w.as_str()))
            })
            .map(|(id, _)| *id)
            .collect();
        let exact = normalise(words);
        hits.sort_by_key(|id| {
            let name = normalise(&self.names[id]);
            (
                name != exact,
                !name.starts_with(&exact),
                name.len(),
                u32::MAX - id,
            )
        });
        hits
    }

    fn details(&self, id: u32) -> Details {
        let equip = self.equipment.get(&id);
        let ui = self.ui_category.get(&id).copied().unwrap_or(0);
        Details {
            id,
            name: self.names.get(&id).cloned().unwrap_or_default(),
            lodestone: self.lodestone_id(id),
            icon: self.icons.get(&id).copied(),
            // The game only shows item level on gear.
            ilvl: equip.and_then(|_| self.ilvls.get(&id).copied()),
            level: equip.map(|e| e.level),
            jobs: equip.and_then(|e| jobs_label(&e.jobs)),
            kind: self.ui_names.get(&ui).cloned().unwrap_or_default(),
            suggested_category: suggest_category(ui),
        }
    }
}

fn http_agent() -> ureq::Agent {
    use ureq::tls::{RootCerts, TlsConfig};
    ureq::Agent::config_builder()
        .tls_config(
            TlsConfig::builder()
                .root_certs(RootCerts::PlatformVerifier)
                .build(),
        )
        .timeout_global(Some(Duration::from_secs(180)))
        .user_agent("ffxiv-shop shop-tools")
        .build()
        .into()
}

/// Downloads are kept with the build output, so they survive between runs:
/// in $CARGO_TARGET_DIR if it's set (as in the dev container), else ./target.
fn cache_dir() -> PathBuf {
    let target = env::var_os("CARGO_TARGET_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| Path::new(env!("CARGO_MANIFEST_DIR")).join("../target"));
    target.join("shop-tools-cache")
}

fn fetch_cached(
    agent: &ureq::Agent,
    file: &str,
    url: &str,
    refresh: bool,
) -> Result<String, String> {
    let dir = cache_dir();
    let path = dir.join(file);
    if !refresh && let Ok(text) = fs::read_to_string(&path) {
        return Ok(text);
    }
    eprintln!("Downloading {file}...");
    let text = agent
        .get(url)
        .call()
        .and_then(|mut response| {
            response
                .body_mut()
                .with_config()
                .limit(200 * 1024 * 1024)
                .read_to_string()
        })
        .map_err(|e| format!("Couldn't download {url}: {e}"))?;
    fs::create_dir_all(&dir).map_err(|e| format!("Couldn't create {}: {e}", dir.display()))?;
    fs::write(&path, &text).map_err(|e| format!("Couldn't save {}: {e}", path.display()))?;
    Ok(text)
}

fn parse<T: serde::de::DeserializeOwned>(file: &str, text: &str) -> Result<T, String> {
    serde_json::from_str(text).map_err(|e| {
        format!("{file} isn't in the expected format ({e}). Try again with --refresh.")
    })
}

/// "/api/asset?path=ui/icon/065000/065002_hr1.tex&format=png" → 65002
fn icon_number(path: &str) -> Option<u32> {
    let file = path.split("ui/icon/").nth(1)?.split('/').nth(1)?;
    file.split(['_', '.']).next()?.parse().ok()
}

/// Lowercase with straight apostrophes, so "Lover’s" finds "Lover's".
fn normalise(text: &str) -> String {
    text.trim()
        .to_lowercase()
        .replace(['\u{2019}', '\u{2018}'], "'")
}

const BASE_CLASSES: [&str; 9] = [
    "GLA", "PGL", "MRD", "LNC", "ARC", "CNJ", "THM", "ACN", "ROG",
];
const HAND: [&str; 8] = ["CRP", "BSM", "ARM", "GSM", "LTW", "WVR", "ALC", "CUL"];
const LAND: [&str; 3] = ["MIN", "BTN", "FSH"];

/// ["GLA","MRD","PLD","WAR","DRK","GNB"] → "PLD WAR DRK GNB"
fn jobs_label(jobs: &[String]) -> Option<String> {
    if jobs.is_empty() {
        return None;
    }
    let set: HashSet<&str> = jobs.iter().map(String::as_str).collect();
    if set.contains("ADV") {
        return Some("All classes".into());
    }
    if set.len() == HAND.len() && HAND.iter().all(|j| set.contains(j)) {
        return Some("Disciples of the Hand".into());
    }
    if set.len() == LAND.len() && LAND.iter().all(|j| set.contains(j)) {
        return Some("Disciples of the Land".into());
    }
    let has_jobs = jobs.iter().any(|j| !BASE_CLASSES.contains(&j.as_str()));
    let shown: Vec<&str> = jobs
        .iter()
        .map(String::as_str)
        .filter(|j| !(has_jobs && BASE_CLASSES.contains(j)))
        .collect();
    Some(shown.join(" "))
}

/// Maps the game's item category to one of the default shop categories.
fn suggest_category(ui_category: u32) -> &'static str {
    match ui_category {
        1..=11 | 84 | 87..=89 | 96..=98 | 105..=111 | 113 => "arms",
        12..=33 | 99 => "tools",
        34..=39 => "armor",
        40..=43 => "accessories",
        44 | 46 => "consumables",
        45 | 47..=56 => "materials",
        57 | 65..=80 | 82 | 95 => "housing",
        _ => "other",
    }
}

// ---------------------------------------------------------------------------
// Commands
// ---------------------------------------------------------------------------

fn find(db: &Db, words: &str) {
    let hits = db.search(words);
    if hits.is_empty() {
        println!("No items match \"{words}\". Check the spelling against the in-game name.");
        return;
    }
    for id in hits.iter().take(8) {
        let d = db.details(*id);
        println!("# {} ({}{})", d.name, d.kind, gear_summary(&d));
        println!("[[item]]");
        println!("name = {}", toml_string(&d.name));
        println!("category = \"{}\"", d.suggested_category);
        println!("price = 0            # gil");
        println!("ready = \"1d\"");
        print_fields(&d);
        println!();
    }
    if hits.len() > 8 {
        println!(
            "...and {} more. Add more words to narrow it down.",
            hits.len() - 8
        );
    }
}

fn gear_summary(d: &Details) -> String {
    let mut parts = Vec::new();
    if let Some(ilvl) = d.ilvl {
        parts.push(format!("i{ilvl}"));
    }
    if let Some(level) = d.level {
        parts.push(format!("Lv. {level}"));
    }
    if let Some(jobs) = &d.jobs {
        parts.push(jobs.clone());
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(", {}", parts.join(", "))
    }
}

fn print_fields(d: &Details) {
    println!("id = {}", d.id);
    match &d.lodestone {
        Some(id) => println!("lodestone = \"{id}\""),
        None => println!("# not in the Eorzea Database yet; the link will search for it by name"),
    }
    if let Some(icon) = d.icon {
        println!("icon = {icon}");
    }
    if let Some(ilvl) = d.ilvl {
        println!("ilvl = {ilvl}");
    }
    if let Some(level) = d.level {
        println!("level = {level}");
    }
    if let Some(jobs) = &d.jobs {
        println!("jobs = {}", toml_string(jobs));
    }
}

fn toml_string(text: &str) -> String {
    toml_edit::Value::from(text).to_string().trim().to_string()
}

fn fill(db: &Db, path: &Path) -> Result<(), String> {
    let text =
        fs::read_to_string(path).map_err(|e| format!("Couldn't read {}: {e}", path.display()))?;
    let mut doc: DocumentMut = text
        .parse()
        .map_err(|e| format!("{} isn't valid TOML:\n{e}", path.display()))?;

    let Some(items) = doc
        .get_mut("item")
        .and_then(TomlItem::as_array_of_tables_mut)
    else {
        println!("{} has no [[item]] entries to fill in.", path.display());
        return Ok(());
    };

    let (mut filled, mut complete, mut problems) = (0, 0, Vec::new());
    for table in items.iter_mut() {
        let name = table
            .get("name")
            .and_then(TomlItem::as_str)
            .unwrap_or("")
            .to_string();
        let given_id = table.get("id").and_then(TomlItem::as_integer);

        let id = match given_id {
            Some(id) if id > 0 && db.names.contains_key(&(id as u32)) => id as u32,
            Some(id) => {
                problems.push(format!("\"{name}\": there's no item with id = {id}"));
                continue;
            }
            None => match db.by_exact_name(&name) {
                Some((id, matches)) => {
                    if matches > 1 {
                        eprintln!(
                            "Note: {matches} items are called \"{name}\"; using id {id}. Set id yourself if that's the wrong one."
                        );
                    }
                    id
                }
                None => {
                    let close: Vec<String> = db
                        .search(&name)
                        .into_iter()
                        .take(3)
                        .map(|id| format!("\"{}\"", db.names[&id]))
                        .collect();
                    let hint = if close.is_empty() {
                        "check the spelling against the in-game name".to_string()
                    } else {
                        format!("did you mean {}?", close.join(", "))
                    };
                    problems.push(format!("\"{name}\": no item has that exact name; {hint}"));
                    continue;
                }
            },
        };

        let d = db.details(id);
        let before = table.len();
        set_missing(table, "id", value(i64::from(d.id)));
        if let Some(lodestone) = &d.lodestone {
            set_missing(table, "lodestone", value(lodestone.as_str()));
        }
        if let Some(icon) = d.icon {
            set_missing(table, "icon", value(i64::from(icon)));
        }
        if let Some(ilvl) = d.ilvl {
            set_missing(table, "ilvl", value(i64::from(ilvl)));
        }
        if let Some(level) = d.level {
            set_missing(table, "level", value(i64::from(level)));
        }
        if let Some(jobs) = &d.jobs {
            set_missing(table, "jobs", value(jobs.as_str()));
        }
        if table.len() > before {
            filled += 1;
        } else {
            complete += 1;
        }
    }

    fs::write(path, doc.to_string())
        .map_err(|e| format!("Couldn't write {}: {e}", path.display()))?;
    println!(
        "Filled in {filled} item(s) in {}; {complete} were already complete.",
        path.display()
    );
    if !problems.is_empty() {
        println!("Couldn't fill {} item(s):", problems.len());
        for p in &problems {
            println!("  {p}");
        }
    }
    Ok(())
}

fn set_missing(table: &mut Table, key: &str, item: TomlItem) {
    if !table.contains_key(key) {
        table.insert(key, item);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_numbers() {
        assert_eq!(
            icon_number("/api/asset?path=ui/icon/065000/065002_hr1.tex&format=png"),
            Some(65002)
        );
        assert_eq!(icon_number("ui/icon/031000/031234.tex"), Some(31234));
        assert_eq!(icon_number("nope"), None);
    }

    #[test]
    fn job_labels() {
        let label =
            |jobs: &[&str]| jobs_label(&jobs.iter().map(|j| j.to_string()).collect::<Vec<_>>());
        assert_eq!(
            label(&["GLA", "MRD", "PLD", "WAR", "DRK", "GNB"]).unwrap(),
            "PLD WAR DRK GNB"
        );
        assert_eq!(label(&["ARC", "BRD"]).unwrap(), "BRD");
        assert_eq!(label(&["ACN"]).unwrap(), "ACN");
        assert_eq!(label(&HAND).unwrap(), "Disciples of the Hand");
        assert_eq!(label(&[]), None);
    }

    #[test]
    fn categories() {
        assert_eq!(suggest_category(97), "arms");
        assert_eq!(suggest_category(35), "armor");
        assert_eq!(suggest_category(43), "accessories");
        assert_eq!(suggest_category(30), "tools");
        assert_eq!(suggest_category(44), "consumables");
        assert_eq!(suggest_category(57), "housing");
        assert_eq!(suggest_category(49), "materials");
        assert_eq!(suggest_category(81), "other");
    }
}
