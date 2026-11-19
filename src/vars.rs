//! Fills in `${SHOP_…}` values in shop.toml when the site is built, so details
//! such as the Discord name and email can live in `.env` rather than the code.
//!
//! In shop.toml, `${SHOP_EMAIL}` becomes the value of SHOP_EMAIL, and
//! `${SHOP_EMAIL:-commissions@example.com}` becomes that value or, when it
//! isn't set or is empty, the example after `:-`. Only names starting with
//! SHOP_ are read. Their values are published on the page, so nothing else
//! from the environment or `.env` can end up there by mistake. Lines that
//! start with `#` are comments and are left as they are.
//!
//! build.rs uses this, looking each name up in the environment first, then in
//! `.env`. main.rs compiles it only for its tests.

/// shop.toml with its `${SHOP_…}` values filled in.
#[derive(Debug)]
pub struct Filled {
    pub text: String,
    /// The name and example of each value that fell back to its example.
    pub examples: Vec<(String, String)>,
}

/// Replaces each `${SHOP_NAME}` or `${SHOP_NAME:-example}` in `text`. A value
/// is written as it would be inside a "double-quoted" TOML string, so quotes
/// or backslashes in it can't break the file.
pub fn fill(text: &str, lookup: impl Fn(&str) -> Option<String>) -> Result<Filled, String> {
    let mut filled = String::with_capacity(text.len());
    let mut examples = Vec::new();
    let mut problems = Vec::new();
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with('#') {
            filled.push_str(line);
            continue;
        }
        let mut rest = line;
        while let Some(start) = rest.find("${") {
            filled.push_str(&rest[..start]);
            let after = &rest[start + 2..];
            let Some(end) = after.find('}') else {
                problems.push(format!("{} has a ${{ with no closing }}", line.trim()));
                rest = "";
                break;
            };
            let inside = &after[..end];
            let (name, example) = match inside.split_once(":-") {
                Some((name, example)) => (name, Some(example)),
                None => (inside, None),
            };
            let valid = name.starts_with("SHOP_")
                && name
                    .chars()
                    .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_');
            if !valid {
                problems.push(format!(
                    "${{{inside}}}: only names in capitals that start with SHOP_ can be used, such as ${{SHOP_EMAIL}}"
                ));
            } else {
                match (lookup(name).filter(|value| !value.is_empty()), example) {
                    (Some(value), _) => filled.push_str(&escape(&value)),
                    (None, Some(example)) => {
                        filled.push_str(example);
                        examples.push((name.to_string(), example.to_string()));
                    }
                    (None, None) => problems.push(format!(
                        "{name} isn't set in .env, and ${{{name}}} has no example to fall back on \
                         (write it as ${{{name}:-example}})"
                    )),
                }
            }
            rest = &after[end + 1..];
        }
        filled.push_str(rest);
    }
    if problems.is_empty() {
        Ok(Filled {
            text: filled,
            examples,
        })
    } else {
        Err(problems.join("\n"))
    }
}

/// A value as it's written inside a "double-quoted" TOML string.
fn escape(value: &str) -> String {
    let mut escaped = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => escaped.push_str("\\\\"),
            '"' => escaped.push_str("\\\""),
            c if c.is_control() => escaped.push_str(&format!("\\u{:04X}", u32::from(c))),
            c => escaped.push(c),
        }
    }
    escaped
}

/// The `NAME=value` lines of a `.env` file, read the way Docker Compose reads
/// them: blank lines and `#` comments are skipped, `export ` before a name is
/// allowed, a value may be in single or double quotes, and an unquoted value
/// ends at ` #`, where a comment starts. Later lines win over earlier ones.
pub fn read_env_file(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| {
            let line = line.trim();
            if line.is_empty() || line.starts_with('#') {
                return None;
            }
            let line = line.strip_prefix("export ").unwrap_or(line);
            let (name, value) = line.split_once('=')?;
            let value = value.trim();
            let quoted = |quote: char| {
                value
                    .strip_prefix(quote)
                    .and_then(|inner| inner.split_once(quote))
                    .map(|(inner, _)| inner.to_string())
            };
            let value = quoted('"').or_else(|| quoted('\'')).unwrap_or_else(|| {
                value
                    .split(" #")
                    .next()
                    .unwrap_or_default()
                    .trim()
                    .to_string()
            });
            Some((name.trim().to_string(), value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from(pairs: &'static [(&'static str, &'static str)]) -> impl Fn(&str) -> Option<String> {
        move |name| {
            pairs
                .iter()
                .find(|(n, _)| *n == name)
                .map(|(_, v)| v.to_string())
        }
    }

    #[test]
    fn a_set_value_replaces_the_example() {
        let filled = fill(
            "email = \"${SHOP_EMAIL:-commissions@example.com}\"\n",
            from(&[("SHOP_EMAIL", "me@example.org")]),
        )
        .unwrap();
        assert_eq!(filled.text, "email = \"me@example.org\"\n");
        assert!(filled.examples.is_empty());
    }

    #[test]
    fn an_unset_or_empty_value_shows_the_example() {
        let text = "email = \"${SHOP_EMAIL:-a@example.com}\"";
        for filled in [
            fill(text, from(&[])),
            fill(text, from(&[("SHOP_EMAIL", "")])),
        ] {
            let filled = filled.unwrap();
            assert_eq!(filled.text, "email = \"a@example.com\"");
            assert_eq!(
                filled.examples,
                vec![("SHOP_EMAIL".to_string(), "a@example.com".to_string())]
            );
        }
    }

    #[test]
    fn quotes_and_backslashes_in_a_value_stay_inside_the_string() {
        let filled = fill(
            "note = \"${SHOP_NOTE}\"",
            from(&[("SHOP_NOTE", "say \"hi\" \\o/")]),
        )
        .unwrap();
        assert_eq!(filled.text, r#"note = "say \"hi\" \\o/""#);
    }

    #[test]
    fn mistakes_are_reported() {
        let err = fill("a = \"${HOME}\"", from(&[("HOME", "/root")])).unwrap_err();
        assert!(err.contains("start with SHOP_"), "{err}");
        let err = fill("a = \"${SHOP_X}\"", from(&[])).unwrap_err();
        assert!(err.contains("SHOP_X isn't set"), "{err}");
        let err = fill("a = \"${SHOP_X\"", from(&[])).unwrap_err();
        assert!(err.contains("no closing"), "{err}");
    }

    #[test]
    fn comments_are_left_alone() {
        let text = "# write ${SHOP_X} like this\n";
        assert_eq!(fill(text, from(&[])).unwrap().text, text);
    }

    #[test]
    fn the_sample_shop_fills_in_both_ways() {
        let shop = include_str!("../shop.toml");
        let examples = fill(shop, from(&[])).unwrap();
        assert!(examples.text.contains("discord = \"your.discord.name\""));
        assert!(
            examples
                .text
                .contains("email = \"commissions@example.com\"")
        );
        assert_eq!(examples.examples.len(), 2);
        crate::model::Shop::from_toml(&examples.text).expect("filled-in shop.toml should be valid");

        let real = fill(
            shop,
            from(&[
                ("SHOP_DISCORD", "someone"),
                ("SHOP_EMAIL", "shop@example.net"),
            ]),
        )
        .unwrap();
        assert!(real.text.contains("discord = \"someone\""));
        assert!(real.text.contains("email = \"shop@example.net\""));
        assert!(real.examples.is_empty());
    }

    #[test]
    fn env_files_are_read_like_docker_compose() {
        let vars = read_env_file(
            "# comment\n\nSHOP_A=plain value # note\nexport SHOP_B=\"in # quotes\"\nSHOP_C='single'\nSHOP_D=\nSHOP_A=later\n",
        );
        assert_eq!(
            vars,
            vec![
                ("SHOP_A".to_string(), "plain value".to_string()),
                ("SHOP_B".to_string(), "in # quotes".to_string()),
                ("SHOP_C".to_string(), "single".to_string()),
                ("SHOP_D".to_string(), String::new()),
                ("SHOP_A".to_string(), "later".to_string()),
            ]
        );
    }
}
