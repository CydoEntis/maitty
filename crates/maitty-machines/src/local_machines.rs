//! Reading this computer's machine list: the `[[machines]]` entries of `config.local.toml`, as
//! the terminal app's loader reads them. Both apps use it, so there is one list (ADR 0048).
//! Hand edits apply when the app starts.

use crate::brand::{APP_COMMAND, APP_ENV};
use crate::def::{MachineDef, validate_machines};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// The machines of this computer and, when the file is bad, what is wrong with it. A bad entry
/// leaves no machines, and the app starts without them.
#[derive(Debug, Default, PartialEq)]
pub struct MachineList {
    pub machines: Vec<MachineDef>,
    pub error: Option<String>,
}

/// Where the app's main config file is: the file named by the environment, else the folder
/// the config loader uses on this platform.
pub fn config_path() -> PathBuf {
    config_path_in(
        std::env::var_os(format!("{APP_ENV}_CONFIG")),
        std::env::var_os("XDG_CONFIG_HOME"),
        cfg!(windows),
    )
}

fn config_path_in(
    own_file: Option<std::ffi::OsString>,
    xdg_config_home: Option<std::ffi::OsString>,
    windows: bool,
) -> PathBuf {
    if let Some(path) = own_file {
        return PathBuf::from(path);
    }
    if windows && let Some(dirs) = directories::BaseDirs::new() {
        return dirs.config_dir().join(APP_COMMAND).join("config.toml");
    }
    // XDG-style on macOS too: that's where terminal people look.
    let base = xdg_config_home
        .map(PathBuf::from)
        .or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".config")))
        .unwrap_or_else(|| PathBuf::from("."));
    base.join(APP_COMMAND).join("config.toml")
}

/// This computer's machine list. As in the terminal app, a config file named by the
/// environment stands alone: the local file beside it is not read, so there are no machines.
pub fn local_machines() -> MachineList {
    if std::env::var_os(format!("{APP_ENV}_CONFIG")).is_some() {
        return MachineList::default();
    }
    machines_beside(&config_path())
}

/// The machines of the `config.local.toml` beside `config`. A missing file is no machines; one
/// that cannot be read or has a bad entry is an error naming it and what to change.
pub fn machines_beside(config: &Path) -> MachineList {
    match read_machines(&config.with_file_name("config.local.toml")) {
        Ok(machines) => MachineList {
            machines,
            error: None,
        },
        Err(error) => MachineList {
            machines: Vec::new(),
            error: Some(format!("{error:#}")),
        },
    }
}

fn read_machines(file: &Path) -> Result<Vec<MachineDef>> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e).with_context(|| format!("reading {}", file.display())),
    };
    let table: toml::Table =
        toml::from_str(&text).with_context(|| format!("parsing {}", file.display()))?;
    let Some(machines) = table.get("machines") else {
        return Ok(Vec::new());
    };
    let machines: Vec<MachineDef> = machines
        .clone()
        .try_into()
        .with_context(|| format!("parsing {}", file.display()))?;
    validate_machines(&machines)?;
    Ok(machines)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-local-machines-{}-{}"),
            label,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn list_of(local: &str) -> MachineList {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir = dir(&format!("list{n}"));
        std::fs::write(dir.join("config.local.toml"), local).unwrap();
        machines_beside(&dir.join("config.toml"))
    }

    #[test]
    fn no_file_is_no_machines_and_no_error() {
        let dir = dir("none");
        assert_eq!(
            machines_beside(&dir.join("config.toml")),
            MachineList::default()
        );
    }

    #[test]
    fn the_entries_of_the_local_file_are_the_list() {
        let list = list_of(
            "theme = 'night'\n[[machines]]\nname = 'build'\ntarget = 'me@build'\nssh_args = ['-p', '2222']\nclipboard = false\n\n[[machines]]\nname = 'box'\ntarget = 'box'\n",
        );
        assert_eq!(list.error, None);
        let names: Vec<_> = list.machines.iter().map(|m| m.name.as_str()).collect();
        assert_eq!(names, ["build", "box"]);
        assert_eq!(list.machines[0].ssh_args, ["-p", "2222"]);
        assert!(!list.machines[0].clipboard && list.machines[1].clipboard);
    }

    #[test]
    fn a_file_without_machines_is_no_machines() {
        assert_eq!(list_of("theme = 'night'\n"), MachineList::default());
    }

    #[test]
    fn a_bad_entry_is_an_error_naming_it_and_leaves_no_machines() {
        let list = list_of(
            "[[machines]]\nname = 'good'\ntarget = 'good'\n\n[[machines]]\nname = 'bad'\ntarget = '-oProxyCommand=x'\n",
        );
        assert!(list.machines.is_empty());
        let error = list.error.expect("a bad entry is reported");
        assert!(error.contains("bad"), "{error}");
    }

    #[test]
    fn a_syntax_error_is_reported_with_the_file() {
        let list = list_of("[[machines]\n");
        assert!(list.machines.is_empty());
        let error = list.error.expect("a syntax error is reported");
        assert!(error.contains("config.local.toml"), "{error}");
    }

    #[test]
    fn a_name_that_is_this_computer_is_refused() {
        let list = list_of("[[machines]]\nname = 'local'\ntarget = 'x'\n");
        assert!(list.error.is_some());
    }

    #[test]
    fn the_environment_file_wins_and_the_default_follows_xdg() {
        assert_eq!(
            config_path_in(Some("/x/c.toml".into()), Some("/cfg".into()), false),
            PathBuf::from("/x/c.toml")
        );
        assert_eq!(
            config_path_in(None, Some("/cfg".into()), false),
            PathBuf::from("/cfg").join(APP_COMMAND).join("config.toml")
        );
    }
}
