//! Pasting the clipboard image into a pane of another machine. The picture is on this computer
//! and the pane's programs run there, so the image is sent over the machine's own hardened ssh
//! line as a file the far side keeps (`remote_paste`), and the path it names is
//! typed into the pane, as a local paste types a local path. Run it off the UI thread: reading
//! the clipboard and the upload both take a while.

use super::{ClientTerm, clipboard_image_to_file};
use crate::cli::machine_update::ssh_run;
use crate::ipc::Machine;
use crate::remote_paste::{MAX_PASTE_BYTES, paste_failure, paste_file_name, pasted_path};
use std::time::Duration;

/// How long sending one image may take, connecting included.
const PASTE_LIMIT: Duration = Duration::from_secs(120);

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

pub(super) fn upload(
    machine: &Machine,
    name: &str,
    png: &std::path::Path,
) -> Result<String, String> {
    let bytes = std::fs::metadata(png)
        .map_err(|e| format!("could not read the image to send: {e}"))?
        .len();
    if bytes > MAX_PASTE_BYTES {
        return Err(format!(
            "the image is {} MB, more than the {} MB that can be sent to {name}",
            bytes.div_ceil(1024 * 1024),
            MAX_PASTE_BYTES / (1024 * 1024)
        ));
    }
    let argv = machine
        .paste_argv(&file_name(), bytes)
        .map_err(|e| format!("pasting an image to {name} needs ssh: {e}"))?;
    let ran = ssh_run::run(&argv, Some(png), PASTE_LIMIT)?;
    if ran.code == Some(0) {
        pasted_path(&ran.stdout).map_err(|why| format!("{name}: {why}"))
    } else {
        Err(paste_failure(name, ran.code, &ran.stderr))
    }
}

/// A name no other paste has: the time, and something that differs between two in a millisecond.
fn file_name() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let salt = now.subsec_nanos() ^ std::process::id().rotate_left(16);
    paste_file_name(now.as_millis(), salt)
}

/// The pane a paste is for, on a machine other than this computer.
pub(super) fn is_remote(term: ClientTerm) -> bool {
    term.machine != super::MachineId::LOCAL
}
