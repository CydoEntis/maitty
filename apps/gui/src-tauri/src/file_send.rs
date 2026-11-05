//! Sending files dropped on a pane of another machine (ADR 0050). The window hands the host the
//! paths the operating system gave on the drop and the machine's name; the host checks them,
//! sends them one after another over the machine's own ssh line (`maitty-machines`' `remote_send`,
//! the runner the terminal app uses), and tells the window how it goes as `file-send` events.
//! Nothing here blocks the UI thread: the sends run on a blocking worker, and the window types the
//! far-side paths into the pane itself, from the events.

use maitty_machines::remote_paste::MAX_SEND_BYTES;
use maitty_machines::remote_send::{send_file, send_image, shown};
use maitty_machines::ssh_run::Watch;
use maitty_machines::{Machine, MachineDef};
use serde::Serialize;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;
use tauri::{Emitter, Manager};

/// The event the window follows.
pub const FILE_SEND_EVENT: &str = "file-send";

/// The most files one drop may send (ADR 0050).
pub const MOST_FILES: usize = 10;

/// The longest id the window may name a send by.
const TRANSFER_ID_MAX: usize = 64;

/// How often the window is told how far a send has got.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

/// What the window is told. Exactly one of `sent`'s siblings ends a send: `done`, `failed` or
/// `cancelled`. Messages are made here from checked pieces, so the window shows them as they are.
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum SendEvent {
    /// How far the file now going has got.
    Progress {
        transfer: String,
        file: String,
        index: usize,
        count: usize,
        percent: u8,
    },
    /// One file is kept on the machine; `remote_path` was checked as an absolute path of plain text.
    Sent {
        transfer: String,
        file: String,
        index: usize,
        count: usize,
        remote_path: String,
        /// The size of the file that was sent, for a chip that shows it (a chat attachment, ADR 0053).
        bytes: u64,
    },
    /// A file could not be sent; the ones after it were not tried.
    Failed {
        transfer: String,
        file: String,
        message: String,
        unsent: usize,
    },
    Cancelled {
        transfer: String,
        sent: usize,
    },
    Done {
        transfer: String,
        sent: usize,
        count: usize,
    },
}

/// What a send carries. The same runner, events and cancel serve both (tickets 11 and 19).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SendKind {
    /// Files the person named, up to 100 MB each (ADR 0050).
    Files,
    /// A pasted image the host staged in its private cache, up to 25 MB (ADR 0049). The window
    /// is told it is `image`, not the staged file's name, which means nothing to a person.
    Image { extension: &'static str },
}

/// What the window is told the file of an image send is called.
pub const IMAGE_LABEL: &str = "image";

/// A send that passed its checks: where it goes and what, in order.
pub struct Job {
    pub transfer: String,
    pub machine: Machine,
    /// The machine's name in words.
    pub name: String,
    pub files: Vec<PathBuf>,
    pub kind: SendKind,
}

impl Job {
    /// How the window is told to call `path`.
    fn label(&self, path: &std::path::Path) -> String {
        match self.kind {
            SendKind::Files => shown(path),
            SendKind::Image { .. } => IMAGE_LABEL.to_owned(),
        }
    }
}

/// The machine a request names, once it is known to be one that can be sent to. Whether it is
/// connected is for the caller to check.
pub struct Target {
    pub machine: Machine,
    /// The machine's name as the person wrote it in the machine file.
    pub name: String,
}

/// Check the machine and the id a request names. Shared by files and images so both refuse a
/// machine in the same words.
pub fn target(
    machines: &[MachineDef],
    ssh: &[String],
    machine: &str,
    transfer: &str,
) -> Result<Target, String> {
    if transfer.is_empty()
        || transfer.len() > TRANSFER_ID_MAX
        || !transfer
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_'))
    {
        return Err("the send has no usable id".into());
    }
    let def = machines
        .iter()
        .find(|def| def.name.eq_ignore_ascii_case(machine))
        .ok_or_else(|| format!("{machine} is not one of this computer's machines"))?;
    if def.via.is_some() {
        return Err(format!(
            "sending files to {} needs ssh, and it is reached by its own command",
            def.name
        ));
    }
    Ok(Target {
        machine: Machine::from_def_via(def, ssh),
        name: def.name.clone(),
    })
}

/// Check what the window asked for. Everything is checked before the first byte is sent, so a
/// bad third file does not leave two already sent. `ssh` is the program and its leading arguments.
pub fn plan(
    machines: &[MachineDef],
    ssh: &[String],
    machine: &str,
    transfer: &str,
    paths: &[String],
) -> Result<Job, String> {
    let target = target(machines, ssh, machine, transfer)?;
    if paths.is_empty() {
        return Err("there is no file to send".into());
    }
    if paths.len() > MOST_FILES {
        return Err(format!(
            "at most {MOST_FILES} files can be sent at a time; {} were dropped",
            paths.len()
        ));
    }
    let files = paths
        .iter()
        .map(|path| checked_file(path))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(Job {
        transfer: transfer.to_owned(),
        machine: target.machine,
        name: target.name,
        files,
        kind: SendKind::Files,
    })
}

/// A path the person dropped, if it names a regular file that may be sent. A symlink is followed
/// because the person named it.
fn checked_file(path: &str) -> Result<PathBuf, String> {
    let file = PathBuf::from(path);
    if path.is_empty() || path.contains('\0') || !file.is_absolute() {
        return Err("a dropped file did not have a full path".into());
    }
    let meta =
        std::fs::metadata(&file).map_err(|e| format!("could not read {}: {e}", shown(&file)))?;
    if !meta.is_file() {
        return Err(format!(
            "{} is not a file; only files can be sent",
            shown(&file)
        ));
    }
    if meta.len() > MAX_SEND_BYTES {
        return Err(format!(
            "{} is {} MB, more than the {} MB that can be sent",
            shown(&file),
            meta.len().div_ceil(1024 * 1024),
            MAX_SEND_BYTES / (1024 * 1024)
        ));
    }
    Ok(file)
}

/// Send the job's files in turn, telling `emit` how it goes. Blocks until the last file is kept,
/// one fails, or `cancel` is set. Run it on a worker, never on the UI thread.
pub fn run(job: &Job, cancel: &Arc<AtomicBool>, emit: &(dyn Fn(SendEvent) + Sync)) {
    let count = job.files.len();
    let watch = Watch {
        cancel: Arc::clone(cancel),
        ..Watch::default()
    };
    let at = AtomicUsize::new(0);
    let finished = AtomicBool::new(false);
    let transfer = job.transfer.as_str();
    std::thread::scope(|scope| {
        scope.spawn(|| {
            while !finished.load(Ordering::Relaxed) {
                std::thread::sleep(PROGRESS_EVERY);
                let total = watch.total.load(Ordering::Relaxed);
                if total == 0 || finished.load(Ordering::Relaxed) {
                    continue;
                }
                let index = at.load(Ordering::Relaxed);
                let sent = watch.sent.load(Ordering::Relaxed).min(total);
                emit(SendEvent::Progress {
                    transfer: transfer.to_owned(),
                    file: job.label(&job.files[index]),
                    index,
                    count,
                    percent: u8::try_from(sent * 100 / total).unwrap_or(100),
                });
            }
        });
        let outcome = send_all(job, &watch, &at, emit);
        finished.store(true, Ordering::Relaxed);
        emit(outcome);
    });
}

/// The files in turn; the event that ends the send.
fn send_all(
    job: &Job,
    watch: &Watch,
    at: &AtomicUsize,
    emit: &(dyn Fn(SendEvent) + Sync),
) -> SendEvent {
    let count = job.files.len();
    let transfer = job.transfer.clone();
    for (index, path) in job.files.iter().enumerate() {
        if watch.cancel.load(Ordering::Relaxed) {
            return SendEvent::Cancelled {
                transfer,
                sent: index,
            };
        }
        at.store(index, Ordering::Relaxed);
        watch.sent.store(0, Ordering::Relaxed);
        watch.total.store(0, Ordering::Relaxed);
        let sent = match job.kind {
            SendKind::Files => send_file(&job.machine, &job.name, path, watch),
            SendKind::Image { extension } => {
                send_image(&job.machine, &job.name, path, extension, watch)
            }
        };
        match sent {
            Ok(remote_path) => emit(SendEvent::Sent {
                transfer: transfer.clone(),
                file: job.label(path),
                index,
                count,
                remote_path,
                // A size is only for display; a file that vanished after it was sent shows as 0.
                bytes: std::fs::metadata(path).map_or(0, |meta| meta.len()),
            }),
            Err(_) if watch.cancel.load(Ordering::Relaxed) => {
                return SendEvent::Cancelled {
                    transfer,
                    sent: index,
                };
            }
            Err(message) => {
                return SendEvent::Failed {
                    transfer,
                    file: job.label(path),
                    message,
                    unsent: count - index,
                };
            }
        }
    }
    SendEvent::Done {
        transfer,
        sent: count,
        count,
    }
}

/// The sends running, each with its flag that ends it. One at a time: the person is watching one
/// progress toast, and each send holds an ssh login.
#[derive(Default)]
pub struct Transfers(Mutex<HashMap<String, Arc<AtomicBool>>>);

impl Transfers {
    fn begin(&self, transfer: &str) -> Result<Arc<AtomicBool>, String> {
        let mut running = self.0.lock().map_err(|_| "the send list is unusable")?;
        if !running.is_empty() {
            return Err("files are already being sent; wait for them or cancel".into());
        }
        let cancel = Arc::new(AtomicBool::new(false));
        running.insert(transfer.to_owned(), Arc::clone(&cancel));
        Ok(cancel)
    }

    fn end(&self, transfer: &str) {
        if let Ok(mut running) = self.0.lock() {
            running.remove(transfer);
        }
    }

    fn cancel(&self, transfer: &str) {
        if let Ok(running) = self.0.lock()
            && let Some(cancel) = running.get(transfer)
        {
            cancel.store(true, Ordering::Relaxed);
        }
    }
}

/// Takes a finished send off the list when it goes out of scope.
struct Ending {
    handle: tauri::AppHandle,
    transfer: String,
}

impl Drop for Ending {
    fn drop(&mut self) {
        self.handle.state::<Transfers>().end(&self.transfer);
    }
}

/// Start sending the dropped files to `machine`. Returns once the files are checked and the send
/// has started; how it goes comes as `file-send` events named by `transfer`.
#[tauri::command]
pub async fn send_files_to_machine(
    app: tauri::AppHandle,
    machines: tauri::State<'_, crate::machines::Machines>,
    machine: String,
    paths: Vec<String>,
    transfer: String,
) -> Result<(), String> {
    let ssh = [maitty_machines::SSH_PROGRAM.to_owned()];
    let job = plan(&machines.0.machines, &ssh, &machine, &transfer, &paths)?;
    start(&app, job)
}

/// Run a planned send on a blocking worker, one at a time. The files of an image job are staged
/// copies the host made, so they are removed once the send ends, however it ends.
pub fn start(app: &tauri::AppHandle, job: Job) -> Result<(), String> {
    let cancel = match app.state::<Transfers>().begin(&job.transfer) {
        Ok(cancel) => cancel,
        Err(refused) => {
            discard_staged(&job);
            return Err(refused);
        }
    };
    let handle = app.clone();
    tauri::async_runtime::spawn_blocking(move || {
        // Dropped even if the send panics, or no later send could start.
        let _ending = Ending {
            handle: handle.clone(),
            transfer: job.transfer.clone(),
        };
        run(&job, &cancel, &|event| {
            if let Err(error) = handle.emit(FILE_SEND_EVENT, event) {
                tracing::warn!("could not tell the window how a file send went: {error}");
            }
        });
        discard_staged(&job);
    });
    Ok(())
}

/// Remove the staged copy of a pasted image; a dropped file is the person's own and stays.
pub(crate) fn discard_staged(job: &Job) {
    if matches!(job.kind, SendKind::Files) {
        return;
    }
    for path in &job.files {
        if let Err(error) = std::fs::remove_file(path)
            && error.kind() != std::io::ErrorKind::NotFound
        {
            tracing::warn!("could not remove a staged image: {error}");
        }
    }
}

/// Stop the send named `transfer`: the file now going is cut off and the rest are not tried.
#[tauri::command]
pub fn cancel_file_send(transfers: tauri::State<'_, Transfers>, transfer: String) {
    transfers.cancel(&transfer);
}

#[cfg(test)]
mod tests;
