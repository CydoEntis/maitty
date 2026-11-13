use super::*;

fn closed(session: &str, at: u64) -> ClosedSession {
    ClosedSession {
        agent: "claude".into(),
        session: session.into(),
        cwd: PathBuf::from("/code/api"),
        branch: Some("main".into()),
        title: Some("fix the parser".into()),
        at,
        worktree: None,
        repo: None,
    }
}

fn ids(list: &ClosedSessions) -> Vec<&str> {
    list.list().iter().map(|s| s.session.as_str()).collect()
}

#[test]
fn the_newest_comes_first_and_only_the_newest_fifty_are_kept() {
    let mut list = ClosedSessions::in_memory();
    for n in 0..60 {
        list.remember(closed(&format!("s{n}"), n));
    }
    assert_eq!(list.list().len(), CLOSED_SESSIONS_MAX);
    assert_eq!(list.list()[0].session, "s59");
    assert_eq!(list.list()[CLOSED_SESSIONS_MAX - 1].session, "s10");
}

#[test]
fn a_session_with_no_conversation_id_or_agent_leaves_no_record() {
    let mut list = ClosedSessions::in_memory();
    list.remember(closed("", 1));
    list.remember(closed(" \u{1b}\n", 2));
    list.remember(ClosedSession { agent: String::new(), ..closed("abc", 3) });
    assert!(list.list().is_empty());
}

#[test]
fn the_same_conversation_closing_again_keeps_one_entry_the_newest() {
    let mut list = ClosedSessions::in_memory();
    list.remember(closed("a", 1));
    list.remember(closed("b", 2));
    list.remember(closed("a", 3));
    assert_eq!(ids(&list), ["a", "b"]);
    assert_eq!(list.list()[0].at, 3);
}

#[test]
fn names_are_kept_as_one_clean_short_line() {
    let mut list = ClosedSessions::in_memory();
    list.remember(ClosedSession {
        branch: Some("feat/x\n\u{1b}[31mred".into()),
        title: Some(format!("  fix\u{7} {}", "y".repeat(500))),
        ..closed("a", 1)
    });
    let kept = &list.list()[0];
    let title = kept.title.as_deref().unwrap();
    assert!(title.starts_with("fix "), "{title:?}");
    assert!(title.chars().count() <= crate::conversation::MAX_TITLE_CHARS);
    assert!(!title.chars().any(char::is_control));
    let branch = kept.branch.as_deref().unwrap();
    assert!(!branch.chars().any(char::is_control), "{branch:?}");
}

#[test]
fn the_list_survives_a_restart() {
    let dir = persist::scratch_dir();
    let file = dir.path().join("closed-sessions.json");
    let rt = tokio::runtime::Builder::new_current_thread().build().unwrap();
    let _in_rt = rt.enter();
    let mut list = ClosedSessions::open(file.clone());
    list.remember(closed("a", 1));
    list.remember(closed("b", 2));
    list.save_now();

    let reopened = ClosedSessions::open(file);
    assert_eq!(reopened.list(), list.list());
    assert_eq!(ids(&reopened), ["b", "a"]);
}

#[test]
fn a_stored_list_is_checked_and_put_newest_first_when_read() {
    let dir = persist::scratch_dir();
    std::fs::create_dir_all(dir.path()).unwrap();
    let file = dir.path().join("closed-sessions.json");
    let entry = |session: &str, at: u64| {
        serde_json::json!({
            "agent": "codex", "session": session, "cwd": "/code", "branch": null,
            "title": null, "at": at, "added_later": true,
        })
    };
    let stored = serde_json::json!({
        "version": 1,
        "sessions": [entry("old", 1), entry("", 9), entry("new", 5)],
    });
    std::fs::write(&file, stored.to_string()).unwrap();

    let list = ClosedSessions::open(file);
    assert_eq!(ids(&list), ["new", "old"]);
}

#[test]
fn a_file_that_cannot_be_read_is_kept_aside_and_the_list_starts_empty() {
    let dir = persist::scratch_dir();
    std::fs::create_dir_all(dir.path()).unwrap();
    let file = dir.path().join("closed-sessions.json");
    std::fs::write(&file, "{not json").unwrap();

    let list = ClosedSessions::open(file.clone());
    assert!(list.list().is_empty());
    assert!(!file.exists());
    assert!(dir.path().join("closed-sessions.json.bak").exists());
}

#[test]
fn a_worktree_is_kept_only_when_both_paths_are_absolute_and_the_folder_is_inside_it() {
    let root = std::env::temp_dir();
    let top = root.join("repo-worktrees").join("feat");
    let in_worktree = |cwd: PathBuf, worktree: PathBuf, repo: PathBuf| ClosedSession {
        cwd,
        worktree: Some(worktree),
        repo: Some(repo),
        ..closed("a", 1)
    };
    let mut list = ClosedSessions::in_memory();
    list.remember(in_worktree(top.join("src"), top.clone(), root.join("repo")));
    assert_eq!(list.list()[0].worktree.as_deref(), Some(top.as_path()));
    assert!(list.list()[0].repo.is_some());

    for odd in [
        in_worktree(root.join("elsewhere"), top.clone(), root.join("repo")),
        in_worktree(top.clone(), PathBuf::from("repo-worktrees/feat"), root.join("repo")),
        in_worktree(top.clone(), top.clone(), PathBuf::from("repo")),
    ] {
        list.remember(odd);
        let kept = &list.list()[0];
        assert_eq!((&kept.worktree, &kept.repo), (&None, &None), "{kept:?}");
    }
}
