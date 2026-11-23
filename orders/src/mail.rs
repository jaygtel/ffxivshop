//! Sending a receipt: by SMTP through Postmark, or, for trying the service
//! out, into files instead.
//!
//! Postmark takes SMTP on port 587 (or 2525, where a host blocks 587), with
//! STARTTLS, and the server token as both the user name and the password.
//! The X-PM-Message-Stream header picks the message stream.

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
    reply_to: Option<Mailbox>,
    stream: String,
    way: Way,
}

enum Way {
    Postmark(Box<AsyncSmtpTransport<Tokio1Executor>>),
    Files(PathBuf),
}

/// Who an email is from, to, and where replies go.
pub struct Addresses<'a> {
    pub shop_name: &'a str,
    pub from: &'a str,
    pub to: &'a str,
    pub reply_to: Option<&'a str>,
}

impl Addresses<'_> {
    fn mailboxes(&self) -> Result<(Mailbox, Mailbox, Option<Mailbox>), String> {
        let mailbox = |setting: &str, address: &str| {
            address
                .parse()
                .map(|address| Mailbox::new(Some(self.shop_name.to_string()), address))
                .map_err(|_| format!("{setting} \"{address}\" isn't an email address"))
        };
        Ok((
            mailbox("ORDER_EMAIL_FROM", self.from)?,
            mailbox("ORDER_EMAIL_TO", self.to)?,
            self.reply_to
                .map(|address| mailbox("ORDER_EMAIL_REPLY_TO", address))
                .transpose()?,
        ))
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
        let (from, to, reply_to) = addresses.mailboxes()?;
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
            reply_to,
            stream: stream.to_string(),
            way: Way::Postmark(Box::new(transport)),
        })
    }

    /// Saves each email in `folder`, as the email itself (.eml) and its HTML
    /// and plain-text copies, instead of sending it.
    pub fn files(addresses: &Addresses, folder: PathBuf, stream: &str) -> Result<Mailer, String> {
        let (from, to, reply_to) = addresses.mailboxes()?;
        std::fs::create_dir_all(&folder)
            .map_err(|e| format!("can't make the folder {}: {e}", folder.display()))?;
        Ok(Mailer {
            from,
            to,
            reply_to,
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

    pub fn describe(&self) -> String {
        match &self.way {
            Way::Postmark(_) => format!(
                "sending orders from {} to {}, through Postmark",
                self.from.email, self.to.email
            ),
            Way::Files(folder) => format!(
                "saving orders from {} to {} in {}, without sending them",
                self.from.email,
                self.to.email,
                folder.display()
            ),
        }
    }

    /// The email for an order: to the shop, with a copy to the buyer.
    pub fn message(
        &self,
        receipt: &Receipt,
        number: &str,
        buyer: Mailbox,
    ) -> Result<Message, String> {
        let mut builder = Message::builder()
            .from(self.from.clone())
            .to(self.to.clone())
            .cc(buyer)
            .subject(&receipt.subject)
            .date_now()
            .message_id(Some(format!(
                "<{number}.{}@{}>",
                random_hex(8),
                self.from.email.domain()
            )))
            .header(MessageStream(self.stream.clone()))
            .header(Tag("order".into()));
        if let Some(reply_to) = &self.reply_to {
            builder = builder.reply_to(reply_to.clone());
        }
        builder
            .multipart(MultiPart::alternative_plain_html(
                receipt.text.clone(),
                receipt.html.clone(),
            ))
            .map_err(|e| format!("couldn't put the email together: {e}"))
    }

    pub async fn send(
        &self,
        message: Message,
        number: &str,
        receipt: &Receipt,
    ) -> Result<(), String> {
        match &self.way {
            Way::Postmark(transport) => transport
                .send(message)
                .await
                .map(|_| ())
                .map_err(|e| e.to_string()),
            Way::Files(folder) => {
                let save = |extension: &str, bytes: &[u8]| {
                    let path = folder.join(format!("{number}.{extension}"));
                    std::fs::write(&path, bytes)
                        .map_err(|e| format!("can't write {}: {e}", path.display()))
                };
                save("eml", &message.formatted())?;
                save("html", receipt.html.as_bytes())?;
                save("txt", receipt.text.as_bytes())
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

/// Postmark's tag, for finding orders among the sent emails in Postmark.
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
        // Only for the Message-ID, which still has the order number.
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

    fn addresses(reply_to: Option<&'static str>) -> Addresses<'static> {
        Addresses {
            shop_name: "Aether & Allure",
            from: "shop@example.com",
            to: "orders@example.com",
            reply_to,
        }
    }

    #[test]
    fn the_email_goes_to_the_shop_with_a_copy_to_the_buyer() {
        let folder = std::env::temp_dir().join(format!("shop-orders-test-{}", random_hex(4)));
        let mailer = Mailer::files(&addresses(None), folder.clone(), "outbound").unwrap();
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
            "Cc: \"Mira Ashwood\" <mira@example.com>\r\n",
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
        assert!(!text.contains("Reply-To"));
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
        block_on(mailer.send(message, "AA-2611-7K3Q", &receipt())).unwrap();
        for extension in ["eml", "html", "txt"] {
            assert!(folder.join(format!("AA-2611-7K3Q.{extension}")).exists());
        }
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn replies_can_go_elsewhere() {
        let folder = std::env::temp_dir().join(format!("shop-orders-test-{}", random_hex(4)));
        let mailer = Mailer::files(
            &addresses(Some("replies@example.com")),
            folder.clone(),
            "outbound",
        )
        .unwrap();
        let message = mailer
            .message(&receipt(), "AA-1", "mira@example.com".parse().unwrap())
            .unwrap();
        let text = String::from_utf8(message.formatted()).unwrap();
        assert!(
            text.contains("Reply-To: =?utf-8?b?QWV0aGVyICYgQWxsdXJl?= <replies@example.com>\r\n")
        );
        std::fs::remove_dir_all(folder).unwrap();
    }

    #[test]
    fn bad_addresses_in_the_settings_are_explained() {
        let bad = Addresses {
            shop_name: "Aether & Allure",
            from: "shop at example",
            to: "orders@example.com",
            reply_to: None,
        };
        let problem = Mailer::files(&bad, std::env::temp_dir(), "outbound")
            .err()
            .unwrap();
        assert!(problem.contains("ORDER_EMAIL_FROM"), "{problem}");
    }

    #[test]
    fn random_hex_is_hex() {
        let hex = random_hex(8);
        assert_eq!(hex.len(), 16);
        assert!(hex.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(random_hex(8), random_hex(8));
    }
}
