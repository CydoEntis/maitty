use super::history::{Content, Entry, IncompleteReason, ItemState, Role, SourceId, Update};
use crate::conversation::ConversationId;
use serde_json::Value;

type Result<T> = std::result::Result<T, IncompleteReason>;
const ERROR_MESSAGE_CHARS: usize = 1024;

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
            output: item
                .get("aggregatedOutput")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .into(),
        },
        // Never retain unknown raw provider objects: they can contain unrelated private state.
        _ => Content::Unsupported { kind: kind.into() },
    };
    let state = if kind == "commandExecution" {
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
