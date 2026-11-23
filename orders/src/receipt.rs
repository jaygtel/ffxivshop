//! The receipt: the email the service sends for each order, to the shop with
//! a copy to the buyer. It's an HTML email made to look like the order slip
//! on the shop page, with a plain-text copy inside the same email for email
//! programs that can't show HTML.
//!
//! Email programs only understand old-fashioned HTML, so it's built from
//! tables, with every style written on its tag. Many can't draw SVG either,
//! so the emblem, the divider, the diamond and the frame's corner pieces are
//! PNG pictures (assets/img/email, served by the site). Everything else is
//! text and colour, so the receipt still reads properly where an email
//! program holds pictures back. Where it can't load the site's fonts, it
//! uses similar ones.
//!
//! Those pictures are Aether & Allure artwork: all rights reserved, and not
//! covered by the BSD licence (see NOTICE.md).

use crate::basket::MATS_RULE;
use crate::model::{Shop, gil};
use crate::order::{Line, Order};

/// The email, ready to send.
pub struct Receipt {
    pub subject: String,
    pub html: String,
    pub text: String,
}

// The colours are the site's (assets/css/main.css), worked out as they look
// over its night sky, since email programs can't blend see-through colours.
const SKY: &str = "#060a1a"; // the page, and the gap inside the frame
const RING: &str = "#302c27"; // the frame's faint outer ring
const LINE: &str = "#61563e"; // the frame's gold line, and the total's dashes
const GLASS: &str = "#0a1229"; // inside the frame
const RULE: &str = "#554c3b"; // under the title
const ROW: &str = "#2b2c31"; // between the items
const INK: &str = "#e3ecff";
const MUTED: &str = "#a3b3d6";
const BRIGHT: &str = "#eaf4ff";
const GOLD: &str = "#d9b45a";
const GOLD_DEEP: &str = "#9c7228";
const OVERLINE: &str = "#d7cba7";
const DASHES: &str = "#ab9b6e"; // around the materials rule
const AETHER: &str = "#7fd8ff";
const LINK_LINE: &str = "#33576f";

const DISPLAY: &str = "Cinzel,'Trajan Pro',Georgia,'Times New Roman',serif";
const BODY: &str = "'Alegreya Sans','Gill Sans','Segoe UI','Helvetica Neue',Arial,sans-serif";

/// The frame's corner pieces are this many pixels square. It's the gold
/// ring (1), the gap (5), the gold line (1) and the glass inside (17).
const CORNER: u32 = 24;

/// Writes the receipt for `order`. `placed` is when it was sent, written
/// out; `assets` is where the site serves the pictures and fonts (normally
/// its own address).
pub fn write(shop: &Shop, order: &Order, number: &str, placed: &str, assets: &str) -> Receipt {
    let subject = format!("Order {number} from {} ({})", order.character, order.world);
    let html = html(
        shop,
        order,
        number,
        placed,
        assets.trim_end_matches('/'),
        &subject,
    );
    let text = text(shop, order, number, placed);
    Receipt {
        subject,
        html,
        text,
    }
}

// ---------------------------------------------------------------------------
// The words, shared by both copies
// ---------------------------------------------------------------------------

pub fn items_word(count: u32) -> String {
    if count == 1 {
        "1 item".to_string()
    } else {
        format!("{count} items")
    }
}

/// What shows in the inbox beside the subject.
fn preview(order: &Order) -> String {
    let items = items_word(order.count());
    match order.totals.in_words() {
        Some(total) => {
            format!("{items}, {total}. I'll reply to confirm the price and when it'll be ready.")
        }
        None => format!(
            "{items}, with no prices yet. I'll reply to tell you the price and when it'll be ready."
        ),
    }
}

fn ready_words(order: &Order) -> String {
    if order.ready.minutes == 0 {
        "Now: everything is in stock".to_string()
    } else {
        format!("In about {}", order.ready.label().to_lowercase())
    }
}

const PAY: &str = "In gil, when you collect";

fn materials_words(order: &Order) -> &'static str {
    match (order.mats_offered, order.own_mats) {
        (_, true) => "You will bring the materials, so the lower prices apply.",
        (true, false) => "I will use my own materials, at the standard price.",
        (false, false) => "I will use my own materials.",
    }
}

const ASK_NOTE: &str = "Items marked Ask have no price yet. I'll tell you the price when I reply.";

fn next_steps(shop: &Shop) -> [String; 3] {
    let place = &shop.location;
    [
        "I'll reply to confirm the price and when it'll be ready.".to_string(),
        format!(
            "When it's ready, come to my house: {}, {}, on {} ({}).",
            place.district.name(),
            place.ward_and_plot(),
            place.world,
            place.data_center
        ),
        "Pay me the gil with a trade, and I'll hand over your order.".to_string(),
    ]
}

const WHY_YOU_GOT_IT: &str = "You got this email because this address was used to send an order on";
const IF_NOT_YOU: &str = "If you did not send it, you can ignore this email.";
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

// ---------------------------------------------------------------------------
// HTML
// ---------------------------------------------------------------------------

/// Text made safe to put in HTML.
fn esc(text: &str) -> String {
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

/// The gil coin, drawn in CSS: a gold face in a deeper gold rim, centred
/// on the figures beside it (the space under it lifts it from the middle of
/// the small letters to the middle of the figures). Outlook on Windows
/// can't draw it, so it gets a gold dot instead.
fn coin(size: u32) -> String {
    let rim = if size >= 18 { 2 } else { 1 };
    let face = size - 2 * rim;
    let lift = if size >= 18 { 6 } else { 3 };
    format!(
        "<!--[if !mso]><!--><span style=\"display:inline-block;width:{face}px;height:{face}px;\
         border:{rim}px solid {GOLD_DEEP};border-radius:50%;background-color:{GOLD};\
         box-shadow:inset 0 0 0 1px rgba(248,233,187,0.55);vertical-align:middle;\
         margin:0 5px {lift}px 0;font-size:0;line-height:0;\">&nbsp;</span><!--<![endif]-->\
         <!--[if mso]><span style=\"color:{GOLD};\">&#9679;&nbsp;</span><![endif]-->"
    )
}

/// A small heading, set like the shop page's.
fn overline(words: &str) -> String {
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
fn body_text(size: u32, line: u32, colour: &str) -> String {
    format!(
        "font-family:{BODY};font-size:{size}px;line-height:{line}px;color:{colour};\
         font-variant-numeric:lining-nums;mso-line-height-rule:exactly;"
    )
}

/// Figures in columns line up, as in the order slip.
const FIGURES: &str = "font-variant-numeric:lining-nums tabular-nums;";

/// A picture from the site's assets/img/email.
fn picture(assets: &str, file: &str, width: u32, height: u32, extra: &str) -> String {
    format!(
        "<img src=\"{assets}/assets/img/email/{file}\" width=\"{width}\" height=\"{height}\" alt=\"\" \
         style=\"display:block;width:{width}px;height:{height}px;border:0;outline:none;\
         text-decoration:none;{extra}\">"
    )
}

/// A row of "Label  value" in the order's details.
fn detail(label: &str, value_html: &str, label_width: u32) -> String {
    format!(
        "<tr><td width=\"{label_width}\" valign=\"top\" style=\"width:{label_width}px;padding:3px 10px 3px 0;{label_style}\">{label}</td>\
         <td valign=\"top\" style=\"padding:3px 0;{value_style}word-break:break-word;\">{value_html}</td></tr>",
        label = esc(label),
        label_style = body_text(15, 22, MUTED),
        value_style = body_text(15, 22, INK),
    )
}

/// Two blocks side by side, one under the other on a phone.
fn side_by_side(left: &str, right: &str) -> String {
    let column = |content: &str, gap: u32, class: &str| {
        format!(
            "<div class=\"{class}\" style=\"display:inline-block;width:100%;max-width:276px;vertical-align:top;\">\
             <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">\
             <tr><td style=\"padding:0 {gap}px 0 0;\">{content}</td></tr></table></div>"
        )
    };
    format!(
        "<div style=\"font-size:0;line-height:0;\">\
         <!--[if mso]><table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr><td width=\"50%\" valign=\"top\"><![endif]-->\
         {}\
         <!--[if mso]></td><td width=\"50%\" valign=\"top\"><![endif]-->\
         {}\
         <!--[if mso]></td></tr></table><![endif]--></div>",
        column(left, 14, "col"),
        column(right, 0, "col col-2")
    )
}

fn details(order: &Order, placed: &str) -> String {
    let buyer = format!(
        "{}<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">{}{}{}</table>",
        overline("Ordered by"),
        detail("Character", &esc(&order.character), 76),
        detail("World", &esc(&order.world), 76),
        detail(
            "Email",
            &format!(
                "<a href=\"mailto:{0}\" style=\"color:{INK};text-decoration:none;\">{0}</a>",
                esc(&order.email)
            ),
            76
        ),
    );
    let when = format!(
        "{}<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">{}{}{}</table>",
        overline("Order"),
        detail("Placed", &esc(placed), 56),
        detail("Ready", &esc(&ready_words(order)), 56),
        detail("Pay", PAY, 56),
    );
    side_by_side(&buyer, &when)
}

/// A price, or "Ask", after the coin, as the order slip shows them.
fn price(amount: Option<u64>) -> String {
    match amount {
        Some(gil_amount) => format!("{}{}", coin(14), gil(gil_amount)),
        None => format!("{}Ask", coin(14)),
    }
}

fn item_row(shop: &Shop, line: &Line) -> String {
    let item = &shop.items[line.index];
    let cell = |align: &str, extra: &str| {
        format!(
            "align=\"{align}\" valign=\"top\" style=\"border-top:1px solid {ROW};padding:8px 0;{}{FIGURES}white-space:nowrap;{extra}\"",
            body_text(15, 22, INK)
        )
    };
    // On a phone there's no Each column, so the price of one goes under the
    // name instead, when there's more than one.
    let each_under = match line.each {
        Some(each) if line.quantity > 1 => format!(
            "<div class=\"each-under\" style=\"display:none;max-height:0;overflow:hidden;mso-hide:all;{}font-weight:400;\">{}{} each</div>",
            body_text(13, 20, MUTED),
            coin(12),
            gil(each)
        ),
        _ => String::new(),
    };
    format!(
        "<tr><td valign=\"top\" style=\"border-top:1px solid {ROW};padding:8px 10px 8px 0;{name_style}font-weight:500;\">\
         <a href=\"{link}\" style=\"color:{INK};text-decoration:none;border-bottom:1px solid {LINK_LINE};\">{name}</a>{each_under}</td>\
         <td {qty}>{quantity}</td>\
         <td class=\"each\" {each_cell}>{each}</td>\
         <td {amount_cell}>{amount}</td></tr>",
        name_style = body_text(15, 22, INK),
        link = esc(&shop.edb_url(item)),
        name = esc(&item.name),
        qty = cell("right", "padding-left:8px;"),
        quantity = line.quantity,
        each_cell = cell("right", "padding-left:10px;"),
        each = price(line.each),
        amount_cell = cell("right", "padding-left:10px;font-weight:500;"),
        amount = price(line.amount()),
    )
}

fn items(shop: &Shop, order: &Order) -> String {
    let head = |words: &str, align: &str, width: Option<u32>, class: &str| {
        let width = width.map(|w| format!(" width=\"{w}\"")).unwrap_or_default();
        format!(
            "<td{width}{class} align=\"{align}\" style=\"padding:0 0 6px;{}\">{words}</td>",
            body_text(13, 18, MUTED)
        )
    };
    let mut rows = String::new();
    for line in &order.lines {
        rows.push_str(&item_row(shop, line));
    }
    let totals = &order.totals;
    let ask = totals.unpriced > 0;
    let total = if totals.priced == 0 {
        format!("<em style=\"font-size:19px;font-weight:400;color:{MUTED};\">Ask</em>")
    } else {
        let mut total = format!("{}{}", coin(19), gil(totals.gil));
        if ask {
            total.push_str(&format!(
                "<span style=\"font-size:19px;font-weight:400;color:{MUTED};\"> + <em>Ask</em></span>"
            ));
        }
        total
    };
    let ask_note = if ask {
        format!(
            "<div style=\"margin:8px 0 0;{}\">Items marked <em>Ask</em> have no price yet. I'll tell you the price when I reply.</div>",
            body_text(14, 20, MUTED)
        )
    } else {
        String::new()
    };
    format!(
        "{overline}<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">\
         <tr>{h_item}{h_qty}{h_each}{h_amount}</tr>{rows}</table>\
         <table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"margin-top:4px;\"><tr>\
         <td valign=\"bottom\" style=\"border-top:1px dashed {LINE};padding:12px 0 0;font-family:{DISPLAY};font-size:14px;\
         line-height:30px;font-weight:600;letter-spacing:2.2px;text-transform:uppercase;color:{OVERLINE};mso-line-height-rule:exactly;\">Total</td>\
         <td align=\"right\" valign=\"bottom\" style=\"border-top:1px dashed {LINE};padding:12px 0 0;{total_style}{FIGURES}font-weight:500;white-space:nowrap;\">{total}</td>\
         </tr></table>{ask_note}",
        overline = overline("Items"),
        h_item = head("Item", "left", None, ""),
        h_qty = head("Qty", "right", Some(44), ""),
        h_each = head("Each", "right", Some(96), " class=\"each\""),
        h_amount = head("Amount", "right", Some(104), ""),
        total_style = body_text(24, 30, BRIGHT),
    )
}

fn materials(order: &Order) -> String {
    let rule = if order.own_mats {
        format!(
            "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\" style=\"margin-top:8px;\"><tr>\
             <td style=\"padding:6px 9px;border:1px dashed {DASHES};border-radius:4px;{}\">{}</td></tr></table>",
            body_text(14, 20, MUTED),
            esc(MATS_RULE)
        )
    } else {
        String::new()
    };
    format!(
        "{}<div style=\"{}\">{}</div>{rule}",
        overline("Materials"),
        body_text(15, 22, INK),
        esc(materials_words(order))
    )
}

fn note(order: &Order) -> String {
    format!(
        "{}<div style=\"{}word-break:break-word;\">{}</div>",
        overline("Anything else?"),
        body_text(15, 22, INK),
        esc(&order.note)
    )
}

fn next(shop: &Shop) -> String {
    let mut rows = String::new();
    for (n, step) in next_steps(shop).iter().enumerate() {
        rows.push_str(&format!(
            "<tr><td width=\"22\" valign=\"top\" style=\"width:22px;padding:2px 0;font-family:{DISPLAY};font-size:15px;line-height:22px;\
             font-weight:600;color:{GOLD};mso-line-height-rule:exactly;\">{}</td>\
             <td valign=\"top\" style=\"padding:2px 0;{}\">{}</td></tr>",
            n + 1,
            body_text(15, 22, INK),
            esc(step)
        ));
    }
    format!(
        "{}<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\">{rows}</table>",
        overline("What happens next")
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

/// The slip's frame, like the shop page's windows: a faint gold ring, a dark
/// gap and a gold line around the glass, with a gold corner piece in each
/// corner. The corners are pictures; the edges between them are table cells
/// of solid colour, so they stretch with the receipt.
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

fn slip(shop: &Shop, order: &Order, number: &str, placed: &str, assets: &str) -> String {
    let title = format!(
        "<table role=\"presentation\" width=\"100%\" cellpadding=\"0\" cellspacing=\"0\" border=\"0\"><tr>\
         <td width=\"22\" valign=\"middle\" style=\"width:22px;padding:0 9px 0 0;\">{diamond}</td>\
         <td valign=\"middle\" style=\"font-family:{DISPLAY};font-size:20px;line-height:26px;font-weight:600;letter-spacing:1px;\
         color:{GOLD};mso-line-height-rule:exactly;\">Order receipt</td>\
         <td align=\"right\" valign=\"middle\" style=\"{number_style}white-space:nowrap;\">{number}</td></tr></table>",
        diamond = picture(assets, "diamond.png", 22, 22, ""),
        number_style = body_text(14, 20, MUTED),
        number = esc(number),
    );
    let mut sections = vec![details(order, placed), items(shop, order), materials(order)];
    if !order.note.is_empty() {
        sections.push(note(order));
    }
    sections.push(next(shop));
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

fn footer(shop: &Shop) -> String {
    let address = shop.site.address.clone().unwrap_or_default();
    let label = site_label(&address);
    let link = if address.is_empty() {
        String::new()
    } else {
        format!(
            " &middot; <a href=\"{}\" style=\"color:{AETHER};text-decoration:none;\">{}</a>",
            esc(&address),
            esc(&label)
        )
    };
    let style = body_text(13, 20, MUTED);
    format!(
        "<div style=\"margin:0 0 8px;{style}\">{name}{link}</div>\
         <div style=\"margin:0 0 8px;{style}\">{why} {on}. {if_not}</div>\
         <div style=\"margin:0;{style}\">{copyright} {square_enix}</div>",
        name = esc(&shop.shop.name),
        why = WHY_YOU_GOT_IT,
        on = esc(if label.is_empty() {
            "the shop's website"
        } else {
            &label
        }),
        if_not = IF_NOT_YOU,
        copyright = esc(&copyright(shop)),
        square_enix = esc(SQUARE_ENIX),
    )
}

fn html(
    shop: &Shop,
    order: &Order,
    number: &str,
    placed: &str,
    assets: &str,
    subject: &str,
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
    let styles = format!(
        ":root{{color-scheme:dark;supported-color-schemes:dark;}}\
         body{{margin:0!important;padding:0!important;width:100%!important;background-color:{SKY};}}\
         table,td{{mso-table-lspace:0pt;mso-table-rspace:0pt;}}\
         img{{-ms-interpolation-mode:bicubic;}}\
         a{{color:{AETHER};}}\
         @media screen and (max-width:540px){{\
         .outer{{padding:18px 6px 26px!important;}}\
         .col{{display:block!important;max-width:100%!important;}}\
         .col-2{{padding-top:18px!important;}}\
         .each{{display:none!important;}}\
         .each-under{{display:block!important;max-height:none!important;overflow:visible!important;}}\
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
<tr><td align=\"center\" style=\"padding:24px 8px 0;\">{footer}</td></tr>\n\
</table>\n\
<!--[if mso]></td></tr></table><![endif]-->\n\
</td></tr>\n\
</table>\n\
</body>\n\
</html>\n",
        title = esc(subject),
        preview = esc(&preview(order)),
        // Stops some email programs filling the preview with the receipt's
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
        slip = slip(shop, order, number, placed, assets),
        footer = footer(shop),
    )
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

/// Wraps `words` to `width` columns, starting each line with `indent`, and
/// the lines after the first with `hang` as well.
fn wrap(words: &str, width: usize, indent: &str, hang: &str) -> String {
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

const WIDTH: usize = 72;

fn text(shop: &Shop, order: &Order, number: &str, placed: &str) -> String {
    let mut t = String::new();
    let rule = format!("{}\n", "-".repeat(WIDTH));
    t.push_str(&format!("{}\n", shop.shop.name.to_uppercase()));
    t.push_str(&format!("Order receipt {number}\n\n"));

    t.push_str("ORDERED BY\n");
    t.push_str(&format!("  Character  {}\n", order.character));
    t.push_str(&format!("  World      {}\n", order.world));
    t.push_str(&format!("  Email      {}\n\n", order.email));
    t.push_str("ORDER\n");
    t.push_str(&format!("  Placed     {placed}\n"));
    t.push_str(&format!("  Ready      {}\n", ready_words(order)));
    t.push_str(&format!("  Pay        {PAY}\n\n"));

    t.push_str("ITEMS\n");
    t.push_str(&rule);
    for line in &order.lines {
        let item = &shop.items[line.index];
        t.push_str(&wrap(&item.name, WIDTH, "", "  "));
        let sum = match (line.each, line.amount()) {
            (Some(each), Some(amount)) => format!(
                "  {} \u{d7} {} gil = {} gil\n",
                line.quantity,
                gil(each),
                gil(amount)
            ),
            _ => format!("  {} \u{d7} Ask (no price yet)\n", line.quantity),
        };
        t.push_str(&sum);
        t.push_str(&format!("  {}\n", shop.edb_url(item)));
    }
    t.push_str(&rule);
    let total = match order.totals.in_words() {
        Some(total) => total,
        None => "Ask (no prices yet)".to_string(),
    };
    t.push_str(&format!("TOTAL  {total}\n"));
    if order.totals.unpriced > 0 {
        t.push('\n');
        t.push_str(&wrap(ASK_NOTE, WIDTH, "", ""));
    }

    t.push_str("\nMATERIALS\n");
    t.push_str(&wrap(materials_words(order), WIDTH, "  ", ""));
    if order.own_mats {
        t.push_str(&wrap(MATS_RULE, WIDTH, "  ", ""));
    }
    if !order.note.is_empty() {
        t.push_str("\nANYTHING ELSE?\n");
        t.push_str(&wrap(&order.note, WIDTH, "  ", ""));
    }
    t.push_str("\nWHAT HAPPENS NEXT\n");
    for (n, step) in next_steps(shop).iter().enumerate() {
        t.push_str(&wrap(&format!("{}. {step}", n + 1), WIDTH, "  ", "   "));
    }

    t.push_str("\n-- \n");
    let address = shop.site.address.clone().unwrap_or_default();
    if address.is_empty() {
        t.push_str(&format!("{}\n", shop.shop.name));
    } else {
        t.push_str(&format!("{} \u{b7} {address}\n", shop.shop.name));
    }
    let label = site_label(&address);
    t.push_str(&wrap(
        &format!(
            "{WHY_YOU_GOT_IT} {}. {IF_NOT_YOU}",
            if label.is_empty() {
                "the shop's website"
            } else {
                &label
            }
        ),
        WIDTH,
        "",
        "",
    ));
    t.push_str(&wrap(&copyright(shop), WIDTH, "", ""));
    t.push_str(&wrap(SQUARE_ENIX, WIDTH, "", ""));
    t
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::basket::{EmailOrder, Ordered};
    use crate::order::check;

    fn sample(own_mats: bool, note: &str, items: &[(&str, u32)]) -> (Shop, Order) {
        let shop = Shop::from_toml(include_str!("../../shop.toml")).unwrap();
        let sent = EmailOrder {
            items: items
                .iter()
                .map(|(name, quantity)| Ordered {
                    name: name.to_string(),
                    quantity: *quantity,
                })
                .collect(),
            own_mats,
            note: note.into(),
            character: "Mira Ashwood".into(),
            world: "Cerberus".into(),
            email: "mira@example.com".into(),
            website: String::new(),
        };
        let order = check(&shop, &sent).unwrap();
        (shop, order)
    }

    fn receipt(own_mats: bool, note: &str, items: &[(&str, u32)]) -> Receipt {
        let (shop, order) = sample(own_mats, note, items);
        write(
            &shop,
            &order,
            "AA-2611-7K3Q",
            "23 Nov 2026, 12:41 UK time",
            "https://aetherandallure.com/",
        )
    }

    const MIXED: [(&str, u32); 3] = [
        ("Courtly Lover's Rapier", 1),
        ("Grade 4 Gemdraught of Strength", 10),
        ("Semicircular Outdoor Stage", 1),
    ];

    #[test]
    fn the_subject_names_the_order_and_the_buyer() {
        let receipt = receipt(false, "", &MIXED);
        assert_eq!(
            receipt.subject,
            "Order AA-2611-7K3Q from Mira Ashwood (Cerberus)"
        );
    }

    #[test]
    fn the_html_holds_the_whole_order() {
        let html = receipt(true, "Jet Black <b>please</b> & thanks", &MIXED).html;
        for part in [
            "Order receipt",
            "AA-2611-7K3Q",
            "Mira Ashwood",
            "Cerberus",
            "mailto:mira@example.com",
            "23 Nov 2026, 12:41 UK time",
            "In about 1 week",
            "In gil, when you collect",
            "Courtly Lover&#39;s Rapier",
            "https://eu.finalfantasyxiv.com/lodestone/playguide/db/item/6246afd2ba5/",
            ">10<",
            "90,000",
            "4,500",
            "45,000",
            "Ask",
            "Items marked <em>Ask</em> have no price yet.",
            "You will bring the materials, so the lower prices apply.",
            "You must bring all the materials, not just some.",
            // The buyer's words can't add their own HTML.
            "Jet Black &lt;b&gt;please&lt;/b&gt; &amp; thanks",
            "Shirogane, Ward 29, Plot 3, on Sagittarius (Chaos).",
            "Website design \u{a9} 2026",
            "FINAL FANTASY XIV \u{a9} SQUARE ENIX",
            "https://aetherandallure.com/assets/img/email/corner-tl.png",
            "https://aetherandallure.com/assets/fonts/cinzel.woff2",
        ] {
            assert!(html.contains(part), "missing {part:?}");
        }
        assert!(!html.contains("<b>please"));
        assert!(!html.contains(".com//assets"));
        // Gmail cuts off emails over about 102 KB.
        assert!(html.len() < 60_000, "{} bytes", html.len());
    }

    #[test]
    fn the_total_never_shows_a_free_order() {
        let html = receipt(false, "", &[("Semicircular Outdoor Stage", 1)]).html;
        assert!(html.contains(">Ask</em>"));
        assert!(!html.contains(">0<"));
        let text = receipt(false, "", &[("Semicircular Outdoor Stage", 1)]).text;
        assert!(text.contains("TOTAL  Ask (no prices yet)"));
    }

    #[test]
    fn the_plain_text_says_the_same() {
        let text = receipt(true, "Jet Black", &MIXED).text;
        for part in [
            "AETHER & ALLURE\nOrder receipt AA-2611-7K3Q\n",
            "  Character  Mira Ashwood\n",
            "  World      Cerberus\n",
            "  Email      mira@example.com\n",
            "  Placed     23 Nov 2026, 12:41 UK time\n",
            "Courtly Lover's Rapier\n  1 \u{d7} 90,000 gil = 90,000 gil\n",
            "Grade 4 Gemdraught of Strength\n  10 \u{d7} 4,500 gil = 45,000 gil\n",
            "Semicircular Outdoor Stage\n  1 \u{d7} Ask (no price yet)\n",
            "TOTAL  ",
            "plus the items with no price yet",
            "MATERIALS\n  You will bring the materials",
            "ANYTHING ELSE?\n  Jet Black\n",
            "  1. I'll reply to confirm the price",
            "\n-- \n",
        ] {
            assert!(text.contains(part), "missing {part:?}");
        }
        for line in text.lines() {
            let long_link = line.trim_start().starts_with("https://");
            assert!(
                line.chars().count() <= WIDTH || long_link,
                "too long: {line:?}"
            );
        }
    }

    #[test]
    fn materials_are_described_either_way() {
        let (_, mine) = sample(false, "", &MIXED);
        assert_eq!(
            materials_words(&mine),
            "I will use my own materials, at the standard price."
        );
        let (_, no_option) = sample(true, "", &[("Gold Thumb's Hatchet", 1)]);
        assert_eq!(materials_words(&no_option), "I will use my own materials.");
        let html = receipt(false, "", &MIXED).html;
        assert!(!html.contains("You must bring all the materials"));
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
    fn its_words_are_plain_english() {
        let receipt = receipt(true, "", &MIXED);
        for words in [
            receipt.text.as_str(),
            ASK_NOTE,
            PAY,
            WHY_YOU_GOT_IT,
            IF_NOT_YOU,
        ] {
            assert!(!words.contains("n't"), "no negative contractions: {words}");
        }
    }
}
