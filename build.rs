//! Checks shop.toml before the site is built, with the same code the site
//! uses to read it. A mistake stops the build and says what to fix.

#[allow(dead_code)]
#[path = "src/model.rs"]
mod model;

fn main() {
    println!("cargo:rerun-if-changed=shop.toml");
    println!("cargo:rerun-if-changed=src/model.rs");

    let text = match std::fs::read_to_string("shop.toml") {
        Ok(text) => text,
        Err(e) => {
            eprintln!("\nCouldn't read shop.toml: {e}\n");
            std::process::exit(1);
        }
    };
    match model::Shop::from_toml(&text) {
        Ok(shop) => {
            for warning in shop.warnings() {
                println!("cargo:warning=shop.toml: {warning}");
            }
        }
        Err(problems) => {
            eprintln!("\nshop.toml needs fixing before the site can be built:\n");
            for line in problems.lines() {
                eprintln!("  {line}");
            }
            eprintln!();
            std::process::exit(1);
        }
    }
}
