//! A chat the daemon runs keeps the checkout's files as they were when a turn began.

use super::*;
use crate::protocol::{ClientMsg, Command, Reply};

fn repo_with_a_changed_file(cwd: &Path) {
    for args in [
        &["init", "-q"][..],
        &["config", "user.name", "t"],
        &["config", "user.email", "t@t"],
    ] {
        crate::proc::git(cwd, args).unwrap();
    }
    std::fs::write(cwd.join("kept.txt"), "one\n").unwrap();
    crate::proc::git(cwd, &["add", "-A"]).unwrap();
    crate::proc::git(cwd, &["commit", "-qm", "first"]).unwrap();
    std::fs::write(cwd.join("kept.txt"), "one\ntwo\n").unwrap();
}

/// The snapshot is taken off the loop, so it is listed a moment after the send is answered.
async fn snapshots_once_listed(cwd: PathBuf) -> Vec<crate::snapshot::Snapshot> {
    tokio::task::spawn_blocking(move || {
        let until = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            let kept = crate::snapshot::list(&cwd);
            if !kept.is_empty() || std::time::Instant::now() >= until {
                return kept;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    })
    .await
    .unwrap()
}

#[tokio::test]
async fn a_turn_starting_in_a_repo_with_a_changed_file_keeps_a_snapshot() {
    // A turn that stays running: one that ends at once with nothing changed would keep its
    // single snapshot under either name.
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let cwd = session.fixture.record.binding.cwd.clone();
        let repo = cwd.clone();
        tokio::task::spawn_blocking(move || repo_with_a_changed_file(&repo))
            .await
            .unwrap();
        let (id, generation) = create(&mut session).await;
        assert!(
            crate::snapshot::list(&cwd).is_empty(),
            "starting the chat is not a turn"
        );

        let sent = exchange(
            &mut session,
            500,
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

        let kept = snapshots_once_listed(cwd.clone()).await;
        assert_eq!(
            kept.iter().map(|s| s.why.as_str()).collect::<Vec<_>>(),
            ["before codex's turn"]
        );
        let changed = cwd.clone();
        let in_snapshot = tokio::task::spawn_blocking(move || {
            crate::proc::git(&changed, &["show", &format!("{}:kept.txt", kept[0].commit)])
        })
        .await
        .unwrap();
        assert_eq!(in_snapshot.as_deref(), Ok("one\ntwo\n"));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
