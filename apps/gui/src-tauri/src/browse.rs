//! Looking into a session's machine from the window (ADR 0007): what is in one folder, and an
//! index of the files below a folder for a search. Read-only, over the machine's own ssh line
//! (`maitty-machines`' `list_folder` and `index_files`), on a blocking worker. Unlike a file
//! transfer these are quick questions, so they do not use the one-at-a-time transfer list.

use crate::file_send::target;
use maitty_machines::{Index, Listing, Machine, MachineDef, index_files, list_folder};

/// The id the shared machine check wants; browsing has no transfer to name.
const NO_TRANSFER: &str = "browse";

/// The machine a request names and the folder, both checked before anything runs. An empty
/// `path` is the machine's home folder.
fn plan(
    machines: &[MachineDef],
    ssh: &[String],
    machine: &str,
    path: &str,
) -> Result<(Machine, String), String> {
    let target = target(machines, ssh, machine, NO_TRANSFER)?;
    // The same check the ssh line makes: a path that is too long or has control characters is
    // refused here, before a worker starts.
    target
        .machine
        .list_argv(path, 1)
        .map_err(|e| format!("{e:#}"))?;
    Ok((target.machine, target.name))
}

fn host_plan(
    machines: &crate::machines::Machines,
    machine: &str,
    path: &str,
) -> Result<(Machine, String), String> {
    let ssh = [maitty_machines::SSH_PROGRAM.to_owned()];
    plan(&machines.0.machines, &ssh, machine, path)
}

/// What is in the folder `path` on `machine`; an empty `path` is its home folder.
#[tauri::command]
pub async fn list_remote_folder(
    machines: tauri::State<'_, crate::machines::Machines>,
    machine: String,
    path: String,
) -> Result<Listing, String> {
    let (machine, name) = host_plan(&machines, &machine, &path)?;
    tauri::async_runtime::spawn_blocking(move || list_folder(&machine, &name, &path))
        .await
        .map_err(|e| format!("the question stopped unexpectedly: {e}"))?
}

/// The files below the folder `folder` on `machine`, for a search.
#[tauri::command]
pub async fn index_remote_files(
    machines: tauri::State<'_, crate::machines::Machines>,
    machine: String,
    folder: String,
) -> Result<Index, String> {
    let (machine, name) = host_plan(&machines, &machine, &folder)?;
    tauri::async_runtime::spawn_blocking(move || index_files(&machine, &name, &folder))
        .await
        .map_err(|e| format!("the question stopped unexpectedly: {e}"))?
}

#[cfg(test)]
mod tests;
