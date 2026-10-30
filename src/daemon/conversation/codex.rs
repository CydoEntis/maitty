use serde_json::Value;
use std::fmt;

pub(super) const VERIFIED_VERSIONS: &[&str] = &["0.164.0", "0.165.0"];
const VERIFIED_VERSIONS_TEXT: &str = "0.164.0 or 0.165.0";
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
pub(super) enum CompatibilityError {
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

pub(super) fn is_verified_version(version: &str) -> bool {
    VERIFIED_VERSIONS.contains(&version)
}

// This pure preflight has no session handle, so refusal cannot mutate a thread.
pub(super) fn check_compatibility(version: &str, schema: &str) -> Result<(), CompatibilityError> {
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
    const MISSING_INTERRUPT: &str = include_str!("fixtures/missing-interrupt.json");

    #[test]
    fn verified_versions_support_required_session_operations() {
        for version in VERIFIED_VERSIONS {
            assert_eq!(check_compatibility(version, SUPPORTED_SCHEMA), Ok(()));
        }
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
            check_compatibility("0.166.0", SUPPORTED_SCHEMA),
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
            "Unsupported Codex app-server: turn/interrupt. Select a verified Codex 0.164.0 or 0.165.0 executable; no session was started or resumed."
        );
    }
}
