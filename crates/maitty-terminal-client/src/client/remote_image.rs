//! Pasting the clipboard image, or sending any file, into a pane of another machine. The file is
//! on this computer and the pane's programs run there, so it is sent over the machine's own
//! hardened ssh line as a file the far side keeps (`remote_paste`, ADR 0007 and 0055), and the
//! path it names is typed into the pane, as a local paste types a local path. Run it off the UI
//! thread: reading the clipboard and the upload both take a while.

use super::{ClientTerm, clipboard_image_to_file};
use crate::ipc::Machine;
use maitty_machines::remote_send;
use std::path::{Path, PathBuf};

pub(super) use maitty_machines::remote_send::{send_file, shown};

/// An image kept on the far side.
pub(super) struct Sent {
    /// Its full path there.
    pub path: String,
    pub width: usize,
    pub height: usize,
}

/// Send the clipboard image to `machine`, called `name` in words for the person.
pub(super) fn send(machine: &Machine, name: &str) -> Result<Sent, String> {
    send_saved(machine, name, clipboard_image_to_file()?)
}

/// Send a clipboard image already saved here (`clipboard_image_to_file`) to `machine`.
pub(super) fn send_saved(machine: &Machine, name: &str, (png, width, height): (PathBuf, usize, usize)) -> Result<Sent, String> {
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
    remote_send::upload_image(machine, name, png)
}

/// The pane a paste is for, on a machine other than this computer.
pub(super) fn is_remote(term: ClientTerm) -> bool {
    term.machine != super::MachineId::LOCAL
}
