//! Registers the app's MCP server (`<app> mcp`) in the config files of agents that keep their
//! MCP servers in a file we can edit: Codex, Qwen Code and Gemini CLI. Claude Code is
//! registered through its own CLI instead (`claude mcp add`).
//!
//! Formats, checked against each tool's docs (Oct 2026):
//! - Codex: `config.toml`, `[mcp_servers.<name>]` with `command` and `args` (openai/codex,
//!   `codex-rs/config/src/mcp_types.rs`).
//! - Qwen Code and Gemini CLI: `settings.json`, `"mcpServers": { "<name>": { "command", "args" } }`
//!   (qwen-code `docs/users/features/mcp.md`, gemini-cli `docs/tools/mcp-server.md`).

use super::integrations::regular_target;
use anyhow::{Context, Result, anyhow};
use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstInputValue, CstObject, CstObjectProp, CstRootNode};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

const NAME: &str = env!("APP_COMMAND");
const ARGS: [&str; 1] = ["mcp"];

#[derive(Clone, Copy)]
enum Format {
    CodexToml,
    McpServersJson,
}

/// An agent whose MCP servers live in a config file.
pub(crate) struct McpAgent {
    pub(crate) id: &'static str,
    pub(crate) name: &'static str,
    pub(crate) config: PathBuf,
    format: Format,
}

impl McpAgent {
    /// Whether the agent is set up on this machine: its config folder exists.
    pub(crate) fn installed(&self) -> bool {
        self.config.parent().is_some_and(Path::is_dir)
    }
}

/// What registering (or removing) did to one agent's config.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum McpChange {
    Registered,
    AlreadyRegistered,
    Removed,
    NotRegistered,
    NotInstalled,
}

pub(crate) fn agents() -> [McpAgent; 3] {
    [
        McpAgent { id: "codex", name: "Codex", config: super::codex_config(), format: Format::CodexToml },
        McpAgent { id: "qwen", name: "Qwen Code", config: super::qwen_settings(), format: Format::McpServersJson },
        McpAgent { id: "gemini", name: "Gemini CLI", config: super::home_settings("GEMINI_CLI_HOME", ".gemini"), format: Format::McpServersJson },
    ]
}

/// Adds (or removes) the app's entry, leaving every other server and setting as it was. An
/// agent without a config folder is left alone rather than given one.
pub(crate) fn set_up(agent: &McpAgent, exe: &str, uninstall: bool) -> Result<McpChange> {
    if !agent.installed() {
        return Ok(McpChange::NotInstalled);
    }
    match agent.format {
        Format::CodexToml => codex_at(&agent.config, exe, uninstall),
        Format::McpServersJson => json_at(&agent.config, exe, uninstall),
    }
}

/// The program the app's entry runs, when there is one.
pub(crate) fn registered_command(agent: &McpAgent) -> Option<String> {
    let text = std::fs::read_to_string(&agent.config).ok()?;
    match agent.format {
        Format::CodexToml => toml::from_str::<toml::Value>(&text).ok()?.get("mcp_servers")?.get(NAME)?.get("command")?.as_str().map(String::from),
        Format::McpServersJson => parse_jsonc(&text)?.get("mcpServers")?.get(NAME)?.get("command")?.as_str().map(String::from),
    }
}

fn read_or_empty(path: &Path) -> Result<String> {
    match std::fs::read_to_string(path) {
        Ok(text) => Ok(text),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(String::new()),
        Err(e) => Err(e).with_context(|| format!("read {}", path.display())),
    }
}

fn back_up_and_write(path: &Path, extension: &str, text: String) -> Result<()> {
    super::back_up_once(path, extension).with_context(|| format!("back up {}", path.display()))?;
    crate::config::write_atomic(path, text).with_context(|| format!("write {}", path.display()))
}

fn codex_at(path: &Path, exe: &str, uninstall: bool) -> Result<McpChange> {
    regular_target(path)?;
    let mut doc: toml_edit::DocumentMut = read_or_empty(path)?.parse().with_context(|| format!("{} has invalid TOML; preserved", path.display()))?;
    let entry = doc.get("mcp_servers").and_then(|s| s.get(NAME));
    let current = entry.is_some_and(|e| {
        e.get("command").and_then(toml_edit::Item::as_str) == Some(exe)
            && e.get("args").and_then(toml_edit::Item::as_array).is_some_and(|a| a.iter().map(toml_edit::Value::as_str).eq(ARGS.map(Some)))
    });
    match (uninstall, entry.is_some()) {
        (false, _) if current => return Ok(McpChange::AlreadyRegistered),
        (true, false) => return Ok(McpChange::NotRegistered),
        _ => {}
    }
    if !doc.contains_key("mcp_servers") {
        let mut servers = toml_edit::Table::new();
        // Implicit, so the file gains `[mcp_servers.<app>]` and no empty `[mcp_servers]` header.
        servers.set_implicit(true);
        doc["mcp_servers"] = toml_edit::Item::Table(servers);
    }
    let servers = doc["mcp_servers"].as_table_mut().with_context(|| format!("mcp_servers in {} must be a table; preserved", path.display()))?;
    if uninstall {
        servers.remove(NAME);
        if servers.is_empty() {
            doc.remove("mcp_servers");
        }
    } else {
        let args = || toml_edit::value(ARGS.into_iter().collect::<toml_edit::Array>());
        // Only command and args are ours; env, enabled, timeouts the user added stay.
        match servers.get_mut(NAME).and_then(toml_edit::Item::as_table_like_mut) {
            Some(entry) => {
                entry.insert("command", toml_edit::value(exe));
                entry.insert("args", args());
            }
            None => {
                let mut entry = toml_edit::Table::new();
                entry["command"] = toml_edit::value(exe);
                entry["args"] = args();
                servers.insert(NAME, toml_edit::Item::Table(entry));
            }
        }
    }
    back_up_and_write(path, "toml", doc.to_string())?;
    Ok(if uninstall { McpChange::Removed } else { McpChange::Registered })
}

/// Qwen Code and Gemini CLI strip comments before parsing settings.json (strip-json-comments),
/// and Qwen a byte-order mark; trailing commas they reject, so they're rejected here too.
fn jsonc_options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: false,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
    }
}

fn split_bom(text: &str) -> (&str, &str) {
    match text.strip_prefix('\u{feff}') {
        Some(body) => ("\u{feff}", body),
        None => ("", text),
    }
}

fn parse_jsonc(text: &str) -> Option<Value> {
    CstRootNode::parse(split_bom(text).1, &jsonc_options()).ok()?.to_serde_value()
}

fn set_prop(object: &CstObject, key: &str, value: CstInputValue) {
    match object.get(key) {
        Some(prop) => prop.set_value(value),
        None => {
            object.append(key, value);
        }
    }
}

/// Edits the file's syntax tree rather than re-serialising it, so comments and layout stay.
fn json_at(path: &Path, exe: &str, uninstall: bool) -> Result<McpChange> {
    regular_target(path)?;
    let text = read_or_empty(path)?;
    let (bom, body) = split_bom(&text);
    let body = if body.trim().is_empty() { "{}\n" } else { body };
    let root = CstRootNode::parse(body, &jsonc_options()).map_err(|e| anyhow!("{} isn't valid JSON ({e}); preserved", path.display()))?;
    let settings = root.value().and_then(|v| v.as_object()).with_context(|| format!("{} must hold a JSON object; preserved", path.display()))?;
    let servers = match settings.get("mcpServers") {
        Some(prop) => Some(prop.object_value().with_context(|| format!("mcpServers in {} must be an object; preserved", path.display()))?),
        None => None,
    };
    let entry = servers.as_ref().and_then(|s| s.get(NAME));
    let value = entry.as_ref().and_then(|e| e.value()).and_then(|v| v.to_serde_value());
    let current = value.is_some_and(|e| e.get("command") == Some(&json!(exe)) && e.get("args") == Some(&json!(ARGS)));
    match (uninstall, entry.is_some()) {
        (false, _) if current => return Ok(McpChange::AlreadyRegistered),
        (true, false) => return Ok(McpChange::NotRegistered),
        _ => {}
    }
    let args = || CstInputValue::Array(ARGS.map(CstInputValue::from).into());
    if uninstall {
        entry.into_iter().for_each(CstObjectProp::remove);
        if servers.is_some_and(|s| s.properties().is_empty()) {
            settings.get("mcpServers").into_iter().for_each(CstObjectProp::remove);
        }
    } else {
        let servers = servers.unwrap_or_else(|| settings.append("mcpServers", CstInputValue::Object(Vec::new())).object_value_or_set());
        // Only command and args are ours; trust, timeout and the rest the user added stay.
        match entry.as_ref().and_then(CstObjectProp::object_value) {
            Some(object) => {
                set_prop(&object, "command", exe.into());
                set_prop(&object, "args", args());
            }
            None => {
                entry.into_iter().for_each(CstObjectProp::remove);
                servers.append(NAME, CstInputValue::Object(vec![("command".into(), exe.into()), ("args".into(), args())]));
            }
        }
    }
    back_up_and_write(path, "json", format!("{bom}{root}"))?;
    Ok(if uninstall { McpChange::Removed } else { McpChange::Registered })
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXE: &str = concat!(r"C:\Program Files\", env!("APP_NAME"), r"\", env!("APP_COMMAND"), ".exe");

    fn temp(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{NAME}-mcp-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn agent(dir: &Path, file: &str, format: Format) -> McpAgent {
        McpAgent { id: "test", name: "test", config: dir.join(file), format }
    }

    #[test]
    fn codex_registration_keeps_other_servers_and_comments_and_is_idempotent() {
        let dir = temp("codex");
        let codex = agent(&dir, "config.toml", Format::CodexToml);
        std::fs::write(&codex.config, "# keep me\nmodel = \"gpt-5\"\n\n[mcp_servers.docs]\ncommand = \"npx\"\nargs = [\"docs-mcp\"]\n").unwrap();

        assert_eq!(set_up(&codex, EXE, false).unwrap(), McpChange::Registered);
        let once = std::fs::read_to_string(&codex.config).unwrap();
        assert_eq!(set_up(&codex, EXE, false).unwrap(), McpChange::AlreadyRegistered);
        assert_eq!(std::fs::read_to_string(&codex.config).unwrap(), once, "no duplicate on re-register");

        assert!(once.contains("# keep me") && once.contains(&format!("[mcp_servers.{NAME}]")));
        let root: toml::Value = toml::from_str(&once).unwrap();
        assert_eq!(root["mcp_servers"][NAME]["command"].as_str(), Some(EXE), "backslashes survive TOML escaping");
        assert_eq!(root["mcp_servers"][NAME]["args"].as_array().unwrap(), &vec![toml::Value::from("mcp")]);
        assert_eq!(registered_command(&codex).as_deref(), Some(EXE));

        assert_eq!(set_up(&codex, EXE, true).unwrap(), McpChange::Removed);
        let removed = std::fs::read_to_string(&codex.config).unwrap();
        assert!(removed.contains("# keep me") && removed.contains("[mcp_servers.docs]") && !removed.contains(NAME));
        assert_eq!(set_up(&codex, EXE, true).unwrap(), McpChange::NotRegistered);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn codex_registration_into_a_file_without_servers_adds_no_empty_header() {
        let dir = temp("codex-new");
        let codex = agent(&dir, "config.toml", Format::CodexToml);
        set_up(&codex, "/usr/local/bin/app", false).unwrap();
        let text = std::fs::read_to_string(&codex.config).unwrap();
        assert!(!text.contains("[mcp_servers]\n"), "{text}");
        set_up(&codex, "/usr/local/bin/app", true).unwrap();
        assert!(!std::fs::read_to_string(&codex.config).unwrap().contains("mcp_servers"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn json_registration_keeps_other_servers_and_settings_and_is_idempotent() {
        let dir = temp("json");
        let qwen = agent(&dir, "settings.json", Format::McpServersJson);
        std::fs::write(&qwen.config, r#"{"theme":"dark","mcpServers":{"docs":{"command":"npx","args":["docs-mcp"]}},"hooks":{"Stop":[]}}"#).unwrap();

        assert_eq!(set_up(&qwen, EXE, false).unwrap(), McpChange::Registered);
        let once = std::fs::read_to_string(&qwen.config).unwrap();
        assert_eq!(set_up(&qwen, EXE, false).unwrap(), McpChange::AlreadyRegistered);
        assert_eq!(std::fs::read_to_string(&qwen.config).unwrap(), once, "no duplicate on re-register");
        let root: Value = serde_json::from_str(&once).unwrap();
        assert_eq!(root["mcpServers"][NAME], json!({ "command": EXE, "args": ["mcp"] }));
        assert_eq!(root["mcpServers"]["docs"]["command"], "npx");
        assert_eq!(root["theme"], "dark");

        assert_eq!(set_up(&qwen, EXE, true).unwrap(), McpChange::Removed);
        let root: Value = serde_json::from_str(&std::fs::read_to_string(&qwen.config).unwrap()).unwrap();
        assert!(root["mcpServers"].get(NAME).is_none());
        assert_eq!(root["mcpServers"]["docs"]["command"], "npx");
        assert_eq!(root["theme"], "dark");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn commented_settings_register_and_keep_their_comments_and_bom() {
        let dir = temp("jsonc");
        let gemini = agent(&dir, "settings.json", Format::McpServersJson);
        let original = "\u{feff}{\n  // my theme\n  \"theme\": \"dark\",\n  /* servers */\n  \"mcpServers\": {\n    \"docs\": { \"command\": \"npx\" } // docs\n  }\n}\n";
        std::fs::write(&gemini.config, original).unwrap();

        assert_eq!(set_up(&gemini, EXE, false).unwrap(), McpChange::Registered);
        let text = std::fs::read_to_string(&gemini.config).unwrap();
        assert!(text.starts_with('\u{feff}') && text.contains("// my theme") && text.contains("/* servers */") && text.contains("// docs"), "{text}");
        assert_eq!(registered_command(&gemini).as_deref(), Some(EXE));
        assert_eq!(set_up(&gemini, EXE, false).unwrap(), McpChange::AlreadyRegistered);

        assert_eq!(set_up(&gemini, EXE, true).unwrap(), McpChange::Removed);
        let text = std::fs::read_to_string(&gemini.config).unwrap();
        assert!(text.contains("// my theme") && text.contains("\"docs\"") && !text.contains(NAME), "{text}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn re_registering_keeps_fields_the_user_added() {
        let dir = temp("fields");
        let codex = agent(&dir, "config.toml", Format::CodexToml);
        std::fs::write(&codex.config, format!("[mcp_servers.{NAME}]\ncommand = \"/old/app\"\nargs = [\"mcp\"]\nstartup_timeout_sec = 20\nenv = {{ A = \"1\" }}\n")).unwrap();
        assert_eq!(set_up(&codex, EXE, false).unwrap(), McpChange::Registered);
        let root: toml::Value = toml::from_str(&std::fs::read_to_string(&codex.config).unwrap()).unwrap();
        let entry = &root["mcp_servers"][NAME];
        assert_eq!(entry["command"].as_str(), Some(EXE));
        assert_eq!(entry["startup_timeout_sec"].as_integer(), Some(20));
        assert_eq!(entry["env"]["A"].as_str(), Some("1"));

        let qwen = agent(&dir, "settings.json", Format::McpServersJson);
        std::fs::write(&qwen.config, format!(r#"{{"mcpServers":{{"{NAME}":{{"command":"/old/app","args":[],"trust":true,"timeout":30000}}}}}}"#)).unwrap();
        assert_eq!(set_up(&qwen, EXE, false).unwrap(), McpChange::Registered);
        let root: Value = serde_json::from_str(&std::fs::read_to_string(&qwen.config).unwrap()).unwrap();
        assert_eq!(root["mcpServers"][NAME], json!({ "command": EXE, "args": ["mcp"], "trust": true, "timeout": 30000 }));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_backup_is_the_file_before_the_first_change() {
        let dir = temp("backup");
        let qwen = agent(&dir, "settings.json", Format::McpServersJson);
        std::fs::write(&qwen.config, "{\"theme\":\"dark\"}").unwrap();
        set_up(&qwen, "/old/app", false).unwrap();
        set_up(&qwen, "/new/app", false).unwrap();
        let backup = dir.join(format!("settings.json.{NAME}-bak"));
        assert_eq!(std::fs::read_to_string(backup).unwrap(), "{\"theme\":\"dark\"}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_moved_install_is_repointed() {
        let dir = temp("moved");
        let gemini = agent(&dir, "settings.json", Format::McpServersJson);
        set_up(&gemini, "/old/app", false).unwrap();
        assert_eq!(set_up(&gemini, "/new/app", false).unwrap(), McpChange::Registered);
        assert_eq!(registered_command(&gemini).as_deref(), Some("/new/app"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_agent_without_a_config_folder_is_skipped_and_left_without_one() {
        let dir = temp("absent");
        let missing = dir.join("no-such-agent");
        for (file, format) in [("config.toml", Format::CodexToml), ("settings.json", Format::McpServersJson)] {
            let absent = agent(&missing, file, format);
            assert_eq!(set_up(&absent, EXE, false).unwrap(), McpChange::NotInstalled);
            assert_eq!(set_up(&absent, EXE, true).unwrap(), McpChange::NotInstalled);
        }
        assert!(!missing.exists(), "no config folder created");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_broken_config_is_preserved() {
        let dir = temp("broken");
        let qwen = agent(&dir, "settings.json", Format::McpServersJson);
        std::fs::write(&qwen.config, "{ not json").unwrap();
        assert!(set_up(&qwen, EXE, false).is_err());
        assert_eq!(std::fs::read_to_string(&qwen.config).unwrap(), "{ not json");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
