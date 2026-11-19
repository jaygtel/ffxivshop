//! Checks shop.toml before the site is built, with the same code the site
//! uses to read it. A mistake stops the build and says what to fix.
//!
//! First it fills in shop.toml's `${SHOP_…}` values (see src/vars.rs),
//! looking each name up in the environment, then in `.env`. The site is built
//! from that filled-in copy, written to OUT_DIR, so details such as the
//! Discord name and email can stay out of the code.
//!
//! A value that isn't set falls back to its example, with a warning. On the
//! live site (demo = false) visitors would see the example, so there it stops
//! the build instead.

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

    let shop = match model::Shop::from_toml(&filled.text) {
        Ok(shop) => shop,
        Err(problems) => fail(&problems),
    };
    if !shop.shop.demo && !filled.examples.is_empty() {
        fail_live(&filled.examples);
    }
    for (name, example) in &filled.examples {
        println!(
            "cargo:warning=shop.toml: {name} isn't set in .env, so the page shows the example \"{example}\""
        );
    }
    for warning in shop.warnings() {
        println!("cargo:warning=shop.toml: {warning}");
    }
}

/// Stops a live build (demo = false) that would show visitors an example in
/// place of a value that isn't set.
fn fail_live(examples: &[(String, String)]) -> ! {
    let (these, them) = if examples.len() == 1 {
        ("this", "it")
    } else {
        ("these", "them")
    };
    eprintln!("\nThe live site (demo = false) needs {these} set in .env:\n");
    for (name, example) in examples {
        eprintln!("  {name}  (or visitors would see the example \"{example}\")");
    }
    eprintln!(
        "\nSet {them} in .env and build again, or set demo = true while the shop is still a sample.\n"
    );
    std::process::exit(1);
}

fn fail(problems: &str) -> ! {
    eprintln!("\nshop.toml needs fixing before the site can be built:\n");
    for line in problems.lines() {
        eprintln!("  {line}");
    }
    eprintln!();
    std::process::exit(1);
}
