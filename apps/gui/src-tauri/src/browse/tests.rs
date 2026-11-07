//! The checks on a browse request, and the answers: against the fake `ssh` shim in `remote-sh`
//! mode, where the real list and index scripts read a fake remote's home.

use super::*;
use maitty_test_support::fake_ssh;

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

fn has_sh() -> bool {
    std::process::Command::new("sh")
        .args(["-c", "command -v find head cut printf cd pwd >/dev/null"])
        .stdin(std::process::Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

#[test]
fn a_request_for_a_known_ssh_machine_and_a_clean_path_is_planned() {
    let (_, name) = plan(&[def("Build")], &plain_ssh(), "build", "~/app").unwrap();
    assert_eq!(name, "Build", "the machine's own spelling");
    assert!(plan(&[def("build")], &plain_ssh(), "build", "").is_ok(), "an empty path is home");
}

#[test]
fn a_request_that_cannot_be_right_is_refused_before_a_worker_starts() {
    let unknown = plan(&[def("build")], &plain_ssh(), "elsewhere", "/x");
    assert_eq!(unknown.err().unwrap(), "elsewhere is not one of this computer's machines");
    assert!(plan(&[def("build")], &plain_ssh(), "local", "/x").is_err());
    let via = MachineDef {
        via: Some(vec!["fly".into()]),
        ..def("fly")
    };
    assert!(plan(&[via], &plain_ssh(), "fly", "/x").is_err());
    for path in ["a\nb", "a\0b", &"a".repeat(4097)] {
        assert!(plan(&[def("build")], &plain_ssh(), "build", path).is_err(), "{path:?}");
    }
}

#[test]
fn the_answers_reach_the_window_in_the_shape_it_reads() {
    let listing = serde_json::to_value(Listing {
        folder: "/srv/app".into(),
        dirs: vec!["src".into()],
        files: vec!["a.txt".into()],
        truncated: false,
        hidden: 2,
    })
    .unwrap();
    assert_eq!(
        listing,
        serde_json::json!({ "folder": "/srv/app", "dirs": ["src"], "files": ["a.txt"], "truncated": false, "hidden": 2 })
    );
    let index = serde_json::to_value(Index {
        folder: "/srv/app".into(),
        files: vec!["src/a.rs".into()],
        truncated: true,
        hidden: 0,
    })
    .unwrap();
    assert_eq!(
        index,
        serde_json::json!({ "folder": "/srv/app", "files": ["src/a.rs"], "truncated": true, "hidden": 0 })
    );
}

#[test]
fn a_fake_remote_answers_a_listing_and_an_index() {
    if !has_sh() {
        eprintln!("skipped: no POSIX sh with the list script's tools on this host");
        return;
    }
    let root = std::env::temp_dir().join(format!("browse-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&root);
    let home = root.join("home");
    for file in ["app/src/main.rs", "app/README.md", "app/.git/config"] {
        let path = home.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
    }
    let machine = Machine::ssh(
        "build",
        vec![
            fake_ssh().to_string_lossy().into_owned(),
            "--mode=remote-sh".into(),
            format!("--home={}", home.display()),
        ],
        "maitty",
    );

    let listing = list_folder(&machine, "build", "~/app").unwrap();
    let index = index_files(&machine, "build", "~/app").unwrap();
    let _ = std::fs::remove_dir_all(&root);

    assert_eq!(listing.dirs, [".git", "src"]);
    assert_eq!(listing.files, ["README.md"]);
    assert_eq!(index.files, ["README.md", "src/main.rs"]);
}
