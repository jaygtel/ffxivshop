//! A message from the contact page, and its checks, without the page. The
//! contact page (src/pages/contact.rs) and the orders service (orders/,
//! which emails the message) both compile this file, so the service checks a
//! message exactly as the page does. Keep it free of browser-only code.

use crate::basket::{looks_like_an_email, tidy};
use serde::{Deserialize, Serialize};

/// Where the contact page sends a message to be emailed: the orders service,
/// behind the site's own web server (see deploy/nginx.conf).
pub const SEND_ADDRESS: &str = "/api/contact";

/// The most characters each box on the contact page takes.
pub const NAME_LIMIT: usize = 50;
pub const SUBJECT_LIMIT: usize = 100;
pub const MESSAGE_LIMIT: usize = 2000;

/// A message to email, as the contact page sends it to the orders service.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct ContactMessage {
    pub name: String,
    /// Where the reply goes.
    pub email: String,
    pub subject: String,
    pub message: String,
    /// A box that's hidden on the page, so people never fill it in. Bots
    /// that fill in every box do.
    #[serde(default)]
    pub website: String,
}

/// The orders service's answer to a message.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct Reply {
    /// The message is on its way.
    #[serde(default)]
    pub sent: bool,
    /// What went wrong, in words the contact page shows the visitor.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub problem: Option<String>,
}

/// The boxes on the contact page, in the order they come.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Part {
    Name,
    Email,
    Subject,
    Message,
}

impl ContactMessage {
    /// The message as the visitor meant it: the name, email and subject
    /// each on one line, and the message's own line breaks kept.
    pub fn tidied(&self) -> ContactMessage {
        ContactMessage {
            name: one_line(&self.name),
            email: self.email.trim().to_string(),
            subject: one_line(&self.subject),
            message: tidy_message(&self.message),
            website: self.website.clone(),
        }
    }

    /// What's wrong with the message, if anything, and in which box: the
    /// first box with a problem, in the order the boxes come on the page. It
    /// checks the message as `tidied` leaves it.
    pub fn problem(&self) -> Option<(Part, &'static str)> {
        let tidied = self.tidied();
        let at = |part: Part| move |words: &'static str| (part, words);
        name_problem(&tidied.name)
            .map(at(Part::Name))
            .or_else(|| email_problem(&tidied.email).map(at(Part::Email)))
            .or_else(|| subject_problem(&tidied.subject).map(at(Part::Subject)))
            .or_else(|| message_problem(&tidied.message).map(at(Part::Message)))
    }
}

/// Text on one line, as the visitor meant it: no spaces at the ends, one
/// space between words, and nothing that isn't printed, such as line breaks.
fn one_line(text: &str) -> String {
    tidy(&text.replace(|c: char| c.is_control(), " "))
}

/// The message with its line breaks kept, but no spaces at the ends of its
/// lines, no empty lines at the start or end, and no more than one empty
/// line in a row. Tabs become spaces, and anything else that isn't printed
/// goes.
fn tidy_message(text: &str) -> String {
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut out = String::with_capacity(text.len());
    let mut empty_lines = 0;
    for line in text.split('\n') {
        let line: String = line
            .chars()
            .map(|c| if c == '\t' { ' ' } else { c })
            .filter(|c| !c.is_control())
            .collect();
        let line = line.trim_end();
        if line.trim().is_empty() {
            empty_lines += 1;
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
            if empty_lines > 0 {
                out.push('\n');
            }
        }
        out.push_str(line);
        empty_lines = 0;
    }
    out
}

fn too_long(text: &str, limit: usize) -> bool {
    text.chars().count() > limit
}

fn has_web_address(text: &str) -> bool {
    let lower = text.to_lowercase();
    lower.contains("://") || lower.contains("www.")
}

/// What's wrong with the visitor's name, if anything.
fn name_problem(name: &str) -> Option<&'static str> {
    if name.is_empty() {
        Some("Enter your name.")
    } else if too_long(name, NAME_LIMIT) {
        Some("Keep your name to 50 characters or fewer.")
    } else if has_web_address(name) {
        Some("Enter only your name, with no web address.")
    } else {
        None
    }
}

/// What's wrong with the visitor's email address, if anything. It only
/// checks the address looks like one: a mistyped address can still look
/// right.
fn email_problem(email: &str) -> Option<&'static str> {
    if email.is_empty() {
        Some("Enter your email address, so I can reply.")
    } else if !looks_like_an_email(email) {
        Some("Check your email address. It should look like name@example.com.")
    } else {
        None
    }
}

/// What's wrong with the subject, if anything. It's the email's subject.
fn subject_problem(subject: &str) -> Option<&'static str> {
    if subject.is_empty() {
        Some("Enter a subject, so I know what your message is about.")
    } else if too_long(subject, SUBJECT_LIMIT) {
        Some("Keep the subject to 100 characters or fewer.")
    } else {
        None
    }
}

/// What's wrong with the message, if anything. Web links are fine here, as
/// the message only goes to the shop.
fn message_problem(message: &str) -> Option<&'static str> {
    if message.is_empty() {
        Some("Write your message.")
    } else if too_long(message, MESSAGE_LIMIT) {
        Some("Keep your message to 2,000 characters or fewer.")
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> ContactMessage {
        ContactMessage {
            name: "  Mira   Ashwood ".into(),
            email: " mira@example.com ".into(),
            subject: " Question\tabout my order\n".into(),
            message: "\n\nHello!  \r\n\r\n\r\nCan you craft\ta rapier?\n\n".into(),
            website: String::new(),
        }
    }

    #[test]
    fn messages_are_tidied_as_the_visitor_meant_them() {
        let tidied = sample().tidied();
        assert_eq!(tidied.name, "Mira Ashwood");
        assert_eq!(tidied.email, "mira@example.com");
        assert_eq!(tidied.subject, "Question about my order");
        assert_eq!(tidied.message, "Hello!\n\nCan you craft a rapier?");
        assert_eq!(sample().problem(), None);
        assert_eq!(tidy_message("one\ntwo\n\nthree"), "one\ntwo\n\nthree");
        assert_eq!(tidy_message(" \n\t\n"), "");
        assert_eq!(tidy_message("  indented\nkept"), "  indented\nkept");
        assert_eq!(one_line("a\u{7}b\u{0}c"), "a b c");
    }

    #[test]
    fn each_box_is_checked_in_turn() {
        let with = |change: fn(&mut ContactMessage)| {
            let mut message = sample();
            change(&mut message);
            message.problem().map(|(_, words)| words)
        };
        assert_eq!(with(|m| m.name = " ".into()), Some("Enter your name."));
        assert!(with(|m| m.name = "a".repeat(51)).unwrap().contains("50"));
        assert_eq!(with(|m| m.name = "a".repeat(50)), None);
        assert!(with(|m| m.name = "www.example.com".into()).is_some());
        // Written the way the game writes a character on a world.
        assert_eq!(with(|m| m.name = "Mira Ashwood@Cerberus".into()), None);
        assert!(
            with(|m| m.email = "".into())
                .unwrap()
                .contains("so I can reply")
        );
        assert!(with(|m| m.email = "mira@example".into()).is_some());
        assert!(with(|m| m.email = "mira@example.com\r\nBcc: other@example.com".into()).is_some());
        assert!(with(|m| m.subject = "".into()).unwrap().contains("subject"));
        assert!(with(|m| m.subject = "a".repeat(101)).is_some());
        assert_eq!(
            with(|m| m.message = "\n \n".into()),
            Some("Write your message.")
        );
        assert!(
            with(|m| m.message = "a".repeat(2001))
                .unwrap()
                .contains("2,000")
        );
        assert_eq!(with(|m| m.message = "a".repeat(2000)), None);
        // Web links are fine in the message: it only goes to the shop.
        assert_eq!(
            with(|m| m.message = "See https://example.com/screenshot.png".into()),
            None
        );
        // The boxes are checked in the order they come on the page.
        let mut empty = ContactMessage::default();
        assert_eq!(empty.problem(), Some((Part::Name, "Enter your name.")));
        empty.name = "Mira".into();
        assert_eq!(empty.problem().unwrap().0, Part::Email);
        empty.email = "mira@example.com".into();
        assert_eq!(empty.problem().unwrap().0, Part::Subject);
        empty.subject = "Hello".into();
        assert_eq!(empty.problem().unwrap().0, Part::Message);
    }

    #[test]
    fn the_words_here_are_plain_english() {
        for words in [
            name_problem("").unwrap(),
            name_problem(&"a".repeat(51)).unwrap(),
            name_problem("https://example.com").unwrap(),
            email_problem("").unwrap(),
            email_problem("mira").unwrap(),
            subject_problem("").unwrap(),
            subject_problem(&"a".repeat(101)).unwrap(),
            message_problem("").unwrap(),
            message_problem(&"a".repeat(2001)).unwrap(),
        ] {
            assert!(!words.contains("n't"), "no negative contractions: {words}");
        }
    }

    #[test]
    fn messages_travel_as_json() {
        let text = serde_json::to_string(&sample()).unwrap();
        assert_eq!(
            serde_json::from_str::<ContactMessage>(&text).unwrap(),
            sample()
        );
        // The hidden box can be left out.
        let short: ContactMessage = serde_json::from_str(
            r#"{"name":"Mira","email":"mira@example.com","subject":"Hi","message":"Hello"}"#,
        )
        .unwrap();
        assert!(short.website.is_empty());
        let sent = Reply {
            sent: true,
            problem: None,
        };
        assert_eq!(serde_json::to_string(&sent).unwrap(), r#"{"sent":true}"#);
    }
}
