//! Everything the shop shows comes from `shop.toml`. This module reads and
//! checks that file.
//!
//! It is compiled into the website and into `build.rs`, so a mistake in
//! `shop.toml` stops the build with a readable message instead of shipping a
//! broken page, and into the orders service (orders/), which reads the same
//! file to price an order it emails. Keep it free of browser-only code.

use serde::Deserialize;
use std::collections::BTreeMap;

/// Square Enix's official script that shows an Eorzea Database tooltip when a
/// visitor hovers a link to the database.
pub const EDB_TOOLTIP_SCRIPT: &str =
    "https://img.finalfantasyxiv.com/lds/pc/global/js/eorzeadb/loader.js?v2";

/// Disciples of the Hand and Land, in the order the game lists them.
pub const CRAFTERS: [(&str, &str); 11] = [
    ("CRP", "Carpenter"),
    ("BSM", "Blacksmith"),
    ("ARM", "Armorer"),
    ("GSM", "Goldsmith"),
    ("LTW", "Leatherworker"),
    ("WVR", "Weaver"),
    ("ALC", "Alchemist"),
    ("CUL", "Culinarian"),
    ("MIN", "Miner"),
    ("BTN", "Botanist"),
    ("FSH", "Fisher"),
];

fn yes() -> bool {
    true
}

fn owned_by() -> String {
    "Run by".to_string()
}

// ---------------------------------------------------------------------------
// The file
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Shop {
    pub site: Site,
    pub shop: ShopInfo,
    pub location: Location,
    pub contact: Contact,
    #[serde(default)]
    pub prices: Option<Prices>,
    #[serde(default, rename = "category")]
    pub categories: Vec<Category>,
    #[serde(default, rename = "item")]
    pub items: Vec<Item>,
}

/// About the website itself, as opposed to the shop in the game.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Site {
    /// Where the site lives, e.g. "https://aetherandallure.com".
    #[serde(default)]
    pub address: Option<String>,
    /// Who owns the copyright in the site's design. Shown in the footer.
    pub copyright: String,
    /// The year the copyright notice starts from.
    pub since: u16,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ShopInfo {
    /// The shop's name, shown large at the top of the page.
    pub name: String,
    /// Shown in «angle quotes» above the name, like an NPC title.
    #[serde(default)]
    pub title: Option<String>,
    /// The character who owns the shop, e.g. "Sigrid Halvorsen".
    pub owner: String,
    /// How the owner is described, e.g. "Owned and designed by".
    #[serde(default = "owned_by")]
    pub owner_role: String,
    #[serde(default)]
    pub tagline: Option<String>,
    #[serde(default)]
    pub status: Status,
    #[serde(default)]
    pub status_note: Option<String>,
    /// Used for any item without its own `ready` time.
    #[serde(default)]
    pub default_ready: Option<Ready>,
    /// Job abbreviation → level, e.g. `GSM = 100`.
    #[serde(default)]
    pub crafters: BTreeMap<String, u16>,
    /// The steps shown under "How ordering works".
    #[serde(default)]
    pub how_to_order: Vec<String>,
    /// Which Lodestone the item links open: na, eu, fr, de or jp.
    #[serde(default)]
    pub lodestone: Region,
    /// Show item icons (loaded from XIVAPI).
    #[serde(default = "yes")]
    pub icons: bool,
    /// Load Square Enix's Eorzea Database hover tooltips.
    #[serde(default = "yes")]
    pub tooltips: bool,
    /// Shows a "sample shop" notice while you're still filling things in.
    #[serde(default)]
    pub demo: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Location {
    pub data_center: String,
    pub world: String,
    pub district: District,
    pub ward: u8,
    pub plot: u8,
    /// "Small", "Medium" or "Large".
    #[serde(default)]
    pub size: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Contact {
    /// For example how quickly you usually reply.
    #[serde(default)]
    pub note: Option<String>,
    /// Show a ready-made `/tell Name@World` command under Request a craft.
    #[serde(default = "yes")]
    pub tell: bool,
    /// The order panel's Send by email button, which needs the orders service
    /// (see compose.yaml) and its settings in .env.
    #[serde(default)]
    pub send_by_email: bool,
    #[serde(default)]
    pub friend_request: bool,
    /// Visitors can leave a note in your estate's message book.
    #[serde(default)]
    pub message_book: bool,
    /// A linkshell or cross-world linkshell people can find you in.
    #[serde(default)]
    pub linkshell: Option<String>,
    #[serde(default)]
    pub discord: Option<String>,
    /// An invite link, starting with https://
    #[serde(default)]
    pub discord_server: Option<String>,
    #[serde(default)]
    pub email: Option<String>,
    #[serde(default)]
    pub links: Vec<Link>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Link {
    pub label: String,
    pub url: String,
}

/// How shop-tools sets prices from the market board (`prices`). The page
/// doesn't use this; it's read here so a mistake stops the build.
#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Prices {
    /// Percent below the cheapest listing, e.g. 0.5.
    pub undercut: f64,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Category {
    /// Short id used by items and in links such as `#housing`.
    pub key: String,
    pub label: String,
    /// The picture shown when an item has no icon.
    #[serde(default)]
    pub glyph: Glyph,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// The in-game name.
    pub name: String,
    pub category: String,
    /// Price in gil. Leave it out to show "Ask".
    #[serde(default)]
    pub price: Option<u64>,
    /// Price if the buyer brings the materials.
    #[serde(default)]
    pub price_own_mats: Option<u64>,
    /// false keeps the prices written here: shop-tools leaves them alone, and
    /// the site doesn't take market prices for it from prices.json.
    #[serde(default = "yes")]
    pub auto_price: bool,
    /// The lowest price shop-tools may set. Only shop-tools reads it.
    #[serde(default)]
    #[allow(dead_code)]
    pub min_price: Option<u64>,
    /// The lowest price with the buyer's own materials that shop-tools may
    /// set. Only shop-tools reads it.
    #[serde(default)]
    #[allow(dead_code)]
    pub min_price_own_mats: Option<u64>,
    /// The market prices (prices.json) say "Ask" for the price with the
    /// buyer's own materials. Never in shop.toml: see src/prices.rs.
    #[serde(skip)]
    pub own_mats_ask: bool,
    /// What the price buys, e.g. "each" or "stack of 99".
    #[serde(default)]
    pub per: Option<String>,
    /// How long until it's ready for delivery: "now", "6h", "2d", "1w"...
    #[serde(default)]
    pub ready: Option<Ready>,
    /// Offered in high quality.
    #[serde(default)]
    pub hq: bool,
    #[serde(default)]
    pub note: Option<String>,
    /// The in-game item ID.
    #[serde(default)]
    pub id: Option<u32>,
    /// The Eorzea Database ID, e.g. "6246afd2ba5". A full database address works too.
    #[serde(default)]
    pub lodestone: Option<LodestoneId>,
    /// The game's icon number (shop-tools fills this in).
    #[serde(default)]
    pub icon: Option<u32>,
    #[serde(default)]
    pub ilvl: Option<u16>,
    /// Level needed to equip it.
    #[serde(default)]
    pub level: Option<u16>,
    /// Who can use it, e.g. "RDM" or "PLD WAR DRK GNB".
    #[serde(default)]
    pub jobs: Option<String>,
}

// ---------------------------------------------------------------------------
// Small value types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Status {
    #[default]
    Open,
    Busy,
    Closed,
}

impl Status {
    pub fn label(self) -> &'static str {
        match self {
            Status::Open => "Taking orders",
            Status::Busy => "Busy, orders may take longer",
            Status::Closed => "Not taking orders right now",
        }
    }

    pub fn class(self) -> &'static str {
        match self {
            Status::Open => "status-open",
            Status::Busy => "status-busy",
            Status::Closed => "status-closed",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Region {
    #[default]
    Na,
    Eu,
    Fr,
    De,
    Jp,
}

impl Region {
    pub fn host(self) -> &'static str {
        match self {
            Region::Na => "na.finalfantasyxiv.com",
            Region::Eu => "eu.finalfantasyxiv.com",
            Region::Fr => "fr.finalfantasyxiv.com",
            Region::De => "de.finalfantasyxiv.com",
            Region::Jp => "jp.finalfantasyxiv.com",
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Glyph {
    Sword,
    Shield,
    Armor,
    Ring,
    Tool,
    Potion,
    Food,
    House,
    Gem,
    #[default]
    Crate,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub enum District {
    Mist,
    LavenderBeds,
    Goblet,
    Shirogane,
    Empyreum,
}

impl TryFrom<String> for District {
    type Error = String;

    fn try_from(raw: String) -> Result<Self, String> {
        let squashed: String = raw
            .to_lowercase()
            .chars()
            .filter(|c| c.is_alphanumeric())
            .collect();
        let key = squashed.strip_prefix("the").unwrap_or(&squashed);
        match key {
            "mist" => Ok(District::Mist),
            "lavenderbeds" => Ok(District::LavenderBeds),
            "goblet" => Ok(District::Goblet),
            "shirogane" => Ok(District::Shirogane),
            "empyreum" => Ok(District::Empyreum),
            _ => Err(format!(
                "\"{raw}\" isn't a housing district; use Mist, The Lavender Beds, The Goblet, Shirogane or Empyreum"
            )),
        }
    }
}

impl District {
    pub fn name(self) -> &'static str {
        match self {
            District::Mist => "Mist",
            District::LavenderBeds => "The Lavender Beds",
            District::Goblet => "The Goblet",
            District::Shirogane => "Shirogane",
            District::Empyreum => "Empyreum",
        }
    }

    /// The Teleport destination whose aetheryte offers the ward list.
    pub fn aetheryte(self) -> &'static str {
        match self {
            District::Mist => "Limsa Lominsa Lower Decks",
            District::LavenderBeds => "New Gridania",
            District::Goblet => "Ul'dah - Steps of Nald",
            District::Shirogane => "Kugane",
            District::Empyreum => "Foundation",
        }
    }
}

/// An Eorzea Database item ID: the 11 characters after `/db/item/` in an
/// item's Lodestone address.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "String")]
pub struct LodestoneId(String);

impl TryFrom<String> for LodestoneId {
    type Error = String;

    fn try_from(raw: String) -> Result<Self, String> {
        let text = raw.trim();
        // A whole address pasted from the Lodestone works too.
        let id = match text.find("/db/item/") {
            Some(at) => text[at + "/db/item/".len()..]
                .split(['/', '?', '#'])
                .next()
                .unwrap_or(""),
            None => text,
        };
        if id.len() == 11 && id.chars().all(|c| c.is_ascii_hexdigit()) {
            Ok(LodestoneId(id.to_ascii_lowercase()))
        } else {
            Err(format!(
                "\"{raw}\" isn't an Eorzea Database item ID; it looks like \"6246afd2ba5\", the 11 characters after /db/item/ in the item's Lodestone address"
            ))
        }
    }
}

impl LodestoneId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// How long until an item is ready for delivery, in minutes.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize)]
#[serde(try_from = "String")]
pub struct Ready {
    pub minutes: u32,
}

impl TryFrom<String> for Ready {
    type Error = String;

    fn try_from(raw: String) -> Result<Self, String> {
        raw.parse()
    }
}

impl std::str::FromStr for Ready {
    type Err = String;

    /// Reads "now", "in stock", "45m", "6h", "2d", "1w", "1d 12h",
    /// "2 days", "1 day and 12 hours" or "1.5d".
    fn from_str(raw: &str) -> Result<Self, String> {
        let text = raw.trim().to_ascii_lowercase();
        let unreadable = || {
            format!(
                "can't read the ready time \"{raw}\"; write it like \"now\", \"6h\", \"2d\", \"1d 12h\" or \"1w\""
            )
        };
        if matches!(text.as_str(), "now" | "in stock" | "ready" | "0") {
            return Ok(Ready { minutes: 0 });
        }

        let mut rest = text.as_str();
        let mut total = 0.0_f64;
        let mut parts = 0;
        loop {
            rest = rest.trim_start_matches(|c: char| c.is_whitespace() || c == ',');
            if rest.is_empty() {
                break;
            }
            if let Some(after) = rest.strip_prefix("and ") {
                rest = after;
                continue;
            }
            let number_len = rest
                .find(|c: char| !(c.is_ascii_digit() || c == '.'))
                .unwrap_or(rest.len());
            if number_len == 0 {
                return Err(unreadable());
            }
            let amount: f64 = rest[..number_len].parse().map_err(|_| unreadable())?;
            rest = rest[number_len..].trim_start();
            let unit_len = rest
                .find(|c: char| !c.is_ascii_alphabetic())
                .unwrap_or(rest.len());
            let minutes_per = match &rest[..unit_len] {
                "m" | "min" | "mins" | "minute" | "minutes" => 1.0,
                "h" | "hr" | "hrs" | "hour" | "hours" => 60.0,
                "d" | "day" | "days" => 1440.0,
                "w" | "wk" | "wks" | "week" | "weeks" => 10080.0,
                _ => return Err(unreadable()),
            };
            rest = &rest[unit_len..];
            total += amount * minutes_per;
            parts += 1;
        }
        if parts == 0 || !total.is_finite() || total > f64::from(u32::MAX) {
            return Err(unreadable());
        }
        Ok(Ready {
            minutes: total.round() as u32,
        })
    }
}

/// How soon something will be ready, for colouring.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Soon {
    Now,
    WithinADay,
    Later,
}

impl Ready {
    pub const NOW: Ready = Ready { minutes: 0 };

    pub fn soon(self) -> Soon {
        match self.minutes {
            0 => Soon::Now,
            1..=1440 => Soon::WithinADay,
            _ => Soon::Later,
        }
    }

    /// "In stock", "45 minutes", "6 hours", "1 day 12 hours", "2 weeks".
    pub fn label(self) -> String {
        const DAY: u32 = 1440;
        const WEEK: u32 = 7 * DAY;
        let m = self.minutes;
        if m == 0 {
            return "In stock".into();
        }
        if m < 60 {
            return count(m, "minute");
        }
        if m < DAY {
            let (h, rest) = (m / 60, m % 60);
            return if rest == 0 {
                count(h, "hour")
            } else {
                format!("{} {}", count(h, "hour"), count(rest, "minute"))
            };
        }
        if m.is_multiple_of(WEEK) {
            return count(m / WEEK, "week");
        }
        let (d, h) = (m / DAY, (m % DAY) / 60);
        if h == 0 {
            count(d, "day")
        } else {
            format!("{} {}", count(d, "day"), count(h, "hour"))
        }
    }
}

fn count(n: u32, unit: &str) -> String {
    if n == 1 {
        format!("1 {unit}")
    } else {
        format!("{n} {unit}s")
    }
}

// ---------------------------------------------------------------------------
// Reading and checking
// ---------------------------------------------------------------------------

impl Category {
    /// Used when shop.toml doesn't define any `[[category]]`.
    pub fn defaults() -> Vec<Category> {
        [
            ("arms", "Arms", Glyph::Sword),
            ("armor", "Armor", Glyph::Armor),
            ("accessories", "Accessories", Glyph::Ring),
            ("tools", "Tools", Glyph::Tool),
            ("consumables", "Medicine & meals", Glyph::Potion),
            ("housing", "Housing", Glyph::House),
            ("materials", "Materials", Glyph::Gem),
            ("other", "Other", Glyph::Crate),
        ]
        .into_iter()
        .map(|(key, label, glyph)| Category {
            key: key.into(),
            label: label.into(),
            glyph,
        })
        .collect()
    }
}

impl Shop {
    /// Reads shop.toml. The error explains every problem found, one per line.
    pub fn from_toml(text: &str) -> Result<Shop, String> {
        let mut shop: Shop = toml::from_str(text).map_err(|e| e.to_string())?;
        if shop.categories.is_empty() {
            shop.categories = Category::defaults();
        }
        let problems = shop.problems();
        if problems.is_empty() {
            Ok(shop)
        } else {
            Err(problems.join("\n"))
        }
    }

    fn problems(&self) -> Vec<String> {
        let mut out = Vec::new();

        if self.site.copyright.trim().is_empty() {
            out.push("[site] copyright needs the name of whoever owns the site design".into());
        }
        if !(2000..=2200).contains(&self.site.since) {
            out.push(format!(
                "[site] since should be a year, like 2026 (got {})",
                self.site.since
            ));
        }
        if let Some(address) = &self.site.address
            && !is_web_url(address)
        {
            out.push(format!(
                "[site] address \"{address}\" should start with https://"
            ));
        }

        for job in self.shop.crafters.keys() {
            if !CRAFTERS.iter().any(|(abbr, _)| abbr == job) {
                out.push(format!(
                    "[shop.crafters] \"{job}\" isn't a crafting or gathering job; use {}",
                    CRAFTERS.map(|(abbr, _)| abbr).join(", ")
                ));
            }
        }

        if !(1..=60).contains(&self.location.plot) {
            out.push(format!(
                "[location] plot is {}, but plots run from 1 to 60 (31-60 are the subdivision)",
                self.location.plot
            ));
        }
        if self.location.ward == 0 {
            out.push("[location] wards are numbered from 1".into());
        }

        if let Some(prices) = &self.prices
            && !(0.0..100.0).contains(&prices.undercut)
        {
            out.push(format!(
                "[prices] undercut should be a percentage from 0 to under 100 (it's {})",
                prices.undercut
            ));
        }

        if let Some(url) = &self.contact.discord_server
            && !is_web_url(url)
        {
            out.push(format!(
                "[contact] discord_server \"{url}\" should be an invite link starting with https://"
            ));
        }
        for link in &self.contact.links {
            if !is_web_url(&link.url) {
                out.push(format!(
                    "[[contact.links]] \"{}\" needs an address starting with https://",
                    link.label
                ));
            }
        }

        let mut keys: Vec<&str> = Vec::new();
        for category in &self.categories {
            let key = category.key.as_str();
            let tidy = !key.is_empty()
                && key
                    .chars()
                    .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-');
            if !tidy {
                out.push(format!(
                    "[[category]] key \"{key}\" should use only lowercase letters, digits and hyphens; it appears in links like #housing"
                ));
            }
            if key == "all" {
                out.push("[[category]] key \"all\" is taken by the All tab; pick another".into());
            }
            if keys.contains(&key) {
                out.push(format!("[[category]] key \"{key}\" is used twice"));
            }
            keys.push(key);
        }

        for (n, item) in self.items.iter().enumerate() {
            let at = format!("[[item]] #{} \"{}\"", n + 1, item.name);
            if item.name.trim().is_empty() {
                out.push(format!("[[item]] #{} needs a name", n + 1));
            }
            if !keys.contains(&item.category.as_str()) {
                out.push(format!(
                    "{at}: category \"{}\" isn't one of the [[category]] keys ({})",
                    item.category,
                    keys.join(", ")
                ));
            }
            if item.ready.is_none() && self.shop.default_ready.is_none() {
                out.push(format!(
                    "{at}: say when it will be ready, e.g. ready = \"2d\" (or set default_ready under [shop])"
                ));
            }
        }

        out
    }

    /// Things worth knowing that don't stop the site from working.
    /// (build.rs prints these.)
    #[allow(dead_code)]
    pub fn warnings(&self) -> Vec<String> {
        let mut out = Vec::new();
        if self.shop.demo {
            out.push(
                "demo = true, so the page says it's a sample shop. Set demo = false under [shop] once your own details are in."
                    .to_string(),
            );
        }
        let unlinked: Vec<&str> = self
            .items
            .iter()
            .filter(|item| item.lodestone.is_none())
            .map(|item| item.name.as_str())
            .collect();
        if !unlinked.is_empty() {
            out.push(format!(
                "{} item(s) have no Eorzea Database ID, so they link to a database search instead: {}. Run `cargo run -p shop-tools -- fill` to look the IDs up.",
                unlinked.len(),
                unlinked.join(", ")
            ));
        }
        out
    }

    pub fn category(&self, key: &str) -> Option<&Category> {
        self.categories.iter().find(|c| c.key == key)
    }

    pub fn ready_of(&self, item: &Item) -> Ready {
        item.ready.or(self.shop.default_ready).unwrap_or(Ready::NOW)
    }

    /// The item's Eorzea Database page, or a database search for its name.
    pub fn edb_url(&self, item: &Item) -> String {
        let host = self.shop.lodestone.host();
        match &item.lodestone {
            Some(id) => format!(
                "https://{host}/lodestone/playguide/db/item/{}/",
                id.as_str()
            ),
            None => format!(
                "https://{host}/lodestone/playguide/db/item/?q={}",
                url_encode(&item.name)
            ),
        }
    }

    /// "© 2026 Jason Telford", or "© 2026–2028 Jason Telford" in later years.
    pub fn copyright_notice(&self, this_year: u16) -> String {
        let since = self.site.since;
        let years = if this_year > since {
            format!("{since}\u{2013}{this_year}")
        } else {
            since.to_string()
        };
        format!("\u{a9} {years} {}", self.site.copyright.trim())
    }

    /// The command a visitor types in game, e.g. `/tell Lyra Valewood@Lich`.
    pub fn tell_command(&self) -> String {
        format!("/tell {}@{}", self.shop.owner, self.location.world)
    }

    /// "Lich, The Lavender Beds, Ward 12, Plot 34 (Subdivision)"
    pub fn address(&self) -> String {
        format!(
            "{}, {}, {}",
            self.location.world,
            self.location.district.name(),
            self.location.ward_and_plot()
        )
    }
}

impl Location {
    pub fn subdivision(&self) -> bool {
        self.plot > 30
    }

    /// "Ward 12, Plot 34 (Subdivision)"
    pub fn ward_and_plot(&self) -> String {
        let sub = if self.subdivision() {
            " (Subdivision)"
        } else {
            ""
        };
        format!("Ward {}, Plot {}{sub}", self.ward, self.plot)
    }
}

/// XIVAPI serves the game's icons as PNGs.
pub fn icon_url(icon: u32) -> String {
    let folder = icon / 1000 * 1000;
    format!("https://v2.xivapi.com/api/asset?path=ui/icon/{folder:06}/{icon:06}_hr1.tex&format=png")
}

/// 1234567 → "1,234,567", the way the game shows gil.
pub fn gil(amount: u64) -> String {
    let digits = amount.to_string();
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i).is_multiple_of(3) {
            out.push(',');
        }
        out.push(c);
    }
    out
}

pub fn is_web_url(text: &str) -> bool {
    text.starts_with("https://") || text.starts_with("http://")
}

fn url_encode(text: &str) -> String {
    let mut out = String::new();
    for byte in text.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_sample_shop_reads_cleanly() {
        let shop =
            Shop::from_toml(include_str!("../shop.toml")).expect("shop.toml should be valid");
        assert!(!shop.items.is_empty());
        for item in &shop.items {
            assert!(shop.category(&item.category).is_some(), "{}", item.name);
        }
    }

    #[test]
    fn ready_times() {
        let mins = |s: &str| s.parse::<Ready>().unwrap().minutes;
        assert_eq!(mins("now"), 0);
        assert_eq!(mins("In stock"), 0);
        assert_eq!(mins("45m"), 45);
        assert_eq!(mins("6h"), 360);
        assert_eq!(mins("2d"), 2880);
        assert_eq!(mins("1d 12h"), 2160);
        assert_eq!(mins("1 day and 12 hours"), 2160);
        assert_eq!(mins("1.5d"), 2160);
        assert_eq!(mins("2 weeks"), 20160);
        assert!("soon".parse::<Ready>().is_err());
        assert!("2".parse::<Ready>().is_err());
        assert!("".parse::<Ready>().is_err());
    }

    #[test]
    fn ready_labels() {
        let label = |s: &str| s.parse::<Ready>().unwrap().label();
        assert_eq!(label("now"), "In stock");
        assert_eq!(label("30m"), "30 minutes");
        assert_eq!(label("1h"), "1 hour");
        assert_eq!(label("90m"), "1 hour 30 minutes");
        assert_eq!(label("12h"), "12 hours");
        assert_eq!(label("1d"), "1 day");
        assert_eq!(label("36h"), "1 day 12 hours");
        assert_eq!(label("7d"), "1 week");
        assert_eq!(label("3d"), "3 days");
    }

    #[test]
    fn gil_has_thousands_separators() {
        assert_eq!(gil(0), "0");
        assert_eq!(gil(999), "999");
        assert_eq!(gil(1000), "1,000");
        assert_eq!(gil(380000), "380,000");
        assert_eq!(gil(1234567), "1,234,567");
    }

    #[test]
    fn lodestone_ids_and_addresses() {
        let id = |s: &str| LodestoneId::try_from(s.to_string()).map(|l| l.0);
        assert_eq!(id("6246afd2ba5").unwrap(), "6246afd2ba5");
        assert_eq!(
            id("https://na.finalfantasyxiv.com/lodestone/playguide/db/item/6246AFD2BA5/").unwrap(),
            "6246afd2ba5"
        );
        assert!(id("6246afd2ba").is_err());
        assert!(id("not-an-id!!").is_err());
    }

    #[test]
    fn districts_are_forgiving() {
        let d = |s: &str| District::try_from(s.to_string());
        assert_eq!(d("The Lavender Beds").unwrap(), District::LavenderBeds);
        assert_eq!(d("lavender beds").unwrap(), District::LavenderBeds);
        assert_eq!(d("Goblet").unwrap(), District::Goblet);
        assert!(d("Gridania").is_err());
    }

    #[test]
    fn mistakes_are_reported() {
        let base = include_str!("../shop.toml");
        let broken = base.replacen("category = \"arms\"", "category = \"weapons\"", 1);
        let err = Shop::from_toml(&broken).unwrap_err();
        assert!(err.contains("\"weapons\""), "{err}");

        let typo = base.replacen("price = ", "prcie = ", 1);
        let err = Shop::from_toml(&typo).unwrap_err();
        assert!(err.contains("prcie"), "{err}");
    }

    #[test]
    fn price_settings_are_checked() {
        let base = include_str!("../shop.toml");
        let shop = Shop::from_toml(base).unwrap();
        assert!(shop.prices.is_some());
        assert!(shop.items.iter().any(|item| !item.auto_price));
        assert!(shop.items.iter().any(|item| item.auto_price));

        let too_much = base.replacen("undercut = 0.5 ", "undercut = 100 ", 1);
        let err = Shop::from_toml(&too_much).unwrap_err();
        assert!(err.contains("[prices] undercut"), "{err}");

        // The old 2% rule is gone: a leftover own_mats stops the build.
        let old = base.replacen("[prices]\n", "[prices]\nown_mats = 2\n", 1);
        let err = Shop::from_toml(&old).unwrap_err();
        assert!(err.contains("own_mats"), "{err}");

        let floors = base.replacen(
            "auto_price = false\n",
            "auto_price = false\nmin_price = 500_000\nmin_price_own_mats = 50_000\n",
            1,
        );
        assert!(Shop::from_toml(&floors).is_ok());
        let not_in_shop_toml = base.replacen("auto_price = false\n", "own_mats_ask = true\n", 1);
        assert!(Shop::from_toml(&not_in_shop_toml).is_err());

        let typo = base.replacen("undercut = ", "undercutt = ", 1);
        let err = Shop::from_toml(&typo).unwrap_err();
        assert!(err.contains("undercutt"), "{err}");
    }

    #[test]
    fn copyright_notice_years() {
        let shop = Shop::from_toml(include_str!("../shop.toml")).unwrap();
        let since = shop.site.since;
        assert_eq!(
            shop.copyright_notice(since),
            format!("\u{a9} {since} Jason Telford")
        );
        assert_eq!(
            shop.copyright_notice(since + 2),
            format!("\u{a9} {since}\u{2013}{} Jason Telford", since + 2)
        );
        let missing = include_str!("../shop.toml").replacen(
            "copyright = \"Jason Telford\"",
            "copyright = \" \"",
            1,
        );
        assert!(Shop::from_toml(&missing).unwrap_err().contains("copyright"));
    }

    #[test]
    fn links_and_icons() {
        let shop = Shop::from_toml(include_str!("../shop.toml")).unwrap();
        let mut item = shop.items[0].clone();
        item.lodestone = None;
        item.name = "Courtly Lover's Rapier".into();
        assert!(
            shop.edb_url(&item)
                .ends_with("/db/item/?q=Courtly%20Lover%27s%20Rapier")
        );
        assert_eq!(
            icon_url(31234),
            "https://v2.xivapi.com/api/asset?path=ui/icon/031000/031234_hr1.tex&format=png"
        );
    }
}
