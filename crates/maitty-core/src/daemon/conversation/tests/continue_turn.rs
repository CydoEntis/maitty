//! With `restore.continue_turn` on, a conversation the daemon resumed is told once to continue.

use super::*;
use crate::conversation::{ConversationId, Delivery, RecoveryState};
use crate::daemon::conversation::continue_turn::{Continues, message_digest};
use crate::daemon::persist::Saved;

/// Restores `saved` as a daemon start does, resumes, and waits until the resume and anything
/// it sent have finished.
async fn restart(session: &mut TestSession, id: ConversationId, saved: Saved) {
    session.daemon.cfg.restore.enabled = true;
    assert!(session.daemon.restore(saved));
    session.daemon.resume_conversations();
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            let in_flight = session
                .daemon
                .conversation_requests
                .values()
                .any(|pending| pending.conversation() == id);
            let ready = session
                .daemon
                .conversation_states
                .get(&id)
                .is_some_and(|state| state.history_ready);
            if session.daemon.conversation_runtimes.owns(id) && ready && !in_flight {
                break;
            }
            if let Some(event) = session.events.recv().await {
                session.daemon.handle(event);
            }
        }
    })
    .await
    .expect("the conversation resumes");
}

async fn turns_started(session: &TestSession) -> usize {
    let path = session.fixture.directory.path().join("requests");
    tokio::fs::read_to_string(path)
        .await
        .map_or(0, |text| text.lines().count())
}

/// A session file holding one conversation, on the thread the synthetic provider resumes.
fn one(mut record: crate::conversation::Conversation) -> Saved {
    record.binding.session_id = "exact-thread".into();
    Saved {
        conversations: vec![record],
        ..Default::default()
    }
}

#[tokio::test]
async fn a_conversation_cut_by_a_restart_is_told_once_to_continue_after_it_resumes() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        let saved = one(running_record(&s, id.0));
        restart(&mut s, id, saved).await;

        assert_eq!(turns_started(&s).await, 1, "one message reached the provider");
        let sent = &s.daemon.conversation_operations[0];
        assert_eq!(sent.digest, message_digest());
        assert_eq!(sent.receipt.generation, s.daemon.conversations[0].generation);
        assert!(matches!(sent.receipt.delivery, Delivery::Acknowledged { .. }));

        let file = s.daemon.session_path.clone();
        let on_disk: Saved =
            serde_json::from_str(&tokio::fs::read_to_string(file).await.unwrap()).unwrap();
        assert_eq!(on_disk.turn_continues.count(id), 1, "the next daemon will know of it");

        s.daemon.resume_conversations();
        assert_eq!(turns_started(&s).await, 1, "a second call sends nothing");
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_resumed_conversation_is_sent_nothing_unless_the_setting_is_on() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        assert!(!s.daemon.cfg.restore.continue_turn, "off by default");
        let saved = one(running_record(&s, id.0));
        restart(&mut s, id, saved).await;

        assert_eq!(turns_started(&s).await, 0);
        assert!(s.daemon.conversation_operations.is_empty());
        assert_eq!(s.daemon.saved().turn_continues, Continues::default());
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_daemon_restarting_in_a_loop_stops_telling_a_conversation_to_continue() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        // What three earlier daemon starts left in the session file moments ago.
        let saved = Saved {
            turn_continues: Continues::full(id, crate::daemon::term::unix_now()),
            ..one(running_record(&s, id.0))
        };
        restart(&mut s, id, saved).await;

        assert_eq!(turns_started(&s).await, 0, "the fourth in five minutes is not sent");
        assert!(s.daemon.conversation_operations.is_empty());
        assert_eq!(s.daemon.saved().turn_continues.count(id), 3);
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_conversation_that_was_mid_transfer_is_resumed_but_not_told_to_continue() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        let mut record = running_record(&s, id.0);
        record.state = RecoveryState::Transferring;
        restart(&mut s, id, one(record)).await;

        assert_eq!(turns_started(&s).await, 0);
        assert!(s.daemon.conversation_operations.is_empty());
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_start_a_person_asked_for_is_never_followed_by_the_message() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        let saved = one(running_record(&s, id.0));
        s.daemon.cfg.restore.conversations = false;
        restart_without_resume(&mut s, saved);
        s.daemon.cfg.restore.conversations = true;

        let generation = s.daemon.conversations[0].generation;
        let started = exchange(
            &mut s,
            500,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target: crate::conversation::StartTarget::Conversation { id, generation },
                branch: None,
            }),
            None,
        )
        .await;
        assert!(
            matches!(started, ServerMsg::Reply(crate::protocol::Reply::ConversationStarted { .. })),
            "{started:?}"
        );
        assert_eq!(turns_started(&s).await, 0);
        assert!(s.daemon.conversation_operations.is_empty());
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

fn restart_without_resume(session: &mut TestSession, saved: Saved) {
    session.daemon.cfg.restore.enabled = true;
    assert!(session.daemon.restore(saved));
    session.daemon.resume_conversations();
}
