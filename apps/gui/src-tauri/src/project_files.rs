//! Folder-name suggestions for the Open project box.
//!
//! Only folder names are listed, never file names or contents, which is what the
//! native folder picker already shows.

use std::path::PathBuf;

const MAX_SUGGESTIONS: usize = 8;

/// Folders that continue what was typed, as full paths ending in a separator.
/// Only folder names are listed, never file names or contents.
pub fn suggest_folders(typed: &str) -> Vec<String> {
    let separator_at = typed.rfind(['/', '\\']);
    let (parent_text, prefix) = match separator_at {
        Some(index) => (&typed[..=index], &typed[index + 1..]),
        None => return Vec::new(),
    };
    let separator = &parent_text[parent_text.len() - 1..];
    let parent = expand_home(parent_text);
    let wanted = prefix.to_lowercase();
    let Ok(read) = std::fs::read_dir(&parent) else {
        return Vec::new();
    };
    let mut names = read
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_dir()))
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        // Hidden folders appear only once a dot has been typed.
        .filter(|name| prefix.starts_with('.') || !name.starts_with('.'))
        .filter(|name| name.to_lowercase().starts_with(&wanted))
        .collect::<Vec<_>>();
    names.sort_by_key(|name| name.to_lowercase());
    names.truncate(MAX_SUGGESTIONS);
    names
        .into_iter()
        .map(|name| format!("{parent_text}{name}{separator}"))
        .collect()
}

fn expand_home(path: &str) -> PathBuf {
    let home = std::env::var_os("USERPROFILE").or_else(|| std::env::var_os("HOME"));
    match (path.strip_prefix('~'), home) {
        (Some(rest), Some(home)) => {
            PathBuf::from(home).join(rest.trim_start_matches(['/', '\\']))
        }
        _ => PathBuf::from(path),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT: AtomicU64 = AtomicU64::new(0);

    fn project() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "maitty-folder-suggestions-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        ));
        std::fs::create_dir_all(root.join("src")).unwrap();
        std::fs::create_dir_all(root.join(".git")).unwrap();
        std::fs::write(root.join("README.md"), "# hello
").unwrap();
        root
    }

    #[test]
    fn suggests_matching_folders_only() {
        let root = project();
        std::fs::create_dir_all(root.join("srv")).unwrap();
        std::fs::create_dir_all(root.join(".hidden")).unwrap();
        let typed = format!("{}/s", root.to_string_lossy().replace('\\', "/"));
        let suggestions = suggest_folders(&typed);
        assert_eq!(suggestions.len(), 2);
        assert!(suggestions[0].ends_with("/src/"));
        assert!(suggestions[1].ends_with("/srv/"));
        assert!(suggest_folders("no-separator").is_empty());
        let dot = format!("{}/.", root.to_string_lossy().replace('\\', "/"));
        assert!(
            suggest_folders(&dot)
                .iter()
                .any(|path| path.contains(".hidden"))
        );
    }
}
