//! Pasting the clipboard image, or sending any file, into a pane of another machine. The file is
//! on this computer and the pane's programs run there, so it is sent over the machine's own
//! hardened ssh line as a file the far side keeps (`remote_paste`, ADR 0049 and 0050), and the
//! path it names is typed into the pane, as a local paste types a local path. Run it off the UI
//! thread: reading the clipboard and the upload both take a while.

use super::{ClientTerm, clipboard_image_to_file};
use crate::cli::machine_update::ssh_run::{self, Watch};
use crate::ipc::Machine;
use crate::remote_paste::{
    MAX_PASTE_BYTES, MAX_SEND_BYTES, paste_failure, paste_file_name, pasted_path, sent_file_name,
};
use std::path::Path;
use std::time::Duration;

/// How long sending one image may take, connecting included.
const PASTE_LIMIT: Duration = Duration::from_secs(120);

/// How long sending one file may take, connecting included.
const SEND_LIMIT: Duration = Duration::from_secs(15 * 60);

/// What is being sent: the words an error uses, the size it may have, how long it may take.
#[derive(Clone, Copy)]
enum Kind {
    Image,
    File,
}

impl Kind {
    fn what(self) -> &'static str {
        match self {
            Kind::Image => "the image",
            Kind::File => "the file",
        }
    }

    fn cap(self) -> u64 {
        match self {
            Kind::Image => MAX_PASTE_BYTES,
            Kind::File => MAX_SEND_BYTES,
        }
    }

    fn limit(self) -> Duration {
        match self {
            Kind::Image => PASTE_LIMIT,
            Kind::File => SEND_LIMIT,
        }
    }
}

/// An image kept on the far side.
pub(super) struct Sent {
    /// Its full path there.
    pub path: String,
    pub width: usize,
    pub height: usize,
}

/// Send the clipboard image to `machine`, called `name` in words for the person.
pub(super) fn send(machine: &Machine, name: &str) -> Result<Sent, String> {
    let (png, width, height) = clipboard_image_to_file()?;
    let result = upload(machine, name, &png);
    // The copy made here was only for sending: the picture is now where the pane can read it.
    let _ = std::fs::remove_file(&png);
    result.map(|path| Sent {
        path,
        width,
        height,
    })
}

pub(super) fn upload(machine: &Machine, name: &str, png: &Path) -> Result<String, String> {
    transfer(
        machine,
        name,
        png,
        Kind::Image,
        &file_name(),
        &Watch::default(),
    )
}

/// Send the file at `path` to `machine`, called `name` in words for the person. Anything but a
/// regular file is refused here, before ssh starts. `watch` reports bytes sent and can end it.
pub(super) fn send_file(
    machine: &Machine,
    name: &str,
    path: &Path,
    watch: &Watch,
) -> Result<String, String> {
    let meta =
        std::fs::metadata(path).map_err(|e| format!("could not read {}: {e}", shown(path)))?;
    if !meta.is_file() {
        return Err(format!(
            "{} is not a file; only files can be sent",
            shown(path)
        ));
    }
    let original = path
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (stamp, salt) = stamp_and_salt();
    let remote_name = sent_file_name(stamp, salt, &original);
    transfer(machine, name, path, Kind::File, &remote_name, watch)
}

/// A file's name for a message: the name only, so a long path does not fill a toast.
pub(super) fn shown(path: &Path) -> String {
    path.file_name().map_or_else(
        || path.display().to_string(),
        |n| n.to_string_lossy().into_owned(),
    )
}

fn transfer(
    machine: &Machine,
    name: &str,
    local: &Path,
    kind: Kind,
    remote_name: &str,
    watch: &Watch,
) -> Result<String, String> {
    let what = kind.what();
    let bytes = std::fs::metadata(local)
        .map_err(|e| format!("could not read {what} to send: {e}"))?
        .len();
    if bytes > kind.cap() {
        return Err(format!(
            "{what} is {} MB, more than the {} MB that can be sent to {name}",
            bytes.div_ceil(1024 * 1024),
            kind.cap() / (1024 * 1024)
        ));
    }
    watch
        .total
        .store(bytes, std::sync::atomic::Ordering::Relaxed);
    let argv = machine
        .paste_argv(remote_name, bytes)
        .map_err(|e| format!("sending {what} to {name} needs ssh: {e}"))?;
    let ran = ssh_run::run_watched(&argv, Some(local), kind.limit(), watch)?;
    if ran.cancelled {
        return Err(format!("stopped sending {what} to {name}"));
    }
    if ran.code == Some(0) {
        pasted_path(&ran.stdout).map_err(|why| format!("{name}: {why}"))
    } else {
        Err(paste_failure(what, name, ran.code, &ran.stderr))
    }
}

fn stamp_and_salt() -> (u128, u32) {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    (
        now.as_millis(),
        now.subsec_nanos() ^ std::process::id().rotate_left(16),
    )
}

/// A name no other paste has: the time, and something that differs between two in a millisecond.
fn file_name() -> String {
    let (stamp, salt) = stamp_and_salt();
    paste_file_name(stamp, salt)
}

/// The pane a paste is for, on a machine other than this computer.
pub(super) fn is_remote(term: ClientTerm) -> bool {
    term.machine != super::MachineId::LOCAL
}
