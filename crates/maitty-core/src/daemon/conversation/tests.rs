use super::runtime::{Payload, TestCleanupState};
use super::transport::{Envelope, RequestId, decode};
use super::*;
use crate::conversation::{Conversation, ExecutionKind, RecoveryState};
use crate::daemon::{Config, Daemon, Ev, ServerMsg};
use futures_util::FutureExt;
use serde_json::{Value, json};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tokio::sync::mpsc;

fn fake_claude_executable(root: &Path) -> PathBuf {
    let executable = root.join(if cfg!(windows) {
        "fake-claude.exe"
    } else {
        "fake-claude"
    });
    let mut command = std::process::Command::new("rustc");
    command
        .arg("--edition=2024")
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/daemon/conversation/fixtures/fake_claude.rs"),
        )
        .arg("-o")
        .arg(&executable);
    let output = crate::proc::quiet(&mut command).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}

fn claude_launch(root: &Path, scenario: Option<&str>) -> (Launch, super::runtime::Owner) {
    std::fs::create_dir_all(root).unwrap();
    let executable = fake_claude_executable(root);
    let root = std::fs::canonicalize(root).unwrap();
    let settings = crate::conversation::LaunchSettings {
        provider: crate::conversation::Provider::Claude,
        executable: executable.clone(),
        config_root: root.clone(),
        cwd: root.clone(),
        model: None,
        model_provider: None,
        approval_policy: crate::conversation::ApprovalPolicy::OnRequest,
        sandbox: crate::conversation::Sandbox::ReadOnly,
    };
    let conversation = Conversation {
        id: crate::conversation::ConversationId(90),
        binding: crate::conversation::NativeBinding {
            provider: crate::conversation::Provider::Claude,
            session_id: "fixture-session".into(),
            account: None,
            profile: None,
            config_root: root.clone(),
            executable,
            cwd: root,
            worktree: None,
        },
        execution: ExecutionKind::Structured,
        generation: 1,
        state: RecoveryState::Stopped,
        settings: Some(settings),
        awaiting_first_user_message: false,
        title: None,
        archived: false,
    };
    let environment = scenario
        .map(|scenario| {
            vec![(
                std::ffi::OsString::from("FAKE_CLAUDE_SCENARIO"),
                std::ffi::OsString::from(scenario),
            )]
        })
        .unwrap_or_default();
    let launch = Launch {
        identity: super::transport::LaunchIdentity::Existing(Box::new(conversation.clone())),
        environment,
        failure_point: None,
    };
    let owner = super::runtime::test_owner(conversation.id, conversation.generation);
    (launch, owner)
}

async fn receive_claude_payload(
    received: &mut mpsc::Receiver<Ev>,
    owner: super::runtime::Owner,
) -> Payload {
    let event = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .expect("Claude adapter event should arrive")
        .expect("Claude adapter should remain connected");
    let Ev::Conversation(event) = event else {
        panic!("expected a Claude conversation event");
    };
    assert_eq!(event.owner, owner);
    event.payload
}

async fn start_claude_adapter(
    launch: Launch,
    owner: super::runtime::Owner,
) -> (
    tokio::task::JoinHandle<()>,
    mpsc::Sender<()>,
    mpsc::Sender<super::runtime::Command>,
    mpsc::Receiver<Ev>,
) {
    let (events, mut received) = mpsc::channel(8);
    let (stop, stopped) = mpsc::channel(1);
    let (requests, incoming) = mpsc::channel(2);
    let task = tokio::spawn(super::claude::run(launch, owner, events, stopped, incoming));
    assert!(matches!(
        receive_claude_payload(&mut received, owner).await,
        Payload::Ready
    ));
    (task, stop, requests, received)
}

async fn start_claude_thread(
    requests: &mpsc::Sender<super::runtime::Command>,
    received: &mut mpsc::Receiver<Ev>,
    owner: super::runtime::Owner,
) {
    requests
        .send(super::runtime::Command {
            token: 1,
            request: super::runtime::Rpc::Start { params: json!({}) },
        })
        .await
        .unwrap();
    assert!(matches!(
        receive_claude_payload(received, owner).await,
        Payload::Response {
            token: 1,
            result: Ok(ref value)
        } if value == &json!({"thread":{"id":"fixture-session"}})
    ));
}

async fn stop_claude_adapter(
    task: tokio::task::JoinHandle<()>,
    stop: mpsc::Sender<()>,
    received: &mut mpsc::Receiver<Ev>,
    owner: super::runtime::Owner,
) {
    stop.send(()).await.unwrap();
    assert!(matches!(
        receive_claude_payload(received, owner).await,
        Payload::Terminated {
            failure: None,
            reaped: true
        }
    ));
    task.await.unwrap();
}

#[tokio::test]
async fn claude_adapter_initializes_streams_a_turn_and_reads_empty_turns() {
    let directory = crate::daemon::persist::scratch_dir();
    let (launch, owner) = claude_launch(directory.path(), None);
    let (events, mut received) = mpsc::channel(8);
    let (stop, stopped) = mpsc::channel(1);
    let (requests, incoming) = mpsc::channel(2);
    let task = tokio::spawn(super::claude::run(launch, owner, events, stopped, incoming));

    let ready = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .expect("Claude adapter should become ready")
        .expect("Claude adapter should emit Ready");
    assert!(
        matches!(ready, Ev::Conversation(event) if event.owner == owner && matches!(event.payload, Payload::Ready))
    );

    requests
        .send(super::runtime::Command {
            token: 1,
            request: super::runtime::Rpc::Start { params: json!({}) },
        })
        .await
        .unwrap();
    let started = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .expect("Claude Start should respond")
        .expect("Claude adapter should emit the Start response");
    assert!(
        matches!(started, Ev::Conversation(event) if event.owner == owner && matches!(event.payload, Payload::Response { token: 1, result: Ok(ref value) } if value == &json!({"thread":{"id":"fixture-session"}})))
    );

    requests
        .send(super::runtime::Command {
            token: 2,
            request: super::runtime::Rpc::Turns { params: json!({}) },
        })
        .await
        .unwrap();
    let turns = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .expect("Claude empty Turns should respond")
        .expect("Claude adapter should emit the Turns response");
    assert!(
        matches!(turns, Ev::Conversation(event) if event.owner == owner && matches!(event.payload, Payload::Response { token: 2, result: Ok(ref value) } if value == &json!({"data":[],"nextCursor":null})))
    );

    requests
        .send(super::runtime::Command {
            token: 3,
            request: super::runtime::Rpc::Send {
                params: json!({
                    "threadId":"fixture-session",
                    "input":[{"type":"text","text":"hello fake Claude"}]
                }),
            },
        })
        .await
        .unwrap();
    let mut notifications = Vec::new();
    let mut acknowledged = false;
    for _ in 0..8 {
        let event = tokio::time::timeout(Duration::from_secs(5), received.recv())
            .await
            .expect("Claude stream event should arrive")
            .expect("Claude adapter should remain connected during the turn");
        let Ev::Conversation(event) = event else {
            panic!("expected a Claude conversation event");
        };
        assert_eq!(event.owner, owner);
        match event.payload {
            Payload::Notification { method, params } => {
                if method == "item/agentMessage/delta" {
                    assert_eq!(params["delta"], "fake reply");
                }
                notifications.push(method);
            }
            Payload::Response {
                token: 3,
                result: Ok(result),
            } => {
                assert_eq!(result["turn"]["status"], "inProgress");
                acknowledged = true;
            }
            _ => panic!("unexpected Claude stream event payload"),
        }
    }
    assert!(
        acknowledged,
        "the submitted user message should be acknowledged"
    );
    assert_eq!(
        notifications,
        [
            "turn/started",
            "item/started",
            "item/completed",
            "item/started",
            "item/agentMessage/delta",
            "item/completed",
            "turn/completed",
        ]
    );

    stop.send(()).await.unwrap();
    let terminated = tokio::time::timeout(Duration::from_secs(5), received.recv())
        .await
        .expect("Claude adapter should stop")
        .expect("Claude adapter should emit Terminated");
    assert!(
        matches!(terminated, Ev::Conversation(event) if event.owner == owner && matches!(event.payload, Payload::Terminated { failure: None, reaped: true }))
    );
    task.await.unwrap();
    drop(directory);
}

async fn exercise_claude_approval(scenario: &str, decision: &str) {
    let directory = crate::daemon::persist::scratch_dir();
    let (launch, owner) = claude_launch(directory.path(), Some(scenario));
    let (task, stop, requests, mut received) = start_claude_adapter(launch, owner).await;
    start_claude_thread(&requests, &mut received, owner).await;
    requests
        .send(super::runtime::Command {
            token: 3,
            request: super::runtime::Rpc::Send {
                params: json!({
                    "threadId":"fixture-session",
                    "input":[{"type":"text","text":"hello fake Claude"}]
                }),
            },
        })
        .await
        .unwrap();

    let mut acknowledged = false;
    let mut approval = None;
    let mut turn_id = None;
    for _ in 0..6 {
        match receive_claude_payload(&mut received, owner).await {
            Payload::Notification { method, params } if method == "turn/started" => {
                turn_id = params["turn"]["id"].as_str().map(str::to_owned);
            }
            Payload::Notification { method, .. } => {
                assert!(method == "item/started" || method == "item/completed");
            }
            Payload::Response {
                token: 3,
                result: Ok(value),
            } => {
                assert_eq!(value["turn"]["status"], "inProgress");
                acknowledged = true;
            }
            Payload::Request { id, method, params } => {
                approval = Some((id, method, params));
                break;
            }
            _ => panic!("unexpected Claude approval payload"),
        }
    }
    assert!(
        acknowledged,
        "the user prompt should be acknowledged before approval"
    );
    let turn_id = turn_id.expect("the turn start should include its exact ID");
    let (id, method, params) = approval.expect("Claude should request tool approval");
    assert_eq!(id, RequestId::Text("tool-request".into()));
    assert_eq!(method, "item/commandExecution/requestApproval");
    assert_eq!(params["threadId"], "fixture-session");
    assert_eq!(params["turnId"], turn_id);
    assert_eq!(params["itemId"], "tool-request");
    assert_eq!(params["command"], "echo hello");
    assert_eq!(params["availableDecisions"], json!(["accept", "decline"]));

    requests
        .send(super::runtime::Command {
            token: 4,
            request: super::runtime::Rpc::Answer {
                request_id: id,
                response: json!({"decision":decision}),
            },
        })
        .await
        .unwrap();
    assert!(matches!(
        receive_claude_payload(&mut received, owner).await,
        Payload::Notification { method, params }
            if method == "turn/completed" && params["turn"]["status"] == "completed"
    ));
    stop_claude_adapter(task, stop, &mut received, owner).await;
    drop(directory);
}

#[tokio::test]
async fn claude_adapter_allows_a_host_approved_tool() {
    exercise_claude_approval("approval", "accept").await;
}

#[test]
fn only_the_most_recent_answered_approvals_are_kept() {
    use crate::conversation::{
        ApprovalDecision, ApprovalRequest, ApprovalRequestKind, ProviderRequestId,
    };
    let request = |n: i64| ApprovalRequest {
        request_id: ProviderRequestId::Integer(n),
        kind: ApprovalRequestKind::Command,
        item_id: None,
        turn_id: None,
        command: Some("ls".into()),
        reason: None,
        cwd: None,
        changes: Vec::new(),
        questions: Vec::new(),
        permissions: None,
        choices: vec![ApprovalDecision::Accept, ApprovalDecision::Decline],
        can_remember: false,
    };
    let mut state = super::State::default();
    for n in 0..70 {
        state.record_answer(request(n), ApprovalDecision::Decline);
    }
    assert_eq!(state.answered_approvals.len(), 64);
    assert_eq!(
        state.answered_approvals[0].request.request_id,
        ProviderRequestId::Integer(6)
    );
    assert_eq!(
        state.answered_approvals[63].request.request_id,
        ProviderRequestId::Integer(69)
    );
}

/// A Claude `Edit` call reaches the history with its input and times, and its approval names
/// the file, carries the diff and points at the tool's own step.
#[tokio::test]
async fn claude_edit_call_and_approval_carry_input_diff_and_a_link_to_the_tool_step() {
    use super::history::{Attachments, Content, ItemState};
    use crate::conversation::ConversationId;

    let directory = crate::daemon::persist::scratch_dir();
    let (launch, owner) = claude_launch(directory.path(), Some("tool-edit"));
    let (task, stop, requests, mut received) = start_claude_adapter(launch, owner).await;
    start_claude_thread(&requests, &mut received, owner).await;
    requests
        .send(super::runtime::Command {
            token: 3,
            request: super::runtime::Rpc::Send {
                params: json!({
                    "threadId":"fixture-session",
                    "input":[{"type":"text","text":"hello fake Claude"}]
                }),
            },
        })
        .await
        .unwrap();

    let mut histories = Attachments::default();
    histories.attach(1, ConversationId(1));
    let mut approval = None;
    loop {
        match receive_claude_payload(&mut received, owner).await {
            Payload::Notification { method, params } => {
                histories
                    .notification(ConversationId(1), "fixture-session", &method, &params)
                    .unwrap();
                if method == "turn/completed" {
                    break;
                }
            }
            Payload::Request { id, method, params } => {
                approval = Some((method, params));
                requests
                    .send(super::runtime::Command {
                        token: 4,
                        request: super::runtime::Rpc::Answer {
                            request_id: id,
                            response: json!({"decision":"accept"}),
                        },
                    })
                    .await
                    .unwrap();
            }
            Payload::Response { .. } => {}
            _ => panic!("unexpected Claude edit payload"),
        }
    }

    let (method, params) = approval.expect("the edit asks for approval");
    assert_eq!(method, "item/fileChange/requestApproval");
    assert_eq!(params["itemId"], "toolu_edit");
    assert_eq!(params["command"], "Edit: src/a.rs");
    assert_eq!(
        params["changes"],
        json!([{"path":"src/a.rs","kind":"update","diff":"-old\n+new"}])
    );

    let history = histories.get(1).unwrap();
    let tool = history
        .entries
        .iter()
        .find(|entry| entry.source.item == "toolu_edit")
        .expect("the edit step is in the history under the id the approval names");
    assert_eq!(tool.state, ItemState::Completed);
    let Content::Tool { label, output, input, .. } = &tool.content else {
        panic!("an edit is a tool row");
    };
    assert_eq!(label, "Edit ");
    assert_eq!(output, "edited");
    assert_eq!(
        input,
        &Some(json!({"file_path":"src/a.rs","old_string":"old","new_string":"new"}))
    );
    let (started, completed) = (tool.started_at_ms.unwrap(), tool.completed_at_ms.unwrap());
    assert!(completed >= started);
    let turn = history
        .entries
        .iter()
        .find(|entry| entry.content == Content::TurnStatus)
        .expect("the turn has a row");
    assert!(turn.started_at_ms.is_some() && turn.completed_at_ms.is_some());

    stop_claude_adapter(task, stop, &mut received, owner).await;
    drop(directory);
}

#[tokio::test]
async fn claude_write_of_a_secret_file_and_an_echoed_token_are_masked_in_history() {
    use super::history::{Attachments, Content};
    use crate::conversation::ConversationId;

    let directory = crate::daemon::persist::scratch_dir();
    let (launch, owner) = claude_launch(directory.path(), Some("tool-secret"));
    let (task, stop, requests, mut received) = start_claude_adapter(launch, owner).await;
    start_claude_thread(&requests, &mut received, owner).await;
    requests
        .send(super::runtime::Command {
            token: 3,
            request: super::runtime::Rpc::Send {
                params: json!({
                    "threadId":"fixture-session",
                    "input":[{"type":"text","text":"hello fake Claude"}]
                }),
            },
        })
        .await
        .unwrap();

    let mut histories = Attachments::default();
    histories.attach(1, ConversationId(1));
    let mut approval = None;
    loop {
        match receive_claude_payload(&mut received, owner).await {
            Payload::Notification { method, params } => {
                histories
                    .notification(ConversationId(1), "fixture-session", &method, &params)
                    .unwrap();
                if method == "turn/completed" {
                    break;
                }
            }
            Payload::Request { id, method, params } => {
                approval = Some((method, params));
                requests
                    .send(super::runtime::Command {
                        token: 4,
                        request: super::runtime::Rpc::Answer {
                            request_id: id,
                            response: json!({"decision":"accept"}),
                        },
                    })
                    .await
                    .unwrap();
            }
            Payload::Response { .. } => {}
            _ => panic!("unexpected Claude payload"),
        }
    }

    // The fixture asserts the agent was answered with its own, unmasked input. The approval the
    // adapter raises is masked where the daemon stores it (see the daemon-level test).
    let (method, _) = approval.expect("the write asks for approval");
    assert_eq!(method, "item/fileChange/requestApproval");

    let history = histories.get(1).unwrap();
    let find = |item: &str| {
        history
            .entries
            .iter()
            .find(|entry| entry.source.item == item)
            .unwrap_or_else(|| panic!("{item} is in the history"))
    };
    let Content::Tool { input, .. } = &find("toolu_write").content else {
        panic!("a write is a tool row");
    };
    let input = input.as_ref().expect("the write keeps its input");
    assert_eq!(input["file_path"], "app/.env");
    assert_eq!(input["content"], super::redact::OMITTED);
    let Content::Tool { output, .. } = &find("toolu_echo").content else {
        panic!("a command is a tool row");
    };
    assert_eq!(output, "token is [redacted]");

    stop_claude_adapter(task, stop, &mut received, owner).await;
    drop(directory);
}

#[tokio::test]
async fn claude_adapter_denies_a_host_declined_tool() {
    exercise_claude_approval("approval-decline", "decline").await;
}

#[tokio::test]
async fn claude_adapter_acknowledges_interrupts_for_the_active_turn() {
    let directory = crate::daemon::persist::scratch_dir();
    let (launch, owner) = claude_launch(directory.path(), Some("interrupt"));
    let (task, stop, requests, mut received) = start_claude_adapter(launch, owner).await;
    start_claude_thread(&requests, &mut received, owner).await;
    requests
        .send(super::runtime::Command {
            token: 3,
            request: super::runtime::Rpc::Send {
                params: json!({
                    "threadId":"fixture-session",
                    "input":[{"type":"text","text":"hello fake Claude"}]
                }),
            },
        })
        .await
        .unwrap();

    let mut turn_id = None;
    let mut acknowledged = false;
    for _ in 0..4 {
        match receive_claude_payload(&mut received, owner).await {
            Payload::Notification { method, params } if method == "turn/started" => {
                turn_id = params["turn"]["id"].as_str().map(str::to_owned);
            }
            Payload::Notification { method, .. } => {
                assert!(method == "item/started" || method == "item/completed");
            }
            Payload::Response {
                token: 3,
                result: Ok(value),
            } => {
                assert_eq!(value["turn"]["status"], "inProgress");
                acknowledged = true;
            }
            _ => panic!("unexpected Claude interrupt setup payload"),
        }
    }
    assert!(acknowledged, "the submitted prompt should be acknowledged");
    let turn_id = turn_id.expect("the turn start should include its exact ID");

    requests
        .send(super::runtime::Command {
            token: 4,
            request: super::runtime::Rpc::Interrupt {
                params: json!({"threadId":"fixture-session","turnId":turn_id}),
            },
        })
        .await
        .unwrap();
    assert!(matches!(
        receive_claude_payload(&mut received, owner).await,
        Payload::Response {
            token: 4,
            result: Ok(ref value)
        } if value == &json!({})
    ));
    assert!(matches!(
        receive_claude_payload(&mut received, owner).await,
        Payload::Notification { method, params }
            if method == "turn/completed" && params["turn"]["status"] == "completed"
    ));
    stop_claude_adapter(task, stop, &mut received, owner).await;
    drop(directory);
}

fn fake_executable(root: &Path) -> PathBuf {
    let executable = root.join(if cfg!(windows) {
        "fake-codex.exe"
    } else {
        "fake-codex"
    });
    let mut command = std::process::Command::new("rustc");
    command
        .arg("--edition=2024")
        .arg(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("src/daemon/conversation/fixtures/fake_provider.rs"),
        )
        .arg("-o")
        .arg(&executable);
    let output = crate::proc::quiet(&mut command).output().unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    executable
}
struct Fixture {
    directory: crate::daemon::persist::ScratchDir,
    record: Conversation,
}
impl Fixture {
    async fn new() -> Self {
        let directory = crate::daemon::persist::scratch_dir();
        let root = directory.path().to_owned();
        let record = tokio::task::spawn_blocking(move || {
            std::fs::create_dir_all(&root).unwrap();
            let executable = fake_executable(&root);
            std::fs::create_dir(root.join("different")).unwrap();
            let inventory: Value = serde_json::from_str(include_str!(
                "../../../../../crates/adapters/maitty-provider-codex/src/fixtures/codex-0.164.0.json"
            ))
            .unwrap();
            for (name, section) in [
                ("ClientRequest.json", "clientRequests"),
                ("ServerRequest.json", "serverRequests"),
            ] {
                std::fs::write(root.join(name), inventory[section].to_string()).unwrap();
            }
            let mut record = crate::test_support::conversation();
            record.binding.profile = None;
            record.binding.executable = executable;
            record.binding.config_root = std::fs::canonicalize(&root).unwrap();
            record.binding.cwd = root;
            record.binding.worktree = None;
            record
        })
        .await
        .unwrap();
        Self { directory, record }
    }
    fn launch(&self, mode: &str) -> Launch {
        let environment = [
            (
                "TEST_SCHEMA_DIR",
                self.directory.path().as_os_str().to_owned(),
            ),
            (
                "TEST_PID_FILE",
                self.directory.path().join("pids").into_os_string(),
            ),
            ("TEST_MODE", mode.into()),
            (
                "TEST_ACCOUNT_VALUE",
                "selected-account-effective-env".into(),
            ),
        ]
        .into_iter()
        .map(|(key, value)| (key.into(), value))
        .collect();
        Launch {
            identity: super::transport::LaunchIdentity::Existing(Box::new(self.record.clone())),
            environment,
            failure_point: None,
        }
    }
    async fn pids(&self) -> Vec<u32> {
        let path = self.directory.path().join("pids");
        tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(path)
                .unwrap()
                .split_whitespace()
                .map(|s| s.parse().unwrap())
                .collect()
        })
        .await
        .unwrap()
    }

    // Child-owning test bodies unwind inside with_fixture so disposal finishes
    // before this directory guard can drop on Windows.
    async fn cleanup(self) {
        let directory = self.directory.path().to_owned();
        let executable = self.record.binding.executable.clone();
        tokio::task::spawn_blocking(move || {
            // Windows refuses to delete a folder while a just-killed process or a scanner still
            // holds a handle in it, and the guard's own removal tries once and ignores failure.
            // Retry here until a deadline, then let the assertions judge the result. A CI runner
            // once held the freshly compiled provider past ten seconds, so the deadline is long;
            // a passing test leaves the loop as soon as the removal succeeds.
            let until = std::time::Instant::now() + Duration::from_secs(60);
            while directory.exists() && std::fs::remove_dir_all(&directory).is_err() {
                if std::time::Instant::now() >= until {
                    break;
                }
                std::thread::sleep(Duration::from_millis(50));
            }
            drop(self);
            assert!(!directory.exists(), "fixture directory survived cleanup");
            assert!(!executable.exists(), "compiled provider survived cleanup");
        })
        .await
        .unwrap();
    }
}

fn daemon(record: Conversation) -> (Daemon, mpsc::Receiver<Ev>) {
    let (tx, rx) = mpsc::channel(32);
    let mut daemon = Daemon::new(Config::default(), tx, || {});
    daemon.conversations.push(record);
    (daemon, rx)
}

struct TestSession {
    fixture: Fixture,
    daemon: Daemon,
    events: mpsc::Receiver<Ev>,
}

type FixtureCheck<'a> = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + 'a>>;

async fn with_fixture(check: impl for<'a> FnOnce(&'a mut TestSession) -> FixtureCheck<'a>) {
    let fixture = Fixture::new().await;
    let (daemon, events) = daemon(fixture.record.clone());
    let mut session = TestSession {
        fixture,
        daemon,
        events,
    };
    let outcome = AssertUnwindSafe(check(&mut session)).catch_unwind().await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

impl TestSession {
    async fn settle_background(&mut self) {
        tokio::time::timeout(Duration::from_secs(20), async {
            while !self.daemon.conversation_cache_writes.is_empty()
                || self
                    .daemon
                    .conversation_states
                    .values()
                    .any(|state| state.checks.running())
            {
                self.daemon.handle(
                    self.events
                        .recv()
                        .await
                        .expect("owned background task completed"),
                );
            }
        })
        .await
        .expect("owned checks and cache writers finish before fixture deletion");
    }
    async fn cleanup(mut self) {
        let native_pids: Vec<_> = self
            .daemon
            .terms
            .values()
            .filter_map(|term| term.pid)
            .collect();
        let terms: Vec<_> = self.daemon.terms.keys().copied().collect();
        for term in terms {
            self.daemon.close_term(term);
        }
        if !native_pids.is_empty() {
            stopped(native_pids).await;
        }
        let id = self.fixture.record.id;
        tokio::time::timeout(Duration::from_secs(40), async {
            loop {
                match self.daemon.conversation_runtimes.test_cleanup_state(id) {
                    None | Some(TestCleanupState::Finished) => break,
                    Some(TestCleanupState::Active) => self.daemon.dispose_conversation(id).unwrap(),
                    Some(TestCleanupState::Requested) => {}
                }
                while let Ok(event) = self.events.try_recv() {
                    self.daemon.handle(event);
                }
                // An assertion may consume a terminal event before applying it;
                // allow the owned actor to finish instead of waiting for a lost event.
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("owned provider disposal completes before fixture cleanup");
        let pid_file = self.fixture.directory.path().join("pids");
        let started = tokio::task::spawn_blocking(move || pid_file.exists())
            .await
            .unwrap();
        if started {
            stopped(self.fixture.pids().await).await;
        }
        self.settle_background().await;
        drop(self.daemon);
        self.fixture.cleanup().await;
    }
}

async fn next(rx: &mut mpsc::Receiver<Ev>) -> Event {
    tokio::time::timeout(Duration::from_secs(40), async {
        loop {
            if let Ev::Conversation(event) = rx.recv().await.unwrap() {
                return event;
            }
        }
    })
    .await
    .expect("provider event within deadline")
}

async fn stopped(pids: Vec<u32>) {
    tokio::task::spawn_blocking(move || {
        // A kill is asked for on another thread and, on Windows, done by `taskkill`: on a busy
        // machine it was measured landing twelve seconds later. A process that stopped ends the
        // wait at once, so the long deadline costs a passing test nothing.
        let until = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            let mut system = sysinfo::System::new();
            system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
            let alive = pids.iter().any(|pid| {
                system
                    .process(sysinfo::Pid::from_u32(*pid))
                    .is_some_and(|p| p.status() != sysinfo::ProcessStatus::Zombie)
            });
            if !alive {
                return;
            }
            assert!(
                std::time::Instant::now() < until,
                "provider process tree survived disposal: {pids:?}"
            );
            std::thread::yield_now();
        }
    })
    .await
    .unwrap();
}

async fn conversation_fixture(mode: &str) -> TestSession {
    let fixture = Fixture::new().await;
    let root = fixture.directory.path().to_path_buf();
    let mode = mode.to_owned();
    let cwd = tokio::task::spawn_blocking(move || {
        std::fs::write(root.join("mode"), mode).unwrap();
        let cwd = root.join("repo");
        std::fs::create_dir(&cwd).unwrap();
        cwd
    })
    .await
    .unwrap();
    let (tx, events) = mpsc::channel(128);
    let mut daemon = Daemon::new(Config::default(), tx, || {});
    daemon.session_path = fixture.directory.path().join("data/session.json");
    daemon.check_dir = fixture.directory.path().join("data/checks");
    daemon.approvals_file = fixture.directory.path().join("data/approvals.json");
    let mut session = TestSession {
        fixture,
        daemon,
        events,
    };
    session.fixture.record.binding.cwd = cwd;
    session
}

fn settings(session: &TestSession) -> crate::conversation::LaunchSettings {
    let b = &session.fixture.record.binding;
    crate::conversation::LaunchSettings {
        provider: crate::conversation::Provider::Codex,
        executable: b.executable.clone(),
        config_root: b.config_root.clone(),
        cwd: b.cwd.clone(),
        model: None,
        model_provider: None,
        approval_policy: crate::conversation::ApprovalPolicy::OnRequest,
        sandbox: crate::conversation::Sandbox::ReadOnly,
    }
}

async fn exchange(
    session: &mut TestSession,
    client: u64,
    message: crate::protocol::ClientMsg,
    from: Option<(u32, String)>,
) -> crate::protocol::ServerMsg {
    exchange_claim(session, client, message, from, None, None).await
}

async fn exchange_claim(
    session: &mut TestSession,
    client: u64,
    message: crate::protocol::ClientMsg,
    from: Option<(u32, String)>,
    structured: Option<crate::protocol::StructuredIdentity>,
    mut deferred: Option<&mut Vec<Ev>>,
) -> crate::protocol::ServerMsg {
    use interprocess::local_socket::{
        ListenerOptions,
        tokio::{Stream, prelude::*},
    };
    #[cfg(windows)]
    let name = {
        use interprocess::local_socket::{GenericNamespaced, ToNsName};
        format!(
            "synthetic-conversation-{}-{client}-{}",
            std::process::id(),
            session
                .fixture
                .directory
                .path()
                .file_name()
                .unwrap()
                .to_string_lossy()
        )
        .to_ns_name::<GenericNamespaced>()
        .unwrap()
        .into_owned()
    };
    #[cfg(unix)]
    let name = {
        use interprocess::local_socket::{GenericFilePath, ToFsName};
        session
            .fixture
            .directory
            .path()
            .join(format!("ipc-{client}"))
            .to_fs_name::<GenericFilePath>()
            .unwrap()
            .into_owned()
    };
    let listener = ListenerOptions::new()
        .name(name.clone())
        .create_tokio()
        .unwrap();
    let events = session.daemon.tx.clone();
    let server = tokio::spawn(async move {
        let stream = listener.accept().await.unwrap();
        crate::daemon::serve(client, stream, events).await;
    });
    let stream = Stream::connect(name).await.unwrap();
    let (mut reader, mut writer) = crate::ipc::framed(stream);
    crate::ipc::send(
        &mut writer,
        &crate::protocol::ClientMsg::Hello {
            version: crate::protocol::PROTOCOL_VERSION,
            attach: false,
            from,
            structured,
            surface: crate::protocol::Surface::Terminal,
            replays: true,
        },
    )
    .await
    .unwrap();
    assert!(matches!(
        crate::ipc::recv_server(&mut reader).await.unwrap(),
        Some(ServerMsg::Welcome { .. })
    ));
    crate::ipc::send(&mut writer, &message).await.unwrap();
    let reply=tokio::time::timeout(Duration::from_secs(40),async {
        loop {
            tokio::select! {
                event=session.events.recv()=>{
                    let event=event.unwrap();
                    if matches!(event,Ev::SessionSaved {..}) && let Some(deferred)=deferred.as_mut() {deferred.push(event);}else{session.daemon.handle(event);}
                },
                message=crate::ipc::recv_server(&mut reader)=>match message.unwrap().unwrap() {
                    reply@ServerMsg::Reply(_)|reply@ServerMsg::Error(_)=>break reply,
                    _=>(),
                }
            }
        }
    }).await.expect("real local IPC operation completed");
    drop(writer);
    drop(reader);
    server.abort();
    session.daemon.handle(Ev::Disconnected(client));
    reply
}

async fn create(session: &mut TestSession) -> (crate::conversation::ConversationId, u64) {
    let target = crate::conversation::StartTarget::Create(settings(session));
    match exchange(
        session,
        100,
        crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart { target, branch: None }),
        None,
    )
    .await
    {
        ServerMsg::Reply(crate::protocol::Reply::ConversationStarted { id, generation }) => {
            (id, generation)
        }
        other => panic!("{other:?}"),
    }
}

#[tokio::test]
async fn new_empty_thread_starts_without_requesting_unmaterialized_history() {
    let mut session = conversation_fixture("conversation-empty-thread").await;
    session.daemon.cfg.restore.output = false;
    session
        .daemon
        .initialize_conversation_cache()
        .await
        .unwrap();
    let outcome = AssertUnwindSafe(async {
        let (id, _) = create(&mut session).await;
        let state = &session.daemon.conversation_states[&id];
        assert!(state.history_ready);
        assert_eq!(state.status, crate::protocol::Status::Idle);
        assert!(!session.daemon.conversation_histories.contains_key(&id));

        let reply = exchange(
            &mut session,
            101,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {
                id,
                cursor: None,
                limit: 128,
            }),
            None,
        )
        .await;
        assert!(matches!(
            reply,
            ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref read))
                if read.history.entries.is_empty()
                    && read.history.completeness == crate::conversation::history::Completeness::Complete
        ));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn uncertain_first_submission_is_not_treated_as_empty_history() {
    let mut session = conversation_fixture("conversation-empty-thread").await;
    session.daemon.cfg.restore.output = false;
    session
        .daemon
        .initialize_conversation_cache()
        .await
        .unwrap();
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        session.daemon.conversation_operations.push(crate::conversation::Operation {
            receipt: crate::conversation::Receipt {
                conversation: id,
                generation,
                operation: 1,
                delivery: crate::conversation::Delivery::SubmissionUncertain,
            },
            digest: "synthetic-digest".into(),
        });

        let reply = exchange(
            &mut session,
            102,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {
                id,
                cursor: None,
                limit: 128,
            }),
            None,
        )
        .await;
        assert!(matches!(reply, ServerMsg::Error(ref error) if error.contains("Codex rejected thread/turns/list")));
        assert!(!session.daemon.conversation_states[&id].history_ready);
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn conversation_diff_query_reads_only_the_current_conversation_checkout() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let cwd = session.fixture.record.binding.cwd.clone();
        tokio::task::spawn_blocking(move || {
            let run = |args: &[&str]| {
                let output = std::process::Command::new("git")
                    .arg("-C")
                    .arg(&cwd)
                    .args(args)
                    .output()
                    .unwrap();
                assert!(
                    output.status.success(),
                    "{}",
                    String::from_utf8_lossy(&output.stderr)
                );
            };
            run(&["init", "--quiet"]);
            run(&["config", "user.name", "Test User"]);
            run(&["config", "user.email", "test@example.invalid"]);
            std::fs::write(cwd.join("tracked.rs"), "before\n").unwrap();
            run(&["add", "tracked.rs"]);
            run(&["commit", "--quiet", "-m", "baseline"]);
            std::fs::write(cwd.join("tracked.rs"), "after\n").unwrap();
            std::fs::write(cwd.join("new.rs"), "new\n").unwrap();
        })
        .await
        .unwrap();

        let (id, generation) = create(&mut session).await;
        let result = exchange(
            &mut session,
            322,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationDiff {
                id,
                generation,
            }),
            None,
        )
        .await;
        let ServerMsg::Reply(crate::protocol::Reply::ConversationDiff(diff)) = result else {
            panic!("expected conversation diff, got {result:?}");
        };
        assert_eq!(diff.total_files, 2, "{diff:?}");
        assert!(diff.files.iter().any(|file| file.path == "tracked.rs"));
        assert!(diff.files.iter().any(|file| file.path == "new.rs"));

        let stale = exchange(
            &mut session,
            323,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationDiff {
                id,
                generation: generation + 1,
            }),
            None,
        )
        .await;
        assert!(matches!(stale, ServerMsg::Error(ref message) if message.contains("stale")));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn native_terminal_access_is_bound_to_the_current_conversation() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = crate::conversation::ConversationId(47);
        let generation = 3;
        let mut conversation = session.fixture.record.clone();
        conversation.id = id;
        conversation.generation = generation;
        conversation.execution = crate::conversation::ExecutionKind::Native;
        conversation.state = crate::conversation::RecoveryState::Running;
        session.daemon.conversations.push(conversation);
        let (term, token) = authenticated_pane(&mut session);
        session.daemon.terms.get_mut(&term).unwrap().conversation_id = Some(id);

        let screen = exchange(
            &mut session,
            324,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationTerminalScreen {
                id,
                generation,
            }),
            None,
        )
        .await;
        assert!(matches!(screen, ServerMsg::Reply(crate::protocol::Reply::Text(ref ansi)) if ansi.starts_with("\x1b[0m\x1b[2J\x1b[H")));

        let replay = exchange(
            &mut session,
            332,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationTerminalReplay {
                id,
                generation,
            }),
            None,
        )
        .await;
        assert!(matches!(replay, ServerMsg::Reply(crate::protocol::Reply::TerminalReplay(ref bytes)) if !bytes.is_empty()));

        let input = exchange(
            &mut session,
            325,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationTerminalInput {
                    id,
                    generation,
                    data: b"x".to_vec(),
                },
            ),
            None,
        )
        .await;
        assert!(matches!(
            input,
            ServerMsg::Reply(crate::protocol::Reply::Ok)
        ));

        let resize = exchange(
            &mut session,
            328,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationTerminalResize {
                    id,
                    generation,
                    cols: 96,
                    rows: 32,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(resize, ServerMsg::Reply(crate::protocol::Reply::Ok)));
        assert_eq!((session.daemon.terms[&term].cols, session.daemon.terms[&term].rows), (96, 32));

        let oversized = exchange(
            &mut session,
            329,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationTerminalResize {
                    id,
                    generation,
                    cols: 501,
                    rows: 32,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(oversized, ServerMsg::Error(ref error) if error.contains("terminal size")));

        let stale = exchange(
            &mut session,
            326,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationTerminalScreen {
                id,
                generation: generation + 1,
            }),
            None,
        )
        .await;
        assert!(matches!(stale, ServerMsg::Error(ref error) if error.contains("stale")));

        let pane_input = exchange(
            &mut session,
            330,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationTerminalInput {
                    id,
                    generation,
                    data: b"x".to_vec(),
                },
            ),
            Some((term, token.clone())),
        )
        .await;
        assert!(
            matches!(pane_input, ServerMsg::Error(ref error) if error.contains("human client"))
        );

        let pane_resize = exchange(
            &mut session,
            331,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationTerminalResize {
                    id,
                    generation,
                    cols: 80,
                    rows: 24,
                },
            ),
            Some((term, token.clone())),
        )
        .await;
        assert!(matches!(pane_resize, ServerMsg::Error(ref error) if error.contains("human client")));

        let pane_replay = exchange(
            &mut session,
            333,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationTerminalReplay {
                id,
                generation,
            }),
            Some((term, token)),
        )
        .await;
        assert!(matches!(pane_replay, ServerMsg::Error(ref error) if error.contains("human client")));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn ending_a_chat_stops_its_agent_and_leaves_it_stopped_not_failed() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        assert!(session.daemon.conversation_runtimes.owns(id));

        let (client, _replies) = mpsc::channel(8);
        session.daemon.handle(Ev::Connected(340, client, false, crate::protocol::Surface::Terminal, true));
        session.daemon.conversation_end(340, id, generation).unwrap();
        tokio::time::timeout(Duration::from_secs(20), async {
            while session.daemon.conversation_runtimes.owns(id) {
                let event = session.events.recv().await.unwrap();
                session.daemon.handle(event);
            }
        })
        .await
        .expect("the agent stops");

        let record = session
            .daemon
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .unwrap();
        assert_eq!(record.state, crate::conversation::RecoveryState::Stopped);
        let state = &session.daemon.conversation_states[&id];
        assert_eq!(state.status, crate::protocol::Status::Idle);
        assert!(state.error.is_none(), "stopping on purpose is not an error");
        assert!(!session.daemon.conversation_ending.contains(&id));

        // A stale generation is refused.
        let stale = session
            .daemon
            .conversation_end(340, id, generation + 1)
            .unwrap_err();
        assert!(stale.to_string().contains("stale"));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

fn authenticated_pane(session: &mut TestSession) -> (u32, String) {
    let mut cfg = session.daemon.cfg.clone();
    cfg.shell = Some(
        session
            .fixture
            .record
            .binding
            .executable
            .to_string_lossy()
            .into_owned(),
    );
    cfg.shell_args = vec!["--descendant".into()];
    // An id of its own for each pane: a second pane under the first one's id would replace it in
    // `terms`, so cleanup would never stop the first, and its process, running the fixture's
    // executable from inside the fixture folder, keeps Windows from deleting either.
    let id = 900001 + u32::try_from(session.daemon.terms.len()).unwrap();
    let mut pane = crate::daemon::term::Term::spawn(
        &cfg,
        crate::daemon::term::SpawnSpec {
            id,
            cmd: None,
            direct: None,
            cwd: &session.fixture.record.binding.cwd,
            cols: 80,
            rows: 24,
            once: true,
        },
        session.daemon.tx.clone(),
    )
    .unwrap();
    pane.agent = Some("codex".into());
    pane.grants = Some(vec!["start".into(), "read".into(), "write".into()]);
    let token = pane.token.clone();
    session.daemon.terms.insert(id, pane);
    (id, token)
}

#[tokio::test]
async fn structured_start_honors_authenticated_pane_never_helper_policy() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let from = authenticated_pane(&mut s);
        s.daemon.cfg.sessions.helpers = "never".into();
        let target = crate::conversation::StartTarget::Create(settings(&s));
        let reply = exchange(
            &mut s,
            200,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target,
                branch: None,
            }),
            Some(from),
        )
        .await;
        assert!(matches!(reply,ServerMsg::Error(ref e) if e.contains("helpers off")));
        assert!(s.daemon.pending_starts.is_empty());
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_start_honors_authenticated_pane_ask_and_no_helper_answer() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let from = authenticated_pane(&mut s);
        s.daemon.cfg.sessions.helpers = "ask".into();
        let target = crate::conversation::StartTarget::Create(settings(&s));
        let reply = exchange(
            &mut s,
            201,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target: target.clone(),
                branch: None,
            }),
            Some(from.clone()),
        )
        .await;
        assert!(matches!(reply,ServerMsg::Error(ref e) if e.contains("user's OK")));
        assert!(!s.daemon.terms[&from.0].helpers.asked.is_empty());
        s.daemon
            .answer_helpers(from.0, crate::protocol::HelperChoice::No)
            .unwrap();
        let reply = exchange(
            &mut s,
            202,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target,
                branch: None,
            }),
            Some(from),
        )
        .await;
        assert!(matches!(reply,ServerMsg::Error(ref e) if e.contains("said no helpers")));
        assert!(s.daemon.pending_starts.is_empty());
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_helpers_count_against_authenticated_pane_concurrency_limit() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let from = authenticated_pane(&mut s);
        s.daemon.cfg.sessions.helpers = "always".into();
        s.daemon.cfg.sessions.helpers_at_once = "1".into();
        let target = crate::conversation::StartTarget::Create(settings(&s));
        let reply = exchange(
            &mut s,
            203,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target: target.clone(),
                branch: None,
            }),
            Some(from.clone()),
        )
        .await;
        let id = match reply {
            ServerMsg::Reply(crate::protocol::Reply::ConversationStarted { id, .. }) => id,
            other => panic!("{other:?}"),
        };
        assert_eq!(s.daemon.conversation_states[&id].started_by, Some(from.0));
        let reply = exchange(
            &mut s,
            204,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target,
                branch: None,
            }),
            Some(from),
        )
        .await;
        assert!(matches!(reply,ServerMsg::Error(ref e) if e.contains("at once is the limit")));
        assert_eq!(s.daemon.conversations.len(), 1);
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_read_preserves_failed_and_approval_checkpoint_context() {
    let mut s = conversation_fixture("conversation").await;
    let outcome=AssertUnwindSafe(async {
        let (id,_)=create(&mut s).await;
        for (offset,state) in [crate::protocol::CheckState::Failed,crate::protocol::CheckState::NeedsApproval].into_iter().enumerate() {
            let info=crate::protocol::CheckInfo {name:"verify".into(),command:"synthetic check".into(),state,first_failing_line:"Synthetic check context".into(),..Default::default()};
            let live=s.daemon.conversation_states.get_mut(&id).unwrap();live.status=crate::protocol::Status::Done;
            live.checks.points=vec![crate::daemon::check::CheckRecord::new(info.clone(),"").into()];
            let reply=exchange(&mut s,210+offset as u64,crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {id,cursor:None,limit:128}),None).await;
            assert!(matches!(reply,ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref r)) if r.status==crate::protocol::Status::Blocked && r.checks==[info]));
            assert_eq!(s.daemon.conversation_summaries()[0].status,crate::protocol::Status::Blocked);
        }
    }).catch_unwind().await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_provider_errors_and_failed_turns_never_read_as_done() {
    for (mode, message) in [
        ("conversation-error", "Synthetic provider failure"),
        ("conversation-failed", "Synthetic failed turn"),
        ("conversation-interrupted", "Synthetic interrupted turn"),
    ] {
        let mut s = conversation_fixture(mode).await;
        let outcome=AssertUnwindSafe(async {
            let (id,generation)=create(&mut s).await;
            let reply=exchange(&mut s,220,crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {id,generation,operation:1,text:"Synthetic failure request".into(),choice:None }),None).await;
            assert!(matches!(reply,ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
            while s.daemon.conversation_states[&id].error.is_none() {let event=tokio::time::timeout(Duration::from_secs(10),s.events.recv()).await.unwrap().unwrap();s.daemon.handle(event);}
            let reply=exchange(&mut s,221,crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {id,cursor:None,limit:128}),None).await;
            assert!(matches!(reply,ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref r)) if r.status==crate::protocol::Status::Blocked && r.error.as_deref()==Some(message)),"{reply:?}");
            assert_eq!(s.daemon.conversation_summaries()[0].status,crate::protocol::Status::Blocked);
        }).catch_unwind().await;
        s.cleanup().await;
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

#[tokio::test]
async fn command_approval_is_human_only_generation_bound_and_uses_exact_provider_id() {
    let mut session = conversation_fixture("conversation-approval").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            300,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic approval prompt".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));

        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("Codex approval request reached the daemon");

        let request_id = crate::conversation::ProviderRequestId::Text("approval-private".into());
        let approval = session.daemon.conversation_states[&id].approvals[&request_id].clone();
        assert_eq!(approval.command.as_deref(), Some("echo synthetic\\u{200b}"));
        assert_eq!(
            approval.choices,
            [
                crate::conversation::ApprovalDecision::Accept,
                crate::conversation::ApprovalDecision::Decline,
            ]
        );
        assert_eq!(serde_json::to_value(&approval.request_id).unwrap(), "approval-private");
        let read = exchange(
            &mut session,
            305,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {
                id,
                cursor: None,
                limit: 128,
            }),
            None,
        )
        .await;
        assert!(matches!(
            read,
            ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref result))
                if result.approvals == [approval.clone()]
        ));

        let unauthorized_from = authenticated_pane(&mut session);
        let unauthorized = exchange(
            &mut session,
            301,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationApprove {
                    id,
                    generation,
                    request_id: request_id.clone(),
                    decision: crate::conversation::ApprovalDecision::Accept,
                },
            ),
            Some(unauthorized_from),
        )
        .await;
        assert!(matches!(unauthorized, ServerMsg::Error(ref message) if message.contains("Only a human")));

        let unavailable = exchange(
            &mut session,
            302,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationApprove {
                    id,
                    generation,
                    request_id: request_id.clone(),
                    decision: crate::conversation::ApprovalDecision::AcceptForSession,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(unavailable, ServerMsg::Error(ref message) if message.contains("not available")));

        let wrong_request_id = exchange(
            &mut session,
            306,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationApprove {
                    id,
                    generation,
                    request_id: crate::conversation::ProviderRequestId::Text("another-request".into()),
                    decision: crate::conversation::ApprovalDecision::Accept,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(wrong_request_id, ServerMsg::Error(ref message) if message.contains("already answered")));

        let stale = exchange(
            &mut session,
            303,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationApprove {
                    id,
                    generation: generation + 1,
                    request_id: request_id.clone(),
                    decision: crate::conversation::ApprovalDecision::Accept,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(stale, ServerMsg::Error(ref message) if message.contains("stale or stopped")));

        let resolved = exchange(
            &mut session,
            304,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationApprove {
                    id,
                    generation,
                    request_id: request_id.clone(),
                    decision: crate::conversation::ApprovalDecision::Accept,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(resolved, ServerMsg::Reply(crate::protocol::Reply::Ok)));

        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].status != crate::protocol::Status::Done {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("provider completed after its approval response");

        let response = tokio::fs::read_to_string(session.fixture.directory.path().join("responses"))
            .await
            .unwrap();
        let response: Value = serde_json::from_str(response.trim()).unwrap();
        assert_eq!(response["id"], "approval-private");
        assert_eq!(response["result"]["decision"], "accept");
        assert!(session.daemon.conversation_states[&id].approvals.is_empty());

        // The answer outlives the request: a viewer that restarts still reads what was decided.
        let reread = exchange(
            &mut session,
            307,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {
                id,
                cursor: None,
                limit: 128,
            }),
            None,
        )
        .await;
        let ServerMsg::Reply(crate::protocol::Reply::ConversationRead(result)) = reread else {
            panic!("expected a conversation read");
        };
        assert!(result.approvals.is_empty());
        let [answered] = result.answered_approvals.as_slice() else {
            panic!("exactly one answered approval is kept");
        };
        assert_eq!(answered.request, approval);
        assert_eq!(answered.decision, crate::conversation::ApprovalDecision::Accept);
        assert!(answered.answered_at_ms.is_some());
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn an_approved_command_is_remembered_for_the_project_and_nothing_else_is() {
    use crate::conversation::ProviderRequestId;
    let mut session = conversation_fixture("conversation-approval").await;
    let outcome = AssertUnwindSafe(async {
        // Never the real data folder.
        let file = session.fixture.directory.path().join("chat-allowed-commands.json");
        session.daemon.chat_allowed_file = file.clone();
        let (id, generation) = create(&mut session).await;
        let send = |operation: u64| crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
            id,
            generation,
            operation,
            text: "Synthetic approval prompt".into(),
            choice: None,
        });
        assert!(matches!(exchange(&mut session, 400, send(1), None).await, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("the request reached the daemon");
        let request_id = ProviderRequestId::Text("approval-private".into());
        assert!(session.daemon.conversation_states[&id].approvals[&request_id].can_remember);
        let always = |request_id: &ProviderRequestId| crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationAllowAlways {
            id,
            generation,
            request_id: request_id.clone(),
        });

        // An agent cannot approve for itself, and nothing is remembered by the try.
        let from = authenticated_pane(&mut session);
        let refused = exchange(&mut session, 401, always(&request_id), Some(from)).await;
        assert!(matches!(refused, ServerMsg::Error(ref message) if message.contains("Only a human")), "{refused:?}");
        assert!(session.daemon.chat_allowed.rules.is_empty());
        assert!(!session.daemon.conversation_states[&id].approvals.is_empty());

        // A request that is not there is not remembered either.
        let stale = exchange(&mut session, 402, always(&ProviderRequestId::Text("another".into())), None).await;
        assert!(matches!(stale, ServerMsg::Error(ref message) if message.contains("already answered")), "{stale:?}");

        let done = exchange(&mut session, 403, always(&request_id), None).await;
        assert!(matches!(done, ServerMsg::Reply(crate::protocol::Reply::Ok)), "{done:?}");
        let [rule] = session.daemon.chat_allowed.rules.as_slice() else {
            panic!("one rule is kept");
        };
        assert!(rule.command.starts_with("echo synthetic"));
        assert_eq!(rule.command_sha256, crate::protocol::command_hash("echo synthetic\u{200b}"));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].status != crate::protocol::Status::Done {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("the first turn finished");
        let response = tokio::fs::read_to_string(session.fixture.directory.path().join("responses")).await.unwrap();
        let response: Value = serde_json::from_str(response.lines().next().unwrap()).unwrap();
        assert_eq!(response["result"]["decision"], "accept", "remembered, but this one is a plain accept");

        // The same command in the same project is answered without asking.
        assert!(matches!(exchange(&mut session, 404, send(2), None).await, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].answered_approvals.len() < 2 {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("the second request was answered by the daemon");
        assert!(session.daemon.conversation_states[&id].approvals.is_empty());
        assert!(session.daemon.conversation_states[&id].command_hashes.is_empty());

        // A rule for another project does not apply here.
        session.daemon.chat_allowed.rules[0].project = std::path::PathBuf::from("/somewhere/else");
        assert!(!session.daemon.chat_allowed.allows_hash(
            &session.daemon.conversation_project(id).unwrap(),
            &session.daemon.chat_allowed.rules[0].command_sha256.clone(),
        ));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn approved_commands_are_listed_and_taken_back_by_a_person_only() {
    let mut session = conversation_fixture("conversation-approval").await;
    let outcome = AssertUnwindSafe(async {
        let file = session.fixture.directory.path().join("chat-allowed-commands.json");
        session.daemon.chat_allowed_file = file.clone();
        let rule = super::allowed::Rule {
            project: std::path::PathBuf::from("/code/app"),
            command_sha256: crate::protocol::command_hash("cargo test"),
            command: "cargo test".into(),
            at: 5,
        };
        // Another daemon wrote it: the list reads the file as it is now.
        super::allowed::remember(&file, rule.clone()).unwrap();
        let list = crate::protocol::ClientMsg::Query(crate::protocol::Query::AllowedCommands);
        let from = authenticated_pane(&mut session);
        let refused = exchange(&mut session, 430, list.clone(), Some(from)).await;
        assert!(matches!(refused, ServerMsg::Error(ref message) if message.contains("Only a human")), "{refused:?}");
        let ServerMsg::Reply(crate::protocol::Reply::AllowedCommands(listed)) = exchange(&mut session, 431, list.clone(), None).await else {
            panic!("expected the approved commands");
        };
        assert_eq!(listed.len(), 1);
        assert_eq!((listed[0].command.as_str(), listed[0].at), ("cargo test", 5));

        let forget = crate::protocol::ClientMsg::Command(crate::protocol::Command::ForgetAllowedCommand {
            project: rule.project.clone(),
            command_sha256: rule.command_sha256.clone(),
        });
        let from = authenticated_pane(&mut session);
        let refused = exchange(&mut session, 432, forget.clone(), Some(from)).await;
        assert!(matches!(refused, ServerMsg::Error(_)), "{refused:?}");
        assert_eq!(session.daemon.chat_allowed.rules.len(), 1, "a pane cannot take it back either");
        let done = exchange(&mut session, 433, forget, None).await;
        assert!(matches!(done, ServerMsg::Reply(crate::protocol::Reply::Ok)), "{done:?}");
        assert!(session.daemon.chat_allowed.rules.is_empty(), "asks again from now on");
        // The file follows, off the loop.
        tokio::time::timeout(Duration::from_secs(5), async {
            while !super::allowed::Allowed::load(&file).unwrap().rules.is_empty() {
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        })
        .await
        .expect("the file forgot it too");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_file_change_request_cannot_be_remembered() {
    let mut session = conversation_fixture("conversation-file-approval").await;
    let outcome = AssertUnwindSafe(async {
        session.daemon.chat_allowed_file = session.fixture.directory.path().join("chat-allowed-commands.json");
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            410,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend { id, generation, operation: 1, text: "Synthetic file change request".into(), choice: None }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("the request reached the daemon");
        let request_id = crate::conversation::ProviderRequestId::Text("file-approval-private".into());
        assert!(!session.daemon.conversation_states[&id].approvals[&request_id].can_remember);
        let refused = exchange(
            &mut session,
            411,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationAllowAlways { id, generation, request_id }),
            None,
        )
        .await;
        assert!(matches!(refused, ServerMsg::Error(ref message) if message.contains("Only a command request")), "{refused:?}");
        assert!(session.daemon.chat_allowed.rules.is_empty());
        assert!(!session.daemon.conversation_states[&id].approvals.is_empty(), "left for the person to answer");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_closed_chat_is_hidden_at_once_comes_back_with_undo_and_ends_when_the_window_passes() {
    let mut session = conversation_fixture("conversation-approval").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let archived = |s: &TestSession| s.daemon.conversations.iter().find(|c| c.id == id).unwrap().archived;
        let close = crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationClose { id, generation });
        let undo = crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationUndoClose { id });

        // An agent cannot close a person's chat.
        let from = authenticated_pane(&mut session);
        let refused = exchange(&mut session, 420, close.clone(), Some(from)).await;
        assert!(matches!(refused, ServerMsg::Error(ref message) if message.contains("Only a human")), "{refused:?}");
        assert!(!archived(&session));

        let closed = exchange(&mut session, 421, close.clone(), None).await;
        assert!(matches!(closed, ServerMsg::Reply(crate::protocol::Reply::Ok)), "{closed:?}");
        assert!(archived(&session), "hidden at once");
        assert!(session.daemon.conversation_runtimes.owns(id), "still running inside the window");

        let back = exchange(&mut session, 422, undo.clone(), None).await;
        assert!(matches!(back, ServerMsg::Reply(crate::protocol::Reply::Ok)), "{back:?}");
        assert!(!archived(&session), "brought back");
        assert!(session.daemon.conversation_closing.is_empty());
        let late = exchange(&mut session, 423, undo.clone(), None).await;
        assert!(matches!(late, ServerMsg::Error(ref message) if message.contains("window has passed")), "{late:?}");

        // Closed again and left alone: the window passing ends it, and it stays hidden.
        assert!(matches!(exchange(&mut session, 424, close, None).await, ServerMsg::Reply(crate::protocol::Reply::Ok)));
        session.daemon.finish_chat_closes();
        assert!(session.daemon.conversation_runtimes.owns(id), "not before the window passes");
        *session.daemon.conversation_closing.get_mut(&id).unwrap() = std::time::Instant::now() - crate::daemon::UNDO_WINDOW;
        session.daemon.finish_chat_closes();
        assert!(session.daemon.conversation_closing.is_empty());
        assert!(archived(&session));
        let late = exchange(&mut session, 425, undo, None).await;
        assert!(matches!(late, ServerMsg::Error(ref message) if message.contains("window has passed")), "{late:?}");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn file_change_approval_is_human_only_and_uses_exact_provider_id() {
    let mut session = conversation_fixture("conversation-file-approval").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            310,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic file change request".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(
            sent,
            ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))
        ));

        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("Codex file-change request reached the daemon");

        let request_id =
            crate::conversation::ProviderRequestId::Text("file-approval-private".into());
        let approval = session.daemon.conversation_states[&id].approvals[&request_id].clone();
        assert_eq!(
            approval.kind,
            crate::conversation::ApprovalRequestKind::FileChange
        );
        assert_eq!(approval.reason.as_deref(), Some("Synthetic file change"));
        assert_eq!(approval.changes.len(), 1);
        assert_eq!(approval.changes[0].path, "src/synthetic.rs");
        assert_eq!(approval.changes[0].diff, "@@ -1 +1 @@\n-old\n+new\n");
        assert_eq!(
            approval.cwd.as_deref(),
            Some(std::path::Path::new("C:/synthetic/project"))
        );
        assert_eq!(
            approval.choices,
            [
                crate::conversation::ApprovalDecision::Accept,
                crate::conversation::ApprovalDecision::Decline,
            ]
        );

        let response = exchange(
            &mut session,
            311,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationApprove {
                id,
                generation,
                request_id,
                decision: crate::conversation::ApprovalDecision::Accept,
            }),
            None,
        )
        .await;
        assert!(matches!(
            response,
            ServerMsg::Reply(crate::protocol::Reply::Ok)
        ));

        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].status != crate::protocol::Status::Done {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("Codex completed after receiving the file-change decision");

        let response =
            tokio::fs::read_to_string(session.fixture.directory.path().join("responses"))
                .await
                .unwrap();
        let response: Value = serde_json::from_str(response.trim()).unwrap();
        assert_eq!(response["id"], "file-approval-private");
        assert_eq!(response["result"]["decision"], "accept");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_file_change_approval_for_a_secret_file_keeps_the_path_and_omits_the_diff() {
    let mut session = conversation_fixture("conversation-file-approval-secret").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            320,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic secret file request".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(
            sent,
            ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))
        ));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("the file-change request reached the daemon");

        let request_id =
            crate::conversation::ProviderRequestId::Text("file-approval-private".into());
        let approval = session.daemon.conversation_states[&id].approvals[&request_id].clone();
        assert_eq!(approval.changes.len(), 1);
        assert_eq!(approval.changes[0].path, "app/.env");
        assert_eq!(approval.changes[0].kind, "add");
        assert_eq!(approval.changes[0].diff, super::redact::OMITTED);
        assert_eq!(
            approval.reason.as_deref(),
            Some("Write the token [redacted] to .env")
        );
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn file_change_approval_is_disabled_until_the_exact_diff_arrives() {
    let mut session = conversation_fixture("conversation-file-approval-no-patch").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            312,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic file change without patch".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("file-change request reached the daemon");

        let request_id = crate::conversation::ProviderRequestId::Text("file-approval-private".into());
        let approval = &session.daemon.conversation_states[&id].approvals[&request_id];
        assert!(approval.changes.is_empty());
        assert!(approval.choices.is_empty());
        let unavailable = exchange(
            &mut session,
            313,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationApprove {
                    id,
                    generation,
                    request_id,
                    decision: crate::conversation::ApprovalDecision::Accept,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(unavailable, ServerMsg::Error(ref message) if message.contains("not available")));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_partial_malformed_file_diff_never_enables_approval() {
    let mut session = conversation_fixture("conversation-file-approval-malformed").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            320,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic malformed file diff".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(
            sent,
            ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))
        ));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("file-change request reached the daemon");

        let approval = session.daemon.conversation_states[&id]
            .approvals
            .values()
            .next()
            .unwrap();
        assert!(approval.changes.is_empty());
        assert!(approval.choices.is_empty());
        assert!(
            session.daemon.conversation_states[&id]
                .error
                .as_deref()
                .is_some_and(|error| error.contains("incomplete file-change diff"))
        );
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn permission_approval_grants_only_the_requested_profile_for_the_turn() {
    let mut session = conversation_fixture("conversation-permission-approval").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            318,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic permission request".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));

        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("Codex permission request reached the daemon");

        let request_id = crate::conversation::ProviderRequestId::Text(
            "permission-approval-private".into(),
        );
        let approval = session.daemon.conversation_states[&id].approvals[&request_id].clone();
        let requested = json!({
            "network": {"enabled": true},
            "fileSystem": {"write": ["C:/synthetic/project/output"]},
        });
        assert_eq!(approval.kind, crate::conversation::ApprovalRequestKind::Permission);
        assert_eq!(approval.permissions, Some(requested.clone()));
        assert_eq!(approval.cwd.as_deref(), Some(Path::new("C:/synthetic/project")));
        assert_eq!(
            approval.choices,
            [
                crate::conversation::ApprovalDecision::Decline,
                crate::conversation::ApprovalDecision::Accept,
            ]
        );

        let pane = authenticated_pane(&mut session);
        let unauthorized = exchange(
            &mut session,
            319,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationApprove {
                id,
                generation,
                request_id: request_id.clone(),
                decision: crate::conversation::ApprovalDecision::Accept,
            }),
            Some(pane),
        )
        .await;
        assert!(matches!(unauthorized, ServerMsg::Error(ref message) if message.contains("Only a human")));

        let session_scope = exchange(
            &mut session,
            320,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationApprove {
                id,
                generation,
                request_id: request_id.clone(),
                decision: crate::conversation::ApprovalDecision::AcceptForSession,
            }),
            None,
        )
        .await;
        assert!(matches!(session_scope, ServerMsg::Error(ref message) if message.contains("not available")));

        let accepted = exchange(
            &mut session,
            321,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationApprove {
                id,
                generation,
                request_id,
                decision: crate::conversation::ApprovalDecision::Accept,
            }),
            None,
        )
        .await;
        assert!(matches!(accepted, ServerMsg::Reply(crate::protocol::Reply::Ok)));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].status != crate::protocol::Status::Done {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("provider completed after its permission response");

        let response = tokio::fs::read_to_string(session.fixture.directory.path().join("responses"))
            .await
            .unwrap();
        let response: Value = serde_json::from_str(response.trim()).unwrap();
        assert_eq!(response["id"], "permission-approval-private");
        assert_eq!(response["result"]["permissions"], requested);
        assert_eq!(response["result"]["scope"], "turn");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn user_input_requests_use_exact_ids_and_validated_ephemeral_answers() {
    let mut session = conversation_fixture("conversation-user-input").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            314,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic input request".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].approvals.is_empty() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("Codex input request reached the daemon");

        let request_id = crate::conversation::ProviderRequestId::Text("user-input-private".into());
        let approval = &session.daemon.conversation_states[&id].approvals[&request_id];
        assert_eq!(approval.kind, crate::conversation::ApprovalRequestKind::UserInput);
        assert_eq!(approval.questions.len(), 2);
        assert!(approval.questions[0].is_other);
        assert!(approval.questions[1].is_secret);
        assert_eq!(approval.questions[0].options[0].label, "A");

        let answers = std::collections::BTreeMap::from([
            ("choice".to_owned(), vec!["A".to_owned(), "user_note: details".to_owned()]),
            ("secret".to_owned(), vec!["private answer".to_owned()]),
        ]);
        let pane = authenticated_pane(&mut session);
        let unauthorized = exchange(
            &mut session,
            315,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationAnswerInput {
                    id,
                    generation,
                    request_id: request_id.clone(),
                    answers: answers.clone(),
                },
            ),
            Some(pane),
        )
        .await;
        assert!(matches!(unauthorized, ServerMsg::Error(ref message) if message.contains("Only a human")));

        let incomplete = exchange(
            &mut session,
            316,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationAnswerInput {
                    id,
                    generation,
                    request_id: request_id.clone(),
                    answers: std::collections::BTreeMap::new(),
                },
            ),
            None,
        )
        .await;
        assert!(matches!(incomplete, ServerMsg::Error(ref message) if message.contains("Answer every")));

        let invalid_option = std::collections::BTreeMap::from([
            ("choice".to_owned(), vec!["not-offered".to_owned()]),
            ("secret".to_owned(), vec!["private answer".to_owned()]),
        ]);
        let invalid = exchange(
            &mut session,
            317,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationAnswerInput {
                    id,
                    generation,
                    request_id: request_id.clone(),
                    answers: invalid_option,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(invalid, ServerMsg::Error(ref message) if message.contains("available choices")));

        let hidden = std::collections::BTreeMap::from([
            ("choice".to_owned(), vec!["A".to_owned()]),
            ("secret".to_owned(), vec!["hidden\u{200b}text".to_owned()]),
        ]);
        let refused = exchange(
            &mut session,
            318,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationAnswerInput {
                    id,
                    generation,
                    request_id: request_id.clone(),
                    answers: hidden,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(refused, ServerMsg::Error(ref message) if message.contains("hidden characters")));

        let answered = exchange(
            &mut session,
            319,
            crate::protocol::ClientMsg::Command(
                crate::protocol::Command::ConversationAnswerInput {
                    id,
                    generation,
                    request_id,
                    answers,
                },
            ),
            None,
        )
        .await;
        assert!(matches!(answered, ServerMsg::Reply(crate::protocol::Reply::Ok)));
        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].status != crate::protocol::Status::Done {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("Codex completed after receiving its user answers");

        let response = tokio::fs::read_to_string(session.fixture.directory.path().join("responses"))
            .await
            .unwrap();
        let response: Value = serde_json::from_str(response.trim()).unwrap();
        assert_eq!(response["id"], "user-input-private");
        assert_eq!(response["result"]["answers"]["choice"]["answers"][0], "A");
        assert_eq!(response["result"]["answers"]["choice"]["answers"][1], "user_note: details");
        assert_eq!(response["result"]["answers"]["secret"]["answers"][0], "private answer");
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn turn_cancellation_uses_only_the_acknowledged_active_turn_id() {
    let mut session = conversation_fixture("conversation-cancel").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut session).await;
        let sent = exchange(
            &mut session,
            320,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic long turn".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(sent, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));

        let stale = exchange(
            &mut session,
            321,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationCancel {
                id,
                generation: generation + 1,
            }),
            None,
        )
        .await;
        assert!(matches!(stale, ServerMsg::Error(ref message) if message.contains("stale or stopped")));

        let from = authenticated_pane(&mut session);
        let unauthorized = exchange(
            &mut session,
            322,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationCancel {
                id,
                generation,
            }),
            Some(from),
        )
        .await;
        assert!(matches!(unauthorized, ServerMsg::Error(ref message) if message.contains("Only a human")));

        let cancelled = exchange(
            &mut session,
            323,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationCancel {
                id,
                generation,
            }),
            None,
        )
        .await;
        assert!(matches!(cancelled, ServerMsg::Reply(crate::protocol::Reply::Ok)));

        tokio::time::timeout(Duration::from_secs(10), async {
            while session.daemon.conversation_states[&id].cancelling.is_some() {
                session.daemon.handle(session.events.recv().await.unwrap());
            }
        })
        .await
        .expect("Codex completed the exact interrupted turn");

        let request = tokio::fs::read_to_string(session.fixture.directory.path().join("interrupts"))
            .await
            .unwrap();
        let requests: Vec<Value> = request.lines().map(|line| serde_json::from_str(line).unwrap()).collect();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0]["method"], "turn/interrupt");
        assert_eq!(requests[0]["params"]["threadId"], "exact-thread");
        assert_eq!(requests[0]["params"]["turnId"], "turn-1");
        assert_eq!(session.daemon.conversation_states[&id].status, crate::protocol::Status::Idle);
        assert_eq!(session.daemon.conversation_states[&id].error, None);

        let read = exchange(
            &mut session,
            324,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {
                id,
                cursor: None,
                limit: 128,
            }),
            None,
        )
        .await;
        assert!(matches!(
            read,
            ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref result))
                if !result.cancelling
                    && result.history.entries.iter().any(|entry| {
                        entry.content == crate::conversation::history::Content::TurnStatus
                            && entry.state == crate::conversation::history::ItemState::Interrupted
                    })
        ));

        let continued = exchange(
            &mut session,
            325,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 2,
                text: "Continue after stopping the previous turn".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(continued, ServerMsg::Reply(crate::protocol::Reply::ConversationSent(ref receipt)) if receipt.operation == 2));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_runtime_claims_apply_grants_without_human_or_pane_exemptions() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let (id, _) = create(&mut s).await;
        let claim = s.daemon.conversation_runtimes.test_identity(id);
        let pane = authenticated_pane(&mut s).0;
        s.daemon.cfg.mcp.grants = vec!["read".into()];
        let list = exchange_claim(
            &mut s,
            230,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::List),
            None,
            Some(claim.clone()),
            None,
        )
        .await;
        assert!(matches!(
            list,
            ServerMsg::Reply(crate::protocol::Reply::List(_))
        ));
        let write = crate::protocol::ClientMsg::Command(crate::protocol::Command::QueueFollowUp {
            term: pane,
            text: "Synthetic scoped write".into(),
            first: false,
        });
        let denied =
            exchange_claim(&mut s, 231, write.clone(), None, Some(claim.clone()), None).await;
        assert!(matches!(denied,ServerMsg::Error(ref error) if error.contains("may not")));
        assert!(s.daemon.terms[&pane].queue.is_empty());
        s.daemon.cfg.mcp.grants = vec!["write".into()];
        let accepted = exchange_claim(&mut s, 232, write, None, Some(claim.clone()), None).await;
        assert!(matches!(
            accepted,
            ServerMsg::Reply(crate::protocol::Reply::Ok)
        ));
        assert_eq!(s.daemon.terms[&pane].queue.len(), 1);
        let denied = exchange_claim(
            &mut s,
            233,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::List),
            None,
            Some(claim.clone()),
            None,
        )
        .await;
        assert!(matches!(denied, ServerMsg::Error(_)));
        s.daemon.cfg.mcp.grants = vec!["read".into(), "start".into(), "admin".into()];
        for (client, command) in [
            (
                234,
                crate::protocol::Command::Grant {
                    term: pane,
                    grants: Some(vec!["admin".into()]),
                },
            ),
            (
                235,
                crate::protocol::Command::AnswerHelpers {
                    term: pane,
                    choice: crate::protocol::HelperChoice::Allow,
                },
            ),
            (
                236,
                crate::protocol::Command::NewWorkspace {
                    cwd: None,
                    name: None,
                    cmd: None,
                },
            ),
        ] {
            let denied = exchange_claim(
                &mut s,
                client,
                crate::protocol::ClientMsg::Command(command),
                None,
                Some(claim.clone()),
                None,
            )
            .await;
            assert!(matches!(denied, ServerMsg::Error(_)));
        }
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_invalid_and_revoked_claims_never_fall_back_to_human() {
    let mut s = conversation_fixture("conversation").await;
    let outcome=AssertUnwindSafe(async {
        let (id,generation)=create(&mut s).await;
        let claim=s.daemon.conversation_runtimes.test_identity(id);
        let mut bad=claim.clone();bad.token="wrong-secret".into();
        let denied=exchange_claim(&mut s,240,crate::protocol::ClientMsg::Query(crate::protocol::Query::List),None,Some(bad),None).await;
        assert!(matches!(denied,ServerMsg::Error(_)));
        let (client,mut replies)=mpsc::channel(8);
        s.daemon.handle(Ev::StructuredConnected {id:241,tx:client,attach:false,claim:claim.clone()});
        s.daemon.conversations[0].generation=generation+1;
        s.daemon.message(241,crate::protocol::ClientMsg::Query(crate::protocol::Query::List));
        assert!(matches!(replies.recv().await,Some(ServerMsg::Error(ref error)) if error.contains("stale or revoked")));
        let denied=exchange_claim(&mut s,242,crate::protocol::ClientMsg::Query(crate::protocol::Query::List),None,Some(claim),None).await;
        assert!(matches!(denied,ServerMsg::Error(_)));
        s.daemon.conversations[0].generation=generation;
    }).catch_unwind().await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_read_acknowledgement_waits_for_durable_receipt_barrier() {
    for fail in [false, true] {
        let mut s = conversation_fixture("conversation").await;
        let outcome=AssertUnwindSafe(async {
            let (id,generation)=create(&mut s).await;
            let (client,mut replies)=mpsc::channel(8);s.daemon.handle(Ev::Connected(250,client,false, crate::protocol::Surface::Terminal, true));
            s.daemon.conversation_send(250,id,generation,1,"Synthetic delayed ack".into(), Default::default()).unwrap();
            loop {
                let event=tokio::time::timeout(Duration::from_secs(20),s.events.recv()).await.unwrap().unwrap();
                let submitted=matches!(&event,Ev::Conversation(Event {payload:Payload::Response {token,..},..}) if matches!(s.daemon.conversation_requests.get(token),Some(super::commands::Pending::Sending {..})));
                if submitted {
                    if fail {let temporary=s.daemon.session_path.with_extension("json.tmp");tokio::task::spawn_blocking(move||std::fs::create_dir(temporary).unwrap()).await.unwrap();}
                    s.daemon.handle(event);break;
                }
                s.daemon.handle(event);
            }
            let mut deferred=Vec::new();
            let read=exchange_claim(&mut s,251,crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {id,cursor:None,limit:128}),None,None,Some(&mut deferred)).await;
            assert!(matches!(read,ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref read)) if read.receipt.as_ref().is_some_and(|receipt|receipt.delivery==crate::conversation::Delivery::SubmissionUncertain)));
            for event in deferred {s.daemon.handle(event);}
            let result=drive_reply(&mut s,&mut replies).await;
            assert_eq!(matches!(result,ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))),!fail);
            let read=exchange(&mut s,252,crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {id,cursor:None,limit:128}),None).await;
            assert!(matches!(read,ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref read)) if read.receipt.as_ref().is_some_and(|receipt|matches!(receipt.delivery,crate::conversation::Delivery::Acknowledged {..})!=fail)));
        }).catch_unwind().await;
        s.cleanup().await;
        if let Err(panic) = outcome {
            std::panic::resume_unwind(panic);
        }
    }
}

/// The send is acknowledged before the provider's streamed items reach the
/// daemon, so a read right after the ack can legitimately miss them. Read until
/// the message is in the history, bounded by a deadline.
async fn wait_for_history_message(
    session: &mut TestSession,
    id: crate::conversation::ConversationId,
    wanted: &str,
) {
    let deadline = tokio::time::Instant::now() + Duration::from_secs(30);
    for client in 1000u64.. {
        let read = exchange(
            session,
            client,
            crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {
                id,
                cursor: None,
                limit: 128,
            }),
            None,
        )
        .await;
        let found = matches!(read, ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref r))
            if r.history.entries.iter().any(|e| matches!(&e.content, super::history::Content::Message { text, .. } if text == wanted)));
        if found {
            return;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "history never contained {wanted:?}"
        );
        tokio::task::yield_now().await;
    }
}

#[tokio::test]
async fn structured_local_ipc_create_send_read_reconnect_and_exact_identity() {
    let mut s = conversation_fixture("conversation").await;
    let outcome=AssertUnwindSafe(async {
        s.daemon.initialize_conversation_cache().await.unwrap();
        let (id,generation)=create(&mut s).await;
        assert_eq!(s.daemon.conversations[0].binding.session_id,"exact-thread");
        let sent=exchange(&mut s,101,crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend { id,generation,operation:1,text:"Synthetic prompt".into(),choice:None }),None).await;
        assert!(matches!(sent,ServerMsg::Reply(crate::protocol::Reply::ConversationSent(crate::conversation::Receipt { delivery:crate::conversation::Delivery::Acknowledged {..},.. }))));
        wait_for_history_message(&mut s,id,"Synthetic stream complete").await;
        let retry=exchange(&mut s,103,crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend { id,generation,operation:1,text:"Synthetic prompt".into(),choice:None }),None).await;
        assert!(matches!(retry,ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
        let root=s.fixture.directory.path().to_owned();
        let (log,saved)=tokio::task::spawn_blocking(move||(std::fs::read_to_string(root.join("requests")).unwrap(),std::fs::read_to_string(root.join("data/session.json")).unwrap())).await.unwrap();
        assert_eq!(log.lines().count(),1);
        assert!(!saved.contains("Synthetic prompt"));
        assert!(saved.contains("turn-1"));
        let stale=exchange(&mut s,104,crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend { id,generation:generation+1,operation:2,text:"Synthetic stale".into(),choice:None }),None).await;
        assert!(matches!(stale,ServerMsg::Error(ref e) if e.contains("Stale")));
    }).catch_unwind().await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_local_ipc_missing_exact_thread_never_creates_fallback() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let target = crate::conversation::StartTarget::Session {
            settings: settings(&s),
            thread_id: "missing-thread".into(),
        };
        let reply = exchange(
            &mut s,
            110,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target,
                branch: None,
            }),
            None,
        )
        .await;
        assert!(matches!(reply, ServerMsg::Error(_)));
        assert!(s.daemon.conversations.is_empty());
        assert_eq!(s.daemon.pending_starts.len(), 1);
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_delivery_stays_uncertain_after_provider_disconnect_and_restart() {
    let mut s = conversation_fixture("conversation-disconnect").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut s).await;
        let reply = exchange(
            &mut s,
            120,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic uncertain prompt".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(reply, ServerMsg::Error(_)));
        let retry = exchange(
            &mut s,
            121,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic uncertain prompt".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(retry,ServerMsg::Error(ref e) if e.contains("uncertain")));
        let path = s.daemon.session_path.clone();
        let saved: crate::daemon::persist::Saved = tokio::task::spawn_blocking(move || {
            serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
        })
        .await
        .unwrap();
        assert_eq!(
            saved.conversation_operations[0].receipt.delivery,
            crate::conversation::Delivery::SubmissionUncertain
        );
        s.daemon.restore(saved);
        let recovered = s.daemon.conversations[0].generation;
        assert!(recovered > generation);
        let next = exchange(
            &mut s,
            122,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation: recovered,
                operation: 2,
                text: "Synthetic next prompt".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(next,ServerMsg::Error(ref e) if e.contains("uncertain")));
        let path = s.fixture.directory.path().join("requests");
        let count = tokio::task::spawn_blocking(move || {
            std::fs::read_to_string(path).unwrap().lines().count()
        })
        .await
        .unwrap();
        assert_eq!(count, 1);
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_unknown_provider_request_remains_blocked_and_unanswered() {
    let mut s = conversation_fixture("conversation-blocked").await;
    let outcome = AssertUnwindSafe(async {
        let (id, generation) = create(&mut s).await;
        let sent = exchange(
            &mut s,
            130,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 1,
                text: "Synthetic question".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(
            sent,
            ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))
        ));
        while !s
            .daemon
            .conversation_states
            .get(&id)
            .is_some_and(|s| s.blocked)
        {
            let event = tokio::time::timeout(Duration::from_secs(10), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            s.daemon.handle(event);
        }
        assert_eq!(
            s.daemon.conversation_summaries()[0].status,
            crate::protocol::Status::Blocked
        );
        let next = exchange(
            &mut s,
            131,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {
                id,
                generation,
                operation: 2,
                text: "Synthetic blocked followup".into(),
                choice: None,
            }),
            None,
        )
        .await;
        assert!(matches!(next,ServerMsg::Error(ref e) if e.contains("blocked")));
        assert!(s.daemon.conversation_runtimes.owns(id));
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_chat_refuses_native_conversations_until_history_is_reconciled() {
    let mut session = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = crate::conversation::ConversationId(48);
        let generation = 5;
        let mut conversation = session.fixture.record.clone();
        conversation.id = id;
        conversation.generation = generation;
        conversation.execution = crate::conversation::ExecutionKind::Native;
        conversation.state = crate::conversation::RecoveryState::RecoveryRequired;
        session.daemon.conversations.push(conversation);
        let (client, _) = mpsc::channel(8);
        session.daemon.handle(Ev::Connected(326, client, false, crate::protocol::Surface::Terminal, true));

        let result = session.daemon.conversation_start(
            326,
            crate::conversation::StartTarget::Conversation { id, generation },
        );
        assert!(matches!(result, Err(ref error) if error.to_string().contains("history must be reconciled")));
        assert!(!session.daemon.conversation_runtimes.owns(id));
    })
    .catch_unwind()
    .await;
    session.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_creation_never_launches_when_restore_is_disabled_during_barrier() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let (client, _) = mpsc::channel(8);
        s.daemon.handle(Ev::Connected(140, client, false, crate::protocol::Surface::Terminal, true));
        let target = crate::conversation::StartTarget::Create(settings(&s));
        s.daemon.conversation_start(140, target).unwrap();
        let event = loop {
            let event = tokio::time::timeout(Duration::from_secs(10), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, Ev::SessionSaved { .. }) {
                break event;
            }
            s.daemon.handle(event);
        };
        assert!(matches!(event, Ev::SessionSaved { .. }));
        s.daemon.cfg.restore.enabled = false;
        s.daemon.handle(event);
        assert!(
            !s.daemon
                .conversation_runtimes
                .owns(crate::conversation::ConversationId(1))
        );
        let path = s.fixture.directory.path().join("pids");
        assert!(
            !tokio::task::spawn_blocking(move || path.exists())
                .await
                .unwrap()
        );
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

async fn drive_reply(
    session: &mut TestSession,
    replies: &mut mpsc::Receiver<ServerMsg>,
) -> ServerMsg {
    tokio::time::timeout(Duration::from_secs(20),async {
        loop {
            tokio::select! {
                event=session.events.recv()=>session.daemon.handle(event.unwrap()),
                reply=replies.recv()=>match reply.unwrap() {reply@ServerMsg::Reply(_)|reply@ServerMsg::Error(_)=>break reply,_=>()}
            }
        }
    }).await.unwrap()
}

#[tokio::test]
async fn structured_failed_ack_persistence_never_returns_an_acknowledged_retry() {
    let mut s = conversation_fixture("conversation").await;
    let outcome=AssertUnwindSafe(async {
        let (id,generation)=create(&mut s).await;
        let (client,mut replies)=mpsc::channel(8);s.daemon.handle(Ev::Connected(170,client,false, crate::protocol::Surface::Terminal, true));
        s.daemon.conversation_send(170,id,generation,1,"Synthetic ack fault".into(), Default::default()).unwrap();
        loop {
            let event=tokio::time::timeout(Duration::from_secs(20),s.events.recv()).await.unwrap().unwrap();
            let submission=matches!(&event,Ev::Conversation(Event {payload:Payload::Response {token,..},..}) if matches!(s.daemon.conversation_requests.get(token),Some(super::commands::Pending::Sending {..})));
            if submission {
                let temporary=s.daemon.session_path.with_extension("json.tmp");
                tokio::task::spawn_blocking(move||std::fs::create_dir(temporary).unwrap()).await.unwrap();
                s.daemon.handle(event);break;
            }
            s.daemon.handle(event);
        }
        assert!(matches!(drive_reply(&mut s,&mut replies).await,ServerMsg::Error(_)));
        assert_eq!(s.daemon.conversation_operations[0].receipt.delivery,crate::conversation::Delivery::SubmissionUncertain);
        let retry=exchange(&mut s,171,crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {id,generation,operation:1,text:"Synthetic ack fault".into(),choice:None }),None).await;
        assert!(matches!(retry,ServerMsg::Error(ref e) if e.contains("uncertain")));
    }).catch_unwind().await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_output_false_reads_native_memory_without_a_detached_cache() {
    let mut s = conversation_fixture("conversation").await;
    let outcome=AssertUnwindSafe(async {
        s.daemon.cfg.restore.output=false;
        s.daemon.initialize_conversation_cache().await.unwrap();
        let (id,generation)=create(&mut s).await;
        let sent=exchange(&mut s,180,crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationSend {id,generation,operation:1,text:"Synthetic private prompt".into(),choice:None }),None).await;
        assert!(matches!(sent,ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))));
        let read=exchange(&mut s,181,crate::protocol::ClientMsg::Query(crate::protocol::Query::ConversationRead {id,cursor:None,limit:128}),None).await;
        assert!(matches!(read,ServerMsg::Reply(crate::protocol::Reply::ConversationRead(ref r)) if !r.history.entries.is_empty()));
        assert!(s.daemon.conversation_histories.is_empty());
        assert!(s.daemon.conversation_history.get(181).is_none());
        let path=s.daemon.session_path.with_extension("history");
        assert!(!tokio::task::spawn_blocking(move||path.exists()).await.unwrap());
    }).catch_unwind().await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn structured_newer_full_commit_releases_two_matching_conversation_barriers() {
    let mut s = conversation_fixture("conversation").await;
    let second = Fixture::new().await;
    let root = second.directory.path().to_path_buf();
    tokio::task::spawn_blocking(move || std::fs::write(root.join("mode"), "conversation").unwrap())
        .await
        .unwrap();
    let outcome = AssertUnwindSafe(async {
        let (first, generation) = create(&mut s).await;
        let b = &second.record.binding;
        let target =
            crate::conversation::StartTarget::Create(crate::conversation::LaunchSettings {
                provider: crate::conversation::Provider::Codex,
                executable: b.executable.clone(),
                config_root: b.config_root.clone(),
                cwd: b.cwd.clone(),
                model: None,
                model_provider: None,
                approval_policy: crate::conversation::ApprovalPolicy::OnRequest,
                sandbox: crate::conversation::Sandbox::ReadOnly,
            });
        let second_id = match exchange(
            &mut s,
            190,
            crate::protocol::ClientMsg::Command(crate::protocol::Command::ConversationStart {
                target,
                branch: None,
            }),
            None,
        )
        .await
        {
            ServerMsg::Reply(crate::protocol::Reply::ConversationStarted { id, .. }) => id,
            other => panic!("{other:?}"),
        };
        let (first_client, mut first_replies) = mpsc::channel(8);
        s.daemon.handle(Ev::Connected(191, first_client, false, crate::protocol::Surface::Terminal, true));
        let (second_client, mut second_replies) = mpsc::channel(8);
        s.daemon.handle(Ev::Connected(192, second_client, false, crate::protocol::Surface::Terminal, true));
        s.daemon
            .conversation_send(191, first, generation, 1, "Synthetic first".into(), Default::default())
            .unwrap();
        s.daemon
            .conversation_send(192, second_id, 1, 1, "Synthetic second".into(), Default::default())
            .unwrap();
        let mut saves = Vec::new();
        while saves.len() < 2 {
            let event = tokio::time::timeout(Duration::from_secs(20), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            if matches!(event, Ev::SessionSaved { .. }) {
                saves.push(event);
            } else {
                s.daemon.handle(event);
            }
        }
        saves.sort_by_key(|event| match event {
            Ev::SessionSaved { ticket, .. } => std::cmp::Reverse(*ticket),
            _ => unreachable!(),
        });
        for event in saves {
            s.daemon.handle(event);
        }
        assert!(matches!(
            drive_reply(&mut s, &mut first_replies).await,
            ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))
        ));
        assert!(matches!(
            drive_reply(&mut s, &mut second_replies).await,
            ServerMsg::Reply(crate::protocol::Reply::ConversationSent(_))
        ));
        assert_eq!(s.daemon.conversation_operations.len(), 2);
        s.daemon.dispose_conversation(second_id).unwrap();
        while s.daemon.conversation_runtimes.owns(second_id) {
            let event = tokio::time::timeout(Duration::from_secs(20), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            s.daemon.handle(event);
        }
    })
    .catch_unwind()
    .await;
    // Drop any second provider guard before deleting its executable after a failed assertion.
    let second_id = crate::conversation::ConversationId(2);
    if s.daemon.conversation_runtimes.owns(second_id)
        && s.daemon.dispose_conversation(second_id).is_ok()
    {
        while s.daemon.conversation_runtimes.owns(second_id) {
            let event = tokio::time::timeout(Duration::from_secs(20), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            s.daemon.handle(event);
        }
    }
    let pidfile = second.directory.path().join("pids");
    if tokio::task::spawn_blocking(move || pidfile.exists())
        .await
        .unwrap()
    {
        stopped(second.pids().await).await;
    }
    s.settle_background().await;
    second.cleanup().await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn fixture_cleanup_removes_the_compiled_provider() {
    Fixture::new().await.cleanup().await;
}

#[tokio::test]
async fn live_provider_and_fixture_are_removed_when_the_test_scope_unwinds() {
    let observation = Arc::new(Mutex::new(None));
    let observed = observation.clone();
    let failure = AssertUnwindSafe(async move {
        with_fixture(move |session| {
            Box::pin(async move {
                let TestSession {
                    fixture: f,
                    daemon: d,
                    events: rx,
                } = session;
                d.start_conversation(f.launch("normal")).unwrap();
                d.handle(Ev::Conversation(next(rx).await));
                d.handle(Ev::Conversation(next(rx).await));
                let pids = f.pids().await;
                assert_eq!(pids.len(), 2);
                let live = pids.clone();
                tokio::task::spawn_blocking(move || {
                    let mut system = sysinfo::System::new();
                    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                    for pid in live {
                        assert!(system.process(sysinfo::Pid::from_u32(pid)).is_some());
                    }
                })
                .await
                .unwrap();
                *observed.lock().unwrap() = Some((
                    f.directory.path().to_owned(),
                    f.record.binding.executable.clone(),
                    pids,
                ));
                panic!("synthetic live-provider scope failure");
            })
        })
        .await;
    })
    .catch_unwind()
    .await;
    let panic = failure.unwrap_err();
    assert_eq!(
        panic.downcast_ref::<&str>().copied(),
        Some("synthetic live-provider scope failure")
    );
    let (directory, executable, pids) = observation.lock().unwrap().take().unwrap();
    tokio::task::spawn_blocking(move || {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        for pid in pids {
            assert!(
                !system
                    .process(sysinfo::Pid::from_u32(pid))
                    .is_some_and(|process| process.status() != sysinfo::ProcessStatus::Zombie)
            );
        }
        let removed = !directory.exists() && !executable.exists();
        // A failed cleanup assertion must still remove known files after confirmed exit.
        if directory.exists() {
            std::fs::remove_dir_all(directory).unwrap();
        }
        assert!(
            removed,
            "live-provider unwind left the compiled fixture behind"
        );
    })
    .await
    .unwrap();
}
#[tokio::test]
async fn disconnect_preserves_daemon_owned_child_and_disposal_kills_startup_descendant() {
    with_fixture(move |session| {
        Box::pin(async move {
            let TestSession {
                fixture: f,
                daemon: d,
                events: rx,
            } = session;
            let (client, mut messages) = mpsc::channel::<ServerMsg>(8);
            d.handle(Ev::Connected(11, client, false, crate::protocol::Surface::Terminal, true));
            d.start_conversation(f.launch("normal")).unwrap();
            let ready = next(rx).await;
            assert!(matches!(ready.payload, Payload::Ready));
            let owner = ready.owner;
            d.handle(Ev::Conversation(ready));
            d.handle(Ev::Disconnected(11));
            assert!(messages.recv().await.is_none());
            let notification = next(rx).await;
            assert_eq!(notification.owner, owner);
            match &notification.payload {
                Payload::Notification { method, params } => {
                    assert_eq!(method, "test/ready");
                    assert_eq!(params["accountEnv"], "selected-account-effective-env");
                }
                _ => panic!("expected post-disconnect child event"),
            }
            d.handle(Ev::Conversation(notification));
            assert_eq!(d.conversations[0].state, RecoveryState::Running);
            let pids = f.pids().await;
            assert_eq!(pids.len(), 2);
            let observed = pids.clone();
            tokio::task::spawn_blocking(move || {
                let mut system = sysinfo::System::new();
                system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
                for pid in observed {
                    assert!(system.process(sysinfo::Pid::from_u32(pid)).is_some());
                }
            })
            .await
            .unwrap();
            d.dispose_conversation(f.record.id).unwrap();
            let terminated = next(rx).await;
            assert!(matches!(
                terminated.payload,
                Payload::Terminated {
                    failure: None,
                    reaped: true
                }
            ));
            d.handle(Ev::Conversation(terminated));
            assert_eq!(d.conversations[0].state, RecoveryState::Stopped);
            stopped(pids).await;
        })
    })
    .await;
}

#[tokio::test]
async fn failed_handshakes_are_visible_and_dispose_the_process_tree() {
    for (mode, expected) in [
        ("malformed", "Malformed Codex JSON frame"),
        ("oversize", "exceeds size limit"),
        ("closed", "Codex stdout closed"),
        ("timeout", "initialize timed out"),
        ("wrong-id", "does not match pending request"),
        ("wrong-home", "different configuration root"),
    ] {
        with_fixture(move |session| {
            Box::pin(async move {
                let TestSession {
                    fixture: f,
                    daemon: d,
                    events: rx,
                } = session;
                let (client, mut messages) = mpsc::channel(8);
                let (unrelated, mut unrelated_messages) = mpsc::channel(8);
                d.handle(Ev::Connected(2, unrelated, false, crate::protocol::Surface::Terminal, true));
                d.handle(Ev::Connected(1, client, true, crate::protocol::Surface::Terminal, true));
                d.start_conversation(f.launch(mode)).unwrap();
                let event = next(rx).await;
                match &event.payload {
                    Payload::Terminated {
                        failure: Some(failure),
                        reaped: true,
                    } => {
                        assert!(failure.contains(expected), "{mode}: {failure}");
                        assert!(!failure.contains("PRIVATE_SENTINEL"));
                    }
                    _ => panic!("{mode}: expected a cleaned-up failure"),
                }
                d.handle(Ev::Conversation(event));
                assert_eq!(d.conversations[0].state, RecoveryState::RecoveryRequired);
                tokio::time::timeout(Duration::from_secs(10), async {
                    while let Some(message) = messages.recv().await {
                        if matches!(message, ServerMsg::Error(_)) {
                            return;
                        }
                    }
                    panic!("attached startup observer disconnected before error");
                })
                .await
                .expect("failed startup is visible");
                assert!(
                    unrelated_messages.try_recv().is_err(),
                    "startup failure cannot abort an unrelated one-shot request"
                );
                stopped(f.pids().await).await;
            })
        })
        .await;
    }
}

#[tokio::test]
async fn unknown_response_fails_without_retrying_or_mutating_a_session() {
    with_fixture(move |session| Box::pin(async move {
    let TestSession { fixture: f, daemon: d, events: rx } = session;
    d.start_conversation(f.launch("unexpected-response"))
        .unwrap();
    d.handle(Ev::Conversation(next(rx).await));
    let event = next(rx).await;
    assert!(
        matches!(&event.payload, Payload::Terminated { failure: Some(f), reaped: true } if f.contains("no pending request"))
    );
    d.handle(Ev::Conversation(event));
    stopped(f.pids().await).await;
    })).await;
}

#[tokio::test]
async fn replaced_connection_and_generation_cannot_change_daemon_state() {
    with_fixture(move |session| {
        Box::pin(async move {
            let TestSession {
                fixture: f,
                daemon: d,
                events: rx,
            } = session;
            d.start_conversation(f.launch("normal")).unwrap();
            let ready = next(rx).await;
            let old = ready.owner;
            d.handle(Ev::Conversation(ready));
            let pending = next(rx).await;
            d.dispose_conversation(f.record.id).unwrap();
            assert!(d.start_conversation(f.launch("normal")).is_err());
            d.handle(Ev::Conversation(pending));
            d.handle(Ev::Conversation(next(rx).await));
            let mut replacement = f.launch("normal");
            if let super::transport::LaunchIdentity::Existing(record) = &mut replacement.identity {
                record.generation += 1;
                d.conversations[0] = (**record).clone();
            }
            d.start_conversation(replacement).unwrap();
            d.handle(Ev::Conversation(Event {
                owner: old,
                payload: Payload::Ready,
            }));
            assert_eq!(d.conversations[0].state, RecoveryState::Stopped);
            let ready = next(rx).await;
            let current = ready.owner;
            d.handle(Ev::Conversation(ready));
            let mut stale_generation = current;
            stale_generation.generation -= 1;
            let mut obsolete_connection = old;
            obsolete_connection.generation = current.generation;
            for owner in [old, stale_generation, obsolete_connection] {
                d.handle(Ev::Conversation(Event {
                    owner,
                    payload: Payload::Terminated {
                        failure: Some("stale".into()),
                        reaped: true,
                    },
                }));
                assert_eq!(d.conversations[0].state, RecoveryState::Running);
            }
            d.handle(Ev::Conversation(next(rx).await));
            d.dispose_conversation(f.record.id).unwrap();
            d.handle(Ev::Conversation(next(rx).await));
            stopped(f.pids().await).await;
        })
    })
    .await;
}

#[tokio::test]
async fn failed_startup_cleanup_keeps_ownership_until_explicit_disposal_succeeds() {
    with_fixture(move |session| Box::pin(async move {
    let TestSession { fixture: f, daemon: d, events: rx } = session;
    let mut launch = f.launch("normal");
    launch.failure_point = Some(super::transport::FailurePoint::PreflightDisposal);
    d.start_conversation(launch).unwrap();
    let failure = next(rx).await;
    assert!(
        matches!(&failure.payload, Payload::Terminated { failure: Some(error), reaped: false }
        if error.contains("injected provider disposal failure"))
    );
    d.handle(Ev::Conversation(failure));
    d.conversations[0].state = RecoveryState::Stopped;
    assert!(d.start_conversation(f.launch("normal")).is_err());
    d.dispose_conversation(f.record.id).unwrap();
    let disposed = next(rx).await;
    assert!(matches!(
        disposed.payload,
        Payload::Terminated {
            failure: None,
            reaped: true
        }
    ));
    d.handle(Ev::Conversation(disposed));
    d.start_conversation(f.launch("normal")).unwrap();
    d.handle(Ev::Conversation(next(rx).await));
    d.handle(Ev::Conversation(next(rx).await));
    d.dispose_conversation(f.record.id).unwrap();
    d.handle(Ev::Conversation(next(rx).await));
    stopped(f.pids().await).await;
    })).await;
}

#[tokio::test]
async fn failed_disposal_keeps_live_process_ownership_for_an_explicit_retry() {
    with_fixture(move |session| Box::pin(async move {
    let TestSession { fixture: f, daemon: d, events: rx } = session;
    let mut launch = f.launch("normal");
    launch.failure_point = Some(super::transport::FailurePoint::RuntimeDisposal);
    d.start_conversation(launch).unwrap();
    d.handle(Ev::Conversation(next(rx).await));
    d.handle(Ev::Conversation(next(rx).await));
    let pids = f.pids().await;
    d.dispose_conversation(f.record.id).unwrap();
    let failure = next(rx).await;
    assert!(
        matches!(&failure.payload, Payload::Terminated { failure: Some(error), reaped: false }
        if error.contains("injected provider disposal failure"))
    );
    let owner = failure.owner;
    d.handle(Ev::Conversation(failure));
    d.handle(Ev::Conversation(Event {
        owner,
        payload: Payload::Ready,
    }));
    assert_eq!(d.conversations[0].state, RecoveryState::RecoveryRequired);
    d.conversations[0].state = RecoveryState::Stopped;
    assert!(d.start_conversation(f.launch("normal")).is_err());
    let observed = pids.clone();
    tokio::task::spawn_blocking(move || {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        for pid in observed {
            assert!(system.process(sysinfo::Pid::from_u32(pid)).is_some());
        }
    })
    .await
    .unwrap();
    d.dispose_conversation(f.record.id).unwrap();
    let disposed = next(rx).await;
    assert!(matches!(
        disposed.payload,
        Payload::Terminated {
            failure: None,
            reaped: true
        }
    ));
    d.handle(Ev::Conversation(disposed));
    stopped(pids).await;
    })).await;
}

#[tokio::test]
async fn unsupported_bindings_refuse_before_provider_start() {
    let f = Fixture::new().await;
    let mut launch = f.launch("normal");
    if let super::transport::LaunchIdentity::Existing(c) = &mut launch.identity {
        c.binding.profile = Some("native-overlay".into());
    }
    assert!(
        launch
            .validate()
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("profile-v2")
    );
    let mut launch = f.launch("normal");
    launch.environment.push((
        "CODEX_HOME".into(),
        f.directory.path().join("different").into_os_string(),
    ));
    assert!(
        launch
            .validate()
            .await
            .err()
            .unwrap()
            .to_string()
            .contains("disagrees")
    );
    assert!(!f.directory.path().join("pids").exists());
    f.cleanup().await;
}

#[test]
fn envelope_ids_and_shapes_are_strict_without_exposing_payloads() {
    assert!(matches!(
        decode(r#"{"method":"configWarning","params":{},"emittedAtMs":1234}"#).unwrap(),
        Envelope::Notification { .. }
    ));
    for invalid in [
        r#"{"id":1,"method":"request","params":{},"emittedAtMs":1234}"#,
        r#"{"id":1,"result":{},"emittedAtMs":1234}"#,
        r#"{"method":"notification","params":{},"emittedAtMs":"invalid"}"#,
    ] {
        assert!(decode(invalid).is_err());
    }
    for invalid in [
        "[]",
        "{",
        r#"{"id":null,"result":0}"#,
        r#"{"id":1.5,"result":0}"#,
        r#"{"id":1,"id":2,"result":0}"#,
        r#"{"id":1,"result":0,"error":{}}"#,
        r#"{"id":1,"method":"x","result":0}"#,
        r#"{"method":"x","params":true}"#,
        r#"{"id":1,"error":{"message":"private","code":"invalid"}}"#,
    ] {
        assert!(decode(invalid).is_err(), "accepted {invalid}");
    }
    for (id, expected) in [
        (json!(42), RequestId::Integer(42)),
        (json!("42"), RequestId::Text("42".into())),
    ] {
        match decode(&json!({"id":id,"method":"request/approval","params":{}}).to_string()).unwrap()
        {
            Envelope::Request { id, .. } => assert_eq!(id, expected),
            _ => panic!("expected exact request id"),
        }
    }
}
#[tokio::test]
async fn history_projection_accepts_only_current_owned_events_and_drops_disconnected_content() {
    with_fixture(move |session| {
        Box::pin(async move {
            let TestSession { fixture: f, daemon: d, events: rx } = session;
            d.start_conversation(f.launch("normal")).unwrap();
            let ready = next(rx).await;
            let owner = ready.owner;
            d.handle(Ev::Conversation(ready));
            d.handle(Ev::Conversation(next(rx).await));
            d.conversation_history.attach(901, f.record.id);
            let params = json!({"threadId":f.record.binding.session_id,"turnId":"synthetic-turn","item":{"id":"synthetic-item","type":"agentMessage","text":"synthetic response"}});
            let mut stale = owner; stale.generation -= 1;
            d.handle(Ev::Conversation(Event { owner: stale, payload: Payload::Notification { method: "item/completed".into(), params: params.clone() } }));
            assert!(d.conversation_history.get(901).unwrap().entries.is_empty());
            for _ in 0..2 {
                d.handle(Ev::Conversation(Event { owner, payload: Payload::Notification { method: "item/completed".into(), params: params.clone() } }));
            }
            assert_eq!(d.conversation_history.get(901).unwrap().entries.len(), 1);
            d.handle(Ev::Disconnected(901));
            assert!(d.conversation_history.get(901).is_none());
            d.dispose_conversation(f.record.id).unwrap();
            d.handle(Ev::Conversation(next(rx).await));
            stopped(f.pids().await).await;
        })
    }).await;
}

/// The stand-in daemon for `kill_server_stops_every_provider_before_the_daemon_exits` is this
/// same test binary run again with this variable set to a scratch folder. `KillServer` ends in
/// `process::exit`, which would end the test runner, so the daemon has to be a process of its own.
#[cfg(unix)]
const STAND_IN_DAEMON_ENV: &str = concat!(env!("APP_ENV"), "_TEST_KILL_SERVER_STAND_IN");

/// Not a test unless the parent started it: a daemon with one live provider that is told to kill
/// the server.
#[cfg(unix)]
#[tokio::test]
async fn kill_server_stand_in_daemon() {
    let Ok(root) = std::env::var(STAND_IN_DAEMON_ENV) else {
        return;
    };
    let root = PathBuf::from(root);
    let fixture = Fixture::new().await;
    let (mut d, mut rx) = daemon(fixture.record.clone());
    let (client, _messages) = mpsc::channel::<ServerMsg>(8);
    d.handle(Ev::Connected(11, client, false, crate::protocol::Surface::Terminal, true));
    d.start_conversation(fixture.launch("normal")).unwrap();
    let ready = next(&mut rx).await;
    assert!(matches!(ready.payload, Payload::Ready));
    d.handle(Ev::Conversation(ready));
    let notification = next(&mut rx).await;
    d.handle(Ev::Conversation(notification));
    let pids = fixture.pids().await;
    assert_eq!(pids.len(), 2, "the provider and the child it started");
    let observed = pids.clone();
    tokio::task::spawn_blocking(move || {
        let mut system = sysinfo::System::new();
        system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
        for pid in observed {
            assert!(system.process(sysinfo::Pid::from_u32(pid)).is_some(), "{pid} runs");
        }
    })
    .await
    .unwrap();
    let listed: Vec<String> = pids.iter().map(u32::to_string).collect();
    std::fs::write(root.join("fixture"), fixture.directory.path().as_os_str().as_encoded_bytes()).unwrap();
    std::fs::write(root.join("pids"), listed.join(" ")).unwrap();
    d.handle(Ev::Msg(
        11,
        crate::protocol::ClientMsg::Command(crate::protocol::Command::KillServer { forget: true }),
    ));
    // The daemon ends the process itself; the loop keeps its runtime turning until it does.
    tokio::time::timeout(Duration::from_secs(30), async {
        loop {
            if let Some(event) = rx.recv().await {
                d.handle(event);
            }
        }
    })
    .await
    .expect_err("the daemon exits after kill-server");
    panic!("kill-server did not end the daemon");
}

#[cfg(unix)]
#[tokio::test]
async fn kill_server_stops_every_provider_before_the_daemon_exits() {
    if std::env::var(STAND_IN_DAEMON_ENV).is_ok() {
        return;
    }
    let scratch = crate::daemon::persist::scratch_dir();
    let root = scratch.path().to_owned();
    let data = root.join("data");
    std::fs::create_dir_all(&data).unwrap();
    let mut daemon = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "daemon::conversation::tests::kill_server_stand_in_daemon",
            "--nocapture",
        ])
        .env(STAND_IN_DAEMON_ENV, &root)
        .env(concat!(env!("APP_ENV"), "_DATA_DIR"), &data)
        .env(concat!(env!("APP_ENV"), "_CONFIG"), root.join("config.toml"))
        .env(concat!(env!("APP_ENV"), "_SOCKET"), format!("killserver-{}", std::process::id()))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("start the stand-in daemon");
    let status = tokio::task::spawn_blocking(move || {
        let until = std::time::Instant::now() + Duration::from_secs(60);
        loop {
            if let Some(status) = daemon.try_wait().unwrap() {
                return Some(status);
            }
            if std::time::Instant::now() > until {
                let _ = daemon.kill();
                let _ = daemon.wait();
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    })
    .await
    .unwrap();
    let pids: Vec<u32> = std::fs::read_to_string(root.join("pids"))
        .expect("the stand-in daemon had a live provider before it was told to exit")
        .split_whitespace()
        .map(|pid| pid.parse().unwrap())
        .collect();
    let fixture_dir = std::fs::read_to_string(root.join("fixture")).ok().map(PathBuf::from);
    let outcome = AssertUnwindSafe(stopped(pids.clone())).catch_unwind().await;
    if outcome.is_err() {
        for pid in &pids {
            // SAFETY: stops processes this test started so a failure leaves nothing behind.
            unsafe { libc::kill(*pid as libc::pid_t, libc::SIGKILL) };
        }
    }
    if let Some(dir) = fixture_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
    assert!(
        status.is_some_and(|status| status.success()),
        "the stand-in daemon exits cleanly after kill-server: {status:?}"
    );
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

/// The stand-in daemon for the provider-group tests: this test binary run again, with this set to
/// a scratch folder. The daemon has to be a process of its own, since the test ends it.
#[cfg(unix)]
const GROUPS_STAND_IN_ENV: &str = concat!(env!("APP_ENV"), "_TEST_PROVIDER_GROUPS_STAND_IN");

/// Not a test unless the parent started it: a daemon that keeps the provider-groups record and has
/// one live provider, which it then either stops cleanly (`mode` says `clean`) or waits to be killed.
#[cfg(unix)]
#[tokio::test]
async fn provider_groups_stand_in_daemon() {
    let Ok(root) = std::env::var(GROUPS_STAND_IN_ENV) else {
        return;
    };
    let root = PathBuf::from(root);
    let record = root.join("providers");
    crate::daemon::provider_groups::enable(record.clone());
    let clean = std::fs::read_to_string(root.join("mode")).is_ok_and(|mode| mode == "clean");
    let fixture = Fixture::new().await;
    let (mut d, mut rx) = daemon(fixture.record.clone());
    d.start_conversation(fixture.launch("normal")).unwrap();
    let ready = next(&mut rx).await;
    assert!(matches!(ready.payload, Payload::Ready));
    d.handle(Ev::Conversation(ready));
    let notification = next(&mut rx).await;
    d.handle(Ev::Conversation(notification));
    let pids = fixture.pids().await;
    assert_eq!(pids.len(), 2, "the provider and the child it started");
    let listed: Vec<String> = pids.iter().map(u32::to_string).collect();
    let while_running = crate::daemon::provider_groups::Groups::at(record.clone()).entries().len();
    std::fs::write(root.join("fixture"), fixture.directory.path().as_os_str().as_encoded_bytes()).unwrap();
    std::fs::write(root.join("entries_while_running"), while_running.to_string()).unwrap();
    std::fs::write(root.join("pids"), listed.join(" ")).unwrap();
    if clean {
        d.dispose_conversation(fixture.record.id).unwrap();
        let terminated = next(&mut rx).await;
        assert!(matches!(terminated.payload, Payload::Terminated { reaped: true, .. }));
        d.handle(Ev::Conversation(terminated));
        stopped(pids).await;
        return;
    }
    // Wait to be killed; the parent ends this process without letting it clean up.
    tokio::time::sleep(Duration::from_secs(120)).await;
    panic!("the stand-in daemon was never killed");
}

/// Runs the stand-in daemon in `root` and waits for its provider to be up.
#[cfg(unix)]
fn start_groups_stand_in(root: &Path, mode: &str) -> (std::process::Child, Vec<u32>) {
    std::fs::create_dir_all(root).unwrap();
    std::fs::write(root.join("mode"), mode).unwrap();
    let child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "daemon::conversation::tests::provider_groups_stand_in_daemon",
            "--nocapture",
        ])
        .env(GROUPS_STAND_IN_ENV, root)
        .env(concat!(env!("APP_ENV"), "_DATA_DIR"), root.join("data"))
        .env(concat!(env!("APP_ENV"), "_CONFIG"), root.join("config.toml"))
        .env(concat!(env!("APP_ENV"), "_SOCKET"), format!("groups-{}-{mode}", std::process::id()))
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .expect("start the stand-in daemon");
    let until = std::time::Instant::now() + Duration::from_secs(60);
    while !root.join("pids").exists() && std::time::Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
    }
    let pids = std::fs::read_to_string(root.join("pids"))
        .expect("the stand-in daemon had a live provider")
        .split_whitespace()
        .map(|pid| pid.parse().unwrap())
        .collect();
    (child, pids)
}

#[cfg(unix)]
fn running(pids: &[u32]) -> bool {
    let mut system = sysinfo::System::new();
    system.refresh_processes(sysinfo::ProcessesToUpdate::All, true);
    pids.iter().any(|pid| {
        system
            .process(sysinfo::Pid::from_u32(*pid))
            .is_some_and(|p| p.status() != sysinfo::ProcessStatus::Zombie)
    })
}

#[cfg(unix)]
#[tokio::test]
async fn a_provider_a_killed_daemon_left_is_stopped_by_the_next_start() {
    if std::env::var(GROUPS_STAND_IN_ENV).is_ok() {
        return;
    }
    let scratch = crate::daemon::persist::scratch_dir();
    let root = scratch.path().to_owned();
    let (mut daemon, pids) = start_groups_stand_in(&root, "kill");
    let record = crate::daemon::provider_groups::Groups::at(root.join("providers"));
    let recorded = record.entries().len();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    // Nothing in the dead daemon stopped them: this is what the sweep is for.
    let left_running = running(&pids);

    let swept = tokio::task::spawn_blocking({
        let root = root.clone();
        move || crate::daemon::provider_groups::Groups::at(root.join("providers")).sweep()
    })
    .await
    .unwrap();

    let outcome = AssertUnwindSafe(stopped(pids.clone())).catch_unwind().await;
    if outcome.is_err() {
        for pid in &pids {
            // SAFETY: stops processes this test started so a failure leaves nothing behind.
            unsafe { libc::kill(*pid as libc::pid_t, libc::SIGKILL) };
        }
    }
    let fixture_dir = std::fs::read_to_string(root.join("fixture")).ok().map(PathBuf::from);
    if let Some(dir) = fixture_dir {
        let _ = std::fs::remove_dir_all(dir);
    }
    assert_eq!(recorded, 1, "the running daemon recorded the provider's group");
    assert!(left_running, "a killed daemon leaves its provider tree running");
    assert!(
        matches!(swept.as_slice(), [crate::daemon::provider_groups::Swept::Killed { processes, .. }] if *processes >= 1),
        "{swept:?}"
    );
    assert!(record.entries().is_empty(), "the record is empty after the sweep");
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[cfg(unix)]
#[tokio::test]
async fn a_provider_stopped_cleanly_leaves_no_entry_in_the_record() {
    if std::env::var(GROUPS_STAND_IN_ENV).is_ok() {
        return;
    }
    let scratch = crate::daemon::persist::scratch_dir();
    let root = scratch.path().to_owned();
    let (mut daemon, pids) = start_groups_stand_in(&root, "clean");
    let status = tokio::task::spawn_blocking(move || daemon.wait().unwrap()).await.unwrap();
    let while_running = std::fs::read_to_string(root.join("entries_while_running")).unwrap();
    let left = crate::daemon::provider_groups::Groups::at(root.join("providers")).entries();
    if let Ok(dir) = std::fs::read_to_string(root.join("fixture")) {
        let _ = std::fs::remove_dir_all(dir);
    }
    assert!(status.success(), "the stand-in daemon stopped its provider and exited: {status:?}");
    assert_eq!(while_running, "1", "the group was recorded while the provider ran");
    assert!(left.is_empty(), "a clean stop strikes the group: {left:?}");
    assert!(!running(&pids));
}

mod queue;
mod branch_chat;
mod claude_chat;
mod turn_snapshot;

#[tokio::test]
async fn the_daemon_resumes_a_stopped_conversation_on_the_same_id_and_the_next_generation() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = crate::conversation::ConversationId(77);
        let mut record = s.fixture.record.clone();
        record.id = id;
        record.generation = 4;
        record.execution = crate::conversation::ExecutionKind::Structured;
        record.state = crate::conversation::RecoveryState::RecoveryRequired;
        record.settings = Some(settings(&s));
        s.daemon.conversations.push(record);
        s.daemon.cfg.restore.enabled = true;

        s.daemon
            .conversation_start(
                super::commands::DAEMON_REQUESTER,
                crate::conversation::StartTarget::Conversation { id, generation: 4 },
            )
            .unwrap();
        loop {
            let event = tokio::time::timeout(Duration::from_secs(10), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            s.daemon.handle(event);
            if s.daemon.conversation_runtimes.owns(id) {
                break;
            }
        }
        let resumed = s.daemon.conversations.iter().find(|c| c.id == id).unwrap();
        assert_eq!(resumed.generation, 5, "one generation on, same conversation");
        assert_eq!(s.daemon.conversations.len(), 1, "no new conversation appeared");
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn the_daemon_may_resume_but_never_create_or_adopt_a_conversation() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        s.daemon.cfg.restore.enabled = true;
        let create = crate::conversation::StartTarget::Create(settings(&s));
        let error = s
            .daemon
            .conversation_start(super::commands::DAEMON_REQUESTER, create)
            .unwrap_err();
        assert!(error.to_string().contains("only resume"), "{error}");
        let adopt = crate::conversation::StartTarget::Session {
            settings: settings(&s),
            thread_id: "thread".into(),
        };
        assert!(s
            .daemon
            .conversation_start(super::commands::DAEMON_REQUESTER, adopt)
            .is_err());
        assert!(s.daemon.conversations.is_empty());
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

fn running_record(s: &TestSession, id: u64) -> crate::conversation::Conversation {
    let mut record = s.fixture.record.clone();
    record.id = crate::conversation::ConversationId(id);
    record.generation = 3;
    record.execution = crate::conversation::ExecutionKind::Structured;
    record.state = crate::conversation::RecoveryState::Running;
    record.settings = Some(settings(s));
    record
}

#[tokio::test]
async fn a_restart_resumes_the_conversation_that_was_running_and_only_once() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = crate::conversation::ConversationId(61);
        s.daemon.cfg.restore.enabled = true;
        let record = running_record(&s, 61);
        assert!(s.daemon.restore(crate::daemon::persist::Saved {
            conversations: vec![record],
            ..Default::default()
        }));
        assert_eq!(
            s.daemon.conversations[0].state,
            crate::conversation::RecoveryState::RecoveryRequired
        );

        s.daemon.resume_conversations();
        assert!(s.daemon.resumable_conversations.is_empty(), "the list is spent");
        loop {
            let event = tokio::time::timeout(Duration::from_secs(10), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            s.daemon.handle(event);
            if s.daemon.conversation_runtimes.owns(id) {
                break;
            }
        }
        assert_eq!(s.daemon.conversations.len(), 1);
        assert_eq!(s.daemon.conversations[0].generation, 5, "fenced once, resumed once");
        s.daemon.resume_conversations();
        assert_eq!(s.daemon.conversations[0].generation, 5, "a second call does nothing");
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn with_the_setting_off_a_running_conversation_waits_for_a_person() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        s.daemon.cfg.restore.enabled = true;
        s.daemon.cfg.restore.conversations = false;
        let record = running_record(&s, 62);
        assert!(s.daemon.restore(crate::daemon::persist::Saved {
            conversations: vec![record],
            ..Default::default()
        }));
        s.daemon.resume_conversations();
        assert!(!s.daemon.conversation_runtimes.owns(crate::conversation::ConversationId(62)));
        assert_eq!(
            s.daemon.conversations[0].state,
            crate::conversation::RecoveryState::RecoveryRequired
        );
        assert_eq!(s.daemon.conversations[0].generation, 4, "fenced, not resumed");
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_resume_that_fails_is_reported_and_not_tried_again() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        let id = crate::conversation::ConversationId(63);
        s.daemon.cfg.restore.enabled = true;
        let mut record = running_record(&s, 63);
        let broken = s.fixture.directory.path().join("not-a-provider");
        std::fs::write(&broken, "not a program").unwrap();
        record.binding.executable = broken.clone();
        record.settings.as_mut().unwrap().executable = broken;
        assert!(s.daemon.restore(crate::daemon::persist::Saved {
            conversations: vec![record],
            ..Default::default()
        }));
        s.daemon.resume_conversations();
        loop {
            let event = tokio::time::timeout(Duration::from_secs(10), s.events.recv())
                .await
                .unwrap()
                .unwrap();
            s.daemon.handle(event);
            if s.daemon.conversation_states.get(&id).is_some_and(|state| state.error.is_some()) {
                break;
            }
        }
        assert!(!s.daemon.conversation_runtimes.owns(id));
        let generation = s.daemon.conversations[0].generation;
        s.daemon.resume_conversations();
        assert_eq!(s.daemon.conversations[0].generation, generation, "no second attempt");
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}

#[tokio::test]
async fn a_running_claude_conversation_is_taken_up_for_resume_too() {
    let mut s = conversation_fixture("conversation").await;
    let outcome = AssertUnwindSafe(async {
        s.daemon.cfg.restore.enabled = true;
        let mut record = running_record(&s, 64);
        record.binding.provider = crate::conversation::Provider::Claude;
        record.settings.as_mut().unwrap().provider = crate::conversation::Provider::Claude;
        assert!(s.daemon.restore(crate::daemon::persist::Saved {
            conversations: vec![record],
            ..Default::default()
        }));
        assert_eq!(s.daemon.resumable_conversations.len(), 1);
        s.daemon.resume_conversations();
        assert_eq!(
            s.daemon.conversations[0].generation, 5,
            "the start was admitted for the same conversation, one generation on"
        );
    })
    .catch_unwind()
    .await;
    s.cleanup().await;
    if let Err(panic) = outcome {
        std::panic::resume_unwind(panic);
    }
}
