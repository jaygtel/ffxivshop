//! The orders service. The order panel's Send by email sends it each order
//! (POST /api/order). It checks the order, prices it from shop.toml and this
//! week's market prices, and emails a receipt through Postmark to the shop,
//! with Reply-To set to the buyer, so replying answers them. The buyer gets
//! no copy, as the address they gave hasn't been confirmed.
//!
//! The contact page sends it each message too (POST /api/contact). It checks
//! the message and emails it through Postmark to the shop, with Reply-To set
//! to the visitor, so replying answers them.
//!
//! On the server it runs in its own container next to the site, and the
//! site's nginx passes orders and messages on to it (see compose.yaml and
//! deploy/nginx.conf). Run `shop-orders --help` for its options and
//! settings.

// The shop's own code, shared with the site, so the receipt prices and
// checks an order exactly as the order panel does, and a message is checked
// exactly as the contact page checks it.
#[allow(dead_code)]
#[path = "../../src/basket.rs"]
mod basket;
#[allow(dead_code)]
#[path = "../../src/message.rs"]
mod message;
#[allow(dead_code)]
#[path = "../../src/model.rs"]
mod model;
#[allow(dead_code)]
#[path = "../../src/prices.rs"]
mod prices;
#[allow(dead_code)]
#[path = "../../src/vars.rs"]
mod vars;

mod clock;
mod letter;
mod limits;
mod look;
mod mail;
mod order;
mod receipt;
mod settings;

use axum::Json;
use axum::Router;
use axum::extract::rejection::JsonRejection;
use axum::extract::{ConnectInfo, DefaultBodyLimit, State};
use axum::http::{HeaderMap, StatusCode};
use axum::routing::post;
use basket::{Answer, EmailOrder};
use lettre::message::Mailbox;
use limits::{Limits, Reached};
use mail::{Addresses, Mailer};
use message::{ContactMessage, Reply};
use model::Shop;
use settings::{Lookup, Settings};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;

struct Service {
    /// shop.toml, read when the service starts.
    shop: Shop,
    /// The prices service's prices.json, read for each order, as it changes
    /// every week.
    prices: Option<PathBuf>,
    /// None when the settings for sending orders are missing.
    mailer: Option<Mailer>,
    limits: Limits,
    /// The same for the contact page's messages, which have limits of their
    /// own.
    messenger: Option<Mailer>,
    message_limits: Limits,
    /// Where the email's pictures and fonts come from.
    assets: String,
}

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let settings = match Settings::read(&args, &Lookup::new()) {
        Ok(settings) => settings,
        Err(problem) => {
            if problem.is_empty() {
                print!("{}", settings::USAGE);
                return;
            }
            eprintln!("shop-orders: {problem}\n\n{}", settings::USAGE);
            std::process::exit(2);
        }
    };
    let shop = match load_shop(&settings.shop_toml) {
        Ok(shop) => shop,
        Err(problem) => {
            eprintln!("shop-orders: {problem}");
            std::process::exit(1);
        }
    };

    let missing = settings.missing();
    let mailer = if !missing.is_empty() {
        println!(
            "Orders can't be emailed until these are set in .env: {}. Until then the order panel says so.",
            missing.join(", ")
        );
        None
    } else {
        Some(make_mailer(
            &settings,
            &Addresses {
                shop_name: &shop.shop.name,
                from: settings.from.as_deref().unwrap_or_default(),
                to: settings.to.as_deref().unwrap_or_default(),
                settings: "ORDER_EMAIL",
            },
        ))
    };
    let missing = settings.missing_for_messages();
    let messenger = if !missing.is_empty() {
        println!(
            "Messages from the contact page can't be emailed until these are set in .env: {}. Until then the contact page says so.",
            missing.join(", ")
        );
        None
    } else {
        Some(make_mailer(
            &settings,
            &Addresses {
                shop_name: &shop.shop.name,
                from: settings.contact_from.as_deref().unwrap_or_default(),
                to: settings.contact_to.as_deref().unwrap_or_default(),
                settings: "CONTACT_EMAIL",
            },
        ))
    };

    let assets = settings
        .assets
        .clone()
        .or_else(|| shop.site.address.clone())
        .unwrap_or_default();
    let service = Arc::new(Service {
        shop,
        prices: settings.prices.clone(),
        mailer,
        limits: Limits::new(settings.per_hour, settings.per_day),
        messenger,
        message_limits: Limits::new(settings.messages_per_hour, settings.messages_per_day),
        assets,
    });
    let app = router(service.clone());

    let listener = match tokio::net::TcpListener::bind(settings.listen).await {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("shop-orders: can't listen on {}: {e}", settings.listen);
            std::process::exit(1);
        }
    };
    println!(
        "Taking orders on {}{}, {}; at most {} an hour from one visitor or to one address, and {} a day.",
        settings.listen,
        basket::SEND_ADDRESS,
        service
            .mailer
            .as_ref()
            .map_or("but not sending them".to_string(), |mailer| mailer
                .describe("orders")),
        settings.per_hour,
        settings.per_day,
    );
    println!(
        "Taking messages on {}{}, {}; at most {} an hour from one visitor or from one address, and {} a day.",
        settings.listen,
        message::SEND_ADDRESS,
        service
            .messenger
            .as_ref()
            .map_or("but not sending them".to_string(), |messenger| messenger
                .describe("messages")),
        settings.messages_per_hour,
        settings.messages_per_day,
    );
    let served = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(stopped())
    .await;
    if let Err(e) = served {
        eprintln!("shop-orders: {e}");
        std::process::exit(1);
    }
}

/// The two addresses the service answers: the order panel's, and the
/// contact page's.
fn router(service: Arc<Service>) -> Router {
    Router::new()
        .route(basket::SEND_ADDRESS, post(take_order))
        .route(message::SEND_ADDRESS, post(take_message))
        .layer(DefaultBodyLimit::max(16 * 1024))
        .with_state(service)
}

/// A mailer for `addresses`: one that sends through Postmark, or one that
/// saves the emails in a folder (--save-to). A mistake in the settings stops
/// the service.
fn make_mailer(settings: &Settings, addresses: &Addresses) -> Mailer {
    let made = match &settings.save_to {
        Some(folder) => Mailer::files(addresses, folder.clone(), &settings.stream),
        None => Mailer::postmark(
            addresses,
            settings.token.as_deref().unwrap_or_default(),
            &settings.smtp_host,
            settings.smtp_port,
            &settings.stream,
        ),
    };
    made.unwrap_or_else(|problem| {
        eprintln!("shop-orders: {problem}");
        std::process::exit(1);
    })
}

/// Reads shop.toml, filling in its ${SHOP_…} values as build.rs does. The
/// service only needs the shop's items, prices and address, so values that
/// aren't set just use their examples.
fn load_shop(path: &PathBuf) -> Result<Shop, String> {
    let text =
        std::fs::read_to_string(path).map_err(|e| format!("can't read {}: {e}", path.display()))?;
    let lookup = Lookup::new();
    let filled = vars::fill(&text, |name| lookup.get(name))
        .map_err(|problem| format!("{} needs fixing:\n{problem}", path.display()))?;
    Shop::from_toml(&filled.text)
        .map_err(|problem| format!("{} needs fixing:\n{problem}", path.display()))
}

/// Waits for Docker (or Ctrl-C) to stop the service.
async fn stopped() {
    let interrupt = async {
        let _ = tokio::signal::ctrl_c().await;
    };
    #[cfg(unix)]
    let terminate = async {
        match tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()) {
            Ok(mut signal) => {
                signal.recv().await;
            }
            Err(_) => std::future::pending::<()>().await,
        }
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();
    tokio::select! {
        () = interrupt => {},
        () = terminate => {},
    }
}

impl Service {
    /// The shop with this week's market prices in place, as the order panel
    /// shows it.
    fn shop_now(&self) -> Shop {
        let mut shop = self.shop.clone();
        let Some(path) = &self.prices else {
            return shop;
        };
        match std::fs::read_to_string(path) {
            Ok(text) => match prices::read(&text) {
                Ok(Some(prices)) => prices::apply(&mut shop, &prices),
                Ok(None) => println!(
                    "{} isn't a prices file; using shop.toml's prices",
                    path.display()
                ),
                Err(problem) => println!(
                    "{} isn't readable ({problem}); using shop.toml's prices",
                    path.display()
                ),
            },
            // Not written yet: the site uses shop.toml's prices too.
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => println!(
                "can't read {}: {e}; using shop.toml's prices",
                path.display()
            ),
        }
        shop
    }

    /// The end of the words for an order that wasn't sent.
    fn otherwise(&self) -> &'static str {
        if self.shop.contact.discord.is_some() {
            " Or copy it for Discord."
        } else {
            ""
        }
    }
}

fn answer(status: StatusCode, problem: String) -> (StatusCode, Json<Answer>) {
    (
        status,
        Json(Answer {
            number: None,
            problem: Some(problem),
        }),
    )
}

/// Where an order came from: the visitor's address as Cloudflare saw it, or
/// as the proxies in front passed it on, or the connection's own.
fn visitor(headers: &HeaderMap, peer: SocketAddr) -> String {
    let header = |name: &str| {
        headers
            .get(name)
            .and_then(|value| value.to_str().ok())
            .map(|value| {
                value
                    .split(',')
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            })
            .filter(|value| !value.is_empty())
    };
    header("cf-connecting-ip")
        .or_else(|| header("x-real-ip"))
        .or_else(|| header("x-forwarded-for"))
        .unwrap_or_else(|| peer.ip().to_string())
}

/// An order number: the shop's initials, the year and month, and four
/// random letters and digits, such as AA-2611-7K3Q. Letters and digits that
/// look alike (0 and O, 1 and I) are left out.
fn order_number(shop_name: &str, now: &clock::UkTime) -> String {
    const CHARS: &[u8] = b"23456789ABCDEFGHJKLMNPQRSTUVWXYZ";
    let initials: String = shop_name
        .split_whitespace()
        .filter_map(|word| word.chars().next())
        .filter(char::is_ascii_alphabetic)
        .map(|c| c.to_ascii_uppercase())
        .take(3)
        .collect();
    let initials = if initials.is_empty() {
        "ORD".to_string()
    } else {
        initials
    };
    let mut random = [0u8; 4];
    if getrandom::fill(&mut random).is_err() {
        random = (clock::now() as u32).to_be_bytes();
    }
    let code: String = random
        .iter()
        .map(|b| CHARS[*b as usize % CHARS.len()] as char)
        .collect();
    format!("{initials}-{:02}{:02}-{code}", now.year % 100, now.month)
}

async fn take_order(
    State(service): State<Arc<Service>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    sent: Result<Json<EmailOrder>, JsonRejection>,
) -> (StatusCode, Json<Answer>) {
    let otherwise = service.otherwise();
    let Ok(Json(sent)) = sent else {
        return answer(
            StatusCode::BAD_REQUEST,
            format!("Your order could not be read. Reload the page and try again.{otherwise}"),
        );
    };
    let Some(mailer) = &service.mailer else {
        return answer(
            StatusCode::SERVICE_UNAVAILABLE,
            format!("Sending orders by email is not ready yet.{otherwise}"),
        );
    };
    if !sent.website.trim().is_empty() {
        println!("Turned an order away: the hidden box was filled in, so it's probably from a bot");
        return answer(
            StatusCode::BAD_REQUEST,
            format!("Your order could not be sent.{otherwise}"),
        );
    }

    let shop = service.shop_now();
    let order = match order::check(&shop, &sent) {
        Ok(order) => order,
        Err(problem) => return answer(StatusCode::BAD_REQUEST, problem),
    };
    let from = visitor(&headers, peer);
    let place = match service.limits.take(&from, &order.email, Instant::now()) {
        Ok(place) => place,
        Err(Reached::Hour) => {
            println!(
                "Turned an order away: too many in the last hour from one visitor or to one address"
            );
            return answer(
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "Too many orders have been sent from here, or to this email address, in the last hour. Please try again later.{otherwise}"
                ),
            );
        }
        Err(Reached::Day) => {
            println!("Turned an order away: the shop has taken today's most orders by email");
            return answer(
                StatusCode::TOO_MANY_REQUESTS,
                format!(
                    "The shop has had a lot of orders by email today. Please try again tomorrow.{otherwise}"
                ),
            );
        }
    };

    let now = clock::UkTime::at(clock::now());
    let number = order_number(&shop.shop.name, &now);
    let receipt = receipt::write(&shop, &order, &number, &now.written(), &service.assets);
    let buyer = match order.email.parse() {
        Ok(address) => Mailbox::new(Some(order.character.clone()), address),
        Err(_) => {
            service.limits.give_back(place);
            return answer(
                StatusCode::BAD_REQUEST,
                "Check your email address. It should look like name@example.com.".into(),
            );
        }
    };
    let sent = match mailer.message(&receipt, &number, buyer) {
        Ok(message) => {
            mailer
                .send(message, &number, &receipt.html, &receipt.text)
                .await
        }
        Err(problem) => Err(problem),
    };
    match sent {
        Ok(()) => {
            println!(
                "{} order {number} ({}, {})",
                mailer.done(),
                receipt::items_word(order.count()),
                order
                    .totals
                    .in_words()
                    .unwrap_or_else(|| "no prices yet".into())
            );
            (
                StatusCode::OK,
                Json(Answer {
                    number: Some(number),
                    problem: None,
                }),
            )
        }
        Err(problem) => {
            service.limits.give_back(place);
            println!("Could not send order {number}: {problem}");
            answer(
                StatusCode::BAD_GATEWAY,
                format!(
                    "Your order could not be sent just now. Please try again in a few minutes.{otherwise}"
                ),
            )
        }
    }
}

fn not_sent(status: StatusCode, problem: &str) -> (StatusCode, Json<Reply>) {
    (
        status,
        Json(Reply {
            sent: false,
            problem: Some(problem.to_string()),
        }),
    )
}

/// A name for a message, for its Message-ID and the service's log: the
/// UK date and time it was sent, and random letters and digits, such as
/// message-20261124-1152-3fa9c1d2.
fn message_reference(now: &clock::UkTime) -> String {
    format!(
        "message-{:04}{:02}{:02}-{:02}{:02}-{}",
        now.year,
        now.month,
        now.day,
        now.hour,
        now.minute,
        mail::random_hex(4)
    )
}

async fn take_message(
    State(service): State<Arc<Service>>,
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    sent: Result<Json<ContactMessage>, JsonRejection>,
) -> (StatusCode, Json<Reply>) {
    let Ok(Json(sent)) = sent else {
        return not_sent(
            StatusCode::BAD_REQUEST,
            "Your message could not be read. Reload the page and try again.",
        );
    };
    let Some(messenger) = &service.messenger else {
        return not_sent(
            StatusCode::SERVICE_UNAVAILABLE,
            "Sending messages is not ready yet.",
        );
    };
    if !sent.website.trim().is_empty() {
        println!(
            "Turned a message away: the hidden box was filled in, so it's probably from a bot"
        );
        return not_sent(StatusCode::BAD_REQUEST, "Your message could not be sent.");
    }
    if let Some((_, problem)) = sent.problem() {
        return not_sent(StatusCode::BAD_REQUEST, problem);
    }
    let message = sent.tidied();

    let from = visitor(&headers, peer);
    let place = match service
        .message_limits
        .take(&from, &message.email, Instant::now())
    {
        Ok(place) => place,
        Err(Reached::Hour) => {
            println!(
                "Turned a message away: too many in the last hour from one visitor or from one address"
            );
            return not_sent(
                StatusCode::TOO_MANY_REQUESTS,
                "Too many messages have been sent from here, or from this email address, in the last hour. Please try again later.",
            );
        }
        Err(Reached::Day) => {
            println!("Turned a message away: the site has taken today's most messages");
            return not_sent(
                StatusCode::TOO_MANY_REQUESTS,
                "The shop has had a lot of messages today. Please try again tomorrow.",
            );
        }
    };

    let now = clock::UkTime::at(clock::now());
    let reference = message_reference(&now);
    let letter = letter::write(&service.shop, &message, &now.written(), &service.assets);
    let reply_to = match message.email.parse() {
        Ok(address) => Mailbox::new(Some(message.name.clone()), address),
        Err(_) => {
            service.message_limits.give_back(place);
            return not_sent(
                StatusCode::BAD_REQUEST,
                "Check your email address. It should look like name@example.com.",
            );
        }
    };
    let sent = match messenger.letter(&letter, &reference, reply_to) {
        Ok(email) => {
            messenger
                .send(email, &reference, &letter.html, &letter.text)
                .await
        }
        Err(problem) => Err(problem),
    };
    match sent {
        Ok(()) => {
            println!("{} {reference} from the contact page", messenger.done());
            (
                StatusCode::OK,
                Json(Reply {
                    sent: true,
                    problem: None,
                }),
            )
        }
        Err(problem) => {
            service.message_limits.give_back(place);
            println!("Could not send {reference}: {problem}");
            not_sent(
                StatusCode::BAD_GATEWAY,
                "Your message could not be sent just now. Please try again in a few minutes.",
            )
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn order_numbers_read_easily() {
        let now = clock::UkTime {
            year: 2026,
            month: 11,
            day: 23,
            hour: 12,
            minute: 41,
        };
        let number = order_number("Aether & Allure", &now);
        assert_eq!(&number[..8], "AA-2611-");
        assert_eq!(number.len(), 12);
        assert!(
            number[8..]
                .chars()
                .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
        );
        assert!(!number[8..].contains(['0', 'O', '1', 'I']));
        assert_eq!(&order_number("The Moonlit Forge", &now)[..4], "TMF-");
        assert_eq!(&order_number("& &", &now)[..4], "ORD-");
    }

    #[test]
    fn the_visitor_is_who_cloudflare_saw() {
        let peer: SocketAddr = "172.18.0.3:40000".parse().unwrap();
        let mut headers = HeaderMap::new();
        assert_eq!(visitor(&headers, peer), "172.18.0.3");
        headers.insert(
            "x-forwarded-for",
            "198.51.100.4, 172.18.0.2".parse().unwrap(),
        );
        assert_eq!(visitor(&headers, peer), "198.51.100.4");
        headers.insert("cf-connecting-ip", "203.0.113.7".parse().unwrap());
        assert_eq!(visitor(&headers, peer), "203.0.113.7");
    }

    use axum::body::{Body, to_bytes};
    use axum::http::Request;
    use tower::ServiceExt;

    fn service(mailer: Option<Mailer>, per_hour: u32) -> Arc<Service> {
        service_with(mailer, None, per_hour)
    }

    /// A service for the contact page's messages only.
    fn messaging(messenger: Option<Mailer>, per_hour: u32) -> Arc<Service> {
        service_with(None, messenger, per_hour)
    }

    fn service_with(
        mailer: Option<Mailer>,
        messenger: Option<Mailer>,
        per_hour: u32,
    ) -> Arc<Service> {
        Arc::new(Service {
            shop: load_shop(&PathBuf::from(concat!(
                env!("CARGO_MANIFEST_DIR"),
                "/../shop.toml"
            )))
            .unwrap(),
            prices: None,
            mailer,
            limits: Limits::new(per_hour, 20),
            messenger,
            message_limits: Limits::new(per_hour, 20),
            assets: "https://aetherandallure.com".into(),
        })
    }

    fn saving(folder: &std::path::Path) -> Mailer {
        let addresses = Addresses {
            shop_name: "Aether & Allure",
            from: "shop@example.com",
            to: "shop@example.com",
            settings: "ORDER_EMAIL",
        };
        Mailer::files(&addresses, folder.to_path_buf(), "outbound").unwrap()
    }

    fn saving_messages(folder: &std::path::Path) -> Mailer {
        let addresses = Addresses {
            shop_name: "Aether & Allure",
            from: "support@example.com",
            to: "support@example.com",
            settings: "CONTACT_EMAIL",
        };
        Mailer::files(&addresses, folder.to_path_buf(), "outbound").unwrap()
    }

    const ORDER: &str = r#"{"items":[{"name":"Courtly Lover's Rapier","quantity":2}],
        "own_mats":true,"note":"","character":"Mira Ashwood","world":"Cerberus",
        "email":"mira@example.com","website":""}"#;

    /// Sends `body` to the service as the order panel would, from `from`.
    async fn send(service: &Arc<Service>, body: &str, from: &str) -> (StatusCode, Answer) {
        let mut request = Request::post(basket::SEND_ADDRESS)
            .header("content-type", "application/json")
            .header("cf-connecting-ip", from)
            .body(Body::from(body.to_string()))
            .unwrap();
        let peer: SocketAddr = "172.18.0.3:40000".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = router(service.clone()).oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    /// Sends `body` to the service as the contact page would, from `from`.
    async fn send_message(service: &Arc<Service>, body: &str, from: &str) -> (StatusCode, Reply) {
        let mut request = Request::post(message::SEND_ADDRESS)
            .header("content-type", "application/json")
            .header("cf-connecting-ip", from)
            .body(Body::from(body.to_string()))
            .unwrap();
        let peer: SocketAddr = "172.18.0.3:40000".parse().unwrap();
        request.extensions_mut().insert(ConnectInfo(peer));
        let response = router(service.clone()).oneshot(request).await.unwrap();
        let status = response.status();
        let bytes = to_bytes(response.into_body(), 64 * 1024).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }

    fn folder() -> PathBuf {
        std::env::temp_dir().join(format!("shop-orders-service-{}", mail::random_hex(4)))
    }

    const MESSAGE: &str = r#"{"name":" Mira Ashwood ","email":"mira@example.com",
        "subject":"Question about my order","message":"Hello!\n\nCan you craft a rapier?",
        "website":""}"#;

    /// The emails saved in `folder`, by name.
    fn saved(folder: &std::path::Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(folder)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[tokio::test]
    async fn an_order_is_emailed_and_numbered() {
        let folder = folder();
        let service = service(Some(saving(&folder)), 3);
        let (status, answer) = send(&service, ORDER, "203.0.113.7").await;
        assert_eq!(status, StatusCode::OK);
        let number = answer.number.unwrap();
        assert!(number.starts_with("AA-"));
        let email = std::fs::read_to_string(folder.join(format!("{number}.eml"))).unwrap();
        // To the shop only, with replies going to the buyer.
        assert!(email.contains("Reply-To: \"Mira Ashwood\" <mira@example.com>"));
        assert!(!email.contains("\r\nCc:"), "{email}");
        let text = std::fs::read_to_string(folder.join(format!("{number}.txt"))).unwrap();
        // Priced by the service, with the buyer's materials.
        assert!(text.contains("2 \u{d7} 90,000 gil = 180,000 gil"), "{text}");
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[tokio::test]
    async fn bots_and_broken_orders_are_turned_away() {
        let folder = folder();
        let service = service(Some(saving(&folder)), 3);
        let trap = ORDER.replace(r#""website":"""#, r#""website":"https://spam.example""#);
        let (status, answer) = send(&service, &trap, "203.0.113.7").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(answer.problem.unwrap().ends_with("Or copy it for Discord."));
        let (status, answer) = send(&service, "{not json", "203.0.113.7").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            answer
                .problem
                .unwrap()
                .starts_with("Your order could not be read.")
        );
        let wrong = ORDER.replace("Mira Ashwood", "Mira");
        let (status, answer) = send(&service, &wrong, "203.0.113.7").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(answer.problem.unwrap().contains("first and last name"));
        // Nothing was sent.
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[tokio::test]
    async fn the_limits_hold() {
        let folder = folder();
        let service = service(Some(saving(&folder)), 1);
        assert_eq!(send(&service, ORDER, "203.0.113.7").await.0, StatusCode::OK);
        let (status, answer) = send(&service, ORDER, "198.51.100.4").await;
        // Another visitor, but the same email address.
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert!(answer.problem.unwrap().contains("in the last hour"));
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[tokio::test]
    async fn without_its_settings_it_says_so() {
        let (status, answer) = send(&service(None, 3), ORDER, "203.0.113.7").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        let words = answer.problem.unwrap();
        assert_eq!(
            words,
            "Sending orders by email is not ready yet. Or copy it for Discord."
        );
    }

    #[tokio::test]
    async fn a_message_is_emailed_with_replies_to_the_visitor() {
        let folder = folder();
        let service = messaging(Some(saving_messages(&folder)), 3);
        let (status, reply) = send_message(&service, MESSAGE, "203.0.113.7").await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(
            reply,
            Reply {
                sent: true,
                problem: None
            }
        );
        let names = saved(&folder);
        assert_eq!(names.len(), 3, "{names:?}");
        let eml = names.iter().find(|name| name.ends_with(".eml")).unwrap();
        assert!(eml.starts_with("message-"), "{eml}");
        let email = std::fs::read_to_string(folder.join(eml)).unwrap();
        for header in [
            "<support@example.com>\r\nTo: ",
            "Reply-To: \"Mira Ashwood\" <mira@example.com>\r\n",
            "Subject: Question about my order\r\n",
            "X-PM-Tag: contact\r\n",
        ] {
            assert!(email.contains(header), "missing {header:?} in\n{email}");
        }
        assert!(!email.contains("Cc:"));
        let text = std::fs::read_to_string(folder.join(eml.replace(".eml", ".txt"))).unwrap();
        assert!(
            text.contains("MESSAGE\n  Hello!\n\n  Can you craft a rapier?\n"),
            "{text}"
        );
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[tokio::test]
    async fn bots_and_broken_messages_are_turned_away() {
        let folder = folder();
        let service = messaging(Some(saving_messages(&folder)), 3);
        let trap = MESSAGE.replace(r#""website":"""#, r#""website":"https://spam.example""#);
        let (status, reply) = send_message(&service, &trap, "203.0.113.7").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(
            reply.problem.as_deref(),
            Some("Your message could not be sent.")
        );
        let (status, reply) = send_message(&service, "{not json", "203.0.113.7").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert!(
            reply
                .problem
                .unwrap()
                .starts_with("Your message could not be read.")
        );
        let nameless = MESSAGE.replace(" Mira Ashwood ", " ");
        let (status, reply) = send_message(&service, &nameless, "203.0.113.7").await;
        assert_eq!(status, StatusCode::BAD_REQUEST);
        assert_eq!(reply.problem.as_deref(), Some("Enter your name."));
        assert!(!reply.sent);
        // Nothing was sent, and nothing counted towards the limits.
        assert!(saved(&folder).is_empty());
        assert_eq!(
            send_message(&service, MESSAGE, "203.0.113.7").await.0,
            StatusCode::OK
        );
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[tokio::test]
    async fn messages_have_limits_of_their_own() {
        let folder = folder();
        let service = service_with(Some(saving(&folder)), Some(saving_messages(&folder)), 1);
        assert_eq!(
            send_message(&service, MESSAGE, "203.0.113.7").await.0,
            StatusCode::OK
        );
        // Another visitor, but the same address to reply to.
        let (status, reply) = send_message(&service, MESSAGE, "198.51.100.4").await;
        assert_eq!(status, StatusCode::TOO_MANY_REQUESTS);
        assert!(reply.problem.unwrap().contains("in the last hour"));
        // The first visitor again, with another address.
        let other = MESSAGE.replace("mira@example.com", "mira.ashwood@example.com");
        assert_eq!(
            send_message(&service, &other, "203.0.113.7").await.0,
            StatusCode::TOO_MANY_REQUESTS
        );
        // Orders are counted apart from messages.
        assert_eq!(send(&service, ORDER, "203.0.113.7").await.0, StatusCode::OK);
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[tokio::test]
    async fn without_its_settings_the_contact_page_says_so() {
        // Orders can be ready while messages aren't.
        let folder = folder();
        let service = service(Some(saving(&folder)), 3);
        let (status, reply) = send_message(&service, MESSAGE, "203.0.113.7").await;
        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(
            reply.problem.as_deref(),
            Some("Sending messages is not ready yet.")
        );
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn message_references_read_easily() {
        let now = clock::UkTime {
            year: 2026,
            month: 11,
            day: 24,
            hour: 9,
            minute: 5,
        };
        let reference = message_reference(&now);
        assert_eq!(&reference[..22], "message-20261124-0905-");
        assert_eq!(reference.len(), 30);
    }

    #[test]
    fn the_shop_loads_with_its_examples() {
        let shop = load_shop(&PathBuf::from(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../shop.toml"
        )))
        .unwrap();
        assert!(!shop.items.is_empty());
    }
}
