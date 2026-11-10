//! `machine update` against a fake remote: the fake-ssh shim in `remote-sh` mode runs the
//! command it is given through the host's POSIX `sh`, with `HOME` a temporary folder, so the
//! real install script runs against a temporary filesystem (see TESTING, "testing over ssh").
//! A host with no usable `sh` (a plain Windows shell) skips these and says so; Git for
//! Windows' `sh` and any Linux or macOS host run them. The release fetch is a fake: no test
//! touches the network.

use super::remote_status_tests::built;
use crate::brand::{APP_COMMAND, APP_VERSION};
use crate::cli::machine_update::binary::{Binary, ReleaseSources, Releases, Sources, measured};
use crate::cli::machine_update::{Failure, Flags, Outcome, Talk, installed_is_current, update};
use crate::ipc::Machine;
use crate::remote_install::Platform;
use std::path::{Path, PathBuf};

/// The tools the install script uses, found on this host's `sh`.
fn sh_has_the_tools() -> bool {
    std::process::Command::new("sh")
        .args([
            "-c",
            "command -v wc mv chmod mktemp cat dirname tr cut printf uname rm mkdir >/dev/null",
        ])
        .stdin(std::process::Stdio::null())
        .status()
        .is_ok_and(|s| s.success())
}

macro_rules! needs_sh {
    () => {
        if !sh_has_the_tools() {
            eprintln!("skipped: no POSIX sh with the install script's tools on this host");
            return;
        }
    };
}

/// A fake remote: its home folder, its `uname`, and what the shim was asked.
struct Remote {
    dir: PathBuf,
    home: PathBuf,
    log: PathBuf,
}

impl Remote {
    fn new(uname: &str) -> Remote {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = std::env::temp_dir().join(format!("machine-update-{}-{n}", std::process::id()));
        let home = dir.join("home");
        std::fs::create_dir_all(home.join("bin")).expect("make the fake home");
        let remote = Remote {
            log: dir.join("remote-commands.txt"),
            dir,
            home,
        };
        remote.write_program("bin/uname", &format!("#!/bin/sh\necho '{uname}'\n"));
        remote
    }

    /// An executable file in the fake home.
    fn write_program(&self, relative: &str, body: &str) -> PathBuf {
        let path = self.home.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, body).unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        path
    }

    /// An installed app that reports `version`.
    fn install(&self, relative: &str, version: &str) -> PathBuf {
        self.write_program(
            relative,
            &format!("#!/bin/sh\necho '{APP_COMMAND} {version}'\n"),
        )
    }

    fn machine(&self, remote_cmd: &str) -> Machine {
        let (_, shim) = built();
        Machine::ssh(
            "box",
            vec![
                shim.to_string_lossy().into_owned(),
                "--mode=remote-sh".into(),
                format!("--home={}", self.home.display()),
                format!("--argv-log={}", self.log.display()),
            ],
            remote_cmd,
        )
    }

    /// Whether anything was sent: the install script runs `mktemp`, nothing else does.
    fn uploaded(&self) -> bool {
        self.commands().iter().any(|c| c.contains("mktemp"))
    }

    fn commands(&self) -> Vec<String> {
        std::fs::read_to_string(&self.log)
            .unwrap_or_default()
            .lines()
            .map(str::to_string)
            .collect()
    }

    fn read(&self, relative: &str) -> String {
        std::fs::read_to_string(self.home.join(relative)).unwrap()
    }

    /// Names in a folder of the fake home (to find a temp file left behind).
    fn names_in(&self, relative: &str) -> Vec<String> {
        std::fs::read_dir(self.home.join(relative))
            .map(|rd| {
                rd.flatten()
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default()
    }
}

impl Drop for Remote {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// Hands out a file as the binary, optionally claiming another length.
struct File {
    path: PathBuf,
    claim_extra_bytes: u64,
}

impl Sources for File {
    fn binary(&self, _: &Platform, _: &mut dyn FnMut(&str)) -> Result<Binary, String> {
        let binary = measured(self.path.clone(), None)?;
        let claimed = binary.len + self.claim_extra_bytes;
        Ok(binary.with_len(claimed))
    }
}

fn new_build(remote: &Remote) -> File {
    let path = remote.dir.join("new-build");
    std::fs::write(
        &path,
        format!("#!/bin/sh\necho '{APP_COMMAND} {APP_VERSION}'\n"),
    )
    .unwrap();
    File {
        path,
        claim_extra_bytes: 0,
    }
}

/// What the person at the keyboard does.
struct Person {
    said: Vec<String>,
    progress: Vec<u8>,
    /// `None`: nobody is there to ask.
    answer: Option<bool>,
    asked: Vec<String>,
}

impl Person {
    fn absent() -> Person {
        Person {
            said: Vec::new(),
            progress: Vec::new(),
            answer: None,
            asked: Vec::new(),
        }
    }
    fn says(answer: bool) -> Person {
        Person {
            said: Vec::new(),
            progress: Vec::new(),
            answer: Some(answer),
            asked: Vec::new(),
        }
    }
    fn heard(&self, part: &str) -> bool {
        self.said.iter().any(|l| l.contains(part))
    }
}

impl Talk for Person {
    fn say(&mut self, line: &str) {
        self.said.push(line.to_string());
    }
    fn progress(&mut self, percent: u8) {
        self.progress.push(percent);
    }
    fn can_ask(&self) -> bool {
        self.answer.is_some()
    }
    fn confirm(&mut self, question: &str) -> bool {
        self.asked.push(question.to_string());
        self.answer.unwrap_or(false)
    }
}

/// A plain command name no host has on its PATH (this host's own `maitty` must not answer).
const UNLISTED: &str = "maitty-not-on-any-path";
const LINUX: &str = "Linux x86_64";
const AT_HOME: &str = "~/.local/bin/maitty";
const INSTALL: Flags = Flags {
    install: true,
    yes: false,
    force: false,
    restart: false,
    wait: false,
};
const YES: Flags = Flags {
    install: false,
    yes: true,
    force: false,
    restart: false,
    wait: false,
};
const NO_FLAGS: Flags = Flags {
    install: false,
    yes: false,
    force: false,
    restart: false,
    wait: false,
};

fn run(
    remote: &Remote,
    remote_cmd: &str,
    flags: Flags,
    source: &dyn Sources,
    person: &mut Person,
) -> Result<Outcome, Failure> {
    update(&remote.machine(remote_cmd), flags, source, person)
}

fn failure_text(result: Result<Outcome, Failure>) -> String {
    match result {
        Err(Failure::Usage(m)) | Err(Failure::Failed(m)) => m,
        Ok(outcome) => panic!("expected a failure, got {outcome:?}"),
    }
}

#[test]
fn an_install_reports_each_stage_it_finishes_in_order() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let mut person = Person::absent();
    run(&remote, AT_HOME, INSTALL, &new_build(&remote), &mut person).unwrap();
    assert!(person.progress.len() >= 4, "{:?}", person.progress);
    assert!(person.progress.windows(2).all(|pair| pair[0] < pair[1]), "{:?}", person.progress);
    assert!(person.progress.iter().all(|percent| *percent < 100));
}

#[test]
fn a_missing_app_is_installed_with_the_flag_and_runs_afterwards() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let mut person = Person::absent();
    let outcome = run(&remote, AT_HOME, INSTALL, &new_build(&remote), &mut person).unwrap();
    assert_eq!(
        outcome,
        Outcome::Installed {
            version: APP_VERSION.into(),
            destination: AT_HOME.into()
        }
    );
    assert_eq!(
        remote.read(".local/bin/maitty"),
        format!("#!/bin/sh\necho '{APP_COMMAND} {APP_VERSION}'\n")
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = std::fs::metadata(remote.home.join(".local/bin/maitty"))
            .unwrap()
            .permissions()
            .mode();
        assert_eq!(mode & 0o777, 0o755);
    }
    assert!(person.heard("not installed"), "{:?}", person.said);
    assert!(person.heard("verified"), "{:?}", person.said);
    assert!(
        remote
            .names_in(".local/bin")
            .iter()
            .all(|n| !n.contains("-new.")),
        "no temp file is left"
    );
    assert!(
        !remote
            .commands()
            .iter()
            .any(|c| c.contains("restart-if-idle")),
        "a first install has no daemon to ask"
    );
}

#[test]
fn a_missing_app_without_the_flag_or_a_person_is_refused_and_nothing_is_sent() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let message = failure_text(run(
        &remote,
        AT_HOME,
        NO_FLAGS,
        &new_build(&remote),
        &mut Person::absent(),
    ));
    assert_eq!(
        message,
        "maitty is not installed on box; run `maitty machine update box --install`"
    );
    assert!(!remote.uploaded());
    assert!(!remote.home.join(".local").exists());
}

#[test]
fn a_person_is_asked_and_a_no_changes_nothing() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let mut person = Person::says(false);
    let message = failure_text(run(
        &remote,
        AT_HOME,
        NO_FLAGS,
        &new_build(&remote),
        &mut person,
    ));
    assert!(message.contains("nothing was changed on box"), "{message}");
    assert_eq!(person.asked.len(), 1);
    assert!(!remote.uploaded());
    let mut yes = Person::says(true);
    run(&remote, AT_HOME, NO_FLAGS, &new_build(&remote), &mut yes).unwrap();
    assert!(remote.home.join(".local/bin/maitty").is_file());
}

#[test]
fn wait_installs_then_asks_the_far_daemon_to_restart_when_idle() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", "0.1.0");
    let flags = Flags {
        wait: true,
        ..YES
    };
    let mut person = Person::absent();
    run(&remote, AT_HOME, flags, &new_build(&remote), &mut person).unwrap();
    assert!(person.heard("waiting for"), "{:?}", person.said);
    let commands = remote.commands();
    assert!(
        commands.last().unwrap().ends_with("restart-if-idle"),
        "{commands:?}"
    );
}

#[test]
fn get_pulls_a_file_from_the_fake_remote_and_says_where_it_landed() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    std::fs::write(remote.home.join("out file.log"), "line one\nline two\n").unwrap();
    let dest = remote.dir.join("downloads");
    let path = crate::cli::machine_get::pull(
        &remote.machine(AT_HOME),
        "~/out file.log",
        &dest,
        false,
    )
    .unwrap();
    assert_eq!(path, dest.join("out file.log"));
    assert_eq!(
        std::fs::read_to_string(&path).unwrap().replace("\r\n", "\n"),
        "line one\nline two\n"
    );
    let again = crate::cli::machine_get::pull(
        &remote.machine(AT_HOME),
        "~/out file.log",
        &dest,
        false,
    )
    .unwrap();
    assert_eq!(again, dest.join("out file-1.log"), "nothing is overwritten");
    let mut left: Vec<_> = std::fs::read_dir(&dest)
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    left.sort();
    assert_eq!(left, ["out file-1.log", "out file.log"], "no part file is left");
}

#[test]
fn get_says_why_a_path_cannot_be_pulled_and_leaves_nothing() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let dest = remote.dir.join("downloads");
    let pull = |path: &str| {
        match crate::cli::machine_get::pull(&remote.machine(AT_HOME), path, &dest, false) {
            Err(Failure::Failed(why)) => why,
            other => panic!("expected a failure for {path}: {other:?}"),
        }
    };
    assert_eq!(pull("~/nope.log"), "~/nope.log does not exist on box");
    assert!(
        std::fs::read_dir(&dest).map_or(true, |mut d| d.next().is_none()),
        "nothing was saved"
    );
}

#[test]
fn browsing_a_fake_remote_lists_a_folder_and_indexes_the_files_below_it() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    for file in ["app/src/main.rs", "app/src/deep/lib.rs", "app/README.md", "app/.git/config", "app/node_modules/x/y.js"] {
        let path = remote.home.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, "x").unwrap();
    }
    let machine = remote.machine(AT_HOME);

    let listing = maitty_machines::list_folder(&machine, "box", "~/app").unwrap();
    assert!(listing.folder.ends_with("/app"), "{}", listing.folder);
    assert_eq!(listing.dirs, [".git", "node_modules", "src"]);
    assert_eq!(listing.files, ["README.md"]);
    assert!(!listing.truncated);

    let index = maitty_machines::index_files(&machine, "box", "~/app").unwrap();
    assert_eq!(index.files, ["README.md", "src/deep/lib.rs", "src/main.rs"], "no .git or node_modules");
    assert!(!index.truncated);
}

#[test]
fn browsing_says_why_a_folder_cannot_be_looked_into() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    std::fs::write(remote.home.join("plain.txt"), "x").unwrap();
    let machine = remote.machine(AT_HOME);
    assert_eq!(
        maitty_machines::list_folder(&machine, "box", "~/nope").unwrap_err(),
        "~/nope does not exist on box"
    );
    assert_eq!(
        maitty_machines::list_folder(&machine, "box", "~/plain.txt").unwrap_err(),
        "~/plain.txt on box is not a folder"
    );
    assert!(maitty_machines::index_files(&machine, "box", "~/nope").is_err());
}

#[test]
fn a_folder_is_pulled_from_the_fake_remote_whole_and_a_second_pull_is_numbered() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    for file in ["app/README.md", "app/src/main.rs", "app/src/deep/lib.rs"] {
        let path = remote.home.join(file);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, format!("contents of {file}")).unwrap();
    }
    std::fs::create_dir_all(remote.home.join("app/empty")).unwrap();
    let machine = remote.machine(AT_HOME);
    let dest = remote.dir.join("downloads");

    let first = maitty_machines::get_folder(&machine, "box", "~/app", &dest, &maitty_machines::ssh_run::Watch::default()).unwrap();
    assert_eq!(first, dest.join("app"));
    assert_eq!(std::fs::read_to_string(first.join("src/deep/lib.rs")).unwrap(), "contents of app/src/deep/lib.rs");
    assert!(first.join("empty").is_dir(), "an empty folder arrives too");
    let second = maitty_machines::get_folder(&machine, "box", "~/app", &dest, &maitty_machines::ssh_run::Watch::default()).unwrap();
    assert_eq!(second, dest.join("app-1"), "nothing existing is touched");

    let mut left: Vec<_> = std::fs::read_dir(&dest).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    left.sort();
    assert_eq!(left, ["app", "app-1"], "no archive or temp folder is left");
}

#[test]
fn a_folder_pull_says_why_it_cannot_and_leaves_nothing() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    std::fs::write(remote.home.join("plain.txt"), "x").unwrap();
    let machine = remote.machine(AT_HOME);
    let dest = remote.dir.join("downloads");
    let pull = |path: &str| maitty_machines::get_folder(&machine, "box", path, &dest, &maitty_machines::ssh_run::Watch::default()).unwrap_err();
    assert_eq!(pull("~/nope"), "~/nope does not exist on box");
    assert_eq!(pull("~/plain.txt"), "~/plain.txt on box is not a folder");
    assert!(std::fs::read_dir(&dest).map_or(true, |mut d| d.next().is_none()), "nothing was saved");
}

#[test]
fn a_folder_is_sent_to_the_fake_remote_and_lands_whole_in_its_private_cache() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let local = remote.dir.join("local");
    let folder = local.join("my project");
    std::fs::create_dir_all(folder.join("src/deep")).unwrap();
    std::fs::create_dir_all(folder.join("empty")).unwrap();
    std::fs::write(folder.join("README.md"), "hello").unwrap();
    std::fs::write(folder.join("src/deep/lib.rs"), "fn x() {}").unwrap();
    let machine = remote.machine(AT_HOME);

    let path = maitty_machines::remote_send::send_folder(&machine, "box", &folder, &maitty_machines::ssh_run::Watch::default()).unwrap();

    assert!(path.starts_with('/'), "{path}");
    assert!(path.contains("/pastes/paste-") && path.ends_with("-my_project"), "{path}");
    let cache = remote.home.join(".cache").join(APP_COMMAND).join("pastes");
    let kept: Vec<_> = std::fs::read_dir(&cache).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    assert_eq!(kept.len(), 1, "one folder and no temporary leftovers: {kept:?}");
    let landed = cache.join(&kept[0]);
    assert_eq!(std::fs::read_to_string(landed.join("README.md")).unwrap(), "hello");
    assert_eq!(std::fs::read_to_string(landed.join("src/deep/lib.rs")).unwrap(), "fn x() {}");
    assert!(landed.join("empty").is_dir());
}

#[test]
fn get_takes_a_folder_by_itself_and_leaves_a_file_as_it_was() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    std::fs::create_dir_all(remote.home.join("app/src")).unwrap();
    std::fs::write(remote.home.join("app/src/main.rs"), "fn main() {}").unwrap();
    std::fs::write(remote.home.join("notes.txt"), "plain").unwrap();
    let machine = remote.machine(AT_HOME);
    let dest = remote.dir.join("downloads");

    let folder = crate::cli::machine_get::pull(&machine, "~/app", &dest, false).unwrap();
    let file = crate::cli::machine_get::pull(&machine, "~/notes.txt", &dest, false).unwrap();

    assert_eq!(folder, dest.join("app"));
    assert_eq!(std::fs::read_to_string(folder.join("src/main.rs")).unwrap(), "fn main() {}");
    assert_eq!(std::fs::read_to_string(&file).unwrap(), "plain");
    match crate::cli::machine_get::pull(&machine, "~/nope", &dest, false) {
        Err(Failure::Failed(why)) => assert_eq!(why, "~/nope does not exist on box"),
        other => panic!("expected a plain failure: {other:?}"),
    }
}

#[test]
fn put_sends_a_file_and_a_folder_and_says_where_each_landed() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let local = remote.dir.join("local");
    std::fs::create_dir_all(local.join("tree/inner")).unwrap();
    std::fs::write(local.join("tree/inner/leaf.txt"), "leaf").unwrap();
    std::fs::write(local.join("single.log"), "one file").unwrap();
    let machine = remote.machine(AT_HOME);

    let file = crate::cli::machine_put::push(&machine, &local.join("single.log"), false).unwrap();
    let folder = crate::cli::machine_put::push(&machine, &local.join("tree"), false).unwrap();

    assert!(file.contains("/pastes/paste-") && file.ends_with("-single.log"), "{file}");
    assert!(folder.contains("/pastes/paste-") && folder.ends_with("-tree"), "{folder}");
    let cache = remote.home.join(".cache").join(APP_COMMAND).join("pastes");
    let names: Vec<String> = std::fs::read_dir(&cache).unwrap().map(|e| e.unwrap().file_name().to_string_lossy().into_owned()).collect();
    let tree = names.iter().find(|n| n.ends_with("-tree")).expect("the folder landed");
    assert_eq!(std::fs::read_to_string(cache.join(tree).join("inner/leaf.txt")).unwrap(), "leaf");
    assert!(names.iter().any(|n| n.ends_with("-single.log")));
    match crate::cli::machine_put::push(&machine, &local.join("missing"), false) {
        Err(Failure::Failed(why)) => assert!(why.starts_with("could not read"), "{why}"),
        other => panic!("expected a plain failure: {other:?}"),
    }
}

#[test]
fn restart_asks_the_far_daemon_to_restart_even_with_sessions_open() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", "0.1.0");
    let flags = Flags {
        restart: true,
        ..YES
    };
    run(&remote, AT_HOME, flags, &new_build(&remote), &mut Person::absent()).unwrap();
    let commands = remote.commands();
    assert!(
        commands.last().unwrap().ends_with("restart-if-idle --now"),
        "{commands:?}"
    );
}

/// A build whose daemon-restart reply is the far side's own: it leaves a daemon with open
/// sessions alone, and restarts it when told `--now`.
fn busy_daemon_build(remote: &Remote) -> File {
    let path = remote.dir.join("busy-build");
    std::fs::write(
        &path,
        format!(
            "#!/bin/sh
case \"$*\" in
*--now*) echo 'restarted: sessions were saved and come back with the new version';;
*restart-if-idle*) echo 'left running: 1 open session(s); it keeps them';;
*) echo '{APP_COMMAND} {APP_VERSION}';;
esac
"
        ),
    )
    .unwrap();
    File {
        path,
        claim_extra_bytes: 0,
    }
}

#[test]
fn a_daemon_left_running_by_open_sessions_is_restarted_when_the_person_agrees() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", "0.1.0");
    let mut person = Person::absent();
    person.answer = Some(true);
    run(&remote, AT_HOME, YES, &busy_daemon_build(&remote), &mut person).unwrap();
    let commands = remote.commands();
    assert!(
        commands.last().unwrap().ends_with("restart-if-idle --now"),
        "{commands:?}"
    );
    assert!(person.heard("daemon: restarted"), "{:?}", person.said);
    assert_eq!(person.asked.len(), 1, "{:?}", person.asked);
}

#[test]
fn a_daemon_left_running_stays_untouched_when_the_person_declines_and_the_way_out_is_said() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", "0.1.0");
    let mut person = Person::absent();
    person.answer = Some(false);
    run(&remote, AT_HOME, YES, &busy_daemon_build(&remote), &mut person).unwrap();
    let commands = remote.commands();
    assert!(
        commands.last().unwrap().ends_with("restart-if-idle"),
        "{commands:?}"
    );
    assert!(person.heard("click Update on box"), "{:?}", person.said);
}

#[test]
fn a_daemon_left_running_with_nobody_to_ask_says_the_way_out_without_restarting() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", "0.1.0");
    let mut person = Person::absent();
    run(&remote, AT_HOME, YES, &busy_daemon_build(&remote), &mut person).unwrap();
    assert!(person.asked.is_empty());
    assert!(person.heard("click Update on box"), "{:?}", person.said);
}

#[test]
fn the_window_is_told_when_the_app_there_is_already_this_version() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", APP_VERSION);
    assert!(installed_is_current(&remote.machine(AT_HOME), "box"));
}

#[test]
fn an_older_app_there_is_not_called_current() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", "0.1.0");
    assert!(!installed_is_current(&remote.machine(AT_HOME), "box"));
}

#[test]
fn an_older_app_is_replaced_on_yes_and_the_daemon_is_asked_not_killed() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", "0.1.0");
    let mut person = Person::absent();
    let outcome = run(&remote, AT_HOME, YES, &new_build(&remote), &mut person).unwrap();
    assert_eq!(
        outcome,
        Outcome::Installed {
            version: APP_VERSION.into(),
            destination: AT_HOME.into()
        }
    );
    assert!(remote.read(".local/bin/maitty").contains(APP_VERSION));
    assert!(person.heard("maitty 0.1.0"), "{:?}", person.said);
    let commands = remote.commands();
    assert!(
        commands.last().unwrap().ends_with("restart-if-idle"),
        "{commands:?}"
    );
    assert!(
        commands.iter().all(|c| !c.contains("kill")),
        "no command signals a process: {commands:?}"
    );
    assert!(person.heard("daemon:"), "{:?}", person.said);
}

#[test]
fn an_older_app_without_yes_or_a_person_is_refused_and_untouched() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let old = remote.install(".local/bin/maitty", "0.1.0");
    let before = std::fs::read_to_string(&old).unwrap();
    let message = failure_text(run(
        &remote,
        AT_HOME,
        NO_FLAGS,
        &new_build(&remote),
        &mut Person::absent(),
    ));
    assert!(message.contains("--yes"), "{message}");
    assert_eq!(std::fs::read_to_string(&old).unwrap(), before);
    assert!(!remote.uploaded());
}

#[test]
fn a_matching_app_is_left_alone() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", APP_VERSION);
    let mut person = Person::absent();
    let outcome = run(&remote, AT_HOME, INSTALL, &new_build(&remote), &mut person).unwrap();
    assert_eq!(
        outcome,
        Outcome::UpToDate {
            version: APP_VERSION.into()
        }
    );
    assert_eq!(
        outcome.line("box"),
        format!("box: maitty {APP_VERSION} is already up to date")
    );
    assert!(!remote.uploaded());
}

#[test]
fn a_matching_app_still_asks_a_stale_far_daemon_to_restart() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", APP_VERSION);
    let mut person = Person::absent();
    run(&remote, AT_HOME, YES, &new_build(&remote), &mut person).unwrap();
    assert!(!remote.uploaded());
    let commands = remote.commands();
    assert!(
        commands.last().unwrap().ends_with("restart-if-idle"),
        "{commands:?}"
    );
    assert!(person.heard("daemon:"), "{:?}", person.said);
}

#[test]
fn restart_with_a_matching_app_restarts_the_far_daemon_even_with_sessions_open() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.install(".local/bin/maitty", APP_VERSION);
    let flags = Flags {
        restart: true,
        ..YES
    };
    run(&remote, AT_HOME, flags, &new_build(&remote), &mut Person::absent()).unwrap();
    let commands = remote.commands();
    assert!(
        commands.last().unwrap().ends_with("restart-if-idle --now"),
        "{commands:?}"
    );
    assert!(!remote.uploaded());
}

#[test]
fn a_newer_app_is_not_downgraded_without_force() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let newer = remote.install(".local/bin/maitty", "99.0.0");
    let before = std::fs::read_to_string(&newer).unwrap();
    let message = failure_text(run(
        &remote,
        AT_HOME,
        YES,
        &new_build(&remote),
        &mut Person::absent(),
    ));
    assert!(message.contains("--force"), "{message}");
    assert_eq!(std::fs::read_to_string(&newer).unwrap(), before);
    let force = Flags {
        force: true,
        ..NO_FLAGS
    };
    run(
        &remote,
        AT_HOME,
        force,
        &new_build(&remote),
        &mut Person::absent(),
    )
    .unwrap();
    assert!(remote.read(".local/bin/maitty").contains(APP_VERSION));
}

#[test]
fn fewer_bytes_than_announced_abort_and_leave_the_old_app_and_no_temp_file() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let old = remote.install(".local/bin/maitty", "0.1.0");
    let before = (
        std::fs::read_to_string(&old).unwrap(),
        std::fs::metadata(&old).unwrap().modified().unwrap(),
    );
    let short = File {
        claim_extra_bytes: 10,
        ..new_build(&remote)
    };
    let message = failure_text(run(&remote, AT_HOME, YES, &short, &mut Person::absent()));
    assert!(message.contains("not the size that was sent"), "{message}");
    assert!(
        message.ends_with("the install on box did not finish; the old version is unchanged"),
        "{message}"
    );
    assert_eq!(std::fs::read_to_string(&old).unwrap(), before.0);
    assert_eq!(
        std::fs::metadata(&old).unwrap().modified().unwrap(),
        before.1,
        "not rewritten"
    );
    assert_eq!(
        remote.names_in(".local/bin"),
        vec!["maitty".to_string()],
        "the temp file is gone"
    );
}

#[test]
fn a_file_that_does_not_run_there_aborts_and_leaves_the_old_app() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let old = remote.install(".local/bin/maitty", "0.1.0");
    let before = std::fs::read_to_string(&old).unwrap();
    let junk = remote.dir.join("junk");
    std::fs::write(&junk, "this is not a program\n").unwrap();
    let source = File {
        path: junk,
        claim_extra_bytes: 0,
    };
    let message = failure_text(run(&remote, AT_HOME, YES, &source, &mut Person::absent()));
    assert!(message.contains("did not run on that machine"), "{message}");
    assert!(
        message.contains("the old version is unchanged"),
        "{message}"
    );
    assert_eq!(std::fs::read_to_string(&old).unwrap(), before);
    assert_eq!(remote.names_in(".local/bin"), vec!["maitty".to_string()]);
}

#[test]
fn a_first_install_that_fails_says_nothing_was_installed() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let junk = remote.dir.join("junk");
    std::fs::write(&junk, "nope\n").unwrap();
    let source = File {
        path: junk,
        claim_extra_bytes: 0,
    };
    let message = failure_text(run(
        &remote,
        AT_HOME,
        INSTALL,
        &source,
        &mut Person::absent(),
    ));
    assert!(
        message.ends_with("did not finish; nothing was installed"),
        "{message}"
    );
    assert!(!remote.home.join(".local/bin/maitty").exists());
}

#[test]
fn a_destination_that_cannot_be_made_aborts_with_the_write_message() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    remote.write_program(".local", "a file where a folder should be");
    let message = failure_text(run(
        &remote,
        UNLISTED,
        INSTALL,
        &new_build(&remote),
        &mut Person::absent(),
    ));
    assert!(
        message.contains("could not write to the install folder"),
        "{message}"
    );
    assert!(message.ends_with("nothing was installed"), "{message}");
}

#[test]
fn a_path_with_spaces_and_a_quote_is_one_literal_destination() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let cmd = "~/my tools/it's/maitty";
    run(
        &remote,
        cmd,
        INSTALL,
        &new_build(&remote),
        &mut Person::absent(),
    )
    .unwrap();
    assert!(remote.read("my tools/it's/maitty").contains(APP_VERSION));
}

#[test]
fn an_absolute_remote_cmd_is_where_the_app_goes() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let target = remote.dir.join("opt").join("maitty");
    let cmd = target.to_string_lossy().replace('\\', "/");
    if !cmd.starts_with('/') {
        eprintln!("skipped: this host's paths are not absolute to a POSIX sh");
        return;
    }
    run(
        &remote,
        &cmd,
        INSTALL,
        &new_build(&remote),
        &mut Person::absent(),
    )
    .unwrap();
    assert!(target.is_file());
}

#[test]
fn a_default_install_says_when_the_folder_is_not_on_the_path() {
    needs_sh!();
    let remote = Remote::new(LINUX);
    let mut person = Person::absent();
    run(&remote, UNLISTED, INSTALL, &new_build(&remote), &mut person).unwrap();
    assert!(
        remote.home.join(".local/bin/maitty").is_file(),
        "the default destination"
    );
    assert!(
        person.heard(&format!(r#"set remote_cmd = "{AT_HOME}""#)),
        "{:?}",
        person.said
    );
}

#[test]
fn a_windows_like_machine_is_refused_before_anything_else() {
    needs_sh!();
    for uname in [
        "MINGW64_NT-10.0-26300 x86_64",
        "CYGWIN_NT-10.0 x86_64",
        "MSYS_NT-10.0 x86_64",
        "Windows_NT x86_64",
    ] {
        let remote = Remote::new(uname);
        let message = failure_text(run(
            &remote,
            AT_HOME,
            INSTALL,
            &new_build(&remote),
            &mut Person::absent(),
        ));
        assert_eq!(
            message, "Windows machines are not supported as a remote in v1",
            "{uname}"
        );
        assert!(!remote.uploaded());
        assert_eq!(remote.commands().len(), 1, "only the platform was asked");
    }
}

#[test]
fn an_unsupported_platform_is_named_and_nothing_is_sent() {
    needs_sh!();
    for uname in ["FreeBSD amd64", "Linux riscv64"] {
        let remote = Remote::new(uname);
        let message = failure_text(run(
            &remote,
            AT_HOME,
            INSTALL,
            &new_build(&remote),
            &mut Person::absent(),
        ));
        assert_eq!(message, format!("unsupported remote platform \"{uname}\""));
        assert!(!remote.uploaded());
    }
}

struct FakeReleases {
    sums: String,
}

impl Releases for FakeReleases {
    fn latest_tag(&self) -> Result<String, String> {
        Ok(format!("v{APP_VERSION}"))
    }
    fn download(&self, _tag: &str, name: &str, dir: &Path) -> Result<PathBuf, String> {
        let bytes = if name == "sha256sums.txt" {
            self.sums.as_bytes()
        } else {
            b"an archive"
        };
        let path = dir.join(name);
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        Ok(path)
    }
}

#[test]
fn a_fetched_release_with_a_bad_checksum_is_never_sent() {
    needs_sh!();
    // The remote is another platform than any test host, so the release path is taken.
    let remote = Remote::new("Linux aarch64");
    let releases = FakeReleases {
        sums: format!(
            "{}  {APP_COMMAND}-aarch64-unknown-linux-musl.tar.gz\n",
            "0".repeat(64)
        ),
    };
    let sources = ReleaseSources {
        releases: &releases,
        local: None,
    };
    let message = failure_text(run(
        &remote,
        AT_HOME,
        INSTALL,
        &sources,
        &mut Person::absent(),
    ));
    assert!(message.starts_with("checksum mismatch"), "{message}");
    assert!(message.ends_with("nothing was sent to box"), "{message}");
    assert!(!remote.uploaded(), "{:?}", remote.commands());
    assert!(!remote.home.join(".local").exists());
}

#[test]
fn a_matching_platform_streams_this_computers_own_build_not_a_download() {
    needs_sh!();
    let Some(here) = Platform::of_this_machine() else {
        eprintln!("skipped: this computer is not a Linux or macOS twin of any remote");
        return;
    };
    let remote = Remote::new(&format!(
        "{} {}",
        if here.os_name() == "linux" {
            "Linux"
        } else {
            "Darwin"
        },
        here.arch_name()
    ));
    let releases = FakeReleases {
        sums: String::new(),
    };
    let exe = remote.dir.join("this-computers-maitty");
    std::fs::write(
        &exe,
        format!("#!/bin/sh\necho '{APP_COMMAND} {APP_VERSION}'\n"),
    )
    .unwrap();
    let sources = ReleaseSources {
        releases: &releases,
        local: Some((here, exe)),
    };
    run(&remote, AT_HOME, INSTALL, &sources, &mut Person::absent()).unwrap();
    assert!(remote.home.join(".local/bin/maitty").is_file());
}

#[test]
fn a_machine_reached_by_a_custom_transport_is_not_installed_over_and_nothing_is_run() {
    let remote = Remote::new(LINUX);
    let calls = remote.dir.join("via-calls.txt");
    let (_, shim) = built();
    let def = maitty_settings::config::MachineDef {
        name: "fly".into(),
        target: "fly".into(),
        via: Some(vec![
            shim.to_string_lossy().into_owned(),
            format!("--calls={}", calls.display()),
        ]),
        ..Default::default()
    };
    let machine = Machine::from_def(&def, "ssh");
    let result = update(&machine, INSTALL, &new_build(&remote), &mut Person::absent());
    assert_eq!(
        failure_text(result),
        format!("installing over a custom transport is not supported; install {APP_COMMAND} on fly yourself")
    );
    assert!(!calls.exists(), "the transport was started");
}
