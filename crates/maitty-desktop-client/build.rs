use std::fs;
use std::path::PathBuf;

fn main() {
    let manifest_dir = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    let workspace_manifest = manifest_dir.join("../../apps/tui/Cargo.toml");
    println!("cargo:rerun-if-changed={}", workspace_manifest.display());
    let manifest =
        fs::read_to_string(workspace_manifest).expect("TUI package manifest is readable");
    let package = manifest
        .split("[package]")
        .nth(1)
        .expect("workspace package exists")
        .lines()
        .take_while(|line| !line.trim_start().starts_with('['))
        .collect::<Vec<_>>()
        .join("\n");
    let command = package
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "name").then(|| value.trim().trim_matches('"').to_owned())
        })
        .expect("workspace package has a name");
    let brand = manifest
        .split("[package.metadata.brand]")
        .nth(1)
        .expect("workspace brand metadata exists")
        .lines()
        .take_while(|line| !line.trim_start().starts_with('['))
        .collect::<Vec<_>>()
        .join("\n");
    let name = brand
        .lines()
        .find_map(|line| {
            let (key, value) = line.split_once('=')?;
            (key.trim() == "name").then(|| value.trim().trim_matches('"').to_owned())
        })
        .expect("workspace brand has a name");
    println!("cargo:rustc-env=MAITTY_APP_COMMAND={command}");
    println!("cargo:rustc-env=MAITTY_APP_NAME={name}");
    println!(
        "cargo:rustc-env=MAITTY_APP_ENV={}",
        command.to_ascii_uppercase().replace('-', "_")
    );
}
