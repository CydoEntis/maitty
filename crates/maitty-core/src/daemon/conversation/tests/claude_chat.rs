//! A Claude chat session through the daemon (ADR 0003), with the scripted fake `claude`: it is
//! created, owned by the daemon, takes a message, and shows the reply in its history.

use super::*;
use crate::protocol::{ClientMsg, Command, Query, Reply};

#[tokio::test]
async fn a_claude_chat_is_created_sent_a_message_and_read_back_through_the_daemon() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let root = session.fixture.directory.path().to_path_buf();
        let (executable, config_root) = tokio::task::spawn_blocking(move || {
            (fake_claude_executable(&root), std::fs::canonicalize(&root).unwrap())
        })
        .await
        .unwrap();
        let claude = crate::conversation::LaunchSettings {
            provider: crate::conversation::Provider::Claude,
            executable,
            config_root,
            cwd: session.fixture.record.binding.cwd.clone(),
            model: None,
            model_provider: None,
            // What the window always asks for.
            approval_policy: crate::conversation::ApprovalPolicy::OnRequest,
            sandbox: crate::conversation::Sandbox::WorkspaceWrite,
        };
        let started = exchange(
            &mut session,
            420,
            ClientMsg::Command(Command::ConversationStart {
                target: crate::conversation::StartTarget::Create(claude),
                branch: None,
            }),
            None,
        )
        .await;
        let ServerMsg::Reply(Reply::ConversationStarted { id, generation }) = started else {
            panic!("{started:?}");
        };
        let summaries = session.daemon.conversation_summaries();
        let summary = summaries.iter().find(|summary| summary.id == id).unwrap();
        assert_eq!(summary.provider, crate::conversation::Provider::Claude);
        assert_eq!(summary.execution, crate::conversation::ExecutionKind::Structured);
        assert_eq!(summary.native_term, None);

        let sent = exchange(
            &mut session,
            421,
            ClientMsg::Command(Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "hello fake Claude".into(),
            }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(Reply::ConversationSent(_))), "{sent:?}");

        // A send for a generation that is not current changes nothing (ADR 0006).
        let stale = exchange(
            &mut session,
            422,
            ClientMsg::Command(Command::ConversationSend {
                id,
                generation: generation + 1,
                operation: 2,
                text: "hello fake Claude".into(),
            }),
            None,
        )
        .await;
        assert!(matches!(stale, ServerMsg::Error(ref e) if e.contains("Stale")), "{stale:?}");

        let mut texts = Vec::new();
        for attempt in 0..40_u64 {
            let read = exchange(
                &mut session,
                430 + attempt,
                ClientMsg::Query(Query::ConversationRead { id, cursor: None, limit: 128 }),
                None,
            )
            .await;
            let ServerMsg::Reply(Reply::ConversationRead(read)) = read else {
                panic!("{read:?}");
            };
            texts = read
                .history
                .entries
                .iter()
                .filter_map(|entry| match &entry.content {
                    super::history::Content::Message { text, .. } => Some(text.clone()),
                    _ => None,
                })
                .collect();
            if texts.iter().any(|text| text == "fake reply") {
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
        assert!(texts.iter().any(|text| text == "fake reply"), "{texts:?}");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
