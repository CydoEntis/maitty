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
        ("UserPromptSubmit", None),
        ("PreToolUse", None),
        ("PermissionRequest", None),
        ("PostToolUse", None),
        ("Stop", None),
        ("Interrupt", None),
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

/// The turn-lifecycle hooks, with the name Codex files each one under in `[hooks.state]`.
const LIFECYCLE: &[(&str, &str)] = &[
    ("UserPromptSubmit", "user_prompt_submit"),
    ("PreToolUse", "pre_tool_use"),
    ("PermissionRequest", "permission_request"),
    ("PostToolUse", "post_tool_use"),
    ("Stop", "stop"),
    ("Interrupt", "interrupt"),
];

/// Where Maitty's Codex lifecycle hooks stand, read from a Codex configuration.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HooksState {
    /// Some lifecycle hook is missing: Maitty reads Codex's screen.
    NotInstalled,
    /// All installed, but Codex has no recorded trust for at least one: Codex skips it until
    /// you review it in `/hooks`.
    NeedsTrust,
    /// All installed, and Codex records trust for each. The hash itself is not checked, so a
    /// hook edited since is still reported here.
    Trusted,
}

/// Read-only: whether Maitty's lifecycle hooks are in `text` and Codex has recorded trusting
/// them. A document that does not parse reads as not installed.
pub fn hooks_state(text: &str) -> HooksState {
    let Ok(document) = text.parse::<toml_edit::DocumentMut>() else {
        return HooksState::NotInstalled;
    };
    let Some(hooks) = document.get("hooks") else {
        return HooksState::NotInstalled;
    };
    let trusted = |event: &str, group: usize, handler: usize| {
        hooks
            .get("state")
            .and_then(|state| state.as_table_like())
            .is_some_and(|state| {
                state.iter().any(|(key, value)| {
                    key.ends_with(&format!("config.toml:{event}:{group}:{handler}"))
                        && value
                            .get("trusted_hash")
                            .and_then(|hash| hash.as_str())
                            .is_some_and(|hash| !hash.is_empty())
                })
            })
    };
    let mut all_trusted = true;
    for (event, filed_as) in LIFECYCLE {
        let Some(groups) = hooks
            .get(event)
            .and_then(|item| item.clone().into_value().ok())
            .and_then(|value| value.as_array().cloned())
        else {
            return HooksState::NotInstalled;
        };
        let ours = groups.iter().enumerate().find_map(|(group, entry)| {
            let handlers = entry.as_inline_table()?.get("hooks")?.as_array()?;
            handlers.iter().enumerate().find_map(|(handler, h)| {
                let h = h.as_inline_table()?;
                ["command", "commandWindows"]
                    .iter()
                    .filter_map(|key| h.get(key)?.as_str())
                    .any(|command| command.contains("hook codex"))
                    .then_some((group, handler))
            })
        });
        match ours {
            None => return HooksState::NotInstalled,
            Some((group, handler)) => all_trusted &= trusted(filed_as, group, handler),
        }
    }
    if all_trusted {
        HooksState::Trusted
    } else {
        HooksState::NeedsTrust
    }
}

#[cfg(test)]
mod tests {
    use super::{HooksState, StatusHookOptions, hooks_state, update_status_hooks};

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
    fn installs_the_whole_turn_lifecycle_and_reads_the_payload_for_each() {
        let linux = update_status_hooks("", options("/bin/maitty", false, false))
            .unwrap()
            .unwrap();
        for event in [
            "UserPromptSubmit",
            "PreToolUse",
            "PermissionRequest",
            "PostToolUse",
            "Stop",
            "Interrupt",
        ] {
            assert!(linux.contains(&format!("[[hooks.{event}]]")), "{event}");
        }
        assert_eq!(
            linux.matches("--status").count(),
            1,
            "only SessionStart has a fixed status; the rest say what happened in their payload"
        );
    }

    #[test]
    fn hooks_read_as_not_installed_then_untrusted_then_trusted() {
        assert_eq!(
            hooks_state(
                "model = 'gpt'
"
            ),
            HooksState::NotInstalled
        );
        assert_eq!(hooks_state("not [toml"), HooksState::NotInstalled);
        let installed = update_status_hooks("", options("/bin/maitty", false, false))
            .unwrap()
            .unwrap();
        assert_eq!(hooks_state(&installed), HooksState::NeedsTrust);
        let mut trusted = installed.clone();
        for (_, filed_as) in super::LIFECYCLE {
            trusted.push_str(&format!(
                "
[hooks.state.'/home/me/.codex/config.toml:{filed_as}:0:0']
trusted_hash = 'sha256:abc'
"
            ));
        }
        assert_eq!(hooks_state(&trusted), HooksState::Trusted);
        let partly = trusted.replace("trusted_hash = 'sha256:abc'", "enabled = true");
        assert_eq!(hooks_state(&partly), HooksState::NeedsTrust);
    }

    #[test]
    fn installing_twice_adds_nothing_the_second_time() {
        let once = update_status_hooks("", options("/bin/maitty", false, false))
            .unwrap()
            .unwrap();
        let twice = update_status_hooks(&once, options("/bin/maitty", false, false))
            .unwrap()
            .unwrap();
        assert_eq!(once, twice);
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
