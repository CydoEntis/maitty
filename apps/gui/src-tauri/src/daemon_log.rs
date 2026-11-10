//! Showing the service's log from the window: when the service has stopped, the log is the only record of why.

use std::path::{Path, PathBuf};
use std::process::Command;

/// The file the service writes (`init_logging` in the daemon), in the data folder.
const LOG_FILE: &str = "daemon.log";

fn log_path(data_dir: &Path) -> PathBuf {
    data_dir.join(LOG_FILE)
}

/// What to run to show `file`: the file selected in its folder where the system can do that, else its folder.
fn reveal_command(file: &Path) -> Command {
    if cfg!(windows) {
        let mut command = Command::new("explorer");
        command.arg(format!("/select,{}", file.display()));
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("open");
        command.arg("-R").arg(file);
        command
    } else {
        let mut command = maitty_desktop_client::host_command("xdg-open");
        command.arg(file.parent().unwrap_or(file));
        command
    }
}

/// Opens the folder holding the service's log with the log selected. Refused when there is no log yet.
#[tauri::command]
pub fn open_daemon_log() -> Result<(), String> {
    let file = log_path(&maitty_desktop_client::data_dir());
    if !file.is_file() {
        return Err("There is no log yet.".into());
    }
    reveal_command(&file)
        .spawn()
        .map(drop)
        .map_err(|error| format!("Could not open the log: {error}"))
}

#[cfg(test)]
mod tests {
    use super::{log_path, reveal_command};
    use std::path::Path;

    #[test]
    fn the_log_is_the_daemons_file_in_the_data_folder() {
        assert_eq!(log_path(Path::new("/data")), Path::new("/data/daemon.log"));
    }

    #[test]
    fn the_log_is_shown_by_the_system_not_run() {
        let command = reveal_command(Path::new("/data/daemon.log"));
        let program = command.get_program().to_string_lossy().into_owned();
        assert!(
            ["explorer", "open", "xdg-open"].contains(&program.as_str()),
            "{program}"
        );
    }
}
