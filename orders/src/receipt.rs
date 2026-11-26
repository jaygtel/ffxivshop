//! The receipt: the email the service sends to the shop for each order, with
//! Reply-To set to the buyer, so a reply answers them and carries the receipt
//! with it. It's an HTML email made to look like the order slip
//! on the shop page, with a plain-text copy inside the same email for email
//! programs that can't show HTML. How it's built, and what it shares with
//! the contact page's messages (letter.rs), is in look.rs.

use crate::basket::MATS_RULE;
use crate::look::{
    self, BRIGHT, DASHES, DISPLAY, GOLD, GOLD_DEEP, INK, LINE, LINK_LINE, MUTED, OVERLINE, ROW,
    WIDTH, body_text, detail, esc, last_words_together, overline, wrap,
};
use crate::model::{Shop, gil};
use crate::order::{Line, Order};

/// The email, ready to send.
pub struct Receipt {
    pub subject: String,
    pub html: String,
    pub text: String,
}

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

/// Where the order came from, before the website's name: the middle line of
/// the footer.
const SENT_FROM: &str = "Sent from the order panel on";

// ---------------------------------------------------------------------------
// HTML
// ---------------------------------------------------------------------------

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

/// Figures in columns line up, as in the order slip.
const FIGURES: &str = "font-variant-numeric:lining-nums tabular-nums;";

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

fn slip(shop: &Shop, order: &Order, number: &str, placed: &str, assets: &str) -> String {
    let mut sections = vec![details(order, placed), items(shop, order), materials(order)];
    if !order.note.is_empty() {
        sections.push(note(order));
    }
    sections.push(next(shop));
    look::slip(assets, "Order receipt", &esc(number), &sections)
}

/// Where the order came from: the middle line of the footer.
fn sent_from(shop: &Shop) -> String {
    format!("{SENT_FROM} {}.", look::website(shop))
}

fn html(
    shop: &Shop,
    order: &Order,
    number: &str,
    placed: &str,
    assets: &str,
    subject: &str,
) -> String {
    // On phones the two blocks of details go one under the other, the Each
    // column goes, and the price of one goes under the item's name instead.
    let phone_styles = ".col{display:block!important;max-width:100%!important;}\
         .col-2{padding-top:18px!important;}\
         .each{display:none!important;}\
         .each-under{display:block!important;max-height:none!important;overflow:visible!important;}";
    let footer = look::footer(shop, &last_words_together(&esc(&sent_from(shop))));
    look::page(
        assets,
        shop,
        subject,
        &preview(order),
        phone_styles,
        &slip(shop, order, number, placed, assets),
        &footer,
    )
}

// ---------------------------------------------------------------------------
// Plain text
// ---------------------------------------------------------------------------

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

    t.push_str(&look::text_footer(shop, &sent_from(shop)));
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
        // The footer is left-aligned, under the frame's left edge, and on
        // phones lines up with the text inside the slip.
        assert!(html.contains(
            "<td align=\"left\" class=\"foot\" style=\"padding:24px 0 0;text-align:left;\"><div style=\"margin:0 0 8px;"
        ));
        let phones = html
            .split("@media screen and (max-width:540px){")
            .nth(1)
            .and_then(|rest| rest.split("</style>").next())
            .unwrap();
        assert!(
            phones.contains(".foot{padding-left:24px!important;padding-right:24px!important;}")
        );
        // Its paragraphs never end with one word alone on a line.
        assert!(html.contains("Sent from the order panel on&nbsp;aetherandallure.com.</div>"));
        // It goes to the shop only, so it says nothing to a stranger about
        // ignoring it.
        assert!(!html.contains("ignore this"));
        assert!(html.contains("All Rights&nbsp;Reserved.</div>"));
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
    fn its_words_are_plain_english() {
        let receipt = receipt(true, "", &MIXED);
        for words in [receipt.text.as_str(), ASK_NOTE, PAY, SENT_FROM] {
            assert!(!words.contains("n't"), "no negative contractions: {words}");
        }
    }
}
