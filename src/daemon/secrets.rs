//! Secrets agents ask you for (an API key, say), so they never go through the chat.
//!
//! A request is made only by a pane proving it is that pane (its secret), and only you answer
//! it: `AnswerSecret` is refused from inside a pane. What you type is held here, in memory,
//! and the agent gets a reference. A reference works once, for the pane that asked, and runs
//! out after a day; a restart forgets every one (ADR-0036). The value is never in the
//! snapshot, a log line or a reply to the request.

use super::{ClientId, Daemon, term};
use crate::protocol::{Reply, SecretRequest, SecretText, ServerMsg, TermId};
use anyhow::{Result, bail};
use std::collections::HashMap;
use std::time::{Duration, Instant};

const REF_PREFIX: &str = "secret-ref:";
/// How long a typed secret is kept if the agent never uses it.
pub(super) const KEPT_FOR: Duration = Duration::from_secs(24 * 60 * 60);
/// Longest label and reason shown on the row; more is cut.
const LABEL_CHARS: usize = 80;
const REASON_CHARS: usize = 240;
/// How often the delivered files are checked for stale ones.
const FILES_SWEPT_EVERY: Duration = Duration::from_secs(10 * 60);

struct Asked {
    request: SecretRequest,
    /// The connection waiting for the reference.
    asker: ClientId,
}

struct Held {
    term: TermId,
    value: SecretText,
    runs_out: Instant,
}

/// The requests waiting for you and the secrets you gave.
#[derive(Default)]
pub(super) struct Secrets {
    asked: Vec<Asked>,
    held: HashMap<String, Held>,
}

impl Secrets {
    pub(super) fn is_asking(&self, term: TermId) -> bool {
        self.asked.iter().any(|a| a.request.term == term)
    }

    pub(super) fn requests(&self) -> Vec<SecretRequest> {
        self.asked.iter().map(|a| a.request.clone()).collect()
    }

    /// Take the request `id` off the list; None when it isn't there (answered, or withdrawn).
    fn take_request(&mut self, id: u64) -> Option<Asked> {
        let i = self.asked.iter().position(|a| a.request.id == id)?;
        Some(self.asked.remove(i))
    }

    fn hold(&mut self, reference: String, term: TermId, value: SecretText, now: Instant) {
        self.held.insert(reference, Held { term, value, runs_out: now + KEPT_FOR });
    }

    /// The value behind `reference`, once, if `by` is the pane that asked and it hasn't run
    /// out. A pane that isn't the asker leaves it where it is.
    pub(super) fn spend(&mut self, reference: &str, by: TermId, now: Instant) -> Option<SecretText> {
        if self.held.get(reference).is_none_or(|h| h.term != by) {
            return None;
        }
        let held = self.held.remove(reference)?;
        (held.runs_out > now).then_some(held.value)
    }

    /// The requests a connection was waiting on, which it no longer is: their panes.
    pub(super) fn forget_client(&mut self, client: ClientId) -> Vec<TermId> {
        let gone: Vec<TermId> = self.asked.iter().filter(|a| a.asker == client).map(|a| a.request.term).collect();
        self.asked.retain(|a| a.asker != client);
        gone
    }

    /// Drop what ran out, and what belongs to a pane that is gone. True when a request was dropped.
    pub(super) fn sweep(&mut self, now: Instant, alive: impl Fn(TermId) -> bool) -> bool {
        self.held.retain(|_, h| h.runs_out > now && alive(h.term));
        let before = self.asked.len();
        self.asked.retain(|a| alive(a.request.term));
        self.asked.len() != before
    }
}

/// One line of at most `limit` characters, for a row you read. Control characters would let an
/// agent draw over the prompt, and hidden ones can say what the text doesn't.
fn one_line(text: &str, limit: usize, what: &str) -> Result<String> {
    if text.chars().any(|c| c.is_control() && !c.is_whitespace()) {
        bail!("the {what} has control characters in it");
    }
    if let Err(why) = crate::hidden::check(text) {
        bail!("the {what}: {why}");
    }
    Ok(text.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(limit).collect())
}

/// A new reference: 128 random bits, as hex.
fn new_reference() -> Result<String> {
    Ok(format!("{REF_PREFIX}{}", term::new_secret()?))
}

impl Daemon {
    /// Pane `term`'s agent asks you for a secret. The reference comes back to `client` once you
    /// answer; until then the pane's row needs you.
    pub(super) fn request_secret(&mut self, client: ClientId, term: TermId, label: &str, reason: &str) -> Result<()> {
        if !self.terms.contains_key(&term) {
            bail!("no pane {term}");
        }
        let label = one_line(label, LABEL_CHARS, "label")?;
        let reason = one_line(reason, REASON_CHARS, "reason")?;
        if label.is_empty() {
            bail!("say what the secret is called");
        }
        if self.secrets.is_asking(term) {
            bail!("you are already asking for a secret; wait for the answer");
        }
        let id = u64::from(self.next());
        self.secrets.asked.push(Asked { request: SecretRequest { id, term, label, reason }, asker: client });
        self.announce(term, crate::protocol::Status::Blocked);
        self.dirty = true;
        Ok(())
    }

    /// Your answer to request `id`: the value, or None to decline. Only while it waits.
    pub(super) fn answer_secret(&mut self, id: u64, value: Option<SecretText>) -> Result<()> {
        if value.as_ref().is_some_and(|v| v.0.is_empty()) {
            bail!("a secret can't be empty; decline to say no");
        }
        let reference = if value.is_some() { Some(new_reference()?) } else { None };
        let asked = self.secrets.take_request(id).ok_or_else(|| anyhow::anyhow!("that request was already answered"))?;
        match (value, reference) {
            (Some(value), Some(reference)) => {
                self.secrets.hold(reference.clone(), asked.request.term, value, Instant::now());
                self.send(asked.asker, ServerMsg::Reply(Reply::Text(reference)));
            }
            _ => self.send(asked.asker, ServerMsg::Error("The user declined to give you that secret. Carry on without it, or say what you need.".into())),
        }
        self.dirty = true;
        Ok(())
    }

    /// The value behind a reference, to the pane that asked, once.
    pub(super) fn spend_secret(&mut self, client: ClientId, reference: &str) -> Result<()> {
        let Some(by) = self.clients.get(&client).and_then(|c| c.from) else { bail!("only a pane spends a secret") };
        let value = self.secrets.spend(reference, by, Instant::now()).ok_or_else(|| anyhow::anyhow!("that secret reference is unknown, already used, run out, or not yours"))?;
        self.send(client, ServerMsg::Reply(Reply::Text(value.0)));
        Ok(())
    }

    /// A connection that was waiting on a secret went away: the request goes with it.
    pub(super) fn withdraw_secret_requests(&mut self, client: ClientId) {
        if !self.secrets.forget_client(client).is_empty() {
            self.dirty = true;
        }
    }

    /// Forget secrets that ran out and anything belonging to a pane that is gone, and clear
    /// the delivered files that outlived a day.
    pub(super) fn sweep_secrets(&mut self) {
        let terms = &self.terms;
        if self.secrets.sweep(Instant::now(), |t| terms.contains_key(&t)) {
            self.dirty = true;
        }
        if self.last_secret_files.elapsed() >= FILES_SWEPT_EVERY {
            self.last_secret_files = Instant::now();
            self.clear_secret_files(crate::secret_files::KEPT_FOR);
        }
    }

    /// Remove delivered secret files older than `older_than`, off the loop.
    pub(super) fn clear_secret_files(&self, older_than: Duration) {
        tokio::task::spawn_blocking(move || {
            if let Err(e) = crate::secret_files::remove_stale(&crate::secret_files::folder(), older_than, std::time::SystemTime::now()) {
                tracing::warn!("clearing delivered secret files: {e:#}");
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ask(secrets: &mut Secrets, id: u64, term: TermId, client: ClientId) {
        secrets.asked.push(Asked { request: SecretRequest { id, term, label: "key".into(), reason: "to call it".into() }, asker: client });
    }

    #[test]
    fn a_reference_works_once_and_only_for_the_pane_that_asked() {
        let mut s = Secrets::default();
        let now = Instant::now();
        s.hold("secret-ref:a".into(), 3, SecretText("hunter2".into()), now);
        assert_eq!(s.spend("secret-ref:a", 4, now), None, "another pane");
        assert_eq!(s.spend("secret-ref:a", 3, now).map(|v| v.0), Some("hunter2".to_string()), "the pane that asked still can, after the other tried");
        assert_eq!(s.spend("secret-ref:a", 3, now), None, "used up");
        assert_eq!(s.spend("secret-ref:nope", 3, now), None, "unknown");
    }

    #[test]
    fn a_reference_runs_out_after_a_day() {
        let mut s = Secrets::default();
        let now = Instant::now();
        s.hold("secret-ref:a".into(), 3, SecretText("x".into()), now);
        s.hold("secret-ref:b".into(), 3, SecretText("y".into()), now);
        assert_eq!(s.spend("secret-ref:a", 3, now + KEPT_FOR), None, "at the end of the day");
        assert!(s.spend("secret-ref:b", 3, now + KEPT_FOR - Duration::from_secs(1)).is_some(), "a second before");
    }

    #[test]
    fn sweeping_drops_what_ran_out_and_what_belongs_to_a_closed_pane() {
        let mut s = Secrets::default();
        let now = Instant::now();
        s.hold("secret-ref:old".into(), 3, SecretText("x".into()), now);
        s.hold("secret-ref:gone".into(), 9, SecretText("y".into()), now);
        s.hold("secret-ref:ok".into(), 3, SecretText("z".into()), now + KEPT_FOR);
        ask(&mut s, 1, 9, 1);
        ask(&mut s, 2, 3, 1);
        assert!(s.sweep(now + KEPT_FOR - Duration::from_secs(1), |t| t == 3), "the closed pane's request is dropped");
        assert_eq!(s.requests().iter().map(|r| r.id).collect::<Vec<_>>(), [2]);
        assert!(s.spend("secret-ref:gone", 9, now).is_none() && s.spend("secret-ref:ok", 3, now).is_some());
        assert!(!s.sweep(now + KEPT_FOR, |t| t == 3), "nothing more to drop");
        assert!(s.spend("secret-ref:old", 3, now).is_none());
    }

    #[test]
    fn a_connection_that_goes_away_takes_its_requests() {
        let mut s = Secrets::default();
        ask(&mut s, 1, 3, 7);
        ask(&mut s, 2, 4, 8);
        assert_eq!(s.forget_client(7), [3]);
        assert!(!s.is_asking(3) && s.is_asking(4));
        assert!(s.forget_client(7).is_empty());
    }

    #[test]
    fn a_label_is_one_plain_line() {
        assert_eq!(one_line("  OpenAI \n API  key ", 80, "label").unwrap(), "OpenAI API key");
        assert_eq!(one_line(&"x".repeat(500), LABEL_CHARS, "label").unwrap().chars().count(), LABEL_CHARS);
        assert!(one_line("key\u{1b}[2J", 80, "label").unwrap_err().to_string().contains("control characters"));
        assert!(one_line("key\u{202e}drowssap", 80, "label").unwrap_err().to_string().contains("hidden characters"));
    }
}
