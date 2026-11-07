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
/// A running daemon of another release is stopped first, but only when its sessions are saved
/// (the next daemon restores them) or it has none; otherwise this fails and leaves it running.
pub fn ensure_shared_daemon(bundled: &Path, app_version: &str) -> Result<PathBuf> {
    let target = shared_daemon_path()?;
    let installed = target
        .is_file()
        .then(|| installed_version(&target))
        .flatten();
    if plan(installed.as_deref(), app_version) == Plan::UseInstalled {
        return Ok(target);
    }
    crate::stop_running_daemon_for_update()?;
    replace_file(bundled, &target)?;
    match installed_version(&target) {
        Some(version) if version == app_version => Ok(target),
        found => bail!(
            "installed {} but it reports {found:?}, not {app_version}",
            target.display()
        ),
    }
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
        let aside = directory.join(format!("{APP_COMMAND}.old.exe"));
        let _ = std::fs::remove_file(&aside);
        std::fs::rename(target, &aside)
            .with_context(|| format!("moving the old {} aside", target.display()))?;
    }
    std::fs::rename(&staged, target)
        .with_context(|| format!("installing {}", target.display()))
}

#[cfg(test)]
mod tests {
    use super::{Plan, parse_version, plan, replace_file};

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
}
