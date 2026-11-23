//! Market prices from /prices.json, which the prices service on the server
//! writes (shop-tools' `prices --json`, run weekly: see compose.yaml). The site
//! reads the file when it opens, before it shows anything, and uses its prices
//! in place of shop.toml's (src/main.rs loads it). If the file is missing,
//! late or unreadable, the site keeps shop.toml's prices.
//!
//! The orders service (orders/) compiles this file too, and reads the same
//! file, so the receipt it emails has the prices the order panel showed.
//! Keep it free of browser-only code.

use crate::model::Shop;
use serde::Deserialize;

/// Where the server serves the file (see deploy/nginx.conf).
pub const ADDRESS: &str = "/prices.json";

/// How long each step of loading the file may take before the site gives up
/// on it and shows shop.toml's prices.
pub const WAIT_MS: i32 = 3000;

#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Prices {
    pub items: Vec<Price>,
}

/// One item's market prices, matched to shop.toml's items by `id` and `hq`.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Price {
    pub id: u32,
    #[serde(default)]
    pub hq: bool,
    pub price: u64,
    /// Left out when the item has no recipe, so shop.toml's stays.
    #[serde(default)]
    pub price_own_mats: Option<OwnMats>,
}

/// The price when the buyer brings the materials: gil, or "ask".
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
#[serde(untagged)]
pub enum OwnMats {
    Gil(u64),
    Ask(AskWord),
}

#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub enum AskWord {
    #[serde(rename = "ask")]
    Ask,
}

pub fn parse(text: &str) -> Result<Prices, String> {
    serde_json::from_str(text).map_err(|e| e.to_string())
}

/// Reads the file. Ok(None) when the text isn't the prices file at all, such
/// as the page `trunk serve` sends for a file it doesn't have.
pub fn read(text: &str) -> Result<Option<Prices>, String> {
    if !text.trim_start().starts_with('{') {
        return Ok(None);
    }
    parse(text).map(Some)
}

/// Puts the market prices in place of shop.toml's, for the items shop-tools
/// prices (not those with auto_price = false).
pub fn apply(shop: &mut Shop, prices: &Prices) {
    for item in shop.items.iter_mut().filter(|item| item.auto_price) {
        let Some(id) = item.id else { continue };
        let Some(market) = prices
            .items
            .iter()
            .find(|price| price.id == id && price.hq == item.hq)
        else {
            continue;
        };
        item.price = Some(market.price);
        match market.price_own_mats {
            Some(OwnMats::Gil(gil)) => {
                item.price_own_mats = Some(gil);
                item.own_mats_ask = false;
            }
            Some(OwnMats::Ask(_)) => {
                item.price_own_mats = None;
                item.own_mats_ask = true;
            }
            None => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// What shop-tools writes for its test shop (tools/src/prices.rs,
    /// writes_the_prices_file), so the two stay in step.
    const FROM_SHOP_TOOLS: &str = r#"{
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

    #[test]
    fn reads_what_shop_tools_writes() {
        let prices = parse(FROM_SHOP_TOOLS).unwrap();
        assert_eq!(prices.items.len(), 3);
        assert_eq!(prices.items[0].price_own_mats, Some(OwnMats::Gil(13_168)));
        assert_eq!(
            prices.items[1].price_own_mats,
            Some(OwnMats::Ask(AskWord::Ask))
        );
        let no_recipe = parse(r#"{"items":[{"id":1,"hq":false,"price":5}]}"#).unwrap();
        assert_eq!(no_recipe.items[0].price_own_mats, None);
        // What `trunk serve` answers for a file it doesn't have.
        assert!(parse("<!DOCTYPE html><html></html>").is_err());
        assert_eq!(read("<!DOCTYPE html><html></html>"), Ok(None));
        assert_eq!(read(FROM_SHOP_TOOLS).unwrap().unwrap().items.len(), 3);
        assert!(read(r#"{"items":"none"}"#).is_err());
        assert!(parse(r#"{"items":[{"id":1,"price":"lots"}]}"#).is_err());
    }

    #[test]
    fn market_prices_replace_shop_toml_ones() {
        let mut shop = Shop::from_toml(include_str!("../shop.toml")).unwrap();
        let find = |shop: &Shop, name: &str| {
            shop.items
                .iter()
                .find(|item| item.name == name)
                .unwrap()
                .clone()
        };
        let sofa_before = find(&shop, "Everkeep Sofa");
        let prices = Prices {
            items: vec![
                Price {
                    id: 49263,
                    hq: true,
                    price: 89_403,
                    price_own_mats: Some(OwnMats::Gil(13_168)),
                },
                Price {
                    id: 47173,
                    hq: true,
                    price: 380_365,
                    price_own_mats: Some(OwnMats::Ask(AskWord::Ask)),
                },
                // NQ prices don't fit the HQ gemdraught.
                Price {
                    id: 49234,
                    hq: false,
                    price: 1,
                    price_own_mats: None,
                },
                // The stage keeps "Ask": it has auto_price = false.
                Price {
                    id: 51271,
                    hq: false,
                    price: 663_289,
                    price_own_mats: Some(OwnMats::Gil(1)),
                },
                Price {
                    id: 46096,
                    hq: false,
                    price: 995,
                    price_own_mats: None,
                },
            ],
        };
        apply(&mut shop, &prices);

        let rapier = find(&shop, "Courtly Lover's Rapier");
        assert_eq!(
            (rapier.price, rapier.price_own_mats),
            (Some(89_403), Some(13_168))
        );
        assert!(!rapier.own_mats_ask);
        let rod = find(&shop, "Gold Thumb's Fishing Rod");
        assert_eq!((rod.price, rod.price_own_mats), (Some(380_365), None));
        assert!(rod.own_mats_ask);
        let gemdraught = find(&shop, "Grade 4 Gemdraught of Strength");
        assert_eq!(gemdraught.price, Some(4_500));
        let stage = find(&shop, "Semicircular Outdoor Stage");
        assert_eq!((stage.price, stage.own_mats_ask), (None, false));
        // No recipe in the file: the price changes, shop.toml's own-materials
        // price (none here) stays.
        let table = find(&shop, "Tuliyollan Table");
        assert_eq!((table.price, table.price_own_mats), (Some(995), None));
        // Not in the file at all: as shop.toml has it.
        let sofa = find(&shop, "Everkeep Sofa");
        assert_eq!(sofa.price, sofa_before.price);
    }
}
