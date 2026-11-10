//! Replacing a running server and window with the installed the app, in one step: what the
//! server publishes about its sessions (so a newer the app can decide without asking an
//! older server anything), the policy for when to ask, and re-opening the window.

use crate::ipc;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
    let written = serde_json::to_vec(a)
        .map_err(std::io::Error::other)
        .and_then(|bytes| std::fs::write(&tmp, bytes))
        .and_then(|_| std::fs::rename(&tmp, &path));
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
    let pid: u32 = std::fs::read_to_string(pid_file)
        .ok()?
        .trim()
        .parse()
        .ok()?;
    (a.pid == pid).then_some(a)
}

/// What to do about a server of another protocol that this the app can't talk to.
#[derive(Debug, PartialEq, Eq)]
pub enum Plan {
    /// No session is open: replace the server without asking.
    Replace,
    /// Sessions are open: ask once. A restart ends whatever runs in them (a build, a dev server,
    /// an agent's turn), and only the person knows whether that can go. `resume` is how many
    /// sessions come back, `busy` how many of them are mid-turn (unknown for a server that
    /// published nothing).
    Ask { resume: usize, busy: Option<usize> },
}

/// `saved_agents`: the agents in the saved session, the best guess for a server that
/// published nothing (one from before this existed).
pub fn plan_for_mismatch(
    daemon_protocol: u32,
    published: Option<&Activity>,
    saved_agents: usize,
) -> Plan {
    // A server newer than this the app is never replaced by it unasked: that would be a downgrade.
    if daemon_protocol > crate::protocol::PROTOCOL_VERSION {
        return Plan::Ask {
            resume: published.map_or(saved_agents, |a| a.sessions),
            busy: published.map(Activity::busy),
        };
    }
    match published {
        Some(a) if a.sessions == 0 => Plan::Replace,
        Some(a) => Plan::Ask {
            resume: a.sessions,
            busy: Some(a.busy()),
        },
        None if saved_agents == 0 => Plan::Replace,
        None => Plan::Ask {
            resume: saved_agents,
            busy: None,
        },
    }
}

/// Whether a server this the app can talk to should still be replaced on launch: it is an
/// older release than this one and holds no session. One with sessions open is left running,
/// whatever they are doing: a restart ends the programs in them, and the window's Restart
/// offer is where the person decides that.
pub fn replace_older_idle_server(published: Option<&Activity>, ours: &str) -> bool {
    published.is_some_and(|a| idle(a) && crate::version::newer_than(ours, &a.version))
}

fn idle(a: &Activity) -> bool {
    a.sessions == 0
}

/// Whether the running daemon already is `ours`. A daemon that published nothing is not known
/// to be, so an update still tries to replace it.
pub fn is_running(published: Option<&Activity>, ours: &str) -> bool {
    published.is_some_and(|a| a.version == ours)
}

/// Whether a daemon may be restarted right now to take a newly installed version: it published
/// its activity and holds no session. One with sessions open, or that said nothing, is left
/// running, so an update never ends anyone's work.
pub fn idle_enough_to_restart(published: Option<&Activity>) -> bool {
    published.is_some_and(|a| a.sessions == 0)
}

/// Whether a daemon may be restarted even with sessions open: they are saved, so the next
/// daemon restores them and resumes their agents. Only a turn in progress is lost.
pub fn may_restart_now(published: Option<&Activity>, session_saved: bool) -> bool {
    session_saved || published.is_some_and(|a| a.sessions == 0)
}

/// Whether a saved session exists for the next daemon to restore.
pub fn session_is_saved() -> bool {
    crate::daemon::session_is_saved()
}

/// The agents in the saved session.
pub fn saved_agents() -> usize {
    crate::daemon::saved_agent_count()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn activity(working: usize, waiting: usize) -> Activity {
        Activity {
            pid: 1,
            version: "0.40.0".into(),
            sessions: 4,
            working,
            waiting,
        }
    }

    fn no_sessions() -> Activity {
        Activity {
            sessions: 0,
            ..activity(0, 0)
        }
    }

    const OLDER: u32 = crate::protocol::PROTOCOL_VERSION - 1;

    #[test]
    fn only_a_daemon_that_published_this_version_counts_as_running_it() {
        assert!(is_running(Some(&activity(0, 0)), "0.40.0"));
        assert!(!is_running(Some(&activity(0, 0)), "0.41.0"));
        assert!(!is_running(None, "0.40.0"), "a daemon that said nothing is not known to be current");
    }

    #[test]
    fn a_newer_server_is_never_replaced_without_asking() {
        let newer = crate::protocol::PROTOCOL_VERSION + 1;
        assert!(matches!(
            plan_for_mismatch(newer, Some(&activity(0, 0)), 0),
            Plan::Ask { .. }
        ));
        assert!(matches!(
            plan_for_mismatch(newer, None, 0),
            Plan::Ask { .. }
        ));
    }

    #[test]
    fn a_server_with_no_session_is_replaced_without_asking() {
        assert_eq!(
            plan_for_mismatch(OLDER, Some(&no_sessions()), 9),
            Plan::Replace,
            "what is saved doesn't matter once the server says it holds nothing"
        );
    }

    #[test]
    fn open_sessions_ask_even_when_no_agent_is_mid_turn() {
        assert_eq!(
            plan_for_mismatch(OLDER, Some(&activity(0, 0)), 0),
            Plan::Ask {
                resume: 4,
                busy: Some(0)
            },
            "a shell may be running a build or a server that a restart would end"
        );
    }

    #[test]
    fn working_or_waiting_sessions_ask_once_and_count_what_resumes() {
        assert_eq!(
            plan_for_mismatch(OLDER, Some(&activity(1, 0)), 0),
            Plan::Ask {
                resume: 4,
                busy: Some(1)
            }
        );
        assert_eq!(
            plan_for_mismatch(OLDER, Some(&activity(0, 2)), 0),
            Plan::Ask {
                resume: 4,
                busy: Some(2)
            }
        );
    }

    #[test]
    fn a_server_that_published_nothing_is_judged_by_the_saved_agents() {
        assert_eq!(plan_for_mismatch(OLDER, None, 0), Plan::Replace);
        assert_eq!(
            plan_for_mismatch(OLDER, None, 3),
            Plan::Ask {
                resume: 3,
                busy: None
            }
        );
    }

    #[test]
    fn only_an_older_idle_server_is_replaced_on_launch() {
        assert!(replace_older_idle_server(Some(&no_sessions()), "0.41.0"));
        assert!(
            !replace_older_idle_server(Some(&activity(1, 0)), "0.41.0"),
            "never under a working agent"
        );
        assert!(
            !replace_older_idle_server(Some(&activity(0, 0)), "0.41.0"),
            "never under open sessions, whatever they are doing"
        );
        assert!(
            !replace_older_idle_server(Some(&no_sessions()), "0.40.0"),
            "the same version stays"
        );
        assert!(
            !replace_older_idle_server(Some(&no_sessions()), "0.39.0"),
            concat!(
                "a newer server is never replaced by an older ",
                env!("APP_COMMAND")
            )
        );
        assert!(!replace_older_idle_server(None, "0.41.0"));
    }

    #[test]
    fn the_published_activity_belongs_to_the_pid_file_s_process() {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-activity-{}"),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let (data, pid) = (dir.join("a.activity"), dir.join("a.pid"));
        let a = activity(0, 0);
        std::fs::write(&data, serde_json::to_vec(&a).unwrap()).unwrap();
        std::fs::write(&pid, "1").unwrap();
        assert_eq!(read_from(&data, &pid), Some(a));
        std::fs::write(&pid, "2").unwrap();
        assert_eq!(
            read_from(&data, &pid),
            None,
            "a file from another server process is ignored"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn only_a_daemon_with_no_session_is_restarted_for_an_update() {
        let mut none = activity(0, 0);
        none.sessions = 0;
        assert!(idle_enough_to_restart(Some(&none)));
        assert!(
            !idle_enough_to_restart(Some(&activity(0, 0))),
            "idle sessions are still sessions"
        );
        assert!(!idle_enough_to_restart(Some(&activity(2, 0))));
        assert!(!idle_enough_to_restart(None), "unknown is left running");
    }

    #[test]
    fn a_restart_now_needs_saved_sessions_or_none_open() {
        let busy = activity(2, 1);
        assert!(may_restart_now(Some(&busy), true), "saved sessions come back");
        assert!(!may_restart_now(Some(&busy), false), "unsaved sessions would be lost");
        assert!(!may_restart_now(None, false), "a server that said nothing is left alone");
        let empty = Activity { sessions: 0, ..activity(0, 0) };
        assert!(may_restart_now(Some(&empty), false));
    }
}
