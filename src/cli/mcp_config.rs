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
use anyhow::{Context, Result};
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
        McpAgent { id: "qwen", name: "Qwen Code", config: super::home_settings("QWEN_HOME", ".qwen"), format: Format::McpServersJson },
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
        Format::McpServersJson => serde_json::from_str::<Value>(&text).ok()?.get("mcpServers")?.get(NAME)?.get("command")?.as_str().map(String::from),
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
    if path.exists() {
        std::fs::copy(path, path.with_extension(format!("{extension}.{NAME}-bak"))).with_context(|| format!("back up {}", path.display()))?;
    }
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
        let mut entry = toml_edit::Table::new();
        entry["command"] = toml_edit::value(exe);
        entry["args"] = toml_edit::value(ARGS.into_iter().collect::<toml_edit::Array>());
        servers.insert(NAME, toml_edit::Item::Table(entry));
    }
    back_up_and_write(path, "toml", doc.to_string())?;
    Ok(if uninstall { McpChange::Removed } else { McpChange::Registered })
}

fn json_at(path: &Path, exe: &str, uninstall: bool) -> Result<McpChange> {
    regular_target(path)?;
    let text = read_or_empty(path)?;
    let mut root: Value = if text.trim().is_empty() { json!({}) } else { serde_json::from_str(&text).with_context(|| format!("{} isn't valid JSON; preserved", path.display()))? };
    let settings = root.as_object_mut().with_context(|| format!("{} must hold a JSON object; preserved", path.display()))?;
    let entry = settings.get("mcpServers").and_then(|s| s.get(NAME));
    let current = entry.is_some_and(|e| e.get("command") == Some(&json!(exe)) && e.get("args") == Some(&json!(ARGS)));
    match (uninstall, entry.is_some()) {
        (false, _) if current => return Ok(McpChange::AlreadyRegistered),
        (true, false) => return Ok(McpChange::NotRegistered),
        _ => {}
    }
    let servers = settings
        .entry("mcpServers")
        .or_insert_with(|| json!({}))
        .as_object_mut()
        .with_context(|| format!("mcpServers in {} must be an object; preserved", path.display()))?;
    if uninstall {
        servers.remove(NAME);
        if servers.is_empty() {
            settings.remove("mcpServers");
        }
    } else {
        servers.insert(NAME.into(), json!({ "command": exe, "args": ARGS }));
    }
    back_up_and_write(path, "json", serde_json::to_string_pretty(&root)? + "\n")?;
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
