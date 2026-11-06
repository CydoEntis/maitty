//! A read-only conversation for an agent a user started in a pane (ADR 0003).
//!
//! The pane stays an ordinary pane. The daemon follows the transcript its own status hooks named
//! (Claude's `<session>.jsonl`, Codex's `rollout-…-<session>.jsonl`), turns it into the same
//! history entries a structured conversation has, and lists it as a `Native` conversation whose
//! `native_term` is the pane, so the desktop chat view can open it.
//!
//! History is display, not status: the conversation's status is the pane's, and nothing read here
//! can change it. Nothing is persisted; the provider's file is the record.

use super::follow_transcript::{Format, Note, Transcript};
use super::history::{Completeness, Cursor, History, IncompleteReason, MAX_READ_BYTES};
use crate::conversation::{ConversationId, Provider};
use crate::daemon::{Daemon, Ev};
use crate::protocol::{Status, Surface, TermId};
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

/// How often a followed file is looked at for new lines.
const FOLLOW_EVERY: Duration = Duration::from_secs(1);
/// The most one poll reads.
const MAX_CHUNK_BYTES: u64 = 4 * 1024 * 1024;
/// How much of the end of a long file is read the first time; the chat then says the start is
/// missing.
const MAX_FIRST_READ_BYTES: u64 = 16 * 1024 * 1024;
/// A line longer than this is dropped whole (a pasted image or a huge tool result).
const MAX_LINE_BYTES: usize = 1024 * 1024;
/// A followed history stops growing here and says it is cut off.
const MAX_ENTRIES: usize = 20_000;

/// One followed pane.
pub(in crate::daemon) struct Followed {
    pub id: ConversationId,
    pub generation: u64,
    pub term: TermId,
    pub provider: Provider,
    session: String,
    /// The transcript path the hook named, if it did (Codex's can be found by its session id).
    hint: Option<PathBuf>,
    history: History,
    /// Where reading resumes, and the parser that carries the file's turn and tool state. The
    /// parser is out with the background read while one is in flight.
    offset: Option<u64>,
    parser: Option<Transcript>,
    in_flight: bool,
    last_read: Option<Instant>,
    /// Why the file could not be read, in words that carry no file content.
    error: Option<String>,
}

impl Followed {
    fn new(
        id: ConversationId,
        generation: u64,
        term: TermId,
        transcript: Option<PathBuf>,
        session: &str,
        provider: Provider,
    ) -> Self {
        let mut history = History::new(id);
        // Complete once the first read succeeds; until then the chat shows what it has.
        history.completeness = Completeness::Loading;
        Self {
            id,
            generation,
            term,
            provider,
            session: session.to_owned(),
            hint: transcript,
            history,
            offset: None,
            parser: Some(Transcript::new(format_of(provider), session)),
            in_flight: false,
            last_read: None,
            error: None,
        }
    }

    pub fn error(&self) -> Option<String> {
        self.error.clone()
    }

    fn reset(&mut self, generation: u64, session: &str, hint: Option<PathBuf>) {
        let mut history = History::new(self.id);
        history.completeness = Completeness::Loading;
        // A revision never goes back: a reader holding the old one must see it change.
        history.revision = self.history.revision.saturating_add(1);
        self.generation = generation;
        self.session = session.to_owned();
        self.hint = hint;
        self.history = history;
        self.offset = None;
        self.parser = Some(Transcript::new(format_of(self.provider), session));
        self.last_read = None;
        self.error = None;
        // A read still in flight belongs to the old session; its result is dropped on arrival
        // because the session no longer matches.
    }
}

fn format_of(provider: Provider) -> Format {
    match provider {
        Provider::Claude => Format::Claude,
        Provider::Codex => Format::Codex,
    }
}

fn provider_of(agent: Option<&str>) -> Option<Provider> {
    match agent?.trim().to_ascii_lowercase().as_str() {
        "claude" => Some(Provider::Claude),
        "codex" => Some(Provider::Codex),
        _ => None,
    }
}

/// What a background read gives back.
pub(in crate::daemon) struct Chunk {
    notes: Vec<Note>,
    offset: u64,
    parser: Transcript,
    /// The file shrank or was replaced: what was read before is void.
    restarted: bool,
    /// Only the end of a long file was read.
    start_missing: bool,
}

/// The result of one background read of a followed file.
pub struct Done {
    term: TermId,
    id: ConversationId,
    generation: u64,
    session: String,
    result: Result<Chunk, ReadFailure>,
}

pub(in crate::daemon) enum ReadFailure {
    /// The file is not one this pane's session may be read from (path, name or place).
    Refused(&'static str),
    NotFound,
    Io,
}

impl ReadFailure {
    fn words(&self) -> &'static str {
        match self {
            Self::Refused(why) => why,
            Self::NotFound => "The agent's transcript file was not found",
            Self::Io => "The agent's transcript file could not be read",
        }
    }
}

impl Daemon {
    /// Whether this id belongs to a followed pane. Such a conversation can be read, nothing else.
    pub(in crate::daemon) fn is_followed(&self, id: ConversationId) -> bool {
        self.followed.iter().any(|f| f.id == id)
    }

    /// Refuses a command a followed conversation cannot take, in words the person can act on.
    pub(in crate::daemon) fn refuse_if_followed(&self, id: ConversationId) -> anyhow::Result<()> {
        anyhow::ensure!(
            !self.is_followed(id),
            "This chat shows an agent running in a terminal pane and is read-only; use the terminal to talk to it"
        );
        Ok(())
    }

    /// The panes that should have a followed conversation, with what each is followed by.
    fn followable(&self) -> Vec<(TermId, Provider, String)> {
        self.terms
            .values()
            .filter(|t| {
                t.owner == Surface::Desktop
                    && t.remote.is_none()
                    && !t.popup
                    && !t.spare
                    && !t.asleep
                    && !t.process_exited
                    // Hook-verified (process chain or pane token), the same trust status has.
                    && t.hooked
                    && t.conversation_id.is_none()
            })
            .filter_map(|t| {
                let provider = provider_of(t.agent.as_deref())?;
                let session = t.session.clone().filter(|s| !s.trim().is_empty())?;
                // Claude's hook names the file; without it the daemon would have to guess.
                (provider == Provider::Codex || t.transcript.is_some())
                    .then_some((t.id, provider, session))
            })
            .collect()
    }

    /// Adds, resets and removes followed conversations to match the panes, and starts a
    /// background read of each file that is due. Called on the daemon's tick.
    pub(in crate::daemon) fn follow_tick(&mut self) {
        let wanted = self.followable();
        let before = self.followed.len();
        self.followed.retain(|f| {
            wanted
                .iter()
                .any(|(term, provider, _)| *term == f.term && *provider == f.provider)
        });
        let mut changed = self.followed.len() != before;
        for (term, provider, session) in wanted {
            let Some(transcript) = self.terms.get(&term).map(|pane| pane.transcript.clone()) else {
                continue;
            };
            match self.followed.iter().position(|f| f.term == term) {
                Some(index) => {
                    let follower = &mut self.followed[index];
                    if follower.session != session {
                        let generation = follower.generation.saturating_add(1);
                        follower.reset(generation, &session, transcript);
                        changed = true;
                    } else if follower.hint.is_none() {
                        follower.hint = transcript;
                    }
                }
                None => {
                    let Ok(id) = self.next_followed_id() else {
                        tracing::warn!("no conversation id left for a followed pane");
                        continue;
                    };
                    self.followed
                        .push(Followed::new(id, 1, term, transcript, &session, provider));
                    changed = true;
                }
            }
        }
        if changed {
            self.dirty = true;
        }
        self.follow_reads();
    }

    fn next_followed_id(&mut self) -> anyhow::Result<ConversationId> {
        let used = self
            .conversations
            .iter()
            .map(|c| c.id.0)
            .chain(self.pending_starts.iter().map(|p| p.id.0))
            .chain(self.followed.iter().map(|f| f.id.0))
            .chain(std::iter::once(self.followed_high_water))
            .max()
            .unwrap_or(0);
        let next = used
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Conversation identity exhausted"))?;
        // Never handed out again while the daemon lives, even after the pane closes.
        self.followed_high_water = next;
        Ok(ConversationId(next))
    }

    fn follow_reads(&mut self) {
        for follower in &mut self.followed {
            if follower.in_flight
                || follower
                    .last_read
                    .is_some_and(|at| at.elapsed() < FOLLOW_EVERY)
            {
                continue;
            }
            let Some(parser) = follower.parser.take() else {
                continue;
            };
            follower.in_flight = true;
            follower.last_read = Some(Instant::now());
            let (term, id, generation) = (follower.term, follower.id, follower.generation);
            let (session, hint, provider) = (
                follower.session.clone(),
                follower.hint.clone(),
                follower.provider,
            );
            let offset = follower.offset;
            let tx = self.tx.clone();
            #[cfg(test)]
            let home = self.followed_home.clone().or_else(user_home);
            #[cfg(not(test))]
            let home = user_home();
            tokio::task::spawn_blocking(move || {
                let result = match home {
                    Some(home) => {
                        read_new(provider, &session, hint.as_deref(), offset, parser, &home)
                    }
                    None => Err(ReadFailure::Refused("The home folder is unknown")),
                };
                let done = Done {
                    term,
                    id,
                    generation,
                    session,
                    result,
                };
                // A closed daemon has no consumer for the read.
                let _ = tx.blocking_send(Ev::FollowRead(Box::new(done)));
            });
        }
    }

    /// Applies what a background read found.
    pub(in crate::daemon) fn follow_read_done(&mut self, done: Done) {
        let Some(follower) = self.followed.iter_mut().find(|f| {
            f.term == done.term
                && f.id == done.id
                && f.generation == done.generation
                && f.session == done.session
        }) else {
            return;
        };
        follower.in_flight = false;
        match done.result {
            // Nothing has been read yet and there is no file: the agent has not been given its first
            // prompt, which is where it writes it. That is an empty chat, not a fault.
            Err(ReadFailure::NotFound) if follower.offset.is_none() => {
                follower.error = None;
                if follower.parser.is_none() {
                    follower.parser = Some(Transcript::new(
                        format_of(follower.provider),
                        &follower.session,
                    ));
                }
            }
            Err(failure) => {
                // The parser went with the failed read, so what was read cannot be continued:
                // start over, once, and say why.
                if follower.error.is_none() {
                    let mut history = History::new(follower.id);
                    history.revision = follower.history.revision.saturating_add(1);
                    history.completeness = Completeness::Incomplete(IncompleteReason::MissingPages);
                    follower.history = history;
                    follower.parser = Some(Transcript::new(
                        format_of(follower.provider),
                        &follower.session,
                    ));
                    follower.offset = None;
                }
                if follower.parser.is_none() {
                    follower.parser = Some(Transcript::new(
                        format_of(follower.provider),
                        &follower.session,
                    ));
                }
                follower.error = Some(failure.words().to_owned());
            }
            Ok(chunk) => {
                follower.error = None;
                if chunk.restarted {
                    let mut history = History::new(follower.id);
                    history.revision = follower.history.revision.saturating_add(1);
                    follower.history = history;
                }
                follower.offset = Some(chunk.offset);
                follower.parser = Some(chunk.parser);
                apply_notes(follower, chunk.notes, chunk.start_missing);
            }
        }
    }

    /// Answers a read of a followed conversation from its own history.
    pub(in crate::daemon) fn followed_read(
        &self,
        client: crate::daemon::ClientId,
        id: ConversationId,
        cursor: Option<Cursor>,
        limit: usize,
    ) -> anyhow::Result<()> {
        let follower = self
            .followed
            .iter()
            .find(|f| f.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        // Only the app that owns the pane may read what its agent did (ADR 0004).
        anyhow::ensure!(
            self.shown_to(follower.term, self.surface_of(client)),
            "No such conversation"
        );
        let page = follower
            .history
            .read_generation(follower.generation, cursor.as_ref(), limit, MAX_READ_BYTES)
            .map_err(|e| anyhow::anyhow!("History read refused: {e:?}"))?;
        let status = self
            .terms
            .get(&follower.term)
            .map_or(Status::None, |t| t.status);
        self.send(
            client,
            crate::daemon::ServerMsg::Reply(crate::protocol::Reply::ConversationRead(
                crate::conversation::ReadResult {
                    conversation: id,
                    generation: follower.generation,
                    status,
                    receipt: None,
                    history: page,
                    error: follower.error(),
                    checks: Vec::new(),
                    approvals: Vec::new(),
                    answered_approvals: Vec::new(),
                    cancelling: false,
                    queued: Vec::new(),
                },
            )),
        );
        Ok(())
    }
}

/// Puts notes into a followed history. History is capped, and a cap or a problem with a note is
/// said in its completeness, never hidden.
fn apply_notes(follower: &mut Followed, notes: Vec<Note>, start_missing: bool) {
    let id = follower.id;
    let session = follower.session.clone();
    let history = &mut follower.history;
    for note in notes {
        if history.entries.len() >= MAX_ENTRIES {
            let _ =
                history.set_completeness(Completeness::Incomplete(IncompleteReason::MissingPages));
            return;
        }
        match super::history_provider::notification(id, &session, note.method, &note.params) {
            Ok(Some(update)) => {
                if history.apply(update).is_err() {
                    return;
                }
            }
            Ok(None) => {}
            Err(reason) => {
                let _ = history.set_completeness(Completeness::Incomplete(reason));
            }
        }
    }
    if !matches!(history.completeness, Completeness::Incomplete(_)) {
        let complete = if start_missing {
            Completeness::Incomplete(IncompleteReason::MissingPages)
        } else {
            Completeness::Complete
        };
        let _ = history.set_completeness(complete);
    }
}

/// Reads the lines a followed file gained since `offset` (everything, or the end of a long file,
/// when `None`). Runs on a blocking thread.
fn read_new(
    provider: Provider,
    session: &str,
    hint: Option<&Path>,
    offset: Option<u64>,
    mut parser: Transcript,
    home: &Path,
) -> Result<Chunk, ReadFailure> {
    let path = match hint {
        Some(path) => path.to_path_buf(),
        None => find_by_session(provider, session).ok_or(ReadFailure::NotFound)?,
    };
    let path = checked_path(&path, provider, session, home)?;
    let mut file = std::fs::File::open(&path).map_err(|_| ReadFailure::Io)?;
    let len = file.metadata().map_err(|_| ReadFailure::Io)?.len();
    let mut restarted = false;
    let mut start_missing = false;
    let mut from = match offset {
        Some(offset) if offset <= len => offset,
        Some(_) => {
            restarted = true;
            parser = Transcript::new(format_of(provider), session);
            0
        }
        None => 0,
    };
    let mut skip_partial_line = false;
    if offset.is_none() || restarted {
        let tail_from = len.saturating_sub(MAX_FIRST_READ_BYTES);
        if tail_from > 0 {
            from = tail_from;
            start_missing = true;
            skip_partial_line = true;
        }
    }
    file.seek(SeekFrom::Start(from))
        .map_err(|_| ReadFailure::Io)?;
    let mut buffer = Vec::new();
    file.take(MAX_CHUNK_BYTES)
        .read_to_end(&mut buffer)
        .map_err(|_| ReadFailure::Io)?;
    let mut consumed = 0usize;
    if skip_partial_line {
        match buffer.iter().position(|b| *b == b'\n') {
            Some(newline) => consumed = newline + 1,
            // No complete line in the whole tail: nothing to show yet.
            None => {
                return Ok(Chunk {
                    notes: Vec::new(),
                    offset: from,
                    parser,
                    restarted,
                    start_missing,
                });
            }
        }
    }
    let base = from;
    let mut notes = Vec::new();
    let mut resume = consumed;
    while let Some(newline) = buffer[consumed..].iter().position(|b| *b == b'\n') {
        let end = consumed + newline;
        let line = &buffer[consumed..end];
        if line.len() <= MAX_LINE_BYTES {
            let text = String::from_utf8_lossy(line);
            notes.extend(parser.feed(base + consumed as u64, text.trim_end_matches('\r')));
        }
        consumed = end + 1;
        resume = consumed;
    }
    // A last line without its newline is still being written; a chunk that is one huge line
    // with no end is dropped, or it would stall the file for good.
    if resume == 0 && buffer.len() >= MAX_LINE_BYTES {
        resume = buffer.len();
    }
    Ok(Chunk {
        notes,
        offset: from + resume as u64,
        parser,
        restarted,
        start_missing,
    })
}

/// The user's home folder, the only place a followed transcript may be read from.
fn user_home() -> Option<PathBuf> {
    directories::BaseDirs::new().map(|dirs| dirs.home_dir().to_path_buf())
}

/// A Codex rollout file for a session id, found among its recent session files.
fn find_by_session(provider: Provider, session: &str) -> Option<PathBuf> {
    (provider == Provider::Codex)
        .then(|| crate::daemon::usage::codex_file_for(session))
        .flatten()
}

/// The real path of `path` if it may be read as `session`'s transcript, else why not.
///
/// The path came from a hook report, so it is not trusted: it must be a regular `.jsonl` file
/// whose real path (symbolic links followed) is inside the user's home folder, and whose name
/// carries the session id the hook reported, so a pane cannot point the daemon at another file.
fn checked_path(
    path: &Path,
    provider: Provider,
    session: &str,
    home: &Path,
) -> Result<PathBuf, ReadFailure> {
    let real = std::fs::canonicalize(path).map_err(|_| ReadFailure::NotFound)?;
    let home = std::fs::canonicalize(home)
        .map_err(|_| ReadFailure::Refused("The home folder is unknown"))?;
    if !real.starts_with(&home) {
        return Err(ReadFailure::Refused(
            "The transcript is outside the home folder",
        ));
    }
    if !real.is_file() {
        return Err(ReadFailure::Refused("The transcript is not a file"));
    }
    let name = real
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or(ReadFailure::Refused("The transcript name is unreadable"))?;
    let stem = name
        .strip_suffix(".jsonl")
        .ok_or(ReadFailure::Refused("The transcript is not a .jsonl file"))?;
    let named_for_session = match provider {
        Provider::Claude => stem == session,
        Provider::Codex => stem.starts_with("rollout-") && stem.ends_with(session),
    };
    if !named_for_session {
        return Err(ReadFailure::Refused("The transcript is not this session's"));
    }
    Ok(real)
}

#[cfg(test)]
mod tests;
