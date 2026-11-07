//! Updating the app on another machine from the window (ADR 0007). The installed `maitty` does
//! the work, the same as `maitty machine update <name>` in a terminal, so there is one update
//! path. The machine's saved sessions come back on the new daemon; a turn in progress is lost.

use std::path::Path;
use std::process::{Command, Stdio};

/// Keeps a window-supplied name from ever being read as an option.
fn valid_name(name: &str) -> bool {
    !name.is_empty() && !name.starts_with('-') && !name.chars().any(char::is_control)
}

fn arguments(name: &str, install: bool, when_idle: bool) -> Vec<String> {
    let restart = if when_idle { "--wait" } else { "--restart" };
    let mut arguments: Vec<String> = ["machine", "update", name, "--yes", restart]
        .map(String::from)
        .into();
    if install {
        arguments.push("--install".into());
    }
    arguments
}

fn last_line(text: &str) -> Option<&str> {
    text.lines().rev().map(str::trim).find(|line| !line.is_empty())
}

/// What to tell the person after a success: the result line, and the daemon's own note when it
/// left the daemon running (so a restart that did not happen is never hidden).
fn summary(stdout: &str, stderr: &str) -> String {
    let result = last_line(stdout).unwrap_or("Updated.");
    match stderr.lines().rev().map(str::trim).find(|line| line.starts_with("note:")) {
        Some(note) => format!("{result}\n{note}"),
        None => result.to_string(),
    }
}

fn run(executable: &Path, name: &str, install: bool, when_idle: bool) -> Result<String, String> {
    let mut command = Command::new(executable);
    command
        .args(arguments(name, install, when_idle))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    let output = command
        .output()
        .map_err(|error| format!("Could not run the update: {error}"))?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    if output.status.success() {
        return Ok(summary(&stdout, &stderr));
    }
    Err(last_line(&stderr)
        .or_else(|| last_line(&stdout))
        .map_or_else(|| format!("The update of {name} failed."), str::to_string))
}

#[tauri::command]
pub async fn update_machine(
    app: tauri::AppHandle,
    name: String,
    install: bool,
    when_idle: bool,
) -> Result<String, String> {
    if !valid_name(&name) {
        return Err("That is not a machine name.".into());
    }
    let executable = crate::daemon_executable(&app)?;
    tauri::async_runtime::spawn_blocking(move || run(&executable, &name, install, when_idle))
        .await
        .map_err(|error| format!("The update stopped unexpectedly: {error}"))?
}

#[cfg(test)]
mod tests {
    use super::{arguments, summary, valid_name};

    #[test]
    fn a_name_that_could_be_an_option_is_refused() {
        assert!(valid_name("box"));
        assert!(!valid_name("--help"));
        assert!(!valid_name(""));
        assert!(!valid_name("a\nb"));
    }

    #[test]
    fn the_update_is_confirmed_and_restarts_the_daemon() {
        assert_eq!(
            arguments("box", false, false),
            ["machine", "update", "box", "--yes", "--restart"]
        );
        assert_eq!(arguments("box", true, false).last().unwrap(), "--install");
    }

    #[test]
    fn updating_when_idle_waits_instead_of_restarting_at_once() {
        let said = arguments("box", false, true);
        assert!(said.contains(&"--wait".to_string()));
        assert!(!said.contains(&"--restart".to_string()));
    }

    #[test]
    fn a_daemon_left_running_is_reported_with_the_result() {
        let said = summary(
            "box: maitty 1.0.0 installed to ~/.local/bin/maitty\n",
            "probing box…\nnote: could not ask the daemon (x); running sessions are untouched\n",
        );
        assert!(said.starts_with("box: maitty 1.0.0 installed"));
        assert!(said.contains("note: could not ask the daemon"));
        assert_eq!(summary("", ""), "Updated.");
    }
}
