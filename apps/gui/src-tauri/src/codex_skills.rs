//! The skills Codex has in a folder, asked of Codex itself. Codex has no way to list its built-in
//! slash commands (they are a list inside its terminal program), but its app server lists the
//! skills of the person, the project and its plugins (`skills/list`), which is what its own
//! message box offers after a `$`. This starts `codex app-server` with fixed arguments (never
//! through a shell), says hello, asks that one question, reads the answer and stops it. No message
//! is sent, so it uses none of the account's allowance.

use crate::claude_models::{Command, command_name, one_line};
use serde_json::Value;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

const ASK_TIMEOUT: Duration = Duration::from_secs(15);
/// A few hundred skills with their descriptions; anything past this is not that answer.
const MAX_REPLY_BYTES: usize = 8 * 1024 * 1024;
const MAX_SKILLS: usize = 2000;
const MAX_DESCRIPTION_CHARS: usize = 240;
const HELLO_ID: u64 = 1;
const SKILLS_ID: u64 = 2;
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// The skills in the answer to `skills/list` that are switched on, each once, cleaned for the
/// window as Claude Code's commands are.
fn skills(answer: &Value) -> Vec<Command> {
    let mut seen = std::collections::HashSet::new();
    answer
        .get("data")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|folder| folder.get("skills").and_then(Value::as_array))
        .flatten()
        .filter(|skill| skill.get("enabled").and_then(Value::as_bool) != Some(false))
        .filter_map(|skill| {
            let text = |key: &str| skill.get(key).and_then(Value::as_str).unwrap_or_default();
            let name = command_name(text("name"))?;
            let said = if text("shortDescription").is_empty() { text("description") } else { text("shortDescription") };
            seen.insert(name.clone()).then(|| Command {
                name,
                description: one_line(said, MAX_DESCRIPTION_CHARS),
                hint: String::new(),
                aliases: Vec::new(),
            })
        })
        .take(MAX_SKILLS)
        .collect()
}

/// What a line of the app server's output holds for the request numbered `id`: its result, or why
/// there is none. None for any other line (a notification, another answer, something unreadable).
fn answer_to(line: &str, id: u64) -> Option<Result<Value, String>> {
    let message: Value = serde_json::from_str(line).ok()?;
    if message.get("id").and_then(Value::as_u64) != Some(id) {
        return None;
    }
    Some(match message.get("result") {
        Some(result) => Ok(result.clone()),
        None => Err(format!(
            "Codex would not list its skills: {}",
            one_line(message.pointer("/error/message").and_then(Value::as_str).unwrap_or("no reason given"), MAX_DESCRIPTION_CHARS)
        )),
    })
}

/// Asks the Codex at `executable` for the skills it has in `folder`.
pub async fn ask_skills(executable: &Path, folder: &Path) -> Result<Vec<Command>, String> {
    let mut command = tokio::process::Command::new(executable);
    command
        .arg("app-server")
        .current_dir(folder)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|error| format!("Could not start Codex: {error}"))?;
    let (Some(mut input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
        return Err("Codex started without its input and output".to_string());
    };
    let hello = serde_json::json!({
        "id": HELLO_ID,
        "method": "initialize",
        "params": {"clientInfo": {"name": "maitty", "title": "Maitty", "version": env!("CARGO_PKG_VERSION")}}
    });
    let ready = serde_json::json!({"method": "initialized"});
    let question = serde_json::json!({
        "id": SKILLS_ID,
        "method": "skills/list",
        "params": {"cwds": [folder], "forceReload": false}
    });
    let answer = tokio::time::timeout(ASK_TIMEOUT, async {
        input
            .write_all(format!("{hello}\n{ready}\n{question}\n").as_bytes())
            .await
            .map_err(|error| format!("Could not ask Codex for its skills: {error}"))?;
        let mut lines = BufReader::new(output).lines();
        let mut read = 0usize;
        while let Some(line) = lines.next_line().await.map_err(|error| format!("Could not read Codex's answer: {error}"))? {
            read += line.len();
            if read > MAX_REPLY_BYTES {
                return Err("Codex's answer is too long to be its skills".to_string());
            }
            if let Some(Err(refused)) = answer_to(&line, HELLO_ID) {
                return Err(refused);
            }
            if let Some(answer) = answer_to(&line, SKILLS_ID) {
                return answer.map(|result| skills(&result));
            }
        }
        Err("Codex closed before it answered".to_string())
    })
    .await
    .unwrap_or_else(|_| Err("Codex did not answer in time".to_string()));
    if let Err(error) = child.kill().await {
        tracing::warn!("stopping the Codex that was asked for its skills: {error}");
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::{SKILLS_ID, answer_to, skills};
    use serde_json::json;

    #[test]
    fn the_skills_switched_on_are_listed_once_each_and_cleaned() {
        let answer = json!({"data": [
            {"cwd": "/code", "errors": [], "skills": [
                {"name": "pdf", "description": "Reads\n  and writes PDFs", "enabled": true, "scope": "user", "path": "/s/pdf"},
                {"name": "review", "shortDescription": "Review a diff", "description": "The long one", "enabled": true},
                {"name": "off", "description": "Switched off", "enabled": false},
                {"name": "bad name\u{1b}[31m", "description": "Not a name", "enabled": true},
            ]},
            {"cwd": "/other", "skills": [{"name": "pdf", "description": "Again", "enabled": true}]},
        ]});
        let listed = skills(&answer);
        assert_eq!(listed.iter().map(|skill| skill.name.as_str()).collect::<Vec<_>>(), ["pdf", "review"]);
        assert_eq!(listed[0].description, "Reads and writes PDFs");
        assert_eq!(listed[1].description, "Review a diff");
        assert!(skills(&json!({})).is_empty());
    }

    #[test]
    fn only_the_answer_to_the_question_asked_is_read() {
        assert!(answer_to(r#"{"method":"account/updated","params":{}}"#, SKILLS_ID).is_none());
        assert!(answer_to(r#"{"id":1,"result":{}}"#, SKILLS_ID).is_none());
        assert!(answer_to("not json", SKILLS_ID).is_none());
        assert_eq!(answer_to(r#"{"id":2,"result":{"data":[]}}"#, SKILLS_ID), Some(Ok(json!({"data": []}))));
        let refused = answer_to(r#"{"id":2,"error":{"code":-32601,"message":"Method not found"}}"#, SKILLS_ID);
        assert_eq!(refused, Some(Err("Codex would not list its skills: Method not found".to_string())));
    }
}
