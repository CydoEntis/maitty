use serde_json::Value;
use std::path::Path;

/// Claude Code hook events used for native session status reporting.
pub const STATUS_EVENTS: &[(&str, Option<&str>)] = &[
    ("UserPromptSubmit", None),
    ("PreToolUse", None),
    ("PostToolUse", None),
    ("PermissionRequest", None),
    ("Notification", None),
    ("Stop", None),
    ("SubagentStart", None),
    ("SubagentStop", None),
    ("SessionStart", None),
    ("SessionEnd", None),
];

/// Maps events from Claude-compatible native hook payloads to Maitty status.
pub fn status_from_hook(payload: &Value) -> Option<maitty_shared::protocol::HookStatus> {
    use maitty_shared::protocol::HookStatus;

    let event = payload
        .get("hook_event_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    let tool = payload
        .get("tool_name")
        .and_then(Value::as_str)
        .unwrap_or("");
    Some(match event {
        "PreToolUse" if tool == "AskUserQuestion" => HookStatus::Blocked,
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "SubagentStart" => HookStatus::Working,
        "SubagentStop" => HookStatus::Same,
        "PermissionRequest" => HookStatus::Blocked,
        "Notification" => {
            let kind = payload
                .get("notification_type")
                .and_then(Value::as_str)
                .unwrap_or("");
            let message = payload
                .get("message")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_lowercase();
            match kind {
                "permission_prompt" | "elicitation_dialog" | "ToolPermission" => {
                    HookStatus::Blocked
                }
                "idle_prompt" => HookStatus::Done,
                _ if message.contains("permission") => HookStatus::Blocked,
                _ => return None,
            }
        }
        "Stop" => HookStatus::Done,
        "SessionStart" => HookStatus::Idle,
        "SessionEnd" => HookStatus::Gone,
        _ => return None,
    })
}

pub fn is_our_name(program: &str, app_names: &[&str]) -> bool {
    app_names.contains(&program)
}

pub fn runs_our_hook(command: &str, app_names: &[&str]) -> bool {
    let (program, args) = split_command(command);
    let stem = Path::new(program)
        .file_stem()
        .map(|name| name.to_string_lossy().to_ascii_lowercase())
        .unwrap_or_default();
    is_our_name(&stem, app_names) && args.starts_with("hook ")
}

pub fn is_our_hook_group(group: &Value, app_names: &[&str]) -> bool {
    let tagged = |name: &&str| {
        group
            .get(format!("_{name}").as_str())
            .and_then(Value::as_bool)
            == Some(true)
    };
    app_names.iter().any(tagged)
        || group
            .get("hooks")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(|hook| hook.get("command").and_then(Value::as_str))
            .any(|command| runs_our_hook(command, app_names))
}

pub fn stale_hooks(root: &Value, wanted: &str, app_names: &[&str]) -> Vec<String> {
    let mut stale: Vec<String> = root
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|hooks| hooks.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter(|group| is_our_hook_group(group, app_names))
        .filter_map(|group| group.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|hook| hook.get("command").and_then(Value::as_str))
        .filter(|command| *command != wanted)
        .map(String::from)
        .collect();
    stale.sort();
    stale.dedup();
    stale
}

fn split_command(command: &str) -> (&str, &str) {
    let command = command.trim_start();
    let (program, args) = match command.strip_prefix('"') {
        Some(rest) => rest.split_once('"').unwrap_or((rest, "")),
        None => command
            .split_once(char::is_whitespace)
            .unwrap_or((command, "")),
    };
    (program, args.trim_start())
}

#[cfg(test)]
mod tests {
    use super::{is_our_hook_group, is_our_name, runs_our_hook, stale_hooks, status_from_hook};
    use serde_json::json;

    const APP_NAMES: &[&str] = &["maitty", "seshi", "hydra", "drover"];

    #[test]
    fn recognizes_current_and_previous_hook_commands_and_tags() {
        assert!(is_our_name("hydra", APP_NAMES));
        assert!(runs_our_hook(
            "\"C:/Users/me/AppData/Local/Programs/seshi/seshi.exe\" hook claude",
            APP_NAMES
        ));
        assert!(!runs_our_hook("hydra-lint check", APP_NAMES));
        assert!(is_our_hook_group(
            &json!({ "_seshi": true, "hooks": [] }),
            APP_NAMES
        ));
    }

    #[test]
    fn stale_hook_projection_keeps_other_tools_and_deduplicates_old_app_hooks() {
        let wanted = "\"/home/me/.local/bin/maitty\" hook claude";
        let stale = "\"/home/me/.cargo/bin/hydra\" hook claude";
        let root = json!({ "hooks": {
            "Stop": [
                { "hooks": [{ "command": stale }] },
                { "hooks": [{ "command": stale }] },
                { "hooks": [{ "command": wanted }] },
                { "hooks": [{ "command": "someone-elses-hook" }] }
            ]
        }});

        assert_eq!(stale_hooks(&root, wanted, APP_NAMES), [stale]);
    }

    #[test]
    fn maps_claude_hook_events_and_leaves_unknown_notifications_unmapped() {
        use maitty_shared::protocol::HookStatus;

        assert_eq!(
            status_from_hook(
                &json!({"hook_event_name":"PreToolUse","tool_name":"AskUserQuestion"})
            ),
            Some(HookStatus::Blocked)
        );
        assert_eq!(
            status_from_hook(
                &json!({"hook_event_name":"Notification","notification_type":"permission_prompt"})
            ),
            Some(HookStatus::Blocked)
        );
        assert_eq!(
            status_from_hook(
                &json!({"hook_event_name":"Notification","notification_type":"unknown"})
            ),
            None
        );
        assert_eq!(
            status_from_hook(&json!({"hook_event_name":"SessionEnd"})),
            Some(HookStatus::Gone)
        );
    }
}
