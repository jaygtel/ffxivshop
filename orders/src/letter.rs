//! The letter: the email the service sends for each message from the
//! contact page, to the shop. It comes from the shop's own address with the
//! visitor's name beside it, and replying to it answers the visitor (see
//! mail.rs). The visitor gets no copy. Like the order receipt, it's an HTML
//! email made to look like a window on the shop page, with a plain-text copy
//! inside the same email (see look.rs).

use crate::basket::tidy;
use crate::look::{self, INK, WIDTH, body_text, detail, esc, overline, wrap};
use crate::message::ContactMessage;
use crate::model::Shop;

/// The email, ready to send.
pub struct Letter {
    pub subject: String,
    pub html: String,
    pub text: String,
}

/// How much of the message shows in the inbox beside the subject.
const PREVIEW_LENGTH: usize = 140;

/// Writes the letter for `message`, which has been checked and tidied.
/// `sent` is when it was sent, written out; `assets` is where the site
/// serves the pictures and fonts (normally its own address).
pub fn write(shop: &Shop, message: &ContactMessage, sent: &str, assets: &str) -> Letter {
    let subject = message.subject.clone();
    let html = html(shop, message, sent, assets.trim_end_matches('/'), &subject);
    let text = text(shop, message, sent);
    Letter {
        subject,
        html,
        text,
    }
}

// ---------------------------------------------------------------------------
// The words, shared by both copies
// ---------------------------------------------------------------------------

/// What shows in the inbox beside the subject: the start of the message.
fn preview(message: &ContactMessage) -> String {
    let words = tidy(&message.message);
    if words.chars().count() <= PREVIEW_LENGTH {
        return words;
    }
    let start: String = words.chars().take(PREVIEW_LENGTH).collect();
    format!("{}\u{2026}", start.trim_end())
}

/// Where the letter came from: the middle line of the footer.
fn sent_from(shop: &Shop) -> String {
    format!("Sent from the contact page on {}.", look::website(shop))
}

/// The message's paragraphs, each a list of its lines.
fn paragraphs(message: &str) -> Vec<Vec<&str>> {
    message
        .split("\n\n")
        .map(|paragraph| paragraph.lines().collect())
        .collect()
}

// ---------------------------------------------------------------------------
// HTML
// ---------------------------------------------------------------------------

fn from(message: &ContactMessage, sent: &str) -> String {
    format!(
        "{}<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">{}{}{}</table>",
        overline("From"),
        detail("Name", &esc(&message.name), 56),
        detail(
            "Email",
            &format!(
                "<a href=\"mailto:{0}\" style=\"color:{INK};text-decoration:none;\">{0}</a>",
                esc(&message.email)
            ),
            56
        ),
        detail("Sent", &esc(sent), 56),
    )
}

fn subject(message: &ContactMessage) -> String {
    format!(
        "{}<div style=\"{}font-weight:500;word-break:break-word;\">{}</div>",
        overline("Subject"),
        body_text(15, 22, INK),
        esc(&message.subject)
    )
}

/// The message as the visitor wrote it: its paragraphs, and the line breaks
/// inside them.
fn body(message: &ContactMessage) -> String {
    let paragraphs = paragraphs(&message.message);
    let last = paragraphs.len().saturating_sub(1);
    let html: String = paragraphs
        .iter()
        .enumerate()
        .map(|(n, lines)| {
            let gap = if n == last { 0 } else { 12 };
            let lines: Vec<String> = lines.iter().map(|line| esc(line)).collect();
            format!(
                "<div style=\"margin:0 0 {gap}px;{}word-break:break-word;overflow-wrap:anywhere;\">{}</div>",
                body_text(15, 22, INK),
                lines.join("<br>")
            )
        })
        .collect();
    format!("{}{html}", overline("Message"))
}

fn html(shop: &Shop, message: &ContactMessage, sent: &str, assets: &str, title: &str) -> String {
    let sections = [from(message, sent), subject(message), body(message)];
    let footer = look::footer(shop, &esc(&sent_from(shop)));
    look::page(
        assets,
        shop,
        title,
        &preview(message),
        "",
        &look::slip(assets, "Message", "", &sections),
        &footer,
    )
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

fn text(shop: &Shop, message: &ContactMessage, sent: &str) -> String {
    let mut t = String::new();
    t.push_str(&format!("{}\n", shop.shop.name.to_uppercase()));
    t.push_str("Message from the contact page\n\n");

    t.push_str("FROM\n");
    t.push_str(&format!("  Name   {}\n", message.name));
    t.push_str(&format!("  Email  {}\n", message.email));
    t.push_str(&format!("  Sent   {sent}\n\n"));

    t.push_str("SUBJECT\n");
    t.push_str(&wrap(&message.subject, WIDTH, "  ", ""));

    t.push_str("\nMESSAGE\n");
    for (n, lines) in paragraphs(&message.message).iter().enumerate() {
        if n > 0 {
            t.push('\n');
        }
        for line in lines {
            t.push_str(&wrap(line, WIDTH, "  ", ""));
        }
    }

    t.push_str(&look::text_footer(shop, &sent_from(shop)));
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop() -> Shop {
        Shop::from_toml(include_str!("../../shop.toml")).unwrap()
    }

    fn sample(message: &str) -> ContactMessage {
        ContactMessage {
            name: "Mira Ashwood".into(),
            email: "mira@example.com".into(),
            subject: "Question about my <order>".into(),
            message: message.into(),
            website: String::new(),
        }
        .tidied()
    }

    fn letter(message: &str) -> Letter {
        write(
            &shop(),
            &sample(message),
            "24 Nov 2026, 11:52 UK time",
            "https://aetherandallure.com/",
        )
    }

    const MESSAGE: &str = "Hello!\n\nCan you craft a rapier in <b>Jet Black</b>?\nThanks & bye";

    #[test]
    fn the_subject_is_the_visitors_own() {
        assert_eq!(letter(MESSAGE).subject, "Question about my <order>");
    }

    #[test]
    fn the_html_holds_the_whole_message() {
        let html = letter(MESSAGE).html;
        for part in [
            "<title>Question about my &lt;order&gt;</title>",
            ">Message</td>",
            "Mira Ashwood",
            "mailto:mira@example.com",
            "24 Nov 2026, 11:52 UK time",
            "Question about my &lt;order&gt;",
            // Its paragraphs and line breaks, and nothing the visitor wrote
            // becomes HTML.
            ">Hello!</div>",
            "Can you craft a rapier in &lt;b&gt;Jet Black&lt;/b&gt;?<br>Thanks &amp; bye</div>",
            "Sent from the contact page on aetherandallure.com.",
            "Website design \u{a9} 2026",
            "https://aetherandallure.com/assets/img/email/corner-tl.png",
            "https://aetherandallure.com/assets/fonts/cinzel.woff2",
        ] {
            assert!(html.contains(part), "missing {part:?}");
        }
        assert!(!html.contains("<b>Jet"));
        assert!(!html.contains("<order>"));
        assert!(!html.contains(".com//assets"));
        // Gmail cuts off emails over about 102 KB.
        assert!(html.len() < 60_000, "{} bytes", html.len());
        // The longest message still fits.
        let long = letter(&"Lorem ipsum dolor sit amet. ".repeat(71)).html;
        assert!(long.len() < 60_000, "{} bytes", long.len());
    }

    #[test]
    fn the_inbox_shows_the_start_of_the_message() {
        let html = letter(MESSAGE).html;
        assert!(html.contains(
            "mso-hide:all;\">Hello! Can you craft a rapier in &lt;b&gt;Jet Black&lt;/b&gt;? Thanks &amp; bye&#847;"
        ));
        let long = sample(&"word ".repeat(100));
        let preview = preview(&long);
        assert!(preview.ends_with("word\u{2026}"), "{preview}");
        assert!(preview.chars().count() <= PREVIEW_LENGTH + 1);
    }

    #[test]
    fn the_plain_text_says_the_same() {
        let long_line =
            "This line is long enough that it has to wrap onto the next line in the plain text.";
        let text = letter(&format!("{MESSAGE}\n\n{long_line}")).text;
        for part in [
            "AETHER & ALLURE\nMessage from the contact page\n\n",
            "FROM\n  Name   Mira Ashwood\n  Email  mira@example.com\n  Sent   24 Nov 2026, 11:52 UK time\n\n",
            "SUBJECT\n  Question about my <order>\n",
            "\nMESSAGE\n  Hello!\n\n  Can you craft a rapier in <b>Jet Black</b>?\n  Thanks & bye\n\n  This line is long",
            "\n-- \nAether & Allure \u{b7} https://aetherandallure.com\nSent from the contact page on aetherandallure.com.\n",
        ] {
            assert!(text.contains(part), "missing {part:?} in\n{text}");
        }
        for line in text.lines() {
            assert!(line.chars().count() <= WIDTH, "too long: {line:?}");
        }
    }

    #[test]
    fn its_words_are_plain_english() {
        let letter = letter(MESSAGE);
        assert!(!letter.text.contains("n't"), "{}", letter.text);
    }
}
