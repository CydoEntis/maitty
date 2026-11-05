//! The checks on a pasted image, what is kept on this computer, and the send through the fake
//! `ssh` shim, where the real paste script keeps the image in a fake remote's home.

use super::*;
use crate::file_send::discard_staged;
#[cfg(unix)]
use crate::file_send::{SendEvent, run};
#[cfg(unix)]
use maitty_machines::PASTE_MARKER;
#[cfg(unix)]
use maitty_test_support::fake_ssh;
#[cfg(unix)]
use std::sync::atomic::AtomicBool;
use std::sync::atomic::{AtomicU32, Ordering};
#[cfg(unix)]
use std::sync::{Arc, Mutex};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDRrest-of-the-image";

fn folder(label: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("image-paste-{label}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make a test folder");
    dir
}

fn def(name: &str) -> MachineDef {
    MachineDef {
        name: name.into(),
        target: format!("{name}.example"),
        ..MachineDef::default()
    }
}

fn plain_ssh() -> Vec<String> {
    vec!["ssh".into()]
}

fn up(_: &str) -> bool {
    true
}

fn plan_with(
    cache: &Path,
    defs: &[MachineDef],
    bytes: &[u8],
    connected: &dyn Fn(&str) -> bool,
) -> Result<Job, String> {
    plan_image(cache, defs, &plain_ssh(), "build", "t-1", bytes, connected)
}

fn files_in(dir: &Path) -> Vec<String> {
    std::fs::read_dir(dir)
        .map(|entries| {
            entries
                .flatten()
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default()
}

#[test]
fn an_image_is_known_by_its_first_bytes_not_by_what_the_window_called_it() {
    assert_eq!(check_image(PNG), Ok(ImageType::Png));
    assert_eq!(check_image(b"\xFF\xD8\xFF\xE0jfif"), Ok(ImageType::Jpeg));
    assert_eq!(check_image(b"GIF89a....."), Ok(ImageType::Gif));
    assert_eq!(check_image(b"GIF87a....."), Ok(ImageType::Gif));
    assert_eq!(check_image(b"RIFF\x10\0\0\0WEBPVP8 "), Ok(ImageType::Webp));
    for bad in [
        &b"%PDF-1.7 not an image"[..],
        b"#!/bin/sh\nrm -rf ~\n",
        b"RIFF\x10\0\0\0WAVEfmt ",
        b"\x89PNG",
        b"<svg xmlns='http://www.w3.org/2000/svg'/>",
    ] {
        let error = check_image(bad).unwrap_err();
        assert!(error.contains("not a PNG, JPEG, GIF or WebP"), "{error}");
    }
    assert!(check_image(b"").unwrap_err().contains("empty"));
}

#[test]
fn an_image_over_twenty_five_megabytes_is_refused_not_cut() {
    let mut big = PNG.to_vec();
    big.resize(usize::try_from(MAX_PASTE_BYTES).unwrap(), 0);
    assert_eq!(check_image(&big), Ok(ImageType::Png), "at the cap is fine");
    big.push(0);
    let error = check_image(&big).unwrap_err();
    assert!(error.contains("more than the 25 MB"), "{error}");
}

#[test]
fn an_image_for_this_computer_is_kept_whole_and_private_and_named_by_its_type() {
    let cache = folder("local").join("pastes");
    let path = save_local(&cache, PNG).unwrap();
    assert_eq!(std::fs::read(&path).unwrap(), PNG);
    let name = path.file_name().unwrap().to_string_lossy().into_owned();
    assert!(
        name.starts_with("paste-") && name.ends_with(".png"),
        "{name}"
    );
    let other = save_local(&cache, b"\xFF\xD8\xFFx").unwrap();
    assert!(other.extension().is_some_and(|e| e == "jpg"));
    assert_ne!(path, other, "two pastes are two files");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = |p: &Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode(&path), 0o600);
        assert_eq!(mode(&cache), 0o700);
    }
}

#[test]
fn nothing_is_kept_for_data_that_is_not_an_image() {
    let cache = folder("local-bad").join("pastes");
    assert!(save_local(&cache, b"not an image").is_err());
    assert!(files_in(&cache).is_empty());
}

fn age(path: &Path, by: Duration) {
    std::fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_modified(SystemTime::now() - by)
        .unwrap();
}

#[test]
fn only_this_apps_old_files_are_swept() {
    let cache = folder("sweep");
    let old = write_private(&cache, "paste-1-0000000a.png", PNG).unwrap();
    let staged = write_private(&cache, "staged-1-0000000a.png", PNG).unwrap();
    let fresh = write_private(&cache, "paste-2-0000000b.png", PNG).unwrap();
    let foreign = write_private(&cache, "notes.txt", b"mine").unwrap();
    age(&old, KEEP + Duration::from_secs(60));
    age(&foreign, KEEP + Duration::from_secs(60));
    age(&staged, KEEP_STAGED + Duration::from_secs(60));
    sweep(&cache, SystemTime::now());
    assert!(!old.exists(), "a week-old paste goes");
    assert!(!staged.exists(), "a day-old leftover goes");
    assert!(fresh.exists());
    assert!(
        foreign.exists(),
        "a file that is not ours stays, however old"
    );
}

#[test]
fn a_refused_paste_stages_nothing() {
    let cache = folder("refused");
    let via = MachineDef {
        via: Some(vec!["fly".into()]),
        ..def("build")
    };
    let cases: [(&str, Result<Job, String>); 4] = [
        (
            "signature",
            plan_with(&cache, &[def("build")], b"nope", &up),
        ),
        ("unknown", plan_with(&cache, &[def("other")], PNG, &up)),
        ("custom transport", plan_with(&cache, &[via], PNG, &up)),
        (
            "not connected",
            plan_with(&cache, &[def("build")], PNG, &|_| false),
        ),
    ];
    for (why, result) in cases {
        assert!(result.is_err(), "{why}");
    }
    assert!(files_in(&cache).is_empty(), "{:?}", files_in(&cache));
}

#[test]
fn a_custom_transport_is_refused_in_the_words_files_get() {
    let via = MachineDef {
        via: Some(vec!["fly".into()]),
        ..def("build")
    };
    let error = plan_with(&folder("via"), &[via], PNG, &up).err().unwrap();
    assert!(
        error.starts_with("sending files to build needs ssh"),
        "{error}"
    );
}

#[test]
fn a_machine_that_is_not_connected_says_so_by_name() {
    let error = plan_with(&folder("down"), &[def("Build")], PNG, &|_| false)
        .err()
        .unwrap();
    assert_eq!(
        error,
        "Build is not connected; reconnect before sending an image to it"
    );
}

#[test]
fn an_id_that_could_be_anything_is_refused_before_anything_is_written() {
    let cache = folder("id");
    for id in ["", "a b", "a/b", &"x".repeat(65)] {
        let result = plan_image(&cache, &[def("build")], &plain_ssh(), "build", id, PNG, &up);
        assert!(result.is_err(), "{id:?}");
    }
    assert!(files_in(&cache).is_empty());
}

#[test]
fn a_planned_image_is_one_staged_private_copy_the_window_calls_image() {
    let cache = folder("plan");
    let job = plan_with(&cache, &[def("Build")], PNG, &up).unwrap();
    assert_eq!(job.name, "Build");
    assert_eq!(job.kind, SendKind::Image { extension: "png" });
    assert_eq!(job.files.len(), 1);
    assert_eq!(std::fs::read(&job.files[0]).unwrap(), PNG);
    discard_staged(&job);
    assert!(files_in(&cache).is_empty(), "the staged copy is removed");
}

#[cfg(unix)]
fn scripted(cache: &Path, script: &str) -> Job {
    use std::os::unix::fs::PermissionsExt;
    let program = folder("ssh").join("ssh");
    std::fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    plan_image(
        cache,
        &[def("build")],
        &[program.display().to_string()],
        "build",
        "t-1",
        PNG,
        &up,
    )
    .unwrap()
}

#[cfg(unix)]
fn collect(job: &Job, cancel: &Arc<AtomicBool>) -> Vec<SendEvent> {
    let events = Mutex::new(Vec::new());
    run(job, cancel, &|event| events.lock().unwrap().push(event));
    events.into_inner().unwrap()
}

#[cfg(unix)]
fn says(path: &str) -> String {
    format!("cat >/dev/null; printf '%s\\n%s\\n' '{PASTE_MARKER}' '{path}'")
}

#[test]
#[cfg(unix)]
fn the_far_side_path_is_reported_for_an_image_named_image() {
    let job = scripted(&folder("ok"), &says("/home/me/.cache/pastes/p.png"));
    let events = collect(&job, &Arc::new(AtomicBool::new(false)));
    assert!(events.contains(&SendEvent::Sent {
        transfer: "t-1".into(),
        file: "image".into(),
        index: 0,
        count: 1,
        remote_path: "/home/me/.cache/pastes/p.png".into(),
        bytes: PNG.len() as u64,
    }));
    assert_eq!(
        events.last(),
        Some(&SendEvent::Done {
            transfer: "t-1".into(),
            sent: 1,
            count: 1
        })
    );
}

#[test]
#[cfg(unix)]
fn a_path_the_far_side_makes_up_is_never_reported_as_kept() {
    for lie in [
        "relative/p.png",
        "~/p.png",
        "/a/\u{1b}[31m/p.png",
        "/a/\u{202e}/p.png",
    ] {
        let job = scripted(&folder("lie"), &says(lie));
        let events = collect(&job, &Arc::new(AtomicBool::new(false)));
        assert!(
            events.iter().all(|e| !matches!(e, SendEvent::Sent { .. })),
            "{lie:?}: {events:?}"
        );
        assert!(matches!(events.last(), Some(SendEvent::Failed { file, .. }) if file == "image"));
    }
}

#[test]
#[cfg(unix)]
fn a_cancel_ends_the_image_going() {
    let job = scripted(&folder("cancel"), "exec sleep 30");
    let cancel = Arc::new(AtomicBool::new(false));
    let stopper = Arc::clone(&cancel);
    let stop = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        stopper.store(true, Ordering::Relaxed);
    });
    let started = std::time::Instant::now();
    let events = collect(&job, &cancel);
    stop.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(
        events.last(),
        Some(&SendEvent::Cancelled {
            transfer: "t-1".into(),
            sent: 0
        })
    );
}

#[cfg(unix)]
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

#[test]
#[cfg(unix)]
fn through_the_shim_the_image_lands_whole_and_private_in_the_far_homes_cache() {
    if !sh_has_the_tools() {
        eprintln!("skipped: no POSIX sh with the paste script's tools on this host");
        return;
    }
    let dir = folder("shim");
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let ssh = vec![
        fake_ssh().display().to_string(),
        "--mode=remote-sh".into(),
        format!("--home={}", home.display()),
    ];
    let job = plan_image(
        &dir.join("cache"),
        &[def("build")],
        &ssh,
        "build",
        "t-1",
        b"\xFF\xD8\xFFa jpeg",
        &up,
    )
    .unwrap();
    let events = collect(&job, &Arc::new(AtomicBool::new(false)));
    let kept = events
        .iter()
        .find_map(|e| match e {
            SendEvent::Sent { remote_path, .. } => Some(remote_path.clone()),
            _ => None,
        })
        .unwrap_or_else(|| panic!("{events:?}"));
    assert_eq!(std::fs::read(&kept).unwrap(), b"\xFF\xD8\xFFa jpeg");
    assert!(kept.ends_with("-image.jpg"), "{kept}");
    assert!(kept.starts_with(&*home.display().to_string()), "{kept}");
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(&kept).unwrap().permissions().mode() & 0o777;
        assert_eq!(mode, 0o600);
    }
    discard_staged(&job);
    assert!(files_in(&dir.join("cache")).is_empty());
}
