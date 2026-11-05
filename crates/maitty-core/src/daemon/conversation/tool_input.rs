//! Turns what a provider sends about a tool call (its input, an approval's subject) into text and
//! JSON that is safe to keep in the conversation history and to show: bounded in size, free of
//! terminal escape sequences and of characters that draw as nothing.
//!
//! Tool inputs hold whatever the agent was working on (file contents, commands, environment), so
//! nothing here may grow with the input: every string, list and the whole value is capped.

use serde_json::{Map, Value};

/// Largest serialized `input` kept for one tool call.
const MAX_INPUT_BYTES: usize = 64 * 1024;
/// A string leaf is first cut to the first budget; if the whole value is still over
/// `MAX_INPUT_BYTES`, to the next one, and so on. A big edit therefore keeps its shape.
const STRING_BUDGETS: [usize; 4] = [16 * 1024, 4 * 1024, 1024, 256];
const MAX_DEPTH: usize = 6;
const MAX_ITEMS: usize = 128;
const TRUNCATED: &str = "\n[truncated]";
/// Largest diff text sent with one approval, so one huge write cannot bloat a pending request.
const MAX_PREVIEW_BYTES: usize = 32 * 1024;
const MAX_SUMMARY_CHARS: usize = 300;
/// Input fields that name what a tool acts on, most telling first.
const SUMMARY_FIELDS: [&str; 9] = [
    "file_path",
    "notebook_path",
    "path",
    "url",
    "pattern",
    "query",
    "command",
    "description",
    "prompt",
];

/// `text` without escape sequences or control characters (a line break and a tab stay) and with
/// each invisible character spelled out, so what is shown is what is there.
pub(super) fn clean_text(text: &str) -> String {
    let mut clean = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\u{1b}' => skip_escape_sequence(&mut chars),
            '\n' | '\t' => clean.push(c),
            c if c.is_control() => {}
            c if crate::hidden::is_invisible(c) => {
                clean.push_str(&format!("\\u{{{:x}}}", c as u32));
            }
            c => clean.push(c),
        }
    }
    clean
}

/// Consumes the rest of an escape sequence whose ESC was just read: CSI up to its final byte,
/// OSC/DCS/APC/PM/SOS up to BEL or ST, or a single following character.
fn skip_escape_sequence(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) {
    match chars.next() {
        Some('[') => {
            for c in chars.by_ref() {
                if ('\u{40}'..='\u{7e}').contains(&c) {
                    break;
                }
            }
        }
        Some(']' | 'P' | 'X' | '^' | '_') => {
            while let Some(c) = chars.next() {
                if c == '\u{7}' || (c == '\u{1b}' && chars.next_if_eq(&'\\').is_some()) {
                    break;
                }
            }
        }
        _ => {}
    }
}

/// `text` cut to at most `budget` bytes on a character boundary, with a marker when it was cut.
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

fn bound_value(value: &Value, budget: usize, depth: usize) -> Option<Value> {
    Some(match value {
        Value::String(text) => Value::String(cut(&clean_text(text), budget)),
        Value::Array(items) => {
            if depth >= MAX_DEPTH || items.len() > MAX_ITEMS {
                return None;
            }
            Value::Array(
                items
                    .iter()
                    .map(|item| bound_value(item, budget, depth + 1))
                    .collect::<Option<_>>()?,
            )
        }
        Value::Object(fields) => {
            if depth >= MAX_DEPTH || fields.len() > MAX_ITEMS {
                return None;
            }
            let mut bounded = Map::with_capacity(fields.len());
            for (key, field) in fields {
                bounded.insert(
                    cut(&clean_text(key), STRING_BUDGETS[3]),
                    bound_value(field, budget, depth + 1)?,
                );
            }
            Value::Object(bounded)
        }
        other => other.clone(),
    })
}

/// The tool input to keep in history. `None` when there is none worth keeping, or when it cannot
/// be kept whole in shape (nested too deep, a list or object over `MAX_ITEMS`, or still over
/// `MAX_INPUT_BYTES` with every string cut to the smallest budget); an input cut off partway
/// would read as complete, so it is left out and the card says the detail is unavailable.
pub(super) fn bound_input(input: &Value) -> Option<Value> {
    if input.is_null() || input.as_object().is_some_and(Map::is_empty) {
        return None;
    }
    STRING_BUDGETS.iter().find_map(|budget| {
        let bounded = bound_value(input, *budget, 0)?;
        (serde_json::to_vec(&bounded).ok()?.len() <= MAX_INPUT_BYTES).then_some(bounded)
    })
}

/// One line saying what an approval is about: the field that names the target, cleaned and cut.
pub(super) fn input_summary(input: &Value) -> String {
    let named = SUMMARY_FIELDS
        .iter()
        .find_map(|field| input.get(field).and_then(Value::as_str));
    let text = named.map_or_else(|| input.to_string(), str::to_owned);
    let line = clean_text(&text)
        .lines()
        .next()
        .unwrap_or_default()
        .to_owned();
    line.chars().take(MAX_SUMMARY_CHARS).collect()
}

/// The label shown for an approval: a Bash command as written, any other tool as
/// "Name: what it acts on".
pub(super) fn approval_command(tool_name: &str, input: &Value) -> String {
    match input.get("command").and_then(Value::as_str) {
        Some(command) => command.to_owned(),
        None => format!("{}: {}", clean_text(tool_name), input_summary(input)),
    }
}

fn prefixed(prefix: char, text: &str) -> String {
    text.lines()
        .map(|line| format!("{prefix}{line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

fn replacement(old: &str, new: &str) -> String {
    [prefixed('-', old), prefixed('+', new)]
        .into_iter()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// The file changes a Claude edit tool is asking to make, in the shape of an approval's
/// `changes` (path, kind, diff). `None` for any other tool, or an input of an unexpected shape.
pub(super) fn file_change_previews(tool_name: &str, input: &Value) -> Option<Vec<Value>> {
    let path = clean_text(input.get("file_path")?.as_str()?);
    let text =
        |value: &Value, field: &str| value.get(field).and_then(Value::as_str).map(str::to_owned);
    let (kind, diff) = match tool_name {
        "Edit" => (
            "update",
            replacement(&text(input, "old_string")?, &text(input, "new_string")?),
        ),
        "MultiEdit" => {
            let edits = input.get("edits")?.as_array()?;
            let parts = edits
                .iter()
                .map(|edit| {
                    Some(replacement(
                        &text(edit, "old_string")?,
                        &text(edit, "new_string")?,
                    ))
                })
                .collect::<Option<Vec<_>>>()?;
            ("update", parts.join("\n"))
        }
        "Write" => ("write", prefixed('+', &text(input, "content")?)),
        _ => return None,
    };
    Some(vec![serde_json::json!({
        "path": path,
        "kind": kind,
        "diff": cut(&clean_text(&diff), MAX_PREVIEW_BYTES),
    })])
}

#[cfg(test)]
mod tests;
