//! "New agent on a branch" through a real daemon: what is refused, and what starts.

use super::branch_agent::MAX_PROMPT_CHARS;
use super::*;
use std::path::Path;

/// Panes write launchers and startup files to the data folder: point it at a scratch one, as the
/// other daemon tests do (the same value, so the two cannot disagree).
fn own_data_folder() {
    static ONCE: std::sync::Once = std::sync::Once::new();
    ONCE.call_once(|| {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-test-data-{}"),
            std::process::id()
        ));
        // SAFETY: the first thing a daemon test does, before its panes read the environment.
        unsafe { std::env::set_var("XDG_DATA_HOME", dir) };
    });
}

pub(super) fn daemon_with(cfg: Config) -> (Daemon, mpsc::Receiver<Ev>) {
    own_data_folder();
    let (tx, rx) = mpsc::channel(4096);
    let mut d = Daemon::new(cfg, tx, || {});
    d.check_dir = persist::scratch_dir().path().to_path_buf();
    (d, rx)
}

pub(super) fn scratch(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-{}-{}"),
        name,
        std::process::id()
    ));
    let _ = std::fs::remove_dir_all(&root);
    std::fs::create_dir_all(&root).unwrap();
    root
}

pub(super) fn repo_in(root: &Path) -> PathBuf {
    let repo = root.join("repo");
    std::fs::create_dir_all(&repo).unwrap();
    let git = |args: &[&str]| crate::proc::git(&repo, args).unwrap();
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(repo.join("a.txt"), "a").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "one"]);
    repo
}

fn open_workspace(d: &mut Daemon, dir: &Path) -> WsId {
    d.command(
        0,
        Command::NewWorkspace {
            cwd: Some(dir.to_path_buf()),
            name: None,
            cmd: None,
        },
    )
    .unwrap();
    d.workspaces.last().unwrap().id
}

fn request(
    ws: WsId,
    repo: &Path,
    branch: &str,
    agent: Option<&str>,
    prompt: Option<&str>,
) -> Command {
    Command::NewWorktree {
        ws,
        branch: branch.into(),
        base: None,
        cmd: None,
        split: None,
        from: Some(repo.to_path_buf()),
        agent: agent.map(String::from),
        prompt: prompt.map(String::from),
    }
}

pub(super) fn close_all(d: &mut Daemon) {
    for t in d.terms.keys().copied().collect::<Vec<_>>() {
        d.close_term(t);
    }
}

#[test]
fn a_request_is_refused_before_anything_is_made_when_a_part_is_wrong() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _in_rt = rt.enter();
    let (mut d, _rx) = daemon_with(Config::default());
    let root = scratch("branch-agent-refuse");
    let repo = repo_in(&root);
    let ws = open_workspace(&mut d, &repo);
    let elsewhere = root.join("elsewhere");
    std::fs::create_dir_all(&elsewhere).unwrap();
    let before = d.pending_ops;

    let both = Command::NewWorktree {
        ws,
        branch: "agent/x".into(),
        base: None,
        cmd: Some("claude".into()),
        split: None,
        from: Some(repo.clone()),
        agent: Some("claude".into()),
        prompt: None,
    };
    let too_long = "x".repeat(MAX_PROMPT_CHARS + 1);
    let cases = [
        (
            request(ws, &repo, "bad name", Some("claude"), None),
            "letters, digits",
        ),
        (
            request(ws, &repo, "a..b", Some("claude"), None),
            "cannot hold ..",
        ),
        (
            request(ws, &repo, "agent/x", Some("rm"), Some("hi")),
            "unknown agent",
        ),
        (
            request(ws, &repo, "agent/x", Some("claude"), Some("--oops")),
            "dash",
        ),
        (
            request(ws, &repo, "agent/x", Some("claude"), Some(&too_long)),
            "at most",
        ),
        (
            request(ws, &elsewhere, "agent/x", Some("claude"), None),
            "not a project",
        ),
        (both, "not both"),
    ];
    for (command, why) in cases {
        let error = d.command(0, command).expect_err("refused").to_string();
        assert!(error.contains(why), "{error:?} should mention {why:?}");
    }
    assert_eq!(d.pending_ops, before, "nothing was started");
    assert!(!root.join("repo-worktrees").exists(), "and no folder made");
    close_all(&mut d);
    let _ = std::fs::remove_dir_all(&root);
}

/// A fake provider: it records the one argument it was given, as a real agent would receive it.
#[cfg(unix)]
#[test]
fn the_agent_starts_in_a_new_worktree_with_the_prompt_as_one_argument() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _in_rt = rt.enter();
    let root = scratch("branch-agent-start");
    let repo = repo_in(&root);
    let recorded = root.join("recorded.txt");
    let mut cfg = Config::default();
    cfg.quick.agents.push(crate::config::QuickAgent {
        name: "fake".into(),
        command: format!("printf '%s' {{prompt}} > {}", recorded.display()),
        ..Default::default()
    });
    let (mut d, mut rx) = daemon_with(cfg);
    let ws = open_workspace(&mut d, &repo);

    let prompt = "it's $(touch pwned) `x` \"y\"\nsecond line";
    d.command(
        0,
        request(ws, &repo, "agent/fix-login", Some("fake"), Some(prompt)),
    )
    .unwrap();
    let until = Instant::now() + Duration::from_secs(60);
    while !recorded.exists() && Instant::now() < until {
        match rx.try_recv() {
            Ok(ev) => d.handle(ev),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    }
    let worktree = root.join("repo-worktrees").join("agent-fix-login");
    assert!(
        worktree.is_dir(),
        "the usual worktree folder, branch slashes made safe"
    );
    assert_eq!(
        crate::gitfs::head(&worktree).map(|h| h.branch).as_deref(),
        Some("agent/fix-login")
    );
    assert_eq!(std::fs::read_to_string(&recorded).unwrap(), prompt);
    assert!(
        !worktree.join("pwned").exists(),
        "nothing in the prompt ran"
    );
    let opened = d.workspaces.last().unwrap();
    assert!(opened.worktree && same_path(&opened.cwd, &worktree));
    close_all(&mut d);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_worktree_asked_by_folder_is_refused_before_anything_is_made_when_it_is_wrong() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _in_rt = rt.enter();
    let (mut d, _rx) = daemon_with(Config::default());
    let root = scratch("branch-worktree-refuse");
    let repo = repo_in(&root);
    open_workspace(&mut d, &repo);
    let plain = root.join("plain");
    std::fs::create_dir_all(&plain).unwrap();
    open_workspace(&mut d, &plain);
    let unseen = root.join("unseen");
    std::fs::create_dir_all(&unseen).unwrap();
    let before = d.pending_ops;

    let ask = |project: &Path, branch: &str| Command::BranchWorktree {
        project: project.to_path_buf(),
        branch: branch.into(),
    };
    let cases = [
        (ask(&repo, "bad name"), "letters, digits"),
        (ask(&repo, "a..b"), "cannot hold .."),
        (ask(&unseen, "feat/x"), "not a project"),
        (ask(&plain, "feat/x"), "not in a git repository"),
    ];
    for (command, why) in cases {
        let error = d.command(0, command).expect_err("refused").to_string();
        assert!(error.contains(why), "{error:?} should mention {why:?}");
    }
    assert_eq!(d.pending_ops, before, "nothing was started");
    assert!(d.made_worktrees.is_empty());
    assert!(!root.join("repo-worktrees").exists(), "and no folder made");
    close_all(&mut d);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn a_worktree_asked_by_folder_is_made_tracked_and_answered_with_its_folder() {
    let rt = tokio::runtime::Runtime::new().unwrap();
    let _in_rt = rt.enter();
    let (mut d, mut rx) = daemon_with(Config::default());
    let root = scratch("branch-worktree-make");
    let repo = repo_in(&root);
    open_workspace(&mut d, &repo);
    let (tx, mut client_rx) = mpsc::channel(64);
    d.handle(Ev::Connected(
        1,
        tx,
        false,
        crate::protocol::Surface::Terminal,
        true,
    ));
    let before = d.pending_ops;

    d.handle(Ev::Msg(
        1,
        ClientMsg::Command(Command::BranchWorktree {
            project: repo.clone(),
            branch: "feat/x".into(),
        }),
    ));
    let until = Instant::now() + Duration::from_secs(60);
    let answer = loop {
        if let Ok(msg) = client_rx.try_recv() {
            break msg;
        }
        assert!(Instant::now() < until, "the daemon never answered");
        match rx.try_recv() {
            Ok(ev) => d.handle(ev),
            Err(_) => std::thread::sleep(Duration::from_millis(20)),
        }
    };
    let ServerMsg::Reply(Reply::Text(folder)) = answer else {
        panic!("expected the new folder, got {answer:?}")
    };
    let folder = PathBuf::from(folder);
    let expected = root.join("repo-worktrees").join("feat-x");
    // Both exist; compared as the filesystem names them, since a temp folder can be spelled with a
    // short (8.3) user name on Windows.
    assert_eq!(
        folder.canonicalize().unwrap(),
        expected.canonicalize().unwrap(),
        "{folder:?} is the usual worktree folder"
    );
    assert_eq!(
        crate::gitfs::head(&folder).map(|h| h.branch).as_deref(),
        Some("feat/x")
    );
    assert!(
        d.made_worktrees
            .iter()
            .any(|made| made.canonicalize().ok() == expected.canonicalize().ok()),
        "listed with the checkouts the app made, so tidy-up offers it"
    );
    assert_eq!(d.pending_ops, before, "the operation is over");
    close_all(&mut d);
    let _ = std::fs::remove_dir_all(&root);
}
