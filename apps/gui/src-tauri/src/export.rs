//! Saving a piece of a chat (a code block, a table) as a file the person asked for. The window
//! names the file and gives its text; the host trusts neither. The name must be a plain file name
//! of a type that only holds text to read, the text has a size limit, and the folder must be one
//! that exists: the person's own choice from Settings, or Downloads when they chose none. An
//! existing file is never replaced; the new one gets a number instead.

use std::fs::OpenOptions;
use std::io::{ErrorKind, Write};
use std::path::{Path, PathBuf};
use tauri::Manager;

/// The most text one export may hold.
const MAX_EXPORT_BYTES: usize = 5 * 1024 * 1024;

/// The longest file name taken, well inside what every file system allows.
const MAX_NAME_CHARS: usize = 120;

/// The types an export may be saved as. Nothing the system would run as a program.
const EXTENSIONS: [&str; 5] = ["md", "html", "csv", "txt", "json"];

/// How many numbered names are tried when the name is taken.
const MAX_NUMBERED: u32 = 200;

/// The name split into its stem and extension, when it is one this feature writes.
fn checked_name(name: &str) -> Result<(&str, &str), String> {
    let plain = |c: char| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_' | '.');
    if name.is_empty() || name.chars().count() > MAX_NAME_CHARS || !name.chars().all(plain) {
        return Err("that is not a file name an export can be saved under".into());
    }
    let (stem, extension) = name
        .rsplit_once('.')
        .ok_or("an export needs a file type")?;
    if stem.is_empty() || stem.starts_with('.') || stem.ends_with(' ') {
        return Err("that is not a file name an export can be saved under".into());
    }
    if !EXTENSIONS.contains(&extension) {
        return Err(format!("exports are not saved as .{extension} files"));
    }
    Ok((stem, extension))
}

/// Write `text` into `folder` under `name`, or under a numbered name when that one is taken.
pub fn save_in(folder: &Path, name: &str, text: &str) -> Result<PathBuf, String> {
    let (stem, extension) = checked_name(name)?;
    if text.len() > MAX_EXPORT_BYTES {
        return Err(format!(
            "that is more than the {} MB an export can hold",
            MAX_EXPORT_BYTES / (1024 * 1024)
        ));
    }
    if !folder.is_absolute() || !folder.is_dir() {
        return Err(format!(
            "the export folder {} does not exist; choose one in Settings",
            folder.display()
        ));
    }
    for number in 1..=MAX_NUMBERED {
        let file = if number == 1 {
            format!("{stem}.{extension}")
        } else {
            format!("{stem} ({number}).{extension}")
        };
        let path = folder.join(file);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(mut created) => {
                created
                    .write_all(text.as_bytes())
                    .map_err(|e| format!("could not write {}: {e}", path.display()))?;
                return Ok(path);
            }
            Err(e) if e.kind() == ErrorKind::AlreadyExists => continue,
            Err(e) => return Err(format!("could not create {}: {e}", path.display())),
        }
    }
    Err(format!("{} already holds too many files named {name}", folder.display()))
}

/// Save an export and return where it went. An empty `folder` means Downloads.
#[tauri::command]
pub async fn save_export(
    app: tauri::AppHandle,
    folder: String,
    name: String,
    text: String,
) -> Result<String, String> {
    let folder = if folder.trim().is_empty() {
        app.path()
            .download_dir()
            .map_err(|e| format!("could not find your Downloads folder: {e}"))?
    } else {
        PathBuf::from(folder.trim())
    };
    tauri::async_runtime::spawn_blocking(move || {
        save_in(&folder, &name, &text).map(|path| path.display().to_string())
    })
    .await
    .map_err(|e| format!("saving the export stopped: {e}"))?
}

#[cfg(test)]
mod tests {
    use super::*;

    fn folder(name: &str) -> PathBuf {
        let folder = std::env::temp_dir().join(format!("maitty-export-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&folder);
        std::fs::create_dir_all(&folder).unwrap();
        folder
    }

    #[test]
    fn an_export_is_written_under_its_name() {
        let folder = folder("plain");
        let path = save_in(&folder, "table 2026-11-10.md", "| a |\n").unwrap();
        assert_eq!(path, folder.join("table 2026-11-10.md"));
        assert_eq!(std::fs::read_to_string(path).unwrap(), "| a |\n");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_taken_name_gets_a_number_and_the_first_file_is_kept() {
        let folder = folder("taken");
        save_in(&folder, "notes.md", "first").unwrap();
        let second = save_in(&folder, "notes.md", "second").unwrap();
        assert_eq!(second, folder.join("notes (2).md"));
        assert_eq!(std::fs::read_to_string(folder.join("notes.md")).unwrap(), "first");
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_name_that_leaves_the_folder_or_could_run_is_refused() {
        let folder = folder("names");
        for name in ["../up.md", "sub/file.md", "C:\\x.md", "run.bat", "page.htm.exe", ".md", "none", ""] {
            assert!(save_in(&folder, name, "x").is_err(), "{name:?} was taken");
        }
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(folder);
    }

    #[test]
    fn a_folder_that_is_missing_or_relative_is_refused() {
        let gone = std::env::temp_dir().join("maitty-export-not-here");
        assert!(save_in(&gone, "a.md", "x").is_err());
        assert!(save_in(Path::new("relative"), "a.md", "x").is_err());
    }

    #[test]
    fn text_past_the_limit_is_refused() {
        let folder = folder("large");
        let text = "x".repeat(MAX_EXPORT_BYTES + 1);
        assert!(save_in(&folder, "big.txt", &text).is_err());
        assert_eq!(std::fs::read_dir(&folder).unwrap().count(), 0);
        let _ = std::fs::remove_dir_all(folder);
    }
}
