//! Pulling a file from a session's machine to this computer (ADR 0007). The window hands the host
//! the machine's name, a path on that machine and a transfer id; the host checks them, pulls the
//! file over the machine's own ssh line (`maitty-machines`' `get_file`, the runner the terminal
//! app's `machine get` uses), and tells the window how it goes as `file-get` events. It saves
//! into `Downloads/maitty/<machine>/`. Nothing here blocks the UI thread: the pull runs on a
//! blocking worker. One transfer runs at a time, shared with file sends, and `cancel_file_send`
//! ends a pull by its id too.

use crate::file_send::{Ending, Transfers, target};
use maitty_machines::ssh_run::Watch;
use maitty_machines::{MAX_GET_BYTES, Machine, MachineDef, get_file, local_name};
use serde::Serialize;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;
use tauri::{Emitter, Manager};

/// The event the window follows.
pub const FILE_GET_EVENT: &str = "file-get";

/// How often the window is told how far a pull has got.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// The folder inside Downloads that every pulled file goes under.
const FOLDER: &str = maitty_machines::APP_COMMAND;

/// What the window is told. Exactly one of `done`, `failed` or `cancelled` ends a pull. Messages
/// are made here from checked pieces, so the window shows them as they are.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum GetEvent {
    /// How far the file has got.
    Progress {
        transfer: String,
        file: String,
        percent: u8,
    },
    /// The file is saved at `path`, in `folder`.
    Done {
        transfer: String,
        file: String,
        path: String,
        folder: String,
    },
    Failed {
        transfer: String,
        file: String,
        message: String,
    },
    Cancelled {
        transfer: String,
        file: String,
    },
}

/// A pull that passed its checks.
pub struct Job {
    pub transfer: String,
    pub machine: Machine,
    /// The machine's name in words.
    pub name: String,
    pub remote_path: String,
    /// Where the file is saved.
    pub folder: PathBuf,
}

impl Job {
    /// What the window calls the file: the name it will be saved under, made from the remote path.
    fn label(&self) -> String {
        local_name(&self.remote_path)
    }
}

/// Check what the window asked for, before anything runs. `downloads` is this computer's
/// Downloads folder.
pub fn plan(
    machines: &[MachineDef],
    ssh: &[String],
    machine: &str,
    transfer: &str,
    remote_path: &str,
    downloads: &Path,
) -> Result<Job, String> {
    let target = target(machines, ssh, machine, transfer)?;
    // The same check the ssh line makes: an empty, over-long or control-character path is refused
    // here, before a worker starts.
    target
        .machine
        .get_argv(remote_path, MAX_GET_BYTES)
        .map_err(|e| format!("{e:#}"))?;
    Ok(Job {
        transfer: transfer.to_owned(),
        folder: downloads.join(FOLDER).join(local_name(&target.name)),
        machine: target.machine,
        name: target.name,
        remote_path: remote_path.to_owned(),
    })
}

/// Pull the job's file, telling `emit` how it goes. Blocks until it is saved, fails, or `cancel`
/// is set. Run it on a worker, never on the UI thread.
pub fn run(job: &Job, cancel: &Arc<AtomicBool>, emit: &(dyn Fn(GetEvent) + Sync)) {
    let watch = Watch {
        cancel: Arc::clone(cancel),
        ..Watch::default()
    };
    let finished = AtomicBool::new(false);
    let transfer = job.transfer.as_str();
    let file = job.label();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !finished.load(Ordering::Relaxed) {
                std::thread::sleep(PROGRESS_EVERY);
                let total = watch.total.load(Ordering::Relaxed);
                if total == 0 || finished.load(Ordering::Relaxed) {
                    continue;
                }
                let got = watch.sent.load(Ordering::Relaxed).min(total);
                emit(GetEvent::Progress {
                    transfer: transfer.to_owned(),
                    file: file.clone(),
                    percent: u8::try_from(got * 100 / total).unwrap_or(100),
                });
            }
        });
        let outcome = match get_file(&job.machine, &job.name, &job.remote_path, &job.folder, &watch) {
            Ok(path) => GetEvent::Done {
                transfer: transfer.to_owned(),
                file: file.clone(),
                path: path.display().to_string(),
                folder: job.folder.display().to_string(),
            },
            Err(_) if watch.cancel.load(Ordering::Relaxed) => GetEvent::Cancelled {
                transfer: transfer.to_owned(),
                file: file.clone(),
            },
            Err(message) => GetEvent::Failed {
                transfer: transfer.to_owned(),
                file: file.clone(),
                message,
            },
        };
        finished.store(true, Ordering::Relaxed);
        emit(outcome);
    });
}

/// Start pulling `path` from `machine`. Returns once the request is checked and the pull has
/// started; how it goes comes as `file-get` events named by `transfer`.
#[tauri::command]
pub async fn get_file_from_machine(
    app: tauri::AppHandle,
    machines: tauri::State<'_, crate::machines::Machines>,
    machine: String,
    path: String,
    transfer: String,
) -> Result<(), String> {
    let downloads = app
        .path()
        .download_dir()
        .map_err(|e| format!("could not find your Downloads folder: {e}"))?;
    let ssh = [maitty_machines::SSH_PROGRAM.to_owned()];
    let job = plan(&machines.0.machines, &ssh, &machine, &transfer, &path, &downloads)?;
    start(&app, job)
}

/// Run a planned pull on a blocking worker, one transfer at a time.
fn start(app: &tauri::AppHandle, job: Job) -> Result<(), String> {
    let cancel = app.state::<Transfers>().begin(&job.transfer)?;
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Dropped even if the pull panics, or no later transfer could start.
        let _ending = Ending::new(handle.clone(), job.transfer.clone());
        run(&job, &cancel, &|event| {
            if let Err(error) = handle.emit(FILE_GET_EVENT, event) {
                tracing::warn!("could not tell the window how a file pull went: {error}");
            }
        });
    });
    Ok(())
}

#[cfg(test)]
mod tests;
