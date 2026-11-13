use super::history::{Content, Entry, IncompleteReason, ItemState, Role, SourceId, Update};
use super::tool_input::{bound_input, clean_text};
use crate::conversation::ConversationId;
use serde_json::Value;

type Result<T> = std::result::Result<T, IncompleteReason>;
const ERROR_MESSAGE_CHARS: usize = 1024;
/// A command is shown whole in the chat view, so the label keeps far more than a title would.
const TOOL_LABEL_CHARS: usize = 2048;
/// A helper agent's title: the first line of the task it was given.
const HELPER_TITLE_CHARS: usize = 120;
/// Largest plan text kept for one plan item.
const MAX_PLAN_BYTES: usize = 64 * 1024;
/// Largest summary text kept for one reasoning item.
const MAX_REASONING_BYTES: usize = 64 * 1024;

fn tool_label(item: &Value) -> String {
    item.get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.trim().is_empty())
        .map(|command| clean_text(command).chars().take(TOOL_LABEL_CHARS).collect())
        .unwrap_or_else(|| "Run command".into())
}

pub(super) fn error_message(value: &Value) -> Result<String> {
    let message = text(value, "message")?;
    Ok(message.chars().take(ERROR_MESSAGE_CHARS).collect())
}
fn text<'a>(value: &'a Value, field: &str) -> Result<&'a str> {
    value
        .get(field)
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
        .ok_or(IncompleteReason::InvalidEvent)
}
fn status(value: &str) -> Result<ItemState> {
    match value {
        "inProgress" => Ok(ItemState::Running),
        "completed" => Ok(ItemState::Completed),
        "failed" | "declined" => Ok(ItemState::Failed),
        "interrupted" => Ok(ItemState::Interrupted),
        _ => Err(IncompleteReason::InvalidEvent),
    }
}
pub(super) fn notification(
    conversation: ConversationId,
    thread: &str,
    method: &str,
    params: &Value,
) -> Result<Option<Update>> {
    if !matches!(
        method,
        "item/started"
            | "error"
            | "item/completed"
            | "item/agentMessage/delta"
            | "turn/started"
            | "turn/completed"
    ) {
        return Ok(None);
    }
    if text(params, "threadId")? != thread {
        return Err(IncompleteReason::InvalidEvent);
    }
    if method == "error" {
        text(params, "turnId")?;
        let message = error_message(params.get("error").ok_or(IncompleteReason::InvalidEvent)?)?;
        let will_retry = params
            .get("willRetry")
            .and_then(Value::as_bool)
            .ok_or(IncompleteReason::InvalidEvent)?;
        return Ok(Some(Update::ProviderError {
            message,
            will_retry,
        }));
    }
    if method.starts_with("turn/") {
        let turn = params.get("turn").ok_or(IncompleteReason::InvalidEvent)?;
        return Ok(Some(Update::SetTurnStatus {
            turn: text(turn, "id")?.into(),
            state: status(text(turn, "status")?)?,
            started_at_ms: seconds_as_ms(turn, "startedAt"),
            completed_at_ms: seconds_as_ms(turn, "completedAt"),
        }));
    }
    let turn = text(params, "turnId")?.to_owned();
    if method == "item/agentMessage/delta" {
        let source = SourceId {
            conversation,
            turn,
            item: text(params, "itemId")?.into(),
        };
        let delta = params
            .get("delta")
            .and_then(Value::as_str)
            .ok_or(IncompleteReason::InvalidEvent)?
            .into();
        return Ok(Some(Update::AppendDelta { source, delta }));
    }
    let item = params.get("item").ok_or(IncompleteReason::InvalidEvent)?;
    let source = SourceId {
        conversation,
        turn,
        item: text(item, "id")?.into(),
    };
    let kind = text(item, "type")?;
    let content = match kind {
        "reasoning" => match reasoning_summary(item) {
            Some(text) => Content::Reasoning { text },
            None => return Ok(None),
        },
        "agentMessage" => Content::Message {
            role: Role::Assistant,
            text: item
                .get("text")
                .and_then(Value::as_str)
                .ok_or(IncompleteReason::InvalidEvent)?
                .into(),
        },
        "userMessage" => {
            let parts = item
                .get("content")
                .and_then(Value::as_array)
                .ok_or(IncompleteReason::InvalidEvent)?;
            let mut message = String::new();
            for part in parts {
                if text(part, "type")? != "text" {
                    return Err(IncompleteReason::UnsupportedItem);
                }
                message.push_str(
                    part.get("text")
                        .and_then(Value::as_str)
                        .ok_or(IncompleteReason::InvalidEvent)?,
                );
            }
            Content::Message {
                role: Role::User,
                text: message,
            }
        }
        "commandExecution" => Content::Tool {
            kind: kind.into(),
            label: tool_label(item),
            output: item
                .get("aggregatedOutput")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
            // Claude's adapter sends the tool's input here; Codex commands have none.
            input: item.get("input").and_then(bound_input),
            exit_code: item.get("exitCode").and_then(Value::as_i64),
            duration_ms: item.get("durationMs").and_then(Value::as_u64),
        },
        "fileChange" => Content::Tool {
            kind: kind.into(),
            label: file_change_label(item),
            output: file_change_output(item),
            input: None,
            exit_code: None,
            duration_ms: None,
        },
        "plan" => Content::Tool {
            kind: kind.into(),
            label: "Plan".into(),
            output: bounded_plan(item.get("text").and_then(Value::as_str).unwrap_or_default()),
            input: None,
            exit_code: None,
            duration_ms: None,
        },
        "collabAgentToolCall" => Content::Tool {
            kind: kind.into(),
            label: helper_title(item),
            output: String::new(),
            input: helper_input(item),
            exit_code: None,
            duration_ms: None,
        },
        // Never retain unknown raw provider objects: they can contain unrelated private state.
        _ => Content::Unsupported { kind: kind.into() },
    };
    // A file edit may not carry a status on its first event; fall back to the event name.
    let state = if kind == "commandExecution"
        || (matches!(kind, "fileChange" | "collabAgentToolCall") && item.get("status").is_some())
    {
        status(text(item, "status")?)?
    } else if method == "item/started" {
        ItemState::Running
    } else {
        ItemState::Completed
    };
    Ok(Some(Update::UpsertItem(Entry {
        source,
        content,
        state,
        started_at_ms: params.get("startedAtMs").and_then(Value::as_u64),
        completed_at_ms: params.get("completedAtMs").and_then(Value::as_u64),
    })))
}

/// A turn's `startedAt` / `completedAt`, which Codex reports in whole seconds.
fn seconds_as_ms(turn: &Value, field: &str) -> Option<u64> {
    turn.get(field)
        .and_then(Value::as_u64)
        .and_then(|seconds| seconds.checked_mul(1000))
}

fn bounded_plan(text: &str) -> String {
    bounded(text, MAX_PLAN_BYTES, "plan")
}

/// `text` with terminal escapes stripped, cut at `max_bytes` with a note naming `what` was cut.
fn bounded(text: &str, max_bytes: usize, what: &str) -> String {
    let clean = clean_text(text);
    if clean.len() <= max_bytes {
        return clean;
    }
    let mut end = max_bytes;
    while !clean.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}\n... {what} cut off", &clean[..end])
}

/// The summary a Codex reasoning item publishes as plain text, its parts one paragraph each.
/// `None` when it has none: the reasoning itself stays encrypted and is never read or kept.
fn reasoning_summary(item: &Value) -> Option<String> {
    let parts: Vec<&str> = item
        .get("summary")?
        .as_array()?
        .iter()
        // A plain string from the app server; `{ "text": … }` in the provider's own item shape.
        .filter_map(|part| part.as_str().or_else(|| part.get("text")?.as_str()))
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .collect();
    if parts.is_empty() {
        return None;
    }
    Some(bounded(&parts.join("\n\n"), MAX_REASONING_BYTES, "reasoning"))
}

/// The first line of the task given to a helper agent, or the helper tool's name when the call
/// carries no prompt (waiting on or closing a helper).
fn helper_title(item: &Value) -> String {
    let first_line = item
        .get("prompt")
        .and_then(Value::as_str)
        .map(clean_text)
        .and_then(|prompt| {
            prompt
                .lines()
                .map(str::trim)
                .find(|line| !line.is_empty())
                .map(str::to_owned)
        });
    match first_line {
        Some(line) => line.chars().take(HELPER_TITLE_CHARS).collect(),
        None => item
            .get("tool")
            .and_then(Value::as_str)
            .map_or_else(|| "Helper agent".into(), clean_text),
    }
}

/// What a helper-agent call was asked to do, in the same shape Claude's `Task` input has.
fn helper_input(item: &Value) -> Option<Value> {
    let field = |name: &str| item.get(name).and_then(Value::as_str);
    let mut input = serde_json::Map::new();
    input.insert("description".into(), helper_title(item).into());
    for name in ["prompt", "tool", "model"] {
        if let Some(value) = field(name) {
            input.insert(name.into(), value.into());
        }
    }
    bound_input(&Value::Object(input))
}

/// Largest diff kept for one edit, so one huge file cannot bloat a conversation's history.
const MAX_FILE_CHANGE_OUTPUT_BYTES: usize = 100_000;

fn file_changes(item: &Value) -> &[Value] {
    item.get("changes")
        .and_then(Value::as_array)
        .map_or(&[], Vec::as_slice)
}

fn change_path(change: &Value) -> &str {
    change.get("path").and_then(Value::as_str).unwrap_or("file")
}

/// "Edited main.go" for one file, "Edited 3 files" for several.
fn file_change_label(item: &Value) -> String {
    match file_changes(item) {
        [] => "Edited files".into(),
        [only] => format!("Edited {}", change_path(only)),
        many => format!("Edited {} files", many.len()),
    }
}

/// How the provider describes a change: a plain word, or an object with a type.
fn change_kind(change: &Value) -> &str {
    let kind = change.get("kind");
    kind.and_then(Value::as_str)
        .or_else(|| {
            kind.and_then(|value| value.get("type"))
                .and_then(Value::as_str)
        })
        .unwrap_or("update")
}

/// The diffs of every changed file, each under a header naming the file.
fn file_change_output(item: &Value) -> String {
    let mut output = String::new();
    for change in file_changes(item) {
        if !output.is_empty() {
            output.push('\n');
        }
        output.push_str(&format!("{} ({})\n", change_path(change), change_kind(change)));
        // The content of a secret file never enters the history; its path and kind do.
        output.push_str(&super::redact::redact_diff(
            change_path(change),
            change.get("diff").and_then(Value::as_str).unwrap_or_default(),
        ));
    }
    if output.len() > MAX_FILE_CHANGE_OUTPUT_BYTES {
        let mut end = MAX_FILE_CHANGE_OUTPUT_BYTES;
        while !output.is_char_boundary(end) {
            end -= 1;
        }
        output.truncate(end);
        output.push_str("\n... diff cut off");
    }
    output
}

#[cfg(test)]
mod tests;
