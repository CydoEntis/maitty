use super::history::{Content, Entry, IncompleteReason, ItemState, Role, SourceId, Update};
use crate::conversation::ConversationId;
use serde_json::Value;

type Result<T> = std::result::Result<T, IncompleteReason>;
const ERROR_MESSAGE_CHARS: usize = 1024;
const TOOL_LABEL_CHARS: usize = 256;

fn tool_label(item: &Value) -> String {
    item.get("command")
        .and_then(Value::as_str)
        .filter(|command| !command.trim().is_empty())
        .map(|command| command.chars().take(TOOL_LABEL_CHARS).collect())
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
    if kind == "reasoning" {
        return Ok(None);
    }
    let content = match kind {
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
        },
        "fileChange" => Content::Tool {
            kind: kind.into(),
            label: file_change_label(item),
            output: file_change_output(item),
        },
        // Never retain unknown raw provider objects: they can contain unrelated private state.
        _ => Content::Unsupported { kind: kind.into() },
    };
    // A file edit may not carry a status on its first event; fall back to the event name.
    let state = if kind == "commandExecution"
        || (kind == "fileChange" && item.get("status").is_some())
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
    })))
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
        output.push_str(change.get("diff").and_then(Value::as_str).unwrap_or_default());
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
