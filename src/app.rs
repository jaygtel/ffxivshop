use crate::browser;
use crate::components::{AetherSky, MenuOpen, SiteNav};
use crate::model::Shop;
use crate::order::Order;
use crate::pages::{self, HomePage, ShopPage};
use crate::prices::{self, Prices};
use leptos::prelude::*;
use leptos_router::components::{Redirect, Route, Router, Routes};
use leptos_router::hooks::use_location;
use leptos_router::{NavigateOptions, path};

/// shop.toml is built into the site, with its ${SHOP_…} values filled in from
/// .env by build.rs, which has already checked it.
const SHOP_TOML: &str = include_str!(concat!(env!("OUT_DIR"), "/shop.toml"));

/// `prices`: the market prices from prices.json, if the site could load them.
#[component]
pub fn App(prices: Option<Prices>) -> impl IntoView {
    match Shop::from_toml(SHOP_TOML) {
        Ok(mut shop) => {
            if let Some(prices) = &prices {
                prices::apply(&mut shop, prices);
            }
            // The shop's details never change while the site is open, so they
            // live for the whole visit and every page can borrow them.
            let shop: &'static Shop = Box::leak(Box::new(shop));
            view! { <Site shop /> }.into_any()
        }
        Err(problems) => view! { <SetupProblem problems /> }.into_any(),
    }
}

/// Every page, with the sky behind, and the navigation bar above all but the
/// homepage. Moving between pages doesn't reload the site: the router swaps
/// the page and updates the address, and Back and Forward work as usual.
#[component]
fn Site(shop: &'static Shop) -> impl IntoView {
    // The order lasts the whole visit, whichever page is showing.
    let order = Order::new(shop);
    let menu_open = RwSignal::new(false);
    provide_context(MenuOpen(menu_open));
    // While the menu is open, the page behind it doesn't scroll (main.css).
    Effect::new(move |_| browser::set_root_class("menu-open", menu_open.get()));

    view! {
        <Router>
            <AetherSky />
            <NavBar shop />
            <PageChanges shop />
            // While the menu is open, nothing behind it can be reached.
            <div class="site-content" inert=move || menu_open.get()>
                // The addresses here must match those in src/pages/mod.rs.
                <Routes fallback=|| view! { <NotFound /> }>
                    <Route path=path!("/") view=move || view! { <HomePage shop /> } />
                    // The homepage by its file name, as some links and tools
                    // ask for it, goes to the homepage's own address.
                    <Route
                        path=path!("/index.html")
                        view=|| {
                            view! {
                                <Redirect
                                    path=pages::HOME
                                    options=NavigateOptions {
                                        replace: true,
                                        ..NavigateOptions::default()
                                    }
                                />
                            }
                        }
                    />
                    // The shop page shows itself (below), as it stays in the
                    // site once opened: see src/pages/shop.rs.
                    <Route path=path!("/shop") view=|| () />
                </Routes>
                <ShopPage shop order />
            </div>
        </Router>
    }
}

/// The navigation bar, on every page but the homepage.
#[component]
fn NavBar(shop: &'static Shop) -> impl IntoView {
    let location = use_location();
    let on_home = move || location.pathname.with(|path| path == pages::HOME);
    view! {
        <Show when=move || !on_home()>
            <SiteNav shop />
        </Show>
    }
}

/// Keeps the browser tab's title and the page's canonical address in step
/// with the page showing. After a move to another page, it also closes the
/// menu and moves keyboard focus: back on the homepage to Order Now (the
/// hero shows finished, as its opening only plays when the site is opened
/// there), and elsewhere to the top, as opening a page does.
#[component]
fn PageChanges(shop: &'static Shop) -> impl IntoView {
    let location = use_location();
    let MenuOpen(menu_open) = expect_context::<MenuOpen>();
    Effect::new(move |before: Option<String>| {
        let path = pages::page_address(&location.pathname.get()).to_string();
        if menu_open.get_untracked() {
            menu_open.set(false);
        }
        browser::set_title(&pages::title(&path, &shop.shop.name));
        // Search engines learn each page's own address from this.
        let canonical = shop
            .site
            .address
            .as_deref()
            .filter(|_| pages::name(&path).is_some())
            .map(|address| format!("{}{path}", address.trim_end_matches('/')));
        browser::set_canonical(canonical.as_deref());
        if before.is_some_and(|before| before != path) {
            let home = path == pages::HOME;
            // Once the new page is on screen.
            request_animation_frame(move || {
                if home {
                    browser::focus_first(".hero-cta");
                } else {
                    browser::focus_quietly("top");
                }
            });
        }
        path
    });
}

/// Only seen while working on the site (trunk serve), at an address with no
/// page: on the server, nginx answers those itself.
#[component]
fn NotFound() -> impl IntoView {
    view! {
        <main class="page">
            <section class="window">
                <header class="window-title">
                    <h1>"Page not found"</h1>
                </header>
                <div class="window-body">
                    <p class="lede">"There is no page at this address."</p>
                    <p>
                        <a href=pages::HOME>"Go to the homepage"</a>
                    </p>
                </div>
            </section>
        </main>
    }
}

/// Only seen if shop.toml is broken in a way build.rs couldn't catch.
#[component]
fn SetupProblem(problems: String) -> impl IntoView {
    view! {
        <main class="page">
            <section class="window">
                <header class="window-title">
                    <h1>"shop.toml needs fixing"</h1>
                </header>
                <div class="window-body">
                    <pre class="problems">{problems}</pre>
                </div>
            </section>
        </main>
    }
}
