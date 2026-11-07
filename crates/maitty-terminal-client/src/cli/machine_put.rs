//! `<app> machine put <name> <local path>`: send one file or one folder from this computer to a
//! configured machine over ssh (ADR 0007). It lands in the machine's private cache folder under a
//! name made from the local one, and the line on stdout is where it is there. A file is sent up to
//! 100 MB, a folder up to 1 GB (as a tar archive). Progress goes to stderr, and only when a person
//! is watching.

use crate::cli::machine_get::draw_progress;
use crate::cli::machine_update::Failure;
use crate::cli::target::resolve_machine;
use crate::config::Config;
use crate::ipc::{Machine, SSH_PROGRAM};
use maitty_machines::remote_send::send_path;
use maitty_machines::ssh_run::Watch;
use std::io::IsTerminal;
use std::path::Path;
use std::sync::atomic::Ordering;

/// `machine put`: the entry point from the command line. Exits through the returned failure; the
/// caller prints it.
pub(crate) fn run(name: &str, local: &Path, cfg: &Config) -> Result<(), Failure> {
    let machine =
        resolve_machine(name, cfg, SSH_PROGRAM).map_err(|e| Failure::Usage(e.to_string()))?;
    if !machine.is_remote() || machine.transport_program().is_some() {
        return Err(Failure::Usage(
            "name a machine from [[machines]] in config.local.toml that is reached over ssh"
                .into(),
        ));
    }
    let remote = push(&machine, local, std::io::stderr().is_terminal())?;
    println!("{remote}");
    Ok(())
}

/// Send `local` to `machine`, drawing progress on stderr when asked. Returns the path there.
pub(crate) fn push(machine: &Machine, local: &Path, show_progress: bool) -> Result<String, Failure> {
    let watch = Watch::default();
    let drawing = show_progress.then(|| draw_progress(machine.name().to_string(), watch.clone()));
    let result = send_path(machine, machine.name(), local, &watch);
    if let Some((stop, thread)) = drawing {
        stop.store(true, Ordering::Relaxed);
        let _ = thread.join();
    }
    result.map_err(Failure::Failed)
}
