use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
use std::ffi::OsString;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

const MAX_FRAME_BYTES: usize = 1024 * 1024;
const VERIFIED_VERSIONS: &[&str] = &["2.1.312"];
const STDERR_CHUNK_BYTES: usize = 8192;
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(2);
const INITIALIZE_REQUEST_ID: &str = "claude-initialize";

#[derive(Debug, PartialEq)]
pub(super) enum OutputFrame {
    Initialized {
        session_id: String,
    },
    MessageStarted {
        message_id: String,
        model: String,
    },
    TextDelta {
        block_index: u64,
        text: String,
    },
    ToolInputDelta {
        block_index: u64,
        partial_json: String,
    },
    ContentBlockEnded {
        block_index: u64,
    },
    ControlRequest {
        request_id: String,
        request: Value,
    },
    ControlResponse {
        response: Value,
    },
    Result {
        session_id: String,
        is_error: bool,
        result: Option<String>,
    },
    UserReplay {
        text: String,
    },
    ToolUse {
        block_index: u64,
        id: String,
        name: String,
        input: Value,
    },
    ToolResults(Vec<ToolResult>),
    Other,
}

#[derive(Debug, PartialEq)]
pub(super) struct ToolResult {
    pub id: String,
    pub output: String,
    pub is_error: bool,
}

pub(super) fn parse_output_frame(line: &str) -> Result<OutputFrame> {
    anyhow::ensure!(
        line.len() <= MAX_FRAME_BYTES,
        "Claude output frame exceeds size limit"
    );
    let frame: Value =
        serde_json::from_str(line).context("Claude output frame is not valid JSON")?;
    let kind = frame
        .get("type")
        .and_then(Value::as_str)
        .context("Claude output frame has no type")?;

    match kind {
        "system" if frame.get("subtype").and_then(Value::as_str) == Some("init") => {
            let session_id = required_text(&frame, "session_id")?;
            Ok(OutputFrame::Initialized { session_id })
        }
        "stream_event" => parse_stream_event(&frame),
        "control_request" => Ok(OutputFrame::ControlRequest {
            request_id: required_text(&frame, "request_id")?,
            request: frame
                .get("request")
                .cloned()
                .context("Claude control request has no request body")?,
        }),
        "control_response" => Ok(OutputFrame::ControlResponse {
            response: frame
                .get("response")
                .cloned()
                .context("Claude control response has no response body")?,
        }),
        "result" => Ok(OutputFrame::Result {
            session_id: required_text(&frame, "session_id")?,
            is_error: frame
                .get("is_error")
                .and_then(Value::as_bool)
                .context("Claude result has no error status")?,
            result: frame
                .get("result")
                .and_then(Value::as_str)
                .map(str::to_owned),
        }),
        "user" => parse_user_frame(&frame),
        "assistant" | "system" | "rate_limit_event" | "tool_progress" => Ok(OutputFrame::Other),
        _ => bail!("Unsupported Claude output frame type: {kind}"),
    }
}

fn parse_stream_event(frame: &Value) -> Result<OutputFrame> {
    let event = frame
        .get("event")
        .context("Claude stream event has no event body")?;
    if frame
        .get("parent_tool_use_id")
        .and_then(Value::as_str)
        .is_some_and(|parent| !parent.is_empty())
    {
        return Ok(OutputFrame::Other);
    }
    if event.get("type").and_then(Value::as_str) == Some("message_start") {
        let message = event
            .get("message")
            .context("Claude message start has no message body")?;
        return Ok(OutputFrame::MessageStarted {
            message_id: required_text(message, "id")?,
            model: required_text(message, "model")?,
        });
    }
    if event.get("type").and_then(Value::as_str) == Some("content_block_start")
        && event
            .get("content_block")
            .and_then(|block| block.get("type"))
            .and_then(Value::as_str)
            == Some("tool_use")
    {
        let block = &event["content_block"];
        return Ok(OutputFrame::ToolUse {
            block_index: event
                .get("index")
                .and_then(Value::as_u64)
                .context("Claude tool block has no valid index")?,
            id: required_text(block, "id")?,
            name: required_text(block, "name")?,
            input: block
                .get("input")
                .cloned()
                .unwrap_or_else(|| serde_json::json!({})),
        });
    }
    if event.get("type").and_then(Value::as_str) == Some("content_block_stop") {
        return Ok(OutputFrame::ContentBlockEnded {
            block_index: event
                .get("index")
                .and_then(Value::as_u64)
                .context("Claude content block end has no valid index")?,
        });
    }
    if event.get("type").and_then(Value::as_str) != Some("content_block_delta") {
        return Ok(OutputFrame::Other);
    }
    let delta = event
        .get("delta")
        .context("Claude content delta has no delta body")?;
    let block_index = event
        .get("index")
        .and_then(Value::as_u64)
        .context("Claude text delta has no valid content block index")?;
    match delta.get("type").and_then(Value::as_str) {
        Some("text_delta") => {
            let text = delta
                .get("text")
                .and_then(Value::as_str)
                .context("Claude text delta has no text")?
                .to_owned();
            Ok(OutputFrame::TextDelta { block_index, text })
        }
        Some("input_json_delta") => {
            let partial_json = delta
                .get("partial_json")
                .and_then(Value::as_str)
                .context("Claude tool input delta has no partial JSON")?
                .to_owned();
            Ok(OutputFrame::ToolInputDelta {
                block_index,
                partial_json,
            })
        }
        _ => Ok(OutputFrame::Other),
    }
}

fn parse_user_frame(frame: &Value) -> Result<OutputFrame> {
    if frame
        .get("parent_tool_use_id")
        .and_then(Value::as_str)
        .is_some_and(|parent| !parent.is_empty())
    {
        return Ok(OutputFrame::Other);
    }
    let Some(content) = frame
        .get("message")
        .and_then(|message| message.get("content"))
        .and_then(Value::as_array)
    else {
        return Ok(OutputFrame::Other);
    };
    let tool_results: Vec<_> = content
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("tool_result"))
        .map(|block| {
            Ok(ToolResult {
                id: required_text(block, "tool_use_id")?,
                output: bounded_content_text(block.get("content")),
                is_error: block
                    .get("is_error")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect::<Result<_>>()?;
    if !tool_results.is_empty() {
        return Ok(OutputFrame::ToolResults(tool_results));
    }
    if frame.get("isReplay").and_then(Value::as_bool) != Some(true) {
        return Ok(OutputFrame::Other);
    }
    let text = content
        .iter()
        .filter(|block| block.get("type").and_then(Value::as_str) == Some("text"))
        .filter_map(|block| block.get("text").and_then(Value::as_str))
        .collect::<String>();
    Ok(OutputFrame::UserReplay { text })
}

fn bounded_content_text(content: Option<&Value>) -> String {
    const MAX_PREVIEW_BYTES: usize = 8192;
    let text = match content {
        Some(Value::String(text)) => text.clone(),
        Some(value) => value
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|block| block.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        None => String::new(),
    };
    let mut end = text.len().min(MAX_PREVIEW_BYTES);
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    let mut preview = text[..end].to_owned();
    if end < text.len() {
        preview.push_str("\n[Output truncated]");
    }
    preview
}

fn required_text(frame: &Value, field: &str) -> Result<String> {
    frame
        .get(field)
        .and_then(Value::as_str)
        .filter(|value| !value.is_empty() && !value.chars().any(char::is_control))
        .map(str::to_owned)
        .with_context(|| format!("Claude frame has no valid {field}"))
}

pub(super) fn verified_version(version: &str) -> bool {
    VERIFIED_VERSIONS.contains(&version)
}

pub(super) fn session_id() -> Result<String> {
    let mut bytes = [0_u8; 16];
    getrandom::fill(&mut bytes)
        .map_err(|error| anyhow::anyhow!("Create Claude session identity: {error}"))?;
    bytes[6] = (bytes[6] & 0x0f) | 0x40;
    bytes[8] = (bytes[8] & 0x3f) | 0x80;
    let hex = bytes.map(|byte| format!("{byte:02x}")).concat();
    Ok(format!(
        "{}-{}-{}-{}-{}",
        &hex[0..8],
        &hex[8..12],
        &hex[12..16],
        &hex[16..20],
        &hex[20..32]
    ))
}

pub(super) fn command_arguments(
    session_id: &str,
    resume: bool,
    settings: &crate::conversation::LaunchSettings,
) -> Vec<OsString> {
    let mut arguments = vec![
        "-p".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--include-partial-messages".into(),
        "--verbose".into(),
        "--permission-prompts".into(),
        "host".into(),
        "--replay-user-messages".into(),
    ];
    let permission_mode = match (settings.sandbox, settings.approval_policy) {
        (crate::conversation::Sandbox::ReadOnly, _) => "plan",
        (_, crate::conversation::ApprovalPolicy::Never) => "dontAsk",
        (
            _,
            crate::conversation::ApprovalPolicy::Untrusted
            | crate::conversation::ApprovalPolicy::OnRequest,
        ) => "default",
    };
    arguments.extend(["--permission-mode".into(), permission_mode.into()]);
    if !resume {
        arguments.extend(["--session-id".into(), session_id.into()]);
    } else {
        arguments.push(format!("--resume={session_id}").into());
    }
    if let Some(model) = settings.model.as_deref() {
        arguments.extend(["--model".into(), model.into()]);
    }
    arguments
}

#[derive(Debug, PartialEq)]
pub(super) enum ProjectedEvent {
    Notification { method: String, params: Value },
    Acknowledged { result: Value },
    ControlRequest { request_id: String, request: Value },
}

struct ProjectedTool {
    id: String,
    name: String,
    initial_input: Value,
    partial_input: String,
    command: String,
}

pub(super) struct TurnProjector {
    thread_id: String,
    turn_id: String,
    submitted_text: String,
    user_item_id: String,
    acknowledged: bool,
    active_message_id: Option<String>,
    assistant_text: BTreeMap<String, String>,
    started_messages: BTreeSet<String>,
    tools_by_block: BTreeMap<u64, ProjectedTool>,
    tools_by_id: BTreeMap<String, ProjectedTool>,
}

impl TurnProjector {
    pub(super) fn new(
        thread_id: String,
        turn_id: String,
        submitted_text: String,
        user_item_id: String,
    ) -> Self {
        Self {
            thread_id,
            turn_id,
            submitted_text,
            user_item_id,
            acknowledged: false,
            active_message_id: None,
            assistant_text: BTreeMap::new(),
            started_messages: BTreeSet::new(),
            tools_by_block: BTreeMap::new(),
            tools_by_id: BTreeMap::new(),
        }
    }

    #[cfg(test)]
    pub(super) fn acknowledged(&self) -> bool {
        self.acknowledged
    }

    pub(super) fn project(&mut self, frame: OutputFrame) -> Result<Vec<ProjectedEvent>> {
        let mut events = Vec::new();
        match frame {
            OutputFrame::UserReplay { text } if !self.acknowledged => {
                anyhow::ensure!(
                    text == self.submitted_text,
                    "Claude replayed a different user message"
                );
                events.push(self.notification(
                    "turn/started",
                    serde_json::json!({"turn":{"id":self.turn_id,"status":"inProgress"}}),
                ));
                let user = serde_json::json!({
                    "id": self.user_item_id,
                    "type": "userMessage",
                    "content": [{"type":"text","text":text}]
                });
                events.push(self.notification("item/started", serde_json::json!({"item":user})));
                events.push(self.notification("item/completed", serde_json::json!({"item":user})));
                events.push(ProjectedEvent::Acknowledged {
                    result: serde_json::json!({"turn":{"id":self.turn_id,"status":"inProgress","items":[]}}),
                });
                self.acknowledged = true;
            }
            OutputFrame::UserReplay { .. } => {}
            OutputFrame::MessageStarted { message_id, .. } => {
                self.active_message_id = Some(message_id);
            }
            OutputFrame::TextDelta { text, .. } => {
                let message_id = self
                    .active_message_id
                    .clone()
                    .context("Claude text arrived outside a message")?;
                if self.started_messages.insert(message_id.clone()) {
                    events.push(self.notification(
                        "item/started",
                        serde_json::json!({"item":{"id":message_id,"type":"agentMessage","text":""}}),
                    ));
                }
                self.assistant_text
                    .entry(message_id.clone())
                    .or_default()
                    .push_str(&text);
                events.push(self.notification(
                    "item/agentMessage/delta",
                    serde_json::json!({"itemId":message_id,"delta":text}),
                ));
            }
            OutputFrame::ToolUse {
                block_index,
                id,
                name,
                input,
            } => {
                if self.tools_by_block.len() >= MAX_PENDING_TOOLS {
                    bail!("Claude emitted too many unfinished tool calls");
                }
                self.tools_by_block.insert(
                    block_index,
                    ProjectedTool {
                        id,
                        name,
                        initial_input: input,
                        partial_input: String::new(),
                        command: String::new(),
                    },
                );
            }
            OutputFrame::ToolInputDelta {
                block_index,
                partial_json,
            } => {
                let tool = self
                    .tools_by_block
                    .get_mut(&block_index)
                    .context("Claude tool input delta has no matching tool call")?;
                let size = tool
                    .partial_input
                    .len()
                    .checked_add(partial_json.len())
                    .filter(|size| *size <= MAX_TOOL_INPUT_BYTES)
                    .context("Claude tool input exceeds size limit")?;
                tool.partial_input.reserve(size - tool.partial_input.len());
                tool.partial_input.push_str(&partial_json);
            }
            OutputFrame::ContentBlockEnded { block_index } => {
                if let Some(mut tool) = self.tools_by_block.remove(&block_index) {
                    let input = if tool.partial_input.is_empty() {
                        tool.initial_input.clone()
                    } else {
                        serde_json::from_str(&tool.partial_input)
                            .context("Claude tool input is invalid JSON")?
                    };
                    tool.command = input
                        .get("command")
                        .and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| {
                            format!("{} {}", tool.name, bounded_content_text(Some(&input)))
                        });
                    let item = serde_json::json!({
                        "id":tool.id,
                        "type":"commandExecution",
                        "command":tool.command,
                        "status":"inProgress",
                        "aggregatedOutput":""
                    });
                    events
                        .push(self.notification("item/started", serde_json::json!({"item":item})));
                    self.tools_by_id.insert(tool.id.clone(), tool);
                }
            }
            OutputFrame::ToolResults(results) => {
                for result in results {
                    let tool = self
                        .tools_by_id
                        .remove(&result.id)
                        .context("Claude returned a result for an unknown tool call")?;
                    let item = serde_json::json!({
                        "id":result.id,
                        "type":"commandExecution",
                        "command":tool.command,
                        "status":if result.is_error {"failed"} else {"completed"},
                        "aggregatedOutput":result.output
                    });
                    events.push(
                        self.notification("item/completed", serde_json::json!({"item":item})),
                    );
                }
            }
            OutputFrame::ControlRequest {
                request_id,
                request,
            } => {
                events.push(ProjectedEvent::ControlRequest {
                    request_id,
                    request,
                });
            }
            OutputFrame::Result {
                session_id,
                is_error,
                result,
            } => {
                anyhow::ensure!(
                    session_id == self.thread_id,
                    "Claude returned a different session identity"
                );
                for (message_id, streamed_text) in std::mem::take(&mut self.assistant_text) {
                    let text = result
                        .as_ref()
                        .filter(|text| !text.is_empty())
                        .unwrap_or(&streamed_text);
                    events.push(self.notification(
                        "item/completed",
                        serde_json::json!({"item":{"id":message_id,"type":"agentMessage","text":text}}),
                    ));
                }
                let status = if is_error { "failed" } else { "completed" };
                events.push(self.notification(
                    "turn/completed",
                    serde_json::json!({"turn":{"id":self.turn_id,"status":status}}),
                ));
            }
            OutputFrame::Initialized { .. }
            | OutputFrame::ControlResponse { .. }
            | OutputFrame::Other => {}
        }
        Ok(events)
    }

    fn notification(&self, method: &str, mut params: Value) -> ProjectedEvent {
        params["threadId"] = Value::String(self.thread_id.clone());
        if method != "turn/started" && method != "turn/completed" {
            params["turnId"] = Value::String(self.turn_id.clone());
        }
        ProjectedEvent::Notification {
            method: method.into(),
            params,
        }
    }
}

const MAX_PENDING_TOOLS: usize = 64;
const MAX_TOOL_INPUT_BYTES: usize = 64 * 1024;

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

    async fn initialize(&mut self) -> Result<()> {
        self.write(&serde_json::json!({
            "type":"control_request",
            "request_id":INITIALIZE_REQUEST_ID,
            "request":{"subtype":"initialize","hooks":null}
        }))
        .await?;
        let frame = tokio::time::timeout(PROCESS_EXIT_TIMEOUT * 5, self.next())
            .await
            .context("Claude Code initialization timed out")??;
        let OutputFrame::ControlResponse { response } = frame else {
            bail!("Claude Code did not respond to adapter initialization");
        };
        anyhow::ensure!(
            response.get("request_id").and_then(Value::as_str) == Some(INITIALIZE_REQUEST_ID)
                && response.get("subtype").and_then(Value::as_str) != Some("error"),
            "Claude Code rejected adapter initialization"
        );
        Ok(())
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
    let version = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    let version = version.strip_prefix("Claude Code ").unwrap_or(&version);
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
    if let Err(error) = client.initialize().await {
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
                    let command = input.get("command").and_then(Value::as_str)
                        .map(str::to_owned)
                        .unwrap_or_else(|| format!("{tool_name}: {}", bounded_content_text(Some(&input))));
                    super::runtime::emit(events, owner, super::runtime::Payload::Request {
                        id: super::transport::RequestId::Text(request_id.clone()),
                        method: "item/commandExecution/requestApproval".into(),
                        params: serde_json::json!({
                            "threadId":session,"turnId":turn_id,"itemId":request_id,
                            "command":command,"availableDecisions":["accept","decline"]
                        }),
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

#[cfg(test)]
mod tests {
    use super::{
        OutputFrame, ProjectedEvent, TurnProjector, command_arguments, parse_output_frame,
        session_id, verified_version,
    };
    use crate::conversation::{ApprovalPolicy, LaunchSettings, Provider, Sandbox};
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
    fn routes_claude_approvals_to_the_documented_host_channel() {
        let args = command_arguments(
            "session-1",
            false,
            &settings(ApprovalPolicy::OnRequest, Sandbox::WorkspaceWrite),
        );
        let args: Vec<_> = args.iter().map(|arg| arg.to_string_lossy()).collect();
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--permission-prompts", "host"])
        );
        assert!(!args.iter().any(|arg| arg == "--permission-prompt-tool"));
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
            "x".repeat(super::MAX_FRAME_BYTES)
        );
        assert!(parse_output_frame(&frame).is_err());
    }

    fn projector() -> TurnProjector {
        TurnProjector::new(
            "session-1".into(),
            "turn-1".into(),
            "show status".into(),
            "user-1".into(),
        )
    }

    #[test]
    fn acknowledges_only_the_exact_submitted_message_once() {
        let mut projector = projector();
        assert!(
            projector
                .project(OutputFrame::UserReplay {
                    text: "different prompt".into()
                })
                .is_err()
        );
        assert!(!projector.acknowledged());

        let events = projector
            .project(OutputFrame::UserReplay {
                text: "show status".into(),
            })
            .unwrap();
        assert!(
            matches!(events.first(), Some(ProjectedEvent::Notification { method, .. }) if method == "turn/started")
        );
        assert!(
            matches!(events.last(), Some(ProjectedEvent::Acknowledged { result }) if result["turn"]["id"] == "turn-1")
        );
        assert!(projector.acknowledged());
        assert!(
            projector
                .project(OutputFrame::UserReplay {
                    text: "show status".into()
                })
                .unwrap()
                .is_empty()
        );
    }

    #[test]
    fn projects_streamed_assistant_text_into_history_notifications() {
        let mut projector = projector();
        projector
            .project(OutputFrame::MessageStarted {
                message_id: "message-1".into(),
                model: "claude-sonnet".into(),
            })
            .unwrap();
        let started = projector
            .project(OutputFrame::TextDelta {
                block_index: 0,
                text: "hello".into(),
            })
            .unwrap();
        assert!(
            matches!(started.first(), Some(ProjectedEvent::Notification { method, .. }) if method == "item/started")
        );
        assert!(
            matches!(started.last(), Some(ProjectedEvent::Notification { method, params }) if method == "item/agentMessage/delta" && params["delta"] == "hello")
        );

        let completed = projector
            .project(OutputFrame::Result {
                session_id: "session-1".into(),
                is_error: false,
                result: Some("hello".into()),
            })
            .unwrap();
        assert!(
            matches!(completed.first(), Some(ProjectedEvent::Notification { method, params }) if method == "item/completed" && params["item"]["text"] == "hello")
        );
        assert!(
            matches!(completed.last(), Some(ProjectedEvent::Notification { method, params }) if method == "turn/completed" && params["turn"]["status"] == "completed")
        );
    }

    #[test]
    fn projects_tool_input_only_after_the_complete_json_arrives() {
        let mut projector = projector();
        projector
            .project(OutputFrame::ToolUse {
                block_index: 2,
                id: "tool-1".into(),
                name: "Bash".into(),
                input: json!({}),
            })
            .unwrap();
        assert!(
            projector
                .project(OutputFrame::ToolInputDelta {
                    block_index: 2,
                    partial_json: "{\"command\":\"pwd\"}".into(),
                })
                .unwrap()
                .is_empty()
        );
        let started = projector
            .project(OutputFrame::ContentBlockEnded { block_index: 2 })
            .unwrap();
        assert!(
            matches!(started.as_slice(), [ProjectedEvent::Notification { method, params }] if method == "item/started" && params["item"]["command"] == "pwd")
        );

        let completed = projector
            .project(OutputFrame::ToolResults(vec![super::ToolResult {
                id: "tool-1".into(),
                output: "done".into(),
                is_error: false,
            }]))
            .unwrap();
        assert!(
            matches!(completed.as_slice(), [ProjectedEvent::Notification { method, params }] if method == "item/completed" && params["item"]["aggregatedOutput"] == "done")
        );
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
    fn fails_closed_on_malformed_identity_and_unknown_frame_types() {
        assert!(
            parse_output_frame(r#"{"type":"result","session_id":"","is_error":false}"#).is_err()
        );
        assert!(parse_output_frame(r#"{"type":"unknown_future_event"}"#).is_err());
    }
}
