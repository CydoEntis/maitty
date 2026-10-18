//! Native status adapters installed explicitly through the CLI or the Actions menu.

use anyhow::{Context, Result, bail};
use std::path::PathBuf;
#[path = "opencode_config.rs"]
mod opencode_config;

const OWNED_HEADER: &str = "// Written by `seshi integrate";

fn install(path: PathBuf, source: &str, exe: &str, uninstall: bool) -> Result<PathBuf> {
    regular_target(&path)?;
    match std::fs::read_to_string(&path) {
        Ok(existing) if !existing.starts_with(OWNED_HEADER) => bail!(
            "{} already contains a different extension; keep it and choose another name",
            path.display()
        ),
        Ok(_) if uninstall => {
            std::fs::remove_file(&path).context("remove seshi's status extension")?;
            return Ok(path);
        }
        Ok(_) => {}
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            if uninstall {
                return Ok(path);
            }
        }
        Err(e) => return Err(e).context("read seshi's status extension"),
    }
    let source = source.replace("__SESHI_EXE__", &serde_json::to_string(exe)?);
    crate::config::write_atomic(&path, source).context("write seshi's status extension")?;
    Ok(path)
}

fn regular_target(path: &std::path::Path) -> Result<()> {
    match std::fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() => {
            bail!("{} must be a regular file; preserved", path.display())
        }
        Ok(_) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e).context("inspect status integration file"),
    }
}

pub(super) fn install_pi(exe: &str, uninstall: bool) -> Result<PathBuf> {
    let dir = std::env::var_os("PI_CODING_AGENT_DIR")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".pi/agent")))
        .context("locate Pi's configuration directory")?;
    install(
        dir.join("extensions/seshi.js"),
        include_str!("pi.js"),
        exe,
        uninstall,
    )
}

pub(super) fn install_omp(exe: &str, uninstall: bool) -> Result<PathBuf> {
    let home = directories::BaseDirs::new()
        .context("locate OMP configuration")?
        .home_dir()
        .to_path_buf();
    let pi = std::env::var_os("PI_CODING_AGENT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| home.join(".pi/agent"));
    let dir = std::env::var_os("PI_CODING_AGENT_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            home.join(std::env::var_os("PI_CONFIG_DIR").unwrap_or_else(|| ".omp".into()))
                .join("agent")
        });
    if dir == pi {
        bail!(
            "Pi and OMP share a configuration directory; choose separate directories before setting up OMP status"
        );
    }
    install(
        dir.join("extensions/seshi.js"),
        include_str!("omp.js"),
        exe,
        uninstall,
    )
}

pub(super) fn install_kilo(exe: &str, uninstall: bool) -> Result<PathBuf> {
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".config")))
        .context("locate Kilo configuration")?;
    install(
        dir.join("kilo/plugin/seshi.js"),
        include_str!("kilo.js"),
        exe,
        uninstall,
    )
}

pub(super) fn install_mastracode(exe: &str, uninstall: bool) -> Result<PathBuf> {
    let home = directories::BaseDirs::new().context("locate Mastracode configuration")?;
    mastracode_at(
        &home.home_dir().join(".mastracode/hooks.json"),
        exe,
        uninstall,
    )
}

fn mastracode_at(path: &std::path::Path, exe: &str, uninstall: bool) -> Result<PathBuf> {
    regular_target(path)?;
    let mut root: serde_json::Value = match std::fs::read_to_string(path) {
        Ok(text) => {
            serde_json::from_str(&text).context("Mastracode hooks are not valid JSON; preserved")?
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => serde_json::json!({}),
        Err(e) => return Err(e).context("read Mastracode hooks"),
    };
    let hooks = root
        .as_object_mut()
        .context("Mastracode hooks must be an object")?
        .entry("hooks")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .context("Mastracode hooks must contain an object")?;
    let events = [
        ("SessionStart", "same"),
        ("UserPromptSubmit", "working"),
        ("AgentStart", "working"),
        ("PreToolUse", "working"),
        ("PermissionRequest", "blocked"),
        ("PermissionResult", "working"),
        ("SubagentStart", "working"),
        ("SubagentEnd", "working"),
        ("Interrupt", "idle"),
        ("AgentEnd", "done"),
        ("Stop", "done"),
    ];
    for (event, status) in events {
        let entries = hooks
            .entry(event)
            .or_insert_with(|| serde_json::json!([]))
            .as_array_mut()
            .with_context(|| format!("Mastracode {event} must be a flat array; preserved"))?;
        entries.retain(|v| v.get("_seshi") != Some(&serde_json::Value::Bool(true)));
        if !uninstall {
            entries.push(serde_json::json!({"_seshi":true,"type":"command","command":format!("\"{exe}\" hook mastracode --status {status}"),"timeout":10000}));
        }
    }
    hooks.retain(|_, value| value.as_array().is_none_or(|a| !a.is_empty()));
    if path.exists() {
        std::fs::copy(path, path.with_extension("json.seshi-bak"))?;
    }
    crate::config::write_atomic(path, serde_json::to_string_pretty(&root)? + "\n")?;
    Ok(path.to_path_buf())
}

pub(super) fn install_kimi(exe: &str, uninstall: bool) -> Result<PathBuf> {
    let dir = std::env::var_os("KIMI_CODE_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".kimi-code")))
        .context("locate Kimi configuration")?;
    kimi_at(&dir.join("config.toml"), exe, uninstall)
}

fn kimi_at(path: &std::path::Path, exe: &str, uninstall: bool) -> Result<PathBuf> {
    regular_target(path)?;
    let text = match std::fs::read_to_string(path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context("read Kimi configuration"),
    };
    let mut doc = text
        .parse::<toml_edit::DocumentMut>()
        .context("Kimi config is invalid TOML; preserved")?;
    if !doc.contains_key("hooks") {
        doc["hooks"] = toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new());
    }
    let hooks = doc["hooks"]
        .as_array_of_tables_mut()
        .context("Kimi hooks must be an array of tables; preserved")?;
    hooks.retain(|table| table.get("_seshi").and_then(toml_edit::Item::as_bool) != Some(true));
    if !uninstall {
        for (event, status) in [
            ("SessionStart", "same"),
            ("UserPromptSubmit", "working"),
            ("PreToolUse", "working"),
            ("PostToolUse", "working"),
            ("PostToolUseFailure", "working"),
            ("PermissionRequest", "blocked"),
            ("PermissionResult", "working"),
            ("Stop", "done"),
            ("Interrupt", "idle"),
            ("SubagentStart", "working"),
            ("PreCompact", "working"),
        ] {
            let mut table = toml_edit::Table::new();
            table["_seshi"] = toml_edit::value(true);
            table["event"] = toml_edit::value(event);
            // AskUserQuestion must be blocked before the tool's permission event exists.
            if event == "PreToolUse" {
                table["matcher"] = toml_edit::value("^(?!AskUserQuestion$).*$");
            }
            table["command"] = toml_edit::value(format!("\"{exe}\" hook kimi --status {status}"));
            table["timeout"] = toml_edit::value(10);
            hooks.push(table);
        }
        let mut ask = toml_edit::Table::new();
        for (key, value) in [
            ("event", "PreToolUse"),
            ("matcher", "^AskUserQuestion$"),
            ("command", &format!("\"{exe}\" hook kimi --status blocked")),
        ] {
            ask[key] = toml_edit::value(value);
        }
        ask["_seshi"] = toml_edit::value(true);
        ask["timeout"] = toml_edit::value(10);
        hooks.push(ask);
    }
    if path.exists() {
        std::fs::copy(path, path.with_extension("toml.seshi-bak"))?;
    }
    crate::config::write_atomic(path, doc.to_string())?;
    Ok(path.to_path_buf())
}

pub(super) fn install_opencode(exe: &str, uninstall: bool) -> Result<PathBuf> {
    let dir = std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".config")))
        .context("locate OpenCode's configuration directory")?;
    let state = std::env::var_os("XDG_STATE_HOME")
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".local/state")))
        .context("locate OpenCode's state directory")?;
    install_opencode_at(
        &dir.join("opencode"),
        &state.join("opencode"),
        exe,
        uninstall,
    )
}

fn install_opencode_at(
    dir: &std::path::Path,
    state: &std::path::Path,
    exe: &str,
    uninstall: bool,
) -> Result<PathBuf> {
    opencode_config::validate_tui_plugin_config(dir)?;
    let assets = [
        (dir.join("plugins/seshi.js"), include_str!("opencode.js")),
        (
            dir.join("seshi-tui-session.js"),
            include_str!("opencode-tui.js"),
        ),
        (
            dir.join("seshi-opencode/tui.js"),
            "// Written by `seshi integrate opencode`: TUI v2 entrypoint.\nexport { default } from '../seshi-tui-session.js';\n",
        ),
    ];
    for (path, _) in &assets {
        regular_target(path)?;
        match std::fs::read_to_string(path) {
            Ok(text) if !text.starts_with(OWNED_HEADER) => {
                bail!("{} is a different extension; preserved", path.display())
            }
            Ok(_) => {}
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {}
            Err(e) => return Err(e).context("inspect OpenCode adapter before setup"),
        }
    }
    if uninstall {
        opencode_config::remove_tui_plugin(dir, "./seshi-tui-session.js")?;
        opencode_config::remove_cli_plugin(dir, "./seshi-opencode")?;
    }
    for (path, source) in assets {
        install(path, source, exe, uninstall)?;
    }
    if !uninstall {
        opencode_config::add_tui_plugin(dir, "./seshi-tui-session.js")?;
        opencode_config::add_cli_plugin(dir, state, "./seshi-opencode")?;
    }
    Ok(dir.join("plugins/seshi.js"))
}

pub(crate) fn install_status(agent: &str) -> Result<PathBuf> {
    let exe = super::this_exe()?;
    match agent {
        "pi" => install_pi(&exe, false),
        "opencode" => install_opencode(&exe, false),
        "omp" => install_omp(&exe, false),
        "mastracode" => install_mastracode(&exe, false),
        "kilo" => install_kilo(&exe, false),
        "kimi" => install_kimi(&exe, false),
        _ => bail!("unsupported status adapter: {agent}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn opencode_setup_preserves_jsonc_and_migration_then_removes_only_our_entries() {
        let dir =
            std::env::temp_dir().join(format!("seshi-opencode-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let state = dir.join("state");
        let config = dir.join("tui.jsonc");
        std::fs::write(
            &config,
            "{\n// keep my theme\n\"theme\":\"dark\",\"plugin\":[\"my-plugin\"],\n}\n",
        )
        .unwrap();
        install_opencode_at(&dir, &state, "/bin/seshi", false).unwrap();
        let once = std::fs::read_to_string(&config).unwrap();
        assert!(
            once.contains("// keep my theme")
                && once.contains("my-plugin")
                && once.contains("\"theme\":\"dark\"")
        );
        assert!(!dir.join("tui.json").exists());
        assert!(dir.join("cli.json").exists());
        install_opencode_at(&dir, &state, "/bin/seshi", false).unwrap();
        assert_eq!(std::fs::read_to_string(&config).unwrap(), once);
        install_opencode_at(&dir, &state, "/bin/seshi", true).unwrap();
        let removed = std::fs::read_to_string(&config).unwrap();
        assert!(removed.contains("my-plugin") && removed.contains("// keep my theme"));
        assert!(!removed.contains("seshi-tui-session"));
        assert!(!dir.join("plugins/seshi.js").exists());
        std::fs::write(dir.join("tui.json"), "{\"theme\":\"keep-for-migration\"}").unwrap();
        std::fs::remove_file(dir.join("cli.json")).unwrap();
        install_opencode_at(&dir, &state, "/bin/seshi", false).unwrap();
        assert!(
            !dir.join("cli.json").exists(),
            "do not prevent OpenCode importing V1 settings"
        );
        install_opencode_at(&dir, &state, "/bin/seshi", true).unwrap();
        std::fs::write(&config, "{\"plugin\":false}").unwrap();
        assert!(install_opencode_at(&dir, &state, "/bin/seshi", false).is_err());
        assert!(
            !dir.join("plugins/seshi.js").exists(),
            "invalid config must fail before writing assets"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn native_hook_configs_preserve_custom_hooks_and_use_each_apps_schema() {
        let dir = std::env::temp_dir().join(format!("seshi-native-config-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let mastra = dir.join("hooks.json");
        std::fs::write(
            &mastra,
            r#"{"hooks":{"Stop":[{"type":"command","command":"my hook"}]},"custom":true}"#,
        )
        .unwrap();
        for _ in 0..2 {
            mastracode_at(&mastra, "/bin/seshi", false).unwrap();
        }
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&mastra).unwrap()).unwrap();
        assert_eq!(root["hooks"]["Stop"].as_array().unwrap().len(), 2);
        assert_eq!(root["hooks"]["PermissionRequest"][0]["timeout"], 10000);
        assert!(
            root["hooks"]["PermissionRequest"][0].get("hooks").is_none(),
            "Mastracode uses flat command arrays"
        );
        mastracode_at(&mastra, "/bin/seshi", true).unwrap();
        let root: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&mastra).unwrap()).unwrap();
        assert_eq!(root["hooks"]["Stop"][0]["command"], "my hook");
        assert_eq!(root["custom"], true);
        let kimi = dir.join("config.toml");
        std::fs::write(&kimi, "# keep my config\nmodel = \"my-model\"\n[[hooks]]\nevent = \"Stop\"\ncommand = \"my hook\"\n").unwrap();
        for _ in 0..2 {
            kimi_at(&kimi, "/bin/seshi", false).unwrap();
        }
        let text = std::fs::read_to_string(&kimi).unwrap();
        assert!(text.contains("# keep my config"));
        let root: toml::Value = toml::from_str(&text).unwrap();
        assert_eq!(root["hooks"].as_array().unwrap().len(), 13);
        kimi_at(&kimi, "/bin/seshi", true).unwrap();
        let root: toml::Value = toml::from_str(&std::fs::read_to_string(&kimi).unwrap()).unwrap();
        assert_eq!(root["hooks"].as_array().unwrap().len(), 1);
        assert_eq!(root["hooks"][0]["command"].as_str(), Some("my hook"));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn status_extensions_preserve_other_files_and_quote_executable_paths() {
        let dir =
            std::env::temp_dir().join(format!("seshi-status-extension-{}", std::process::id()));
        let path = dir.join("extensions/seshi.js");
        let exe = r#"C:\Program Files\seshi\seshi.exe"#;
        let source = include_str!("pi.js");
        install(path.clone(), source, exe, false).unwrap();
        let written = std::fs::read_to_string(&path).unwrap();
        assert!(written.contains(&serde_json::to_string(exe).unwrap()));
        assert!(!written.contains("__SESHI_EXE__"));
        install(path.clone(), source, exe, false).unwrap();
        install(path.clone(), source, exe, true).unwrap();
        assert!(!path.exists());
        std::fs::write(&path, "// user's extension").unwrap();
        assert!(install(path.clone(), source, exe, false).is_err());
        assert!(install(path.clone(), source, exe, true).is_err());
        assert_eq!(
            std::fs::read_to_string(&path).unwrap(),
            "// user's extension"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }
}
