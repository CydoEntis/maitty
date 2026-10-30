use anyhow::{Result, bail};
use serde_json::{Map, Value, json};

pub(super) fn overrides(response: &Value) -> Result<Map<String, Value>> {
    let config = response
        .get("config")
        .and_then(Value::as_object)
        .ok_or_else(|| anyhow::anyhow!("Codex effective configuration response is unsupported"))?;
    let empty = Map::new();
    let raw_policy = match config.get("shell_environment_policy") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(policy)) => policy,
        _ => bail!("Codex shell environment policy has an unsupported shape"),
    };
    let policy: Map<String, Value> = raw_policy
        .iter()
        .filter(|(_, value)| !value.is_null())
        .map(|(key, value)| (key.clone(), value.clone()))
        .collect();
    if policy.keys().any(|key| {
        ![
            "inherit",
            "ignore_default_excludes",
            "exclude",
            "set",
            "include_only",
            "filters",
            "experimental_use_profile",
        ]
        .contains(&key.as_str())
    }) || policy.get("inherit").is_some_and(|value| value != "all")
        || policy
            .get("exclude")
            .is_some_and(|value| value.as_array().is_none_or(|values| !values.is_empty()))
        || policy
            .get("include_only")
            .is_some_and(|value| value.as_array().is_none_or(|values| !values.is_empty()))
        || policy
            .get("set")
            .is_some_and(|value| value.as_object().is_none_or(|values| !values.is_empty()))
        || policy.get("filters").is_some_and(|value| !value.is_null())
        || policy
            .get("experimental_use_profile")
            .is_some_and(|value| !value.is_null() && value != false)
    {
        bail!(
            "Unsupported Codex shell environment policy: structured identity must survive unchanged; use the native terminal for this policy"
        );
    }
    let ignore = match policy.get("ignore_default_excludes") {
        None => true,
        Some(Value::Bool(value)) => *value,
        _ => bail!("Unsupported Codex default environment filtering policy"),
    };
    let mut overrides = Map::new();
    overrides.insert("shell_environment_policy".into(),json!({"inherit":"all","ignore_default_excludes":ignore,"exclude":[],"set":{},"include_only":[],"experimental_use_profile":false}));
    let servers = match config.get("mcp_servers") {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(servers)) => servers,
        _ => bail!("Codex MCP configuration has an unsupported shape"),
    };
    for (name, server) in servers {
        let server = server
            .as_object()
            .ok_or_else(|| anyhow::anyhow!("Unsupported Codex MCP server configuration"))?;
        if server.get("enabled") == Some(&Value::Bool(false)) {
            continue;
        }
        if server.get("url").is_some_and(Value::is_string) {
            continue;
        }
        if !server.get("command").is_some_and(Value::is_string)
            || name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-'))
        {
            bail!("Unsupported Codex stdio MCP configuration for structured identity propagation");
        }
        if let Some(env) = server.get("env").filter(|value| !value.is_null()) {
            let env = env
                .as_object()
                .ok_or_else(|| anyhow::anyhow!("Unsupported Codex MCP environment"))?;
            if env.keys().any(|key| {
                crate::ipc::STRUCTURED_ENV
                    .iter()
                    .any(|reserved| key.eq_ignore_ascii_case(reserved))
            }) {
                bail!("Codex MCP environment overrides reserved structured identity");
            }
        }
        let mut variables = match server.get("env_vars") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(variables)) if variables.iter().all(Value::is_string) => {
                variables.clone()
            }
            _ => bail!("Unsupported Codex MCP inherited environment list"),
        };
        for key in crate::ipc::STRUCTURED_ENV
            .into_iter()
            .chain(crate::ipc::RUNTIME_ROUTE_ENV)
        {
            if !variables.iter().any(|value| value == key) {
                variables.push(json!(key));
            }
        }
        overrides.insert(
            format!("mcp_servers.{name}.env_vars"),
            Value::Array(variables),
        );
    }
    Ok(overrides)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn default_policy_and_existing_mcp_environment_are_preserved_without_secret_values() {
        let response = json!({"config":{"mcp_servers":{"synthetic":{"command":"server","args":["--stdio"],"env_vars":["USER_CHOICE"]}}}});
        let overrides = overrides(&response).unwrap();
        let variables = overrides["mcp_servers.synthetic.env_vars"]
            .as_array()
            .unwrap();
        assert_eq!(variables[0], "USER_CHOICE");
        for key in crate::ipc::STRUCTURED_ENV {
            assert!(variables.contains(&json!(key)));
        }
        for key in crate::ipc::RUNTIME_ROUTE_ENV {
            assert!(variables.contains(&json!(key)));
        }
        assert_eq!(overrides.len(), 2);
        assert!(!overrides.contains_key("mcp_servers.synthetic.command"));
    }
    #[test]
    fn filtering_shell_or_reserved_mcp_overrides_fail_closed() {
        for policy in [
            json!({"inherit":"none"}),
            json!({"exclude":["*"]}),
            json!({"include_only":["PATH"]}),
            json!({"set":{"USER_CHOICE":"value"}}),
            json!({"filters":{}}),
        ] {
            assert!(overrides(&json!({"config":{"shell_environment_policy":policy}})).is_err());
        }
        let mut env = Map::new();
        env.insert(crate::ipc::STRUCTURED_ENV[3].into(), json!("forged"));
        assert!(
            overrides(
                &json!({"config":{"mcp_servers":{"synthetic":{"command":"server","env":env}}}})
            )
            .is_err()
        );
        assert!(
            overrides(
                &json!({"config":{"shell_environment_policy":{"ignore_default_excludes":false}}})
            )
            .is_ok()
        );
    }
    #[test]
    fn serialized_absent_shell_policy_options_use_verified_native_defaults() {
        let response = json!({"config":{"shell_environment_policy":{"inherit":null,"ignore_default_excludes":null,"exclude":null,"set":null,"include_only":null,"filters":null,"experimental_use_profile":null}}});
        let defaults = overrides(&json!({"config":{}})).unwrap();
        assert_eq!(overrides(&response).unwrap(), defaults);
    }
}
