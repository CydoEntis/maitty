//! Sending a pasted image, or any file, to another machine over its own hardened ssh line
//! (ADR 0049, 0050), shared by the terminal app and the desktop app. The caller runs it off its
//! UI thread: the upload takes a while. It returns the file's path on the far side, already
//! checked by `remote_paste::pasted_path`; typing that path into a pane is the caller's job.

use crate::Machine;
use crate::remote_paste::{
    MAX_PASTE_BYTES, MAX_SEND_BYTES, paste_failure, paste_file_name, pasted_path, sent_file_name,
};
use crate::ssh_run::{self, Watch};
use std::path::Path;
use std::sync::atomic::Ordering;
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

/// Send the image file at `png` to `machine`, called `name` in words for the person.
pub fn upload_image(machine: &Machine, name: &str, png: &Path) -> Result<String, String> {
    transfer(
        machine,
        name,
        png,
        Kind::Image,
        &image_name(),
        &Watch::default(),
    )
}

/// Send the image file at `image` to `machine`, kept there as `image.<extension>` so a JPEG is
/// not named `.png`. `extension` is cleaned like any name. Like `send_file` it reports bytes sent
/// and can be cancelled through `watch`, but it has the image's cap and time limit (ADR 0049).
pub fn send_image(
    machine: &Machine,
    name: &str,
    image: &Path,
    extension: &str,
    watch: &Watch,
) -> Result<String, String> {
    let (stamp, salt) = stamp_and_salt();
    let remote_name = sent_file_name(stamp, salt, &format!("image.{extension}"));
    transfer(machine, name, image, Kind::Image, &remote_name, watch)
}

/// Send the file at `path` to `machine`, called `name` in words for the person. Anything but a
/// regular file is refused here, before ssh starts. `watch` reports bytes sent and can end it.
pub fn send_file(
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
pub fn shown(path: &Path) -> String {
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
    watch.total.store(bytes, Ordering::Relaxed);
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
fn image_name() -> String {
    let (stamp, salt) = stamp_and_salt();
    paste_file_name(stamp, salt)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MachineDef;

    fn box_machine() -> Machine {
        Machine::ssh("box", vec!["ssh-that-must-not-run".into()], "maitty")
    }

    #[test]
    fn a_folder_or_a_missing_path_is_refused_before_ssh_is_started() {
        let folder = send_file(
            &box_machine(),
            "box",
            &std::env::temp_dir(),
            &Watch::default(),
        );
        assert!(folder.unwrap_err().contains("is not a file"));
        let nope = std::env::temp_dir().join("remote-send-no-such-file");
        let missing = send_file(&box_machine(), "box", &nope, &Watch::default());
        assert!(
            missing
                .unwrap_err()
                .starts_with("could not read remote-send-no-such-file")
        );
    }

    #[test]
    fn a_file_over_the_cap_is_refused_and_never_truncated() {
        let big = std::env::temp_dir().join(format!("remote-send-big-{}", std::process::id()));
        std::fs::File::create(&big)
            .unwrap()
            .set_len(MAX_SEND_BYTES + 1)
            .unwrap();
        let error = send_file(&box_machine(), "box", &big, &Watch::default()).unwrap_err();
        let _ = std::fs::remove_file(&big);
        assert!(
            error.contains("more than the 100 MB that can be sent to box"),
            "{error}"
        );
    }

    #[test]
    fn an_image_over_its_cap_is_refused_though_a_file_that_size_may_go() {
        let big = std::env::temp_dir().join(format!("remote-send-image-{}", std::process::id()));
        std::fs::File::create(&big)
            .unwrap()
            .set_len(MAX_PASTE_BYTES + 1)
            .unwrap();
        let error = send_image(&box_machine(), "box", &big, "png", &Watch::default()).unwrap_err();
        let _ = std::fs::remove_file(&big);
        assert!(
            error.contains("more than the 25 MB that can be sent to box"),
            "{error}"
        );
    }

    #[test]
    fn a_machine_on_a_custom_transport_is_told_it_needs_ssh() {
        let def = MachineDef {
            name: "fly".into(),
            target: "fly".into(),
            via: Some(vec!["fly".into()]),
            ..Default::default()
        };
        let file = std::env::temp_dir().join(format!("remote-send-via-{}", std::process::id()));
        std::fs::write(&file, b"x").unwrap();
        let machine = Machine::from_def(&def, "ssh");
        let error = send_file(&machine, "fly", &file, &Watch::default()).unwrap_err();
        let _ = std::fs::remove_file(&file);
        assert!(
            error.starts_with("sending the file to fly needs ssh"),
            "{error}"
        );
    }
}
