//! Turns the lines of an agent's own transcript (Claude's `<session>.jsonl`, Codex's
//! `rollout-….jsonl`) into the provider notifications the structured runtimes produce, so the
//! history of an agent running in a pane goes through the same projection as a conversation the
//! daemon started (`history_provider::notification`).
//!
//! Pure: lines in, notes out. A line that is not understood gives no note; nothing is guessed.
//! Every string that reaches a note is cleaned and bounded here or by that projection, because a
//! transcript holds whatever the agent was working on.

use super::tool_input::{bound_input, clean_text};
use maitty_provider_claude::bounded_content_text;
use serde_json::{Value, json};
use std::collections::HashMap;

/// Longest message kept whole. A page of history may not exceed `MAX_READ_BYTES`, and one pasted
/// log must not make every read fail.
const MAX_MESSAGE_BYTES: usize = 64 * 1024;
/// Largest output kept for one Codex tool call (Claude's goes through `bounded_content_text`).
const MAX_OUTPUT_BYTES: usize = 8 * 1024;
/// Unfinished tool calls remembered to pair them with their results.
const MAX_PENDING_TOOLS: usize = 256;
const TRUNCATED: &str = "\n[truncated]";

/// One provider notification, as `history_provider::notification` reads it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::daemon) struct Note {
    pub method: &'static str,
    pub params: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::daemon) enum Format {
    Claude,
    Codex,
}

struct PendingTool {
    command: String,
    input: Option<Value>,
    /// Codex `apply_patch`: the files it changes, to complete the file-change item.
    changes: Option<Value>,
}

/// The state needed to read a transcript line by line.
pub(in crate::daemon) struct Transcript {
    format: Format,
    thread: String,
    turns: u64,
    open_turn: Option<String>,
    /// The turn was announced by the provider (Codex `task_started`), so a user message inside it
    /// does not start another one.
    explicit_turn: bool,
    /// The last user message of the open turn, so one Codex writes twice (as an event and as a response
    /// item) is shown once. A new turn, and any agent message, clears it: the same words later are new.
    last_user_text: Option<String>,
    tools: HashMap<String, PendingTool>,
}

impl Transcript {
    pub fn new(format: Format, thread: &str) -> Self {
        Self {
            format,
            thread: thread.to_owned(),
            turns: 0,
            open_turn: None,
            explicit_turn: false,
            last_user_text: None,
            tools: HashMap::new(),
        }
    }

    /// The notes one line gives. `offset` is where the line starts in the file: item ids that the
    /// provider did not number are made from it, so reading the file again gives the same ids.
    pub fn feed(&mut self, offset: u64, line: &str) -> Vec<Note> {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            return Vec::new();
        };
        let mut notes = Vec::new();
        match self.format {
            Format::Claude => self.claude(offset, &value, &mut notes),
            Format::Codex => self.codex(offset, &value, &mut notes),
        }
        notes
    }

    fn note(&self, method: &'static str, mut params: Value) -> Note {
        params["threadId"] = Value::String(self.thread.clone());
        Note { method, params }
    }

    fn begin_turn(&mut self, id: Option<&str>, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        self.close_turn("completed", at_ms, notes);
        self.last_user_text = None;
        self.turns += 1;
        let id = id.map_or_else(|| format!("turn-{}", self.turns), str::to_owned);
        let mut turn = json!({ "id": id, "status": "inProgress" });
        if let Some(ms) = at_ms {
            turn["startedAt"] = json!(ms / 1000);
        }
        notes.push(self.note("turn/started", json!({ "turn": turn })));
        self.open_turn = Some(id);
        self.explicit_turn = false;
    }

    fn close_turn(&mut self, status: &str, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        let Some(id) = self.open_turn.take() else {
            return;
        };
        let mut turn = json!({ "id": id, "status": status });
        if let Some(ms) = at_ms {
            turn["completedAt"] = json!(ms / 1000);
        }
        notes.push(self.note("turn/completed", json!({ "turn": turn })));
        self.explicit_turn = false;
    }

    fn turn_id(&mut self, at_ms: Option<u64>, notes: &mut Vec<Note>) -> String {
        if self.open_turn.is_none() {
            self.begin_turn(None, at_ms, notes);
        }
        self.open_turn.clone().unwrap_or_default()
    }

    fn user_message(&mut self, id: &str, text: &str, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        if text.trim().is_empty() || self.last_user_text.as_deref() == Some(text.trim()) {
            return;
        }
        if self.open_turn.is_none() || !self.explicit_turn {
            self.begin_turn(None, at_ms, notes);
        }
        self.last_user_text = Some(text.trim().to_owned());
        let turn = self.turn_id(at_ms, notes);
        let item = json!({
            "id": id,
            "type": "userMessage",
            "content": [{ "type": "text", "text": bounded_text(text) }],
        });
        notes.push(self.note("item/completed", item_params(&turn, item, at_ms)));
    }

    fn agent_message(&mut self, id: &str, text: &str, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        if text.trim().is_empty() {
            return;
        }
        self.last_user_text = None;
        let turn = self.turn_id(at_ms, notes);
        let item = json!({ "id": id, "type": "agentMessage", "text": bounded_text(text) });
        notes.push(self.note("item/completed", item_params(&turn, item, at_ms)));
    }

    fn remember(&mut self, id: &str, tool: PendingTool) {
        if self.tools.len() >= MAX_PENDING_TOOLS {
            self.tools.clear();
        }
        self.tools.insert(id.to_owned(), tool);
    }

    // ---- Claude: one JSON object per content block ----

    fn claude(&mut self, offset: u64, line: &Value, notes: &mut Vec<Note>) {
        // A helper agent's own conversation is a separate file in current Claude, and an older
        // one wrote it into this file marked as a side chain; it is not this conversation.
        if line.get("isSidechain").and_then(Value::as_bool) == Some(true) {
            return;
        }
        let at_ms = line
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(iso_ms);
        let uuid = line
            .get("uuid")
            .and_then(Value::as_str)
            .map_or_else(|| format!("o{offset}"), str::to_owned);
        match line.get("type").and_then(Value::as_str) {
            Some("user") => {
                if line.get("isMeta").and_then(Value::as_bool) == Some(true) {
                    return;
                }
                let Some(content) = line.pointer("/message/content") else {
                    return;
                };
                match content {
                    Value::String(text) => {
                        if !is_claude_chrome(text) {
                            self.user_message(&uuid, text, at_ms, notes);
                        }
                    }
                    Value::Array(blocks) => self.claude_user_blocks(&uuid, blocks, at_ms, notes),
                    _ => {}
                }
            }
            Some("assistant") => {
                let Some(blocks) = line.pointer("/message/content").and_then(Value::as_array)
                else {
                    return;
                };
                for (index, block) in blocks.iter().enumerate() {
                    self.claude_assistant_block(&format!("{uuid}:{index}"), block, at_ms, notes);
                }
            }
            Some("system")
                if line.get("subtype").and_then(Value::as_str) == Some("turn_duration") =>
            {
                self.close_turn("completed", at_ms, notes);
            }
            _ => {}
        }
    }

    fn claude_user_blocks(
        &mut self,
        uuid: &str,
        blocks: &[Value],
        at_ms: Option<u64>,
        notes: &mut Vec<Note>,
    ) {
        let mut text = String::new();
        for block in blocks {
            match block.get("type").and_then(Value::as_str) {
                Some("tool_result") => self.claude_result(block, at_ms, notes),
                Some("text") => {
                    if let Some(part) = block.get("text").and_then(Value::as_str) {
                        text.push_str(part);
                    }
                }
                _ => {}
            }
        }
        if !text.is_empty() && !is_claude_chrome(&text) {
            self.user_message(uuid, &text, at_ms, notes);
        }
    }

    fn claude_assistant_block(
        &mut self,
        id: &str,
        block: &Value,
        at_ms: Option<u64>,
        notes: &mut Vec<Note>,
    ) {
        match block.get("type").and_then(Value::as_str) {
            Some("text") => {
                if let Some(text) = block.get("text").and_then(Value::as_str) {
                    self.agent_message(id, text, at_ms, notes);
                }
            }
            Some("tool_use") => {
                let (Some(tool_id), Some(name)) = (
                    block.get("id").and_then(Value::as_str),
                    block.get("name").and_then(Value::as_str),
                ) else {
                    return;
                };
                let input = block.get("input").cloned().unwrap_or(Value::Null);
                // The structured adapter's label: the command, else the tool's name and a space.
                let command = input
                    .get("command")
                    .and_then(Value::as_str)
                    .map_or_else(|| format!("{name} "), str::to_owned);
                let turn = self.turn_id(at_ms, notes);
                let item = json!({
                    "id": tool_id,
                    "type": "commandExecution",
                    "command": command,
                    "input": input,
                    "status": "inProgress",
                    "aggregatedOutput": "",
                });
                notes.push(self.note("item/started", item_params(&turn, item, at_ms)));
                self.remember(
                    tool_id,
                    PendingTool {
                        command,
                        input: Some(input),
                        changes: None,
                    },
                );
            }
            _ => {}
        }
    }

    fn claude_result(&mut self, block: &Value, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        let Some(tool_id) = block.get("tool_use_id").and_then(Value::as_str) else {
            return;
        };
        let Some(tool) = self.tools.remove(tool_id) else {
            return;
        };
        let failed = block.get("is_error").and_then(Value::as_bool) == Some(true);
        let turn = self.turn_id(at_ms, notes);
        let item = json!({
            "id": tool_id,
            "type": "commandExecution",
            "command": tool.command,
            "input": tool.input,
            "status": if failed { "failed" } else { "completed" },
            "aggregatedOutput": bounded_content_text(block.get("content")),
        });
        notes.push(self.note("item/completed", item_params(&turn, item, at_ms)));
    }

    // ---- Codex: response items and event messages ----

    fn codex(&mut self, offset: u64, line: &Value, notes: &mut Vec<Note>) {
        let at_ms = line
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(iso_ms);
        let Some(payload) = line.get("payload") else {
            return;
        };
        let kind = payload.get("type").and_then(Value::as_str);
        match (line.get("type").and_then(Value::as_str), kind) {
            (Some("event_msg"), Some("task_started")) => {
                let id = payload.get("turn_id").and_then(Value::as_str);
                self.begin_turn(id, at_ms, notes);
                self.explicit_turn = true;
            }
            (Some("event_msg"), Some("task_complete")) => {
                self.close_turn("completed", at_ms, notes);
            }
            (Some("event_msg"), Some("turn_aborted")) => {
                self.close_turn("interrupted", at_ms, notes);
            }
            (Some("event_msg"), Some("user_message")) => {
                if let Some(text) = payload.get("message").and_then(Value::as_str) {
                    self.user_message(&format!("o{offset}"), text, at_ms, notes);
                }
            }
            (Some("response_item"), Some("message")) => {
                match payload.get("role").and_then(Value::as_str) {
                    Some("assistant") => {
                        self.agent_message(&format!("o{offset}"), &codex_text(payload), at_ms, notes);
                    }
                    // Codex 0.156 writes the person's words only here. What it injects as a user message
                    // (the project's instructions, the environment) is not the person's.
                    Some("user") if !is_codex_context(payload) => {
                        self.user_message(&format!("o{offset}"), &codex_text(payload), at_ms, notes);
                    }
                    _ => {}
                }
            }
            (Some("response_item"), Some("function_call" | "local_shell_call")) => {
                self.codex_call(payload, at_ms, notes);
            }
            (Some("response_item"), Some("custom_tool_call")) => {
                self.codex_patch(payload, at_ms, notes);
            }
            (Some("response_item"), Some("function_call_output" | "custom_tool_call_output")) => {
                self.codex_output(payload, at_ms, notes)
            }
            _ => {}
        }
    }

    fn codex_call(&mut self, payload: &Value, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        let Some(call_id) = payload.get("call_id").and_then(Value::as_str) else {
            return;
        };
        let command = if payload.get("type").and_then(Value::as_str) == Some("local_shell_call") {
            payload
                .pointer("/action/command")
                .map(command_text)
                .unwrap_or_default()
        } else {
            let arguments = payload
                .get("arguments")
                .and_then(Value::as_str)
                .and_then(|text| serde_json::from_str::<Value>(text).ok())
                .unwrap_or(Value::Null);
            match arguments.get("command").or_else(|| arguments.get("cmd")) {
                Some(command) => command_text(command),
                None => payload
                    .get("name")
                    .and_then(Value::as_str)
                    .unwrap_or("Tool")
                    .to_owned(),
            }
        };
        let turn = self.turn_id(at_ms, notes);
        let item = json!({
            "id": call_id,
            "type": "commandExecution",
            "command": command,
            "status": "inProgress",
            "aggregatedOutput": "",
        });
        notes.push(self.note("item/started", item_params(&turn, item, at_ms)));
        self.remember(
            call_id,
            PendingTool {
                command,
                input: None,
                changes: None,
            },
        );
    }

    fn codex_patch(&mut self, payload: &Value, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        let Some(call_id) = payload.get("call_id").and_then(Value::as_str) else {
            return;
        };
        let name = payload
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("Tool");
        let input = payload
            .get("input")
            .and_then(Value::as_str)
            .unwrap_or_default();
        if name != "apply_patch" {
            let item = json!({
                "id": call_id,
                "type": "commandExecution",
                "command": name,
                "status": "inProgress",
                "aggregatedOutput": "",
            });
            let turn = self.turn_id(at_ms, notes);
            notes.push(self.note("item/started", item_params(&turn, item, at_ms)));
            self.remember(
                call_id,
                PendingTool {
                    command: name.to_owned(),
                    input: None,
                    changes: None,
                },
            );
            return;
        }
        let changes = Value::Array(patch_changes(input));
        let turn = self.turn_id(at_ms, notes);
        let item = json!({
            "id": call_id,
            "type": "fileChange",
            "status": "inProgress",
            "changes": changes,
        });
        notes.push(self.note("item/started", item_params(&turn, item, at_ms)));
        self.remember(
            call_id,
            PendingTool {
                command: String::new(),
                input: None,
                changes: Some(changes),
            },
        );
    }

    fn codex_output(&mut self, payload: &Value, at_ms: Option<u64>, notes: &mut Vec<Note>) {
        let Some(call_id) = payload.get("call_id").and_then(Value::as_str) else {
            return;
        };
        let Some(tool) = self.tools.remove(call_id) else {
            return;
        };
        let raw = payload
            .get("output")
            .map_or_else(String::new, |output| match output {
                Value::String(text) => text.clone(),
                other => other
                    .get("content")
                    .and_then(Value::as_str)
                    .unwrap_or_default()
                    .to_owned(),
            });
        let result = codex_result(&raw);
        let status = if result.exit_code.is_some_and(|code| code != 0) {
            "failed"
        } else {
            "completed"
        };
        let turn = self.turn_id(at_ms, notes);
        let item = match tool.changes {
            Some(changes) => json!({
                "id": call_id,
                "type": "fileChange",
                "status": status,
                "changes": changes,
            }),
            None => {
                let mut item = json!({
                    "id": call_id,
                    "type": "commandExecution",
                    "command": tool.command,
                    "status": status,
                    "aggregatedOutput": result.text,
                });
                if let Some(code) = result.exit_code {
                    item["exitCode"] = json!(code);
                }
                if let Some(ms) = result.duration_ms {
                    item["durationMs"] = json!(ms);
                }
                item
            }
        };
        notes.push(self.note("item/completed", item_params(&turn, item, at_ms)));
    }
}

fn item_params(turn: &str, mut item: Value, at_ms: Option<u64>) -> Value {
    // The projection bounds an object input itself; a command's input is the only one here.
    if let Some(input) = item.get("input").and_then(bound_input) {
        item["input"] = input;
    } else if let Some(object) = item.as_object_mut() {
        object.remove("input");
    }
    let mut params = json!({ "turnId": turn, "item": item });
    if let Some(ms) = at_ms {
        params["startedAtMs"] = json!(ms);
    }
    params
}

/// What Claude writes into the transcript as a "user" line that no one typed: slash-command
/// echoes and local command output.
fn is_claude_chrome(text: &str) -> bool {
    let text = text.trim_start();
    [
        "<command-name>",
        "<local-command-",
        "<system-reminder>",
        "<command-message>",
    ]
    .iter()
    .any(|tag| text.starts_with(tag))
}

/// What opens a user-role message that Codex adds itself, not the person.
const CODEX_CONTEXT_PREFIXES: [&str; 3] = ["# AGENTS.md instructions", "<environment_context>", "<user_instructions>"];

/// Whether a user-role response item is context Codex injected, which no chat should show as said.
fn is_codex_context(payload: &Value) -> bool {
    payload.get("content").and_then(Value::as_array).is_some_and(|parts| {
        parts.iter().filter_map(|part| part.get("text").and_then(Value::as_str)).any(|text| {
            let text = text.trim_start();
            CODEX_CONTEXT_PREFIXES.iter().any(|prefix| text.starts_with(prefix))
        })
    })
}

fn codex_text(payload: &Value) -> String {
    payload
        .get("content")
        .and_then(Value::as_array)
        .map(|parts| {
            parts
                .iter()
                .filter_map(|part| part.get("text").and_then(Value::as_str))
                .collect::<String>()
        })
        .unwrap_or_default()
}

/// A command given as an argument list or a string, as a person would type it: `bash -lc "x"`
/// is shown as `x`.
fn command_text(command: &Value) -> String {
    match command {
        Value::String(text) => text.clone(),
        Value::Array(parts) => {
            let words: Vec<&str> = parts.iter().filter_map(Value::as_str).collect();
            match words.as_slice() {
                [_, "-lc" | "-c", script] => (*script).to_owned(),
                _ => words.join(" "),
            }
        }
        _ => String::new(),
    }
}

struct CodexResult {
    text: String,
    exit_code: Option<i64>,
    duration_ms: Option<u64>,
}

/// Codex writes a command's result either as JSON (`{"output", "metadata": {"exit_code",
/// "duration_seconds"}}`) or as text (`Exit code: N`, `Wall time: S seconds`, `Output:`, …).
fn codex_result(raw: &str) -> CodexResult {
    let seconds = |value: f64| (value >= 0.0).then_some((value * 1000.0) as u64);
    if let Ok(json) = serde_json::from_str::<Value>(raw)
        && json.is_object()
    {
        return CodexResult {
            text: bounded_output(
                json.get("output")
                    .and_then(Value::as_str)
                    .unwrap_or_default(),
            ),
            exit_code: json.pointer("/metadata/exit_code").and_then(Value::as_i64),
            duration_ms: json
                .pointer("/metadata/duration_seconds")
                .and_then(Value::as_f64)
                .and_then(seconds),
        };
    }
    let mut exit_code = None;
    let mut duration_ms = None;
    let mut body = raw;
    if let Some(rest) = raw.strip_prefix("Exit code: ") {
        let (first, tail) = rest.split_once('\n').unwrap_or((rest, ""));
        exit_code = first.trim().parse().ok();
        body = tail;
        if let Some(rest) = tail.strip_prefix("Wall time: ") {
            let (first, tail) = rest.split_once('\n').unwrap_or((rest, ""));
            duration_ms = first
                .trim()
                .trim_end_matches("seconds")
                .trim()
                .parse::<f64>()
                .ok()
                .and_then(seconds);
            body = tail;
        }
        body = body.strip_prefix("Output:\n").unwrap_or(body);
    }
    CodexResult {
        text: bounded_output(body),
        exit_code,
        duration_ms,
    }
}

/// The files an `apply_patch` input changes: one change per `*** Add/Update/Delete File:` header,
/// its diff being the lines under it.
fn patch_changes(patch: &str) -> Vec<Value> {
    let mut changes: Vec<(String, &'static str, String)> = Vec::new();
    for line in patch.lines() {
        let header = [
            ("*** Add File: ", "add"),
            ("*** Update File: ", "update"),
            ("*** Delete File: ", "delete"),
        ]
        .iter()
        .find_map(|(prefix, kind)| line.strip_prefix(prefix).map(|path| (path, *kind)));
        match header {
            Some((path, kind)) => changes.push((clean_text(path.trim()), kind, String::new())),
            None if line.starts_with("*** ") => {}
            None => {
                if let Some((_, _, diff)) = changes.last_mut() {
                    diff.push_str(line);
                    diff.push('\n');
                }
            }
        }
    }
    changes
        .into_iter()
        .map(
            |(path, kind, diff)| json!({ "path": path, "kind": kind, "diff": bounded_text(&diff) }),
        )
        .collect()
}

fn bounded_output(text: &str) -> String {
    cut(&clean_text(text), MAX_OUTPUT_BYTES)
}

fn bounded_text(text: &str) -> String {
    cut(&clean_text(text), MAX_MESSAGE_BYTES)
}

fn cut(text: &str, budget: usize) -> String {
    if text.len() <= budget {
        return text.to_owned();
    }
    let mut end = budget;
    while !text.is_char_boundary(end) {
        end -= 1;
    }
    format!("{}{TRUNCATED}", &text[..end])
}

/// Unix milliseconds of an RFC 3339 UTC time (`2026-11-04T09:30:15.123Z`); other zones and
/// malformed text give `None`.
pub(in crate::daemon) fn iso_ms(text: &str) -> Option<u64> {
    let bytes = text.as_bytes();
    let number = |from: usize, to: usize| -> Option<i64> {
        std::str::from_utf8(bytes.get(from..to)?).ok()?.parse().ok()
    };
    if bytes.len() < 20 || bytes[4] != b'-' || bytes[7] != b'-' || !matches!(bytes[10], b'T' | b' ')
    {
        return None;
    }
    let (year, month, day) = (number(0, 4)?, number(5, 7)?, number(8, 10)?);
    let (hour, minute, second) = (number(11, 13)?, number(14, 16)?, number(17, 19)?);
    let mut rest = &text[19..];
    let mut millis = 0;
    if let Some(fraction) = rest.strip_prefix('.') {
        let digits: String = fraction.chars().take_while(char::is_ascii_digit).collect();
        millis = format!("{digits:0<3}")[..3].parse::<i64>().ok()?;
        rest = &fraction[digits.len()..];
    }
    if rest != "Z" && rest != "+00:00" {
        return None;
    }
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    // Days from 1970-01-01 (Howard Hinnant's civil-date algorithm).
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year.rem_euclid(400);
    let shifted_month = (month + 9) % 12;
    let day_of_year = (153 * shifted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146_097 + day_of_era - 719_468;
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second;
    u64::try_from(seconds * 1000 + millis).ok()
}

#[cfg(test)]
mod tests;
