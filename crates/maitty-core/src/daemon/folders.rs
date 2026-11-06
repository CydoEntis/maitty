//! Listing the folders inside a folder, for a client that is choosing where to start a session
//! (the TUI's New Session popup, the desktop app's folder dialog). Only folder names, never file
//! names or contents.

use crate::protocol::{FolderEntry, FolderListing, FOLDERS_MAX};
use std::path::{Path, PathBuf};

/// The longest path worth listing; a longer one is a mistake or an attack, and is refused.
pub(super) const PATH_MAX_BYTES: usize = 4096;

/// A path as a client types it: `~` and `~/x` mean the daemon's home folder, anything else must
/// be absolute. None for a relative path, which means nothing to a daemon whose working folder is
/// not the client's.
pub(super) fn resolve(typed: &str, home: Option<&Path>) -> Option<PathBuf> {
    let typed = typed.trim();
    let expanded = match typed.strip_prefix('~') {
        Some("") => home?.to_path_buf(),
        Some(rest) if rest.starts_with(['/', '\\']) => {
            home?.join(rest.trim_start_matches(['/', '\\']))
        }
        Some(_) => return None,
        None => PathBuf::from(typed),
    };
    expanded.is_absolute().then_some(expanded)
}

/// The folders directly inside `typed`, sorted by name without regard to case. A folder that is
/// not there, or cannot be read, is a listing that says so, not an error: the client is typing a
/// path and most of what it types does not exist yet.
pub(super) fn list(typed: &str, home: Option<&Path>) -> FolderListing {
    let Some(folder) = resolve(typed, home) else {
        return FolderListing::missing(typed);
    };
    let shown = folder.to_string_lossy().replace('\\', "/");
    let Ok(read) = std::fs::read_dir(&folder) else {
        return FolderListing::missing(&shown);
    };
    let mut entries = Vec::new();
    let mut truncated = false;
    for entry in read.filter_map(Result::ok) {
        let path = entry.path();
        // A link to a folder counts as a folder; a link to a file does not.
        if !std::fs::metadata(&path).is_ok_and(|meta| meta.is_dir()) {
            continue;
        }
        if entries.len() >= FOLDERS_MAX {
            truncated = true;
            break;
        }
        entries.push(FolderEntry {
            name: entry.file_name().to_string_lossy().into_owned(),
            git: branch_of(&path),
            has_children: has_subfolder(&path),
        });
    }
    entries.sort_by_cached_key(|entry| entry.name.to_lowercase());
    FolderListing {
        path: shown,
        exists: true,
        entries,
        truncated,
    }
}

/// Whether `folder` holds a folder worth opening: hidden ones (`.git`) do not count, so a
/// repository with nothing else in it is a place to start a session, not a place to browse.
fn has_subfolder(folder: &Path) -> bool {
    std::fs::read_dir(folder).is_ok_and(|read| {
        read.filter_map(Result::ok).any(|entry| {
            !entry.file_name().to_string_lossy().starts_with('.')
                && std::fs::metadata(entry.path()).is_ok_and(|meta| meta.is_dir())
        })
    })
}

/// The branch a git checkout is on (a short commit for a detached HEAD), or None for a folder
/// that is not a checkout. Reads two small files; runs no git.
fn branch_of(folder: &Path) -> Option<String> {
    let dot_git = folder.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        // A linked worktree has a file naming its real git folder.
        let pointer = std::fs::read_to_string(&dot_git).ok()?;
        let target = pointer.trim().strip_prefix("gitdir:")?.trim();
        let target = Path::new(target);
        if target.is_absolute() {
            target.to_path_buf()
        } else {
            folder.join(target)
        }
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let head = head.trim();
    Some(match head.strip_prefix("ref: refs/heads/") {
        Some(branch) => branch.to_string(),
        None => head.chars().take(7).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scratch(label: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!(
            "{}-folders-{label}-{}",
            env!("APP_COMMAND"),
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn a_tilde_is_the_home_folder_and_a_relative_path_is_nothing() {
        // A path with no drive letter is not absolute on Windows, so the fake home needs one there.
        let home = Path::new(if cfg!(windows) { r"C:\Users\me" } else { "/home/me" });
        assert_eq!(resolve("~", Some(home)), Some(home.to_path_buf()));
        assert_eq!(resolve("~/code/x", Some(home)), Some(home.join("code").join("x")));
        assert_eq!(resolve("~other/x", Some(home)), None);
        assert_eq!(resolve("code/x", Some(home)), None);
        assert_eq!(resolve("~/x", None), None);
        #[cfg(unix)]
        assert_eq!(resolve("/srv/x", None), Some(PathBuf::from("/srv/x")));
    }

    #[test]
    fn lists_folders_only_sorted_without_regard_to_case_with_their_branch_and_children() {
        let root = scratch("list");
        std::fs::create_dir_all(root.join("beta/inner")).unwrap();
        std::fs::create_dir_all(root.join("Alpha/.git")).unwrap();
        std::fs::write(root.join("Alpha/.git/HEAD"), "ref: refs/heads/fixes/one\n").unwrap();
        std::fs::write(root.join("a-file.txt"), "x").unwrap();

        let listing = list(&root.to_string_lossy(), None);

        assert!(listing.exists);
        assert!(!listing.truncated);
        let names: Vec<_> = listing.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["Alpha", "beta"]);
        let alpha = &listing.entries[0];
        assert_eq!(alpha.git.as_deref(), Some("fixes/one"));
        // Its only subfolder is `.git`, which does not count.
        assert!(!alpha.has_children);
        let beta = &listing.entries[1];
        assert_eq!(beta.git, None);
        assert!(beta.has_children);
    }

    #[test]
    fn a_detached_head_shows_a_short_commit_and_a_worktree_file_is_followed() {
        let root = scratch("head");
        std::fs::create_dir_all(root.join("detached/.git")).unwrap();
        std::fs::write(
            root.join("detached/.git/HEAD"),
            "0123456789abcdef0123456789abcdef01234567\n",
        )
        .unwrap();
        std::fs::create_dir_all(root.join("real-git")).unwrap();
        std::fs::write(root.join("real-git/HEAD"), "ref: refs/heads/main\n").unwrap();
        std::fs::create_dir_all(root.join("linked")).unwrap();
        std::fs::write(
            root.join("linked/.git"),
            format!("gitdir: {}\n", root.join("real-git").display()),
        )
        .unwrap();

        let listing = list(&root.to_string_lossy(), None);
        let branch = |name: &str| {
            listing
                .entries
                .iter()
                .find(|entry| entry.name == name)
                .and_then(|entry| entry.git.clone())
        };
        assert_eq!(branch("detached").as_deref(), Some("0123456"));
        assert_eq!(branch("linked").as_deref(), Some("main"));
    }

    #[test]
    fn a_folder_that_is_not_there_is_a_listing_that_says_so() {
        let root = scratch("missing");
        let listing = list(&root.join("nope").to_string_lossy(), None);
        assert!(!listing.exists);
        assert!(listing.entries.is_empty());
        let relative = list("code/x", None);
        assert!(!relative.exists);
    }

    #[test]
    fn a_huge_folder_is_cut_and_says_so() {
        let root = scratch("many");
        for number in 0..=FOLDERS_MAX {
            std::fs::create_dir(root.join(format!("d{number:05}"))).unwrap();
        }
        let listing = list(&root.to_string_lossy(), None);
        assert_eq!(listing.entries.len(), FOLDERS_MAX);
        assert!(listing.truncated);
    }
}
