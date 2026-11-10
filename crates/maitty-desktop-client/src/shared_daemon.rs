//! The one daemon file per PC (ADR 0004). The terminal install puts `maitty` in a per-user
//! folder; the desktop app keeps that same file current instead of running a private copy.

use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;

const APP_COMMAND: &str = env!("MAITTY_APP_COMMAND");
const APP_ENV: &str = env!("MAITTY_APP_ENV");

/// Where `install.ps1`, `install.sh` and this module all keep the executable.
pub fn shared_daemon_path() -> Result<PathBuf> {
    let file = if cfg!(windows) {
        format!("{APP_COMMAND}.exe")
    } else {
        APP_COMMAND.to_string()
    };
    if let Some(directory) = std::env::var_os(format!("{APP_ENV}_INSTALL_DIR")) {
        return Ok(PathBuf::from(directory).join(file));
    }
    #[cfg(windows)]
    let directory = std::env::var_os("LOCALAPPDATA")
        .map(|local| PathBuf::from(local).join("Programs").join(APP_COMMAND));
    #[cfg(not(windows))]
    let directory = std::env::var_os("HOME")
        .map(|home| PathBuf::from(home).join(".local").join("bin"));
    Ok(directory
        .context("couldn't find the folder the daemon is installed in")?
        .join(file))
}

#[derive(Debug, PartialEq, Eq)]
enum Plan {
    UseInstalled,
    Install,
}

/// Replace the installed file only when it is missing or older than the app. A newer one (the
/// terminal app updated first) is kept: the app asks its user to update instead of downgrading.
fn plan(installed: Option<&str>, app_version: &str) -> Plan {
    let Some(installed) = installed else {
        return Plan::Install;
    };
    if installed == app_version {
        return Plan::UseInstalled;
    }
    match (parse_version(installed), parse_version(app_version)) {
        (Some(installed), Some(app)) if installed > app => Plan::UseInstalled,
        _ => Plan::Install,
    }
}

fn parse_version(version: &str) -> Option<(u64, u64, u64)> {
    let mut parts = version.split('.').map(|part| part.parse::<u64>().ok());
    let version = (parts.next()??, parts.next()??, parts.next()??);
    parts.next().is_none().then_some(version)
}

/// Whether the daemon runs an older release than the app. A version either side cannot read is
/// never "older": the window offers a restart only when it is sure there is one to take.
pub(crate) fn is_older(daemon: &str, app: &str) -> bool {
    matches!(
        (parse_version(daemon), parse_version(app)),
        (Some(daemon), Some(app)) if daemon < app
    )
}

/// What `<executable> --version` prints after the program name, if it runs.
fn installed_version(executable: &Path) -> Option<String> {
    let output = Command::new(executable).arg("--version").output().ok()?;
    output.status.success().then_some(())?;
    let text = String::from_utf8_lossy(&output.stdout);
    text.trim()
        .strip_prefix(APP_COMMAND)
        .map(|version| version.trim().to_string())
}

/// Make the shared daemon file the same release as the app, and return its path.
///
/// A running daemon is never stopped for this: the old file is moved aside and goes on running
/// with its sessions. When it speaks another protocol the window asks before restarting it
/// (`restart_mismatched_daemon`); an agent's turn is the user's to end, never an update's.
pub fn ensure_shared_daemon(bundled: &Path, app_version: &str) -> Result<PathBuf> {
    let target = shared_daemon_path()?;
    let installed = target
        .is_file()
        .then(|| installed_version(&target))
        .flatten();
    if plan(installed.as_deref(), app_version) == Plan::UseInstalled {
        return Ok(target);
    }
    replace_file(bundled, &target)?;
    match installed_version(&target) {
        Some(version) if version == app_version => {}
        found => bail!(
            "installed {} but it reports {found:?}, not {app_version}",
            target.display()
        ),
    }
    // A convenience for typing the command in a terminal: it must not stop the app starting.
    if let Err(error) = put_folder_on_path(&target) {
        tracing::warn!("couldn't add the install folder to your PATH: {error:#}");
    }
    Ok(target)
}

/// The user PATH with `directory` added, or `None` when it is already there. Windows compares
/// folders without case and ignores a trailing separator.
#[cfg_attr(not(windows), allow(dead_code))]
fn path_with(existing: &str, directory: &str) -> Option<String> {
    let same = |entry: &str| {
        entry
            .trim()
            .trim_end_matches(['\\', '/'])
            .eq_ignore_ascii_case(directory.trim_end_matches(['\\', '/']))
    };
    if existing.split(';').any(same) {
        return None;
    }
    let kept = existing.trim().trim_end_matches(';');
    Some(if kept.is_empty() {
        directory.to_string()
    } else {
        format!("{kept};{directory}")
    })
}

/// Like `install.ps1`: add the folder to the user's PATH so `maitty` works in a new terminal.
/// `<APP>_NO_PATH=1` leaves PATH alone. Other systems keep their shell startup files to
/// themselves; `install.sh` tells the person what to add.
fn put_folder_on_path(target: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        if std::env::var(format!("{APP_ENV}_NO_PATH")).as_deref() == Ok("1") {
            return Ok(());
        }
        let directory = target
            .parent()
            .and_then(Path::to_str)
            .context("the install folder is not plain text")?;
        let read = windows_user_path()?;
        if let Some(updated) = path_with(&read, directory) {
            set_windows_user_path(&updated)?;
        }
    }
    #[cfg(not(windows))]
    let _ = target;
    Ok(())
}

#[cfg(windows)]
fn windows_powershell(script: &str) -> Result<std::process::Output> {
    use std::os::windows::process::CommandExt;
    let output = Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-Command", script])
        .creation_flags(0x0800_0000)
        .output()
        .context("running PowerShell")?;
    anyhow::ensure!(
        output.status.success(),
        "PowerShell said: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(output)
}

#[cfg(windows)]
fn windows_user_path() -> Result<String> {
    let output =
        windows_powershell("[Environment]::GetEnvironmentVariable('Path','User')")?;
    Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// The new value reaches PowerShell as an environment variable, never as part of the script.
#[cfg(windows)]
fn set_windows_user_path(value: &str) -> Result<()> {
    use std::os::windows::process::CommandExt;
    let output = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "[Environment]::SetEnvironmentVariable('Path',$env:MAITTY_NEW_USER_PATH,'User')",
        ])
        .env("MAITTY_NEW_USER_PATH", value)
        .creation_flags(0x0800_0000)
        .output()
        .context("running PowerShell")?;
    anyhow::ensure!(
        output.status.success(),
        "PowerShell said: {}",
        String::from_utf8_lossy(&output.stderr).trim()
    );
    Ok(())
}

/// Windows locks a running exe but lets it be renamed, so a terminal app still open from the
/// old file keeps running while the new one takes its name.
fn replace_file(source: &Path, target: &Path) -> Result<()> {
    let directory = target
        .parent()
        .context("the install path has no folder")?;
    std::fs::create_dir_all(directory)
        .with_context(|| format!("creating {}", directory.display()))?;
    let staged = directory.join(format!(".{APP_COMMAND}.new"));
    std::fs::copy(source, &staged)
        .with_context(|| format!("copying the daemon to {}", staged.display()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))
            .context("making the daemon executable")?;
    }
    #[cfg(windows)]
    if target.exists() {
        let aside = free_aside(directory);
        std::fs::rename(target, &aside)
            .with_context(|| format!("moving the old {} aside", target.display()))?;
    }
    std::fs::rename(&staged, target)
        .with_context(|| format!("installing {}", target.display()))
}

/// A free name for the old file. One moved aside earlier is removed when nothing runs it; a daemon
/// or an agent's `mcp` still running from it keeps it locked, so the next one gets a number.
#[cfg(windows)]
fn free_aside(directory: &Path) -> PathBuf {
    let named = |number: u32| match number {
        1 => directory.join(format!("{APP_COMMAND}.old.exe")),
        _ => directory.join(format!("{APP_COMMAND}.old-{number}.exe")),
    };
    (1..)
        .map(named)
        .find(|aside| std::fs::remove_file(aside).is_ok() || !aside.exists())
        .unwrap_or_else(|| named(1))
}

#[cfg(test)]
mod tests {
    use super::{Plan, is_older, parse_version, path_with, plan, replace_file};

    #[test]
    fn a_daemon_is_older_only_when_both_versions_read_and_it_is_lower() {
        assert!(is_older("0.71.0", "0.72.0"));
        assert!(is_older("0.9.0", "0.10.0"), "compared as numbers, not text");
        assert!(!is_older("0.72.0", "0.72.0"));
        assert!(!is_older("0.73.0", "0.72.0"), "a newer daemon is never replaced by an older app");
        assert!(!is_older("", "0.72.0"), "a daemon that did not say its version is left alone");
        assert!(!is_older("0.71.0", "dev"));
    }

    #[test]
    fn installs_when_nothing_is_installed() {
        assert_eq!(plan(None, "0.67.0"), Plan::Install);
    }

    #[test]
    fn keeps_the_installed_file_when_it_is_the_same_release() {
        assert_eq!(plan(Some("0.67.0"), "0.67.0"), Plan::UseInstalled);
    }

    #[test]
    fn replaces_an_older_installed_file() {
        assert_eq!(plan(Some("0.66.0"), "0.67.0"), Plan::Install);
        assert_eq!(plan(Some("0.9.0"), "0.10.0"), Plan::Install);
    }

    #[test]
    fn never_downgrades_a_newer_installed_file() {
        assert_eq!(plan(Some("0.68.0"), "0.67.0"), Plan::UseInstalled);
    }

    #[test]
    fn replaces_a_file_whose_version_cannot_be_read() {
        assert_eq!(plan(Some("garbage"), "0.67.0"), Plan::Install);
    }

    #[test]
    fn parses_three_part_versions_only() {
        assert_eq!(parse_version("1.22.3"), Some((1, 22, 3)));
        assert_eq!(parse_version("1.2"), None);
        assert_eq!(parse_version("1.2.3.4"), None);
        assert_eq!(parse_version("1.2.x"), None);
    }

    #[test]
    fn replacing_overwrites_the_old_file_and_leaves_no_staging_file() {
        let directory = std::env::temp_dir().join(format!("shared-daemon-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("new");
        let target = directory.join("installed");
        std::fs::write(&source, b"new").unwrap();
        std::fs::write(&target, b"old").unwrap();

        replace_file(&source, &target).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"new");
        assert!(!directory.join(concat!(".", env!("MAITTY_APP_COMMAND"), ".new")).exists());
        std::fs::remove_dir_all(&directory).unwrap();
    }

    /// A daemon still running from the file moved aside last time keeps that file locked.
    #[cfg(windows)]
    #[test]
    fn replacing_twice_works_while_the_first_old_file_is_still_in_use() {
        let directory = std::env::temp_dir().join(format!("shared-daemon-twice-{}", std::process::id()));
        std::fs::create_dir_all(&directory).unwrap();
        let source = directory.join("new");
        let target = directory.join("installed.exe");
        std::fs::write(&source, b"newest").unwrap();
        std::fs::write(&target, b"newer").unwrap();
        let first_aside = directory.join(concat!(env!("MAITTY_APP_COMMAND"), ".old.exe"));
        std::fs::write(&first_aside, b"oldest").unwrap();
        // A running program's file cannot be removed; an open handle that shares nothing is the same lock.
        let in_use = {
            use std::os::windows::fs::OpenOptionsExt;
            std::fs::OpenOptions::new()
                .read(true)
                .share_mode(0)
                .open(&first_aside)
                .unwrap()
        };

        replace_file(&source, &target).unwrap();

        assert_eq!(std::fs::read(&target).unwrap(), b"newest");
        let second_aside = directory.join(concat!(env!("MAITTY_APP_COMMAND"), ".old-2.exe"));
        assert_eq!(std::fs::read(&second_aside).unwrap(), b"newer");
        drop(in_use);
        assert_eq!(std::fs::read(&first_aside).unwrap(), b"oldest");
        std::fs::remove_dir_all(&directory).unwrap();
    }

    #[test]
    fn the_folder_is_added_once_and_matched_loosely() {
        assert_eq!(
            path_with(r"C:\bin;C:\tools\", r"C:\Apps\maitty"),
            Some(r"C:\bin;C:\tools\;C:\Apps\maitty".to_string())
        );
        assert_eq!(path_with(r"C:\bin;c:\apps\MAITTY\", r"C:\Apps\maitty"), None);
        assert_eq!(path_with("", r"C:\Apps\maitty"), Some(r"C:\Apps\maitty".to_string()));
    }
}
