//! What another machine's daemon may answer to a chat request from this window. That daemon is
//! someone else's computer (ADR 0007), so only the answers a chat asks for are accepted, and
//! every string in them is made safe to draw before anything reads it.
//!
//! The walk is over the answer's serialized form, so a string field added to the protocol is
//! cleaned without anyone remembering to list it here.

use crate::remote_snapshot::{ITEMS_MAX, NODES_MAX};
use maitty_shared::hidden::{is_invisible, sanitize};
use maitty_shared::protocol::Reply;
use serde::Serialize;
use serde::de::DeserializeOwned;
use serde_json::Value;

/// Longest string kept in a chat answer, in characters: a message, a tool's output, a diff.
pub const CHAT_TEXT_MAX: usize = 256 * 1024;

/// `reply` made safe, or why it is refused. Only the answers a chat asks for pass: a history
/// page, a send's receipt (delivered or queued) and a bare acknowledgement. Anything else, a
/// file's contents or a screen included, is refused, so a remote cannot answer a chat question
/// with something this window would treat differently.
pub fn vet_chat_reply(reply: Reply) -> Result<Reply, String> {
    match reply {
        Reply::Ok => Ok(Reply::Ok),
        Reply::ConversationRead(page) => Ok(Reply::ConversationRead(cleaned(page)?)),
        Reply::ConversationSent(receipt) => Ok(Reply::ConversationSent(cleaned(receipt)?)),
        Reply::ConversationQueued(receipt) => Ok(Reply::ConversationQueued(cleaned(receipt)?)),
        _ => Err("it is not an answer a chat asks for".into()),
    }
}

fn cleaned<T: Serialize + DeserializeOwned>(answer: T) -> Result<T, String> {
    let mut value = serde_json::to_value(&answer).map_err(|_| "it could not be inspected")?;
    drop(answer);
    let mut nodes = 0;
    clean(&mut value, &mut nodes)?;
    serde_json::from_value(value).map_err(|_| "it was not the answer it claimed once cleaned".into())
}

fn clean(value: &mut Value, nodes: &mut usize) -> Result<(), String> {
    *nodes += 1;
    if *nodes > NODES_MAX {
        return Err("it holds too many values".into());
    }
    match value {
        Value::String(text) => *text = clean_text(text),
        Value::Array(items) => {
            items.truncate(ITEMS_MAX);
            for item in items {
                clean(item, nodes)?;
            }
        }
        Value::Object(map) => {
            let extra: Vec<String> = map.keys().skip(ITEMS_MAX).cloned().collect();
            for key in extra {
                map.remove(&key);
            }
            for item in map.values_mut() {
                clean(item, nodes)?;
            }
        }
        Value::Null | Value::Bool(_) | Value::Number(_) => {}
    }
    Ok(())
}

/// Chat text keeps its lines and its tabs (code reads badly without them); everything else that
/// is a control or an invisible character goes, and a string is cut at `CHAT_TEXT_MAX`.
fn clean_text(text: &str) -> String {
    text.chars()
        .take(CHAT_TEXT_MAX)
        .flat_map(|c| match c {
            '\n' | '\t' => Some(c),
            c if c.is_control() || is_invisible(c) => sanitize(&c.to_string(), true).chars().next(),
            c => Some(c),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_shared::conversation::{ConversationId, QueuedReceipt};

    #[test]
    fn chat_text_keeps_lines_and_tabs_and_loses_escapes_and_hidden_characters() {
        let said = "fn main() {\n\tprintln!(\"hi\");\x1b[31m\x07\u{202e}\n}\r\n";
        assert_eq!(clean_text(said), "fn main() {\n\tprintln!(\"hi\");[31m\n} \n");
    }

    #[test]
    fn a_string_past_the_limit_is_cut() {
        let long = "a".repeat(CHAT_TEXT_MAX + 10);
        assert_eq!(clean_text(&long).chars().count(), CHAT_TEXT_MAX);
    }

    #[test]
    fn every_string_in_an_answer_is_cleaned_wherever_it_sits() {
        let mut value = serde_json::json!({ "a": ["x\x1b[2Jy", { "b": "p\x07q" }], "n": 3 });
        clean(&mut value, &mut 0).unwrap();
        assert_eq!(value, serde_json::json!({ "a": ["x[2Jy", { "b": "pq" }], "n": 3 }));
    }

    #[test]
    fn the_answers_a_chat_asks_for_pass_and_the_rest_are_refused() {
        let queued = QueuedReceipt { conversation: ConversationId(4), generation: 2, operation: 9 };
        assert!(matches!(vet_chat_reply(Reply::Ok), Ok(Reply::Ok)));
        let Ok(Reply::ConversationQueued(back)) = vet_chat_reply(Reply::ConversationQueued(queued.clone())) else {
            panic!("a receipt is an answer a chat asks for");
        };
        assert_eq!(back, queued);
        for other in [Reply::Text("hi".into()), Reply::ConversationFile("secret".into()), Reply::TerminalReplay(vec![1])] {
            assert!(vet_chat_reply(other).is_err());
        }
    }
}
