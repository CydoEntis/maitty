//! Code names its parts by role, never by the product's name, so a rename of the product
//! doesn't mean a mass find-and-replace. Retired product names may appear only where the
//! app migrates what an old install left behind.

use std::path::{Path, PathBuf};

const RETIRED_NAMES: [&str; 2] = ["hydra", "drover"];

/// Files that read or rewrite what an old install left on disk, so they must name it.
const MIGRATION_FILES: [&str; 5] = ["src/cli.rs", "src/config.rs", "src/project.rs", "src/theme.rs", "src/client/themes.rs"];

fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    for entry in std::fs::read_dir(dir).expect("readable source dir") {
        let path = entry.expect("dir entry").path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|e| e == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn retired_product_names_stay_in_migration_code() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    rust_files(&root.join("src"), &mut files);
    let mut hits = Vec::new();
    for file in files {
        let rel = file.strip_prefix(root).unwrap().to_string_lossy().replace('\\', "/");
        if MIGRATION_FILES.contains(&rel.as_str()) {
            continue;
        }
        let text = std::fs::read_to_string(&file).expect("utf-8 source");
        for (n, line) in text.lines().enumerate() {
            let lower = line.to_lowercase();
            if RETIRED_NAMES.iter().any(|name| lower.contains(name)) {
                hits.push(format!("{rel}:{}: {}", n + 1, line.trim()));
            }
        }
    }
    assert!(hits.is_empty(), "name parts by role, not by a retired product name:\n{}", hits.join("\n"));
}
