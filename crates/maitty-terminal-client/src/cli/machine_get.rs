//! `<app> machine get <name> <remote path> [folder]`: pull one file from a configured machine
//! to this computer over ssh (ADR 0007). The file is saved in `folder` (the current folder when
//! none is given) under a name made here from the remote path, never overwriting; the line on
//! stdout is where it landed. Progress goes to stderr, and only when a person is watching.

use crate::cli::machine_update::Failure;
use crate::cli::target::resolve_machine;
use crate::config::Config;
use crate::ipc::{Machine, SSH_PROGRAM};
use maitty_machines::get_file;
use maitty_machines::ssh_run::Watch;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

/// How often the progress line is redrawn.
const PROGRESS_EVERY: Duration = Duration::from_millis(250);

const MEGABYTE: f64 = 1024.0 * 1024.0;

/// `machine get`: the entry point from the command line. Exits through the returned failure;
/// the caller prints it.
pub(crate) fn run(
    name: &str,
    remote_path: &str,
    folder: Option<&Path>,
    cfg: &Config,
) -> Result<(), Failure> {
    let machine =
        resolve_machine(name, cfg, SSH_PROGRAM).map_err(|e| Failure::Usage(e.to_string()))?;
    if !machine.is_remote() || machine.transport_program().is_some() {
        return Err(Failure::Usage(
            "name a machine from [[machines]] in config.local.toml that is reached over ssh"
                .into(),
        ));
    }
    let folder = match folder {
        Some(folder) => folder.to_path_buf(),
        None => std::env::current_dir()
            .map_err(|e| Failure::Failed(format!("could not find the current folder: {e}")))?,
    };
    let path = pull(&machine, remote_path, &folder, std::io::stderr().is_terminal())?;
    println!("{}", path.display());
    Ok(())
}

/// Pull `remote_path` from `machine` into `folder`, drawing progress on stderr when asked.
pub(crate) fn pull(
    machine: &Machine,
    remote_path: &str,
    folder: &Path,
    show_progress: bool,
) -> Result<PathBuf, Failure> {
    let watch = Watch::default();
    let drawing = show_progress.then(|| draw_progress(machine.name().to_string(), watch.clone()));
    let result = get_file(machine, machine.name(), remote_path, folder, &watch);
    if let Some((stop, thread)) = drawing {
        stop.store(true, Ordering::Relaxed);
        let _ = thread.join();
    }
    result.map_err(Failure::Failed)
}

/// A thread that redraws `name: 12.0 of 100.0 MB` until told to stop, then ends the line.
fn draw_progress(
    name: String,
    watch: Watch,
) -> (Arc<AtomicBool>, std::thread::JoinHandle<()>) {
    let stop = Arc::new(AtomicBool::new(false));
    let flag = Arc::clone(&stop);
    let thread = std::thread::spawn(move || {
        let mut drew = false;
        while !flag.load(Ordering::Relaxed) {
            let total = watch.total.load(Ordering::Relaxed);
            if total > 0 {
                let sent = watch.sent.load(Ordering::Relaxed);
                eprint!(
                    "\r{name}: {:.1} of {:.1} MB   ",
                    sent as f64 / MEGABYTE,
                    total as f64 / MEGABYTE
                );
                drew = true;
            }
            std::thread::sleep(PROGRESS_EVERY);
        }
        if drew {
            eprintln!();
        }
    });
    (stop, thread)
}
