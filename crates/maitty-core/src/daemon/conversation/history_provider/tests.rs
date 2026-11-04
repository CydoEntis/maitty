use crate::conversation::ConversationId;
use crate::daemon::conversation::history::{
    Attachments, Completeness, Content, Entry, History, IncompleteReason, ItemState,
};
use serde_json::json;

fn read_item(method: &str, extra: serde_json::Value, item: serde_json::Value) -> Entry {
    let mut a = Attachments::default();
    a.attach(1, ConversationId(1));
    let mut params = json!({"threadId":"native","turnId":"turn","item":item});
    for (key, value) in extra.as_object().into_iter().flatten() {
        params[key] = value.clone();
    }
    a.notification(ConversationId(1), "native", method, &params)
        .unwrap();
    let history = a.get(1).unwrap();
    assert_ne!(
        history.completeness,
        Completeness::Incomplete(IncompleteReason::UnsupportedItem),
        "{item}"
    );
    history.entries[0].clone()
}

#[test]
fn a_claude_tool_call_keeps_its_bounded_input_for_every_tool() {
    let cases = [
        json!({"file_path":"a.rs","old_string":"x","new_string":"y"}),
        json!({"file_path":"a.rs","edits":[{"old_string":"x","new_string":"y"}]}),
        json!({"file_path":"a.rs","content":"fn main() {}"}),
        json!({"todos":[{"content":"ship","status":"pending","activeForm":"Shipping"}]}),
        json!({"description":"Explore","prompt":"look around","subagent_type":"Explore"}),
    ];
    for input in cases {
        let entry = read_item(
            "item/completed",
            json!({}),
            json!({"id":"1","type":"commandExecution","command":"Edit ","status":"completed","aggregatedOutput":"ok","input":input}),
        );
        let Content::Tool { input: kept, .. } = entry.content else {
            panic!("a tool row");
        };
        assert_eq!(kept, Some(input));
    }
}

#[test]
fn tool_input_is_cut_and_cleaned_before_it_is_kept() {
    let entry = read_item(
        "item/completed",
        json!({}),
        json!({"id":"1","type":"commandExecution","command":"Write ","status":"completed","input":{
            "file_path":"a.txt",
            "content": format!("\u{1b}[31mred\u{1b}[0m{}", "x".repeat(40_000)),
        }}),
    );
    let Content::Tool {
        input: Some(input), ..
    } = entry.content
    else {
        panic!("input is kept");
    };
    let content = input["content"].as_str().unwrap();
    assert!(content.starts_with("redxx"), "escape sequences are gone");
    assert!(content.ends_with("[truncated]"));
    assert!(content.len() < 17_000);
}

#[test]
fn codex_commands_carry_their_exit_code_duration_and_a_long_label() {
    let long = format!("echo {}", "a".repeat(1500));
    let entry = read_item(
        "item/completed",
        json!({"completedAtMs": 1_700_000_005_000_u64}),
        json!({"id":"1","type":"commandExecution","command":long,"status":"failed","aggregatedOutput":"boom","exitCode":2,"durationMs":4200}),
    );
    assert_eq!(entry.completed_at_ms, Some(1_700_000_005_000));
    let Content::Tool {
        label,
        exit_code,
        duration_ms,
        input,
        ..
    } = entry.content
    else {
        panic!("a tool row");
    };
    assert_eq!(label, long, "a label of ordinary length is no longer cut");
    assert_eq!((exit_code, duration_ms, input), (Some(2), Some(4200), None));

    let huge = "b".repeat(5000);
    let entry = read_item(
        "item/completed",
        json!({}),
        json!({"id":"1","type":"commandExecution","command":huge,"status":"completed"}),
    );
    let Content::Tool {
        label, exit_code, ..
    } = entry.content
    else {
        panic!("a tool row");
    };
    assert_eq!(label.chars().count(), 2048);
    assert_eq!(exit_code, None, "a command that reports no status has none");
}

#[test]
fn codex_plan_and_helper_items_keep_their_text_and_title() {
    let plan = read_item(
        "item/completed",
        json!({}),
        json!({"id":"p","type":"plan","text":"1. Read\n2. Fix\u{1b}[0m"}),
    );
    assert_eq!(plan.state, ItemState::Completed);
    assert_eq!(
        plan.content,
        Content::Tool {
            kind: "plan".into(),
            output: "1. Read\n2. Fix".into(),
            label: "Plan".into(),
            input: None,
            exit_code: None,
            duration_ms: None,
        }
    );

    let helper = read_item(
        "item/started",
        json!({}),
        json!({"id":"h","type":"collabAgentToolCall","tool":"spawnAgent","status":"inProgress","prompt":"\nAudit the parser\nand report","model":"gpt-5","agentsStates":{},"receiverThreadIds":[],"senderThreadId":"native"}),
    );
    assert_eq!(helper.state, ItemState::Running);
    let Content::Tool {
        kind, label, input, ..
    } = helper.content
    else {
        panic!("a tool row");
    };
    assert_eq!(kind, "collabAgentToolCall");
    assert_eq!(label, "Audit the parser");
    assert_eq!(
        input,
        Some(json!({
            "description":"Audit the parser",
            "prompt":"\nAudit the parser\nand report",
            "tool":"spawnAgent",
            "model":"gpt-5"
        }))
    );

    let waiting = read_item(
        "item/completed",
        json!({}),
        json!({"id":"w","type":"collabAgentToolCall","tool":"wait","status":"completed"}),
    );
    assert!(matches!(waiting.content, Content::Tool { ref label, .. } if label == "wait"));
}

#[test]
fn items_and_turns_carry_the_times_the_provider_reported() {
    let mut a = Attachments::default();
    a.attach(1, ConversationId(1));
    let item = json!({"id":"1","type":"commandExecution","command":"ls","status":"inProgress"});
    let done = json!({"id":"1","type":"commandExecution","command":"ls","status":"completed"});
    a.notification(
        ConversationId(1),
        "native",
        "item/started",
        &json!({"threadId":"native","turnId":"turn","startedAtMs":1000_u64,"item":item}),
    )
    .unwrap();
    a.notification(
        ConversationId(1),
        "native",
        "item/completed",
        &json!({"threadId":"native","turnId":"turn","completedAtMs":8000_u64,"item":done}),
    )
    .unwrap();
    a.notification(ConversationId(1), "native", "turn/completed", &json!({"threadId":"native","turn":{"id":"turn","status":"completed","startedAt":10,"completedAt":17}})).unwrap();
    let entries = &a.get(1).unwrap().entries;
    assert_eq!(
        (entries[0].started_at_ms, entries[0].completed_at_ms),
        (Some(1000), Some(8000)),
        "the completion keeps the start the first event set"
    );
    assert_eq!(
        (entries[1].started_at_ms, entries[1].completed_at_ms),
        (Some(10_000), Some(17_000)),
        "turn times arrive in seconds"
    );
}

#[test]
fn history_cached_before_these_fields_still_reads() {
    let history: History = serde_json::from_value(json!({
        "conversation": 1,
        "revision": 3,
        "completeness": "Complete",
        "entries": [{
            "source": {"conversation": 1, "turn": "t", "item": "i"},
            "content": {"Tool": {"kind": "commandExecution", "output": "ok", "label": "ls"}},
            "state": "Completed"
        }]
    }))
    .unwrap();
    assert_eq!(history.entries.len(), 1);
    assert_eq!(history.entries[0].started_at_ms, None);
}
