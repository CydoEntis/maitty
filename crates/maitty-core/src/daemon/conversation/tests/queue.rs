//! The daemon holds a message sent while a turn is running and delivers it when the turn ends.

use super::*;
use crate::conversation::{Delivery, QueuedMessage};
use crate::protocol::{ClientMsg, Command, Query, Reply};

pub(super) async fn send(
    session: &mut TestSession,
    client: u64,
    id: crate::conversation::ConversationId,
    generation: u64,
    operation: u64,
    text: &str,
) -> ServerMsg {
    exchange(
        session,
        client,
        ClientMsg::Command(Command::ConversationSend {
            id,
            generation,
            operation,
            text: text.into(),
            choice: None,
        }),
        None,
    )
    .await
}

pub(super) async fn read(
    session: &mut TestSession,
    client: u64,
    id: crate::conversation::ConversationId,
) -> crate::conversation::ReadResult {
    match exchange(
        session,
        client,
        ClientMsg::Query(Query::ConversationRead {
            id,
            cursor: None,
            limit: 128,
        }),
        None,
    )
    .await
    {
        ServerMsg::Reply(Reply::ConversationRead(read)) => read,
        other => panic!("{other:?}"),
    }
}

pub(super) fn queued(read: &crate::conversation::ReadResult) -> Vec<(u64, String)> {
    read.queued
        .iter()
        .map(|QueuedMessage { operation, text }| (*operation, text.clone()))
        .collect()
}

/// Lets the daemon deliver what is waiting, as its tick does, until nothing is waiting or in flight.
async fn deliver_until_settled(session: &mut TestSession, id: crate::conversation::ConversationId) {
    tokio::time::timeout(Duration::from_secs(20), async {
        loop {
            session.daemon.deliver_queued();
            let waiting = session
                .daemon
                .conversation_queues
                .get(&id)
                .is_some_and(|queue| !queue.is_empty());
            let in_flight = session
                .daemon
                .conversation_requests
                .values()
                .any(|pending| pending.conversation() == id);
            if !waiting && !in_flight {
                break;
            }
            let event = tokio::time::timeout(Duration::from_millis(200), session.events.recv()).await;
            if let Ok(Some(event)) = event {
                session.daemon.handle(event);
            }
        }
    })
    .await
    .expect("the queue drains");
}

pub(super) async fn turns_started(session: &TestSession) -> usize {
    let path = session.fixture.directory.path().join("requests");
    tokio::fs::read_to_string(path)
        .await
        .map_or(0, |text| text.lines().count())
}

#[tokio::test]
async fn a_message_sent_during_a_turn_waits_in_the_daemon_and_is_delivered_once_in_order() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let first = send(&mut session, 400, id, generation, 1, "first").await;
        assert!(matches!(first, ServerMsg::Reply(Reply::ConversationSent(_))));
        assert_eq!(turns_started(&session).await, 1);

        // The turn is running: these are held, not refused.
        let second = send(&mut session, 401, id, generation, 2, "second").await;
        let third = send(&mut session, 402, id, generation, 3, "third").await;
        for reply in [&second, &third] {
            assert!(matches!(reply, ServerMsg::Reply(Reply::ConversationQueued(_))), "{reply:?}");
        }
        // Another client, long after the sender went away, sees them and the order.
        assert_eq!(
            queued(&read(&mut session, 403, id).await),
            [(2, "second".to_string()), (3, "third".to_string())]
        );

        // A retry of a queued message is the same message, not a second one.
        let retry = send(&mut session, 404, id, generation, 2, "second").await;
        assert!(matches!(retry, ServerMsg::Reply(Reply::ConversationQueued(ref q)) if q.operation == 2));
        // The same number with other text, or an older number, is refused.
        let reused = send(&mut session, 405, id, generation, 2, "different").await;
        assert!(matches!(reused, ServerMsg::Error(ref e) if e.contains("reused")), "{reused:?}");
        let old = send(&mut session, 406, id, generation, 1, "first").await;
        assert!(!matches!(old, ServerMsg::Reply(Reply::ConversationQueued(_))));
        assert_eq!(queued(&read(&mut session, 407, id).await).len(), 2);

        // Taking one back out; taking it twice says it is gone.
        let removed = exchange(
            &mut session,
            408,
            ClientMsg::Command(Command::ConversationDequeue { id, generation, operation: 3 }),
            None,
        )
        .await;
        assert!(matches!(removed, ServerMsg::Reply(Reply::Ok)));
        let again = exchange(
            &mut session,
            409,
            ClientMsg::Command(Command::ConversationDequeue { id, generation, operation: 3 }),
            None,
        )
        .await;
        assert!(matches!(again, ServerMsg::Error(ref e) if e.contains("no longer waiting")));
        assert_eq!(queued(&read(&mut session, 410, id).await), [(2, "second".to_string())]);

        // Nothing is delivered into the running turn, however often the daemon looks.
        for _ in 0..3 {
            session.daemon.deliver_queued();
        }
        assert_eq!(turns_started(&session).await, 1);

        // The turn ends (interrupted); the daemon sends the waiting message by itself.
        let stopped = exchange(
            &mut session,
            411,
            ClientMsg::Command(Command::ConversationCancel { id, generation }),
            None,
        )
        .await;
        assert!(matches!(stopped, ServerMsg::Reply(Reply::Ok)));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].cancelling.is_some() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("the interrupted turn completes");
        deliver_until_settled(&mut session, id).await;

        assert_eq!(turns_started(&session).await, 2, "delivered once");
        let after = read(&mut session, 412, id).await;
        assert!(after.queued.is_empty());
        let receipt = after.receipt.expect("a receipt for the delivered message");
        assert_eq!(receipt.operation, 2);
        assert!(matches!(receipt.delivery, Delivery::Acknowledged { .. }));
        // The removed message was never sent.
        for _ in 0..3 {
            session.daemon.deliver_queued();
        }
        assert_eq!(turns_started(&session).await, 2);

        // The sender retries after the fact: the same number and text is answered, not sent again.
        let late = send(&mut session, 413, id, generation, 2, "second").await;
        assert!(matches!(late, ServerMsg::Reply(Reply::ConversationSent(ref r)) if r.operation == 2));
        assert_eq!(turns_started(&session).await, 2);
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_message_for_a_conversation_that_started_over_is_not_delivered_into_the_new_one() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        send(&mut session, 420, id, generation, 1, "first").await;
        let held = send(&mut session, 421, id, generation, 2, "stale soon").await;
        assert!(matches!(held, ServerMsg::Reply(Reply::ConversationQueued(_))));

        session.daemon.conversations.iter_mut().find(|c| c.id == id).unwrap().generation += 1;
        session.daemon.deliver_queued();

        assert!(session.daemon.conversation_queues[&id].is_empty());
        assert_eq!(turns_started(&session).await, 1);
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_pane_cannot_remove_a_queued_message() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        send(&mut session, 430, id, generation, 1, "first").await;
        send(&mut session, 431, id, generation, 2, "second").await;
        let from = authenticated_pane(&mut session);

        let refused = exchange(
            &mut session,
            432,
            ClientMsg::Command(Command::ConversationDequeue { id, generation, operation: 2 }),
            Some(from),
        )
        .await;

        assert!(matches!(refused, ServerMsg::Error(ref e) if e.contains("Only a human")), "{refused:?}");
        assert_eq!(queued(&read(&mut session, 433, id).await).len(), 1);
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
