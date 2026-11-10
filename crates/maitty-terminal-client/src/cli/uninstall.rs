//! `<app> uninstall`: take the app off this computer in one step. It stops the server, takes the
//! app's entries out of every agent's config and removes the program. Settings and saved sessions
//! stay unless `--purge` is given, so installing again picks up where this left off.
//!
//! The desktop app is its own install with its own uninstaller; this only says where that is.

use crate::brand::{APP_COMMAND, APP_NAME};
use crate::cli::target::Target;
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};

/// What an uninstall will touch, worked out before anything is changed.
#[derive(Debug)]
struct Plan {
    /// The installed program. `None` for a build run from its build folder, which is left alone.
    program: Option<PathBuf>,
    /// The sessions open on the running server; `None` when no server runs.
    sessions: Option<usize>,
    purge: bool,
    config: PathBuf,
    data: PathBuf,
}

impl Plan {
    fn here(purge: bool) -> Result<Plan> {
        let program = maitty_core::installed_executable::installed_executable()?;
        let config = maitty_core::config::config_path();
        Ok(Plan {
            program: (!is_build_folder(program.parent())).then_some(program),
            sessions: crate::cli::server_running()
                .then(|| crate::restart::read().map_or(0, |a| a.sessions)),
            purge,
            config: config.parent().map(Path::to_path_buf).unwrap_or(config),
            data: maitty_core::config::data_dir(),
        })
    }

    /// What is about to happen, said before the question.
    fn lines(&self) -> Vec<String> {
        let mut lines = vec![format!("This removes {APP_NAME} from this computer:")];
        if let Some(sessions) = self.sessions {
            let ends = match (sessions, self.purge) {
                (0, _) => "no session is open".to_string(),
                (n, true) => format!("{n} open session(s) end for good"),
                (n, false) => {
                    format!("{n} open session(s) end; they are saved and come back if you install again")
                }
            };
            lines.push(format!("  - stops the server ({ends})"));
        }
        if !crate::cli::update::integrate_opted_out() {
            lines.push(format!(
                "  - takes {APP_NAME}'s hooks and MCP entry out of your agents' config"
            ));
        }
        lines.push(match &self.program {
            Some(program) => format!("  - removes {}", program.display()),
            None => "  - leaves this build where it is (it runs from its build folder)".to_string(),
        });
        let kept = format!(
            "your settings ({}) and saved sessions ({})",
            self.config.display(),
            self.data.display()
        );
        lines.push(if self.purge {
            format!("  - deletes {kept}")
        } else {
            format!("  - keeps {kept}; `{APP_COMMAND} uninstall --purge` deletes them too")
        });
        lines.push(format!(
            "The {APP_NAME} desktop app, if you installed it, is separate: {DESKTOP_REMOVAL}."
        ));
        lines
    }
}

const DESKTOP_REMOVAL: &str = if cfg!(windows) {
    "remove it in Settings > Apps > Installed apps"
} else if cfg!(target_os = "macos") {
    "drag it out of Applications"
} else {
    "remove it with your package manager"
};

/// Whether `dir` is where cargo builds: a program there is a working copy, not an install.
fn is_build_folder(dir: Option<&Path>) -> bool {
    dir.is_some_and(|dir| dir.join("deps").is_dir() && dir.join(".fingerprint").is_dir())
}

/// Whether a folder is the app's own: it is named after the app, or sits right inside one that
/// is (the data folder on Windows). A folder shared with other things is never deleted.
fn owned_by_app(dir: &Path) -> bool {
    let named = |dir: &Path| {
        dir.file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case(APP_COMMAND))
    };
    named(dir) || dir.parent().is_some_and(named)
}

/// The settings that point the app's files somewhere of the user's choosing. What they name
/// could be any folder (a repo, a synced drive), so `--purge` never deletes it.
const PLACED_BY_USER: [&str; 2] = [
    concat!(env!("APP_ENV"), "_CONFIG"),
    concat!(env!("APP_ENV"), "_DATA_DIR"),
];

/// Delete one of the app's folders, and its parent when that is the app's too and now empty.
/// A folder that is not the app's own is left, and said.
fn delete_owned(dir: &Path, placed_by_user: bool) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    if placed_by_user || !owned_by_app(dir) {
        println!(
            "Left {} alone: it is where you chose to keep it, not a folder of {APP_NAME}'s own; delete it yourself if you want it gone.",
            dir.display()
        );
        return Ok(());
    }
    std::fs::remove_dir_all(dir).with_context(|| format!("deleting {}", dir.display()))?;
    if let Some(parent) = dir.parent()
        && parent
            .file_name()
            .is_some_and(|name| name.eq_ignore_ascii_case(APP_COMMAND))
    {
        // Fails when something else is in it, which is the point.
        let _ = std::fs::remove_dir(parent);
    }
    Ok(())
}

/// `path` without the entries that are `dir`, compared as Windows compares folders.
#[cfg_attr(not(windows), allow(dead_code))]
fn path_without(path: &str, dir: &str) -> String {
    let same = |entry: &str| {
        entry
            .trim_end_matches('\\')
            .eq_ignore_ascii_case(dir.trim_end_matches('\\'))
    };
    path.split(';')
        .filter(|entry| !entry.is_empty() && !same(entry))
        .collect::<Vec<_>>()
        .join(";")
}

#[cfg(not(windows))]
fn remove_program(program: &Path) -> Result<()> {
    std::fs::remove_file(program).with_context(|| format!("removing {}", program.display()))
}

/// Windows will not delete a program that is running, and this one is. It is moved aside, and a
/// hidden helper deletes what was moved once this process has ended. The install folder made by
/// the installer goes with it, and so does its entry on the user's PATH.
#[cfg(windows)]
fn remove_program(program: &Path) -> Result<()> {
    if std::fs::remove_file(program).is_err() {
        let aside = crate::cli::update::free_old_path(program);
        std::fs::rename(program, &aside)
            .with_context(|| format!("moving {} aside", program.display()))?;
    }
    let Some(dir) = program.parent() else {
        return Ok(());
    };
    let own_folder = dir
        .file_name()
        .is_some_and(|name| name.eq_ignore_ascii_case(APP_COMMAND));
    if own_folder {
        take_off_the_path(dir);
    }
    if own_folder || moved_aside_in(dir) {
        sweep_after_exit(dir, own_folder);
    }
    Ok(())
}

/// Whether a program an update or this uninstall moved aside is still in `dir`.
#[cfg(windows)]
fn moved_aside_in(dir: &Path) -> bool {
    let old = format!("{APP_COMMAND}.old");
    std::fs::read_dir(dir).into_iter().flatten().flatten().any(|entry| {
        let name = entry.file_name().to_string_lossy().to_lowercase();
        name.starts_with(&old) && name.ends_with(".exe")
    })
}

#[cfg(windows)]
const USER_PATH: &str = "[Environment]::GetEnvironmentVariable('Path', 'User')";

/// Undo what the installer did to the user's PATH. A failure is said and is not the uninstall's.
#[cfg(windows)]
fn take_off_the_path(dir: &Path) {
    let Some(dir) = dir.to_str() else { return };
    let powershell = |script: &str, value: Option<&str>| {
        let mut command = crate::proc::system_tool("powershell");
        command.args(["-NoProfile", "-NonInteractive", "-Command", script]);
        if let Some(value) = value {
            command.env(NEW_PATH_ENV, value);
        }
        crate::proc::run(crate::proc::quiet(&mut command))
    };
    let changed = powershell(USER_PATH, None).and_then(|path| {
        let path = path.trim();
        let without = path_without(path, dir);
        if without == path {
            return Ok(());
        }
        // The value travels in the environment, never in the script's text.
        let write =
            format!("[Environment]::SetEnvironmentVariable('Path', $env:{NEW_PATH_ENV}, 'User')");
        powershell(&write, Some(&without)).map(|_| ())
    });
    if let Err(why) = changed {
        println!("Note: could not take {dir} off your PATH ({why}); remove it in Windows' environment variables.");
    }
}

#[cfg(windows)]
const NEW_PATH_ENV: &str = concat!(env!("APP_ENV"), "_UNINSTALL_PATH");
#[cfg(windows)]
const SWEEP_DIR_ENV: &str = concat!(env!("APP_ENV"), "_UNINSTALL_DIR");

/// Start the helper that deletes the moved-aside programs after this process ends, and the
/// folder too when it is the installer's own and nothing else is left in it.
#[cfg(windows)]
fn sweep_after_exit(dir: &Path, own_folder: bool) {
    let old = format!("{APP_COMMAND}.old*.exe");
    let mut script = format!(
        "Start-Sleep -Seconds 2; Get-ChildItem -LiteralPath $env:{SWEEP_DIR_ENV} -Filter '{old}' | Remove-Item -Force -ErrorAction SilentlyContinue"
    );
    if own_folder {
        script.push_str(&format!(
            "; if (-not (Get-ChildItem -LiteralPath $env:{SWEEP_DIR_ENV} -Force)) {{ Remove-Item -LiteralPath $env:{SWEEP_DIR_ENV} -Force }}"
        ));
    }
    let mut command = crate::proc::system_tool("powershell");
    command
        .args(["-NoProfile", "-NonInteractive", "-Command", &script])
        .env(SWEEP_DIR_ENV, dir)
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    if let Err(why) = crate::proc::quiet(&mut command).spawn() {
        println!(
            "Note: could not start the clean-up ({why}); delete {} yourself.",
            dir.display()
        );
    }
}

/// A yes typed at the terminal. Without a terminal there is nobody to ask, which is refused:
/// nothing is removed unasked.
fn confirmed() -> Result<bool> {
    use std::io::{IsTerminal, Write};
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        bail!(
            "`{APP_COMMAND} uninstall` asks before it removes anything, and there is no terminal to ask in; pass --yes to go ahead"
        );
    }
    print!("Remove {APP_NAME}? [y/N] ");
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    Ok(matches!(answer.trim().to_ascii_lowercase().as_str(), "y" | "yes"))
}

pub fn run(yes: bool, purge: bool) -> Result<()> {
    let plan = Plan::here(purge)?;
    for line in plan.lines() {
        println!("{line}");
    }
    if !yes && !confirmed()? {
        println!("Nothing was changed.");
        return Ok(());
    }
    if plan.sessions.is_some() {
        crate::cli::kill_server(&Target::current(), purge)?;
    }
    if crate::cli::update::integrate_opted_out() {
        println!("Left your agents' config alone (you asked the installer to as well).");
    } else if let Err(why) = crate::cli::integrate(None, true) {
        // An agent whose config could not be changed must not leave the rest of the app installed.
        println!("Warning: {why:#}. Its entry stays in that agent's config; delete it there by hand.");
    }
    if purge {
        let placed = PLACED_BY_USER
            .iter()
            .any(|name| std::env::var_os(name).is_some());
        delete_owned(&plan.data, placed)?;
        delete_owned(&plan.config, placed)?;
    }
    if let Some(program) = &plan.program {
        remove_program(program)?;
    }
    println!("{APP_NAME} is removed.");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan(sessions: Option<usize>, purge: bool, program: Option<&str>) -> Vec<String> {
        Plan {
            program: program.map(PathBuf::from),
            sessions,
            purge,
            config: PathBuf::from("cfg"),
            data: PathBuf::from("data"),
        }
        .lines()
    }

    fn scratch(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            "{APP_COMMAND}-uninstall-test-{}-{name}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn the_plan_says_what_stops_what_goes_and_what_stays() {
        let kept = plan(Some(3), false, Some("bin/app")).join("\n");
        assert!(kept.contains("3 open session(s) end; they are saved"), "{kept}");
        assert!(kept.contains("removes bin/app"), "{kept}");
        assert!(kept.contains("keeps your settings (cfg) and saved sessions (data)"), "{kept}");
        let purged = plan(Some(3), true, Some("bin/app")).join("\n");
        assert!(purged.contains("3 open session(s) end for good"), "{purged}");
        assert!(purged.contains("deletes your settings (cfg)"), "{purged}");
    }

    #[test]
    fn no_server_means_no_line_about_stopping_one() {
        let lines = plan(None, false, Some("bin/app")).join("\n");
        assert!(!lines.contains("stops the server"), "{lines}");
        assert!(plan(Some(0), false, None).join("\n").contains("no session is open"));
    }

    #[test]
    fn a_build_run_from_its_build_folder_is_left_where_it_is() {
        let lines = plan(None, false, None).join("\n");
        assert!(lines.contains("leaves this build where it is"), "{lines}");
        let dir = scratch("build");
        assert!(!is_build_folder(Some(&dir)));
        std::fs::create_dir_all(dir.join("deps")).unwrap();
        std::fs::create_dir_all(dir.join(".fingerprint")).unwrap();
        assert!(is_build_folder(Some(&dir)));
        assert!(!is_build_folder(None));
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn purge_deletes_the_apps_own_folder_and_its_empty_parent() {
        let root = scratch("purge");
        let data = root.join(APP_COMMAND).join("data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(data.join("session.json"), b"{}").unwrap();
        delete_owned(&data, false).unwrap();
        assert!(!root.join(APP_COMMAND).exists());
        assert!(root.exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn purge_keeps_a_parent_that_holds_something_else() {
        let root = scratch("shared-parent");
        let data = root.join(APP_COMMAND).join("data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(root.join(APP_COMMAND).join("other.txt"), b"x").unwrap();
        delete_owned(&data, false).unwrap();
        assert!(!data.exists());
        assert!(root.join(APP_COMMAND).join("other.txt").exists());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn purge_never_deletes_a_folder_that_is_not_the_apps_own() {
        assert!(!owned_by_app(Path::new("/home/me/dotfiles")));
        assert!(owned_by_app(&Path::new("/home/me/.config").join(APP_COMMAND)));
        assert!(owned_by_app(&Path::new("/data").join(APP_COMMAND).join("data")));
        assert!(!owned_by_app(&Path::new("/data").join(format!("{APP_COMMAND}-notes"))));
        // Deep inside a folder that merely has the app's name (a checkout, say) is not its own.
        assert!(!owned_by_app(&Path::new("/dev").join(APP_COMMAND).join("src").join("x")));
        let root = scratch("not-owned");
        let other = root.join("notes");
        std::fs::create_dir_all(&other).unwrap();
        delete_owned(&other, false).unwrap();
        assert!(other.exists());
        let chosen = root.join(APP_COMMAND);
        std::fs::create_dir_all(&chosen).unwrap();
        delete_owned(&chosen, true).unwrap();
        assert!(chosen.exists(), "a folder the user placed is kept even when named after the app");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn the_install_folder_is_taken_off_a_path_and_nothing_else_is() {
        let path = r"C:\Windows;C:\Users\me\AppData\Local\Programs\App\;C:\Tools;;C:\users\ME\appdata\local\programs\app";
        assert_eq!(
            path_without(path, r"C:\Users\me\AppData\Local\Programs\App"),
            r"C:\Windows;C:\Tools"
        );
        assert_eq!(path_without(r"C:\Windows;C:\Tools", r"C:\Nope"), r"C:\Windows;C:\Tools");
    }

    #[test]
    fn a_program_nothing_runs_is_removed() {
        let dir = scratch("program");
        let program = dir.join(if cfg!(windows) { "tool.exe" } else { "tool" });
        std::fs::write(&program, b"x").unwrap();
        remove_program(&program).unwrap();
        assert!(!program.exists());
        let _ = std::fs::remove_dir_all(dir);
    }
}
