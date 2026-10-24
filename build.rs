//! Hands the app's name from Cargo.toml to the code (ADR 0019): `APP_COMMAND` is the package
//! name, `APP_ENV` its environment-variable prefix, `APP_NAME` and `APP_PREVIOUS` come from
//! `[package.metadata.brand]`.

use std::fs;

fn main() {
    println!("cargo:rerun-if-changed=Cargo.toml");
    let command = std::env::var("CARGO_PKG_NAME").expect("cargo sets CARGO_PKG_NAME");
    let manifest = fs::read_to_string("Cargo.toml").expect("Cargo.toml is readable");
    let brand = section(&manifest, "[package.metadata.brand]");
    let name = value(&brand, "name").unwrap_or_else(|| panic!("[package.metadata.brand] needs a name"));
    let previous = value(&brand, "previous").unwrap_or_default();
    let previous: Vec<&str> = previous.trim_matches(['[', ']']).split(',').map(|p| p.trim().trim_matches('"')).filter(|p| !p.is_empty()).collect();
    println!("cargo:rustc-env=APP_NAME={name}");
    println!("cargo:rustc-env=APP_COMMAND={command}");
    println!("cargo:rustc-env=APP_ENV={}", command.to_ascii_uppercase().replace('-', "_"));
    println!("cargo:rustc-env=APP_PREVIOUS={}", previous.join(","));
}

/// The lines of one table, up to the next `[`.
fn section(manifest: &str, header: &str) -> String {
    manifest.lines().skip_while(|l| l.trim() != header).skip(1).take_while(|l| !l.trim_start().starts_with('[')).collect::<Vec<_>>().join("\n")
}

/// A `key = value` in a table, its quotes taken off a plain string.
fn value(table: &str, key: &str) -> Option<String> {
    table.lines().filter(|l| !l.trim_start().starts_with('#')).find_map(|l| {
        let (k, v) = l.split_once('=')?;
        (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
    })
}
