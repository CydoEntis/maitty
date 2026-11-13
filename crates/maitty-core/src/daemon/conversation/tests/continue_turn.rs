//! With `restore.continue_turn` on, a conversation the daemon resumed is told once to continue.

use super::*;
use crate::conversation::{ConversationId, Delivery, RecoveryState};
use crate::daemon::conversation::commands::DAEMON_REQUESTER;
use crate::daemon::conversation::continue_turn::{Continues, message_digest};
use crate::daemon::persist::Saved;

/// Restores `saved` as a daemon start does, resumes, and waits until the resume and anything
/// it sent have finished.
async fn restart(session: &mut TestSession, id: ConversationId, saved: Saved) {
    restore(session, saved);
    session.daemon.resume_conversations();
    settle(session, id).await;
}

fn restore(session: &mut TestSession, saved: Saved) {
    session.daemon.cfg.restore.enabled = true;
    assert!(session.daemon.restore(saved));
}

/// Handles the daemon's events until `id` runs with its history read and nothing is in flight.
async fn settle(session: &mut TestSession, id: ConversationId) {
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

/// A session file holding one conversation that was idle, on the thread the synthetic provider
/// resumes.
fn one(mut record: crate::conversation::Conversation) -> Saved {
    record.binding.session_id = "exact-thread".into();
    Saved {
        conversations: vec![record],
        ..Default::default()
    }
}

/// The same, with a turn in flight when it was saved.
fn cut(record: crate::conversation::Conversation) -> Saved {
    let id = record.id;
    Saved {
        turns_in_flight: vec![id],
        ..one(record)
    }
}

/// Sends nothing more: handles events for a moment, then counts the turns started.
async fn nothing_more_sent(session: &mut TestSession, id: ConversationId, before: usize) {
    settle(session, id).await;
    assert_eq!(turns_started(session).await, before);
}

#[tokio::test]
async fn a_conversation_cut_by_a_restart_is_told_once_to_continue_after_it_resumes() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        let saved = cut(running_record(&s, id.0));
        restart(&mut s, id, saved).await;

        assert_eq!(turns_started(&s).await, 1, "one message reached the provider");
        let sent = &s.daemon.conversation_operations[0];
        assert_eq!(sent.digest, message_digest());
        let generation = s.daemon.conversations[0].generation;
        assert_eq!(sent.receipt.generation, generation);
        assert!(matches!(sent.receipt.delivery, Delivery::Acknowledged { .. }));

        let file = s.daemon.session_path.clone();
        let on_disk: Saved =
            serde_json::from_str(&tokio::fs::read_to_string(file).await.unwrap()).unwrap();
        assert_eq!(on_disk.turn_continues.count(id), 1, "the next daemon will know of it");

        // The resume's start completing again at the same generation sends no second one.
        s.daemon.continue_cut_turn(DAEMON_REQUESTER, id, generation);
        nothing_more_sent(&mut s, id, 1).await;
        assert_eq!(s.daemon.saved().turn_continues.count(id), 1);
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn an_idle_conversation_is_resumed_but_not_told_to_continue() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        let saved = one(running_record(&s, id.0));
        restart(&mut s, id, saved).await;

        assert!(s.daemon.conversation_runtimes.owns(id), "it was resumed");
        assert_eq!(turns_started(&s).await, 0, "its turn had ended; nothing to continue");
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
async fn the_session_file_names_only_conversations_with_a_turn_in_flight() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        let record = running_record(&s, id.0);
        s.daemon.conversations.push(record);
        let state = s.daemon.conversation_states.entry(id).or_default();
        state.status = crate::protocol::Status::Idle;
        state.history_ready = true;
        assert!(s.daemon.saved().turns_in_flight.is_empty(), "an idle chat");

        s.daemon.conversation_states.entry(id).or_default().status =
            crate::protocol::Status::Working;
        assert_eq!(s.daemon.saved().turns_in_flight, [id], "a chat mid-turn");

        s.daemon.conversation_states.entry(id).or_default().history_ready = false;
        assert!(
            s.daemon.saved().turns_in_flight.is_empty(),
            "a chat still starting up has no turn yet"
        );
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
        let saved = cut(running_record(&s, id.0));
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
async fn turning_the_setting_off_while_a_resume_runs_sends_nothing() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        let saved = cut(running_record(&s, id.0));
        restore(&mut s, saved);
        s.daemon.resume_conversations();
        s.daemon.cfg.restore.continue_turn = false;
        settle(&mut s, id).await;

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
            ..cut(running_record(&s, id.0))
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
        restart(&mut s, id, cut(record)).await;

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
        let saved = cut(running_record(&s, id.0));
        restore(&mut s, saved);
        let generation = s.daemon.conversations[0].generation;
        // The daemon expects the message to follow the start that runs at the next generation;
        // that start is the person's, so only who asked for it stands in the way.
        s.daemon.turns_to_continue.insert(id, generation + 1);

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
        assert_eq!(s.daemon.conversations[0].generation, generation + 1);
        nothing_more_sent(&mut s, id, 0).await;
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
async fn a_resume_at_a_later_generation_than_the_one_noted_sends_nothing() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = s.fixture.record.id;
        s.daemon.cfg.restore.continue_turn = true;
        let saved = cut(running_record(&s, id.0));
        restore(&mut s, saved);
        let generation = s.daemon.conversations[0].generation;
        // Noted for this generation, which the daemon's own start below supersedes.
        s.daemon.continue_when_resumed(id);

        s.daemon
            .conversation_start(
                DAEMON_REQUESTER,
                crate::conversation::StartTarget::Conversation { id, generation },
            )
            .unwrap();
        nothing_more_sent(&mut s, id, 0).await;
        assert_eq!(s.daemon.conversations[0].generation, generation + 1);
        assert!(s.daemon.conversation_operations.is_empty());
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
