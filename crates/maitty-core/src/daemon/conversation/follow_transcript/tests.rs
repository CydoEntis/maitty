use super::*;
use crate::conversation::ConversationId;
use crate::daemon::conversation::history::{Content, Entry, History, ItemState, Role};
use crate::daemon::conversation::history_provider;
use serde_json::json;

const SESSION: &str = "11111111-2222-3333-4444-555555555555";

/// The history a transcript's lines give, through the same projection a structured runtime uses.
fn history_of(format: Format, lines: &[String]) -> History {
    let id = ConversationId(7);
    let mut transcript = Transcript::new(format, SESSION);
    let mut history = History::new(id);
    let mut offset = 0u64;
    for line in lines {
        for note in transcript.feed(offset, line) {
            if let Some(update) =
                history_provider::notification(id, SESSION, note.method, &note.params).unwrap()
            {
                history.apply(update).unwrap();
            }
        }
        offset += line.len() as u64 + 1;
    }
    history
}

fn line(value: serde_json::Value) -> String {
    value.to_string()
}

fn message(entry: &Entry) -> (&Role, &str) {
    match &entry.content {
        Content::Message { role, text } => (role, text),
        other => panic!("not a message: {other:?}"),
    }
}

fn turns(history: &History) -> Vec<&Entry> {
    history
        .entries
        .iter()
        .filter(|e| e.content == Content::TurnStatus)
        .collect()
}

fn items(history: &History) -> Vec<&Entry> {
    history
        .entries
        .iter()
        .filter(|e| e.content != Content::TurnStatus)
        .collect()
}

fn claude_user(uuid: &str, text: &str) -> String {
    line(json!({
        "type": "user", "uuid": uuid, "isSidechain": false,
        "timestamp": "2026-11-04T09:30:00.000Z",
        "message": { "role": "user", "content": text },
    }))
}

fn claude_assistant(uuid: &str, block: serde_json::Value) -> String {
    line(json!({
        "type": "assistant", "uuid": uuid, "isSidechain": false,
        "timestamp": "2026-11-04T09:30:05.000Z",
        "message": { "role": "assistant", "content": [block] },
    }))
}

fn claude_result(tool: &str, content: serde_json::Value, is_error: bool) -> String {
    line(json!({
        "type": "user", "uuid": format!("r-{tool}"), "isSidechain": false,
        "timestamp": "2026-11-04T09:30:07.000Z",
        "message": { "role": "user", "content": [
            { "type": "tool_result", "tool_use_id": tool, "content": content, "is_error": is_error }
        ] },
    }))
}

#[test]
fn a_claude_turn_becomes_messages_tool_steps_and_a_finished_turn() {
    let lines = vec![
        claude_user("u1", "fix the build"),
        claude_assistant(
            "a1",
            json!({ "type": "thinking", "thinking": "", "signature": "x" }),
        ),
        claude_assistant("a2", json!({ "type": "text", "text": "I will run cargo." })),
        claude_assistant(
            "a3",
            json!({ "type": "tool_use", "id": "toolu_1", "name": "Bash", "input": { "command": "cargo build" } }),
        ),
        claude_result("toolu_1", json!("Finished dev"), false),
        claude_assistant(
            "a4",
            json!({ "type": "tool_use", "id": "toolu_2", "name": "Edit", "input": { "file_path": "/w/a.rs", "old_string": "a", "new_string": "b" } }),
        ),
        claude_result(
            "toolu_2",
            json!([{ "type": "text", "text": "edited" }]),
            false,
        ),
        claude_assistant("a5", json!({ "type": "text", "text": "Done." })),
        line(
            json!({ "type": "system", "subtype": "turn_duration", "durationMs": 9000, "timestamp": "2026-11-04T09:30:09.000Z" }),
        ),
    ];

    let history = history_of(Format::Claude, &lines);

    let items = items(&history);
    assert_eq!(message(items[0]), (&Role::User, "fix the build"));
    assert_eq!(message(items[1]), (&Role::Assistant, "I will run cargo."));
    assert_eq!(
        items[2].content,
        Content::Tool {
            kind: "commandExecution".into(),
            output: "Finished dev".into(),
            label: "cargo build".into(),
            input: Some(json!({ "command": "cargo build" })),
            exit_code: None,
            duration_ms: None,
        }
    );
    assert_eq!(items[2].state, ItemState::Completed);
    // The label is the tool's name for a tool that has no command, as the structured adapter's.
    let Content::Tool {
        label,
        input,
        output,
        ..
    } = &items[3].content
    else {
        panic!("not a tool step")
    };
    assert_eq!(label, "Edit ");
    assert_eq!(input.as_ref().unwrap()["file_path"], "/w/a.rs");
    assert_eq!(output, "edited");
    assert_eq!(message(items[4]), (&Role::Assistant, "Done."));
    let turns = turns(&history);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].state, ItemState::Completed);
    assert_eq!(turns[0].started_at_ms, Some(1_793_784_600_000));
    assert_eq!(turns[0].completed_at_ms, Some(1_793_784_609_000));
}

#[test]
fn a_failed_claude_tool_and_a_running_one_keep_their_state() {
    let lines = vec![
        claude_user("u1", "go"),
        claude_assistant(
            "a1",
            json!({ "type": "tool_use", "id": "t1", "name": "Bash", "input": { "command": "false" } }),
        ),
        claude_result("t1", json!("Exit code 1"), true),
        claude_assistant(
            "a2",
            json!({ "type": "tool_use", "id": "t2", "name": "Bash", "input": { "command": "sleep 99" } }),
        ),
    ];

    let history = history_of(Format::Claude, &lines);

    let tools: Vec<_> = history
        .entries
        .iter()
        .filter(|e| matches!(e.content, Content::Tool { .. }))
        .collect();
    assert_eq!(tools[0].state, ItemState::Failed);
    assert_eq!(tools[1].state, ItemState::Running);
    assert_eq!(turns(&history)[0].state, ItemState::Running);
}

#[test]
fn a_new_prompt_ends_the_turn_before_it() {
    let lines = vec![
        claude_user("u1", "one"),
        claude_assistant("a1", json!({ "type": "text", "text": "first" })),
        claude_user("u2", "two"),
        claude_assistant("a2", json!({ "type": "text", "text": "second" })),
    ];

    let history = history_of(Format::Claude, &lines);

    let turns = turns(&history);
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].state, ItemState::Completed);
    assert_eq!(turns[1].state, ItemState::Running);
}

#[test]
fn claude_lines_that_are_not_the_conversation_are_left_out() {
    let lines = vec![
        "this is not json".to_string(),
        line(json!({ "type": "file-history-snapshot", "snapshot": {} })),
        line(
            json!({ "type": "user", "isMeta": true, "uuid": "m", "message": { "content": "Caveat: ..." } }),
        ),
        line(
            json!({ "type": "user", "isSidechain": true, "uuid": "s", "message": { "content": "helper prompt" } }),
        ),
        claude_user("c", "<command-name>/clear</command-name>"),
        claude_user("lc", "<local-command-stdout></local-command-stdout>"),
        claude_user("u1", "real prompt"),
    ];

    let history = history_of(Format::Claude, &lines);

    let said: Vec<_> = items(&history)
        .into_iter()
        .map(|e| message(e).1.to_string())
        .collect();
    assert_eq!(said, ["real prompt"]);
}

#[test]
fn text_is_cleaned_and_a_huge_message_is_cut() {
    let huge = "x".repeat(MAX_MESSAGE_BYTES * 2);
    let lines = vec![
        claude_user("u1", "red \u{1b}[31mtext\u{1b}[0m"),
        claude_assistant("a1", json!({ "type": "text", "text": huge })),
    ];

    let history = history_of(Format::Claude, &lines);

    let items = items(&history);
    assert_eq!(message(items[0]).1, "red text");
    let cut = message(items[1]).1;
    assert!(cut.len() < MAX_MESSAGE_BYTES + 32 && cut.ends_with("[truncated]"));
}

#[test]
fn the_same_lines_always_give_the_same_item_ids() {
    let lines = vec![
        claude_user("u1", "hi"),
        claude_assistant("a1", json!({ "type": "text", "text": "yo" })),
    ];

    assert_eq!(
        history_of(Format::Claude, &lines),
        history_of(Format::Claude, &lines)
    );
}

fn codex(timestamp: &str, kind: &str, mut payload: serde_json::Value) -> String {
    let (outer, inner) = kind.split_once(':').unwrap();
    payload["type"] = json!(inner);
    line(json!({ "timestamp": timestamp, "type": outer, "payload": payload }))
}

#[test]
fn a_codex_turn_becomes_messages_commands_a_patch_and_a_finished_turn() {
    let at = "2026-11-04T10:00:00.000Z";
    let lines = vec![
        codex(at, "event_msg:task_started", json!({ "turn_id": "turn-a" })),
        codex(
            at,
            "event_msg:user_message",
            json!({ "message": "list files", "kind": "plain" }),
        ),
        codex(
            at,
            "response_item:message",
            json!({ "role": "user", "content": [{ "type": "input_text", "text": "<environment_context>" }] }),
        ),
        codex(at, "response_item:reasoning", json!({ "summary": [] })),
        codex(
            at,
            "response_item:message",
            json!({ "role": "assistant", "content": [{ "type": "output_text", "text": "Listing." }] }),
        ),
        codex(
            at,
            "response_item:function_call",
            json!({
                "name": "shell", "call_id": "c1",
                "arguments": json!({ "command": ["bash", "-lc", "ls -la"], "workdir": "/w" }).to_string(),
            }),
        ),
        codex(
            at,
            "response_item:function_call_output",
            json!({
                "call_id": "c1",
                "output": json!({ "output": "a\nb", "metadata": { "exit_code": 0, "duration_seconds": 0.5 } }).to_string(),
            }),
        ),
        codex(
            at,
            "response_item:function_call",
            json!({
                "name": "exec_command", "call_id": "c2", "arguments": json!({ "cmd": "false" }).to_string(),
            }),
        ),
        codex(
            at,
            "response_item:function_call_output",
            json!({
                "call_id": "c2", "output": "Exit code: 2\nWall time: 1.5 seconds\nOutput:\nboom",
            }),
        ),
        codex(
            at,
            "response_item:custom_tool_call",
            json!({
                "name": "apply_patch", "call_id": "c3",
                "input": "*** Begin Patch\n*** Update File: src/a.rs\n@@\n-old\n+new\n*** End Patch",
            }),
        ),
        codex(
            at,
            "response_item:custom_tool_call_output",
            json!({ "call_id": "c3", "output": "{\"output\":\"Success\",\"metadata\":{\"exit_code\":0}}" }),
        ),
        codex(
            "2026-11-04T10:00:09.000Z",
            "event_msg:task_complete",
            json!({ "turn_id": "turn-a" }),
        ),
    ];

    let history = history_of(Format::Codex, &lines);

    let items = items(&history);
    assert_eq!(message(items[0]), (&Role::User, "list files"));
    assert_eq!(message(items[1]), (&Role::Assistant, "Listing."));
    assert_eq!(
        items[2].content,
        Content::Tool {
            kind: "commandExecution".into(),
            output: "a\nb".into(),
            label: "ls -la".into(),
            input: None,
            exit_code: Some(0),
            duration_ms: Some(500),
        }
    );
    let Content::Tool {
        exit_code,
        duration_ms,
        output,
        ..
    } = &items[3].content
    else {
        panic!("not a tool step")
    };
    assert_eq!(
        (*exit_code, *duration_ms, output.as_str()),
        (Some(2), Some(1500), "boom")
    );
    assert_eq!(items[3].state, ItemState::Failed);
    let Content::Tool {
        kind,
        label,
        output,
        ..
    } = &items[4].content
    else {
        panic!("not a file change")
    };
    assert_eq!(
        (kind.as_str(), label.as_str()),
        ("fileChange", "Edited src/a.rs")
    );
    assert!(output.contains("+new") && output.contains("src/a.rs (update)"));
    assert_eq!(items[4].state, ItemState::Completed);
    let turns = turns(&history);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].source.turn, "turn-a");
    assert_eq!(turns[0].state, ItemState::Completed);
}

#[test]
fn an_aborted_codex_turn_is_interrupted() {
    let at = "2026-11-04T10:00:00.000Z";
    let lines = vec![
        codex(at, "event_msg:task_started", json!({ "turn_id": "turn-a" })),
        codex(at, "event_msg:user_message", json!({ "message": "go" })),
        codex(
            at,
            "event_msg:turn_aborted",
            json!({ "reason": "interrupted" }),
        ),
    ];

    assert_eq!(
        turns(&history_of(Format::Codex, &lines))[0].state,
        ItemState::Interrupted
    );
}

#[test]
fn a_codex_file_without_task_events_still_splits_turns_on_prompts() {
    let at = "2026-11-04T10:00:00.000Z";
    let lines = vec![
        codex(at, "event_msg:user_message", json!({ "message": "one" })),
        codex(at, "event_msg:user_message", json!({ "message": "two" })),
    ];

    assert_eq!(turns(&history_of(Format::Codex, &lines)).len(), 2);
}

#[test]
fn shell_wrappers_are_shown_as_the_command_a_person_typed() {
    assert_eq!(
        command_text(&json!(["bash", "-lc", "ls | wc -l"])),
        "ls | wc -l"
    );
    assert_eq!(command_text(&json!(["git", "status"])), "git status");
    assert_eq!(command_text(&json!("pwd")), "pwd");
}

#[test]
fn rfc3339_times_become_unix_milliseconds() {
    assert_eq!(iso_ms("2026-11-04T09:30:15.123Z"), Some(1_793_784_615_123));
    assert_eq!(iso_ms("2000-02-29T23:59:59Z"), Some(951_868_799_000));
    assert_eq!(iso_ms("1970-01-01T00:00:01.5Z"), Some(1500));
    assert_eq!(iso_ms("2026-11-04T09:30:15+02:00"), None);
    assert_eq!(iso_ms("yesterday"), None);
    assert_eq!(iso_ms("2026-13-04T09:30:15Z"), None);
}

/// A GitHub-token-shaped string built from pieces, so no secret-shaped literal sits in the source.
fn fake_token() -> String {
    format!("{}{}", "ghp_", "aB3d".repeat(9))
}

#[test]
fn a_followed_claude_transcript_masks_tokens_and_omits_a_secret_files_text() {
    let lines = vec![
        claude_user("u1", &format!("my token is {}", fake_token())),
        claude_assistant(
            "a1",
            json!({ "type": "tool_use", "id": "toolu_1", "name": "Write", "input": { "file_path": "/w/.env", "content": "API_KEY=abc123def456\n" } }),
        ),
        claude_result("toolu_1", json!("created"), false),
        claude_assistant(
            "a2",
            json!({ "type": "tool_use", "id": "toolu_2", "name": "Bash", "input": { "command": "echo $T" } }),
        ),
        claude_result("toolu_2", json!(format!("T={}", fake_token())), false),
    ];
    let history = history_of(Format::Claude, &lines);
    let items = items(&history);
    assert_eq!(message(items[0]), (&Role::User, "my token is [redacted]"));
    let Content::Tool { input: Some(input), .. } = &items[1].content else {
        panic!("a write")
    };
    assert_eq!(input["file_path"], "/w/.env");
    assert_eq!(input["content"], "[content of a secret file omitted]");
    let Content::Tool { output, .. } = &items[2].content else {
        panic!("a command")
    };
    assert_eq!(output, "T=[redacted]");
}

#[test]
fn a_followed_codex_rollout_masks_tokens_and_omits_a_secret_patch() {
    let at = "2026-11-04T10:00:00.000Z";
    let lines = vec![
        codex(at, "event_msg:task_started", json!({ "turn_id": "turn-a" })),
        codex(
            at,
            "response_item:custom_tool_call",
            json!({
                "name": "apply_patch", "call_id": "c1",
                "input": "*** Begin Patch\n*** Add File: .env\n+DB_PASSWORD=hunter2hunter2\n*** End Patch",
            }),
        ),
        codex(
            at,
            "response_item:custom_tool_call_output",
            json!({ "call_id": "c1", "output": "{\"output\":\"Success\",\"metadata\":{\"exit_code\":0}}" }),
        ),
        codex(
            at,
            "response_item:function_call",
            json!({ "name": "exec_command", "call_id": "c2", "arguments": json!({ "cmd": "env" }).to_string() }),
        ),
        codex(
            at,
            "response_item:function_call_output",
            json!({ "call_id": "c2", "output": format!("Exit code: 0\nWall time: 1 seconds\nOutput:\nGH={}", fake_token()) }),
        ),
    ];
    let history = history_of(Format::Codex, &lines);
    let items = items(&history);
    let Content::Tool { output, .. } = &items[0].content else {
        panic!("a patch")
    };
    assert!(output.contains(".env (add)"));
    assert!(output.contains("[content of a secret file omitted]"));
    assert!(!output.contains("hunter2"));
    let Content::Tool { output, .. } = &items[1].content else {
        panic!("a command")
    };
    assert_eq!(output, "GH=[redacted]");
}
