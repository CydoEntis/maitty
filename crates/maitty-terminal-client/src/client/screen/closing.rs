//! Closing a session: it is always asked first (a dialog, `menu_act`); once accepted the daemon hides the session
//! for six seconds with its process still running (`Command::CloseSession`), and the banner
//! offers Undo until it ends the process (`Command::UndoClose`). Docs: UI.md, "Closing".

use super::*;
use std::time::Duration;

/// How long the daemon keeps a closed session before it ends it (`UNDO_WINDOW` there).
pub(in crate::client) const UNDO_FOR: Duration = Duration::from_secs(6);

/// What the undo takes back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum Undone {
    Closed,
    PutAway,
}

/// A session closed or put away a moment ago.
#[derive(Debug, Clone)]
struct Undo {
    term: ClientTerm,
    name: String,
    at: Instant,
    what: Undone,
}

/// What closing has pending: the undo window.
#[derive(Debug, Default)]
pub(in crate::client) struct Closing {
    undo: Option<Undo>,
}

/// The seconds left of `window` at `elapsed`, never below 1 while it is open.
fn seconds_left(window: Duration, elapsed: Duration) -> u64 {
    window.saturating_sub(elapsed).as_secs_f64().ceil() as u64
}

impl Closing {
    /// The session closed last, while its undo is open.
    #[cfg(test)]
    pub fn last_closed(&self) -> Option<ClientTerm> {
        self.undo.as_ref().map(|u| u.term)
    }

    /// Offer to take back what was just done to `term`.
    pub fn offer(&mut self, term: ClientTerm, name: String, what: Undone) {
        self.undo = Some(Undo { term, name, at: Instant::now(), what });
    }

    /// A close is in its undo window.
    pub fn holds_close(&self) -> bool {
        self.undo
            .as_ref()
            .is_some_and(|u| u.what == Undone::Closed && u.at.elapsed() < UNDO_FOR)
    }

    /// The banner under the pane header: the undo offer.
    pub fn banner(&self, undo_key: &str) -> Option<Banner> {
        let undo = self.undo.as_ref().filter(|u| u.at.elapsed() < UNDO_FOR)?;
        let left = seconds_left(UNDO_FOR, undo.at.elapsed());
        let done = match undo.what {
            Undone::Closed => "Closed",
            Undone::PutAway => "Put away",
        };
        Some(Banner { text: format!("{done} {} · {undo_key} undo {left}s", undo.name), warn: false })
    }

    /// Drop what has run out; true when the banner changed and the screen needs drawing.
    pub fn expire(&mut self) -> bool {
        let before = self.undo.is_some();
        if self.undo.as_ref().is_some_and(|u| u.at.elapsed() >= UNDO_FOR) {
            self.undo = None;
        }
        // The countdown changes every second while it shows.
        before != self.undo.is_some() || self.undo.is_some()
    }
}

/// One line the workspace shows under its header.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::client) struct Banner {
    pub text: String,
    /// A question (amber) rather than an offer.
    pub warn: bool,
}

impl App {
    /// Close a session, once the confirmation (`menu_act`) has been accepted: the daemon hides it and ends it after `UNDO_FOR`
    /// unless it is undone.
    pub(in crate::client) fn close_session(&mut self, term: ClientTerm) {
        let model = self.screen_model();
        let name = session_name(self, &model, term);
        // Closed from the sidebar, the keys stay there, on the next row.
        let from_side = self.mode == Mode::Side || self.screen.cursor.is_some();
        let next = if from_side { self.side_after_close() } else { None };
        // Out of its split first: the pane beside it takes the room and the focus.
        self.screen_unshow(term);
        if let Some(id) = self.term_info(term).and_then(|x| x.conversation_id) {
            self.cmd_on(term.machine, Command::ConversationArchive { id, archived: true });
        }
        self.cmd_on(term.machine, Command::CloseSession { term: term.term });
        self.closing.offer(term, name, Undone::Closed);
        if from_side {
            match next {
                Some(item) => self.screen_side_set(item),
                None => self.screen_side_move(0),
            }
        }
    }

    /// Whether an agent is doing something in the session: closing it would stop that.
    pub(in crate::client) fn agent_running(&self, term: ClientTerm) -> bool {
        self.term_info(term)
            .is_some_and(|t| t.agent.is_some() && matches!(t.status, Status::Working | Status::Blocked))
    }

    /// Bring back the session closed or put away in the last few seconds.
    pub(in crate::client) fn undo_close(&mut self) {
        match self.closing.undo.take().filter(|u| u.at.elapsed() < UNDO_FOR) {
            Some(u) if u.what == Undone::PutAway => self.bring_back(u.term),
            Some(u) => {
                self.cmd_on(u.term.machine, Command::UndoClose { term: u.term.term });
                self.notify(format!("Restored {}", u.name), false);
            }
            None => self.notify("nothing to undo".into(), true),
        }
    }

    /// The banner to draw now.
    pub(in crate::client) fn closing_banner(&self) -> Option<Banner> {
        if let Some(bar) = self.service_banner() {
            return Some(bar);
        }
        let long = crate::client::design::long_key;
        let lead = long(&self.keymap.prefix);
        let undo = self
            .keymap
            .prefixed_order
            .iter()
            .find(|k| self.keymap.prefixed.get(k) == Some(&crate::keys::Action::UndoClose))
            .map_or_else(|| format!("{lead} then search \"undo\""), |k| format!("{lead} then {}", long(k)));
        self.closing.banner(&undo)
    }

    /// Everything the banner line can say, the most pressing first.
    pub(in crate::client) fn banner_now(&mut self) -> Option<Banner> {
        self.closing_banner().or_else(|| self.restart_banner())
    }
}

/// Draw the banner over the first row of the focused pane's content.
pub(in crate::client) fn draw_banner(app: &mut App, f: &mut Frame, t: &Theme) {
    let Some(banner) = app.banner_now() else { return };
    let Some(pane) = app.focused().and_then(|f| app.screen.leaf_rects.iter().find(|(id, _)| *id == f)).map(|(_, r)| *r) else {
        return;
    };
    let (fg, bg) = if banner.warn { (t.acc_ink, t.blocked) } else { (t.strong, t.card2) };
    // Inside the box, on the row under its top border with the title chip.
    let row = Rect { x: pane.x + 1, y: pane.y + 1, width: pane.width.saturating_sub(2), height: 1 };
    let buf = f.buffer_mut();
    fill(buf, row, bg);
    put(buf, row.x + 1, row.y, &[seg(banner.text.clone(), Style::default().fg(fg).bg(bg).add_modifier(Modifier::BOLD))], row.right());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_countdown_never_shows_zero_while_the_window_is_open() {
        assert_eq!(seconds_left(UNDO_FOR, Duration::ZERO), 6);
        assert_eq!(seconds_left(UNDO_FOR, Duration::from_millis(5900)), 1);
        assert_eq!(seconds_left(UNDO_FOR, Duration::from_millis(1500)), 5);
    }

    #[test]
    fn a_closed_session_offers_undo() {
        let term = ClientTerm::local(7);
        let mut c = Closing::default();
        assert_eq!(c.banner("Ctrl+A"), None);
        c.offer(term, "pacman".into(), Undone::Closed);
        let b = c.banner("Ctrl+A").unwrap();
        assert_eq!(b.text, "Closed pacman · Ctrl+A undo 6s");
        assert!(!b.warn);
    }
}
