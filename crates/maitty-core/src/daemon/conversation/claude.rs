use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::BTreeMap;
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

use maitty_provider_claude::{
    MAX_FRAME_BYTES, MAX_PENDING_TOOLS, OutputFrame, ProjectedEvent, TurnProjector,
    command_arguments, parse_output_frame, session_id, verified_version,
    version_number,
};

const STDERR_CHUNK_BYTES: usize = 8192;
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
const INITIALIZE_TIMEOUT: Duration = Duration::from_secs(30);
const INITIALIZE_REQUEST_ID: &str = "claude-initialize";

struct Client {
    process: super::process::OwnedChild,
    input: FramedWrite<tokio::process::ChildStdin, LinesCodec>,
    output: FramedRead<tokio::process::ChildStdout, LinesCodec>,
    stderr_task: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
    input_closed: bool,
}

impl Client {
    async fn spawn(
        executable: &Path,
        cwd: &Path,
        config_root: &Path,
        environment: &[(OsString, OsString)],
        arguments: &[OsString],
    ) -> std::result::Result<Self, super::transport::StartupFailure> {
        let mut command = Command::new(executable);
        command
            .env_clear()
            .envs(environment.iter().filter_map(|(key, value)| {
                let name = key.to_string_lossy();
                let reserved = [
                    concat!(env!("APP_ENV"), "_TERM_ID"),
                    concat!(env!("APP_ENV"), "_PANE_TOKEN"),
                    "CODEX_HOME",
                ];
                (!crate::ipc::STRUCTURED_ENV
                    .iter()
                    .any(|entry| name.eq_ignore_ascii_case(entry))
                    && !reserved
                        .iter()
                        .any(|entry| name.eq_ignore_ascii_case(entry)))
                .then_some((key, value))
            }))
            .env("CLAUDE_CONFIG_DIR", config_root)
            .env_remove("CLAUDECODE")
            .args(arguments)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::proc::quiet(&mut command);
        let mut process = super::process::OwnedChild::spawn(command)
            .await
            .map_err(|failure| {
                super::transport::StartupFailure::spawn(failure, "Launch Claude Code CLI")
            })?;
        let (Some(stdin), Some(stdout), Some(stderr)) = (
            process.child.stdin.take(),
            process.child.stdout.take(),
            process.child.stderr.take(),
        ) else {
            return Err(super::transport::StartupFailure::dispose(
                process,
                anyhow::anyhow!("Claude Code CLI pipes are unavailable"),
            )
            .await);
        };
        let stderr_task = tokio::spawn(discard_stderr(stderr));
        Ok(Self {
            process,
            input: FramedWrite::new(stdin, LinesCodec::new_with_max_length(MAX_FRAME_BYTES)),
            output: FramedRead::new(stdout, LinesCodec::new_with_max_length(MAX_FRAME_BYTES)),
            stderr_task: Some(stderr_task),
            input_closed: false,
        })
    }

    async fn write(&mut self, value: &Value) -> Result<()> {
        self.input
            .send(serde_json::to_string(value)?)
            .await
            .context("Write Claude Code stream input")
    }

    async fn next(&mut self) -> Result<OutputFrame> {
        let line = self
            .output
            .next()
            .await
            .context("Claude Code stdout closed before a complete frame")?
            .context("Read Claude Code stream frame")
            .map_err(|_| anyhow::anyhow!("Claude Code stream frame is invalid or too large"))?;
        parse_output_frame(&line)
    }

    async fn initialize(&mut self, expected_session: &str) -> Result<()> {
        self.write(&serde_json::json!({
            "type":"control_request",
            "request_id":INITIALIZE_REQUEST_ID,
            "request":{"subtype":"initialize","hooks":null}
        }))
        .await?;
        tokio::time::timeout(INITIALIZE_TIMEOUT, async {
            loop {
                match self.next().await? {
                    OutputFrame::Initialized { session_id } => {
                        anyhow::ensure!(
                            session_id == expected_session,
                            "Claude initialized a different session than the requested identity"
                        );
                    }
                    OutputFrame::ControlResponse { response } => {
                        anyhow::ensure!(
                            response.get("request_id").and_then(Value::as_str)
                                == Some(INITIALIZE_REQUEST_ID)
                                && response.get("subtype").and_then(Value::as_str) != Some("error"),
                            "Claude Code rejected adapter initialization"
                        );
                        return Ok(());
                    }
                    OutputFrame::Other => {}
                    _ => {
                        bail!("Claude Code sent an unexpected frame during adapter initialization")
                    }
                }
            }
        })
        .await
        .context("Claude Code initialization timed out")?
    }

    async fn stop(&mut self) -> Result<()> {
        if !self.input_closed {
            SinkExt::<String>::close(&mut self.input)
                .await
                .context("Close Claude Code input")?;
            self.input_closed = true;
        }
        let _ = tokio::time::timeout(PROCESS_EXIT_TIMEOUT, self.process.child.wait()).await;
        self.process
            .stop()
            .await
            .context("Dispose Claude Code process tree")?;
        if let Some(stderr_task) = self.stderr_task.take() {
            stderr_task
                .await
                .context("Join Claude Code stderr reader")?
                .context("Read Claude Code stderr")?;
        }
        Ok(())
    }
}

async fn discard_stderr(mut stderr: impl AsyncRead + Unpin) -> std::io::Result<()> {
    let mut chunk = [0; STDERR_CHUNK_BYTES];
    while stderr.read(&mut chunk).await? != 0 {}
    Ok(())
}

pub(super) async fn run(
    launch: super::transport::Launch,
    owner: super::runtime::Owner,
    events: tokio::sync::mpsc::Sender<crate::daemon::Ev>,
    mut stop: tokio::sync::mpsc::Receiver<()>,
    mut requests: tokio::sync::mpsc::Receiver<super::runtime::Command>,
) {
    let Some(settings) = launch.settings().cloned() else {
        let _ = super::runtime::emit(
            &events,
            owner,
            super::runtime::Payload::Terminated {
                failure: Some("Claude launch settings are missing".into()),
                reaped: true,
            },
        )
        .await;
        return;
    };
    let (session, mut awaiting_first_message) = match &launch.identity {
        super::transport::LaunchIdentity::Pending(pending) => (
            pending.thread_id.clone().map_or_else(session_id, Ok),
            pending.thread_id.is_none(),
        ),
        super::transport::LaunchIdentity::Existing(conversation) => (
            Ok(conversation.binding.session_id.clone()),
            conversation.awaiting_first_user_message,
        ),
    };
    let session = match session {
        Ok(session) => session,
        Err(error) => {
            let _ = super::runtime::emit(
                &events,
                owner,
                super::runtime::Payload::Terminated {
                    failure: Some(format!("{error:#}")),
                    reaped: true,
                },
            )
            .await;
            return;
        }
    };
    if let Err(error) = verify_cli_version(
        launch.paths().1,
        launch.paths().2,
        launch.paths().0,
        &launch.environment,
    )
    .await
    {
        let _ = super::runtime::emit(
            &events,
            owner,
            super::runtime::Payload::Terminated {
                failure: Some(format!("{error:#}")),
                reaped: true,
            },
        )
        .await;
        return;
    }
    if super::runtime::emit(&events, owner, super::runtime::Payload::Ready)
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            biased;
            _ = stop.recv() => {
                let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Terminated { failure: None, reaped: true }).await;
                return;
            }
            command = requests.recv() => {
                let Some(command) = command else {
                    let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Terminated { failure: Some("Claude command channel closed".into()), reaped: true }).await;
                    return;
                };
                match command.request {
                    super::runtime::Rpc::Start { .. } | super::runtime::Rpc::Resume { .. } => {
                        let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Response {
                            token: command.token,
                            result: Ok(serde_json::json!({"thread":{"id":session}})),
                        }).await;
                    }
                    super::runtime::Rpc::Turns { .. } => {
                        let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Response {
                            token: command.token,
                            result: Ok(serde_json::json!({"data":[],"nextCursor":null})),
                        }).await;
                    }
                    super::runtime::Rpc::Send { params } => {
                        match run_turn(
                            TurnContext {
                                launch: &launch,
                                settings: &settings,
                                session: &session,
                                awaiting_first_message,
                                owner,
                                events: &events,
                                requests: &mut requests,
                                stop: &mut stop,
                            },
                            command.token,
                            params,
                        ).await {
                            Ok(mut client) => {
                                awaiting_first_message = false;
                                if let Err(error) = client.stop().await {
                                    let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Terminated { failure: Some(format!("Claude process cleanup failed: {error:#}")), reaped: false }).await;
                                    return;
                                }
                            }
                            Err(TurnFailure::BeforeStart { token, error }) => {
                                let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Response {
                                    token,
                                    result: Err(super::transport::RpcError::adapter_failure(-32000)),
                                }).await;
                                tracing::warn!(conversation = owner.conversation.0, error = %error, "Claude turn was rejected before launch");
                            }
                            Err(TurnFailure::Startup { token, failure }) => {
                                let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Response {
                                    token,
                                    result: Err(super::transport::RpcError::adapter_failure(-32000)),
                                }).await;
                                tracing::warn!(conversation = owner.conversation.0, error = %failure.error, "Claude CLI failed to start");
                                super::runtime::retain_failed_startup(failure, owner, &events, &mut stop).await;
                                return;
                            }
                            Err(TurnFailure::Active { token, error }) => {
                                if let Some(token) = token {
                                    let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Response {
                                        token,
                                        result: Err(super::transport::RpcError::adapter_failure(-32000)),
                                    }).await;
                                }
                                let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Terminated { failure: Some(format!("{error:#}")), reaped: true }).await;
                                return;
                            }
                        }
                    }
                    super::runtime::Rpc::Answer { .. } | super::runtime::Rpc::Interrupt { .. } => {
                        let _ = super::runtime::emit(&events, owner, super::runtime::Payload::Response {
                            token: command.token,
                            result: Err(super::transport::RpcError::adapter_failure(-32001)),
                        }).await;
                    }
                }
            }
        }
    }
}

async fn verify_cli_version(
    executable: &Path,
    cwd: &Path,
    config_root: &Path,
    environment: &[(OsString, OsString)],
) -> Result<()> {
    let mut command = tokio::process::Command::new(executable);
    command
        .arg("--version")
        .env_clear()
        .envs(
            environment
                .iter()
                .filter(|(key, _)| {
                    !crate::ipc::STRUCTURED_ENV
                        .iter()
                        .any(|name| key.to_string_lossy().eq_ignore_ascii_case(name))
                        && !["CODEX_HOME", "CLAUDECODE"]
                            .iter()
                            .any(|name| key.to_string_lossy().eq_ignore_ascii_case(name))
                })
                .map(|(key, value)| (key.clone(), value.clone())),
        )
        .env("CLAUDE_CONFIG_DIR", config_root)
        .current_dir(cwd)
        .kill_on_drop(true);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command
            .as_std_mut()
            .creation_flags(crate::proc::CREATE_NO_WINDOW);
    }
    let output = tokio::time::timeout(PROCESS_EXIT_TIMEOUT * 5, command.output())
        .await
        .context("Claude Code version check timed out")?
        .context("Read Claude Code version")?;
    anyhow::ensure!(output.status.success(), "Claude Code version check failed");
    let output = String::from_utf8_lossy(&output.stdout);
    let version = version_number(&output).context("Unrecognized Claude Code version output")?;
    anyhow::ensure!(
        verified_version(version),
        "Claude Code version {version} has not been verified for structured chat"
    );
    Ok(())
}

enum TurnFailure {
    BeforeStart {
        token: u64,
        error: anyhow::Error,
    },
    Startup {
        token: u64,
        failure: super::transport::StartupFailure,
    },
    Active {
        token: Option<u64>,
        error: anyhow::Error,
    },
}

struct TurnContext<'a> {
    launch: &'a super::transport::Launch,
    settings: &'a crate::conversation::LaunchSettings,
    session: &'a str,
    awaiting_first_message: bool,
    owner: super::runtime::Owner,
    events: &'a tokio::sync::mpsc::Sender<crate::daemon::Ev>,
    requests: &'a mut tokio::sync::mpsc::Receiver<super::runtime::Command>,
    stop: &'a mut tokio::sync::mpsc::Receiver<()>,
}

async fn run_turn(
    context: TurnContext<'_>,
    token: u64,
    params: Value,
) -> std::result::Result<Client, TurnFailure> {
    let TurnContext {
        launch,
        settings,
        session,
        awaiting_first_message,
        owner,
        events,
        requests,
        stop,
    } = context;
    if params.get("threadId").and_then(Value::as_str) != Some(session) {
        return Err(TurnFailure::BeforeStart {
            token,
            error: anyhow::anyhow!("Claude send targets a different session identity"),
        });
    }
    let text = params
        .get("input")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("text"))
        .and_then(|item| item.get("text").and_then(Value::as_str))
        .filter(|text| !text.is_empty())
        .context("Claude send has no text input")
        .map_err(|error| TurnFailure::BeforeStart { token, error })?
        .to_owned();
    let (_, executable, cwd) = launch.paths();
    let args = command_arguments(session, !awaiting_first_message, settings);
    let mut client = Client::spawn(
        executable,
        cwd,
        &settings.config_root,
        &launch.environment,
        &args,
    )
    .await
    .map_err(|failure| TurnFailure::Startup { token, failure })?;
    if let Err(error) = client.initialize(session).await {
        return Err(TurnFailure::Active {
            token: Some(token),
            error,
        });
    }
    let turn_id = session_id().map_err(|error| TurnFailure::Active {
        token: Some(token),
        error,
    })?;
    let user_item_id = session_id().map_err(|error| TurnFailure::Active {
        token: Some(token),
        error,
    })?;
    let mut projector = TurnProjector::new(
        session.to_owned(),
        turn_id.clone(),
        text.clone(),
        user_item_id,
    );
    if let Err(error) = client
        .write(&serde_json::json!({
            "type":"user",
            "message":{"role":"user","content":[{"type":"text","text":text}]}
        }))
        .await
    {
        return Err(TurnFailure::Active {
            token: Some(token),
            error,
        });
    }
    let mut pending_token = Some(token);
    let mut approvals = BTreeMap::new();
    let mut interrupts = BTreeMap::new();
    loop {
        tokio::select! {
            biased;
            _ = stop.recv() => {
                return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Claude conversation stopped during a turn") });
            }
            command = requests.recv() => {
                let Some(command) = command else {
                    return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Claude command channel closed during a turn") });
                };
                match command.request {
                    super::runtime::Rpc::Answer { request_id, response } => {
                        let super::transport::RequestId::Text(request_id) = request_id else {
                            return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Claude approval ID is not textual") });
                        };
                        let Some(input) = approvals.remove(&request_id) else {
                            return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Claude approval is stale") });
                        };
                        let decision = response.get("decision").and_then(Value::as_str);
                        let (behavior, message) = match decision {
                            Some("accept") => ("allow", None),
                            Some("decline") => ("deny", Some("The user declined this action.")),
                            _ => return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Unsupported Claude approval decision") }),
                        };
                        let mut answer = serde_json::json!({"behavior":behavior,"updatedInput":input});
                        if let Some(message) = message { answer["message"] = Value::String(message.into()); }
                        client.write(&serde_json::json!({"type":"control_response","response":{"subtype":"success","request_id":request_id,"response":answer}}))
                            .await
                            .map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                    }
                    super::runtime::Rpc::Interrupt { params } => {
                        if params.get("threadId").and_then(Value::as_str) != Some(session)
                            || params.get("turnId").and_then(Value::as_str) != Some(turn_id.as_str()) {
                            return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Claude interruption targets a stale turn") });
                        }
                        let request_id = format!("interrupt-{}", command.token);
                        interrupts.insert(request_id.clone(), command.token);
                        client.write(&serde_json::json!({"type":"control_request","request_id":request_id,"request":{"subtype":"interrupt"}}))
                            .await
                            .map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                    }
                    super::runtime::Rpc::Send { .. } => {
                        super::runtime::emit(events, owner, super::runtime::Payload::Response {
                            token: command.token,
                            result: Err(super::transport::RpcError::adapter_failure(-32001)),
                        }).await.map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                    }
                    _ => return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Unsupported Claude command during an active turn") }),
                }
            }
            frame = client.next() => {
                let frame = frame.map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                if let OutputFrame::ControlRequest { request_id, request } = &frame {
                    if request.get("subtype").and_then(Value::as_str) != Some("can_use_tool") {
                        return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Unsupported Claude control request") });
                    }
                    let tool_name = request.get("tool_name").and_then(Value::as_str).unwrap_or_default();
                    let input = request.get("input").filter(|input| input.is_object()).cloned().unwrap_or_else(|| serde_json::json!({}));
                    if tool_name == "AskUserQuestion" {
                        client.write(&serde_json::json!({"type":"control_response","response":{"subtype":"success","request_id":request_id,"response":{"behavior":"deny","message":"Ask the user in your response; chat accepts free-text replies."}}}))
                            .await
                            .map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                        continue;
                    }
                    if approvals.len() >= MAX_PENDING_TOOLS || approvals.insert(request_id.clone(), input.clone()).is_some() {
                        return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Claude approval request queue overflow or ID reuse") });
                    }
                    let (method, params) = approval_request(session, &turn_id, request_id, request, tool_name, &input);
                    super::runtime::emit(events, owner, super::runtime::Payload::Request {
                        id: super::transport::RequestId::Text(request_id.clone()),
                        method: method.into(),
                        params,
                    }).await.map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                    continue;
                }
                if let OutputFrame::ControlResponse { response } = &frame
                    && let Some(request_id) = response.get("request_id").and_then(Value::as_str)
                {
                        if let Some(token) = interrupts.remove(request_id) {
                            super::runtime::emit(events, owner, super::runtime::Payload::Response {
                                token,
                                result: Ok(serde_json::json!({})),
                            }).await.map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                        }
                        continue;
                }
                if matches!(frame, OutputFrame::Result { .. }) && pending_token.is_some() {
                    return Err(TurnFailure::Active { token: pending_token, error: anyhow::anyhow!("Claude completed without echoing the submitted prompt; delivery is uncertain") });
                }
                let finished = matches!(frame, OutputFrame::Result { .. });
                let projected = projector.project(frame).map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                for event in projected {
                    match event {
                        ProjectedEvent::Notification { method, params } => {
                            super::runtime::emit(events, owner, super::runtime::Payload::Notification { method, params })
                                .await
                                .map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                        }
                        ProjectedEvent::Acknowledged { result } => {
                            let acknowledged = pending_token.take().context("Claude acknowledgement token was already consumed")
                                .map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                            super::runtime::emit(events, owner, super::runtime::Payload::Response {
                                token: acknowledged,
                                result: Ok(result),
                            }).await.map_err(|error| TurnFailure::Active { token: pending_token, error })?;
                        }
                        ProjectedEvent::ControlRequest { .. } => unreachable!("control requests are mapped before turn projection"),
                    }
                }
                if finished { return Ok(client); }
            }
        }
    }
}

/// Longest tool-use id accepted as a link to a tool step.
const MAX_TOOL_USE_ID_CHARS: usize = 256;

/// The provider request that asks the person to decide on one Claude `can_use_tool` call.
///
/// The request is linked to the tool's own history step by `itemId` when Claude names the tool
/// use (`tool_use_id`, the same id the step has); otherwise `itemId` falls back to the request id.
/// An edit tool is raised as a file change that carries its diff, any other tool as a command
/// whose text names its target.
fn approval_request(
    session: &str,
    turn_id: &str,
    request_id: &str,
    request: &Value,
    tool_name: &str,
    input: &Value,
) -> (&'static str, Value) {
    let item_id = request
        .get("tool_use_id")
        .and_then(Value::as_str)
        .filter(|id| {
            !id.is_empty()
                && id.chars().count() <= MAX_TOOL_USE_ID_CHARS
                && !id.chars().any(char::is_control)
        })
        .unwrap_or(request_id);
    let mut params = serde_json::json!({
        "threadId": session,
        "turnId": turn_id,
        "itemId": item_id,
        "command": super::tool_input::approval_command(tool_name, input),
        "availableDecisions": ["accept", "decline"],
    });
    match super::tool_input::file_change_previews(tool_name, input) {
        Some(changes) => {
            params["changes"] = changes.into();
            ("item/fileChange/requestApproval", params)
        }
        None => ("item/commandExecution/requestApproval", params),
    }
}

#[cfg(test)]
mod tests {
    use super::{approval_request, parse_output_frame, verified_version};
    use crate::conversation::{ApprovalPolicy, LaunchSettings, Provider, Sandbox};
    use maitty_provider_claude::{MAX_FRAME_BYTES, OutputFrame, command_arguments, session_id};
    use serde_json::json;
    use std::path::PathBuf;

    fn settings(approval_policy: ApprovalPolicy, sandbox: Sandbox) -> LaunchSettings {
        LaunchSettings {
            provider: Provider::Claude,
            executable: PathBuf::from("claude"),
            config_root: PathBuf::from("config"),
            cwd: PathBuf::from("workspace"),
            model: Some("sonnet".into()),
            model_provider: None,
            approval_policy,
            sandbox,
        }
    }

    #[test]
    fn recognizes_only_the_locally_verified_cli_version() {
        assert!(verified_version("2.1.312"));
        assert!(verified_version("2.1.314"));
        assert!(verified_version("2.1.316"));
        assert!(!verified_version("2.1.313"));
        assert!(!verified_version("2.1.31"));
    }

    #[test]
    fn creates_a_valid_unique_provider_session_id() {
        let first = session_id().unwrap();
        let second = session_id().unwrap();
        assert_eq!(first.len(), 36);
        assert_eq!(first.as_bytes()[14], b'4');
        assert!(matches!(first.as_bytes()[19], b'8'..=b'b'));
        assert_ne!(first, second);
    }

    #[test]
    fn maps_read_only_settings_to_claude_plan_mode() {
        let args = command_arguments(
            "session-1",
            false,
            &settings(ApprovalPolicy::OnRequest, Sandbox::ReadOnly),
        );
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--permission-mode", "plan"])
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--session-id", "session-1"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--model", "sonnet"]));
    }

    #[test]
    fn full_access_skips_claude_permission_checks_and_says_so() {
        let args = command_arguments(
            "session-1",
            false,
            &settings(ApprovalPolicy::Never, Sandbox::FullAccess),
        );
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--permission-mode", "bypassPermissions"])
        );
        assert!(args.iter().any(|arg| arg == "--allow-dangerously-skip-permissions"));
    }

    #[test]
    fn routes_claude_approvals_to_the_agent_sdk_stdio_channel() {
        let args = command_arguments(
            "session-1",
            false,
            &settings(ApprovalPolicy::OnRequest, Sandbox::WorkspaceWrite),
        );
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        assert!(
            args.iter()
                .any(|argument| argument == "--permission-prompt-tool=stdio")
        );
    }

    #[test]
    fn resume_uses_the_exact_existing_provider_session_id() {
        let args = command_arguments(
            "provider-session-exact",
            true,
            &settings(ApprovalPolicy::Never, Sandbox::WorkspaceWrite),
        );
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        assert!(
            args.iter()
                .any(|arg| arg == "--resume=provider-session-exact")
        );
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--permission-mode", "dontAsk"])
        );
        assert!(!args.iter().any(|arg| arg == "--session-id"));
    }

    #[test]
    fn reads_session_identity_from_initialization() {
        assert_eq!(
            parse_output_frame(r#"{"type":"system","subtype":"init","session_id":"session-1"}"#)
                .unwrap(),
            OutputFrame::Initialized {
                session_id: "session-1".into()
            }
        );
    }

    #[test]
    fn reads_streaming_text_and_content_block_identity() {
        assert_eq!(
            parse_output_frame(r#"{"type":"stream_event","event":{"type":"content_block_delta","index":2,"delta":{"type":"text_delta","text":"hello"}}}"#).unwrap(),
            OutputFrame::TextDelta { block_index: 2, text: "hello".into() }
        );
    }

    #[test]
    fn reads_message_identity_and_model_from_stream_start() {
        assert_eq!(
            parse_output_frame(r#"{"type":"stream_event","event":{"type":"message_start","message":{"id":"msg-1","model":"claude-sonnet"}}}"#).unwrap(),
            OutputFrame::MessageStarted {
                message_id: "msg-1".into(),
                model: "claude-sonnet".into()
            }
        );
    }

    #[test]
    fn reads_tool_input_deltas_and_content_block_completion() {
        assert_eq!(
            parse_output_frame(r#"{"type":"stream_event","event":{"type":"content_block_delta","index":3,"delta":{"type":"input_json_delta","partial_json":"{\"command\":"}}}"#).unwrap(),
            OutputFrame::ToolInputDelta {
                block_index: 3,
                partial_json: "{\"command\":".into()
            }
        );
        assert_eq!(
            parse_output_frame(
                r#"{"type":"stream_event","event":{"type":"content_block_stop","index":3}}"#
            )
            .unwrap(),
            OutputFrame::ContentBlockEnded { block_index: 3 }
        );
    }

    #[test]
    fn reads_tool_name_and_initial_input() {
        assert_eq!(
            parse_output_frame(r#"{"type":"stream_event","event":{"type":"content_block_start","index":1,"content_block":{"type":"tool_use","id":"tool-1","name":"Bash","input":{"command":"pwd"}}}}"#).unwrap(),
            OutputFrame::ToolUse {
                block_index: 1,
                id: "tool-1".into(),
                name: "Bash".into(),
                input: json!({"command":"pwd"})
            }
        );
    }

    #[test]
    fn ignores_nested_subagent_streams() {
        assert_eq!(
            parse_output_frame(r#"{"type":"stream_event","parent_tool_use_id":"tool-1","event":{"type":"content_block_delta","index":0,"delta":{"type":"text_delta","text":"private child output"}}}"#).unwrap(),
            OutputFrame::Other
        );
    }

    #[test]
    fn bounds_tool_output_and_preserves_error_state() {
        let frame = json!({
            "type": "user",
            "message": {"content": [{
                "type": "tool_result",
                "tool_use_id": "tool-1",
                "content": "x".repeat(9000),
                "is_error": true
            }]}
        });
        let OutputFrame::ToolResults(results) = parse_output_frame(&frame.to_string()).unwrap()
        else {
            panic!("tool result frame was not recognized");
        };
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, "tool-1");
        assert!(results[0].is_error);
        assert!(results[0].output.ends_with("[Output truncated]"));
        assert!(results[0].output.len() < 9000);
    }

    #[test]
    fn rejects_oversized_stream_frames() {
        let frame = format!(
            "{{\"type\":\"assistant\",\"padding\":\"{}\"}}",
            "x".repeat(MAX_FRAME_BYTES)
        );
        assert!(parse_output_frame(&frame).is_err());
    }

    #[test]
    fn preserves_control_request_body_for_provider_native_approval() {
        let request =
            json!({"subtype":"can_use_tool","tool_name":"Bash","input":{"command":"git status"}});
        let frame = json!({"type":"control_request","request_id":"request-8","request":request});
        assert_eq!(
            parse_output_frame(&frame.to_string()).unwrap(),
            OutputFrame::ControlRequest {
                request_id: "request-8".into(),
                request
            }
        );
    }

    #[test]
    fn approvals_name_their_target_and_link_to_the_tool_step() {
        let cases = [
            (
                "Bash",
                json!({"command":"cargo test"}),
                json!({"tool_use_id":"toolu_1"}),
                "item/commandExecution/requestApproval",
                "toolu_1",
                "cargo test",
                false,
            ),
            (
                "Edit",
                json!({"file_path":"src/a.rs","old_string":"x","new_string":"y"}),
                json!({"tool_use_id":"toolu_2"}),
                "item/fileChange/requestApproval",
                "toolu_2",
                "Edit: src/a.rs",
                true,
            ),
            (
                "Write",
                json!({"file_path":"b.txt","content":"hi"}),
                json!({}),
                "item/fileChange/requestApproval",
                "request-1",
                "Write: b.txt",
                true,
            ),
            (
                "WebFetch",
                json!({"url":"https://example.com"}),
                json!({"tool_use_id":""}),
                "item/commandExecution/requestApproval",
                "request-1",
                "WebFetch: https://example.com",
                false,
            ),
            (
                "Bash",
                json!({"command":"ls"}),
                json!({"tool_use_id":"bad\nid"}),
                "item/commandExecution/requestApproval",
                "request-1",
                "ls",
                false,
            ),
        ];
        for (tool, input, request, method, item_id, command, has_changes) in cases {
            let (got_method, params) =
                approval_request("session", "turn", "request-1", &request, tool, &input);
            assert_eq!(got_method, method, "{tool}");
            assert_eq!(params["itemId"], item_id, "{tool}");
            assert_eq!(params["command"], command, "{tool}");
            assert_eq!(params.get("changes").is_some(), has_changes, "{tool}");
            assert_eq!(params["turnId"], "turn");
        }
    }

    #[test]
    fn fails_closed_on_malformed_identity_and_unknown_frame_types() {
        assert!(
            parse_output_frame(r#"{"type":"result","session_id":"","is_error":false}"#).is_err()
        );
        assert!(parse_output_frame(r#"{"type":"unknown_future_event"}"#).is_err());
    }
}
