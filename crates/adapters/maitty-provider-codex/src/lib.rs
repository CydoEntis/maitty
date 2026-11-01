use maitty_shared::protocol::{Limit, Usage};
use regex::Regex;
use serde_json::Value;
use std::fmt;
use std::path::PathBuf;
use std::sync::LazyLock;

mod runtime_policy;
mod status_hooks;

pub use runtime_policy::runtime_policy_overrides;
pub use status_hooks::{StatusHookOptions, update_status_hooks};

/// Maps Codex's native completion notification to Maitty's shared hook status.
pub fn status_from_hook(payload: &Value) -> Option<maitty_shared::protocol::HookStatus> {
    (payload.get("type").and_then(Value::as_str) == Some("agent-turn-complete"))
        .then_some(maitty_shared::protocol::HookStatus::Done)
}

/// A Codex session's id, parent, working folder and display name from its metadata line.
pub fn session_metadata(head: &str) -> Option<(String, Option<String>, Option<PathBuf>, String)> {
    static FIELD: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#""(id|parent_thread_id|cwd|agent_path|agent_nickname|thread_source)"\s*:\s*("(?:[^"\\]|\\.)*")"#).unwrap()
    });
    let mut fields = std::collections::HashMap::new();
    for captures in FIELD.captures_iter(head) {
        let value = serde_json::from_str::<String>(&captures[2]).unwrap_or_default();
        fields.entry(captures[1].to_owned()).or_insert(value);
    }
    let id = fields.get("id").cloned().filter(|id| !id.is_empty())?;
    let is_subagent = fields
        .get("thread_source")
        .is_some_and(|source| source == "subagent");
    let parent = fields
        .get("parent_thread_id")
        .cloned()
        .filter(|parent| is_subagent && !parent.is_empty());
    let name = fields
        .get("agent_path")
        .and_then(|path| path.rsplit('/').next().map(str::to_owned))
        .filter(|name| !name.is_empty())
        .or_else(|| fields.get("agent_nickname").cloned())
        .unwrap_or_else(|| "subagent".into());
    Some((id, parent, fields.get("cwd").map(PathBuf::from), name))
}

/// Whether the last task event in a Codex session tail is still going.
pub fn task_running(tail: &str) -> Option<bool> {
    match task_event(tail)? {
        "task_started" => Some(true),
        "task_complete" | "turn_aborted" => Some(false),
        _ => None,
    }
}

/// Whether the latest task started from a Codex goal.
pub fn goal_turn(tail: &str) -> bool {
    tail.lines()
        .rev()
        .find(|line| line.contains("\"type\":\"task_started\""))
        .is_some_and(|line| line.contains("\"turn_trigger\":\"goal\""))
}

/// The last recognized Codex task event in a session tail.
pub fn task_event(tail: &str) -> Option<&'static str> {
    tail.lines().rev().find_map(|line| {
        let event = serde_json::from_str::<Value>(line).ok().and_then(|value| {
            value
                .get("payload")
                .and_then(|payload| payload.get("type"))
                .and_then(Value::as_str)
                .or_else(|| value.get("type").and_then(Value::as_str))
                .map(str::to_owned)
        })?;
        match event.as_str() {
            "task_started" => Some("task_started"),
            "task_complete" => Some("task_complete"),
            "turn_aborted" => Some("turn_aborted"),
            _ => None,
        }
    })
}

/// Context usage and plan limits from the last Codex token-count event in a session tail.
pub fn session_facts(tail: &str) -> (Option<Usage>, Option<Vec<Limit>>) {
    let Some(value) = tail
        .lines()
        .rev()
        .filter(|line| line.contains("\"token_count\""))
        .find_map(|line| serde_json::from_str::<Value>(line).ok())
    else {
        return (None, None);
    };
    let payload = &value["payload"];
    let info = &payload["info"];
    let window = info["model_context_window"]
        .as_f64()
        .filter(|window| *window > 0.0);
    let used = info["last_token_usage"]["total_tokens"].as_f64();
    let usage = window.zip(used).map(|(window, used)| Usage {
        context: Some((used / window * 100.0).min(100.0) as f32),
        cost: None,
    });
    let limits: Vec<Limit> = ["primary", "secondary"]
        .iter()
        .filter_map(|key| {
            let limit = &payload["rate_limits"][key];
            Some(Limit {
                name: window_name(limit["window_minutes"].as_u64()?),
                used: limit["used_percent"].as_f64()? as f32,
                resets_at: limit["resets_at"].as_u64()?,
            })
        })
        .collect();
    (usage, (!limits.is_empty()).then_some(limits))
}

fn window_name(minutes: u64) -> String {
    match minutes {
        300 => "5h".into(),
        10080 => "week".into(),
        minutes if minutes % 1440 == 0 => format!("{}d", minutes / 1440),
        minutes if minutes % 60 == 0 => format!("{}h", minutes / 60),
        minutes => format!("{minutes}m"),
    }
}

#[cfg(test)]
mod usage_tests {
    use super::*;

    #[test]
    fn parses_codex_session_metadata_without_claiming_nested_identity() {
        let root = r#"{"type":"session_meta","payload":{"id":"aaa","cwd":"C:\\dev\\game","thread_source":"user"}}"#;
        assert_eq!(
            session_metadata(root),
            Some((
                "aaa".into(),
                None,
                Some(PathBuf::from(r"C:\dev\game")),
                "subagent".into()
            ))
        );
        let subagent = r#"{"type":"session_meta","payload":{"id":"bbb","parent_thread_id":"aaa","thread_source":"subagent","agent_path":"/root/reconcile_audit"}}"#;
        assert_eq!(
            session_metadata(subagent),
            Some((
                "bbb".into(),
                Some("aaa".into()),
                None,
                "reconcile_audit".into()
            ))
        );
        assert_eq!(session_metadata("not a session"), None);
    }

    #[test]
    fn reads_the_last_codex_task_event_and_goal_trigger() {
        let started = r#"{"type":"event_msg","payload":{"type":"task_started","turn_attribution":{"turn_trigger":"user"}}}"#;
        let by_goal = r#"{"type":"event_msg","payload":{"type":"task_started","turn_attribution":{"turn_trigger":"goal"}}}"#;
        let done = r#"{"type":"event_msg","payload":{"type":"task_complete"}}"#;
        assert_eq!(task_running(&format!("{started}\n{done}")), Some(false));
        assert_eq!(
            task_running(&format!(
                "{done}\n{started}\n{{\"type\":\"response_item\"}}"
            )),
            Some(true)
        );
        assert_eq!(task_running("{\"type\":\"response_item\"}"), None);
        assert!(goal_turn(&format!("{started}\n{done}\n{by_goal}")));
        assert!(!goal_turn(&format!("{by_goal}\n{done}\n{started}")));
    }

    #[test]
    fn parses_context_and_rate_limits_from_codex_session_events() {
        let tail = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"total_tokens":64600},"model_context_window":258400},"rate_limits":{"primary":{"used_percent":12.0,"window_minutes":300,"resets_at":1791322893},"secondary":{"used_percent":40.0,"window_minutes":10080,"resets_at":1791900000}}}}"#;
        let (usage, limits) = session_facts(tail);
        assert_eq!(
            usage.and_then(|usage| usage.context).map(f32::round),
            Some(25.0)
        );
        let limits = limits.unwrap();
        assert_eq!(
            (
                limits[0].name.as_str(),
                limits[1].name.as_str(),
                limits[1].used
            ),
            ("5h", "week", 40.0)
        );
        assert_eq!(session_facts("nothing here"), (None, None));
    }
}

/// Codex app-server versions whose required methods and payloads are verified.
pub const VERIFIED_VERSIONS: &[&str] = &["0.164.0", "0.165.0", "0.166.0-alpha.5.1", "0.166.0"];
const VERIFIED_VERSIONS_TEXT: &str = "0.164.0, 0.165.0, 0.166.0-alpha.5.1 or 0.166.0";
const CLIENT_METHODS: &[(&str, &str)] = &[
    ("initialize", "InitializeParams"),
    ("config/read", "ConfigReadParams"),
    ("thread/start", "ThreadStartParams"),
    ("thread/resume", "ThreadResumeParams"),
    ("thread/read", "ThreadReadParams"),
    ("thread/turns/list", "ThreadTurnsListParams"),
    ("thread/items/list", "ThreadItemsListParams"),
    ("turn/start", "TurnStartParams"),
    ("turn/interrupt", "TurnInterruptParams"),
];
const SERVER_METHODS: &[(&str, &str)] = &[
    (
        "item/commandExecution/requestApproval",
        "CommandExecutionRequestApprovalParams",
    ),
    (
        "item/fileChange/requestApproval",
        "FileChangeRequestApprovalParams",
    ),
    (
        "item/permissions/requestApproval",
        "PermissionsRequestApprovalParams",
    ),
    ("item/tool/requestUserInput", "ToolRequestUserInputParams"),
];

#[derive(Debug, PartialEq, Eq)]
/// A Codex app-server failed the no-session-mutation compatibility preflight.
pub enum CompatibilityError {
    Unsupported { capability: String },
}

impl fmt::Display for CompatibilityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        let Self::Unsupported { capability } = self;
        write!(
            formatter,
            "Unsupported Codex app-server: {capability}. Select a verified Codex {VERIFIED_VERSIONS_TEXT} executable; no session was started or resumed."
        )
    }
}

impl std::error::Error for CompatibilityError {}

fn unsupported(capability: &str) -> CompatibilityError {
    CompatibilityError::Unsupported {
        capability: capability.into(),
    }
}

/// Returns whether `version` is in the verified Codex app-server set.
pub fn is_verified_version(version: &str) -> bool {
    VERIFIED_VERSIONS.contains(&version)
}

/// Checks that a verified Codex server advertises every operation Maitty uses.
///
/// This preflight takes no session handle, so refusal cannot mutate a thread.
pub fn check_compatibility(version: &str, schema: &str) -> Result<(), CompatibilityError> {
    if !is_verified_version(version) {
        return Err(unsupported("unverified executable version"));
    }
    let schema: Value = serde_json::from_str(schema)
        .map_err(|_| unsupported("invalid generated-schema representation"))?;
    for (section, requirements) in [
        ("clientRequests", CLIENT_METHODS),
        ("serverRequests", SERVER_METHODS),
    ] {
        let variants = schema[section]["oneOf"]
            .as_array()
            .ok_or_else(|| unsupported(section))?;
        for (method, params) in requirements {
            let reference = format!("#/definitions/{params}");
            let found = variants.iter().any(|variant| {
                variant["properties"]["method"]["enum"] == serde_json::json!([method])
                    && variant["properties"]["params"]["$ref"].as_str() == Some(reference.as_str())
            });
            if !found {
                return Err(unsupported(method));
            }
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SUPPORTED_SCHEMA: &str = include_str!("fixtures/codex-0.164.0.json");
    const CURRENT_SCHEMA: &str = include_str!("fixtures/codex-0.166.0-alpha.5.1.json");
    const STABLE_SCHEMA: &str = include_str!("fixtures/codex-0.166.0.json");
    const MISSING_INTERRUPT: &str = include_str!("fixtures/missing-interrupt.json");

    #[test]
    fn maps_only_codex_turn_completion_notifications() {
        assert_eq!(
            status_from_hook(&serde_json::json!({"type": "agent-turn-complete"})),
            Some(maitty_shared::protocol::HookStatus::Done)
        );
        assert_eq!(
            status_from_hook(&serde_json::json!({"type": "other-event"})),
            None
        );
    }

    #[test]
    fn verified_versions_support_required_session_operations() {
        for version in VERIFIED_VERSIONS {
            let schema = if *version == "0.166.0-alpha.5.1" {
                CURRENT_SCHEMA
            } else if *version == "0.166.0" {
                STABLE_SCHEMA
            } else {
                SUPPORTED_SCHEMA
            };
            assert_eq!(check_compatibility(version, schema), Ok(()));
        }
    }

    #[test]
    fn codex_0_166_alpha_supports_the_required_session_operations() {
        assert_eq!(
            check_compatibility("0.166.0-alpha.5.1", CURRENT_SCHEMA),
            Ok(())
        );
    }

    #[test]
    fn each_required_method_is_checked_before_session_mutation() {
        for section in ["clientRequests", "serverRequests"] {
            let baseline: Value = serde_json::from_str(SUPPORTED_SCHEMA).unwrap();
            let count = baseline[section]["oneOf"].as_array().unwrap().len();
            for index in 0..count {
                let mut schema = baseline.clone();
                let removed = schema[section]["oneOf"]
                    .as_array_mut()
                    .unwrap()
                    .remove(index);
                let method = removed["properties"]["method"]["enum"][0].as_str().unwrap();
                assert_eq!(
                    check_compatibility("0.164.0", &schema.to_string()),
                    Err(unsupported(method)),
                    "missing {method}"
                );
            }
        }
    }

    #[test]
    fn changed_params_contract_is_unsupported() {
        let mut schema: Value = serde_json::from_str(SUPPORTED_SCHEMA).unwrap();
        let initialize = schema["clientRequests"]["oneOf"]
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .find(|request| {
                request["properties"]["method"]["enum"] == serde_json::json!(["initialize"])
            })
            .unwrap();
        initialize["properties"]["params"]["$ref"] =
            serde_json::json!("#/definitions/DifferentInitializeParams");
        assert_eq!(
            check_compatibility("0.164.0", &schema.to_string()),
            Err(unsupported("initialize"))
        );
    }

    #[test]
    fn unverified_executable_version_is_unsupported() {
        assert_eq!(
            check_compatibility("0.167.0", SUPPORTED_SCHEMA),
            Err(unsupported("unverified executable version"))
        );
    }

    #[test]
    fn malformed_schema_is_unsupported() {
        assert_eq!(
            check_compatibility("0.164.0", "{"),
            Err(unsupported("invalid generated-schema representation"))
        );
    }

    #[test]
    fn initialize_response_does_not_advertise_server_capabilities() {
        let response = include_str!("fixtures/initialize-response.json");
        assert_eq!(
            check_compatibility("0.164.0", response),
            Err(unsupported("clientRequests"))
        );
    }

    #[test]
    fn missing_capability_refuses_session_mutation_with_actionable_error() {
        let mut session_requests = Vec::new();
        let result = check_compatibility("0.164.0", MISSING_INTERRUPT).map(|()| {
            session_requests.push("thread/start");
        });

        assert_eq!(
            result,
            Err(CompatibilityError::Unsupported {
                capability: "turn/interrupt".into(),
            })
        );
        assert!(session_requests.is_empty());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Unsupported Codex app-server: turn/interrupt. Select a verified Codex 0.164.0, 0.165.0, 0.166.0-alpha.5.1 or 0.166.0 executable; no session was started or resumed."
        );
    }
}
