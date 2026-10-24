//! The app's name lives in `Cargo.toml` (ADR 0019): code, adapters and comments name their parts
//! by role and take the name from `brand`, so a rename is that file and the docs. Names the app
//! had before may appear only where it carries over what an old install left behind.

use std::path::{Path, PathBuf};

/// Files that read or rewrite what an old install left on disk, and their tests, which name it.
const CARRY_OVER_FILES: [&str; 5] = ["src/cli.rs", "src/config.rs", "src/project.rs", "src/theme.rs", "src/client/themes.rs"];

/// Source the app is built from: Rust and the adapters it installs. Reference frames (`.txt`)
/// come from the design source and are regenerated with it.
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            sources(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs" || e == "js" || e == "mjs") {
            out.push(path);
        }
    }
}

/// Whether `line` holds `name` as a word: no letter right before or after it, so `_seshi` and
/// `seshi.exe` count and `hydrate` doesn't.
fn names_word(line: &str, name: &str) -> bool {
    line.match_indices(name).any(|(at, _)| {
        let before = line[..at].chars().next_back();
        let after = line[at + name.len()..].chars().next();
        !before.is_some_and(char::is_alphabetic) && !after.is_some_and(char::is_alphabetic)
    })
}

/// Every line of `src` holding one of `names` (any case), outside `skip`.
fn lines_naming(names: &[String], skip: &[&str]) -> Vec<String> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    sources(&root.join("src"), &mut files);
    let mut hits = Vec::new();
    for file in files {
        let rel = file.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if skip.contains(&rel.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("utf-8 source");
        for (n, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if names.iter().any(|name| names_word(&lower, name)) {
                hits.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    hits
}

#[test]
fn the_apps_name_is_never_written_out() {
    let names = [env!("APP_COMMAND"), env!("APP_NAME")].map(str::to_lowercase).to_vec();
    let hits = lines_naming(&names, &[]);
    assert!(hits.is_empty(), "take the name from brand (env!(\"APP_COMMAND\") and friends), not as text:\n{}", hits.join("\n"));
}

#[test]
fn names_the_app_had_before_stay_in_carry_over_code() {
    let previous: Vec<String> = env!("APP_PREVIOUS").split(',').filter(|p| !p.is_empty()).map(str::to_lowercase).collect();
    let hits = lines_naming(&previous, &CARRY_OVER_FILES);
    assert!(hits.is_empty(), "name parts by role, not by a name the app had before:\n{}", hits.join("\n"));
}
