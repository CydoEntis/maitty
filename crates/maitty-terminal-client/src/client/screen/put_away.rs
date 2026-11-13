//! Putting a session away and bringing it back. Putting away sets the pane's `archived` flag
//! (`Command::ArchivePane`), a view setting the daemon keeps for every window: nothing closes or
//! stops, the session keeps running and leaves the lists (`screen_model`), and the Put away list
//! (`Action::PutAway`) brings it back. Docs: UI.md, "Put away".

use super::closing::Undone;
use super::*;
use crate::client::menu::Act;

/// How wide a row of the Put away list may get, as the menu draws it.
const LIST_LABEL_MAX: usize = 44;

impl App {
    /// A finished or idle session in a terminal the daemon owns, shells too. A chat cannot be
    /// put away: archiving one is how it is closed, which stops its agent.
    pub(in crate::client) fn can_put_away(&self, term: ClientTerm) -> bool {
        let Some(t) = self.term_info(term) else {
            return false;
        };
        let chat = t.conversation_id.is_some()
            || self
                .snap_of(term.machine)
                .is_some_and(|s| s.conversations.iter().any(|c| c.native_term == Some(term.term)));
        let limited = t.resume_at.is_some_and(|at| at > now_secs());
        let settled = matches!(t.status, Status::None | Status::Idle | Status::Done);
        settled && !t.archived && !chat && !limited && t.exit.is_none()
    }

    /// Put a session away, with the closing banner's Undo.
    pub(in crate::client) fn put_away(&mut self, term: ClientTerm) {
        let name = session_name(self, &self.screen_model(), term);
        // Put away from the sidebar, the keys stay there, on the next row.
        let from_side = self.mode == Mode::Side || self.screen.cursor.is_some();
        let next = if from_side { self.side_after_close() } else { None };
        self.cmd_on(term.machine, Command::ArchivePane { term: term.term, archived: true });
        self.closing.offer(term, name, Undone::PutAway);
        if from_side {
            match next {
                Some(item) => self.screen_side_set(item),
                None => self.screen_side_move(0),
            }
        }
    }

    /// List a put-away session again.
    pub(in crate::client) fn bring_back(&mut self, term: ClientTerm) {
        let name = session_name(self, &self.screen_model_build(true), term);
        self.cmd_on(term.machine, Command::ArchivePane { term: term.term, archived: false });
        self.notify(format!("Brought back {name}"), false);
    }

    /// The sessions put away, as created, each a row that brings it back. It opens upward from
    /// the sidebar's foot, where the menu chip that offers it is.
    pub(in crate::client) fn put_away_list(&mut self) {
        let mut rows: Vec<(ClientTerm, String)> = self
            .screen_model_build(true)
            .iter()
            .flat_map(|p| p.sessions().map(move |s| (p, s)))
            .filter_map(|(p, s)| {
                let label = format!("Bring back {} · {}", s.name, p.name);
                Some((s.native_term?, truncate(&label, LIST_LABEL_MAX)))
            })
            .collect();
        if rows.is_empty() {
            return self.notify("Nothing is put away".into(), false);
        }
        rows.sort_by_key(|(term, _)| *term);
        let items = rows
            .into_iter()
            .map(|(term, label)| (label, Act::BringBack(term)))
            .collect();
        let side = self.screen.side_rect;
        self.menu("put away".into(), items, (side.x + 2, side.bottom().saturating_sub(1)));
    }
}
