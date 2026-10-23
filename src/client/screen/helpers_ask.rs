//! A session asks to start helpers. Its row turns "needs you" with the line "wants to start N
//! helpers"; Enter on the row (or a click on the line) opens this popover beside it, the
//! screen dimmed except that row. 1 allows, 2 allows in this project for good, 3 says do it
//! alone, Esc decides later and the row stays as it is. Enter, y and Space do nothing, so a
//! stray key cannot hand an agent helpers. The daemon holds the request and takes the answer
//! only from here (`Command::AnswerHelpers`).

use super::*;
use crate::protocol::{ClientMsg, HelperChoice};

const POP_W: u16 = 64;
/// The most tasks listed; the rest are counted.
const TASKS_LISTED: usize = 5;
const NO_REFLEX: &str = "Enter does nothing here. Choose 1, 2 or 3.";

/// A row's second line while its session asks for helpers.
pub(in crate::client) fn helpers_line(n: usize) -> String {
    format!("wants to start {n} helper{}", if n == 1 { "" } else { "s" })
}

/// The same in `room` columns: the short form on a narrow sidebar, so it is not cut off.
pub(in crate::client) fn helpers_line_in(n: usize, room: usize) -> String {
    let full = helpers_line(n);
    if full.width() <= room { full } else { format!("wants {n} helper{}", if n == 1 { "" } else { "s" }) }
}

pub(in crate::client) fn draw_helpers_pop(app: &mut App, buf: &mut Buffer, screen: Rect, side: Rect, term: TermId, t: &Theme) {
    let Some(info) = app.snap.terms.get(&term).cloned() else { return };
    if info.helper_ask.is_empty() {
        return;
    }
    let x0 = side.right() + 1;
    let w = POP_W.min(screen.right().saturating_sub(x0 + 1));
    if w < 30 {
        return;
    }
    let iw = w as usize - 4;
    let agent = info.agent.clone().unwrap_or_else(|| "agent".into());
    let n = info.helper_ask.len();
    let intro = wrap(&format!("{agent} wants to split this into {n} helper{}, each in its own worktree:", if n == 1 { "" } else { "s" }), iw);
    let listed = n.min(TASKS_LISTED);
    let more = n - listed;
    let note = wrap("Their permission prompts come to your Inbox. Nothing merges.", iw);
    let h = (1 + 1 + intro.len() + listed + usize::from(more > 0) + 1 + note.len() + 1 + 1 + 1) as u16;

    let (r, _) = frame_beside_row(app, buf, screen, side, term, (x0, w, h), t);
    let card = Style::default().bg(t.card);
    let name = app.session_name(term);
    put(buf, r.x + 2, r.y, &[seg(" START HELPERS? ", Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD)), seg(format!("· {name} "), card.fg(t.muted))], r.right() - 2);
    let later = " Esc later ";
    put(buf, (r.right() - 2).saturating_sub(later.width() as u16), r.y, &[seg(later, card.fg(t.muted))], r.right() - 2);

    let (x, right) = (r.x + 2, r.right() - 2);
    let mut y = r.y + 2;
    for l in &intro {
        put(buf, x, y, &[seg(l.clone(), card.fg(t.text))], right);
        y += 1;
    }
    for task in info.helper_ask.iter().take(listed) {
        put(buf, x, y, &[seg("  · ", card.fg(t.muted)), seg(truncate(task, iw.saturating_sub(4)), card.fg(t.strong))], right);
        y += 1;
    }
    if more > 0 {
        put(buf, x, y, &[seg(format!("  + {more} more"), card.fg(t.muted))], right);
        y += 1;
    }
    y += 1;
    for l in &note {
        put(buf, x, y, &[seg(l.clone(), card.fg(t.muted))], right);
        y += 1;
    }
    y += 1;
    let project = app.screen_model().iter().find(|p| p.sessions().any(|s| s.term == term)).map(|p| p.name.clone()).unwrap_or_else(|| "this project".into());
    // The full words if the three buttons fit side by side, else shorter ones.
    let named = format!("Always in {}", truncate(&project, 14));
    let tiers = [(named.as_str(), "No, do it alone"), ("Always here", "No, do it alone"), ("Always", "No, alone")];
    let fits = |(always, no): &(&str, &str)| {
        let width = |label: &str, key: &str| segs_width(&button(t, label, key, BtnKind::Normal)) as usize;
        width("Allow", "1") + width(always, "2") + width(no, "3") + 2 <= iw
    };
    let (always, no) = tiers.iter().find(|tier| fits(tier)).unwrap_or(&tiers[2]);
    let mut bx = btn(app, buf, x, y, "Allow", "1", BtnKind::Primary, ScreenHit::HelperAnswer(term, HelperChoice::Allow), right) + 1;
    bx = btn(app, buf, bx, y, always, "2", BtnKind::Normal, ScreenHit::HelperAnswer(term, HelperChoice::Always), right) + 1;
    btn(app, buf, bx, y, no, "3", BtnKind::Normal, ScreenHit::HelperAnswer(term, HelperChoice::No), right);
}

impl App {
    /// Whether session `term` has asked to start helpers and waits for you.
    pub(in crate::client) fn helpers_asked(&self, term: TermId) -> bool {
        self.snap.terms.get(&term).is_some_and(|t| !t.helper_ask.is_empty())
    }

    /// Open the question beside `term`'s row; `side`: back to the sidebar after.
    pub(in crate::client) fn open_helpers(&mut self, term: TermId, side: bool) {
        if self.helpers_asked(term) {
            self.mode = Mode::Helpers { term, side };
        }
    }

    fn helpers_back(&mut self, side: bool) {
        self.mode = if side { Mode::Side } else { Mode::Normal };
    }

    /// Send your answer, say what it did, and close the question.
    pub(in crate::client) fn answer_helpers(&mut self, term: TermId, choice: HelperChoice) {
        let Mode::Helpers { side, .. } = self.mode else { return };
        if !self.helpers_asked(term) {
            self.helpers_back(side);
            return;
        }
        let who = self.session_name(term);
        self.send(ClientMsg::Command(Command::AnswerHelpers { term, choice }));
        self.helpers_back(side);
        let said = match choice {
            HelperChoice::Allow => format!("{who} may start helpers"),
            HelperChoice::Always => format!("helpers may always start here · {who} is starting them"),
            HelperChoice::No => format!("{who} will do it alone"),
        };
        self.notify(said, false);
    }

    pub(in crate::client) fn on_helpers_key(&mut self, term: TermId, side: bool, k: &KeyEvent) {
        let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        match k.code {
            KeyCode::Esc => self.helpers_back(side),
            KeyCode::Char('1') if plain => self.answer_helpers(term, HelperChoice::Allow),
            KeyCode::Char('2') if plain => self.answer_helpers(term, HelperChoice::Always),
            KeyCode::Char('3') if plain => self.answer_helpers(term, HelperChoice::No),
            KeyCode::Enter | KeyCode::Char('y' | ' ') => self.notify(NO_REFLEX.into(), false),
            _ => {}
        }
    }
}
