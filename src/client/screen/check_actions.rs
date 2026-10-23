//! What can be done with a check result: read its output, run it again, accept a failure and
//! take that back. The row menu, the evidence sheet, the why popover, the Inbox and Settings
//! all come through here, so each surface offers a key exactly when it acts.
use super::check;
use super::*;
use crate::client::menu::{Act, Confirm};
use crate::protocol::{CheckState, OVERRIDE_REASON_MAX_CHARS};
use ratatui::Frame;

const OVERRIDE_TITLE: &str = "OVERRIDE A FAILED CHECK";
const OVERRIDE_WIDTH: u16 = 70;
const OVERRIDE_HEIGHT: u16 = 15;
const REASON_PLACEHOLDER: &str = "why is this OK?";
const HIDES: &str = "This hides a failure. It stays visible:";
const STAYS: &str = "on the row, in the Inbox history and in the evidence";
const SCOPE: &str = "for this result only; the next run replaces it";

/// The row menu's items for a session whose check has a result, in menu order.
pub(in crate::client) fn check_menu_items(term: TermId, info: &CheckInfo) -> Vec<(String, Act)> {
    let mut items = vec![("Check output".to_string(), Act::CheckOutput(term))];
    if check::settled(info) {
        items.push(("Re-run check".into(), Act::RerunCheck(term)));
    }
    if check::failed(info) {
        items.push(("Override check…".into(), Act::OverrideCheck(term, String::new(), None)));
    }
    if info.state == CheckState::Overridden {
        items.push(("Undo override".into(), Act::UndoOverride(term)));
    }
    items
}

/// What a re-run of `check` (None: the session has no result yet) says, or why it is not sent.
fn rerun_words(t: &TermInfo, check: Option<&CheckInfo>) -> Result<Option<String>, &'static str> {
    let working = t.status == Status::Working;
    match check {
        Some(c) if c.state == CheckState::Checking => Err("its check is already running"),
        Some(c) if c.rerun_queued => Err("a re-run is already queued: it runs when this turn ends"),
        Some(_) if working => Ok(Some("re-run queued: it runs when this turn ends".into())),
        Some(c) => Ok(Some(format!("re-running {}…", c.command))),
        // The daemon says why a working agent with no result cannot be re-run.
        None if working => Ok(None),
        None => Ok(Some("running the check…".into())),
    }
}

/// `t`'s checkpoint `name`, or its headline for None or a name it no longer has.
fn checkpoint<'a>(t: &'a TermInfo, name: Option<&str>) -> Option<&'a CheckInfo> {
    name.and_then(|n| t.checks.iter().find(|c| c.name == n)).or_else(|| t.check())
}

/// `text`'s first line added to `reason`, as far as the daemon will take it.
pub(in crate::client) fn append_reason(reason: &mut String, text: &str) {
    let room = OVERRIDE_REASON_MAX_CHARS.saturating_sub(reason.chars().count());
    reason.extend(text.lines().next().unwrap_or_default().chars().filter(|c| !c.is_control()).take(room));
}

impl App {
    pub(in crate::client) fn rerun_check(&mut self, term: TermId) {
        self.rerun_checkpoint(term, None);
    }

    /// Run `term`'s checkpoint `name` again (None: all of them).
    pub(in crate::client) fn rerun_checkpoint(&mut self, term: TermId, name: Option<String>) {
        let Some(t) = self.snap.terms.get(&term) else { return };
        match rerun_words(t, checkpoint(t, name.as_deref())) {
            Err(why) => self.notify(why.into(), false),
            Ok(said) => {
                self.cmd(Command::RerunCheck { term, name });
                if let Some(said) = said {
                    self.notify(said, false);
                }
            }
        }
    }

    pub(in crate::client) fn override_check(&mut self, term: TermId, name: Option<String>, reason: String) {
        self.cmd(Command::OverrideCheck { term, name, reason });
        let name = self.session_name(term);
        self.notify(format!("Overridden. {name} reads done (overridden); the failure stays in its evidence."), false);
    }

    pub(in crate::client) fn undo_override(&mut self, term: TermId, name: Option<String>) {
        self.cmd(Command::UndoOverride { term, name });
        self.notify("override undone: the check reads as failed again".into(), false);
    }

    /// The confirm for overriding `term`'s checkpoint `name` (None: its headline), or None (with
    /// a toast) when it is not failed.
    pub(in crate::client) fn override_confirm(&mut self, term: TermId, name: Option<String>) -> Option<Confirm> {
        let Some(info) = self.snap.terms.get(&term).and_then(|t| checkpoint(t, name.as_deref()).cloned()).filter(check::failed) else {
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
            act: Act::OverrideCheck(term, String::new(), name),
        })
    }

    pub(in crate::client) fn ask_override(&mut self, term: TermId, name: Option<String>) {
        if let Some(c) = self.override_confirm(term, name) {
            self.mode = Mode::Confirm(Box::new(c));
        }
    }

    /// One of a check's keys for `term`'s checkpoint `name` (None: its headline): `o` output,
    /// `r` re-run, `O` override, `u` undo, `,` its project's check settings. Does nothing, and
    /// says so with false, when the result does not allow it.
    pub(in crate::client) fn do_check_key(&mut self, term: TermId, name: Option<String>, key: char) -> bool {
        let Some(info) = self.snap.terms.get(&term).and_then(|t| checkpoint(t, name.as_deref()).cloned()) else { return false };
        match key {
            'o' => self.open_check_sheet(term),
            'r' if check::settled(&info) => self.rerun_checkpoint(term, name),
            'O' if check::failed(&info) => self.ask_override(term, name),
            'u' if info.state == CheckState::Overridden => self.undo_override(term, name),
            ',' => self.open_project_check(term),
            _ => return false,
        }
        true
    }

    /// Keys in the override confirm: the reason is typed into it; Enter overrides, and so does O
    /// while no reason is typed (after that O is a letter of it); Esc doesn't.
    pub(in crate::client) fn on_override_key(&mut self, mut c: Confirm, k: &KeyEvent) {
        let no_reason = matches!(&c.act, Act::OverrideCheck(_, reason, _) if reason.is_empty());
        match k.code {
            KeyCode::Char('O') if no_reason => {
                self.mode = Mode::Normal;
                return self.confirm_done(Some(c.act));
            }
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
        if let Act::OverrideCheck(_, reason, _) = &mut c.act {
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

/// The override confirm: what it will do, what it hides, where the failure stays visible, and
/// the reason.
pub(in crate::client) fn draw_override(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, c: &Confirm) {
    let Act::OverrideCheck(term, reason, name) = &c.act else { return };
    let info = app.snap.terms.get(term).and_then(|x| checkpoint(x, name.as_deref()).cloned()).unwrap_or_default();
    let mark = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).over;
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    let r = panel(app, buf, area, OVERRIDE_WIDTH, OVERRIDE_HEIGHT, OVERRIDE_TITLE, &[], t);
    let card = Style::default().bg(t.card);
    let bold = |c: Color| card.fg(c).add_modifier(Modifier::BOLD);
    let (x, right) = (r.x + 2, r.right().saturating_sub(2));
    let exit = info.exit_code.map(|code| format!("exit {code} at ")).unwrap_or_default();
    let name = if info.name.is_empty() { &info.command } else { &info.name };
    let lines: [(u16, Vec<Seg>); 6] = [
        (2, vec![seg(format!("{} will read ", c.what), card.fg(t.text)), seg("done (overridden)", bold(t.strong)), seg(" though ", card.fg(t.text)), seg(format!("{name} failed"), bold(t.err))]),
        (3, vec![seg(format!("({exit}{}).", crate::clock::local_hm(info.started_at)), card.fg(t.text))]),
        (5, vec![seg(HIDES, bold(t.strong))]),
        (6, vec![seg(format!("  {mark} {STAYS}"), card.fg(t.text))]),
        (7, vec![seg(format!("  {SCOPE}"), card.fg(t.text))]),
        (9, vec![seg("Reason (optional)", card.fg(t.muted))]),
    ];
    for (dy, segs) in &lines {
        put(buf, x, r.y + dy, segs, right);
    }
    let fy = r.y + 10;
    fill(buf, Rect { x, y: fy, width: right.saturating_sub(x), height: 1 }, t.card2);
    let field = Style::default().bg(t.card2);
    let room = right.saturating_sub(x + 2) as usize;
    let tail: String = reason.chars().skip(reason.chars().count().saturating_sub(room)).collect();
    let shown = if reason.is_empty() { seg(REASON_PLACEHOLDER, field.fg(t.muted).add_modifier(Modifier::ITALIC)) } else { seg(tail.clone(), field.fg(t.strong).add_modifier(Modifier::BOLD)) };
    put(buf, x + 1, fy, &[shown], right);
    let by = r.bottom().saturating_sub(2);
    let yes = button(t, "Override", "O", BtnKind::Risky);
    let no = button(t, "Cancel", "Esc", BtnKind::Normal);
    let yr = Rect { x, y: by, width: segs_width(&yes), height: 1 };
    let nr = Rect { x: yr.right() + 1, y: by, width: segs_width(&no), height: 1 };
    put(buf, yr.x, by, &yes, right);
    put(buf, nr.x, by, &no, right);
    hit(app, area, ScreenHit::ConfirmNo);
    hit(app, r, ScreenHit::Noop);
    hit(app, yr, ScreenHit::ConfirmYes);
    hit(app, nr, ScreenHit::ConfirmNo);
    f.set_cursor_position(Position::new(x + 1 + tail.width() as u16, fy));
}

