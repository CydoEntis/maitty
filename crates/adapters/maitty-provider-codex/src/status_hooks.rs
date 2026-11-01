use anyhow::{Context, Result};

/// Inputs for projecting Maitty's Codex status handlers into a config document.
pub struct StatusHookOptions<'a> {
    pub executable: &'a str,
    pub uninstall: bool,
    pub windows: bool,
}

/// Adds or removes Maitty's Codex status hooks while preserving unrelated TOML.
///
/// Returns `None` when uninstalling a document with no hooks table; the caller
/// should then leave the file untouched.
pub fn update_status_hooks(text: &str, options: StatusHookOptions<'_>) -> Result<Option<String>> {
    const EVENTS: &[(&str, Option<&str>)] = &[
        ("SessionStart", Some("same")),
        ("SubagentStart", None),
        ("SubagentStop", None),
    ];
    const TIMEOUT_SECONDS: i64 = 3;

    let StatusHookOptions {
        executable,
        uninstall,
        windows,
    } = options;
    let mut document = text
        .parse::<toml_edit::DocumentMut>()
        .context("Codex configuration has invalid TOML; preserved")?;
    if !document.contains_key("hooks") {
        if uninstall {
            return Ok(None);
        }
        document["hooks"] = toml_edit::Item::Table(toml_edit::Table::new());
    }
    let hooks = document["hooks"]
        .as_table_mut()
        .context("Codex hooks must be a table; preserved")?;
    let command = format!(
        "{} hook codex",
        serde_json::to_string(executable).context("quote Codex hook executable")?
    );

    for (event, status) in EVENTS {
        if !hooks.contains_key(event) {
            if uninstall {
                continue;
            }
            hooks[*event] = toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new());
        }
        let entries = hooks[*event].as_array_of_tables_mut().with_context(|| {
            format!("Codex hooks.{event} must be an array of tables; preserved")
        })?;
        let mut owned_blocks = Vec::new();
        for (index, entry) in entries.iter_mut().enumerate() {
            let Some(handlers) = entry.get_mut("hooks") else {
                continue;
            };
            let handlers = handlers.as_array_of_tables_mut().with_context(|| {
                format!("Codex hooks.{event}.hooks must be an array of tables; preserved")
            })?;
            let before = handlers.len();
            handlers.retain(|handler| {
                let configured_command = handler.get("command").and_then(toml_edit::Item::as_str);
                handler.get("type").and_then(toml_edit::Item::as_str) != Some("command")
                    || !configured_command.is_some_and(|configured| {
                        configured == command || configured == format!("{command} --status same")
                    })
            });
            if handlers.len() != before {
                owned_blocks.push(index);
            }
        }
        for index in owned_blocks.into_iter().rev() {
            let only_our_empty_handlers = entries.iter().nth(index).is_some_and(|entry| {
                entry.len() == 1
                    && entry
                        .get("hooks")
                        .and_then(toml_edit::Item::as_array_of_tables)
                        .is_some_and(|handlers| handlers.is_empty())
            });
            if only_our_empty_handlers {
                entries.remove(index);
            }
        }
        if !uninstall {
            let mut handler = toml_edit::Table::new();
            handler["type"] = toml_edit::value("command");
            let handler_command = status.map_or_else(
                || command.clone(),
                |status| format!("{command} --status {status}"),
            );
            handler["command"] = toml_edit::value(&handler_command);
            if windows {
                let windows_command = status.map_or_else(
                    || format!("& \"{executable}\" hook codex"),
                    |status| format!("& \"{executable}\" hook codex --status {status}"),
                );
                handler["commandWindows"] = toml_edit::value(windows_command);
            }
            handler["timeout"] = toml_edit::value(TIMEOUT_SECONDS);
            let mut handlers = toml_edit::ArrayOfTables::new();
            handlers.push(handler);
            let mut entry = toml_edit::Table::new();
            entry["hooks"] = toml_edit::Item::ArrayOfTables(handlers);
            entries.push(entry);
        }
        if entries.is_empty() {
            hooks.remove(event);
        }
    }

    if hooks.is_empty() {
        document.remove("hooks");
    }
    Ok(Some(document.to_string()))
}

#[cfg(test)]
mod tests {
    use super::{StatusHookOptions, update_status_hooks};

    fn options(executable: &str, uninstall: bool, windows: bool) -> StatusHookOptions<'_> {
        StatusHookOptions {
            executable,
            uninstall,
            windows,
        }
    }

    #[test]
    fn adds_platform_specific_handlers_and_preserves_other_config() {
        let windows = update_status_hooks(
            "# keep this comment\nmodel = 'gpt'\n",
            options("C:\\Maitty\\maitty.exe", false, true),
        )
        .unwrap()
        .unwrap();
        assert!(windows.contains("# keep this comment"));
        assert!(
            windows.contains(
                "commandWindows = '& \"C:\\Maitty\\maitty.exe\" hook codex --status same'"
            )
        );

        let linux = update_status_hooks("", options("/home/user/bin/maitty", false, false))
            .unwrap()
            .unwrap();
        assert!(linux.contains("\"/home/user/bin/maitty\" hook codex --status same"));
        assert!(!linux.contains("commandWindows"));
    }

    #[test]
    fn uninstall_removes_only_maitty_handlers_and_leaves_other_hooks() {
        let installed = update_status_hooks(
            "[[hooks.SessionStart]]\n[[hooks.SessionStart.hooks]]\ntype = 'command'\ncommand = '\"/bin/maitty\" hook codex --status same'\n[[hooks.SessionStart.hooks]]\ntype = 'command'\ncommand = 'my-hook'\n",
            options("/bin/maitty", true, false),
        )
        .unwrap()
        .unwrap();
        assert!(!installed.contains("/bin/maitty"));
        assert!(installed.contains("my-hook"));
    }

    #[test]
    fn uninstall_without_hooks_leaves_the_document_untouched() {
        assert_eq!(
            update_status_hooks("model = 'gpt'\n", options("/bin/maitty", true, false)).unwrap(),
            None
        );
    }
}
