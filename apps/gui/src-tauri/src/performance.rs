//! What the window cannot see from inside the web view: how much of the machine the host, the
//! web view's own processes and the daemons are using. Settings › Performance and the overlay
//! ask for a sample once a second while they are shown; nothing is sampled otherwise.
use serde::Serialize;
use std::sync::Mutex;
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System};
use tauri::Manager;

/// The process table, kept between samples: a CPU share is the work done since the last refresh,
/// so the first sample of a process reads as 0.
#[derive(Default)]
pub struct ProcessStats(Mutex<Option<System>>);

#[derive(Serialize, Debug, PartialEq)]
pub struct ProcessLoad {
    pub pid: u32,
    pub name: String,
    /// What the process was started as, for telling daemons apart.
    pub command: String,
    /// Percent of one core since the last sample.
    pub cpu: f32,
    pub memory_bytes: u64,
}

#[derive(Serialize, Debug)]
pub struct ProcessSample {
    pub cores: usize,
    pub host: ProcessLoad,
    /// The web view's processes: the host's children that are not daemons. Empty where the
    /// web view is not a child process (macOS runs it under the system).
    pub webview: Vec<ProcessLoad>,
    /// Every daemon of this product on the machine, whoever started it.
    pub daemons: Vec<ProcessLoad>,
}

#[tauri::command]
pub fn performance_sample(app: tauri::AppHandle) -> Result<ProcessSample, String> {
    let product = app.package_info().name.to_ascii_lowercase();
    let stats = app.state::<ProcessStats>();
    let mut system = stats
        .0
        .lock()
        .map_err(|_| "The process table is unavailable".to_string())?;
    let system = system.get_or_insert_with(System::new);
    system.refresh_processes_specifics(
        ProcessesToUpdate::All,
        true,
        ProcessRefreshKind::nothing().without_tasks(),
    );
    let host_pid = Pid::from_u32(std::process::id());
    let mut interesting: Vec<Pid> = vec![host_pid];
    for (pid, process) in system.processes() {
        if process.parent() == Some(host_pid) || is_daemon(process, &product) {
            interesting.push(*pid);
        }
    }
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&interesting),
        true,
        ProcessRefreshKind::nothing()
            .without_tasks()
            .with_cpu()
            .with_memory(),
    );
    let host = system
        .process(host_pid)
        .map(load_of)
        .ok_or_else(|| "The host process is missing from the process table".to_string())?;
    let mut webview = Vec::new();
    let mut daemons = Vec::new();
    for (pid, process) in system.processes() {
        if *pid == host_pid {
            continue;
        }
        if is_daemon(process, &product) {
            daemons.push(load_of(process));
        } else if process.parent() == Some(host_pid) {
            webview.push(load_of(process));
        }
    }
    webview.sort_by_key(|load| load.pid);
    daemons.sort_by_key(|load| load.pid);
    Ok(ProcessSample {
        cores: std::thread::available_parallelism()
            .map(|n| n.get())
            .unwrap_or(1),
        host,
        webview,
        daemons,
    })
}

/// A daemon is this product's executable started with the fixed `daemon` argument, which is how
/// the host, the CLI and the TUI all start one.
fn is_daemon(process: &Process, product: &str) -> bool {
    let name = process.name().to_string_lossy().to_ascii_lowercase();
    name.starts_with(product)
        && process
            .cmd()
            .get(1)
            .is_some_and(|argument| argument.to_string_lossy() == "daemon")
}

fn load_of(process: &Process) -> ProcessLoad {
    let command = process
        .cmd()
        .iter()
        .map(|argument| argument.to_string_lossy())
        .collect::<Vec<_>>()
        .join(" ");
    ProcessLoad {
        pid: process.pid().as_u32(),
        name: process.name().to_string_lossy().into_owned(),
        command,
        cpu: process.cpu_usage(),
        memory_bytes: process.memory(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn system_with_self() -> (System, Pid) {
        let mut system = System::new();
        let pid = Pid::from_u32(std::process::id());
        system.refresh_processes_specifics(
            ProcessesToUpdate::Some(&[pid]),
            true,
            ProcessRefreshKind::nothing().without_tasks().with_memory(),
        );
        (system, pid)
    }

    #[test]
    fn the_test_runner_is_not_a_daemon() {
        let (system, pid) = system_with_self();
        let process = system.process(pid).expect("this process is in the table");
        assert!(!is_daemon(process, ""));
    }

    #[test]
    fn a_load_names_the_process_and_its_memory() {
        let (system, pid) = system_with_self();
        let process = system.process(pid).expect("this process is in the table");
        let load = load_of(process);
        assert_eq!(load.pid, pid.as_u32());
        assert!(!load.name.is_empty());
        assert!(load.memory_bytes > 0);
    }
}
