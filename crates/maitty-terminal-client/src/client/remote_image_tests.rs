//! Sending a pasted image to another machine, against the fake ssh shim in `remote-sh` mode:
//! the real paste script runs on the host's POSIX `sh` with `HOME` a temporary folder, so the
//! file really lands in a fake remote's cache. A host with no usable `sh` skips these and says so.

use super::remote_image::{send_file, upload};
use super::remote_status_tests::built;
use crate::brand::APP_COMMAND;
use crate::cli::machine_update::ssh_run::Watch;
use crate::ipc::Machine;
use crate::remote_paste::MAX_PASTE_BYTES;
use std::path::PathBuf;

fn sh_has_the_tools() -> bool {
    std::process::Command::new("sh")
        .args([
            "-c",
            "command -v wc mv mktemp cat tr printf rm mkdir find >/dev/null",
        ])
        .stdin(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

macro_rules! needs_sh {
    () => {
        if !sh_has_the_tools() {
            eprintln!("skipped: no POSIX sh with the paste script's tools on this host");
            return;
        }
    };
}

/// A fake remote: its home folder, a local image to send, and what the shim was asked.
struct Remote {
    dir: PathBuf,
    home: PathBuf,
    log: PathBuf,
}

impl Remote {
    fn new() -> Remote {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("remote-image-{}-{n}", std::process::id()));
        let home = dir.join("home");
        std::fs::create_dir_all(&home).expect("make the fake home");
        Remote {
            log: dir.join("remote-commands.txt"),
            dir,
            home,
        }
    }

    fn machine(&self, mode: &str) -> Machine {
        let (_, shim) = built();
        Machine::ssh(
            "box",
            vec![
                shim.to_string_lossy().into_owned(),
                format!("--mode={mode}"),
                format!("--home={}", self.home.display()),
                format!("--argv-log={}", self.log.display()),
            ],
            APP_COMMAND,
        )
    }

    fn image(&self, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join("clipboard.png");
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn pastes(&self) -> Vec<PathBuf> {
        let dir = self.home.join(".cache").join(APP_COMMAND).join("pastes");
        std::fs::read_dir(dir)
            .map(|rd| rd.flatten().map(|e| e.path()).collect())
            .unwrap_or_default()
    }

    fn commands(&self) -> usize {
        std::fs::read_to_string(&self.log).map_or(0, |text| text.lines().count())
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

#[test]
fn the_image_arrives_whole_on_the_far_side_and_its_path_there_comes_back() {
    needs_sh!();
    let remote = Remote::new();
    let image = remote.image(b"\x89PNG\r\n\x1a\nrest of the picture");
    let path = upload(&remote.machine("remote-sh"), "box", &image).expect("it is sent");
    let kept = remote.pastes();
    assert_eq!(kept.len(), 1, "one file kept: {kept:?}");
    // The far side's shell writes its own kind of path (`/tmp/...` where Git's `sh` runs on
    // Windows), so only the file's name is compared, not the folder it is spelled under.
    assert_eq!(
        std::path::Path::new(&path).file_name(),
        kept[0].file_name(),
        "the path named is the file kept"
    );
    assert!(path.starts_with('/'), "{path}");
    assert!(path.ends_with(".png"));
    assert_eq!(
        std::fs::read(&kept[0]).unwrap(),
        b"\x89PNG\r\n\x1a\nrest of the picture"
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert!(
            path.starts_with(&remote.home.display().to_string()),
            "{path}"
        );
        let mode = std::fs::metadata(&kept[0]).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600, "readable by its owner alone");
    }
}

#[test]
fn two_pastes_are_two_files() {
    needs_sh!();
    let remote = Remote::new();
    let image = remote.image(b"png");
    let first = upload(&remote.machine("remote-sh"), "box", &image).unwrap();
    let second = upload(&remote.machine("remote-sh"), "box", &image).unwrap();
    assert_ne!(first, second);
    assert_eq!(remote.pastes().len(), 2);
}

#[test]
fn an_image_too_big_to_send_is_refused_before_ssh_is_started() {
    let remote = Remote::new();
    let image = remote.dir.join("big.png");
    let file = std::fs::File::create(&image).unwrap();
    file.set_len(MAX_PASTE_BYTES + 1).unwrap();
    let error = upload(&remote.machine("remote-sh"), "box", &image).unwrap_err();
    assert!(
        error.contains("more than the 25 MB that can be sent to box"),
        "{error}"
    );
    assert_eq!(remote.commands(), 0, "no ssh was started");
}

#[test]
fn a_machine_reached_by_a_custom_transport_is_told_it_needs_ssh() {
    let remote = Remote::new();
    let image = remote.image(b"png");
    let (_, shim) = built();
    let def = maitty_settings::config::MachineDef {
        name: "fly".into(),
        target: "fly".into(),
        via: Some(vec![shim.to_string_lossy().into_owned()]),
        ..Default::default()
    };
    let machine = Machine::from_def(&def, "ssh");
    let error = upload(&machine, "fly", &image).unwrap_err();
    assert!(
        error.starts_with("sending the image to fly needs ssh"),
        "{error}"
    );
}

#[test]
fn a_login_that_fails_says_so_and_sends_nothing() {
    needs_sh!();
    let remote = Remote::new();
    let image = remote.image(b"png");
    let error = upload(&remote.machine("auth-fail"), "box", &image).unwrap_err();
    assert!(
        error.starts_with("could not send the image to box: ")
            && error.contains("Permission denied"),
        "{error}"
    );
    assert!(remote.pastes().is_empty());
}

// Sending any file (ADR 0050).

impl Remote {
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
}

#[test]
fn a_file_arrives_whole_under_a_plain_name_that_ends_like_the_original() {
    needs_sh!();
    let remote = Remote::new();
    let file = remote.file("my report (final).txt", b"line one\nline two\n");
    let watch = Watch::default();
    let path = send_file(&remote.machine("remote-sh"), "box", &file, &watch).expect("it is sent");
    let kept = remote.pastes();
    assert_eq!(kept.len(), 1, "{kept:?}");
    assert_eq!(std::fs::read(&kept[0]).unwrap(), b"line one\nline two\n");
    assert_eq!(std::path::Path::new(&path).file_name(), kept[0].file_name());
    assert!(path.ends_with("-my_report__final_.txt"), "{path}");
    assert_eq!(
        watch.total.load(std::sync::atomic::Ordering::Relaxed),
        18,
        "the size was reported"
    );
    assert_eq!(watch.sent.load(std::sync::atomic::Ordering::Relaxed), 18);
}

#[test]
fn two_files_with_the_same_name_are_two_files() {
    needs_sh!();
    let remote = Remote::new();
    let file = remote.file("a.log", b"x");
    let watch = Watch::default();
    let first = send_file(&remote.machine("remote-sh"), "box", &file, &watch).unwrap();
    let second = send_file(&remote.machine("remote-sh"), "box", &file, &watch).unwrap();
    assert_ne!(first, second);
    assert_eq!(remote.pastes().len(), 2);
}

#[test]
fn a_folder_or_a_missing_path_is_refused_before_ssh_is_started() {
    let remote = Remote::new();
    let watch = Watch::default();
    let machine = remote.machine("remote-sh");
    let folder = send_file(&machine, "box", &remote.dir, &watch).unwrap_err();
    assert!(folder.contains("is not a file"), "{folder}");
    let missing = send_file(&machine, "box", &remote.dir.join("nope"), &watch).unwrap_err();
    assert!(missing.starts_with("could not read nope"), "{missing}");
    assert_eq!(remote.commands(), 0);
}

#[test]
fn a_file_over_the_cap_is_refused_before_ssh_is_started() {
    let remote = Remote::new();
    let big = remote.dir.join("big.bin");
    let file = std::fs::File::create(&big).unwrap();
    file.set_len(crate::remote_paste::MAX_SEND_BYTES + 1)
        .unwrap();
    let error =
        send_file(&remote.machine("remote-sh"), "box", &big, &Watch::default()).unwrap_err();
    assert!(
        error.contains("more than the 100 MB that can be sent to box"),
        "{error}"
    );
    assert_eq!(remote.commands(), 0);
}

#[test]
fn a_file_sent_to_a_machine_on_a_custom_transport_is_told_it_needs_ssh() {
    let remote = Remote::new();
    let file = remote.file("a.txt", b"x");
    let (_, shim) = built();
    let def = maitty_settings::config::MachineDef {
        name: "fly".into(),
        target: "fly".into(),
        via: Some(vec![shim.to_string_lossy().into_owned()]),
        ..Default::default()
    };
    let machine = Machine::from_def(&def, "ssh");
    let error = send_file(&machine, "fly", &file, &Watch::default()).unwrap_err();
    assert!(
        error.starts_with("sending the file to fly needs ssh"),
        "{error}"
    );
}

#[test]
fn a_send_that_is_stopped_ends_at_once_and_keeps_nothing() {
    needs_sh!();
    let remote = Remote::new();
    // Bigger than a pipe holds, so the sender is still writing when the stop comes.
    let file = remote.file("big.bin", &vec![7u8; 4 * 1024 * 1024]);
    let watch = Watch::default();
    let stopper = watch.clone();
    let thread = std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_millis(300));
        stopper
            .cancel
            .store(true, std::sync::atomic::Ordering::Relaxed);
    });
    let started = std::time::Instant::now();
    let error = send_file(&remote.machine("hang"), "box", &file, &watch).unwrap_err();
    thread.join().unwrap();
    assert_eq!(error, "stopped sending the file to box");
    assert!(started.elapsed() < std::time::Duration::from_secs(10));
    assert!(remote.pastes().is_empty());
}

#[test]
fn a_login_that_fails_while_sending_a_file_says_so() {
    needs_sh!();
    let remote = Remote::new();
    let file = remote.file("a.txt", b"x");
    let error = send_file(
        &remote.machine("auth-fail"),
        "box",
        &file,
        &Watch::default(),
    )
    .unwrap_err();
    assert!(
        error.starts_with("could not send the file to box: ")
            && error.contains("Permission denied"),
        "{error}"
    );
}
