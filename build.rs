//! Checks shop.toml before the site is built, with the same code the site
//! uses to read it. A mistake stops the build and says what to fix.
//!
//! First it fills in shop.toml's `${SHOP_…}` values (see src/vars.rs),
//! looking each name up in the environment, then in `.env`. The site is built
//! from that filled-in copy, written to OUT_DIR, so details such as the
//! Discord name and email can stay out of the code.

#[allow(dead_code)]
#[path = "src/model.rs"]
mod model;
#[path = "src/vars.rs"]
mod vars;

use std::collections::HashMap;

fn main() {
    println!("cargo:rerun-if-changed=shop.toml");
    println!("cargo:rerun-if-changed=src/model.rs");
    println!("cargo:rerun-if-changed=src/vars.rs");
    // Watched even before it exists, so creating it is noticed too.
    println!("cargo:rerun-if-changed=.env");

    let text = match std::fs::read_to_string("shop.toml") {
        Ok(text) => text,
        Err(e) => {
            eprintln!("\nCouldn't read shop.toml: {e}\n");
            std::process::exit(1);
        }
    };

    // On the server, .env isn't copied into the Docker build: compose.yaml
    // passes its SHOP_ values in as build arguments, which arrive here as
    // environment variables. On your computer, .env is read directly.
    let dotenv: HashMap<String, String> = std::fs::read_to_string(".env")
        .map(|text| vars::read_env_file(&text).into_iter().collect())
        .unwrap_or_default();
    let filled = vars::fill(&text, |name| {
        println!("cargo:rerun-if-env-changed={name}");
        std::env::var(name)
            .ok()
            .filter(|value| !value.is_empty())
            .or_else(|| dotenv.get(name).cloned())
    });
    let filled = match filled {
        Ok(filled) => filled,
        Err(problems) => fail(&problems),
    };

    let out = std::path::Path::new(&std::env::var("OUT_DIR").expect("Cargo sets OUT_DIR"))
        .join("shop.toml");
    if let Err(e) = std::fs::write(&out, &filled.text) {
        eprintln!("\nCouldn't write {}: {e}\n", out.display());
        std::process::exit(1);
    }

    match model::Shop::from_toml(&filled.text) {
        Ok(shop) => {
            for (name, example) in &filled.examples {
                let live = if shop.shop.demo {
                    ""
                } else {
                    " (demo = false, so visitors would see it)"
                };
                println!(
                    "cargo:warning=shop.toml: {name} isn't set in .env, so the page shows the example \"{example}\"{live}"
                );
            }
            for warning in shop.warnings() {
                println!("cargo:warning=shop.toml: {warning}");
            }
        }
        Err(problems) => fail(&problems),
    }
}

fn fail(problems: &str) -> ! {
    eprintln!("\nshop.toml needs fixing before the site can be built:\n");
    for line in problems.lines() {
        eprintln!("  {line}");
    }
    eprintln!();
    std::process::exit(1);
}
