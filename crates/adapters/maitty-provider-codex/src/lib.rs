use maitty_shared::protocol::{Limit, Usage};
use regex::Regex;
use serde_json::Value;
use std::fmt;
use std::path::PathBuf;
use std::sync::LazyLock;

mod runtime_policy;
mod status_hooks;

pub use runtime_policy::runtime_policy_overrides;
pub use status_hooks::{HooksState, StatusHookOptions, hooks_state, update_status_hooks};

/// The tool Codex calls to ask you a question mid-turn.
const QUESTION_TOOL: &str = "request_user_input";

/// Maps what only Codex says to Maitty's shared hook status: its completion notification, an
/// interrupted turn, and a question to you. Every other Codex hook event is in Claude's
/// dialect and maps there.
pub fn status_from_hook(payload: &Value) -> Option<maitty_shared::protocol::HookStatus> {
    use maitty_shared::protocol::HookStatus;
    let text = |key: &str| payload.get(key).and_then(Value::as_str).unwrap_or("");
    if text("type") == "agent-turn-complete" {
        return Some(HookStatus::Done);
    }
    match (text("hook_event_name"), text("tool_name")) {
        ("Interrupt", _) => Some(HookStatus::Idle),
        ("PreToolUse", QUESTION_TOOL) => Some(HookStatus::Blocked),
        _ => None,
    }
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
        tokens: info["total_token_usage"]["total_tokens"].as_u64(),
        tokens_in: info["total_token_usage"]["input_tokens"].as_u64(),
        tokens_out: info["total_token_usage"]["output_tokens"].as_u64(),
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

    #[test]
    fn takes_the_input_and_output_totals_codex_reports_and_no_cost() {
        let tail = r#"{"type":"event_msg","payload":{"type":"token_count","info":{"total_token_usage":{"input_tokens":900,"cached_input_tokens":400,"output_tokens":120,"total_tokens":1020},"last_token_usage":{"total_tokens":64600},"model_context_window":258400}}}"#;
        let usage = session_facts(tail).0.unwrap();
        assert_eq!(
            (usage.tokens, usage.tokens_in, usage.tokens_out, usage.cost),
            (Some(1020), Some(900), Some(120), None)
        );
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
/// Methods some verified Codex versions lack. A connection that lacks one still passes
/// [`check_compatibility`]; the caller asks before using it.
const TURN_STEER: (&str, &str) = ("turn/steer", "TurnSteerParams");
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
            if !declares(variants, method, params) {
                return Err(unsupported(method));
            }
        }
    }
    Ok(())
}

fn declares(variants: &[Value], method: &str, params: &str) -> bool {
    let reference = format!("#/definitions/{params}");
    variants.iter().any(|variant| {
        variant["properties"]["method"]["enum"] == serde_json::json!([method])
            && variant["properties"]["params"]["$ref"].as_str() == Some(reference.as_str())
    })
}

/// Whether the connected Codex's own generated schema declares `turn/steer`.
///
/// Reads the same schema inventory as [`check_compatibility`], not the version string, so a
/// Codex that gains or lacks the method is reported as it is. Any unreadable schema is "no".
/// Not used by the daemon yet; it is `pub` so the next change can call it.
pub fn supports_turn_steer(schema: &str) -> bool {
    let (method, params) = TURN_STEER;
    serde_json::from_str::<Value>(schema)
        .ok()
        .and_then(|schema| schema["clientRequests"]["oneOf"].as_array().cloned())
        .is_some_and(|variants| declares(&variants, method, params))
}

/// A `turn/steer` JSON-RPC request: adds text to the turn that is running now.
///
/// `expected_turn_id` is Codex's own precondition: it fails the request when another turn is
/// active, so a steer never lands on the wrong turn.
pub fn turn_steer_request(id: u64, thread_id: &str, expected_turn_id: &str, text: &str) -> Value {
    serde_json::json!({
        "id": id,
        "method": TURN_STEER.0,
        "params": {
            "threadId": thread_id,
            "expectedTurnId": expected_turn_id,
            "input": [{"type": "text", "text": text, "text_elements": []}],
        }
    })
}

/// The turn id from a `turn/steer` response, or why the response is not an acknowledgement.
pub fn parse_turn_steer_ack(response: &Value) -> Result<String, String> {
    if let Some(error) = response.get("error") {
        let message = error["message"].as_str().unwrap_or("no message");
        return Err(format!("Codex refused the steer: {message}"));
    }
    response["result"]["turnId"]
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| "Codex's steer response has no turnId".to_owned())
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
    fn an_interrupt_is_idle_and_a_question_to_you_needs_you() {
        use maitty_shared::protocol::HookStatus;
        let event = |name: &str, tool: &str| {
            status_from_hook(&serde_json::json!({"hook_event_name": name, "tool_name": tool}))
        };
        assert_eq!(event("Interrupt", ""), Some(HookStatus::Idle));
        assert_eq!(
            event("PreToolUse", "request_user_input"),
            Some(HookStatus::Blocked)
        );
        assert_eq!(
            event("PreToolUse", "shell"),
            None,
            "an ordinary tool is Claude's dialect: working"
        );
        assert_eq!(event("Stop", ""), None);
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

    #[test]
    fn steering_is_reported_per_connected_codex() {
        assert!(supports_turn_steer(STABLE_SCHEMA));
        assert!(!supports_turn_steer(SUPPORTED_SCHEMA));
        assert!(!supports_turn_steer("{"));
        assert_eq!(check_compatibility("0.166.0", STABLE_SCHEMA), Ok(()));
    }

    #[test]
    fn a_steer_is_sent_and_its_acknowledgement_read_from_the_fixture() {
        let request = turn_steer_request(7, "thread-1", "turn-active", "use the other file");
        assert_eq!(
            request,
            serde_json::json!({
                "id": 7,
                "method": "turn/steer",
                "params": {
                    "threadId": "thread-1",
                    "expectedTurnId": "turn-active",
                    "input": [{"type": "text", "text": "use the other file", "text_elements": []}],
                }
            })
        );
        let ack: Value = serde_json::from_str(include_str!("fixtures/turn-steer-ack.json")).unwrap();
        assert_eq!(ack["id"], request["id"]);
        assert_eq!(parse_turn_steer_ack(&ack), Ok("turn-active".to_owned()));
    }

    #[test]
    fn a_steer_acknowledgement_without_a_turn_id_is_refused() {
        let error = serde_json::json!({"id": 7, "error": {"code": -32600, "message": "no active turn"}});
        assert!(parse_turn_steer_ack(&error).is_err());
        assert!(parse_turn_steer_ack(&serde_json::json!({"id": 7, "result": {}})).is_err());
    }
}
