//! Showing the service's log from the window, and saving a copy of it to send to someone: when the
//! service has stopped or misbehaved, the log is the only record of why.

use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::process::Command;
use tauri::Manager;

/// The name a copy of the log is saved under (a number is added when it is taken).
const COPY_NAME: &str = "maitty-log.txt";
/// The most of the log's end a copy holds: what happened lately is what is asked for, and it has
/// to fit what an export may be.
const COPY_BYTES: u64 = 4 * 1024 * 1024;

/// The end of the log as text, from the start of a line: at most `limit` bytes of it.
fn log_tail(file: &Path, limit: u64) -> Result<String, String> {
    let mut log = std::fs::File::open(file).map_err(|error| format!("Could not read the log: {error}"))?;
    let length = log.metadata().map_err(|error| format!("Could not read the log: {error}"))?.len();
    let start = length.saturating_sub(limit);
    log.seek(SeekFrom::Start(start)).map_err(|error| format!("Could not read the log: {error}"))?;
    let mut bytes = Vec::new();
    log.take(limit).read_to_end(&mut bytes).map_err(|error| format!("Could not read the log: {error}"))?;
    let text = String::from_utf8_lossy(&bytes);
    // A copy that starts partway through the log starts at its first whole line.
    let whole = if start == 0 { &text[..] } else { text.split_once('\n').map_or("", |(_, rest)| rest) };
    Ok(whole.to_string())
}

/// Saves a copy of the end of the service's log in the Downloads folder, never over a file that is
/// there, shows it in the file manager and says where it is. The window names nothing: the log,
/// the folder and the file name are the host's.
#[tauri::command]
pub async fn export_daemon_log(app: tauri::AppHandle) -> Result<String, String> {
    let folder = app.path().download_dir().map_err(|error| format!("There is no Downloads folder to save it in: {error}"))?;
    tauri::async_runtime::spawn_blocking(move || {
        let file = log_path(&maitty_desktop_client::data_dir());
        if !file.is_file() {
            return Err("There is no log yet.".to_string());
        }
        let saved = crate::export::save_in(&folder, COPY_NAME, &log_tail(&file, COPY_BYTES)?)?;
        // Shown when the system can; the path is told either way.
        if let Err(error) = reveal_command(&saved).spawn() {
            tracing::warn!("showing the saved log: {error}");
        }
        Ok(saved.display().to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

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
    use super::{log_path, log_tail, reveal_command};
    use std::path::Path;

    #[test]
    fn a_copy_holds_the_end_of_the_log_from_a_whole_line() {
        let folder = std::env::temp_dir().join(format!("maitty-log-tail-{}", std::process::id()));
        std::fs::create_dir_all(&folder).unwrap();
        let file = folder.join("daemon.log");
        std::fs::write(&file, "first line\nsecond line\nthird line\n").unwrap();

        assert_eq!(log_tail(&file, 1024).unwrap(), "first line\nsecond line\nthird line\n");
        // Cut inside the second line: the copy starts at the third.
        assert_eq!(log_tail(&file, 15).unwrap(), "third line\n");
        assert!(log_tail(&folder.join("missing.log"), 1024).is_err());
        std::fs::remove_dir_all(&folder).unwrap();
    }

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
