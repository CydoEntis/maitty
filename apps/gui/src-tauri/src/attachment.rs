//! Chat attachments kept on this computer (ticket 20, ADR 0053). The window hands the host a path
//! the operating system gave on a drop or from the file picker; the host checks that it is a regular
//! file within the caps, copies it into a private folder of the app's cache and returns the copy's
//! path and cleaned name. The agent is told the copy's path, never the original: the original may
//! change or disappear before the agent reads it, and the path string came from outside.
//!
//! Nothing here decodes an image or opens an archive; a zip is just a file. The copy runs on a
//! blocking worker, so the UI thread is never held.

use crate::image_paste::{check_image, private_folder, stamp};
use maitty_machines::remote_paste::{MAX_PASTE_BYTES, MAX_SEND_BYTES};
use serde::Serialize;
use std::io::{Read, Write};
use std::path::{Component, Path, PathBuf};
use std::time::{Duration, SystemTime};
use tauri::Manager;

/// The folder inside the app's cache where attachments are kept.
const FOLDER: &str = "attachments";

/// The folder pasted images are kept in (`image_paste`); a message may name a file there too.
const PASTE_FOLDER: &str = "pastes";

/// Every kept file starts with this; the sweep touches nothing else.
const PREFIX: &str = "attach-";

/// Attachments older than this are removed (as pasted images are).
const KEEP: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// The longest cleaned name kept, extension included.
const NAME_MAX: usize = 64;

/// The longest extension kept when a long name is cut.
const EXTENSION_MAX: usize = 8;

/// How many leading bytes tell an image or an archive apart from any other file.
const SNIFF: usize = 16;

const BYTES_PER_MB: u64 = 1024 * 1024;

/// What the window shows an attachment as.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kind {
    Image,
    Archive,
    File,
}

/// A file kept for a message.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Attached {
    /// The copy's absolute path: what the message tells the agent.
    pub path: String,
    /// The cleaned name the person sees; the copy's name ends with it.
    pub name: String,
    pub size: u64,
    pub kind: Kind,
}

/// A name that is safe in a path with no quoting: letters, digits, `.`, `_` and `-`, no leading
/// dot, at most [`NAME_MAX`] characters. Directory parts are dropped; spaces, control, direction
/// and invisible characters become `_`, so what the window shows is what is on disk.
pub fn clean_name(raw: &str) -> String {
    let base = raw.rsplit(['/', '\\']).next().unwrap_or("");
    let mapped: String = base
        .chars()
        .map(|c| {
            if c.is_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let mut name = mapped.trim_start_matches('.').to_owned();
    if name.chars().count() > NAME_MAX {
        // Keep the extension: it is what tells the agent what the file is.
        let extension = name
            .rsplit_once('.')
            .map(|(_, ext)| ext)
            .filter(|ext| (1..=EXTENSION_MAX).contains(&ext.chars().count()))
            .map(str::to_owned);
        name = match extension {
            Some(ext) => {
                let stem: String = name.chars().take(NAME_MAX - ext.chars().count() - 1).collect();
                format!("{stem}.{ext}")
            }
            None => name.chars().take(NAME_MAX).collect(),
        };
    }
    if name.is_empty() {
        "file".to_owned()
    } else {
        name
    }
}

/// The kind from a file's first bytes; the name's extension is not trusted.
fn kind_of(head: &[u8]) -> Kind {
    if check_image(head).is_ok() {
        Kind::Image
    } else if head.starts_with(b"PK\x03\x04") || head.starts_with(b"PK\x05\x06") {
        Kind::Archive
    } else {
        Kind::File
    }
}

fn too_big(name: &str, size: u64, cap: u64) -> String {
    format!(
        "{name} is {} MB, more than the {} MB that can be attached",
        size.div_ceil(BYTES_PER_MB),
        cap / BYTES_PER_MB
    )
}

/// Copy `source` into `folder` and describe the copy. Refuses a path that is not a regular file
/// (a folder, a device, a pipe), an empty file, and a file over the cap for its kind; a refused
/// file leaves nothing behind.
pub fn attach(folder: &Path, source: &Path) -> Result<Attached, String> {
    let shown = source.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
    let name = clean_name(&shown);
    // `metadata` follows a link: a link to a file is a file the person chose.
    let meta = std::fs::metadata(source).map_err(|e| format!("could not read {name}: {e}"))?;
    if !meta.is_file() {
        return Err(format!("{name} is not a file; folders cannot be attached"));
    }
    if meta.len() == 0 {
        return Err(format!("{name} is empty"));
    }
    if meta.len() > MAX_SEND_BYTES {
        return Err(too_big(&name, meta.len(), MAX_SEND_BYTES));
    }
    let mut input = std::fs::File::open(source).map_err(|e| format!("could not open {name}: {e}"))?;
    let mut head = [0u8; SNIFF];
    let got = read_up_to(&mut input, &mut head).map_err(|e| format!("could not read {name}: {e}"))?;
    let kind = kind_of(&head[..got]);
    if kind == Kind::Image && meta.len() > MAX_PASTE_BYTES {
        return Err(too_big(&name, meta.len(), MAX_PASTE_BYTES));
    }
    private_folder(folder).map_err(|e| format!("could not make the attachments folder: {e}"))?;
    sweep(folder, SystemTime::now());
    let cap = if kind == Kind::Image { MAX_PASTE_BYTES } else { MAX_SEND_BYTES };
    let copy = folder.join(format!("{PREFIX}{}-{name}", stamp()));
    let size = copy_private(&mut input, &head[..got], &copy, cap).map_err(|e| format!("could not copy {name}: {e}"))?;
    Ok(Attached { path: copy.display().to_string(), name, size, kind })
}

fn read_up_to(input: &mut impl Read, buf: &mut [u8]) -> std::io::Result<usize> {
    let mut got = 0;
    while got < buf.len() {
        let n = input.read(&mut buf[got..])?;
        if n == 0 {
            break;
        }
        got += n;
    }
    Ok(got)
}

/// Write `head` then the rest of `input` to a new owner-only file, never over an existing one;
/// over `cap` bytes the half-written copy is removed.
fn copy_private(input: &mut impl Read, head: &[u8], path: &Path, cap: u64) -> std::io::Result<u64> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut out = options.open(path)?;
    let result = (|| {
        out.write_all(head)?;
        // One byte past the cap, to tell a file that grew from one that fits.
        let rest = std::io::copy(&mut input.take(cap + 1 - head.len() as u64), &mut out)?;
        let total = head.len() as u64 + rest;
        if total > cap {
            return Err(std::io::Error::other("the file grew past the limit while it was copied"));
        }
        out.flush()?;
        Ok(total)
    })();
    if result.is_err() {
        drop(out);
        let _ = std::fs::remove_file(path);
    }
    result
}

/// Remove this folder's own files that are past their time. Anything else in it is left alone.
fn sweep(folder: &Path, now: SystemTime) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        if !entry.file_name().to_string_lossy().starts_with(PREFIX) {
            continue;
        }
        let old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .is_ok_and(|at| now.duration_since(at).is_ok_and(|age| age > KEEP));
        if old && let Err(error) = std::fs::remove_file(entry.path()) {
            tracing::warn!("could not clear an old attachment: {error}");
        }
    }
}

/// Which of `paths` are still kept files: directly inside one of `folders` and present. A path
/// anywhere else is reported missing, so the window cannot use this to probe the disk.
pub fn exist(folders: &[PathBuf], paths: &[String]) -> Vec<bool> {
    paths
        .iter()
        .map(|path| {
            let path = Path::new(path);
            // Compared as written, not resolved: a `..` part never matches a folder.
            let inside = path.parent().is_some_and(|parent| folders.iter().any(|folder| folder == parent));
            inside
                && !path.components().any(|part| matches!(part, Component::ParentDir))
                && std::fs::symlink_metadata(path).is_ok_and(|meta| meta.is_file())
        })
        .collect()
}

fn cache_of(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map_err(|e| format!("this computer has no cache folder for the app: {e}"))
}

/// Keep a copy of the file at `path` (from a drop or the picker) for a chat message.
#[tauri::command]
pub async fn attach_local_file(app: tauri::AppHandle, path: String) -> Result<Attached, String> {
    let folder = cache_of(&app)?.join(FOLDER);
    tauri::async_runtime::spawn_blocking(move || attach(&folder, Path::new(&path)))
        .await
        .map_err(|e| format!("attaching the file stopped: {e}"))?
}

/// For each path of a sent message, whether the kept file is still there (a swept one is not).
#[tauri::command]
pub async fn attachments_exist(app: tauri::AppHandle, paths: Vec<String>) -> Result<Vec<bool>, String> {
    let cache = cache_of(&app)?;
    let folders = [cache.join(FOLDER), cache.join(PASTE_FOLDER)];
    tauri::async_runtime::spawn_blocking(move || exist(&folders, &paths))
        .await
        .map_err(|e| format!("checking the files stopped: {e}"))
}

#[cfg(test)]
mod tests;
