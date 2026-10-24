//! Replacing a running server and window with the installed vibecasso, in one step: what the
//! server publishes about its sessions (so a newer vibecasso can decide without asking an
//! older server anything), the policy for when to ask, and re-opening the window.

use crate::ipc;
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::time::Duration;

/// How long to wait for a stopped server to let go of its socket before starting the next.
const SERVER_GONE_WITHIN: Duration = Duration::from_secs(5);
const POLL: Duration = Duration::from_millis(100);

/// What the server tells whoever replaces it: which version it is and how many sessions it
/// would interrupt. Written next to its pid file, because a server of another protocol
/// can't be asked.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Activity {
    pub pid: u32,
    pub version: String,
    pub sessions: usize,
    pub working: usize,
    /// Sessions stopped on a question for you.
    pub waiting: usize,
}

impl Activity {
    /// Sessions a restart would interrupt mid-turn.
    pub fn busy(&self) -> usize {
        self.working + self.waiting
    }
}

fn file() -> PathBuf {
    ipc::pid_file().with_extension("activity")
}

/// Note this server's activity (atomically, so a reader never sees half of it).
pub fn publish(a: &Activity) {
    let path = file();
    let tmp = path.with_extension("activity.tmp");
    let written = serde_json::to_vec(a).map_err(std::io::Error::other).and_then(|bytes| std::fs::write(&tmp, bytes)).and_then(|_| std::fs::rename(&tmp, &path));
    if let Err(e) = written {
        tracing::warn!("noting the server's activity: {e}");
    }
}

pub fn clear() {
    let _ = std::fs::remove_file(file());
}

/// The running server's activity, when it published some and it is the process the pid file
/// names (a file left by a server that died says nothing about the one running now).
pub fn read() -> Option<Activity> {
    read_from(&file(), &ipc::pid_file())
}

fn read_from(data: &std::path::Path, pid_file: &std::path::Path) -> Option<Activity> {
    let a: Activity = serde_json::from_slice(&std::fs::read(data).ok()?).ok()?;
    let pid: u32 = std::fs::read_to_string(pid_file).ok()?.trim().parse().ok()?;
    (a.pid == pid).then_some(a)
}

/// What to do about a server of another protocol that this vibecasso can't talk to.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    /// Nothing is mid-turn: replace the server without asking.
    Replace,
    /// Something is working or waiting: ask once. `resume` is how many sessions come back,
    /// `busy` how many of them are mid-turn (unknown for a server that published nothing).
    Ask { resume: usize, busy: Option<usize> },
}

/// `saved_agents`: the agents in the saved session, the best guess for a server that
/// published nothing (one from before this existed).
pub fn plan_for_mismatch(daemon_protocol: u32, published: Option<&Activity>, saved_agents: usize) -> Plan {
    // A server newer than this vibecasso is never replaced by it unasked: that would be a downgrade.
    if daemon_protocol > crate::protocol::PROTOCOL_VERSION {
        return Plan::Ask { resume: published.map_or(saved_agents, |a| a.sessions), busy: published.map(Activity::busy) };
    }
    match published {
        Some(a) if a.busy() == 0 => Plan::Replace,
        Some(a) => Plan::Ask { resume: a.sessions, busy: Some(a.busy()) },
        None if saved_agents == 0 => Plan::Replace,
        None => Plan::Ask { resume: saved_agents, busy: None },
    }
}

/// Whether a server this vibecasso can talk to should still be replaced on launch: it is an
/// older release than this one and nothing is mid-turn.
pub fn replace_older_idle_server(published: Option<&Activity>, ours: &str) -> bool {
    published.is_some_and(|a| a.busy() == 0 && crate::update::newer_than(ours, &a.version))
}

/// The agents in the saved session.
pub fn saved_agents() -> usize {
    crate::daemon::saved_agent_count()
}

/// Wait until no server answers on this socket.
pub fn wait_until_server_gone() -> Result<()> {
    let start = std::time::Instant::now();
    while crate::cli::server_running() {
        if start.elapsed() > SERVER_GONE_WITHIN {
            bail!("the old server is still running; try `vibecasso kill-server`");
        }
        std::thread::sleep(POLL);
    }
    Ok(())
}

/// Open the installed vibecasso in this terminal in place of this one, with the same arguments
/// and environment (so `VIBECASSO_SOCKET` keeps meaning the same server). Returns only when
/// that failed.
pub fn relaunch() -> Result<()> {
    let exe = crate::update::installed_executable()?;
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    let mut cmd = std::process::Command::new(&exe);
    cmd.args(args);
    #[cfg(unix)]
    {
        use std::os::unix::process::CommandExt;
        Err(cmd.exec()).with_context(|| format!("starting {}", exe.display()))
    }
    #[cfg(not(unix))]
    {
        // No exec here: run the new one in this console and leave with its exit code.
        let status = cmd.status().with_context(|| format!("starting {}", exe.display()))?;
        std::process::exit(status.code().unwrap_or(0));
    }
}

/// Set when the window should be replaced by the installed vibecasso as it closes.
static RELAUNCH: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);

pub fn request_relaunch() {
    RELAUNCH.store(true, std::sync::atomic::Ordering::SeqCst);
}

pub fn relaunch_requested() -> bool {
    RELAUNCH.load(std::sync::atomic::Ordering::SeqCst)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn activity(working: usize, waiting: usize) -> Activity {
        Activity { pid: 1, version: "0.40.0".into(), sessions: 4, working, waiting }
    }

    const OLDER: u32 = crate::protocol::PROTOCOL_VERSION - 1;

    #[test]
    fn a_newer_server_is_never_replaced_without_asking() {
        let newer = crate::protocol::PROTOCOL_VERSION + 1;
        assert!(matches!(plan_for_mismatch(newer, Some(&activity(0, 0)), 0), Plan::Ask { .. }));
        assert!(matches!(plan_for_mismatch(newer, None, 0), Plan::Ask { .. }));
    }

    #[test]
    fn an_idle_server_is_replaced_without_asking() {
        assert_eq!(plan_for_mismatch(OLDER, Some(&activity(0, 0)), 9), Plan::Replace, "what is saved doesn't matter once the server says it is idle");
    }

    #[test]
    fn working_or_waiting_sessions_ask_once_and_count_what_resumes() {
        assert_eq!(plan_for_mismatch(OLDER, Some(&activity(1, 0)), 0), Plan::Ask { resume: 4, busy: Some(1) });
        assert_eq!(plan_for_mismatch(OLDER, Some(&activity(0, 2)), 0), Plan::Ask { resume: 4, busy: Some(2) });
    }

    #[test]
    fn a_server_that_published_nothing_is_judged_by_the_saved_agents() {
        assert_eq!(plan_for_mismatch(OLDER, None, 0), Plan::Replace);
        assert_eq!(plan_for_mismatch(OLDER, None, 3), Plan::Ask { resume: 3, busy: None });
    }

    #[test]
    fn only_an_older_idle_server_is_replaced_on_launch() {
        assert!(replace_older_idle_server(Some(&activity(0, 0)), "0.41.0"));
        assert!(!replace_older_idle_server(Some(&activity(1, 0)), "0.41.0"), "never under a working agent");
        assert!(!replace_older_idle_server(Some(&activity(0, 0)), "0.40.0"), "the same version stays");
        assert!(!replace_older_idle_server(Some(&activity(0, 0)), "0.39.0"), "a newer server is never replaced by an older vibecasso");
        assert!(!replace_older_idle_server(None, "0.41.0"));
    }

    #[test]
    fn the_published_activity_belongs_to_the_pid_file_s_process() {
        let dir = std::env::temp_dir().join(format!("vibecasso-activity-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let (data, pid) = (dir.join("a.activity"), dir.join("a.pid"));
        let a = activity(0, 0);
        std::fs::write(&data, serde_json::to_vec(&a).unwrap()).unwrap();
        std::fs::write(&pid, "1").unwrap();
        assert_eq!(read_from(&data, &pid), Some(a));
        std::fs::write(&pid, "2").unwrap();
        assert_eq!(read_from(&data, &pid), None, "a file from another server process is ignored");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
