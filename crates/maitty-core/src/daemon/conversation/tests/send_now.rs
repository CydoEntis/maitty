//! A queued message sent now: steered into the running Codex turn when Codex can take it there,
//! otherwise by stopping the turn and sending the message next.

use super::queue::{queued, read, send, turns_started};
use super::*;
use crate::conversation::{ConversationId, Delivery, ProviderRequestId};
use crate::protocol::{ClientMsg, Command, Reply};

const STEERING_SCHEMA: &str = include_str!(
    "../../../../../../crates/adapters/maitty-provider-codex/src/fixtures/codex-0.166.0.json"
);

async fn send_now(
    session: &mut TestSession,
    client: u64,
    id: ConversationId,
    generation: u64,
    operation: u64,
) -> ServerMsg {
    exchange(
        session,
        client,
        ClientMsg::Command(Command::ConversationSendNow { id, generation, operation }),
        None,
    )
    .await
}

/// A conversation on a Codex whose schema declares `turn/steer`, once the daemon knows it.
async fn started_with_steering(session: &mut TestSession) -> (ConversationId, u64) {
    let inventory: Value = serde_json::from_str(STEERING_SCHEMA).unwrap();
    for (name, section) in [("ClientRequest.json", "clientRequests"), ("ServerRequest.json", "serverRequests")] {
        tokio::fs::write(session.fixture.directory.path().join(name), inventory[section].to_string())
            .await
            .unwrap();
    }
    let (id, generation) = create(session).await;
    pump_until(session, |s| s.daemon.conversation_runtimes.can_steer(id, generation)).await;
    (id, generation)
}

async fn pump_until(session: &mut TestSession, done: impl Fn(&mut TestSession) -> bool) {
    tokio::time::timeout(Duration::from_secs(20), async {
        while !done(session) {
            let event = tokio::time::timeout(Duration::from_millis(200), session.events.recv()).await;
            if let Ok(Some(event)) = event {
                session.daemon.handle(event);
            }
        }
    })
    .await
    .expect("the daemon reached the state the test waits for");
}

async fn lines(session: &TestSession, name: &str) -> Vec<String> {
    tokio::fs::read_to_string(session.fixture.directory.path().join(name))
        .await
        .map(|text| text.lines().map(str::to_owned).collect())
        .unwrap_or_default()
}

fn waiting(session: &TestSession, id: ConversationId) -> usize {
    session.daemon.conversation_queues.get(&id).map_or(0, Vec::len)
}

async fn finish(session: TestSession, outcome: Result<(), Box<dyn std::any::Any + Send>>) {
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn codex_with_steering_takes_the_message_into_the_running_turn() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = started_with_steering(&mut session).await;
        send(&mut session, 600, id, generation, 1, "first").await;
        send(&mut session, 601, id, generation, 2, "use the other file").await;
        send(&mut session, 602, id, generation, 3, "and then rest").await;

        let steered = send_now(&mut session, 603, id, generation, 2).await;
        assert!(matches!(steered, ServerMsg::Reply(Reply::Ok)), "{steered:?}");

        // Codex was told which turn to join, and no new turn was started or stopped.
        let steers = lines(&session, "steers").await;
        assert_eq!(steers.len(), 1, "{steers:?}");
        let request: Value = serde_json::from_str(&steers[0]).unwrap();
        assert_eq!(request["method"], "turn/steer");
        assert_eq!(request["params"]["threadId"], "exact-thread");
        assert_eq!(request["params"]["expectedTurnId"], "turn-1");
        assert_eq!(request["params"]["input"][0]["text"], "use the other file");
        assert_eq!(turns_started(&session).await, 1);
        assert!(lines(&session, "interrupts").await.is_empty());

        // It left the queue; the other message still waits. Its number now answers as sent.
        assert_eq!(queued(&read(&mut session, 604, id).await), [(3, "and then rest".to_string())]);
        let receipt = read(&mut session, 605, id).await.receipt.expect("a receipt for the steered message");
        assert_eq!(receipt.operation, 2);
        assert_eq!(receipt.delivery, Delivery::Acknowledged { turn_id: "turn-1".into() });
        let retried = send(&mut session, 606, id, generation, 2, "use the other file").await;
        assert!(matches!(retried, ServerMsg::Reply(Reply::ConversationSent(ref r)) if r.operation == 2), "{retried:?}");
        // The message waiting behind it can still be retried, though a higher number is the receipt.
        let behind = send(&mut session, 607, id, generation, 3, "and then rest").await;
        assert!(matches!(behind, ServerMsg::Reply(Reply::ConversationQueued(_))), "{behind:?}");

        // Asking again is refused with a reason, and nothing is sent a second time.
        let again = send_now(&mut session, 608, id, generation, 2).await;
        assert!(matches!(again, ServerMsg::Reply(Reply::Ok)), "{again:?}");
        // A number nobody ever queued is still refused.
        let unknown = send_now(&mut session, 609, id, generation, 9).await;
        assert!(matches!(unknown, ServerMsg::Error(ref e) if e.contains("no longer waiting")), "{unknown:?}");
        assert_eq!(lines(&session, "steers").await.len(), 1);
        assert_eq!(turns_started(&session).await, 1);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn a_steer_codex_refuses_leaves_the_message_waiting_and_it_is_sent_once_the_turn_ends() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = started_with_steering(&mut session).await;
        send(&mut session, 610, id, generation, 1, "first").await;
        send(&mut session, 611, id, generation, 2, "second").await;
        // The turn ended between the click and the steer: Codex refuses it.
        tokio::fs::write(session.fixture.directory.path().join("steer-refuse"), "x").await.unwrap();

        let refused = send_now(&mut session, 612, id, generation, 2).await;
        assert!(
            matches!(refused, ServerMsg::Error(ref e) if e.contains("still waiting")),
            "{refused:?}"
        );
        assert_eq!(queued(&read(&mut session, 613, id).await), [(2, "second".to_string())]);
        // A refusal is not a fault of the conversation: it keeps working and shows no error.
        let state = &session.daemon.conversation_states[&id];
        assert_eq!(state.error, None);
        assert!(!state.blocked);
        assert_eq!(turns_started(&session).await, 1);

        // The turn ends; the ordinary delivery sends the message, once.
        let stopped = exchange(&mut session, 614, ClientMsg::Command(Command::ConversationCancel { id, generation }), None).await;
        assert!(matches!(stopped, ServerMsg::Reply(Reply::Ok)));
        pump_until(&mut session, |s| s.daemon.conversation_states[&id].cancelling.is_none()).await;
        pump_until(&mut session, |s| {
            s.daemon.deliver_queued();
            waiting(s, id) == 0
        })
        .await;
        pump_until(&mut session, |s| {
            s.daemon.conversation_requests.values().all(|p| p.conversation() != id)
        })
        .await;
        assert_eq!(turns_started(&session).await, 2, "sent as its own turn, once");
        let receipt = read(&mut session, 615, id).await.receipt.unwrap();
        assert_eq!(receipt.operation, 2);
        assert_eq!(lines(&session, "steers").await.len(), 1, "never steered again");
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn a_retry_while_the_steer_is_on_its_way_is_the_same_ask() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = started_with_steering(&mut session).await;
        send(&mut session, 620, id, generation, 1, "first").await;
        send(&mut session, 621, id, generation, 2, "second").await;

        // Two asks before the provider answers.
        session.daemon.conversation_send_now(621, id, generation, 2).unwrap();
        session.daemon.conversation_send_now(621, id, generation, 2).unwrap();
        // Taking it back now would lose a message Codex is about to have.
        let dequeue = session.daemon.conversation_dequeue(622, id, generation, 2);
        assert!(
            matches!(dequeue, Err(ref e) if e.to_string().contains("being sent now")),
            "{dequeue:?}"
        );
        pump_until(&mut session, |s| waiting(s, id) == 0).await;

        assert_eq!(lines(&session, "steers").await.len(), 1, "steered once");
        assert_eq!(turns_started(&session).await, 1);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn codex_without_steering_has_the_turn_interrupted_and_the_message_sent_next_ahead_of_the_rest() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        // The default fixture's Codex schema has no `turn/steer`.
        let (id, generation) = create(&mut session).await;
        send(&mut session, 630, id, generation, 1, "first").await;
        send(&mut session, 631, id, generation, 2, "second").await;
        send(&mut session, 632, id, generation, 3, "third").await;
        assert!(!session.daemon.conversation_runtimes.can_steer(id, generation));

        let now = send_now(&mut session, 633, id, generation, 3).await;
        assert!(matches!(now, ServerMsg::Reply(Reply::Ok)), "{now:?}");
        assert!(lines(&session, "steers").await.is_empty());
        // Asking again while the turn is being stopped changes nothing.
        let again = send_now(&mut session, 634, id, generation, 3).await;
        assert!(matches!(again, ServerMsg::Reply(Reply::Ok)), "{again:?}");

        // Nothing is sent into the running turn, and the chosen message goes first.
        pump_until(&mut session, |s| s.daemon.conversation_states[&id].cancelling.is_none()).await;
        assert_eq!(lines(&session, "interrupts").await.len(), 1);
        pump_until(&mut session, |s| {
            s.daemon.deliver_queued();
            waiting(s, id) == 1
        })
        .await;
        pump_until(&mut session, |s| {
            s.daemon.conversation_requests.values().all(|p| p.conversation() != id)
        })
        .await;
        assert_eq!(turns_started(&session).await, 2);
        let receipt = read(&mut session, 636, id).await.receipt.unwrap();
        assert_eq!(receipt.operation, 3, "the chosen message was the next turn");
        assert_eq!(queued(&read(&mut session, 637, id).await), [(2, "second".to_string())]);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn a_message_sent_now_after_the_turn_ended_is_just_sent() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = started_with_steering(&mut session).await;
        send(&mut session, 640, id, generation, 1, "first").await;
        send(&mut session, 641, id, generation, 2, "second").await;
        let stopped = exchange(&mut session, 642, ClientMsg::Command(Command::ConversationCancel { id, generation }), None).await;
        assert!(matches!(stopped, ServerMsg::Reply(Reply::Ok)));
        pump_until(&mut session, |s| s.daemon.conversation_states[&id].cancelling.is_none()).await;

        // The click lands after the turn ended and before the daemon's tick: nothing to steer.
        let now = send_now(&mut session, 643, id, generation, 2).await;
        assert!(matches!(now, ServerMsg::Reply(Reply::Ok)), "{now:?}");
        pump_until(&mut session, |s| {
            s.daemon.conversation_requests.values().all(|p| p.conversation() != id)
        })
        .await;
        assert_eq!(turns_started(&session).await, 2);
        assert!(lines(&session, "steers").await.is_empty());
        assert_eq!(waiting(&session, id), 0);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn nothing_is_sent_now_while_an_approval_or_question_waits() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = started_with_steering(&mut session).await;
        send(&mut session, 650, id, generation, 1, "first").await;
        send(&mut session, 651, id, generation, 2, "second").await;

        session
            .daemon
            .conversation_states
            .get_mut(&id)
            .unwrap()
            .unsupported_requests
            .insert(ProviderRequestId::Text("question".into()), "item/tool/requestUserInput".into());
        let refused = send_now(&mut session, 652, id, generation, 2).await;
        assert!(matches!(refused, ServerMsg::Error(ref e) if e.contains("pending approval or question")), "{refused:?}");
        session.daemon.conversation_states.get_mut(&id).unwrap().unsupported_requests.clear();

        session.daemon.conversation_states.get_mut(&id).unwrap().blocked = true;
        let blocked = send_now(&mut session, 653, id, generation, 2).await;
        assert!(matches!(blocked, ServerMsg::Error(ref e) if e.contains("pending approval or question")), "{blocked:?}");
        session.daemon.conversation_states.get_mut(&id).unwrap().blocked = false;

        // Neither path ran, and the message waits.
        assert!(lines(&session, "steers").await.is_empty());
        assert!(lines(&session, "interrupts").await.is_empty());
        assert_eq!(queued(&read(&mut session, 654, id).await), [(2, "second".to_string())]);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn the_interrupt_path_is_also_refused_while_a_request_waits() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        send(&mut session, 660, id, generation, 1, "first").await;
        send(&mut session, 661, id, generation, 2, "second").await;
        session
            .daemon
            .conversation_states
            .get_mut(&id)
            .unwrap()
            .unsupported_requests
            .insert(ProviderRequestId::Text("question".into()), "item/tool/requestUserInput".into());
        let refused = send_now(&mut session, 662, id, generation, 2).await;
        assert!(matches!(refused, ServerMsg::Error(ref e) if e.contains("pending approval or question")), "{refused:?}");
        assert!(lines(&session, "interrupts").await.is_empty());
        assert_eq!(waiting(&session, id), 1);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn a_stale_generation_an_unknown_message_or_a_pane_is_refused_with_a_reason() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = started_with_steering(&mut session).await;
        send(&mut session, 670, id, generation, 1, "first").await;
        send(&mut session, 671, id, generation, 2, "second").await;

        let stale = send_now(&mut session, 672, id, generation + 1, 2).await;
        assert!(matches!(stale, ServerMsg::Error(ref e) if e.contains("Stale")), "{stale:?}");
        let unknown = send_now(&mut session, 673, id, generation, 9).await;
        assert!(matches!(unknown, ServerMsg::Error(ref e) if e.contains("no longer waiting")), "{unknown:?}");
        // The running turn's own message was acknowledged: the same ask is answered, nothing sent.
        let running = send_now(&mut session, 674, id, generation, 1).await;
        assert!(matches!(running, ServerMsg::Reply(Reply::Ok)), "{running:?}");
        let from = authenticated_pane(&mut session);
        let pane = exchange(
            &mut session,
            675,
            ClientMsg::Command(Command::ConversationSendNow { id, generation, operation: 2 }),
            Some(from),
        )
        .await;
        assert!(matches!(pane, ServerMsg::Error(ref e) if e.contains("Only a human")), "{pane:?}");

        assert!(lines(&session, "steers").await.is_empty());
        assert!(lines(&session, "interrupts").await.is_empty());
        assert_eq!(queued(&read(&mut session, 676, id).await).len(), 1);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn a_claude_chat_has_its_turn_interrupted_and_the_message_sent_as_the_next_turn() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let root = session.fixture.directory.path().to_path_buf();
        let (executable, config_root) = tokio::task::spawn_blocking(move || {
            (fake_claude_executable(&root), std::fs::canonicalize(&root).unwrap())
        })
        .await
        .unwrap();
        std::fs::write(executable.with_extension("scenario"), "interrupt-any").unwrap();
        let messages = executable.with_extension("messages");
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
            680,
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
        let first = send(&mut session, 681, id, generation, 1, "one").await;
        assert!(matches!(first, ServerMsg::Reply(Reply::ConversationSent(_))), "{first:?}");
        send(&mut session, 682, id, generation, 2, "two").await;
        send(&mut session, 683, id, generation, 3, "three").await;
        assert!(!session.daemon.conversation_runtimes.can_steer(id, generation), "Claude cannot steer");

        let now = send_now(&mut session, 684, id, generation, 3).await;
        assert!(matches!(now, ServerMsg::Reply(Reply::Ok)), "{now:?}");

        pump_until(&mut session, |s| {
            s.daemon.deliver_queued();
            std::fs::read_to_string(&messages).is_ok_and(|log| log.lines().count() >= 2)
        })
        .await;
        let log = std::fs::read_to_string(&messages).unwrap();
        assert_eq!(log.lines().collect::<Vec<_>>(), ["one", "three"], "the chosen message went next");
        assert_eq!(queued(&read(&mut session, 685, id).await), [(2, "two".to_string())]);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn an_interrupt_the_provider_refuses_because_the_turn_just_ended_blocks_nothing() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        send(&mut session, 690, id, generation, 1, "first").await;
        send(&mut session, 691, id, generation, 2, "second").await;
        tokio::fs::write(session.fixture.directory.path().join("interrupt-refuse"), "x").await.unwrap();

        let now = send_now(&mut session, 692, id, generation, 2).await;
        assert!(matches!(now, ServerMsg::Reply(Reply::Ok)), "{now:?}");
        pump_until(&mut session, |s| {
            s.daemon.deliver_queued();
            waiting(s, id) == 0
                && s.daemon.conversation_requests.values().all(|p| p.conversation() != id)
        })
        .await;

        // The message went out as the next turn, and the chat never looked like it needed you.
        assert_eq!(turns_started(&session).await, 2);
        let state = &session.daemon.conversation_states[&id];
        assert_eq!(state.error, None);
        assert!(!state.blocked && state.history_ready && state.cancelling.is_none());
        assert_ne!(state.status, crate::protocol::Status::Blocked);
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}

#[tokio::test]
async fn a_message_sent_out_of_turn_is_still_answered_as_sent_after_others_are_delivered() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = started_with_steering(&mut session).await;
        send(&mut session, 700, id, generation, 1, "first").await;
        send(&mut session, 701, id, generation, 2, "second").await;
        send(&mut session, 702, id, generation, 3, "third").await;
        let steered = send_now(&mut session, 703, id, generation, 3).await;
        assert!(matches!(steered, ServerMsg::Reply(Reply::Ok)), "{steered:?}");

        // The running turn's own number is no longer the receipt, and still answers as sent.
        let first = send(&mut session, 704, id, generation, 1, "first").await;
        assert!(matches!(first, ServerMsg::Reply(Reply::ConversationSent(ref r)) if r.operation == 1), "{first:?}");

        // The turn ends and the older message is delivered, which replaces the receipt.
        let stopped = exchange(&mut session, 705, ClientMsg::Command(Command::ConversationCancel { id, generation }), None).await;
        assert!(matches!(stopped, ServerMsg::Reply(Reply::Ok)));
        pump_until(&mut session, |s| s.daemon.conversation_states[&id].cancelling.is_none()).await;
        pump_until(&mut session, |s| {
            s.daemon.deliver_queued();
            waiting(s, id) == 0
                && s.daemon.conversation_requests.values().all(|p| p.conversation() != id)
        })
        .await;
        assert_eq!(turns_started(&session).await, 2);
        assert_eq!(read(&mut session, 706, id).await.receipt.unwrap().operation, 2);

        // A retry of the steered message must not be sent again as a new turn.
        let retry = send(&mut session, 707, id, generation, 3, "third").await;
        assert!(matches!(retry, ServerMsg::Reply(Reply::ConversationSent(ref r)) if r.operation == 3), "{retry:?}");
        let changed = send(&mut session, 708, id, generation, 3, "other text").await;
        assert!(matches!(changed, ServerMsg::Error(ref e) if e.contains("reused")), "{changed:?}");
        assert_eq!(turns_started(&session).await, 2);
        assert_eq!(waiting(&session, id), 0);
        let again = send_now(&mut session, 709, id, generation, 3).await;
        assert!(matches!(again, ServerMsg::Reply(Reply::Ok)), "{again:?}");
    })
    .catch_unwind()
    .await;
    finish(session, outcome).await;
}
