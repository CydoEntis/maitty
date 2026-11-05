//! The checks on a drop, and the sends themselves: against a stand-in for ssh that is a `sh` line
//! (so no ssh is needed), and against the fake `ssh` shim in `remote-sh` mode, where the real
//! paste script keeps the file in a fake remote's home.

use super::*;
#[cfg(unix)]
use maitty_machines::PASTE_MARKER;
#[cfg(unix)]
use maitty_test_support::fake_ssh;
#[cfg(unix)]
use std::sync::Mutex;

fn folder(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("file-send-{label}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make a test folder");
    dir
}

fn file(dir: &std::path::Path, name: &str, bytes: &[u8]) -> String {
    let path = dir.join(name);
    std::fs::write(&path, bytes).unwrap();
    path.display().to_string()
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

fn plan_for(paths: &[String]) -> Result<Job, String> {
    plan(&[def("build")], &plain_ssh(), "build", "t-1", paths)
}

fn error_of(paths: &[String]) -> String {
    plan_for(paths).err().expect("the plan is refused")
}

#[test]
fn a_regular_file_on_a_known_ssh_machine_is_planned_in_order() {
    let dir = folder("plan");
    let paths = [file(&dir, "a.log", b"a"), file(&dir, "b.log", b"b")];
    let job = plan(&[def("Build")], &plain_ssh(), "build", "t-1", &paths).unwrap();
    assert_eq!(job.name, "Build", "the machine's own spelling");
    assert_eq!(
        job.files,
        [PathBuf::from(&paths[0]), PathBuf::from(&paths[1])]
    );
}

#[test]
fn a_machine_this_computer_does_not_know_gets_nothing() {
    let dir = folder("unknown");
    let paths = [file(&dir, "a.log", b"a")];
    let error = plan(&[def("build")], &plain_ssh(), "elsewhere", "t-1", &paths)
        .err()
        .unwrap();
    assert_eq!(error, "elsewhere is not one of this computer's machines");
    let local = plan(&[def("build")], &plain_ssh(), "local", "t-1", &paths);
    assert!(local.is_err(), "this computer is not a place to send to");
}

#[test]
fn a_machine_on_a_custom_transport_is_refused() {
    let dir = folder("via");
    let paths = [file(&dir, "a.log", b"a")];
    let via = MachineDef {
        via: Some(vec!["fly".into()]),
        ..def("fly")
    };
    let error = plan(&[via], &plain_ssh(), "fly", "t-1", &paths)
        .err()
        .unwrap();
    assert!(
        error.starts_with("sending files to fly needs ssh"),
        "{error}"
    );
}

#[test]
fn what_is_not_one_regular_file_is_refused_before_anything_is_sent() {
    let dir = folder("kinds");
    let good = file(&dir, "a.log", b"a");
    assert!(error_of(&[]).contains("no file"));
    assert!(error_of(&[good.clone(), dir.display().to_string()]).contains("is not a file"));
    let missing = dir.join("nope").display().to_string();
    assert!(error_of(&[good.clone(), missing]).starts_with("could not read nope"));
    assert!(error_of(&["relative/a.log".into()]).contains("full path"));
    assert!(error_of(&[String::new()]).contains("full path"));
    assert!(error_of(&[format!("{good}\0x")]).contains("full path"));
}

#[test]
fn a_file_over_the_cap_is_refused_whole() {
    let dir = folder("big");
    let big = dir.join("big.bin");
    std::fs::File::create(&big)
        .unwrap()
        .set_len(MAX_SEND_BYTES + 1)
        .unwrap();
    let error = error_of(&[big.display().to_string()]);
    assert!(error.contains("more than the 100 MB"), "{error}");
}

#[test]
fn more_than_ten_files_are_refused_not_cut() {
    let dir = folder("many");
    let ten: Vec<String> = (0..MOST_FILES)
        .map(|n| file(&dir, &format!("f{n}"), b"x"))
        .collect();
    assert!(plan_for(&ten).is_ok());
    let eleven: Vec<String> = ten.into_iter().chain([file(&dir, "last", b"x")]).collect();
    let error = error_of(&eleven);
    assert_eq!(
        error,
        "at most 10 files can be sent at a time; 11 were dropped"
    );
}

#[test]
fn an_id_that_could_be_anything_is_refused() {
    let dir = folder("id");
    let paths = [file(&dir, "a.log", b"a")];
    for id in [
        "",
        "has space",
        "a/b",
        "quote\"",
        &"x".repeat(TRANSFER_ID_MAX + 1),
    ] {
        let refused = plan(&[def("build")], &plain_ssh(), "build", id, &paths);
        assert!(refused.is_err(), "{id:?}");
    }
}

/// A job whose "ssh" is a small `sh` program running `script`: it gets ssh's arguments and the
/// file on stdin, and says what it likes.
#[cfg(unix)]
fn scripted(files: Vec<PathBuf>, script: &str) -> Job {
    use std::os::unix::fs::PermissionsExt;
    let program = folder("ssh").join("ssh");
    std::fs::write(&program, format!("#!/bin/sh\n{script}\n")).unwrap();
    std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
    Job {
        transfer: "t-1".into(),
        machine: Machine::ssh("box", vec![program.display().to_string()], "maitty"),
        name: "box".into(),
        files,
        kind: SendKind::Files,
    }
}

#[cfg(unix)]
fn collect(job: &Job, cancel: &Arc<AtomicBool>) -> Vec<SendEvent> {
    let events = Mutex::new(Vec::new());
    run(job, cancel, &|event| events.lock().unwrap().push(event));
    events.into_inner().unwrap()
}

#[cfg(unix)]
fn no_cancel() -> Arc<AtomicBool> {
    Arc::new(AtomicBool::new(false))
}

#[cfg(unix)]
fn says(path: &str) -> String {
    format!("cat >/dev/null; printf '%s\\n%s\\n' '{PASTE_MARKER}' '{path}'")
}

#[test]
#[cfg(unix)]
fn files_are_sent_in_turn_and_each_far_side_path_is_reported() {
    let dir = folder("turn");
    let files = vec![
        PathBuf::from(file(&dir, "a.log", b"a")),
        PathBuf::from(file(&dir, "b.log", b"b")),
    ];
    let events = collect(
        &scripted(files, &says("/home/me/.cache/pastes/p.log")),
        &no_cancel(),
    );
    let sent: Vec<_> = events
        .iter()
        .filter_map(|e| match e {
            SendEvent::Sent {
                file,
                index,
                remote_path,
                ..
            } => Some((file.as_str(), *index, remote_path.as_str())),
            _ => None,
        })
        .collect();
    assert_eq!(
        sent,
        [
            ("a.log", 0, "/home/me/.cache/pastes/p.log"),
            ("b.log", 1, "/home/me/.cache/pastes/p.log")
        ]
    );
    assert_eq!(
        events.last(),
        Some(&SendEvent::Done {
            transfer: "t-1".into(),
            sent: 2,
            count: 2
        })
    );
}

#[test]
#[cfg(unix)]
fn a_failure_ends_the_send_and_says_what_ssh_said() {
    let dir = folder("fail");
    let files = vec![
        PathBuf::from(file(&dir, "a.log", b"a")),
        PathBuf::from(file(&dir, "b.log", b"b")),
    ];
    let events = collect(
        &scripted(
            files,
            "cat >/dev/null; echo 'Permission denied (publickey).' >&2; exit 255",
        ),
        &no_cancel(),
    );
    assert_eq!(
        events,
        [SendEvent::Failed {
            transfer: "t-1".into(),
            file: "a.log".into(),
            message: "could not send the file to box: Permission denied (publickey).".into(),
            unsent: 2,
        }]
    );
}

#[test]
#[cfg(unix)]
fn a_path_the_far_side_makes_up_is_never_reported_as_kept() {
    let dir = folder("lie");
    for lie in [
        "relative/p.png",
        "~/p.png",
        "/a/\u{1b}[31m/p.png",
        "/a/\u{202e}/p.png",
    ] {
        let files = vec![PathBuf::from(file(&dir, "a.log", b"a"))];
        let events = collect(&scripted(files, &says(lie)), &no_cancel());
        assert!(
            events.iter().all(|e| !matches!(e, SendEvent::Sent { .. })),
            "{lie:?}: {events:?}"
        );
        assert!(
            matches!(events.last(), Some(SendEvent::Failed { .. })),
            "{lie:?}"
        );
    }
}

#[test]
#[cfg(unix)]
fn a_cancel_ends_the_file_going_and_tries_no_other() {
    let dir = folder("cancel");
    let files = vec![
        PathBuf::from(file(&dir, "a.log", b"a")),
        PathBuf::from(file(&dir, "b.log", b"b")),
    ];
    let cancel = no_cancel();
    let stopper = Arc::clone(&cancel);
    let stop = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(300));
        stopper.store(true, Ordering::Relaxed);
    });
    let started = std::time::Instant::now();
    // `exec`: a real ssh is one process, so killing it closes its pipes; a shell that kept a
    // `sleep` child would hold them open for the full thirty seconds.
    let events = collect(&scripted(files, "exec sleep 30"), &cancel);
    stop.join().unwrap();
    assert!(started.elapsed() < Duration::from_secs(10));
    assert_eq!(
        events.last(),
        Some(&SendEvent::Cancelled {
            transfer: "t-1".into(),
            sent: 0
        })
    );
    assert!(events.iter().all(|e| !matches!(e, SendEvent::Sent { .. })));
}

#[test]
#[cfg(unix)]
fn a_send_cancelled_before_it_starts_sends_nothing() {
    let dir = folder("precancel");
    let files = vec![PathBuf::from(file(&dir, "a.log", b"a"))];
    let cancel = Arc::new(AtomicBool::new(true));
    let events = collect(&scripted(files, &says("/p")), &cancel);
    assert_eq!(
        events,
        [SendEvent::Cancelled {
            transfer: "t-1".into(),
            sent: 0
        }]
    );
}

#[test]
#[cfg(unix)]
fn the_window_is_told_how_far_a_slow_send_has_got() {
    let dir = folder("progress");
    let body = vec![1u8; 300_000];
    let files = vec![PathBuf::from(file(&dir, "big.bin", &body))];
    // The line reads slowly enough for a progress tick to land before it ends.
    let script = format!("sleep 1; {}", says("/p"));
    let events = collect(&scripted(files, &script), &no_cancel());
    let percents: Vec<u8> = events
        .iter()
        .filter_map(|e| match e {
            SendEvent::Progress { percent, file, .. } => {
                assert_eq!(file, "big.bin");
                Some(*percent)
            }
            _ => None,
        })
        .collect();
    assert!(!percents.is_empty(), "{events:?}");
    assert!(percents.iter().all(|p| *p <= 100));
}

#[test]
fn only_one_send_runs_at_a_time_and_a_cancel_reaches_the_right_one() {
    let transfers = Transfers::default();
    let first = transfers.begin("one").unwrap();
    assert!(transfers.begin("two").is_err());
    transfers.cancel("other");
    assert!(!first.load(Ordering::Relaxed), "another id cancels nothing");
    transfers.cancel("one");
    assert!(first.load(Ordering::Relaxed));
    transfers.end("one");
    assert!(transfers.begin("two").is_ok());
}

#[test]
fn an_event_reaches_the_window_in_the_shape_it_reads() {
    let json = serde_json::to_value(SendEvent::Sent {
        transfer: "t-1".into(),
        file: "a.log".into(),
        index: 0,
        count: 2,
        remote_path: "/p".into(),
    })
    .unwrap();
    assert_eq!(
        json,
        serde_json::json!({
            "kind": "sent", "transfer": "t-1", "file": "a.log",
            "index": 0, "count": 2, "remotePath": "/p"
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
fn through_the_shim_the_files_really_land_privately_in_the_far_homes_cache() {
    if !sh_has_the_tools() {
        eprintln!("skipped: no POSIX sh with the paste script's tools on this host");
        return;
    }
    let dir = folder("shim");
    let home = dir.join("home");
    std::fs::create_dir_all(&home).unwrap();
    let paths = [
        file(&dir, "report one.pdf", b"first"),
        file(&dir, "two.log", b"second"),
    ];
    let ssh = vec![
        fake_ssh().display().to_string(),
        "--mode=remote-sh".into(),
        format!("--home={}", home.display()),
    ];
    let job = plan(&[def("build")], &ssh, "build", "t-1", &paths).unwrap();
    let events = collect(&job, &no_cancel());
    let kept: Vec<&str> = events
        .iter()
        .filter_map(|e| match e {
            SendEvent::Sent { remote_path, .. } => Some(remote_path.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(kept.len(), 2, "{events:?}");
    assert_eq!(std::fs::read(kept[0]).unwrap(), b"first");
    assert_eq!(std::fs::read(kept[1]).unwrap(), b"second");
    assert!(kept[0].ends_with("-report_one.pdf"), "{}", kept[0]);
    assert!(matches!(
        events.last(),
        Some(SendEvent::Done { sent: 2, .. })
    ));
}
