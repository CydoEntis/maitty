//! Hands the app's release identity from its Cargo.toml to the code: `APP_COMMAND` and
//! `APP_VERSION` come from `[package]`; `APP_NAME`, `APP_TAGLINE` and `APP_PREVIOUS` come from
//! `[package.metadata.brand]`.

use std::fs;
use std::path::Path;

fn main() {
    configure(Path::new("Cargo.toml"));
}

pub fn configure(manifest_path: &Path) {
    println!("cargo:rerun-if-changed={}", manifest_path.display());
    let manifest = fs::read_to_string(manifest_path).expect("Cargo.toml is readable");
    let package = section(&manifest, "[package]");
    let command = value(&package, "name").expect("[package] needs a name");
    let version = value(&package, "version").expect("[package] needs a version");
    let brand = section(&manifest, "[package.metadata.brand]");
    let name =
        value(&brand, "name").unwrap_or_else(|| panic!("[package.metadata.brand] needs a name"));
    let tagline = value(&brand, "tagline").unwrap_or_default();
    let previous = value(&brand, "previous").unwrap_or_default();
    let previous: Vec<&str> = previous
        .trim_matches(['[', ']'])
        .split(',')
        .map(|p| p.trim().trim_matches('"'))
        .filter(|p| !p.is_empty())
        .collect();
    println!("cargo:rustc-env=APP_NAME={name}");
    println!("cargo:rustc-env=APP_TAGLINE={tagline}");
    println!("cargo:rustc-env=APP_COMMAND={command}");
    println!("cargo:rustc-env=APP_VERSION={version}");
    println!("cargo:rustc-env=APP_PACKAGE={command}");
    println!(
        "cargo:rustc-env=APP_ENV={}",
        command.to_ascii_uppercase().replace('-', "_")
    );
    println!("cargo:rustc-env=APP_PREVIOUS={}", previous.join(","));
}

/// The lines of one table, up to the next `[`.
fn section(manifest: &str, header: &str) -> String {
    manifest
        .lines()
        .skip_while(|l| l.trim() != header)
        .skip(1)
        .take_while(|l| !l.trim_start().starts_with('['))
        .collect::<Vec<_>>()
        .join("\n")
}

/// A `key = value` in a table, its quotes taken off a plain string.
fn value(table: &str, key: &str) -> Option<String> {
    table
        .lines()
        .filter(|l| !l.trim_start().starts_with('#'))
        .find_map(|l| {
            let (k, v) = l.split_once('=')?;
            (k.trim() == key).then(|| v.trim().trim_matches('"').to_string())
        })
}
