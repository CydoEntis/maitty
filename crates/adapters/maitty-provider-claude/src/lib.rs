use anyhow::{Context, Result, bail};
use serde_json::Value;
use std::ffi::OsString;

mod hooks;
mod projection;
pub use hooks::{
    JsonHookGroupOptions, STATUS_EVENTS, StatusLineFileChange, is_our_hook_group, is_our_name,
    is_our_statusline, json_hook_group, replace_json_hook_group, runs_our_hook, stale_hooks,
    status_from_hook, update_statusline,
};
pub use projection::{MAX_PENDING_TOOLS, ProjectedEvent, TurnProjector};

pub const MAX_FRAME_BYTES: usize = 1024 * 1024;
const VERIFIED_VERSIONS: &[&str] = &["2.1.312", "2.1.314", "2.1.316"];

#[derive(Debug, PartialEq)]
pub enum OutputFrame {
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
pub struct ToolResult {
    pub id: String,
    pub output: String,
    pub is_error: bool,
}

pub fn parse_output_frame(line: &str) -> Result<OutputFrame> {
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

/// Converts provider tool content to a bounded text preview.
pub fn bounded_content_text(content: Option<&Value>) -> String {
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

pub fn verified_version(version: &str) -> bool {
    VERIFIED_VERSIONS.contains(&version)
}

pub fn version_number(output: &str) -> Option<&str> {
    let output = output.trim();
    let output = output.strip_prefix("Claude Code ").unwrap_or(output);
    let version = output.strip_suffix(" (Claude Code)").unwrap_or(output);
    (!version.is_empty() && !version.chars().any(char::is_whitespace)).then_some(version)
}

pub fn session_id() -> Result<String> {
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

pub fn command_arguments(
    session_id: &str,
    resume: bool,
    settings: &maitty_shared::conversation::LaunchSettings,
) -> Vec<OsString> {
    let mut arguments = vec![
        "-p".into(),
        "--input-format".into(),
        "stream-json".into(),
        "--output-format".into(),
        "stream-json".into(),
        "--include-partial-messages".into(),
        "--verbose".into(),
        "--permission-prompt-tool=stdio".into(),
        "--replay-user-messages".into(),
    ];
    let permission_mode = match (settings.sandbox, settings.approval_policy) {
        (maitty_shared::conversation::Sandbox::ReadOnly, _) => "plan",
        (_, maitty_shared::conversation::ApprovalPolicy::Never) => "dontAsk",
        (
            _,
            maitty_shared::conversation::ApprovalPolicy::Untrusted
            | maitty_shared::conversation::ApprovalPolicy::OnRequest,
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

#[cfg(test)]
mod tests {
    use super::{
        OutputFrame, bounded_content_text, command_arguments, parse_output_frame, session_id,
        verified_version, version_number,
    };
    use maitty_shared::conversation::{ApprovalPolicy, LaunchSettings, Provider, Sandbox};
    use std::path::PathBuf;

    #[test]
    fn only_the_locally_verified_cli_version_is_supported() {
        assert!(verified_version("2.1.312"));
        assert!(verified_version("2.1.314"));
        assert!(verified_version("2.1.316"));
        assert!(!verified_version("2.1.313"));
    }

    #[test]
    fn reads_supported_versions_from_claude_version_output() {
        assert_eq!(version_number("2.1.314 (Claude Code)"), Some("2.1.314"));
        assert_eq!(version_number("Claude Code 2.1.312"), Some("2.1.312"));
        assert_eq!(version_number(" 2.1.313 "), Some("2.1.313"));
        assert_eq!(version_number("2.1.314 (unexpected)"), None);
    }

    #[test]
    fn parses_streamed_text_and_refuses_unknown_frame_types() {
        assert_eq!(
            parse_output_frame(r#"{"type":"stream_event","event":{"type":"content_block_delta","index":2,"delta":{"type":"text_delta","text":"hello"}}}"#).unwrap(),
            OutputFrame::TextDelta { block_index: 2, text: "hello".into() }
        );
        assert!(parse_output_frame(r#"{"type":"unknown_future_event"}"#).is_err());
    }

    #[test]
    fn tool_content_previews_are_bounded() {
        let preview = bounded_content_text(Some(&serde_json::json!("x".repeat(9000))));
        assert!(preview.ends_with("[Output truncated]"));
        assert!(preview.len() < 9000);
    }

    #[test]
    fn session_ids_are_unique_valid_uuids() {
        let first = session_id().unwrap();
        let second = session_id().unwrap();
        assert_eq!(first.len(), 36);
        assert_eq!(first.as_bytes()[14], b'4');
        assert!(matches!(first.as_bytes()[19], b'8'..=b'b'));
        assert_ne!(first, second);
    }

    #[test]
    fn read_only_launch_arguments_select_plan_mode_and_the_model() {
        let settings = LaunchSettings {
            provider: Provider::Claude,
            executable: PathBuf::from("claude"),
            config_root: PathBuf::from("config"),
            cwd: PathBuf::from("workspace"),
            model: Some("sonnet".into()),
            model_provider: None,
            approval_policy: ApprovalPolicy::OnRequest,
            sandbox: Sandbox::ReadOnly,
        };
        let arguments = command_arguments("session-1", false, &settings);
        let arguments = arguments
            .iter()
            .map(|argument| argument.to_string_lossy())
            .collect::<Vec<_>>();

        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--permission-mode", "plan"])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--session-id", "session-1"])
        );
        assert!(
            arguments
                .windows(2)
                .any(|pair| pair == ["--model", "sonnet"])
        );
    }
}
