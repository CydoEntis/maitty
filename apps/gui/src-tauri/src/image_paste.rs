//! Pasting an image into a pane (tickets 19 and 20, ADR 0049, 0055). The window reads the
//! clipboard image and hands the host its bytes as the raw body of a command, never as base64 in
//! an event. The host checks them at the boundary (a PNG, JPEG, GIF or WebP signature, at most
//! 25 MB), keeps them in the app's private cache and either
//!
//! - gives the window the path on this computer (`save_pasted_image`), which it types into a pane
//!   of this computer, or
//! - sends them to a pane's machine through the same runner as dropped files
//!   (`send_pasted_image`, `file_send`), where the progress, the cancel and the checks of what
//!   the far side says are the ones files have.
//!
//! The image itself is never logged. Both commands are small and take only bytes and a machine
//! name, so a chat composer's attachment can call them as a pane's paste does.

use crate::file_send::{Job, SendKind, start, target};
use crate::terminal_stream::MachineStates;
use maitty_machines::MachineDef;
use maitty_machines::remote_paste::MAX_PASTE_BYTES;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};
use tauri::Manager;

/// The folder inside the app's cache where images are kept.
const FOLDER: &str = "pastes";

/// Pasted images older than this are removed (as on the far side, ADR 0049).
const KEEP: Duration = Duration::from_secs(7 * 24 * 60 * 60);

/// Staged copies older than this are leftovers of a send that was killed.
const KEEP_STAGED: Duration = Duration::from_secs(24 * 60 * 60);

/// Header names carrying the small arguments next to the raw body.
const MACHINE_HEADER: &str = "x-maitty-machine";
const TRANSFER_HEADER: &str = "x-maitty-transfer";

/// The image types a pane's agent can read, by their signature.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ImageType {
    Png,
    Jpeg,
    Gif,
    Webp,
}

impl ImageType {
    pub fn extension(self) -> &'static str {
        match self {
            ImageType::Png => "png",
            ImageType::Jpeg => "jpg",
            ImageType::Gif => "gif",
            ImageType::Webp => "webp",
        }
    }
}

/// What the bytes are, from their first bytes alone; the window's word for the type is not
/// trusted. Refuses empty, too large and unknown data.
pub fn check_image(bytes: &[u8]) -> Result<ImageType, String> {
    if bytes.is_empty() {
        return Err("the clipboard image is empty".into());
    }
    if bytes.len() as u64 > MAX_PASTE_BYTES {
        return Err(format!(
            "the image is {} MB, more than the {} MB that can be pasted",
            (bytes.len() as u64).div_ceil(1024 * 1024),
            MAX_PASTE_BYTES / (1024 * 1024)
        ));
    }
    if bytes.starts_with(b"\x89PNG\r\n\x1a\n") {
        Ok(ImageType::Png)
    } else if bytes.starts_with(&[0xFF, 0xD8, 0xFF]) {
        Ok(ImageType::Jpeg)
    } else if bytes.starts_with(b"GIF87a") || bytes.starts_with(b"GIF89a") {
        Ok(ImageType::Gif)
    } else if bytes.len() >= 12 && &bytes[..4] == b"RIFF" && &bytes[8..12] == b"WEBP" {
        Ok(ImageType::Webp)
    } else {
        Err("the clipboard holds something that is not a PNG, JPEG, GIF or WebP image".into())
    }
}

/// What makes a file name differ from every other made in the same millisecond: the time, and
/// something that differs between two processes.
pub(crate) fn stamp() -> String {
    let now = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or_default();
    let salt = now.subsec_nanos() ^ std::process::id().rotate_left(16);
    format!("{}-{salt:08x}", now.as_millis())
}

/// A name no other paste has.
fn unique(prefix: &str, extension: &str) -> String {
    format!("{prefix}-{}.{extension}", stamp())
}

/// Make `folder` (owner-only on Unix) and refuse a link in its place: the person's cache is
/// theirs, but nothing here should follow a link someone else left there.
pub(crate) fn private_folder(folder: &Path) -> std::io::Result<()> {
    if let Ok(meta) = std::fs::symlink_metadata(folder)
        && !meta.is_dir()
    {
        return Err(std::io::Error::other(format!(
            "{} is not a folder",
            folder.display()
        )));
    }
    std::fs::create_dir_all(folder)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(folder, std::fs::Permissions::from_mode(0o700))?;
    }
    Ok(())
}

/// Write `bytes` as a new file `name` in `folder`, readable by its owner alone, never over an
/// existing file.
fn write_private(folder: &Path, name: &str, bytes: &[u8]) -> std::io::Result<PathBuf> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let path = folder.join(name);
    let mut file = options.open(&path)?;
    if let Err(error) = file.write_all(bytes).and_then(|()| file.flush()) {
        drop(file);
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }
    Ok(path)
}

/// Remove this folder's own files that are past their time: pasted images after a week, staged
/// copies after a day. Anything else in the folder is left alone.
fn sweep(folder: &Path, now: SystemTime) {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        let keep = if name.starts_with("paste-") {
            KEEP
        } else if name.starts_with("staged-") {
            KEEP_STAGED
        } else {
            continue;
        };
        let old = entry
            .metadata()
            .and_then(|meta| meta.modified())
            .is_ok_and(|at| now.duration_since(at).is_ok_and(|age| age > keep));
        if old && let Err(error) = std::fs::remove_file(entry.path()) {
            tracing::warn!("could not clear an old pasted image: {error}");
        }
    }
}

/// Keep a pasted image for a pane of this computer: its path.
pub fn save_local(folder: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    let kind = check_image(bytes)?;
    private_folder(folder).map_err(|e| format!("could not make the image folder: {e}"))?;
    sweep(folder, SystemTime::now());
    write_private(folder, &unique("paste", kind.extension()), bytes)
        .map_err(|e| format!("could not save the image: {e}"))
}

/// Check a pasted image headed for `machine` and stage it for the runner: the image's
/// signature and size, the machine (known, ssh, connected) and the id are all checked before a
/// byte is written, so a refused paste leaves nothing behind.
pub fn plan_image(
    folder: &Path,
    machines: &[MachineDef],
    ssh: &[String],
    machine: &str,
    transfer: &str,
    bytes: &[u8],
    is_connected: &dyn Fn(&str) -> bool,
) -> Result<Job, String> {
    let kind = check_image(bytes)?;
    let target = target(machines, ssh, machine, transfer)?;
    if !is_connected(&target.name) {
        return Err(format!(
            "{} is not connected; reconnect before sending an image to it",
            target.name
        ));
    }
    private_folder(folder).map_err(|e| format!("could not make the image folder: {e}"))?;
    sweep(folder, SystemTime::now());
    let staged = write_private(folder, &unique("staged", kind.extension()), bytes)
        .map_err(|e| format!("could not stage the image: {e}"))?;
    Ok(Job {
        transfer: transfer.to_owned(),
        machine: target.machine,
        name: target.name,
        files: vec![staged],
        kind: SendKind::Image {
            extension: kind.extension(),
        },
    })
}

/// The image's folder in the app's cache.
fn folder_of(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_cache_dir()
        .map(|dir| dir.join(FOLDER))
        .map_err(|e| format!("this computer has no cache folder for the app: {e}"))
}

/// The raw bytes of a command's body.
fn body_of(request: &tauri::ipc::Request<'_>) -> Result<Vec<u8>, String> {
    match request.body() {
        tauri::ipc::InvokeBody::Raw(bytes) => Ok(bytes.clone()),
        tauri::ipc::InvokeBody::Json(_) => Err("the image did not arrive as bytes".into()),
    }
}

fn header_of(request: &tauri::ipc::Request<'_>, name: &str) -> Result<String, String> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
        .ok_or_else(|| format!("the request has no {name}"))
}

/// Keep the clipboard image for a pane of this computer and return its path, which the window
/// types into the pane.
#[tauri::command]
pub async fn save_pasted_image(
    app: tauri::AppHandle,
    request: tauri::ipc::Request<'_>,
) -> Result<String, String> {
    let bytes = body_of(&request)?;
    let folder = folder_of(&app)?;
    tauri::async_runtime::spawn_blocking(move || {
        save_local(&folder, &bytes).map(|path| path.display().to_string())
    })
    .await
    .map_err(|e| format!("saving the image stopped: {e}"))?
}

/// Send the clipboard image to the machine named in the `x-maitty-machine` header, as the files
/// of a drop are: how it goes comes as `file-send` events named by `x-maitty-transfer`, and the
/// window types the far-side path into the pane.
#[tauri::command]
pub async fn send_pasted_image(
    app: tauri::AppHandle,
    machines: tauri::State<'_, crate::machines::Machines>,
    request: tauri::ipc::Request<'_>,
) -> Result<(), String> {
    let bytes = body_of(&request)?;
    let machine = header_of(&request, MACHINE_HEADER)?;
    let transfer = header_of(&request, TRANSFER_HEADER)?;
    let folder = folder_of(&app)?;
    let defs = machines.0.machines.clone();
    let handle = app.clone();
    let job = tauri::async_runtime::spawn_blocking(move || {
        let ssh = [maitty_machines::SSH_PROGRAM.to_owned()];
        let states = handle.state::<MachineStates>();
        plan_image(&folder, &defs, &ssh, &machine, &transfer, &bytes, &|name| {
            states.is_connected(name)
        })
    })
    .await
    .map_err(|e| format!("preparing the image stopped: {e}"))??;
    start(&app, job)
}

#[cfg(test)]
mod tests;
