// SPDX-License-Identifier: Apache-2.0
// Adapted from Herdr 143bf5fd4781d164260c63fe28e7dc08054ad32f; see vendor/herdr-detection/NOTICE.md.
// Local changes: the app atomic writes and file validation; omit unused public inspection helpers.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use jsonc_parser::ParseOptions;
use jsonc_parser::cst::{CstInputValue, CstRootNode};
use serde_json::Value;

fn check_config_target(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(meta) if !meta.is_file() => Err(io::Error::other(format!(
            "{} must be a regular config file",
            path.display()
        ))),
        Ok(_) => Ok(()),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(e),
    }
}

fn write_config(path: &Path, text: String) -> io::Result<()> {
    crate::config::write_atomic(path, text)
}

const TUI_CONFIG_NAME: &str = "tui.jsonc";

fn tui_config_paths(config_dir: &Path) -> [PathBuf; 2] {
    [
        config_dir.join(TUI_CONFIG_NAME),
        config_dir.join("tui.json"),
    ]
}

pub(crate) fn validate_tui_plugin_config(config_dir: &Path) -> io::Result<()> {
    for path in tui_config_paths(config_dir) {
        validate_plugin_config(&path, "plugin")?;
    }
    validate_plugin_config(&config_dir.join("cli.json"), "plugins")
}

fn validate_plugin_config(config_path: &Path, key: &str) -> io::Result<()> {
    check_config_target(config_path)?;
    if !config_path.is_file() {
        return Ok(());
    }

    let content = fs::read_to_string(config_path)?;
    let root = parse_root(&content, config_path)?;
    let object = root_object(&root, config_path)?;
    if object
        .get(key)
        .is_some_and(|property| property.array_value().is_none())
    {
        return Err(invalid_plugin_list(config_path));
    }
    Ok(())
}

pub(crate) fn add_tui_plugin(config_dir: &Path, plugin_spec: &str) -> io::Result<PathBuf> {
    for path in tui_config_paths(config_dir) {
        if plugin_is_configured(&path, "plugin", plugin_spec) {
            return Ok(path);
        }
    }
    // Keep tui.json absent on fresh installs so OpenCode can migrate its settings.
    add_plugin(config_dir.join(TUI_CONFIG_NAME), "plugin", plugin_spec)
}

pub(crate) fn add_cli_plugin(
    config_dir: &Path,
    state_dir: &Path,
    plugin_spec: &str,
) -> io::Result<Option<PathBuf>> {
    let path = config_dir.join("cli.json");
    check_config_target(&path)?;
    // OpenCode imports V1 TUI preferences (`tui.json`, `kv.json`) into cli.json on
    // its first V2 start, but only while cli.json is absent. Defer registration
    // while those sources still exist so we do not skip the migration; otherwise
    // create cli.json ourselves, since OpenCode will never do it for a fresh V2
    // install with nothing to migrate.
    if !path.is_file() && cli_migration_pending(config_dir, state_dir) {
        return Ok(None);
    }
    add_plugin(path, "plugins", plugin_spec).map(Some)
}

fn cli_migration_pending(config_dir: &Path, state_dir: &Path) -> bool {
    config_dir.join("tui.json").is_file() || state_dir.join("kv.json").is_file()
}

fn add_plugin(config_path: PathBuf, key: &str, plugin_spec: &str) -> io::Result<PathBuf> {
    check_config_target(&config_path)?;
    let content = if config_path.is_file() {
        fs::read_to_string(&config_path)?
    } else {
        "{}\n".to_string()
    };
    let root = parse_root(&content, &config_path)?;
    let object = root_object(&root, &config_path)?;

    match object.get(key) {
        Some(property) => {
            let plugins = property
                .array_value()
                .ok_or_else(|| invalid_plugin_list(&config_path))?;
            if plugins.elements().iter().any(|entry| {
                entry
                    .to_serde_value()
                    .is_some_and(|entry| plugin_entry_matches(&entry, plugin_spec))
            }) {
                return Ok(config_path);
            }
            plugins.append(CstInputValue::String(plugin_spec.to_string()));
        }
        None => {
            object.append(
                key,
                CstInputValue::Array(vec![CstInputValue::String(plugin_spec.to_string())]),
            );
        }
    }

    write_config(&config_path, root.to_string())?;
    Ok(config_path)
}

pub(crate) fn remove_tui_plugin(config_dir: &Path, plugin_spec: &str) -> io::Result<Vec<PathBuf>> {
    let mut updated = Vec::new();
    let mut errors = Vec::new();
    for path in tui_config_paths(config_dir) {
        match remove_plugin(&path, "plugin", plugin_spec) {
            Ok(true) => updated.push(path),
            Ok(false) => {}
            Err(err) => errors.push(err.to_string()),
        }
    }
    if errors.is_empty() {
        Ok(updated)
    } else {
        Err(io::Error::other(errors.join("; ")))
    }
}

pub(crate) fn remove_cli_plugin(config_dir: &Path, plugin_spec: &str) -> io::Result<bool> {
    remove_plugin(&config_dir.join("cli.json"), "plugins", plugin_spec)
}

fn remove_plugin(config_path: &Path, key: &str, plugin_spec: &str) -> io::Result<bool> {
    check_config_target(config_path)?;
    if !config_path.is_file() {
        return Ok(false);
    }

    let content = fs::read_to_string(config_path)?;
    let root = parse_root(&content, config_path)?;
    let object = root_object(&root, config_path)?;
    let Some(property) = object.get(key) else {
        return Ok(false);
    };
    let plugins = property
        .array_value()
        .ok_or_else(|| invalid_plugin_list(config_path))?;
    let mut removed = false;
    for entry in plugins.elements() {
        if entry
            .to_serde_value()
            .is_some_and(|entry| plugin_entry_matches(&entry, plugin_spec))
        {
            entry.remove();
            removed = true;
        }
    }
    if !removed {
        return Ok(false);
    }
    if plugins.elements().is_empty() {
        property.remove();
    }

    write_config(config_path, root.to_string())?;
    Ok(true)
}

fn plugin_is_configured(config_path: &Path, key: &str, plugin_spec: &str) -> bool {
    let Ok(content) = fs::read_to_string(config_path) else {
        return false;
    };
    let Ok(root) = parse_root(&content, config_path) else {
        return false;
    };
    let Ok(object) = root_object(&root, config_path) else {
        return false;
    };
    object
        .get(key)
        .and_then(|property| property.array_value())
        .is_some_and(|plugins| {
            plugins.elements().iter().any(|entry| {
                entry
                    .to_serde_value()
                    .is_some_and(|entry| plugin_entry_matches(&entry, plugin_spec))
            })
        })
}

fn parse_root(content: &str, path: &Path) -> io::Result<CstRootNode> {
    CstRootNode::parse(content, &jsonc_parse_options()).map_err(|err| {
        io::Error::other(format!(
            "failed to parse OpenCode TUI config at {}: {err}",
            path.display()
        ))
    })
}

fn root_object(root: &CstRootNode, path: &Path) -> io::Result<jsonc_parser::cst::CstObject> {
    root.value()
        .and_then(|value| value.as_object())
        .ok_or_else(|| invalid_root(path))
}

fn jsonc_parse_options() -> ParseOptions {
    ParseOptions {
        allow_comments: true,
        allow_loose_object_property_names: false,
        allow_trailing_commas: true,
        allow_missing_commas: false,
        allow_single_quoted_strings: false,
        allow_hexadecimal_numbers: false,
        allow_unary_plus_numbers: false,
    }
}

fn plugin_entry_matches(entry: &Value, plugin_spec: &str) -> bool {
    entry.as_str() == Some(plugin_spec)
        || entry.get("package").and_then(Value::as_str) == Some(plugin_spec)
        || entry
            .as_array()
            .and_then(|parts| parts.first())
            .and_then(Value::as_str)
            == Some(plugin_spec)
}

fn invalid_root(path: &Path) -> io::Error {
    io::Error::other(format!(
        "OpenCode TUI config at {} must be a JSON object",
        path.display()
    ))
}

fn invalid_plugin_list(path: &Path) -> io::Error {
    io::Error::other(format!(
        "OpenCode TUI config plugin list at {} must be an array",
        path.display()
    ))
}
