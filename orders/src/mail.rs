//! Sending a receipt, or a message from the contact page: by SMTP through
//! Postmark, or, for trying the service out, into files instead.
//!
//! Postmark takes SMTP on port 587 (or 2525, where a host blocks 587), with
//! STARTTLS, and the server token as both the user name and the password.
//! The X-PM-Message-Stream header picks the message stream.

use crate::letter::Letter;
use crate::receipt::Receipt;
use lettre::message::header::{Header, HeaderName, HeaderValue};
use lettre::message::{Mailbox, MultiPart};
use lettre::transport::smtp::authentication::{Credentials, Mechanism};
use lettre::transport::smtp::extension::ClientId;
use lettre::{AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor};
use std::path::PathBuf;
use std::time::Duration;

/// How long a send may take before it counts as failed.
const SEND_TIMEOUT: Duration = Duration::from_secs(20);

pub struct Mailer {
    from: Mailbox,
    to: Mailbox,
    stream: String,
    way: Way,
}

enum Way {
    Postmark(Box<AsyncSmtpTransport<Tokio1Executor>>),
    Files(PathBuf),
}

/// Who an email is from and to. Replies go to whoever sent the order or
/// message (see Mailer::message and Mailer::letter).
pub struct Addresses<'a> {
    pub shop_name: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    /// How the settings for these addresses start, to say which one is
    /// wrong: "ORDER_EMAIL" or "CONTACT_EMAIL".
    pub settings: &'a str,
}

impl Addresses<'_> {
    fn mailboxes(&self) -> Result<(Mailbox, Mailbox), String> {
        let mailbox = |setting: &str, address: &str| {
            address
                .parse()
                .map(|address| Mailbox::new(Some(self.shop_name.to_string()), address))
                .map_err(|_| {
                    format!(
                        "{}_{setting} \"{address}\" isn't an email address",
                        self.settings
                    )
                })
        };
        Ok((mailbox("FROM", self.from)?, mailbox("TO", self.to)?))
    }
}

impl Mailer {
    /// Sends through Postmark's SMTP server.
    pub fn postmark(
        addresses: &Addresses,
        token: &str,
        host: &str,
        port: u16,
        stream: &str,
    ) -> Result<Mailer, String> {
        let (from, to) = addresses.mailboxes()?;
        let transport = AsyncSmtpTransport::<Tokio1Executor>::starttls_relay(host)
            .map_err(|e| format!("can't use {host} for email: {e}"))?
            .port(port)
            .credentials(Credentials::new(token.to_string(), token.to_string()))
            .authentication(vec![Mechanism::Plain])
            .hello_name(ClientId::Domain(from.email.domain().to_string()))
            .timeout(Some(SEND_TIMEOUT))
            .build();
        Ok(Mailer {
            from,
            to,
            stream: stream.to_string(),
            way: Way::Postmark(Box::new(transport)),
        })
    }

    /// Saves each email in `folder`, as the email itself (.eml) and its HTML
    /// and plain-text copies, instead of sending it.
    pub fn files(addresses: &Addresses, folder: PathBuf, stream: &str) -> Result<Mailer, String> {
        let (from, to) = addresses.mailboxes()?;
        std::fs::create_dir_all(&folder)
            .map_err(|e| format!("can't make the folder {}: {e}", folder.display()))?;
        Ok(Mailer {
            from,
            to,
            stream: stream.to_string(),
            way: Way::Files(folder),
        })
    }

    /// "Sent", or "Saved" when it only saves emails in files.
    pub fn done(&self) -> &'static str {
        match self.way {
            Way::Postmark(_) => "Sent",
            Way::Files(_) => "Saved",
        }
    }

    /// What it does with the emails, which are `what` (orders or messages).
    pub fn describe(&self, what: &str) -> String {
        match &self.way {
            Way::Postmark(_) => format!(
                "sending {what} from {} to {}, through Postmark",
                self.from.email, self.to.email
            ),
            Way::Files(folder) => format!(
                "saving {what} from {} to {} in {}, without sending them",
                self.from.email,
                self.to.email,
                folder.display()
            ),
        }
    }

    /// The email for an order: to the shop only, with Reply-To set to the
    /// buyer, so replying answers them. The buyer gets no copy: the address
    /// they gave hasn't been confirmed, so nothing is sent to it.
    pub fn message(
        &self,
        receipt: &Receipt,
        number: &str,
        buyer: Mailbox,
    ) -> Result<Message, String> {
        Message::builder()
            .from(self.from.clone())
            .to(self.to.clone())
            .reply_to(buyer)
            .subject(&receipt.subject)
            .date_now()
            .message_id(Some(format!(
                "<{number}.{}@{}>",
                random_hex(8),
                self.from.email.domain()
            )))
            .header(MessageStream(self.stream.clone()))
            .header(Tag("order".into()))
            .multipart(MultiPart::alternative_plain_html(
                receipt.text.clone(),
                receipt.html.clone(),
            ))
            .map_err(|e| format!("couldn't put the email together: {e}"))
    }

    /// The email for a message from the contact page: to the shop, from the
    /// shop's own address with the visitor's name beside it ("Mira Ashwood
    /// via Aether & Allure"), and with Reply-To set to the visitor, so
    /// replying answers them. `reference` makes its Message-ID unique.
    pub fn letter(
        &self,
        letter: &Letter,
        reference: &str,
        visitor: Mailbox,
    ) -> Result<Message, String> {
        let shop = self.from.name.clone().unwrap_or_default();
        let name = match visitor.name.as_deref() {
            Some(name) if !name.is_empty() => format!("{name} via {shop}"),
            _ => shop,
        };
        Message::builder()
            .from(Mailbox::new(Some(name), self.from.email.clone()))
            .to(self.to.clone())
            .reply_to(visitor)
            .subject(&letter.subject)
            .date_now()
            .message_id(Some(format!("<{reference}@{}>", self.from.email.domain())))
            .header(MessageStream(self.stream.clone()))
            .header(Tag("contact".into()))
            .multipart(MultiPart::alternative_plain_html(
                letter.text.clone(),
                letter.html.clone(),
            ))
            .map_err(|e| format!("couldn't put the email together: {e}"))
    }

    /// Sends `message`, or saves it as `name`.eml, with its `html` and
    /// `text` copies beside it as `name`.html and `name`.txt.
    pub async fn send(
        &self,
        message: Message,
        name: &str,
        html: &str,
        text: &str,
    ) -> Result<(), String> {
        match &self.way {
            Way::Postmark(transport) => transport
                .send(message)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Way::Files(folder) => {
                let save = |extension: &str, bytes: &[u8]| {
                    let path = folder.join(format!("{name}.{extension}"));
                    std::fs::write(&path, bytes)
                        .map_err(|e| format!("can't write {}: {e}", path.display()))
                };
                save("eml", &message.formatted())?;
                save("html", html.as_bytes())?;
                save("txt", text.as_bytes())
            }
        }
    }
}

/// Picks Postmark's message stream: "outbound" is the transactional one.
#[derive(Debug, Clone)]
struct MessageStream(String);

impl Header for MessageStream {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str("X-PM-Message-Stream")
    }

    fn parse(text: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(MessageStream(text.to_string()))
    }

    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// Postmark's tag, for finding orders or messages among the sent emails in
/// Postmark.
#[derive(Debug, Clone)]
struct Tag(String);

impl Header for Tag {
    fn name() -> HeaderName {
        HeaderName::new_from_ascii_str("X-PM-Tag")
    }

    fn parse(text: &str) -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        Ok(Tag(text.to_string()))
    }

    fn display(&self) -> HeaderValue {
        HeaderValue::new(Self::name(), self.0.clone())
    }
}

/// `bytes` random bytes, as hex.
pub fn random_hex(bytes: usize) -> String {
    let mut buffer = vec![0u8; bytes];
    if getrandom::fill(&mut buffer).is_err() {
        // Only for the Message-ID, which still has the order number, or for
        // a message's reference, which also has the time.
        let nanos = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.subsec_nanos())
            .unwrap_or(0);
        buffer = nanos.to_be_bytes().repeat(bytes.div_ceil(4))[..bytes].to_vec();
    }
    buffer.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> Receipt {
        Receipt {
            subject: "Order AA-2611-7K3Q from Mira Ashwood (Cerberus)".into(),
            html: "<!DOCTYPE html><p>Receipt</p>".into(),
            text: "Receipt\n".into(),
        }
    }

    fn addresses() -> Addresses<'static> {
        Addresses {
            shop_name: "Aether & Allure",
            from: "shop@example.com",
            to: "orders@example.com",
            settings: "ORDER_EMAIL",
        }
    }

    #[test]
    fn the_email_goes_to_the_shop_and_replies_go_to_the_buyer() {
        let folder = std::env::temp_dir().join(format!("shop-orders-test-{}", random_hex(4)));
        let mailer = Mailer::files(&addresses(), folder.clone(), "outbound").unwrap();
        let buyer = Mailbox::new(
            Some("Mira Ashwood".into()),
            "mira@example.com".parse().unwrap(),
        );
        let message = mailer.message(&receipt(), "AA-2611-7K3Q", buyer).unwrap();
        let text = String::from_utf8(message.formatted()).unwrap();
        // "Aether & Allure", written the way email headers carry an "&".
        let shop = "=?utf-8?b?QWV0aGVyICYgQWxsdXJl?=";
        for header in [
            &format!("From: {shop} <shop@example.com>\r\n"),
            &format!("To: {shop} <orders@example.com>\r\n"),
            "Reply-To: \"Mira Ashwood\" <mira@example.com>\r\n",
            "Subject: Order AA-2611-7K3Q from Mira Ashwood (Cerberus)\r\n",
            "X-PM-Message-Stream: outbound\r\n",
            "X-PM-Tag: order\r\n",
            "Message-ID: <AA-2611-7K3Q.",
            "@example.com>\r\n",
            "Content-Type: multipart/alternative;",
            "Content-Type: text/plain; charset=utf-8",
            "Content-Type: text/html; charset=utf-8",
        ] {
            assert!(text.contains(header), "missing {header:?} in\n{text}");
        }
        // Nothing goes to the buyer: the address they gave hasn't been
        // confirmed.
        assert!(!text.contains("\r\nCc:"), "{text}");
        assert!(!text.contains("\r\nBcc:"), "{text}");
        assert_eq!(text.matches("mira@example.com").count(), 1, "{text}");
        // The plain text comes first, so email programs that can show HTML
        // show the HTML.
        assert!(text.find("text/plain").unwrap() < text.find("text/html").unwrap());

        let block_on = |future| {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(future)
        };
        let message = mailer
            .message(
                &receipt(),
                "AA-2611-7K3Q",
                "mira@example.com".parse().unwrap(),
            )
            .unwrap();
        let receipt = receipt();
        block_on(mailer.send(message, "AA-2611-7K3Q", &receipt.html, &receipt.text)).unwrap();
        for extension in ["eml", "html", "txt"] {
            assert!(folder.join(format!("AA-2611-7K3Q.{extension}")).exists());
        }
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn bad_addresses_in_the_settings_are_explained() {
        let bad = Addresses {
            shop_name: "Aether & Allure",
            from: "shop at example",
            to: "orders@example.com",
            settings: "ORDER_EMAIL",
        };
        let problem = Mailer::files(&bad, std::env::temp_dir(), "outbound")
            .err()
            .unwrap();
        assert!(problem.contains("ORDER_EMAIL_FROM"), "{problem}");
        let bad = Addresses {
            to: "support",
            settings: "CONTACT_EMAIL",
            ..bad
        };
        let problem = Mailer::files(&bad, std::env::temp_dir(), "outbound")
            .err()
            .unwrap();
        assert!(problem.contains("CONTACT_EMAIL_FROM"), "{problem}");
    }

    #[test]
    fn a_message_goes_to_the_shop_and_replies_go_to_the_visitor() {
        let folder = std::env::temp_dir().join(format!("shop-orders-test-{}", random_hex(4)));
        let support = Addresses {
            shop_name: "Aether & Allure",
            from: "support@example.com",
            to: "support@example.com",
            settings: "CONTACT_EMAIL",
        };
        let mailer = Mailer::files(&support, folder.clone(), "outbound").unwrap();
        let letter = Letter {
            subject: "Question about my order".into(),
            html: "<!DOCTYPE html><p>Message</p>".into(),
            text: "Message\n".into(),
        };
        let visitor = Mailbox::new(
            Some("Mira Ashwood".into()),
            "mira@example.com".parse().unwrap(),
        );
        let message = mailer
            .letter(&letter, "message-2611241152-ab12cd34", visitor)
            .unwrap();
        let text = String::from_utf8(message.formatted()).unwrap();
        for header in [
            // "Mira Ashwood via Aether & Allure", written the way email
            // headers carry an "&".
            "From: =?utf-8?b?TWlyYSBBc2h3b29kIHZpYSBBZXRoZXIgJiBBbGx1cmU=?= <support@example.com>\r\n",
            "To: =?utf-8?b?QWV0aGVyICYgQWxsdXJl?= <support@example.com>\r\n",
            "Reply-To: \"Mira Ashwood\" <mira@example.com>\r\n",
            "Subject: Question about my order\r\n",
            "X-PM-Message-Stream: outbound\r\n",
            "X-PM-Tag: contact\r\n",
            "Message-ID: <message-2611241152-ab12cd34@example.com>\r\n",
            "Content-Type: multipart/alternative;",
        ] {
            assert!(text.contains(header), "missing {header:?} in\n{text}");
        }
        // No copy goes to the visitor: the shop is the only recipient.
        assert!(!text.contains("Cc:"));
        assert!(!text.contains("Bcc:"));
        let envelope = message.envelope();
        assert_eq!(
            envelope.from().map(ToString::to_string).as_deref(),
            Some("support@example.com")
        );
        let recipients: Vec<String> = envelope.to().iter().map(ToString::to_string).collect();
        assert_eq!(recipients, ["support@example.com"]);

        let block_on = |future| {
            tokio::runtime::Builder::new_current_thread()
                .build()
                .unwrap()
                .block_on(future)
        };
        block_on(mailer.send(message, "message-1", &letter.html, &letter.text)).unwrap();
        for extension in ["eml", "html", "txt"] {
            assert!(folder.join(format!("message-1.{extension}")).exists());
        }
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn random_hex_is_hex() {
        let hex = random_hex(8);
        assert_eq!(hex.len(), 16);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(random_hex(8), random_hex(8));
    }
}
