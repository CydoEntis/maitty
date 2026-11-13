//! Agent sessions that closed for good or whose agent exited with an error: the agent, its own
//! conversation id, the folder, branch and name, and when. Enough to find the work again and
//! resume it; never a message or terminal output.
//!
//! The file is `closed-sessions-<socket>.json` in the data folder, private to the user, replaced
//! atomically, newest first and at most `CLOSED_SESSIONS_MAX` long. It is read tolerantly: a file
//! that can't be read is kept as `.bak` and the list starts empty, never stopping the daemon.

use super::usage_store::{set_aside, write_file};
use super::*;
use crate::conversation::{ConversationId, Provider, clean_title};
use serde::{Deserialize, Serialize};
use std::path::Path;

/// The file format this build writes. A file with a higher number was written by a newer build
/// and is kept untouched as `.bak` rather than read wrongly.
const FORMAT_VERSION: u32 = 1;
const AGENT_MAX_CHARS: usize = 64;
const SESSION_MAX_CHARS: usize = 256;
const BRANCH_MAX_CHARS: usize = 255;

/// The file as stored. Unknown fields are ignored and missing ones default, so a field can be
/// added without a version bump.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    sessions: Vec<ClosedSession>,
}

pub(super) struct ClosedSessions {
    path: Option<PathBuf>,
    /// Newest first.
    sessions: Vec<ClosedSession>,
    /// The latest list not yet on disk. A writer takes it, so a slow write never overwrites a
    /// newer list with an older one.
    unsaved: Arc<Mutex<Option<Vec<u8>>>>,
    /// Held while the file is written, so two writes never share the temporary file.
    writing: Arc<Mutex<()>>,
}

impl ClosedSessions {
    /// A list that lives in memory only (tests, and daemons with no data folder).
    pub fn in_memory() -> ClosedSessions {
        ClosedSessions {
            path: None,
            sessions: Vec::new(),
            unsaved: Arc::default(),
            writing: Arc::default(),
        }
    }

    /// Where this daemon keeps the list, named by socket so a nested daemon never shares it.
    pub fn default_path() -> PathBuf {
        let label =
            std::env::var(concat!(env!("APP_ENV"), "_SOCKET")).unwrap_or_else(|_| "default".into());
        crate::config::data_dir().join(format!("closed-sessions-{label}.json"))
    }

    /// Read the file at `path`, keeping it as the place to save. A missing file starts empty;
    /// one that can't be read is set aside as `.bak` and the list starts empty, saying why.
    pub fn open(path: PathBuf) -> ClosedSessions {
        let mut list = ClosedSessions::in_memory();
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Stored>(&text) {
                Ok(stored) if stored.version <= FORMAT_VERSION => list.fill(stored.sessions),
                Ok(stored) => {
                    tracing::warn!(
                        "closed sessions file {} is format {}, newer than this build reads \
                         ({FORMAT_VERSION}); keeping it as .bak and starting empty",
                        path.display(),
                        stored.version
                    );
                    set_aside(&path);
                }
                Err(error) => {
                    tracing::warn!(
                        "closed sessions file {} can't be read ({error}); keeping it as .bak and \
                         starting empty",
                        path.display()
                    );
                    set_aside(&path);
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => tracing::warn!(
                "closed sessions file {} can't be opened ({error}); starting empty",
                path.display()
            ),
        }
        list.path = Some(path);
        list
    }

    /// What was stored, checked as if it had just been recorded: a hand-edited or damaged entry
    /// is dropped, never trusted.
    fn fill(&mut self, mut stored: Vec<ClosedSession>) {
        stored.sort_by_key(|session| session.at);
        for session in stored {
            self.add(session);
        }
    }

    /// Newest first.
    pub fn list(&self) -> &[ClosedSession] {
        &self.sessions
    }

    /// Put `session` first and save the list off the loop. One with no agent or conversation id
    /// is not kept. The same conversation closing again replaces its older entry.
    pub fn remember(&mut self, session: ClosedSession) {
        if self.add(session) {
            self.save_soon();
        }
    }

    fn add(&mut self, session: ClosedSession) -> bool {
        let Some(session) = cleaned(session) else {
            return false;
        };
        self.sessions
            .retain(|kept| !(kept.agent == session.agent && kept.session == session.session));
        self.sessions.insert(0, session);
        self.sessions.truncate(CLOSED_SESSIONS_MAX);
        true
    }

    fn save_soon(&self) {
        let Some(path) = self.path.clone() else {
            return;
        };
        let bytes = match self.to_bytes() {
            Ok(bytes) => bytes,
            Err(error) => {
                tracing::warn!("couldn't encode the closed sessions: {error:#}");
                return;
            }
        };
        *lock(&self.unsaved) = Some(bytes);
        let (unsaved, writing) = (self.unsaved.clone(), self.writing.clone());
        tokio::task::spawn_blocking(move || write_latest(&path, &unsaved, &writing));
    }

    /// Write what is not on disk yet, on the caller's thread, for when the daemon is about to exit.
    pub fn save_now(&self) {
        if let Some(path) = &self.path {
            write_latest(path, &self.unsaved, &self.writing);
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        Ok(serde_json::to_vec(&Stored {
            version: FORMAT_VERSION,
            sessions: self.sessions.clone(),
        })?)
    }
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Write the latest unsaved list, if any. A failed write puts it back unless a newer one came,
/// so the next write, or the one at exit, tries again. Blocks.
fn write_latest(path: &Path, unsaved: &Mutex<Option<Vec<u8>>>, writing: &Mutex<()>) {
    let _one_at_a_time = lock(writing);
    let Some(bytes) = lock(unsaved).take() else {
        return;
    };
    if let Err(error) = write_file(path, &bytes) {
        tracing::warn!("couldn't save the closed sessions: {error:#}");
        lock(unsaved).get_or_insert(bytes);
    }
}

/// `session` as it may be kept: one clean line per name, cut short, and a title cleaned as a
/// conversation's is. None when there is no agent or conversation id left to resume by.
fn cleaned(session: ClosedSession) -> Option<ClosedSession> {
    let line = |text: &str, max: usize| -> String {
        crate::hidden::sanitize(text, false)
            .trim()
            .chars()
            .take(max)
            .collect()
    };
    let agent = line(&session.agent, AGENT_MAX_CHARS);
    let id = line(&session.session, SESSION_MAX_CHARS);
    if agent.is_empty() || id.is_empty() {
        return None;
    }
    Some(ClosedSession {
        agent,
        session: id,
        cwd: session.cwd,
        branch: session
            .branch
            .map(|branch| line(&branch, BRANCH_MAX_CHARS))
            .filter(|branch| !branch.is_empty()),
        title: session.title.as_deref().and_then(clean_title),
        at: session.at,
    })
}

/// A pane's agent session as it ends, or None for a pane with no agent conversation (a shell)
/// or a hidden spare nobody used. Its name is the one you or the agent gave it, never a prompt.
fn closed_pane(t: &Term) -> Option<ClosedSession> {
    if t.spare {
        return None;
    }
    let title = if t.label.trim().is_empty() { &t.name } else { &t.label };
    Some(ClosedSession {
        agent: t.agent.clone()?,
        session: t.session.clone()?,
        cwd: t.cwd.clone(),
        branch: t.head.as_ref().map(|head| head.branch.clone()),
        title: Some(title.clone()),
        at: term::unix_now(),
    })
}

fn provider_agent(provider: Provider) -> &'static str {
    match provider {
        Provider::Codex => "codex",
        Provider::Claude => "claude",
        Provider::Gemini => "gemini",
    }
}

impl Daemon {
    /// Keep the agent session of a pane a person is closing for good.
    pub(super) fn remember_closed_pane(&mut self, term: TermId) {
        if let Some(closed) = self.terms.get(&term).and_then(closed_pane) {
            self.closed_sessions.remember(closed);
        }
    }

    /// The pane's output ended before the daemon learned how its process exited: keep what it
    /// would record until `pane_process_exited` says.
    pub(super) fn await_pane_exit(&mut self, term: TermId) {
        if let Some(closed) = self.terms.get(&term).and_then(closed_pane) {
            self.unreaped.insert(term, closed);
        }
    }

    /// The pane's own process exited; keep its agent session when that was a failure. A process
    /// the daemon ended on purpose (asleep, or the daemon stopping) is no failure of the agent's.
    pub(super) fn pane_process_exited(&mut self, term: TermId, failed: bool) {
        let ended = self.unreaped.remove(&term);
        if !failed || self.stopping {
            return;
        }
        let closed = match self.terms.get(&term) {
            Some(t) if t.asleep => None,
            Some(t) => closed_pane(t),
            None => ended,
        };
        if let Some(closed) = closed {
            self.closed_sessions.remember(closed);
        }
    }

    /// Keep a chat's agent session once its undo window has passed.
    pub(super) fn remember_closed_chat(&mut self, id: ConversationId) {
        let Some(chat) = self.conversations.iter().find(|c| c.id == id) else {
            return;
        };
        let cwd = chat.binding.worktree.clone().unwrap_or_else(|| chat.binding.cwd.clone());
        let closed = ClosedSession {
            agent: provider_agent(chat.binding.provider).into(),
            session: chat.binding.session_id.clone(),
            branch: crate::gitfs::head(&cwd).map(|head| head.branch),
            cwd,
            title: chat.title.clone(),
            at: term::unix_now(),
        };
        self.closed_sessions.remember(closed);
    }
}

#[cfg(test)]
#[path = "closed_sessions_tests.rs"]
mod tests;
