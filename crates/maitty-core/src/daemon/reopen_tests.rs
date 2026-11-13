use super::*;
use crate::daemon::branch_agent_tests::{close_all, daemon_with, repo_in, scratch};

/// A repository with a worktree on `feat` beside it, at the usual place.
fn repo_with_worktree(name: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = scratch(name);
    let repo = repo_in(&root);
    let top = root.join("repo-worktrees").join("feat");
    git(&repo, &["worktree", "add", "-q", "-b", "feat", &top.to_string_lossy()]);
    (root, repo, top)
}

fn git(dir: &Path, args: &[&str]) {
    crate::proc::git(dir, args).unwrap();
}

fn record(repo: &Path, top: &Path, cwd: &Path) -> ClosedSession {
    ClosedSession {
        agent: "claude".into(),
        session: "abc-123".into(),
        cwd: cwd.to_path_buf(),
        branch: Some("feat".into()),
        title: None,
        at: 1,
        worktree: Some(top.to_path_buf()),
        repo: Some(repo.to_path_buf()),
    }
}

fn branch_at(dir: &Path) -> Option<String> {
    crate::gitfs::head(dir).map(|h| h.branch)
}

fn same(a: &Path, b: &Path) -> bool {
    a.canonicalize().ok() == b.canonicalize().ok()
}

fn answer(d: &mut Daemon, rx: &mut mpsc::Receiver<Ev>, client_rx: &mut mpsc::Receiver<ServerMsg>) -> ServerMsg {
    let until = Instant::now() + Duration::from_secs(60);
    loop {
        if let Ok(msg) = client_rx.try_recv() {
            return msg;
        }
        assert!(Instant::now() < until, "the daemon never answered");
        match rx.try_recv() {
            Ok(ev) => d.handle(ev),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
}

#[test]
fn a_closed_session_whose_worktree_is_gone_gets_it_back_on_its_branch_and_resumes_there() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _in_rt = rt.enter();
    let (mut d, mut rx) = daemon_with(Config::default());
    let (root, repo, top) = repo_with_worktree("reopen-remade");
    d.command(0, Command::NewWorkspace { cwd: Some(top.clone()), name: None, cmd: None }).unwrap();
    let pane = *d.terms.keys().next().unwrap();
    let t = d.terms.get_mut(&pane).unwrap();
    (t.agent, t.session) = (Some("claude".into()), Some("abc-123".into()));
    d.command(0, Command::ClosePane { term: pane }).unwrap();
    let [kept] = d.closed_sessions.list() else { panic!("one record") };
    assert!(kept.worktree.as_deref().is_some_and(|w| same(w, &top)), "{kept:?}");
    assert!(kept.repo.as_deref().is_some_and(|r| same(r, &repo)), "{kept:?}");
    // Retries while Windows still holds the closed pane's folder.
    crate::daemon::git::remove_worktree(&top, true, false).unwrap();
    assert!(!top.exists());

    let (tx, mut client_rx) = mpsc::channel(64);
    d.handle(Ev::Connected(1, tx, false, crate::protocol::Surface::Terminal, true));
    let reopen = Command::ReopenClosedSession { agent: "claude".into(), session: "abc-123".into() };
    d.handle(Ev::Msg(1, ClientMsg::Command(reopen)));
    let reply = answer(&mut d, &mut rx, &mut client_rx);
    assert!(matches!(reply, ServerMsg::Reply(Reply::Ok)), "{reply:?}");

    assert_eq!(branch_at(&top).as_deref(), Some("feat"));
    let started = d.terms.values().find(|t| t.cmd.as_deref().is_some_and(|c| c.ends_with(" --resume abc-123")));
    assert!(started.is_some_and(|t| same(&t.cwd, &top)), "the agent resumes its conversation in the worktree");
    assert!(d.made_worktrees.iter().any(|m| same(m, &top)), "tidy-up offers it like any worktree the app made");
    assert_eq!(d.pending_ops, 0);
    close_all(&mut d);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_worktree_folder_deleted_by_hand_is_made_again_and_other_missing_ones_stay_registered() {
    let (root, repo, top) = repo_with_worktree("reopen-by-hand");
    let unplugged = root.join("unplugged");
    git(&repo, &["worktree", "add", "-q", "-b", "keep", &unplugged.to_string_lossy()]);
    std::fs::remove_dir_all(&unplugged).unwrap();
    std::fs::remove_dir_all(&top).unwrap();

    let ready = ready_folder(&record(&repo, &top, &top)).expect("made again");
    assert!(same(&ready.folder, &top));
    assert_eq!(branch_at(&top).as_deref(), Some("feat"));
    let registered = git::list_worktrees(&repo).unwrap();
    assert!(
        registered.iter().any(|w| w.branch == "keep"),
        "a worktree whose folder is missing for another reason keeps its registration: {registered:?}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn only_the_parent_folders_made_for_a_worktree_are_removed_again() {
    let root = scratch("reopen-parents");
    let made = make_parents(&root.join("a").join("b").join("top")).unwrap();
    assert!(root.join("a").join("b").is_dir());
    remove_made(&made);
    assert!(!root.join("a").exists(), "the folders it made are gone");
    assert!(root.is_dir(), "the one that was there stays");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_branch_that_is_gone_fails_naming_it_and_nothing_is_made() {
    let (root, repo, top) = repo_with_worktree("reopen-no-branch");
    git(&repo, &["worktree", "remove", &top.to_string_lossy()]);
    git(&repo, &["branch", "-D", "feat"]);

    let error = ready_folder(&record(&repo, &top, &top)).expect_err("refused");
    assert!(error.contains("feat"), "{error}");
    assert!(!top.exists(), "nothing was made");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn whatever_is_at_the_worktree_path_is_left_alone() {
    let (root, repo, top) = repo_with_worktree("reopen-left-alone");
    git(&repo, &["worktree", "remove", &top.to_string_lossy()]);
    let gone_subfolder = top.join("src");

    std::fs::create_dir_all(&top).unwrap();
    std::fs::write(top.join("notes.txt"), "mine").unwrap();
    let error = ready_folder(&record(&repo, &top, &gone_subfolder)).expect_err("a folder that is not that worktree");
    assert!(error.contains("left alone"), "{error}");
    assert_eq!(std::fs::read_to_string(top.join("notes.txt")).unwrap(), "mine");

    std::fs::remove_dir_all(&top).unwrap();
    std::fs::write(&top, "a file").unwrap();
    let error = ready_folder(&record(&repo, &top, &top)).expect_err("a file");
    assert!(error.contains("left alone"), "{error}");
    assert_eq!(std::fs::read_to_string(&top).unwrap(), "a file");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_branch_checked_out_in_another_worktree_is_refused() {
    let (root, repo, top) = repo_with_worktree("reopen-elsewhere");
    git(&repo, &["worktree", "remove", &top.to_string_lossy()]);
    let other = root.join("other");
    git(&repo, &["worktree", "add", "-q", &other.to_string_lossy(), "feat"]);

    let error = ready_folder(&record(&repo, &top, &top)).expect_err("refused");
    assert!(error.contains("checked out"), "{error}");
    assert!(!top.exists(), "nothing was made");
    assert_eq!(branch_at(&other).as_deref(), Some("feat"));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_session_not_kept_one_with_an_unsafe_id_or_a_panes_request_is_refused() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _in_rt = rt.enter();
    let (mut d, _rx) = daemon_with(Config::default());
    let folder = scratch("reopen-refused");
    d.closed_sessions.remember(ClosedSession { session: "-rf; rm".into(), ..record(&folder, &folder, &folder) });
    let reopen = |session: &str| Command::ReopenClosedSession { agent: "claude".into(), session: session.into() };

    let error = d.command(0, reopen("abc-123")).expect_err("not kept").to_string();
    assert!(error.contains("not in the closed sessions"), "{error}");
    let error = d.command(0, reopen("-rf; rm")).expect_err("unsafe id").to_string();
    assert!(error.contains("can't be resumed"), "{error}");
    assert_eq!(d.pending_ops, 0, "nothing was started");

    d.command(0, Command::NewWorkspace { cwd: Some(folder.clone()), name: None, cmd: None }).unwrap();
    let pane = *d.terms.keys().next().unwrap();
    let (tx, mut agent) = mpsc::channel(16);
    d.handle(Ev::Connected(8, tx, false, crate::protocol::Surface::Terminal, true));
    let token = d.terms[&pane].token.clone();
    d.handle(Ev::From(8, pane, token));
    d.message(8, ClientMsg::Command(reopen("-rf; rm")));
    assert!(
        matches!(agent.try_recv(), Ok(ServerMsg::Error(e)) if e.contains("not a pane's")),
        "a pane cannot reopen other work"
    );
    close_all(&mut d);
    let _ = std::fs::remove_dir_all(&folder);
}
