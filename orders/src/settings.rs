//! The service's settings. Where things are comes from the command line;
//! everything else from the environment (on the server, compose.yaml passes
//! it in from the server's .env), or else from a .env file in the folder it
//! runs in, as when you try it out on your computer.

use crate::vars;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;

pub const USAGE: &str = "\
Emails the orders sent from the order panel, through Postmark.

Usage: shop-orders [--listen ADDRESS] [--prices FILE] [--save-to FOLDER]
                   [--assets ADDRESS] [SHOP_TOML]

  --listen ADDRESS   where to take orders (default 127.0.0.1:8081)
  --prices FILE      the prices service's prices.json, if there is one
  --save-to FOLDER   save each email in FOLDER instead of sending it
  --assets ADDRESS   where the email's pictures and fonts come from
                     (default: [site] address in shop.toml)
  SHOP_TOML          the shop (default shop.toml)

Settings, from the environment or .env (see .env.example):
  POSTMARK_SERVER_TOKEN   Postmark server token (a secret)
  ORDER_EMAIL_FROM        the address orders come from, e.g. shop@your.domain
  ORDER_EMAIL_TO          the address orders go to
  ORDER_EMAIL_REPLY_TO    where replies go (default: ORDER_EMAIL_FROM)
  POSTMARK_SMTP_PORT      587 (the default) or 2525
  ORDERS_PER_HOUR         orders an hour from one visitor, or to one address (3)
  ORDERS_PER_DAY          orders a day for the whole shop (20)
";

pub struct Settings {
    pub listen: SocketAddr,
    pub shop_toml: PathBuf,
    pub prices: Option<PathBuf>,
    pub save_to: Option<PathBuf>,
    pub assets: Option<String>,
    pub token: Option<String>,
    pub from: Option<String>,
    pub to: Option<String>,
    pub reply_to: Option<String>,
    pub smtp_host: String,
    pub smtp_port: u16,
    pub stream: String,
    pub per_hour: u32,
    pub per_day: u32,
}

/// Looks a setting up in the environment, then in the .env file's lines.
pub struct Lookup {
    /// Whether to look in the environment (the tests don't).
    environment: bool,
    dotenv: HashMap<String, String>,
}

impl Lookup {
    pub fn new() -> Lookup {
        let dotenv = std::fs::read_to_string(".env")
            .map(|text| vars::read_env_file(&text).into_iter().collect())
            .unwrap_or_default();
        Lookup {
            environment: true,
            dotenv,
        }
    }

    pub fn get(&self, name: &str) -> Option<String> {
        self.environment
            .then(|| std::env::var(name).ok())
            .flatten()
            .or_else(|| self.dotenv.get(name).cloned())
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
    }
}

impl Settings {
    /// Reads the command line (without the program's name) and the settings.
    pub fn read(args: &[String], lookup: &Lookup) -> Result<Settings, String> {
        let mut listen = "127.0.0.1:8081".to_string();
        let mut shop_toml = None;
        let mut prices = None;
        let mut save_to = None;
        let mut assets = None;
        let mut args = args.iter();
        while let Some(arg) = args.next() {
            let mut value = |name: &str| {
                args.next()
                    .cloned()
                    .ok_or_else(|| format!("{name} needs a value after it"))
            };
            match arg.as_str() {
                "--listen" => listen = value("--listen")?,
                "--prices" => prices = Some(PathBuf::from(value("--prices")?)),
                "--save-to" => save_to = Some(PathBuf::from(value("--save-to")?)),
                "--assets" => assets = Some(value("--assets")?),
                "-h" | "--help" => return Err(String::new()),
                other if other.starts_with('-') => {
                    return Err(format!("unknown option {other}"));
                }
                other => {
                    if shop_toml.replace(PathBuf::from(other)).is_some() {
                        return Err("only one shop.toml, please".into());
                    }
                }
            }
        }
        let listen: SocketAddr = listen.parse().map_err(|_| {
            format!("--listen {listen} should be an address and port, like 0.0.0.0:8080")
        })?;
        let number = |name: &str, default: u32| -> Result<u32, String> {
            match lookup.get(name) {
                None => Ok(default),
                Some(value) => value.parse::<u32>().ok().filter(|n| *n > 0).ok_or_else(|| {
                    format!("{name} should be a whole number above 0 (it's {value})")
                }),
            }
        };
        let smtp_port = number("POSTMARK_SMTP_PORT", 587)?;
        let smtp_port = u16::try_from(smtp_port)
            .map_err(|_| format!("POSTMARK_SMTP_PORT {smtp_port} isn't a port"))?;
        Ok(Settings {
            listen,
            shop_toml: shop_toml.unwrap_or_else(|| PathBuf::from("shop.toml")),
            prices,
            save_to,
            assets,
            token: lookup.get("POSTMARK_SERVER_TOKEN"),
            from: lookup.get("ORDER_EMAIL_FROM"),
            to: lookup.get("ORDER_EMAIL_TO"),
            reply_to: lookup.get("ORDER_EMAIL_REPLY_TO"),
            smtp_host: lookup
                .get("POSTMARK_SMTP_HOST")
                .unwrap_or_else(|| "smtp.postmarkapp.com".into()),
            smtp_port,
            stream: lookup
                .get("POSTMARK_MESSAGE_STREAM")
                .unwrap_or_else(|| "outbound".into()),
            per_hour: number("ORDERS_PER_HOUR", 3)?,
            per_day: number("ORDERS_PER_DAY", 20)?,
        })
    }

    /// The settings that sending needs and doesn't have. Saving to files
    /// doesn't need the token.
    pub fn missing(&self) -> Vec<&'static str> {
        let mut missing = Vec::new();
        if self.save_to.is_none() && self.token.is_none() {
            missing.push("POSTMARK_SERVER_TOKEN");
        }
        if self.from.is_none() {
            missing.push("ORDER_EMAIL_FROM");
        }
        if self.to.is_none() {
            missing.push("ORDER_EMAIL_TO");
        }
        missing
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn lookup(pairs: &[(&str, &str)]) -> Lookup {
        Lookup {
            environment: false,
            dotenv: pairs
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect(),
        }
    }

    fn args(text: &str) -> Vec<String> {
        text.split_whitespace().map(String::from).collect()
    }

    #[test]
    fn defaults() {
        let settings = Settings::read(&[], &lookup(&[])).unwrap();
        assert_eq!(settings.listen.to_string(), "127.0.0.1:8081");
        assert_eq!(settings.shop_toml, PathBuf::from("shop.toml"));
        assert_eq!(settings.smtp_host, "smtp.postmarkapp.com");
        assert_eq!(settings.smtp_port, 587);
        assert_eq!(settings.stream, "outbound");
        assert_eq!((settings.per_hour, settings.per_day), (3, 20));
        assert_eq!(
            settings.missing(),
            [
                "POSTMARK_SERVER_TOKEN",
                "ORDER_EMAIL_FROM",
                "ORDER_EMAIL_TO"
            ]
        );
    }

    #[test]
    fn the_server_and_its_env() {
        let settings = Settings::read(
            &args("--listen 0.0.0.0:8080 --prices /srv/prices/prices.json /app/shop.toml"),
            &lookup(&[
                ("POSTMARK_SERVER_TOKEN", " secret "),
                ("ORDER_EMAIL_FROM", "shop@example.com"),
                ("ORDER_EMAIL_TO", "shop@example.com"),
                ("POSTMARK_SMTP_PORT", "2525"),
                ("ORDERS_PER_DAY", "50"),
                ("ORDER_EMAIL_REPLY_TO", ""),
            ]),
        )
        .unwrap();
        assert_eq!(settings.listen.to_string(), "0.0.0.0:8080");
        assert_eq!(
            settings.prices,
            Some(PathBuf::from("/srv/prices/prices.json"))
        );
        assert_eq!(settings.shop_toml, PathBuf::from("/app/shop.toml"));
        assert_eq!(settings.token.as_deref(), Some("secret"));
        assert_eq!(settings.smtp_port, 2525);
        assert_eq!(settings.per_day, 50);
        assert_eq!(settings.reply_to, None);
        assert!(settings.missing().is_empty());
    }

    #[test]
    fn saving_to_files_needs_no_token() {
        let settings = Settings::read(
            &args("--save-to target/emails --assets http://localhost:8080"),
            &lookup(&[
                ("ORDER_EMAIL_FROM", "shop@example.com"),
                ("ORDER_EMAIL_TO", "shop@example.com"),
            ]),
        )
        .unwrap();
        assert!(settings.missing().is_empty());
        assert_eq!(settings.assets.as_deref(), Some("http://localhost:8080"));
    }

    #[test]
    fn mistakes_are_explained() {
        assert!(Settings::read(&args("--listen nowhere"), &lookup(&[])).is_err());
        assert!(Settings::read(&args("--prices"), &lookup(&[])).is_err());
        assert!(Settings::read(&args("--colour gold"), &lookup(&[])).is_err());
        assert!(Settings::read(&args("a.toml b.toml"), &lookup(&[])).is_err());
        assert!(Settings::read(&[], &lookup(&[("ORDERS_PER_HOUR", "lots")])).is_err());
        assert!(Settings::read(&[], &lookup(&[("ORDERS_PER_DAY", "0")])).is_err());
        assert!(Settings::read(&[], &lookup(&[("POSTMARK_SMTP_PORT", "70000")])).is_err());
    }
}
