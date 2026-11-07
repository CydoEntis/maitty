//! The checks on a pull request, and the pull itself: against a stand-in for ssh that does not
//! exist, and against the fake `ssh` shim in `remote-sh` mode, where the real get script reads a
//! file from a fake remote's home.

use super::*;
use maitty_test_support::fake_ssh;
use std::sync::Mutex;

fn folder(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("file-get-{label}-{}-{n}", std::process::id()));
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

fn plan_for(path: &str) -> Result<Job, String> {
    plan(
        &[def("build")],
        &plain_ssh(),
        "build",
        "t-1",
        path,
        Path::new("/home/me/Downloads"),
    )
}

fn has_sh() -> bool {
    std::process::Command::new("sh")
        .args(["-c", "command -v wc cat tr cut printf >/dev/null"])
        .stdin(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn events_of(job: &Job, cancel: &Arc<AtomicBool>) -> Vec<GetEvent> {
    let seen = Mutex::new(Vec::new());
    run(job, cancel, &|event| seen.lock().unwrap().push(event));
    seen.into_inner().unwrap()
}

fn is_end(event: &GetEvent) -> bool {
    !matches!(event, GetEvent::Progress { .. })
}

#[test]
fn a_pull_is_saved_in_downloads_under_the_app_and_the_machine() {
    let job = plan_for("~/out file.log").unwrap();
    assert_eq!(job.name, "build");
    assert_eq!(
        job.folder,
        Path::new("/home/me/Downloads")
            .join(maitty_machines::APP_COMMAND)
            .join("build")
    );
    assert_eq!(job.label(), "out file.log", "the name it will be saved under");
}

#[test]
fn a_request_that_cannot_be_right_is_refused_before_a_worker_starts() {
    let downloads = Path::new("/home/me/Downloads");
    let unknown = plan(&[def("build")], &plain_ssh(), "elsewhere", "t-1", "/x", downloads);
    assert_eq!(
        unknown.err().unwrap(),
        "elsewhere is not one of this computer's machines"
    );
    assert!(plan(&[def("build")], &plain_ssh(), "local", "t-1", "/x", downloads).is_err());
    let via = MachineDef {
        via: Some(vec!["fly".into()]),
        ..def("fly")
    };
    assert!(plan(&[via], &plain_ssh(), "fly", "t-1", "/x", downloads).is_err());
    for id in ["", "a b", "../x", &"a".repeat(65)] {
        assert!(
            plan(&[def("build")], &plain_ssh(), "build", id, "/x", downloads).is_err(),
            "{id:?}"
        );
    }
    for path in ["", "a\nb", "a\0b", &"a".repeat(4097)] {
        assert!(plan_for(path).is_err(), "{path:?}");
    }
}

#[test]
fn an_event_reaches_the_window_in_the_shape_it_reads() {
    let done = serde_json::to_value(GetEvent::Done {
        transfer: "t-1".into(),
        file: "out.log".into(),
        path: "/d/maitty/build/out.log".into(),
        folder: "/d/maitty/build".into(),
    })
    .unwrap();
    assert_eq!(
        done,
        serde_json::json!({
            "kind": "done",
            "transfer": "t-1",
            "file": "out.log",
            "path": "/d/maitty/build/out.log",
            "folder": "/d/maitty/build",
        })
    );
    let progress = serde_json::to_value(GetEvent::Progress {
        transfer: "t-1".into(),
        file: "out.log".into(),
        percent: 40,
    })
    .unwrap();
    assert_eq!(progress["kind"], "progress");
    assert_eq!(progress["percent"], 40);
}

#[test]
fn a_pull_that_cannot_start_ssh_ends_with_one_failed_event_that_says_why() {
    let mut job = plan_for("/x/out.log").unwrap();
    job.machine = Machine::ssh("build", vec!["ssh-that-must-not-run".into()], "maitty");
    let events = events_of(&job, &Arc::new(AtomicBool::new(false)));
    let ends: Vec<_> = events.iter().filter(|e| is_end(e)).collect();
    assert_eq!(ends.len(), 1, "{events:?}");
    match ends[0] {
        GetEvent::Failed { message, file, .. } => {
            assert!(message.contains("OpenSSH"), "{message}");
            assert_eq!(file, "out.log");
        }
        other => panic!("expected a failure, got {other:?}"),
    }
}

#[test]
fn a_cancel_ends_with_one_cancelled_event_and_saves_nothing() {
    let mut job = plan_for("/x/out.log").unwrap();
    job.folder = folder("cancel");
    job.machine = Machine::ssh("build", vec!["ssh-that-must-not-run".into()], "maitty");
    let events = events_of(&job, &Arc::new(AtomicBool::new(true)));
    let ends: Vec<_> = events.iter().filter(|e| is_end(e)).collect();
    assert!(matches!(ends[..], [GetEvent::Cancelled { .. }]), "{events:?}");
    assert!(
        std::fs::read_dir(&job.folder).map_or(true, |mut d| d.next().is_none()),
        "nothing was saved"
    );
    let _ = std::fs::remove_dir_all(&job.folder);
}

#[test]
fn a_file_is_pulled_from_a_fake_remote_and_the_window_is_told_where_it_is() {
    if !has_sh() {
        eprintln!("skipped: no POSIX sh with the get script's tools on this host");
        return;
    }
    let root = folder("remote");
    let home = root.join("home");
    std::fs::create_dir_all(&home).unwrap();
    std::fs::write(home.join("out file.log"), "line one\nline two\n").unwrap();
    let mut job = plan_for("~/out file.log").unwrap();
    job.folder = root.join("Downloads").join("app").join("build");
    job.machine = Machine::ssh(
        "build",
        vec![
            fake_ssh().to_string_lossy().into_owned(),
            "--mode=remote-sh".into(),
            format!("--home={}", home.display()),
        ],
        "maitty",
    );

    let events = events_of(&job, &Arc::new(AtomicBool::new(false)));

    let ends: Vec<_> = events.iter().filter(|e| is_end(e)).collect();
    assert_eq!(ends.len(), 1, "{events:?}");
    let GetEvent::Done { path, folder, file, .. } = ends[0] else {
        panic!("expected done, got {:?}", ends[0]);
    };
    assert_eq!(file, "out file.log");
    assert_eq!(Path::new(folder), job.folder);
    assert_eq!(
        std::fs::read_to_string(path).unwrap().replace("\r\n", "\n"),
        "line one\nline two\n"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn only_a_folder_inside_the_apps_downloads_folder_can_be_opened() {
    let downloads = Path::new("/home/me/Downloads");
    let app = downloads.join(maitty_machines::APP_COMMAND);
    assert!(inside_pulls(&app, downloads));
    assert!(inside_pulls(&app.join("build"), downloads));
    assert!(!inside_pulls(downloads, downloads), "Downloads itself is not one of ours");
    assert!(!inside_pulls(Path::new("/etc"), downloads));
    assert!(!inside_pulls(&app.join("..").join(".."), downloads), "no climbing out");
    assert!(!inside_pulls(&downloads.join("other").join("build"), downloads));
}

#[test]
fn a_folder_is_pulled_through_the_same_command_and_the_window_is_told_where_it_is() {
    if !has_sh() {
        eprintln!("skipped: no POSIX sh with the get script's tools on this host");
        return;
    }
    let root = folder("remote-folder");
    let home = root.join("home");
    for file in ["app/README.md", "app/src/lib.rs"] {
        let path = home.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("contents of {file}")).unwrap();
    }
    let mut job = plan_for("~/app").unwrap();
    job.folder = root.join("Downloads").join("app").join("build");
    job.machine = Machine::ssh(
        "build",
        vec![
            fake_ssh().to_string_lossy().into_owned(),
            "--mode=remote-sh".into(),
            format!("--home={}", home.display()),
        ],
        "maitty",
    );

    let events = events_of(&job, &Arc::new(AtomicBool::new(false)));

    let ends: Vec<_> = events.iter().filter(|e| is_end(e)).collect();
    assert_eq!(ends.len(), 1, "{events:?}");
    let GetEvent::Done { path, file, .. } = ends[0] else {
        panic!("expected done, got {:?}", ends[0]);
    };
    assert_eq!(file, "app");
    assert_eq!(
        std::fs::read_to_string(Path::new(path).join("src").join("lib.rs")).unwrap(),
        "contents of app/src/lib.rs"
    );
    let _ = std::fs::remove_dir_all(&root);
}
