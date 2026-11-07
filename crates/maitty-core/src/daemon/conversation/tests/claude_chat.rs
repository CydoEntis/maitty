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
                choice: None,
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
                choice: None,
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

#[tokio::test]
async fn a_picked_model_and_effort_start_the_next_claude_message_and_a_bad_pick_is_refused() {
    let mut session = conversation_fixture("conversation-choice").await;
    let outcome = AssertUnwindSafe(async {
        let root = session.fixture.directory.path().to_path_buf();
        let (executable, config_root) = tokio::task::spawn_blocking(move || {
            (fake_claude_executable(&root), std::fs::canonicalize(&root).unwrap())
        })
        .await
        .unwrap();
        let arguments_file = executable.with_extension("args");
        let claude = crate::conversation::LaunchSettings {
            provider: crate::conversation::Provider::Claude,
            executable,
            config_root,
            cwd: session.fixture.record.binding.cwd.clone(),
            model: None,
            model_provider: None,
            approval_policy: crate::conversation::ApprovalPolicy::OnRequest,
            sandbox: crate::conversation::Sandbox::WorkspaceWrite,
        };
        let started = exchange(
            &mut session,
            520,
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
        let send = |operation: u64, choice: crate::conversation::TurnChoice| {
            ClientMsg::Command(Command::ConversationSend {
                id,
                generation,
                operation,
                text: "hello fake Claude".into(),
                choice: Some(choice),
            })
        };

        // An option-shaped model never reaches a command line.
        let refused = exchange(
            &mut session,
            521,
            send(1, crate::conversation::TurnChoice { model: Some("--add-dir".into()), effort: None }),
            None,
        )
        .await;
        assert!(matches!(refused, ServerMsg::Error(ref e) if e.contains("model name")), "{refused:?}");

        let sent = exchange(
            &mut session,
            522,
            send(1, crate::conversation::TurnChoice { model: Some("opus".into()), effort: Some("max".into()) }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(Reply::ConversationSent(_))), "{sent:?}");
        let arguments = std::fs::read_to_string(&arguments_file).unwrap();
        let arguments: Vec<&str> = arguments.lines().collect();
        assert!(arguments.windows(2).any(|pair| pair == ["--model", "opus"]), "{arguments:?}");
        assert!(arguments.windows(2).any(|pair| pair == ["--effort", "max"]), "{arguments:?}");
        assert!(arguments.windows(2).any(|pair| pair == ["--permission-mode", "default"]), "{arguments:?}");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
