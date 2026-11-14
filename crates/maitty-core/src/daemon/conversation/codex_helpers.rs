//! The helpers a daemon-owned Codex conversation starts, from what its app-server reports (ADR
//! 0010). Measured with Codex 0.166: the parent thread gets a `subAgentActivity` item when a helper
//! starts (`kind: "started"`, with `agentThreadId`, `agentPath` `/root/<task name>`, `model` and
//! `reasoningEffort`) and when it finishes (`kind: "completed"`). The helper's own thread streams
//! its `turn/started`, items and `turn/completed` on the same connection, under its own
//! `threadId`. Helpers are display only: nothing here touches the conversation's status.

use super::tool_input::clean_text;
use crate::protocol::{SubagentActivity, SubagentState};
use serde_json::Value;

/// The most helpers one conversation keeps; finished ones go first when there are more.
pub(super) const HELPERS_MAX: usize = 32;
/// The longest helper name kept, in characters.
const NAME_CHARS: usize = 120;
/// The longest final answer kept for a helper, in characters.
const RESULT_CHARS: usize = 4000;

/// A `subAgentActivity` item seen on `parent`'s thread (the conversation's own thread when
/// `parent` is `None`). Any other item is left alone; returns whether it was one.
pub(super) fn activity(helpers: &mut Vec<SubagentActivity>, item: &Value, parent: Option<&str>, now: u64) -> bool {
    if item.get("type").and_then(Value::as_str) != Some("subAgentActivity") {
        return false;
    }
    let Some(thread) = item.get("agentThreadId").and_then(Value::as_str).filter(|id| !id.is_empty()) else {
        return true;
    };
    match item.get("kind").and_then(Value::as_str) {
        Some("started") => {
            if !helpers.iter().any(|helper| helper.id == thread) {
                let name = helper_name(item);
                helpers.push(SubagentActivity {
                    id: thread.to_owned(),
                    name: name.clone(),
                    parent: parent.map(str::to_owned),
                    state: SubagentState::Working,
                    task: Some(name),
                    model: field(item, "model"),
                    effort: field(item, "reasoningEffort"),
                    started_at: item.get("startedAtMs").and_then(Value::as_u64).map(|ms| ms / 1000),
                    updated_at: now,
                    latest_activity: None,
                    result: None,
                });
                keep_bounded(helpers);
            }
        }
        Some("completed") => {
            if let Some(helper) = helpers.iter_mut().find(|helper| helper.id == thread)
                && helper.state.active()
            {
                helper.state = SubagentState::Completed;
                helper.updated_at = now;
            }
        }
        _ => {}
    }
    true
}

/// Whether `thread` is one of the conversation's helpers, so its events are the helper's own.
pub(super) fn is_helper(helpers: &[SubagentActivity], thread: &str) -> bool {
    helpers.iter().any(|helper| helper.id == thread)
}

/// An event on a helper's own thread: its turns say whether it works and how it ended, and a
/// helper it starts in turn becomes one of the conversation's helpers.
pub(super) fn helper_event(helpers: &mut Vec<SubagentActivity>, thread: &str, method: &str, params: &Value, now: u64) {
    if matches!(method, "item/started" | "item/completed")
        && let Some(item) = params.get("item")
        && activity(helpers, item, Some(thread), now)
    {
        return;
    }
    let Some(helper) = helpers.iter_mut().find(|helper| helper.id == thread) else {
        return;
    };
    match method {
        "turn/started" => {
            helper.state = SubagentState::Working;
            helper.updated_at = now;
        }
        "turn/completed" => {
            let turn = params.get("turn");
            helper.state = match turn.and_then(|turn| turn.get("status")).and_then(Value::as_str) {
                Some("failed") => SubagentState::Failed,
                Some("interrupted") => SubagentState::Cancelled,
                _ => SubagentState::Completed,
            };
            helper.updated_at = now;
            if let Some(answer) = turn.and_then(final_answer) {
                helper.result = Some(answer);
            }
        }
        "item/completed" => {
            if let Some(text) = params
                .get("item")
                .filter(|item| item.get("type").and_then(Value::as_str) == Some("agentMessage"))
                .and_then(|item| item.get("text"))
                .and_then(Value::as_str)
                .map(|text| clip(&clean_text(text), RESULT_CHARS))
                .filter(|text| !text.trim().is_empty())
            {
                helper.latest_activity = Some(clip(&text, NAME_CHARS));
                helper.updated_at = now;
            }
        }
        _ => {}
    }
}

/// The last agent message a finished turn lists, which is the helper's answer.
fn final_answer(turn: &Value) -> Option<String> {
    turn.get("items")?
        .as_array()?
        .iter()
        .rev()
        .filter(|item| item.get("type").and_then(Value::as_str) == Some("agentMessage"))
        .find_map(|item| item.get("text").and_then(Value::as_str))
        .map(|text| clip(&clean_text(text), RESULT_CHARS))
        .filter(|text| !text.trim().is_empty())
}

/// The task name Codex gave the helper: the end of its `agentPath` (`/root/review` is `review`).
fn helper_name(item: &Value) -> String {
    item.get("agentPath")
        .and_then(Value::as_str)
        .and_then(|path| path.rsplit('/').find(|part| !part.is_empty()))
        .map(|name| clip(&clean_text(name), NAME_CHARS))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "helper".into())
}

fn field(item: &Value, name: &str) -> Option<String> {
    item.get(name)
        .and_then(Value::as_str)
        .map(|text| clip(&clean_text(text), NAME_CHARS))
        .filter(|text| !text.is_empty())
}

fn clip(text: &str, most: usize) -> String {
    text.chars().take(most).collect()
}

/// Drops the oldest finished helpers, then the oldest of all, until at most `HELPERS_MAX` remain.
fn keep_bounded(helpers: &mut Vec<SubagentActivity>) {
    while helpers.len() > HELPERS_MAX {
        let at = helpers.iter().position(|helper| !helper.state.active()).unwrap_or(0);
        helpers.remove(at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn started(thread: &str, path: &str) -> Value {
        json!({"type": "subAgentActivity", "kind": "started", "id": "call_1", "agentThreadId": thread,
               "agentPath": path, "model": "gpt-a", "reasoningEffort": "low", "startedAtMs": 5_000})
    }

    #[test]
    fn a_started_helper_is_listed_by_its_task_name_and_finishes_with_its_answer() {
        let mut helpers = Vec::new();
        assert!(activity(&mut helpers, &started("child", "/root/probe_helper"), None, 10));
        assert_eq!(helpers[0].name, "probe_helper");
        assert_eq!((helpers[0].state, helpers[0].started_at, helpers[0].model.as_deref()), (SubagentState::Working, Some(5), Some("gpt-a")));
        assert!(is_helper(&helpers, "child") && !is_helper(&helpers, "other"));
        let done = json!({"threadId": "child", "turn": {"status": "completed", "items": [
            {"type": "agentMessage", "text": "thinking"}, {"type": "agentMessage", "text": "pong"}]}});
        helper_event(&mut helpers, "child", "turn/completed", &done, 12);
        assert_eq!((helpers[0].state, helpers[0].result.as_deref(), helpers[0].updated_at), (SubagentState::Completed, Some("pong"), 12));
    }

    #[test]
    fn the_completed_activity_ends_a_helper_and_a_failed_turn_says_so() {
        let mut helpers = Vec::new();
        activity(&mut helpers, &started("a", "/root/a"), None, 1);
        activity(&mut helpers, &started("b", "/root/b"), None, 1);
        activity(&mut helpers, &json!({"type": "subAgentActivity", "kind": "completed", "agentThreadId": "a"}), None, 2);
        helper_event(&mut helpers, "b", "turn/completed", &json!({"turn": {"status": "failed", "items": []}}), 3);
        assert_eq!(helpers.iter().map(|h| h.state).collect::<Vec<_>>(), vec![SubagentState::Completed, SubagentState::Failed]);
    }

    #[test]
    fn a_helper_started_by_a_helper_names_its_parent() {
        let mut helpers = Vec::new();
        activity(&mut helpers, &started("a", "/root/a"), None, 1);
        let nested = json!({"threadId": "a", "item": started("a1", "/root/a/check")});
        helper_event(&mut helpers, "a", "item/completed", &nested, 2);
        assert_eq!((helpers[1].name.as_str(), helpers[1].parent.as_deref()), ("check", Some("a")));
    }

    #[test]
    fn other_items_are_not_activity_and_a_repeat_start_adds_nothing() {
        let mut helpers = Vec::new();
        assert!(!activity(&mut helpers, &json!({"type": "agentMessage", "text": "hi"}), None, 1));
        activity(&mut helpers, &started("a", "/root/a"), None, 1);
        activity(&mut helpers, &started("a", "/root/a"), None, 2);
        assert_eq!(helpers.len(), 1);
    }

    #[test]
    fn the_list_stays_bounded_dropping_finished_helpers_first() {
        let mut helpers = Vec::new();
        activity(&mut helpers, &started("first", "/root/first"), None, 1);
        activity(&mut helpers, &json!({"type": "subAgentActivity", "kind": "completed", "agentThreadId": "first"}), None, 1);
        for n in 0..HELPERS_MAX {
            activity(&mut helpers, &started(&format!("h{n}"), "/root/h"), None, 1);
        }
        assert_eq!(helpers.len(), HELPERS_MAX);
        assert!(!is_helper(&helpers, "first"), "the finished one went first");
    }

    #[test]
    fn names_lose_control_characters() {
        let mut helpers = Vec::new();
        activity(&mut helpers, &started("a", "/root/bad\u{1b}[31mname"), None, 1);
        assert!(!helpers[0].name.contains('\u{1b}'));
    }
}
