//! What can be done with a check result: read its output, run it again, accept a failure and
//! take that back. The row menu, the evidence sheet, the why popover, the Inbox and Settings
//! all come through here, so each surface offers a key exactly when it acts.
use super::check;
use super::*;
use crate::client::menu::{Act, Confirm};
use crate::protocol::{CheckState, OVERRIDE_REASON_MAX_CHARS};
use ratatui::Frame;

const OVERRIDE_TITLE: &str = "Override a failed check";
const OVERRIDE_WIDTH: u16 = 72;
/// Columns the field's padding and cursor take.
const FIELD_CHROME_COLS: usize = 4;
const REASON_PLACEHOLDER: &str = "e.g. flaky on CI";
const HIDES: &str = "This hides a failure. It stays visible:";
const STAYS: &str = "⊘ on the row, in the Inbox history and in the evidence";
const SCOPE: &str = "for this result only; the next run replaces it";

/// The row menu's items for a session whose check has a result, in menu order.
pub(in crate::client) fn check_menu_items(term: TermId, info: &CheckInfo) -> Vec<(String, Act)> {
    let mut items = vec![("Check output".to_string(), Act::CheckOutput(term))];
    if check::settled(info) {
        items.push(("Re-run check".into(), Act::RerunCheck(term)));
    }
    if check::failed(info) {
        items.push(("Override check…".into(), Act::OverrideCheck(term, String::new())));
    }
    items
}

/// What a re-run says, or why it is not sent.
fn rerun_words(t: &TermInfo) -> Result<Option<String>, &'static str> {
    let working = t.status == Status::Working;
    match &t.check {
        Some(c) if c.state == CheckState::Checking => Err("its check is already running"),
        Some(c) if c.rerun_queued => Err("a re-run is already queued: it runs when this turn ends"),
        Some(_) if working => Ok(Some("re-run queued: it runs when this turn ends".into())),
        Some(c) => Ok(Some(format!("re-running {}…", c.command))),
        // The daemon says why a working agent with no result cannot be re-run.
        None if working => Ok(None),
        None => Ok(Some("running the check…".into())),
    }
}

/// `text`'s first line added to `reason`, as far as the daemon will take it.
pub(in crate::client) fn append_reason(reason: &mut String, text: &str) {
    let room = OVERRIDE_REASON_MAX_CHARS.saturating_sub(reason.chars().count());
    reason.extend(text.lines().next().unwrap_or_default().chars().filter(|c| !c.is_control()).take(room));
}

impl App {
    pub(in crate::client) fn rerun_check(&mut self, term: TermId) {
        let Some(t) = self.snap.terms.get(&term) else { return };
        match rerun_words(t) {
            Err(why) => self.notify(why.into(), false),
            Ok(said) => {
                self.cmd(Command::RerunCheck { term });
                if let Some(said) = said {
                    self.notify(said, false);
                }
            }
        }
    }

    pub(in crate::client) fn override_check(&mut self, term: TermId, reason: String) {
        self.cmd(Command::OverrideCheck { term, reason });
        self.notify("overridden: ⊘ stays on the row until the next run replaces it".into(), false);
    }

    pub(in crate::client) fn undo_override(&mut self, term: TermId) {
        self.cmd(Command::UndoOverride { term });
        self.notify("override undone: the check reads as failed again".into(), false);
    }

    /// The confirm for overriding `term`'s check, or None (with a toast) when it is not failed.
    pub(in crate::client) fn override_confirm(&mut self, term: TermId) -> Option<Confirm> {
        let Some(info) = self.snap.terms.get(&term).and_then(|t| t.check.clone()).filter(check::failed) else {
            self.notify("that check isn't failed any more".into(), true);
            return None;
        };
        Some(Confirm {
            title: OVERRIDE_TITLE.into(),
            sub: String::new(),
            what: self.session_name(term),
            detail: info.command,
            note: String::new(),
            list: Vec::new(),
            yes: "Override".into(),
            key: '\n',
            danger: true,
            act: Act::OverrideCheck(term, String::new()),
        })
    }

    pub(in crate::client) fn ask_override(&mut self, term: TermId) {
        if let Some(c) = self.override_confirm(term) {
            self.mode = Mode::Confirm(Box::new(c));
        }
    }

    /// One of a check's keys for `term`: `o` output, `r` re-run, `O` override, `u` undo. Does
    /// nothing, and says so with false, when the result does not allow it.
    pub(in crate::client) fn do_check_key(&mut self, term: TermId, key: char) -> bool {
        let Some(info) = self.snap.terms.get(&term).and_then(|t| t.check.clone()) else { return false };
        match key {
            'o' => self.open_check_sheet(term),
            'r' if check::settled(&info) => self.rerun_check(term),
            'O' if check::failed(&info) => self.ask_override(term),
            'u' if info.state == CheckState::Overridden => self.undo_override(term),
            _ => return false,
        }
        true
    }

    /// Keys in the override confirm: the reason is typed into it; Enter overrides, Esc doesn't.
    pub(in crate::client) fn on_override_key(&mut self, mut c: Confirm, k: &KeyEvent) {
        match k.code {
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                return self.confirm_done(Some(c.act));
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return self.confirm_done(None);
            }
            _ => {}
        }
        if let Act::OverrideCheck(_, reason) = &mut c.act {
            let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
            match k.code {
                KeyCode::Backspace => {
                    reason.pop();
                }
                KeyCode::Char('u') if !plain => reason.clear(),
                KeyCode::Char(ch) if plain => append_reason(reason, &ch.to_string()),
                _ => {}
            }
        }
        self.mode = Mode::Confirm(Box::new(c));
    }
}

/// The override confirm: what it will do, what it hides, what stays visible, and the reason.
pub(in crate::client) fn draw_override(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, c: &Confirm) {
    let Act::OverrideCheck(term, reason) = &c.act else { return };
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    let info = app.snap.terms.get(term).and_then(|x| x.check.clone()).unwrap_or_default();
    let w = OVERRIDE_WIDTH.min(area.width.saturating_sub(2));
    let text_w = w.saturating_sub(6) as usize;
    let result = match info.exit_code {
        Some(code) => format!("exit {code}, {} ago", age(info.started_at + info.duration_ms / 1000)),
        None => format!("{} ago", age(info.started_at)),
    };
    let headline = super::popover::wrap(&format!("{} will read done (overridden) though {} failed ({result}).", c.what, c.detail), text_w);
    // Rows: the headline, a gap, the warning, its two lines, a gap, the field's label and the
    // field, a gap, the buttons.
    let rows = headline.len() as u16 + 9;
    let r = panel(app, buf, area, w, rows + 4, OVERRIDE_TITLE, &[], t);
    let card = Style::default().bg(t.card);
    let (x, right) = (r.x + 3, r.right() - 2);
    let mut y = r.y + 2;
    for line in &headline {
        put(buf, x, y, &[seg(line.clone(), card.fg(t.strong).add_modifier(Modifier::BOLD))], right);
        y += 1;
    }
    y += 1;
    put(buf, x, y, &[seg(HIDES, card.fg(t.text))], right);
    put(buf, x + 2, y + 1, &[seg(STAYS, card.fg(t.muted))], right);
    put(buf, x + 2, y + 2, &[seg(SCOPE, card.fg(t.muted))], right);
    y += 4;
    put(buf, x, y, &[seg("Reason (optional)", card.fg(t.muted).add_modifier(Modifier::BOLD))], right);
    y += 1;
    let field = Style::default().bg(t.card2);
    let inner = (right.saturating_sub(x)) as usize;
    for xx in x..right {
        buf[(xx, y)].set_style(field);
    }
    let keep = inner.saturating_sub(FIELD_CHROME_COLS);
    let tail: String = reason.chars().skip(reason.chars().count().saturating_sub(keep)).collect();
    let mut shown = vec![seg(format!(" {tail}"), field.fg(t.strong)), seg("█", field.fg(t.accent))];
    if reason.is_empty() {
        shown.push(seg(format!(" {REASON_PLACEHOLDER}"), field.fg(t.muted).add_modifier(Modifier::ITALIC)));
    }
    put(buf, x, y, &shown, right);
    y += 2;
    let yes = button(t, &c.yes, "Enter", BtnKind::Danger, false);
    let yr = Rect { x, y, width: segs_width(&yes), height: 1 };
    put(buf, yr.x, yr.y, &yes, r.right());
    let nr = Rect { x: yr.right() + 2, y, width: 12, height: 1 };
    put(buf, nr.x, nr.y, &button(t, "Cancel", "Esc", BtnKind::Normal, hovered(app, nr)), r.right());
    hit(app, area, ScreenHit::ConfirmNo);
    hit(app, r, ScreenHit::Noop);
    hit(app, yr, ScreenHit::ConfirmYes);
    hit(app, nr, ScreenHit::ConfirmNo);
}
