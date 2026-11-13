//! A chat session started on a branch (ADR 0003): checked before anything is made, started in its
//! own worktree, and owned by the daemon like any other structured conversation.

use super::*;
use crate::protocol::{ClientMsg, Command, Query, Reply};

/// Opening a workspace starts a pane, which writes launchers to the data folder: point it at a
/// scratch one, with the same value the other daemon tests use so the two cannot disagree.
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

/// Makes the fixture's `repo` folder a git repository the daemon is showing, and returns it.
fn shown_repo(session: &mut TestSession) -> PathBuf {
    own_data_folder();
    let repo = session.fixture.record.binding.cwd.clone();
    let git = |args: &[&str]| crate::proc::git(&repo, args).unwrap();
    git(&["init", "-q", "-b", "main"]);
    git(&["config", "user.email", "t@t"]);
    git(&["config", "user.name", "t"]);
    std::fs::write(repo.join("a.txt"), "a").unwrap();
    git(&["add", "."]);
    git(&["commit", "-qm", "one"]);
    session
        .daemon
        .command(
            0,
            Command::NewWorkspace {
                cwd: Some(repo.clone()),
                name: None,
                cmd: None,
            },
        )
        .unwrap();
    repo
}

fn start_on(settings: crate::conversation::LaunchSettings, branch: &str) -> ClientMsg {
    ClientMsg::Command(Command::ConversationStart {
        target: crate::conversation::StartTarget::Create(settings),
        branch: Some(branch.into()),
    })
}

/// Closes every pane and waits until its shell is gone. `close_term` only starts the kill and
/// forgets the pane, so the fixture cleanup can no longer see these shells; on Windows one that
/// is still alive keeps the repo folder as its working directory and blocks its removal.
async fn close_panes(session: &mut TestSession) {
    let pids: Vec<_> = session
        .daemon
        .terms
        .values()
        .filter_map(|term| term.pid)
        .collect();
    for term in session.daemon.terms.keys().copied().collect::<Vec<_>>() {
        session.daemon.close_term(term);
    }
    if !pids.is_empty() {
        stopped(pids).await;
    }
}

/// Starts a chat on `branch` in a new worktree and returns it with its folder.
async fn chat_on(
    session: &mut TestSession,
    client: u64,
    branch: &str,
) -> (crate::conversation::ConversationId, u64, PathBuf) {
    let message = start_on(settings(session), branch);
    let reply = exchange(session, client, message, None).await;
    let ServerMsg::Reply(Reply::ConversationStarted { id, generation }) = reply else {
        panic!("{reply:?}");
    };
    let record = session.daemon.conversations.iter().find(|c| c.id == id).unwrap();
    (id, generation, record.binding.cwd.clone())
}

/// Closes a chat and lets its undo window pass.
async fn close_for_good(
    session: &mut TestSession,
    client: u64,
    id: crate::conversation::ConversationId,
    generation: u64,
) {
    let close = ClientMsg::Command(Command::ConversationClose { id, generation });
    let closed = exchange(session, client, close, None).await;
    assert!(matches!(closed, ServerMsg::Reply(Reply::Ok)), "{closed:?}");
    *session.daemon.conversation_closing.get_mut(&id).unwrap() =
        std::time::Instant::now() - crate::daemon::UNDO_WINDOW;
    session.daemon.finish_chat_closes();
}

fn still_made(session: &TestSession, folder: &std::path::Path) -> bool {
    session
        .daemon
        .made_worktrees
        .iter()
        .any(|made| crate::daemon::same_path(made, folder))
}

/// Handles daemon events until a worktree's automatic removal reports, and returns how it went.
async fn auto_removal(session: &mut TestSession) -> crate::daemon::Removal {
    tokio::time::timeout(Duration::from_secs(60), async {
        loop {
            let event = session.events.recv().await.unwrap();
            if let Ev::WorktreeAutoRemoved { result, .. } = &event {
                let result = result.clone();
                session.daemon.handle(event);
                return result;
            }
            session.daemon.handle(event);
        }
    })
    .await
    .expect("a worktree removal reported")
}

#[tokio::test]
async fn closing_a_chat_for_good_removes_its_clean_merged_worktree_but_not_on_undo() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        shown_repo(&mut session);
        let (id, generation, merged) = chat_on(&mut session, 430, "chat/merged").await;

        // A close that is undone leaves the worktree alone.
        let close = ClientMsg::Command(Command::ConversationClose { id, generation });
        assert!(matches!(exchange(&mut session, 431, close, None).await, ServerMsg::Reply(Reply::Ok)));
        let undo = ClientMsg::Command(Command::ConversationUndoClose { id });
        assert!(matches!(exchange(&mut session, 432, undo, None).await, ServerMsg::Reply(Reply::Ok)));
        session.daemon.finish_chat_closes();
        assert!(still_made(&session, &merged) && merged.is_dir(), "an undone close keeps it");

        close_for_good(&mut session, 433, id, generation).await;
        let gone = auto_removal(&mut session).await;
        assert!(
            matches!(gone, crate::daemon::Removal::Removed { .. }) && !merged.is_dir(),
            "a clean, merged one goes, got: {gone:?}"
        );
    })
    .catch_unwind()
    .await;
    close_panes(&mut session).await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

// One chat per fixture: the synthetic provider gives every chat it starts the same session.
#[tokio::test]
async fn closing_a_chat_for_good_keeps_a_worktree_with_uncommitted_work() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        shown_repo(&mut session);
        let (id, generation, dirty) = chat_on(&mut session, 434, "chat/dirty").await;
        std::fs::write(dirty.join("wip.txt"), "w").unwrap();
        close_for_good(&mut session, 435, id, generation).await;
        let kept = auto_removal(&mut session).await;
        assert!(
            matches!(&kept, crate::daemon::Removal::Kept(why) if why.contains("uncommitted")) && dirty.is_dir(),
            "uncommitted work stays, got: {kept:?}"
        );
    })
    .catch_unwind()
    .await;
    close_panes(&mut session).await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_chat_and_a_pane_in_one_worktree_keep_it_while_either_is_open() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        shown_repo(&mut session);
        let (id, generation, folder) = chat_on(&mut session, 440, "chat/shared").await;
        let open_pane = |session: &mut TestSession| {
            let cwd = Some(folder.clone());
            session
                .daemon
                .command(0, Command::NewWorkspace { cwd, name: None, cmd: None })
                .unwrap();
        };

        // The pane closes first: the chat still works in the folder.
        open_pane(&mut session);
        close_panes(&mut session).await;
        session.daemon.cleanup_worktrees(0, vec![folder.clone()]);
        assert!(still_made(&session, &folder) && folder.is_dir(), "the running chat keeps it");

        // The chat closes first: the pane still works in the folder.
        open_pane(&mut session);
        close_for_good(&mut session, 441, id, generation).await;
        assert!(still_made(&session, &folder) && folder.is_dir(), "the open pane keeps it");

        close_panes(&mut session).await;
        session.daemon.cleanup_worktrees(0, vec![folder.clone()]);
        let gone = auto_removal(&mut session).await;
        assert!(
            matches!(gone, crate::daemon::Removal::Removed { .. }) && !folder.is_dir(),
            "the last one out takes it, got: {gone:?}"
        );
    })
    .catch_unwind()
    .await;
    close_panes(&mut session).await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_chat_on_a_branch_is_refused_before_anything_is_made_when_a_part_is_wrong() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let repo = shown_repo(&mut session);
        let elsewhere = session.fixture.directory.path().join("elsewhere");
        std::fs::create_dir_all(&elsewhere).unwrap();
        let known = settings(&session);
        let mut unknown = known.clone();
        unknown.cwd = elsewhere;
        let before = session.daemon.pending_ops;

        for (client, (message, wanted)) in (400..).zip([
            (start_on(known.clone(), "bad name"), "letters, digits"),
            (start_on(known.clone(), "-x"), "dash"),
            (start_on(unknown, "chat/x"), "not a project"),
            (
                ClientMsg::Command(Command::ConversationStart {
                    target: crate::conversation::StartTarget::Session {
                        settings: known,
                        thread_id: "t".into(),
                    },
                    branch: Some("chat/x".into()),
                }),
                "only a new conversation",
            ),
        ]) {
            match exchange(&mut session, client, message, None).await {
                ServerMsg::Error(error) => assert!(error.contains(wanted), "{error}"),
                other => panic!("{other:?}"),
            }
        }
        assert_eq!(session.daemon.pending_ops, before, "nothing was started");
        assert!(session.daemon.conversations.is_empty());
        assert!(session.daemon.pending_starts.is_empty());
        assert!(crate::daemon::git::branch_names_in_use(&repo).iter().all(|name| name != "chat/x"));
    })
    .catch_unwind()
    .await;
    close_panes(&mut session).await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_chat_on_a_branch_starts_in_its_own_worktree_and_the_daemon_owns_it() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let repo = shown_repo(&mut session);
        let message = start_on(settings(&session), "chat/one");
        let reply = exchange(&mut session, 410, message, None).await;
        let ServerMsg::Reply(Reply::ConversationStarted { id, generation }) = reply else {
            panic!("{reply:?}");
        };

        let record = session
            .daemon
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .unwrap();
        let folder = record.binding.cwd.clone();
        assert_ne!(folder, repo.canonicalize().unwrap(), "not the main checkout");
        assert!(folder.is_dir());
        assert!(crate::gitfs::head(&folder).is_some_and(|head| head.linked));
        assert_eq!(crate::daemon::git::branch_of(&folder).as_deref(), Some("chat/one"));
        // Workspace write, ask on request: the window can choose nothing broader.
        let launched = record.settings.as_ref().unwrap();
        assert_eq!(launched.approval_policy, crate::conversation::ApprovalPolicy::OnRequest);
        assert_ne!(launched.sandbox, crate::conversation::Sandbox::FullAccess);
        assert!(session.daemon.made_worktrees.iter().any(|made| crate::daemon::same_path(made, &folder)));
        assert!(session.daemon.conversation_runtimes.owns(id));

        // The ordinary send path works in it.
        let sent = exchange(
            &mut session,
            411,
            ClientMsg::Command(Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic prompt".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(Reply::ConversationSent(_))), "{sent:?}");

        let summaries = session.daemon.conversation_summaries();
        let summary = summaries.iter().find(|summary| summary.id == id).unwrap();
        assert_eq!(summary.execution, crate::conversation::ExecutionKind::Structured);
        assert_eq!(summary.native_term, None);
        assert_eq!(summary.cwd, folder);
        // The list groups it with its project and shows its branch.
        assert_eq!(summary.branch.as_deref(), Some("chat/one"));
        assert_eq!(
            summary.root.as_ref().map(|root| root.canonicalize().unwrap()),
            Some(repo.canonicalize().unwrap())
        );

        // A branch that is taken says so and starts nothing.
        let message = start_on(settings(&session), "chat/one");
        let again = exchange(&mut session, 412, message, None).await;
        assert!(matches!(again, ServerMsg::Error(ref e) if e.contains("already exists") || e.contains("checked out")), "{again:?}");
        assert_eq!(session.daemon.conversations.len(), 1);

        let read = exchange(
            &mut session,
            413,
            ClientMsg::Query(Query::ConversationRead { id, cursor: None, limit: 128 }),
            None,
        )
        .await;
        assert!(matches!(read, ServerMsg::Reply(Reply::ConversationRead(_))));
    })
    .catch_unwind()
    .await;
    close_panes(&mut session).await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
