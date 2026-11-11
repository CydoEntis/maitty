//! Claude Code's own model list, asked of the program itself. Claude Code keeps no file that lists
//! every model an account can run; it says them in the reply to the `initialize` request of its
//! stream mode, which is what its own SDK reads. This starts the program with fixed arguments (never
//! through a shell), sends that one request, reads the reply and stops it. No message is sent, so it
//! uses none of the account's allowance, and the session is not kept.

use serde_json::Value;
use std::path::Path;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

/// Claude Code runs the person's start-up hooks before it answers, so this is generous.
const ASK_TIMEOUT: Duration = Duration::from_secs(20);
/// The reply also lists every slash command and agent; anything past this is not that reply.
const MAX_REPLY_BYTES: usize = 8 * 1024 * 1024;
const REQUEST_ID: &str = "maitty-models";
const ARGUMENTS: &[&str] = &[
    "-p",
    "--input-format",
    "stream-json",
    "--output-format",
    "stream-json",
    "--verbose",
    "--no-session-persistence",
];
#[cfg(windows)]
const CREATE_NO_WINDOW: u32 = 0x0800_0000;

/// One model as Claude Code lists it.
#[derive(Debug, PartialEq)]
pub struct Listed {
    /// What `--model` takes: an alias (`opus`) or a full model id.
    pub id: String,
    pub label: String,
    pub blurb: String,
    /// The full model id an alias stands for today.
    pub resolved: String,
    /// The effort levels the model takes; empty when it takes none.
    pub efforts: Vec<String>,
}

/// The reply to our request, when this line of output is it. A refusal is an error.
fn reply_in(line: &str) -> Option<Result<Value, String>> {
    let frame: Value = serde_json::from_str(line).ok()?;
    if frame.get("type")?.as_str()? != "control_response" {
        return None;
    }
    let reply = frame.get("response")?;
    if reply.get("request_id")?.as_str()? != REQUEST_ID {
        return None;
    }
    if reply.get("subtype").and_then(Value::as_str) == Some("error") {
        return Some(Err("Claude Code refused to list its models".to_string()));
    }
    Some(Ok(reply.get("response").cloned().unwrap_or_default()))
}

fn listed(reply: &Value) -> Vec<Listed> {
    reply
        .get("models")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|model| {
            let text = |key: &str| model.get(key).and_then(Value::as_str);
            let id = text("value")?.trim();
            if id.is_empty() {
                return None;
            }
            let takes_effort = model.get("supportsEffort").and_then(Value::as_bool) == Some(true);
            let efforts = model
                .get("supportedEffortLevels")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter(|_| takes_effort)
                .filter_map(|level| level.as_str().map(str::to_string))
                .collect();
            Some(Listed {
                id: id.to_string(),
                label: text("displayName").unwrap_or(id).to_string(),
                blurb: text("description").unwrap_or_default().to_string(),
                resolved: text("resolvedModel").unwrap_or(id).to_string(),
                efforts,
            })
        })
        .collect()
}

/// Asks the Claude Code at `executable` for its models, started in `folder`.
pub async fn ask(executable: &Path, folder: &Path) -> Result<Vec<Listed>, String> {
    let mut command = tokio::process::Command::new(executable);
    command
        .args(ARGUMENTS)
        .current_dir(folder)
        // Claude Code will not start inside another Claude Code session, and this app may have been
        // started from one.
        .env_remove("CLAUDECODE")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true);
    #[cfg(windows)]
    command.creation_flags(CREATE_NO_WINDOW);
    let mut child = command.spawn().map_err(|error| format!("Could not start Claude Code: {error}"))?;
    let (Some(mut input), Some(output)) = (child.stdin.take(), child.stdout.take()) else {
        return Err("Claude Code started without its input and output".to_string());
    };
    let request = serde_json::json!({
        "type": "control_request",
        "request_id": REQUEST_ID,
        "request": {"subtype": "initialize", "hooks": null}
    });
    let answer = tokio::time::timeout(ASK_TIMEOUT, async {
        input
            .write_all(format!("{request}\n").as_bytes())
            .await
            .map_err(|error| format!("Could not ask Claude Code for its models: {error}"))?;
        let mut lines = BufReader::new(output).lines();
        let mut read = 0usize;
        while let Some(line) =
            lines.next_line().await.map_err(|error| format!("Could not read Claude Code's reply: {error}"))?
        {
            read += line.len();
            if read > MAX_REPLY_BYTES {
                return Err("Claude Code's reply is too long to be its model list".to_string());
            }
            if let Some(reply) = reply_in(&line) {
                return reply.map(|reply| listed(&reply));
            }
        }
        Err("Claude Code closed before it listed its models".to_string())
    })
    .await
    .unwrap_or_else(|_| Err("Claude Code did not list its models in time".to_string()));
    if let Err(error) = child.kill().await {
        tracing::warn!("stopping the Claude Code that listed its models: {error}");
    }
    answer
}

#[cfg(test)]
mod tests {
    use super::{listed, reply_in, Listed};

    const REPLY: &str = r#"{"type":"control_response","response":{"request_id":"maitty-models","subtype":"success","response":{"models":[
        {"value":"default","resolvedModel":"claude-fable-5-2","displayName":"Default (recommended)","description":"Fable 5.2","supportsEffort":true,"supportedEffortLevels":["low","medium","high","xhigh","max"]},
        {"value":"opus","resolvedModel":"claude-opus-5-5","displayName":"Opus 5.5","description":"For complex work","supportsEffort":true,"supportedEffortLevels":["low","medium","high","max"]},
        {"value":"claude-haiku-4-5-20251001","resolvedModel":"claude-haiku-4-5-20251001","displayName":"Haiku 4.5","description":"Fastest"},
        {"value":"","displayName":"No id"}]}}}"#;

    #[test]
    fn the_reply_lists_each_model_with_the_efforts_it_takes() {
        let reply = reply_in(&REPLY.replace('\n', "")).unwrap().unwrap();
        let models = listed(&reply);
        let ids: Vec<&str> = models.iter().map(|model| model.id.as_str()).collect();
        assert_eq!(ids, ["default", "opus", "claude-haiku-4-5-20251001"]);
        assert_eq!(
            models[1],
            Listed {
                id: "opus".into(),
                label: "Opus 5.5".into(),
                blurb: "For complex work".into(),
                resolved: "claude-opus-5-5".into(),
                efforts: vec!["low".into(), "medium".into(), "high".into(), "max".into()],
            }
        );
        assert!(models[2].efforts.is_empty());
    }

    #[test]
    fn other_output_is_not_the_reply_and_a_refusal_is_an_error() {
        assert!(reply_in(r#"{"type":"system","subtype":"hook_started"}"#).is_none());
        assert!(reply_in("not json").is_none());
        assert!(reply_in(r#"{"type":"control_response","response":{"request_id":"someone-else","subtype":"success"}}"#).is_none());
        let refused = r#"{"type":"control_response","response":{"request_id":"maitty-models","subtype":"error","error":"no"}}"#;
        assert!(reply_in(refused).unwrap().is_err());
    }

    #[test]
    fn a_reply_with_no_models_lists_none() {
        let reply = reply_in(r#"{"type":"control_response","response":{"request_id":"maitty-models","subtype":"success"}}"#).unwrap().unwrap();
        assert!(listed(&reply).is_empty());
    }
}
