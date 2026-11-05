//! What the window cannot see from inside the web view: how much of the machine the host, the
//! web view's own processes and the daemons are using. Settings › Performance and the overlay
//! ask for a sample once a second while they are shown; nothing is sampled otherwise.
use serde::Serialize;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Mutex;
use sysinfo::{Pid, Process, ProcessRefreshKind, ProcessesToUpdate, System, UpdateKind};
use tauri::Manager;

/// Terminal frames the host has handed to the window's channel since it started. Read next to the
/// window's own count of frames received: a gap that stays open while nothing is typed means the
/// frames are lost, or held, between the two (ticket 35).
static TERMINAL_FRAMES_SENT: AtomicU64 = AtomicU64::new(0);

/// One terminal frame was handed to the window's channel.
pub fn count_terminal_frame_sent() {
    TERMINAL_FRAMES_SENT.fetch_add(1, Ordering::Relaxed);
}

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
    /// Terminal frames sent to the window since the host started; never reset.
    pub terminal_frames_sent: u64,
}

#[tauri::command]
pub fn performance_sample(app: tauri::AppHandle) -> Result<ProcessSample, String> {
    let stats = app.state::<ProcessStats>();
    let mut system = stats
        .0
        .lock()
        .map_err(|_| "The process table is unavailable".to_string())?;
    let system = system.get_or_insert_with(System::new);
    system.refresh_processes_specifics(ProcessesToUpdate::All, true, identity_refresh());
    let host_pid = Pid::from_u32(std::process::id());
    let mut interesting: Vec<Pid> = vec![host_pid];
    for (pid, process) in system.processes() {
        if process.parent() == Some(host_pid) || is_daemon(process) {
            interesting.push(*pid);
        }
    }
    system.refresh_processes_specifics(
        ProcessesToUpdate::Some(&interesting),
        true,
        identity_refresh().with_cpu().with_memory(),
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
        if is_daemon(process) {
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
        terminal_frames_sent: TERMINAL_FRAMES_SENT.load(Ordering::Relaxed),
    })
}

/// What it takes to tell a process by its command line. `nothing()` leaves the command line
/// unread, so every process looked like it had none and no daemon was ever found (ticket 38).
fn identity_refresh() -> ProcessRefreshKind {
    ProcessRefreshKind::nothing()
        .without_tasks()
        .with_cmd(UpdateKind::OnlyIfNotSet)
}

/// A daemon is this product's executable, or the copy the desktop app bundles, started with the
/// fixed `daemon` argument, which is how the host, the CLI and the TUI all start one.
fn is_daemon(process: &Process) -> bool {
    let cmd = process.cmd();
    cmd.first().is_some_and(|program| {
        maitty_desktop_client::is_daemon_invocation(
            program.as_encoded_bytes(),
            cmd.get(1).map(|argument| argument.as_encoded_bytes()),
        )
    })
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
        assert!(!is_daemon(process));
    }

    #[test]
    fn a_refresh_reads_the_command_line_a_daemon_is_told_by() {
        let mut system = System::new();
        let pid = Pid::from_u32(std::process::id());
        system.refresh_processes_specifics(ProcessesToUpdate::Some(&[pid]), true, identity_refresh());
        let process = system.process(pid).expect("this process is in the table");
        assert!(!process.cmd().is_empty());
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
