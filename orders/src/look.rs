//! How the emails the service sends look: the order receipt (receipt.rs)
//! and the contact page's messages (letter.rs). Each is an HTML email made
//! to look like a window on the shop page, with a plain-text copy inside the
//! same email for email programs that can't show HTML. This holds what they
//! share: the colours, the fonts, the frame, the page around it and the
//! footer.
//!
//! Email programs only understand old-fashioned HTML, so the emails are
//! built from tables, with every style written on its tag. Many can't draw
//! SVG either, so the emblem, the divider, the diamond and the frame's
//! corner pieces are PNG pictures (assets/img/email, served by the site).
//! Everything else is text and colour, so the emails still read properly
//! where an email program holds pictures back. Where it can't load the
//! site's fonts, it uses similar ones.
//!
//! Those pictures are Aether & Allure artwork: all rights reserved, and not
//! covered by the BSD licence (see docs/NOTICE.md).

use crate::model::Shop;

// The colours are the site's (assets/css/main.css), worked out as they look
// over its night sky, since email programs can't blend see-through colours.
pub const SKY: &str = "#060a1a"; // the page, and the gap inside the frame
pub const RING: &str = "#302c27"; // the frame's faint outer ring
pub const LINE: &str = "#61563e"; // the frame's gold line, and the total's dashes
pub const GLASS: &str = "#0a1229"; // inside the frame
pub const RULE: &str = "#554c3b"; // under the title
pub const ROW: &str = "#2b2c31"; // between the items
pub const INK: &str = "#e3ecff";
pub const MUTED: &str = "#a3b3d6";
pub const BRIGHT: &str = "#eaf4ff";
pub const GOLD: &str = "#d9b45a";
pub const GOLD_DEEP: &str = "#9c7228";
pub const OVERLINE: &str = "#d7cba7";
pub const DASHES: &str = "#ab9b6e"; // around the materials rule
pub const AETHER: &str = "#7fd8ff";
pub const LINK_LINE: &str = "#33576f";

pub const DISPLAY: &str = "Cinzel,'Trajan Pro',Georgia,'Times New Roman',serif";
pub const BODY: &str = "'Alegreya Sans','Gill Sans','Segoe UI','Helvetica Neue',Arial,sans-serif";

/// The frame's corner pieces are this many pixels square. It's the gold
/// ring (1), the gap (5), the gold line (1) and the glass inside (17).
pub const CORNER: u32 = 24;

const SQUARE_ENIX: &str = "FINAL FANTASY XIV \u{a9} SQUARE ENIX CO., LTD. All Rights Reserved.";

fn this_year() -> u16 {
    crate::clock::UkTime::at(crate::clock::now()).year as u16
}

fn copyright(shop: &Shop) -> String {
    format!(
        "Website design {}. All rights reserved.",
        shop.copyright_notice(this_year())
    )
}

/// "aetherandallure.com"
fn site_label(address: &str) -> String {
    address
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/')
        .to_string()
}

/// The shop's website, as the footer names it: "aetherandallure.com".
pub fn website(shop: &Shop) -> String {
    let label = site_label(shop.site.address.as_deref().unwrap_or_default());
    if label.is_empty() {
        "the shop's website".to_string()
    } else {
        label
    }
}

// ---------------------------------------------------------------------------
// HTML
// ---------------------------------------------------------------------------

/// Text made safe to put in HTML.
pub fn esc(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    for c in text.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#39;"),
            c => out.push(c),
        }
    }
    out
}

/// `html` with its last two words joined by a no-break space, so a narrow
/// screen never leaves the last word on a line of its own.
pub fn last_words_together(html: &str) -> String {
    match html.rfind(' ') {
        Some(at) => format!("{}&nbsp;{}", &html[..at], &html[at + 1..]),
        None => html.to_string(),
    }
}

/// A small heading, set like the shop page's.
pub fn overline(words: &str) -> String {
    format!(
        "<div style=\"margin:0 0 7px;font-family:{DISPLAY};font-size:13px;line-height:18px;\
         font-weight:600;letter-spacing:2px;text-transform:uppercase;color:{OVERLINE};\
         mso-line-height-rule:exactly;\">{}</div>",
        esc(words)
    )
}

/// The style of ordinary text, which every cell repeats, as Outlook doesn't
/// pass it down. Figures stand on the line, as on the shop page (Alegreya
/// Sans's own figures dip below it).
pub fn body_text(size: u32, line: u32, colour: &str) -> String {
    format!(
        "font-family:{BODY};font-size:{size}px;line-height:{line}px;color:{colour};\
         font-variant-numeric:lining-nums;mso-line-height-rule:exactly;"
    )
}

/// A picture from the site's assets/img/email.
pub fn picture(assets: &str, file: &str, width: u32, height: u32, extra: &str) -> String {
    format!(
        "<img src=\"{assets}/assets/img/email/{file}\" width=\"{width}\" height=\"{height}\" alt=\"\" \
         style=\"display:block;width:{width}px;height:{height}px;border:0;outline:none;\
         text-decoration:none;{extra}\">"
    )
}

/// A row of "Label  value", in a table of details.
pub fn detail(label: &str, value_html: &str, label_width: u32) -> String {
    format!(
        "<tr><td width=\"{label_width}\" valign=\"top\" style=\"width:{label_width}px;padding:3px 10px 3px 0;{label_style}\">{label}</td>\
         <td valign=\"top\" style=\"padding:3px 0;{value_style}word-break:break-word;\">{value_html}</td></tr>",
        label = esc(label),
        label_style = body_text(15, 22, MUTED),
        value_style = body_text(15, 22, INK),
    )
}

/// A cell of solid colour, part of the frame.
fn band(colour: &str, width: Option<u32>, height: Option<u32>) -> String {
    let mut size = String::new();
    let mut style = format!(
        "background-color:{colour};font-size:0;line-height:0;mso-line-height-rule:exactly;"
    );
    if let Some(w) = width {
        size.push_str(&format!(" width=\"{w}\""));
        style.push_str(&format!("width:{w}px;"));
    }
    if let Some(h) = height {
        size.push_str(&format!(" height=\"{h}\""));
        style = format!(
            "background-color:{colour};height:{h}px;font-size:1px;line-height:1px;mso-line-height-rule:exactly;"
        );
    }
    format!("<td{size} bgcolor=\"{colour}\" style=\"{style}\">&nbsp;</td>")
}

/// The frame, like the shop page's windows: a faint gold ring, a dark gap and
/// a gold line around the glass, with a gold corner piece in each corner.
/// The corners are pictures; the edges between them are table cells of
/// solid colour, so they stretch with the email.
fn frame(assets: &str, inside: &str) -> String {
    let corner = |file: &str, valign: &str| {
        format!(
            "<td colspan=\"4\" rowspan=\"4\" width=\"{CORNER}\" valign=\"{valign}\" bgcolor=\"{SKY}\" \
             style=\"width:{CORNER}px;background-color:{SKY};font-size:0;line-height:0;\">{}</td>",
            picture(assets, file, CORNER, CORNER, "")
        )
    };
    let side = |inner_first: bool| {
        let cells = [
            band(RING, Some(1), None),
            band(SKY, Some(5), None),
            band(LINE, Some(1), None),
            band(GLASS, Some(17), None),
        ];
        if inner_first {
            cells.iter().rev().cloned().collect::<Vec<_>>().concat()
        } else {
            cells.concat()
        }
    };
    format!(
        "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"border-collapse:collapse;\">\
         <tr>{tl}{ring}{tr}</tr><tr>{gap}</tr><tr>{line}</tr><tr>{glass}</tr>\
         <tr>{left}<td valign=\"top\" bgcolor=\"{GLASS}\" style=\"background-color:{GLASS};\">{inside}</td>{right}</tr>\
         <tr>{bl}{glass}{br}</tr><tr>{line}</tr><tr>{gap}</tr><tr>{ring}</tr></table>",
        tl = corner("corner-tl.png", "top"),
        tr = corner("corner-tr.png", "top"),
        bl = corner("corner-bl.png", "bottom"),
        br = corner("corner-br.png", "bottom"),
        ring = band(RING, None, Some(1)),
        gap = band(SKY, None, Some(5)),
        line = band(LINE, None, Some(1)),
        glass = band(GLASS, None, Some(17)),
        left = side(false),
        right = side(true),
    )
}

/// The framed slip, like a window on the shop page: the diamond and `title`
/// along the top, with `aside` (already HTML) on the right, then each of
/// `sections` (already HTML) under it.
pub fn slip(assets: &str, title: &str, aside: &str, sections: &[String]) -> String {
    let title = format!(
        "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>\
         <td width=\"22\" valign=\"middle\" style=\"width:22px;padding:0 9px 0 0;\">{diamond}</td>\
         <td valign=\"middle\" style=\"font-family:{DISPLAY};font-size:20px;line-height:26px;font-weight:600;letter-spacing:1px;\
         color:{GOLD};mso-line-height-rule:exactly;\">{title}</td>\
         <td align=\"right\" valign=\"middle\" style=\"{aside_style}white-space:nowrap;\">{aside}</td></tr></table>",
        diamond = picture(assets, "diamond.png", 22, 22, ""),
        title = esc(title),
        aside_style = body_text(14, 20, MUTED),
    );
    let body: String = sections
        .iter()
        .enumerate()
        .map(|(n, section)| {
            let top = if n == 0 { 16 } else { 22 };
            format!("<tr><td style=\"padding:{top}px 0 0;\">{section}</td></tr>")
        })
        .collect();
    let inside = format!(
        "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">\
         <tr><td style=\"padding:0 0 10px;border-bottom:1px solid {RULE};\">{title}</td></tr>{body}</table>"
    );
    frame(assets, &inside)
}

/// The footer under the slip: the shop's name and website, `why` (already
/// HTML), then the copyright notices.
pub fn footer(shop: &Shop, why: &str) -> String {
    let address = shop.site.address.clone().unwrap_or_default();
    let link = if address.is_empty() {
        String::new()
    } else {
        format!(
            " &middot; <a href=\"{}\" style=\"color:{AETHER};text-decoration:none;\">{}</a>",
            esc(&address),
            esc(&site_label(&address))
        )
    };
    // Left-aligned, starting where the slip's frame starts. On phones it
    // starts where the slip's text starts (see `.foot` in the styles).
    let style = format!("{}text-align:left;", body_text(13, 20, MUTED));
    format!(
        "<div style=\"margin:0 0 8px;{style}\">{name}{link}</div>\
         <div style=\"margin:0 0 8px;{style}\">{why}</div>\
         <div style=\"margin:0;{style}\">{copyright} {square_enix}</div>",
        name = esc(&shop.shop.name),
        copyright = esc(&copyright(shop)),
        square_enix = last_words_together(&esc(SQUARE_ENIX)),
    )
}

/// The whole email around the slip: the emblem, the shop's name and the
/// divider over it, and the footer under it, on the night sky. `title` is
/// the email's subject; `preview` is what shows in the inbox beside it.
/// `phone_styles` are the email's own styles for phones.
pub fn page(
    assets: &str,
    shop: &Shop,
    title: &str,
    preview: &str,
    phone_styles: &str,
    slip: &str,
    footer: &str,
) -> String {
    let fonts = format!(
        "@font-face{{font-family:'Cinzel';font-style:normal;font-weight:400 700;src:url('{assets}/assets/fonts/cinzel.woff2') format('woff2');}}\
         @font-face{{font-family:'Alegreya Sans';font-style:normal;font-weight:400;src:url('{assets}/assets/fonts/alegreya-sans-latin-400-normal.woff2') format('woff2');}}\
         @font-face{{font-family:'Alegreya Sans';font-style:italic;font-weight:400;src:url('{assets}/assets/fonts/alegreya-sans-latin-400-italic.woff2') format('woff2');}}\
         @font-face{{font-family:'Alegreya Sans';font-style:normal;font-weight:500;src:url('{assets}/assets/fonts/alegreya-sans-latin-500-normal.woff2') format('woff2');}}\
         @font-face{{font-family:'Alegreya Sans';font-style:normal;font-weight:700;src:url('{assets}/assets/fonts/alegreya-sans-latin-700-normal.woff2') format('woff2');}}"
    );
    // Separate from the fonts: an email program that throws away a style
    // block it doesn't like keeps the other.
    //
    // On phones the page's edge comes in to 6px, to give the slip more room.
    // The footer has no frame around it, so there it moves in by the frame's
    // width on both sides, lining up with the text inside the slip.
    let styles = format!(
        ":root{{color-scheme:dark;supported-color-schemes:dark;}}\
         body{{margin:0!important;padding:0!important;width:100%!important;background-color:{SKY};}}\
         table,td{{mso-table-lspace:0pt;mso-table-rspace:0pt;}}\
         img{{-ms-interpolation-mode:bicubic;}}\
         a{{color:{AETHER};}}\
         @media screen and (max-width:540px){{\
         .outer{{padding:18px 6px 26px!important;}}\
         .foot{{padding-left:{CORNER}px!important;padding-right:{CORNER}px!important;}}\
         {phone_styles}\
         .name{{font-size:22px!important;line-height:28px!important;}}\
         .divider img{{width:240px!important;height:auto!important;}}\
         }}"
    );
    format!(
        "<!DOCTYPE html>\n\
<html lang=\"en\" xmlns=\"http://www.w3.org/1999/xhtml\" xmlns:v=\"urn:schemas-microsoft-com:vml\" xmlns:o=\"urn:schemas-microsoft-com:office:office\">\n\
<head>\n\
<meta charset=\"utf-8\">\n\
<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n\
<meta name=\"x-apple-disable-message-reformatting\">\n\
<meta name=\"format-detection\" content=\"telephone=no, date=no, address=no, email=no, url=no\">\n\
<meta name=\"color-scheme\" content=\"dark\">\n\
<meta name=\"supported-color-schemes\" content=\"dark\">\n\
<title>{title}</title>\n\
<!--[if mso]><noscript><xml><o:OfficeDocumentSettings><o:PixelsPerInch>96</o:PixelsPerInch></o:OfficeDocumentSettings></xml></noscript><![endif]-->\n\
<style>{fonts}</style>\n\
<style>{styles}</style>\n\
</head>\n\
<body style=\"margin:0;padding:0;background-color:{SKY};\" bgcolor=\"{SKY}\">\n\
<div style=\"display:none;font-size:1px;line-height:1px;max-height:0;max-width:0;opacity:0;overflow:hidden;mso-hide:all;\">{preview}{spacer}</div>\n\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" bgcolor=\"{SKY}\" style=\"background-color:{SKY};\">\n\
<tr><td align=\"center\" class=\"outer\" style=\"padding:30px 14px 34px;\">\n\
<!--[if mso]><table role=\"presentation\" width=\"600\" align=\"center\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr><td><![endif]-->\n\
<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"max-width:600px;margin:0 auto;\">\n\
<tr><td align=\"center\" style=\"padding:0 0 10px;\">{emblem}</td></tr>\n\
<tr><td align=\"center\" class=\"name\" style=\"font-family:{DISPLAY};font-size:26px;line-height:32px;font-weight:600;letter-spacing:1.5px;color:{GOLD};mso-line-height-rule:exactly;\">{shop_name}</td></tr>\n\
<tr><td align=\"center\" class=\"divider\" style=\"padding:12px 0 22px;\">{divider}</td></tr>\n\
<tr><td>{slip}</td></tr>\n\
<tr><td align=\"left\" class=\"foot\" style=\"padding:24px 0 0;text-align:left;\">{footer}</td></tr>\n\
</table>\n\
<!--[if mso]></td></tr></table><![endif]-->\n\
</td></tr>\n\
</table>\n\
</body>\n\
</html>\n",
        title = esc(title),
        preview = esc(preview),
        // Stops some email programs filling the preview with the email's
        // first words after the real preview.
        spacer = "&#847;&zwnj;&nbsp;".repeat(30),
        emblem = picture(assets, "emblem.png", 64, 64, "margin:0 auto;"),
        shop_name = esc(&shop.shop.name),
        divider = picture(
            assets,
            "divider.png",
            300,
            22,
            "margin:0 auto;max-width:100%;"
        ),
    )
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

/// How wide the plain-text copies are.
pub const WIDTH: usize = 72;

/// Wraps `words` to `width` columns, starting each line with `indent`, and
/// the lines after the first with `hang` as well.
pub fn wrap(words: &str, width: usize, indent: &str, hang: &str) -> String {
    let mut out = String::new();
    let mut line = indent.to_string();
    let mut empty = true;
    for word in words.split_whitespace() {
        if !empty && line.chars().count() + 1 + word.chars().count() > width {
            out.push_str(line.trim_end());
            out.push('\n');
            line = format!("{indent}{hang}");
            empty = true;
        }
        if !empty {
            line.push(' ');
        }
        line.push_str(word);
        empty = false;
    }
    out.push_str(line.trim_end());
    out.push('\n');
    out
}

/// The plain-text footer: a signature line, the shop's name and website,
/// `why`, then the copyright notices.
pub fn text_footer(shop: &Shop, why: &str) -> String {
    let mut t = String::from("\n-- \n");
    let address = shop.site.address.clone().unwrap_or_default();
    if address.is_empty() {
        t.push_str(&format!("{}\n", shop.shop.name));
    } else {
        t.push_str(&format!("{} \u{b7} {address}\n", shop.shop.name));
    }
    t.push_str(&wrap(why, WIDTH, "", ""));
    t.push_str(&wrap(&copyright(shop), WIDTH, "", ""));
    t.push_str(&wrap(SQUARE_ENIX, WIDTH, "", ""));
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shop() -> Shop {
        Shop::from_toml(include_str!("../../shop.toml")).unwrap()
    }

    #[test]
    fn the_last_two_words_stay_together() {
        assert_eq!(
            last_words_together("ignore this email."),
            "ignore this&nbsp;email."
        );
        assert_eq!(last_words_together("Reserved."), "Reserved.");
    }

    #[test]
    fn wrapping_keeps_words_whole() {
        assert_eq!(
            wrap("one two three four", 9, "  ", "   "),
            "  one two\n     three\n     four\n"
        );
        assert_eq!(wrap("", 10, "", ""), "\n");
    }

    #[test]
    fn text_is_made_safe_for_html() {
        assert_eq!(
            esc("<b>Mira's</b> & \"friends\""),
            "&lt;b&gt;Mira&#39;s&lt;/b&gt; &amp; &quot;friends&quot;"
        );
    }

    #[test]
    fn the_footer_names_the_website() {
        let mut shop = shop();
        assert_eq!(website(&shop), "aetherandallure.com");
        let text = text_footer(&shop, "Why you got it.");
        assert!(text.starts_with("\n-- \nAether & Allure \u{b7} https://aetherandallure.com\nWhy you got it.\nWebsite design \u{a9} 2026"));
        assert!(text.ends_with("SQUARE ENIX CO., LTD. All Rights Reserved.\n"));
        shop.site.address = None;
        assert_eq!(website(&shop), "the shop's website");
        assert!(text_footer(&shop, "Why.").starts_with("\n-- \nAether & Allure\nWhy.\n"));
        assert!(!footer(&shop, "Why.").contains("&middot;"));
    }
}
