//! The site's pages, each at its own address: the homepage (the hero) at /,
//! and the shop at /shop. src/app.rs shows the page for the address, and
//! the navigation at the top (components/site_nav.rs) links to each one.
//!
//! The server answers each page's address with the same index.html, then the
//! site shows the right page. So a new page needs its address added to
//! deploy/nginx.conf as well, or the server won't find it: a test below
//! checks that every page is there.

mod home;
mod shop;

pub use home::HomePage;
pub use shop::ShopPage;

/// The homepage's address.
pub const HOME: &str = "/";
/// The shop page's address.
pub const SHOP: &str = "/shop";

/// Every page's name and address, in the order the navigation lists them.
pub const PAGES: [(&str, &str); 2] = [("Home", HOME), ("Shop", SHOP)];

/// The page's own address for `path`: the homepage's for its file name,
/// /index.html (which the site sends on to /), and `path` otherwise.
pub fn page_address(path: &str) -> &str {
    if path == "/index.html" { HOME } else { path }
}

/// The name of the page at `path`, if there's a page there.
pub fn name(path: &str) -> Option<&'static str> {
    PAGES
        .iter()
        .find(|(_, address)| *address == path)
        .map(|(name, _)| *name)
}

/// The browser tab's title on the page at `path`: just the shop's name on
/// the homepage, with the page's name before it on the others.
pub fn title(path: &str, shop_name: &str) -> String {
    if path == HOME {
        return shop_name.to_string();
    }
    let page = name(path).unwrap_or("Page not found");
    format!("{page} – {shop_name}")
}

#[cfg(test)]
mod tests {
    use super::{HOME, PAGES, SHOP, name, page_address, title};

    /// The server's settings, which must answer every page's address.
    const NGINX: &str = include_str!("../../deploy/nginx.conf");

    #[test]
    fn the_server_answers_every_page() {
        for (page, address) in PAGES {
            // The homepage is index.html itself.
            if address == HOME {
                continue;
            }
            assert!(
                NGINX.contains(&format!("location = {address} {{")),
                "deploy/nginx.conf has no `location = {address}` for the {page} page"
            );
        }
    }

    #[test]
    fn each_page_has_its_own_title() {
        assert_eq!(title(HOME, "Aether & Allure"), "Aether & Allure");
        assert_eq!(title(SHOP, "Aether & Allure"), "Shop – Aether & Allure");
        assert_eq!(
            title("/nowhere", "Aether & Allure"),
            "Page not found – Aether & Allure"
        );
        assert_eq!(name(SHOP), Some("Shop"));
        assert_eq!(name("/shop/"), None);
    }

    #[test]
    fn the_file_name_is_the_homepage() {
        assert_eq!(page_address("/index.html"), HOME);
        assert_eq!(page_address(HOME), HOME);
        assert_eq!(page_address(SHOP), SHOP);
        assert_eq!(page_address("/nowhere"), "/nowhere");
    }
}
