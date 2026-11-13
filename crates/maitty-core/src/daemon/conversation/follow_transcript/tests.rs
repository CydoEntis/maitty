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
        claude_user("r", "<system-reminder>\nread the instructions\n</system-reminder>"),
        claude_user(
            "cv",
            "<local-command-caveat>The command below was run directly.</local-command-caveat>",
        ),
        claude_user("c", "<command-name>/clear</command-name>"),
        claude_user("lc", "<local-command-stdout></local-command-stdout>"),
        claude_user("u1", "real prompt"),
    ];

    let history = history_of(Format::Claude, &lines);

    // A command that printed nothing is the command alone: no row for its empty output.
    let said: Vec<_> = items(&history)
        .into_iter()
        .map(|e| message(e).1.to_string())
        .collect();
    assert_eq!(said, ["/clear", "real prompt"]);
}

const SLASH_COMMAND: &str = "<command-name>/plugin</command-name>\n            <command-message>plugin</command-message>\n            <command-args>marketplace add CydoEntis/Kitchen</command-args>";
const SLASH_OUTPUT: &str =
    "<local-command-stdout>Added the \u{1b}[1mKitchen\u{1b}[0m marketplace</local-command-stdout>";

fn claude_local_command(uuid: &str, text: &str) -> String {
    line(json!({
        "type": "system", "subtype": "local_command", "uuid": uuid, "isMeta": false,
        "timestamp": "2026-11-04T09:30:00.000Z", "content": text,
    }))
}

fn assert_slash_command_and_output(history: &History) {
    let items = items(history);
    assert_eq!(items.len(), 2);
    assert_eq!(
        message(items[0]),
        (&Role::User, "/plugin marketplace add CydoEntis/Kitchen")
    );
    assert_eq!(
        items[1].content,
        Content::Tool {
            kind: "commandExecution".into(),
            label: "/plugin marketplace add CydoEntis/Kitchen".into(),
            output: "Added the Kitchen marketplace".into(),
            input: None,
            exit_code: None,
            duration_ms: None,
        }
    );
    assert_eq!(items[1].state, ItemState::Completed);
    // Nothing answers a command Claude ran itself, so its turn is over once it has printed.
    let turns = turns(history);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].state, ItemState::Completed);
}

#[test]
fn a_claude_slash_command_is_a_sent_message_with_its_output_beneath() {
    let lines = vec![
        claude_user("c", SLASH_COMMAND),
        claude_user("o", SLASH_OUTPUT),
    ];

    assert_slash_command_and_output(&history_of(Format::Claude, &lines));
}

#[test]
fn a_claude_slash_command_written_as_a_system_line_shows_the_same() {
    let lines = vec![
        claude_local_command("c", SLASH_COMMAND),
        claude_local_command("o", SLASH_OUTPUT),
    ];

    assert_slash_command_and_output(&history_of(Format::Claude, &lines));
}

#[test]
fn what_a_claude_slash_command_printed_as_an_error_is_shown_too() {
    let lines = vec![
        claude_user("c", SLASH_COMMAND),
        claude_user(
            "o",
            "<local-command-stderr>Marketplace not found</local-command-stderr>",
        ),
    ];

    let history = history_of(Format::Claude, &lines);

    let Content::Tool { output, .. } = &items(&history)[1].content else {
        panic!("not a tool step")
    };
    assert_eq!(output, "Marketplace not found");
}

#[test]
fn the_same_claude_slash_command_run_twice_shows_twice() {
    let lines = vec![
        claude_user("c1", "<command-name>/clear</command-name>"),
        claude_user("o1", "<local-command-stdout></local-command-stdout>"),
        claude_user("c2", "<command-name>/clear</command-name>"),
        claude_user("o2", "<local-command-stdout></local-command-stdout>"),
    ];

    let history = history_of(Format::Claude, &lines);

    let said: Vec<_> = items(&history)
        .into_iter()
        .map(|e| message(e).1.to_string())
        .collect();
    assert_eq!(said, ["/clear", "/clear"]);
}

#[test]
fn a_claude_command_that_prompts_the_agent_shows_the_command_and_its_answer() {
    let lines = vec![
        claude_user(
            "c",
            "<command-message>statusline</command-message>\n<command-name>/statusline</command-name>\n<command-args>from my shell</command-args>",
        ),
        line(
            json!({ "type": "user", "isMeta": true, "uuid": "m", "message": { "content": [{ "type": "text", "text": "Create an Agent" }] } }),
        ),
        claude_assistant("a1", json!({ "type": "text", "text": "Configured." })),
    ];

    let history = history_of(Format::Claude, &lines);

    let items = items(&history);
    assert_eq!(message(items[0]), (&Role::User, "/statusline from my shell"));
    assert_eq!(message(items[1]), (&Role::Assistant, "Configured."));
    assert_eq!(turns(&history)[0].state, ItemState::Running);
}

fn claude_at(kind: &str, uuid: &str, timestamp: &str, content: serde_json::Value) -> String {
    line(json!({
        "type": kind, "uuid": uuid, "isSidechain": false, "timestamp": timestamp,
        "message": { "role": kind, "content": content },
    }))
}

#[test]
fn a_command_run_while_the_agent_works_does_not_end_its_turn_early() {
    // Claude writes such a command after the turn's last reply, stamped with when it was typed.
    let lines = vec![
        claude_at("user", "u1", "2026-11-04T09:30:00.000Z", json!("go")),
        claude_at(
            "assistant",
            "a1",
            "2026-11-04T09:30:40.000Z",
            json!([{ "type": "text", "text": "Done." }]),
        ),
        claude_at("user", "c", "2026-11-04T09:30:10.000Z", json!(SLASH_COMMAND)),
        claude_at("user", "o", "2026-11-04T09:30:10.000Z", json!(SLASH_OUTPUT)),
        line(
            json!({ "type": "system", "subtype": "turn_duration", "durationMs": 40000, "timestamp": "2026-11-04T09:30:40.000Z" }),
        ),
    ];

    let history = history_of(Format::Claude, &lines);

    let turns = turns(&history);
    assert_eq!(turns.len(), 2);
    assert_eq!(turns[0].state, ItemState::Completed);
    assert_eq!(turns[0].started_at_ms, Some(1_793_784_600_000));
    assert_eq!(turns[0].completed_at_ms, Some(1_793_784_640_000));
    assert_eq!(turns[1].started_at_ms, Some(1_793_784_610_000));
    assert_eq!(turns[1].completed_at_ms, Some(1_793_784_610_000));
}

#[test]
fn command_output_whose_command_was_not_read_leaves_the_agents_turn_running() {
    let lines = vec![
        claude_user("u1", "go"),
        claude_assistant(
            "a1",
            json!({ "type": "tool_use", "id": "t1", "name": "Bash", "input": { "command": "sleep 99" } }),
        ),
        claude_user("o", SLASH_OUTPUT),
    ];

    let history = history_of(Format::Claude, &lines);

    let turns = turns(&history);
    assert_eq!(turns.len(), 1);
    assert_eq!(turns[0].state, ItemState::Running);
    let Content::Tool { output, .. } = &items(&history)[2].content else {
        panic!("not a tool step")
    };
    assert_eq!(output, "Added the Kitchen marketplace");
}

#[test]
fn a_claude_system_line_that_is_no_command_is_not_something_said() {
    let lines = vec![
        claude_local_command("s", "words Claude wrote, which no one typed"),
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
fn a_claude_transcript_says_its_permission_mode_and_the_model_answering() {
    let mut transcript = Transcript::new(Format::Claude, SESSION);
    let feed = |transcript: &mut Transcript, value: serde_json::Value| {
        transcript.feed(0, &line(value));
    };

    feed(&mut transcript, json!({ "type": "user", "uuid": "u1", "permissionMode": "auto", "isSidechain": false,
        "timestamp": "2026-11-06T10:00:00.000Z", "message": { "role": "user", "content": "hi" } }));
    assert_eq!(transcript.meta().mode.as_deref(), Some("auto"));
    assert_eq!(transcript.meta().model, None, "no assistant message yet: no model said");

    feed(&mut transcript, json!({ "type": "assistant", "uuid": "a1", "isSidechain": false,
        "timestamp": "2026-11-06T10:00:01.000Z",
        "message": { "role": "assistant", "model": "claude-sonnet-5-5", "content": [{ "type": "text", "text": "hello" }] } }));
    assert_eq!(transcript.meta().model.as_deref(), Some("claude-sonnet-5-5"));

    // A message Claude made itself names a placeholder, which is not a model, and a helper's lines are not this session's.
    feed(&mut transcript, json!({ "type": "assistant", "uuid": "a2", "isSidechain": false,
        "timestamp": "2026-11-06T10:00:02.000Z",
        "message": { "role": "assistant", "model": "<synthetic>", "content": [{ "type": "text", "text": "x" }] } }));
    feed(&mut transcript, json!({ "type": "user", "uuid": "u2", "permissionMode": "plan", "isSidechain": true,
        "timestamp": "2026-11-06T10:00:03.000Z", "message": { "role": "user", "content": "helper" } }));
    assert_eq!(transcript.meta().model.as_deref(), Some("claude-sonnet-5-5"));
    assert_eq!(transcript.meta().mode.as_deref(), Some("auto"));

    // The mode changes when the person changes it: the latest word stands.
    feed(&mut transcript, json!({ "type": "user", "uuid": "u3", "permissionMode": "plan", "isSidechain": false,
        "timestamp": "2026-11-06T10:01:00.000Z", "message": { "role": "user", "content": "again" } }));
    assert_eq!(transcript.meta().mode.as_deref(), Some("plan"));
}

/// Codex 0.156 writes the person's words as a user-role response item, with no `user_message` event.
#[test]
fn a_codex_user_message_written_only_as_a_response_item_is_shown_and_its_injected_context_is_not() {
    let at = "2026-11-06T11:17:51.000Z";
    let lines = vec![
        line(json!({ "timestamp": at, "type": "session_meta", "payload": { "id": SESSION } })),
        codex(
            at,
            "response_item:message",
            json!({ "role": "developer", "content": [{ "type": "input_text", "text": "<permissions instructions>" }] }),
        ),
        codex(
            at,
            "response_item:message",
            json!({ "role": "user", "content": [
                { "type": "input_text", "text": "# AGENTS.md instructions for /w" },
                { "type": "input_text", "text": "<environment_context><cwd>/w</cwd></environment_context>" },
            ] }),
        ),
        codex(at, "event_msg:task_started", json!({ "turn_id": "turn-a" })),
        codex(
            at,
            "response_item:message",
            json!({ "role": "user", "content": [{ "type": "input_text", "text": "Hey" }] }),
        ),
        codex(at, "event_msg:item_completed", json!({ "item": { "type": "UserMessage", "content": [] } })),
        codex(
            at,
            "response_item:message",
            json!({ "role": "assistant", "content": [{ "type": "output_text", "text": "Hey! What can I help you with?" }] }),
        ),
        codex(at, "event_msg:task_complete", json!({ "turn_id": "turn-a" })),
        codex(at, "event_msg:task_started", json!({ "turn_id": "turn-b" })),
        codex(
            at,
            "response_item:message",
            json!({ "role": "user", "content": [{ "type": "input_text", "text": "Hey" }] }),
        ),
        codex(
            at,
            "response_item:message",
            json!({ "role": "assistant", "content": [{ "type": "output_text", "text": "Again." }] }),
        ),
        codex(at, "event_msg:task_complete", json!({ "turn_id": "turn-b" })),
    ];

    let history = history_of(Format::Codex, &lines);

    let said: Vec<_> = history
        .entries
        .iter()
        .filter(|e| matches!(e.content, Content::Message { .. }))
        .map(|e| {
            let (role, text) = message(e);
            (role.clone(), text.to_string())
        })
        .collect();
    assert_eq!(
        said,
        [
            (Role::User, "Hey".to_string()),
            (Role::Assistant, "Hey! What can I help you with?".to_string()),
            // The same words in a later turn are a new message, not a repeat.
            (Role::User, "Hey".to_string()),
            (Role::Assistant, "Again.".to_string()),
        ]
    );
}

/// An older Codex wrote both the event and the response item for one message: shown once.
#[test]
fn a_codex_user_message_written_as_an_event_and_a_response_item_is_shown_once() {
    let at = "2026-11-04T10:00:00.000Z";
    let lines = vec![
        codex(at, "event_msg:task_started", json!({ "turn_id": "turn-a" })),
        codex(at, "event_msg:user_message", json!({ "message": "list files" })),
        codex(
            at,
            "response_item:message",
            json!({ "role": "user", "content": [{ "type": "input_text", "text": "list files" }] }),
        ),
        codex(at, "event_msg:task_complete", json!({ "turn_id": "turn-a" })),
    ];

    let history = history_of(Format::Codex, &lines);

    let users = history
        .entries
        .iter()
        .filter(|e| matches!(&e.content, Content::Message { role, .. } if *role == Role::User))
        .count();
    assert_eq!(users, 1);
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

// ---- Gemini: synthetic lines in the shape of its `session-….jsonl`; no real conversation ----

fn gemini_user(id: &str, text: &str) -> String {
    line(json!({
        "id": id, "timestamp": "2026-11-04T09:30:00.000Z", "type": "user",
        "content": [{ "text": text }],
    }))
}

fn gemini_reply(id: &str, text: &str, tools: serde_json::Value) -> String {
    let mut value = json!({
        "id": id, "timestamp": "2026-11-04T09:30:01.000Z", "type": "gemini",
        "content": text, "thoughts": [], "model": "gemini-synthetic",
    });
    if !tools.is_null() {
        value["toolCalls"] = tools;
    }
    line(value)
}

#[test]
fn gemini_messages_and_a_finished_tool_call_become_a_conversation() {
    let lines = vec![
        line(json!({ "sessionId": SESSION, "projectHash": "h", "startTime": "2026-11-04T09:29:00.000Z", "kind": "main" })),
        gemini_user("u1", "list the files"),
        gemini_reply(
            "g1",
            "Looking.",
            json!([{
                "id": "call-1", "name": "run_shell_command",
                "args": { "command": "ls" }, "status": "success",
                "resultDisplay": "a.txt
b.txt",
                "result": [{ "functionResponse": { "response": { "output": "ignored when shown" } } }],
            }]),
        ),
        line(json!({ "$set": { "lastUpdated": "2026-11-04T09:30:02.000Z" } })),
        gemini_reply("g2", "Two files.", serde_json::Value::Null),
    ];

    let history = history_of(Format::Gemini, &lines);

    let items = items(&history);
    assert_eq!(message(items[0]), (&Role::User, "list the files"));
    assert_eq!(message(items[1]), (&Role::Assistant, "Looking."));
    assert_eq!(
        items[2].content,
        Content::Tool {
            kind: "commandExecution".into(),
            output: "a.txt
b.txt".into(),
            label: "ls".into(),
            input: Some(json!({ "command": "ls" })),
            exit_code: None,
            duration_ms: None,
        }
    );
    assert_eq!(items[2].state, ItemState::Completed);
    assert_eq!(message(items[3]), (&Role::Assistant, "Two files."));
    assert_eq!(turns(&history).len(), 1, "one user message, one turn");
}

#[test]
fn a_gemini_tool_call_that_failed_is_shown_failed_and_its_output_comes_from_the_response() {
    let lines = vec![
        gemini_user("u1", "run it"),
        gemini_reply(
            "g1",
            "",
            json!([{
                "id": "call-1", "name": "read_file", "args": { "path": "x" }, "status": "error",
                "result": [{ "functionResponse": { "response": { "output": "no such file" } } }],
            }]),
        ),
    ];

    let history = history_of(Format::Gemini, &lines);

    let items = items(&history);
    let tool = items.iter().find(|e| matches!(e.content, Content::Tool { .. })).unwrap();
    assert_eq!(tool.state, ItemState::Failed);
    let Content::Tool { label, output, .. } = &tool.content else { unreachable!() };
    assert_eq!(label, "read_file ");
    assert_eq!(output, "no such file");
}

#[test]
fn gemini_info_and_error_lines_and_a_second_user_message_are_handled() {
    let lines = vec![
        gemini_user("u1", "first"),
        line(json!({ "id": "i1", "timestamp": "2026-11-04T09:30:00.500Z", "type": "info", "content": "Switched model" })),
        gemini_reply("g1", "one", serde_json::Value::Null),
        line(json!({ "id": "e1", "timestamp": "2026-11-04T09:30:01.500Z", "type": "error", "content": "quota" })),
        gemini_user("u2", "second"),
        gemini_reply("g2", "two", serde_json::Value::Null),
    ];

    let history = history_of(Format::Gemini, &lines);

    let said: Vec<&str> = items(&history).iter().map(|e| message(e).1).collect();
    assert_eq!(said, ["first", "one", "second", "two"]);
    assert_eq!(turns(&history).len(), 2);
}

#[test]
fn the_markers_claude_puts_around_a_paste_are_not_shown_as_said() {
    let said = "<pasted_content id=\"aa4a\">\nErrors occurred\nretry it\n</pasted_content id=\"aa4a\">";
    assert_eq!(without_paste_tags(said), "Errors occurred\nretry it\n");
    assert_eq!(without_paste_tags("plain words"), "plain words");
    assert_eq!(without_paste_tags("keep <pasted_content oops"), "keep <pasted_content oops");
}

#[test]
fn a_codex_custom_tools_output_in_text_parts_is_shown_without_its_header() {
    let at = "2026-11-13T19:44:40.000Z";
    let lines = [
        codex(at, "event_msg:task_started", json!({ "turn_id": "turn-a" })),
        codex(
            at,
            "response_item:custom_tool_call",
            json!({ "name": "exec", "call_id": "c1", "input": "cat a.txt" }),
        ),
        // As Codex 0.166 writes it: the output is a list of text parts, a header then the output.
        codex(
            at,
            "response_item:custom_tool_call_output",
            json!({ "call_id": "c1", "output": [
                { "type": "input_text", "text": "Script completed\nWall time 0.2 seconds\nOutput:\n" },
                { "type": "input_text", "text": "alpha\n" },
            ] }),
        ),
    ];
    let history = history_of(Format::Codex, &lines);
    let tool = items(&history)
        .into_iter()
        .find_map(|item| match &item.content {
            Content::Tool { output, duration_ms, .. } => Some((output.clone(), *duration_ms)),
            _ => None,
        })
        .expect("the tool call is shown");
    assert_eq!(tool, ("alpha\n".to_string(), Some(200)));
}
