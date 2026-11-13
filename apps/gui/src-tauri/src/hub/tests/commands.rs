//! A request for a pane goes to the machine that has the pane, over that machine's line, and the
//! answer it brings is taken only because the request was made. Each command, against a real
//! daemon behind the fake `ssh` shim.

use super::*;
use crate::terminal_stream::Wire;
use maitty_desktop_client::PaneCommand;
use maitty_shared::protocol::{ClientMsg, Command, Reply, TermId, TermInfo};

const BOX: MachineId = MachineId(1);

impl Watch {
    /// The newest copy of a pane the machine has told of, from a snapshot or a delta.
    fn pane(&self, machine: MachineId, term: TermId) -> Option<TermInfo> {
        let mut found = None;
        for (m, msg) in &self.msgs {
            if *m != machine {
                continue;
            }
            match msg {
                ServerMsg::State(snapshot) => found = snapshot.terms.get(&term).cloned(),
                ServerMsg::TermsChanged { changed, removed } => {
                    if let Some(pane) = changed.iter().find(|pane| pane.id == term) {
                        found = Some(pane.clone());
                    }
                    if removed.contains(&term) {
                        found = None;
                    }
                }
                _ => {}
            }
        }
        found
    }

    fn any_pane(&self, machine: MachineId) -> Option<TermId> {
        self.msgs.iter().find_map(|(m, msg)| match msg {
            ServerMsg::State(s) if *m == machine => s.terms.keys().next().copied(),
            ServerMsg::TermsChanged { changed, .. } if *m == machine => {
                changed.first().map(|pane| pane.id)
            }
            _ => None,
        })
    }

    fn replies(&self, machine: MachineId) -> usize {
        self.msgs
            .iter()
            .filter(|(m, msg)| *m == machine && matches!(msg, ServerMsg::Reply(Reply::Ok)))
            .count()
    }

    fn printed(&self, machine: MachineId, term: TermId) -> String {
        let mut text = String::new();
        for (m, msg) in &self.msgs {
            if let ServerMsg::Output { term: t, data } = msg
                && *m == machine
                && *t == term
            {
                text.push_str(&String::from_utf8_lossy(data));
            }
        }
        text
    }
}

/// A machine that is up, with a pane started on it through the wire, like the window does.
async fn with_a_pane(sandbox: &Sandbox) -> (Watch, Wire, TermId) {
    let mut watch = Watch::new(&[machine("box", sandbox, "proxy")], sandbox, quick(WAIT));
    let wire = Wire::reaching(watch.hub.handle());
    watch
        .until("connected and a snapshot", |w| {
            w.is(BOX, "connected") && w.snapshots(BOX) > 0
        })
        .await;
    let handle = watch.hub.handle();
    handle
        .send(Outgoing {
            machine: BOX,
            msg: ClientMsg::Command(Command::NewWorkspace {
                cwd: None,
                name: None,
                cmd: None,
            }),
        })
        .expect("the line is up");
    watch
        .until("a pane on the machine", |w| w.any_pane(BOX).is_some())
        .await;
    let term = watch.any_pane(BOX).unwrap();
    (watch, wire, term)
}

#[tokio::test]
async fn typing_goes_to_the_pane_of_that_machine() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let (mut watch, wire, term) = with_a_pane(&sandbox).await;
    let token = "TYPED_ON_THE_OTHER_MACHINE_77";
    wire.command(
        Some(1),
        term,
        PaneCommand::Input(format!("echo {token}\r").into_bytes()),
    )
    .expect("the machine takes typing");
    watch
        .until("the shell to print it", |w| {
            w.printed(BOX, term).matches(token).count() >= 2
        })
        .await;
}

#[tokio::test]
async fn a_replay_is_asked_for_over_the_line_and_comes_back_as_that_panes() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let (mut watch, wire, term) = with_a_pane(&sandbox).await;
    wire.command(Some(1), term, PaneCommand::WantReplay)
        .expect("the line is up");
    watch
        .until("the replay", |w| {
            w.msgs.iter().any(|(m, msg)| {
                *m == BOX && matches!(msg, ServerMsg::Replay { term: t, .. } if *t == term)
            })
        })
        .await;
}

#[tokio::test]
async fn a_resize_reaches_the_pane_and_its_answer_is_taken_once() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let (mut watch, wire, term) = with_a_pane(&sandbox).await;
    let before = watch.replies(BOX);
    wire.command(
        Some(1),
        term,
        PaneCommand::Resize {
            cols: 101,
            rows: 31,
        },
    )
    .expect("the line is up");
    watch
        .until("the pane to be that size", |w| {
            w.pane(BOX, term)
                .is_some_and(|pane| (pane.cols, pane.rows) == (101, 31))
        })
        .await;
    watch.until("the answer", |w| w.replies(BOX) > before).await;
}

#[tokio::test]
async fn a_rename_a_pin_and_an_archive_reach_the_pane() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let (mut watch, wire, term) = with_a_pane(&sandbox).await;
    wire.command(Some(1), term, PaneCommand::Rename("nightly".into()))
        .expect("the line is up");
    wire.command(Some(1), term, PaneCommand::Pin(true))
        .expect("the line is up");
    watch
        .until("renamed and pinned", |w| {
            w.pane(BOX, term)
                .is_some_and(|pane| pane.label == "nightly" && pane.pinned)
        })
        .await;
    wire.command(Some(1), term, PaneCommand::Pin(false))
        .expect("the line is up");
    wire.command(Some(1), term, PaneCommand::Archive(true))
        .expect("the line is up");
    watch
        .until("unpinned and archived", |w| {
            w.pane(BOX, term)
                .is_some_and(|pane| !pane.pinned && pane.archived)
        })
        .await;
}

#[tokio::test]
async fn looking_at_a_pane_is_answered_and_the_answer_is_taken_only_once_asked() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let (mut watch, wire, term) = with_a_pane(&sandbox).await;
    let before = watch.replies(BOX);
    wire.command(Some(1), term, PaneCommand::Seen)
        .expect("the line is up");
    watch.until("the answer", |w| w.replies(BOX) > before).await;
    // Nothing is out now, so a second answer from the machine would be ignored.
    let mut hub = watch.hub;
    let asked = lock(&hub.handle.shared.allowance)
        .entry(BOX)
        .or_default()
        .awaiting(Instant::now());
    assert!(!asked, "every request that was made has been answered");
    assert!(
        hub.admit(BOX, ServerMsg::Reply(Reply::Ok)).is_none(),
        "an answer nobody asked for"
    );
}

#[tokio::test]
async fn closing_a_pane_ends_the_one_on_that_machine() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let (mut watch, wire, term) = with_a_pane(&sandbox).await;
    wire.command(Some(1), term, PaneCommand::Close)
        .expect("the line is up");
    watch
        .until("the pane to go", |w| w.pane(BOX, term).is_none())
        .await;
}

#[tokio::test]
async fn typing_to_a_machine_that_is_down_is_refused_naming_it_and_counted() {
    let sandbox = Sandbox::new();
    let mut watch = Watch::new(
        &[machine("box", &sandbox, "auth-fail")],
        &sandbox,
        quick(WAIT),
    );
    let wire = Wire::reaching(watch.hub.handle());
    watch
        .until("needs-login", |w| w.is(BOX, "needs-login"))
        .await;

    for _ in 0..3 {
        let refused = wire
            .command(Some(1), 1, PaneCommand::Input(b"x".to_vec()))
            .expect_err("a machine with no line takes nothing");
        assert_eq!(refused, "box is not connected");
    }
    let refused = wire
        .command(Some(1), 1, PaneCommand::Pin(true))
        .expect_err("nor does it take a command");
    assert_eq!(refused, "box is not connected");
    let handle = watch.hub.handle();
    assert_eq!(handle.take_unsent(BOX), 3, "only the typing is counted");
    assert_eq!(handle.take_unsent(BOX), 0, "and counted once");
}

#[tokio::test]
async fn a_machine_that_is_not_ours_is_refused_and_not_counted() {
    let sandbox = Sandbox::new();
    let watch = Watch::new(&[], &sandbox, quick(WAIT));
    let wire = Wire::reaching(watch.hub.handle());
    let refused = wire
        .command(Some(9), 1, PaneCommand::Input(b"x".to_vec()))
        .expect_err("not one of ours");
    assert_eq!(refused, "Machine 9 is not one of this computer's machines");
    assert_eq!(watch.hub.handle().take_unsent(MachineId(9)), 0);
}

#[tokio::test]
async fn this_computers_own_refusal_is_the_words_it_always_had() {
    let sandbox = Sandbox::new();
    let watch = Watch::new(&[], &sandbox, quick(WAIT));
    let wire = Wire::reaching(watch.hub.handle());
    for machine in [None, Some(0)] {
        let refused = wire
            .command(machine, 1, PaneCommand::Input(b"x".to_vec()))
            .expect_err("no local line in this test");
        assert_eq!(refused, "The terminal is not connected to the daemon");
    }
    assert_eq!(watch.hub.handle().take_unsent(MachineId::LOCAL), 0);
}

#[tokio::test]
async fn a_session_started_on_a_machine_is_a_pane_of_that_machine() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    let wire = Wire::reaching(watch.hub.handle());
    watch
        .until("connected and a snapshot", |w| {
            w.is(BOX, "connected") && w.snapshots(BOX) > 0
        })
        .await;
    assert!(watch.any_pane(BOX).is_none());
    // A folder that is not there opens in the machine's home folder, so `~` works as a default.
    wire.new_session(1, "~").expect("the machine takes it");
    watch
        .until("a pane on the machine", |w| w.any_pane(BOX).is_some())
        .await;
    assert_eq!(watch.replies(BOX), 1, "its answer was taken once");
}

#[tokio::test]
async fn the_changes_of_a_pane_on_a_machine_are_read_from_that_machines_daemon_and_nothing_else_is() {
    use maitty_shared::repo::RepoRequest;
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let repository = sandbox.dir.join("checkout");
    std::fs::create_dir_all(&repository).unwrap();
    let init = std::process::Command::new("git").arg("-C").arg(&repository).args(["init", "--quiet"]).status().unwrap();
    assert!(init.success(), "git init");
    std::fs::write(repository.join("new.txt"), "one\n").unwrap();

    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    let wire = Wire::reaching(watch.hub.handle());
    watch
        .until("connected and a snapshot", |w| w.is(BOX, "connected") && w.snapshots(BOX) > 0)
        .await;
    let start = ClientMsg::Command(Command::NewWorkspace {
        cwd: Some(repository.clone()),
        name: None,
        cmd: None,
    });
    watch.hub.handle().send(Outgoing { machine: BOX, msg: start }).expect("the line is up");
    watch.until("a pane on the machine", |w| w.any_pane(BOX).is_some()).await;
    let term = watch.any_pane(BOX).unwrap();

    let not_here = || Err("this computer's daemon is not asked".to_string());
    let daemon = wire.repo_daemon(Some(BOX.0), &RepoRequest::Status, not_here).expect("a connected machine is asked");
    let status = maitty_desktop_client::repo_query(term, RepoRequest::Status, daemon).await.expect("the machine's daemon answers");
    let paths: Vec<&str> = status["Status"]["files"].as_array().unwrap().iter().filter_map(|file| file["path"].as_str()).collect();
    assert_eq!(paths, ["new.txt"]);

    for question in [RepoRequest::Tree { path: None }, RepoRequest::File { path: "new.txt".into() }, RepoRequest::Branches] {
        let refused = wire.repo_daemon(Some(BOX.0), &question, not_here).err().expect("only the changes are asked of a machine");
        assert_eq!(refused, "Inspect is local-only in v1: the files are on box, not on this computer.", "{question:?}");
    }
    assert!(matches!(
        wire.repo_daemon(None, &RepoRequest::Branches, || Ok(std::path::PathBuf::from("maitty"))),
        Ok(maitty_desktop_client::Daemon::Local(_))
    ));
}

#[tokio::test]
async fn a_checkout_on_a_machine_is_pushed_and_pulled_by_that_machines_daemon() {
    use maitty_shared::protocol::GitRequest;
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let (repository, remote) = (sandbox.dir.join("checkout"), sandbox.dir.join("remote.git"));
    let git = |folder: &std::path::Path, args: &[&str]| {
        std::fs::create_dir_all(folder).unwrap();
        let identity = ["-c", "user.name=Test", "-c", "user.email=test@example.invalid", "-c", "commit.gpgsign=false"];
        let ran = std::process::Command::new("git").arg("-C").arg(folder).args(identity).args(args).output().unwrap();
        assert!(ran.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&ran.stderr));
    };
    git(&remote, &["init", "--quiet", "--bare", "-b", "main"]);
    git(&repository, &["init", "--quiet", "-b", "main"]);
    std::fs::write(repository.join("a.txt"), "one\n").unwrap();
    git(&repository, &["add", "--", "a.txt"]);
    git(&repository, &["commit", "--quiet", "-m", "start"]);
    git(&repository, &["remote", "add", "origin", remote.to_str().unwrap()]);

    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    let wire = Wire::reaching(watch.hub.handle());
    watch
        .until("connected and a snapshot", |w| w.is(BOX, "connected") && w.snapshots(BOX) > 0)
        .await;
    let start = ClientMsg::Command(Command::NewWorkspace {
        cwd: Some(repository.clone()),
        name: None,
        cmd: None,
    });
    watch.hub.handle().send(Outgoing { machine: BOX, msg: start }).expect("the line is up");
    watch.until("a pane on the machine", |w| w.any_pane(BOX).is_some()).await;
    let term = watch.any_pane(BOX).unwrap();

    let ask = |request: GitRequest| {
        let daemon = wire.git_daemon(Some(BOX.0)).expect("a connected machine is asked");
        async move { maitty_desktop_client::git_request(term, request, daemon).await.map_err(|error| format!("{error:#}")) }
    };
    assert_eq!(ask(GitRequest::Push).await, Ok("Pushed main".into()));
    assert_eq!(ask(GitRequest::Fetch).await, Ok("Fetched".into()));
    assert_eq!(ask(GitRequest::Pull).await, Ok("Already up to date".into()));
    git(&repository, &["checkout", "--quiet", "--detach"]);
    assert_eq!(ask(GitRequest::Push).await, Err("this checkout is on no branch, so there is nothing to push".into()));
    assert!(wire.git_daemon(None).is_err(), "this computer's checkouts are not synced through its daemon");
}

#[tokio::test]
async fn a_session_is_not_started_on_this_computer_or_a_machine_that_is_down_or_in_a_bad_folder() {
    let sandbox = Sandbox::new();
    let watch = Watch::new(&[machine("box", &sandbox, "auth-fail")], &sandbox, quick(WAIT));
    let wire = Wire::reaching(watch.hub.handle());
    assert_eq!(
        wire.new_session(0, "~").unwrap_err(),
        "This computer starts its own sessions"
    );
    assert_eq!(wire.new_session(1, "~").unwrap_err(), "box is not connected");
    assert_eq!(wire.github_sign_in(0).unwrap_err(), "This computer starts its own sessions");
    assert_eq!(wire.github_sign_in(1).unwrap_err(), "box is not connected");
    for (owner, search) in [(Some("a\nb"), None), (None, Some("x\u{7}")), (Some(&*"o".repeat(300)), None)] {
        assert!(wire.github_repos(Some(1), owner, search, 1).is_err(), "{owner:?} {search:?} was sent");
    }
    for folder in ["", "   ", "/a\nb", &"x".repeat(5000)] {
        assert!(wire.new_session(1, folder).unwrap_err().starts_with("Type a folder"), "{folder:?}");
    }
}
