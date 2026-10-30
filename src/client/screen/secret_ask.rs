//! A session asks you for a secret (an API key, say). Its row turns "needs you" with the line
//! "asks for a secret: <label>"; Enter on the row (or a click on the line) opens this popover
//! beside it, the screen dimmed except that row. What you type shows as dots, goes only to the
//! daemon (`Command::AnswerSecret`), and is dropped from here the moment you send, decline or
//! leave. Enter gives it; Esc declines, and the agent is told to carry on without it. The
//! daemon holds the request and takes the answer only from here.

use super::*;
use crate::protocol::{ClientMsg, SecretRequest, SecretText};

const POP_W: u16 = 64;
/// The longest label the row's line shows before it is cut.
const LABEL_CHARS: usize = 40;
const MASK: char = '•';

/// A row's second line while its session asks for a secret.
pub(in crate::client) fn secret_line(label: &str) -> String {
    format!("asks for a secret: {}", truncate(label, LABEL_CHARS))
}

/// The same in `room` columns: the short form on a narrow sidebar, so it is not cut off.
pub(in crate::client) fn secret_line_in(label: &str, room: usize) -> String {
    let full = secret_line(label);
    if full.width() <= room { full } else { truncate("asks for a secret", room) }
}

/// What a row says while it asks you for something: helpers or a secret.
pub(in crate::client) fn ask_line_in(s: &Session, room: usize) -> String {
    match &s.secret_ask {
        Some(label) => secret_line_in(label, room),
        None => helpers_line_in(s.helpers_wanted, room),
    }
}

pub(in crate::client) fn draw_secret_pop(app: &mut App, buf: &mut Buffer, screen: Rect, side: Rect, term: TermId, t: &Theme) {
    let Some(req) = app.secret_request(term) else { return };
    let x0 = side.right() + 1;
    let w = POP_W.min(screen.right().saturating_sub(x0 + 1));
    if w < 30 {
        return;
    }
    let iw = w as usize - 4;
    let agent = app.snap.terms.get(&term).and_then(|i| i.agent.clone()).unwrap_or_else(|| "agent".into());
    let intro = wrap(&format!("{agent} asks for: {}", req.label), iw);
    let why = if req.reason.is_empty() { Vec::new() } else { wrap(&req.reason, iw) };
    let note = wrap("Goes to this session only, once; never into the chat.", iw);
    let h = (1 + 1 + intro.len() + why.len() + 1 + note.len() + 1 + 1 + 1 + 1 + 1) as u16;

    let (r, _) = frame_beside_row(app, buf, screen, side, term, (x0, w, h), t);
    let card = Style::default().bg(t.card);
    let name = app.session_name(term);
    put(buf, r.x + 2, r.y, &plain(vec![seg(" SECRET? ", Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD)), seg(format!("· {name} "), card.fg(t.muted))]), r.right() - 2);
    let decline = " Esc declines ";
    put(buf, (r.right() - 2).saturating_sub(decline.width() as u16), r.y, &[seg(decline, card.fg(t.muted))], r.right() - 2);

    let (x, right) = (r.x + 2, r.right() - 2);
    let mut y = r.y + 2;
    for l in &intro {
        put(buf, x, y, &[seg(l.clone(), card.fg(t.text))], right);
        y += 1;
    }
    for l in &why {
        put(buf, x, y, &[seg(l.clone(), card.fg(t.muted))], right);
        y += 1;
    }
    y += 1;
    for l in &note {
        put(buf, x, y, &[seg(l.clone(), card.fg(t.muted))], right);
        y += 1;
    }
    y += 1;
    let room = iw.saturating_sub(4);
    let typed = app.screen.secret_typed.0.chars().count();
    let dots: String = std::iter::repeat_n(MASK, typed.min(room)).collect();
    put(buf, x, y, &[seg("▸ ", card.fg(t.accent)), seg(dots, card.fg(t.strong)), seg("▏", card.fg(t.accent))], right);
    y += 2;
    let bx = btn(app, buf, x, y, "Give it", "Enter", BtnKind::Primary, ScreenHit::SecretGive(term), right) + 1;
    btn(app, buf, bx, y, "Decline", "Esc", BtnKind::Normal, ScreenHit::SecretDecline(term), right);
}

impl App {
    /// The request session `term` is waiting on you for, if any.
    pub(in crate::client) fn secret_request(&self, term: TermId) -> Option<SecretRequest> {
        self.snap.secret_requests.iter().find(|r| r.term == term).cloned()
    }

    pub(in crate::client) fn secret_asked(&self, term: TermId) -> bool {
        self.secret_request(term).is_some()
    }

    /// Open the question beside `term`'s row; `side`: back to the sidebar after.
    pub(in crate::client) fn open_secret(&mut self, term: TermId, side: bool) {
        if let Some(req) = self.secret_request(term) {
            self.screen.inbox_return = None;
            self.screen.secret_typed = SecretText::default();
            self.mode = Mode::Secret { term, id: req.id, side };
        }
    }

    /// Open the question from the Inbox's row `sel` (typed `query`): leaving it goes back there.
    pub(in crate::client) fn open_secret_from_inbox(&mut self, term: TermId, query: String, sel: usize) {
        self.open_secret(term, false);
        if matches!(self.mode, Mode::Secret { .. }) {
            self.screen.inbox_return = Some((query, sel));
        }
    }

    /// Whatever session `term` asks you, open it: helpers or a secret.
    pub(in crate::client) fn open_ask(&mut self, term: TermId, side: bool) {
        if self.secret_asked(term) {
            self.open_secret(term, side);
        } else {
            self.open_helpers(term, side);
        }
    }

    pub(in crate::client) fn open_ask_from_inbox(&mut self, term: TermId, query: String, sel: usize) {
        if self.secret_asked(term) {
            self.open_secret_from_inbox(term, query, sel);
        } else {
            self.open_helpers_from_inbox(term, query, sel);
        }
    }

    /// Whether session `term` asks you something this popover family answers.
    pub(in crate::client) fn asks_you(&self, term: TermId) -> bool {
        self.helpers_asked(term) || self.secret_asked(term)
    }

    fn secret_back(&mut self, side: bool) {
        self.screen.secret_typed = SecretText::default();
        self.mode = match self.screen.inbox_return.take() {
            Some((query, sel)) => Mode::GoTo { query, sel },
            None if side => Mode::Side,
            None => Mode::Normal,
        };
    }

    /// Send your answer (None declines), say what it did, and close the question. A request
    /// that is gone, or was replaced, is not answered.
    pub(in crate::client) fn answer_secret(&mut self, term: TermId, give: bool) {
        let Mode::Secret { id, side, .. } = self.mode else { return };
        if self.secret_request(term).is_none_or(|r| r.id != id) {
            self.secret_back(side);
            self.notify("that request is gone".into(), false);
            return;
        }
        if give && self.screen.secret_typed.0.is_empty() {
            self.notify("type the secret first, or Esc to decline".into(), false);
            return;
        }
        let value = give.then(|| std::mem::take(&mut self.screen.secret_typed));
        let who = self.session_name(term);
        self.send(ClientMsg::Command(Command::AnswerSecret { id, value }));
        self.secret_back(side);
        self.notify(if give { format!("gave {who} the secret") } else { format!("declined {who}'s request") }, false);
    }

    pub(in crate::client) fn on_secret_key(&mut self, term: TermId, k: &KeyEvent) {
        let typing = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match k.code {
            KeyCode::Esc => self.answer_secret(term, false),
            KeyCode::Enter => self.answer_secret(term, true),
            KeyCode::Backspace => {
                self.screen.secret_typed.0.pop();
            }
            KeyCode::Char('u') if k.modifiers.contains(KeyModifiers::CONTROL) => self.screen.secret_typed = SecretText::default(),
            KeyCode::Char(c) if typing => self.screen.secret_typed.0.push(c),
            _ => {}
        }
    }

    /// A pasted secret: one line, without the newline a copy often ends with.
    pub(in crate::client) fn paste_into_secret(&mut self, text: &str) {
        if matches!(self.mode, Mode::Secret { .. }) {
            self.screen.secret_typed.0.push_str(text.trim_end_matches(['\r', '\n']));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_row_line_names_the_secret_and_gives_way_on_a_narrow_sidebar() {
        assert_eq!(secret_line_in("OpenAI key", 60), "asks for a secret: OpenAI key");
        assert_eq!(secret_line_in("OpenAI key", 20), "asks for a secret");
        assert!(secret_line(&"x".repeat(200)).width() <= "asks for a secret: ".len() + LABEL_CHARS);
    }
}
