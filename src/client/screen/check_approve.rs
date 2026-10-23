//! Approving a command a repository ships in its `.seshi/checks.toml`. It never pops up over
//! your work: Enter on the waiting Inbox item or sidebar row opens it. It is hard to approve by
//! reflex: there is no default, Enter, y and Space do nothing, and Always takes 2 twice. You
//! approve what you read: 1 and 2 answer only once the whole command was on screen, and only
//! for that command.
use super::*;
use crate::protocol::{ApprovalChoice, CheckInfo, CheckSource, CheckState, command_hash};
use unicode_width::UnicodeWidthChar;

const WIDTH: u16 = 84;
const HEIGHT: u16 = 20;
/// Below this width the buttons use their short labels.
const SHORT_BELOW_W: u16 = 80;
/// Below this height the modal loses a row, and the blank line after the command goes.
const ROOMY_FROM_H: u16 = 30;
/// The label column of "Runs in", "From" and "When".
const LABEL_COLS: usize = 9;
const ARMED_TINT: f32 = 0.18;
const SECS_PER_HOUR: u64 = 3600;
const SECS_PER_DAY: u64 = 86_400;
const NO_REFLEX: &str = "Enter and y do nothing here. Choose 1, 2 or 3.";
const NOT_SHOWN_WHOLE: &str = "The whole command doesn't fit. Make the window bigger and read it first.";
const CHANGED_MEANWHILE: &str = "The command changed. Read it again before you choose.";
/// Rows the modal keeps below its command rows, before the buttons: a blank (when roomy),
/// Runs in, From, When, a blank, the two warnings, the armed line and a blank.
const ROWS_AFTER_COMMAND: u16 = 9;
/// Where a command's text starts in its row: an inset and the sign ("$ ").
const SIGN_COLS: u16 = 3;
/// A run of this many spaces or more is shown by its length, so padding cannot push the
/// rest of a command out of sight.
const SPACE_RUN_MARKED_FROM: usize = 2;

/// The modal for one checkpoint of one session; the next waiting one opens after a choice.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct Approve {
    pub term: TermId,
    pub name: String,
    /// 2 was pressed once: the next 2 means Always.
    pub armed: bool,
    /// Checkpoints of this session already answered here.
    pub answered: Vec<String>,
    /// `shown_hash` of what the last frame showed in full. 1 and 2 answer only while the
    /// waiting command is still that one.
    pub shown_whole: Option<String>,
}

/// A click on one of its buttons.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum ApproveHit {
    Once,
    Always,
    Never,
    NotNow,
}

/// The checkpoints of `t` waiting for your approval, in order. While its agent works or asks,
/// none: what waits is about the turn that ended.
fn waiting(t: &TermInfo) -> impl Iterator<Item = &CheckInfo> {
    t.checks.iter().filter(|c| c.state == CheckState::NeedsApproval && crate::protocol::reported_check(t.status, Some(c)).is_some())
}

/// The command as the modal shows it: line breaks, tabs and other control or invisible
/// characters spelled out, and a run of spaces shown by its length, so nothing in it hides.
pub(in crate::client) fn shown_command(command: &str) -> String {
    let mut out = String::new();
    let mut chars = command.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            ' ' => {
                let mut run = 1;
                while chars.next_if_eq(&' ').is_some() {
                    run += 1;
                }
                if run >= SPACE_RUN_MARKED_FROM {
                    out.push_str(&format!("[{run} spaces]"));
                } else {
                    out.push(' ');
                }
            }
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() || is_invisible(c) => out.push_str(&format!("\\u{{{:04x}}}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// Characters that draw as nothing, or reorder the text around them.
fn is_invisible(c: char) -> bool {
    matches!(c, '\u{00ad}' | '\u{200b}'..='\u{200f}' | '\u{202a}'..='\u{202e}' | '\u{2060}'..='\u{2069}' | '\u{feff}')
}

/// `text` cut into rows of at most `cols` cells, anywhere: every character is kept.
fn wrap_cells(text: &str, cols: usize) -> Vec<String> {
    let cols = cols.max(2);
    let mut rows = vec![String::new()];
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w > cols {
            rows.push(String::new());
            used = 0;
        }
        rows.last_mut().expect("rows starts with one").push(c);
        used += w;
    }
    rows
}

/// What 1 and 2 are tied to: the command waiting, and the one it replaces.
fn shown_hash(info: &CheckInfo) -> String {
    match &info.previous_command {
        Some(old) => command_hash(&format!("{}\0{old}", info.command)),
        None => command_hash(&info.command),
    }
}

fn project_name(t: &TermInfo) -> String {
    folder_name(t.root.as_deref().unwrap_or(&t.cwd))
}

/// "3 days ago", "2h ago".
fn ago_words(at: u64) -> String {
    let secs = now_secs().saturating_sub(at);
    match secs {
        s if s < 60 => "just now".into(),
        s if s < SECS_PER_HOUR => format!("{} min ago", s / 60),
        s if s < SECS_PER_DAY => format!("{}h ago", s / SECS_PER_HOUR),
        s if s < 2 * SECS_PER_DAY => "1 day ago".into(),
        s => format!("{} days ago", s / SECS_PER_DAY),
    }
}

/// Where the command comes from, after the file's name.
fn origin(info: &CheckInfo) -> String {
    let CheckSource::Repo { commit, committed_at, uncommitted, .. } = &info.source else { return String::new() };
    let verb = if info.previous_command.is_some() { "changed" } else { "added" };
    match (commit, uncommitted) {
        (_, true) => " · changed, not committed".into(),
        (Some(sha), false) => match committed_at {
            Some(at) => format!(" · {verb} in {sha}, {}", ago_words(*at)),
            None => format!(" · {verb} in {sha}"),
        },
        (None, false) => String::new(),
    }
}

impl App {
    pub(in crate::client) fn approval_waits(&self, term: TermId) -> bool {
        self.snap.terms.get(&term).is_some_and(|t| waiting(t).next().is_some())
    }

    /// Enter on a session's Inbox item or sidebar row: ask about its first checkpoint waiting
    /// for your approval, if one is.
    pub(in crate::client) fn open_approval(&mut self, term: TermId) {
        if let Some(name) = self.snap.terms.get(&term).and_then(|t| waiting(t).next()).map(|c| c.name.clone()) {
            self.mode = Mode::Approve(Box::new(Approve { term, name, armed: false, answered: Vec::new(), shown_whole: None }));
        }
    }

    pub(in crate::client) fn on_approve_key(&mut self, mut a: Approve, k: &KeyEvent) {
        let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        self.mode = Mode::Approve(Box::new(a.clone()));
        if plain
            && matches!(k.code, KeyCode::Char('1' | '2'))
            && let Some(why) = self.not_readable(&a)
        {
            a.armed = false;
            self.mode = Mode::Approve(Box::new(a));
            self.notify(why.into(), false);
            return;
        }
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Char('1') if plain => self.approve(a, ApprovalChoice::Once),
            KeyCode::Char('2') if plain && a.armed => self.approve(a, ApprovalChoice::Always),
            KeyCode::Char('2') if plain => {
                a.armed = true;
                self.mode = Mode::Approve(Box::new(a));
            }
            KeyCode::Char('3') if plain => self.approve(a, ApprovalChoice::Never),
            KeyCode::Enter | KeyCode::Char('y' | ' ') => self.notify(NO_REFLEX.into(), false),
            _ => {}
        }
    }

    pub(in crate::client) fn on_approve_hit(&mut self, h: ApproveHit) {
        let code = match h {
            ApproveHit::Once => KeyCode::Char('1'),
            ApproveHit::Always => KeyCode::Char('2'),
            ApproveHit::Never => KeyCode::Char('3'),
            ApproveHit::NotNow => KeyCode::Esc,
        };
        self.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    /// Why 1 and 2 can't answer yet: the waiting command isn't what the last frame showed
    /// whole. None when they can.
    fn not_readable(&self, a: &Approve) -> Option<&'static str> {
        let info = self.snap.terms.get(&a.term).and_then(|t| t.checks.iter().find(|c| c.name == a.name))?;
        match &a.shown_whole {
            Some(hash) if *hash == shown_hash(info) => None,
            Some(_) => Some(CHANGED_MEANWHILE),
            None => Some(NOT_SHOWN_WHOLE),
        }
    }

    fn approve(&mut self, mut a: Approve, choice: ApprovalChoice) {
        let Some(term) = self.snap.terms.get(&a.term) else { return };
        let project = project_name(term);
        let command = term.checks.iter().find(|c| c.name == a.name).map(|c| c.command.clone()).unwrap_or_default();
        self.cmd(Command::ApproveCheck { term: a.term, name: a.name.clone(), choice, command_sha256: command_hash(&command) });
        self.mode = Mode::Normal;
        let said = match choice {
            ApprovalChoice::Once => "Running once. Asks again next turn.".to_string(),
            ApprovalChoice::Always => format!("Always for {project}. Asks again if the command changes."),
            ApprovalChoice::Never => {
                self.open_check_sheet(a.term);
                "Never: this repo's check will not run. Change it in Settings › Check.".to_string()
            }
        };
        self.notify(said, false);
        a.answered.push(std::mem::take(&mut a.name));
        let next = self.snap.terms.get(&a.term).and_then(|t| waiting(t).find(|c| !a.answered.iter().any(|n| n.eq_ignore_ascii_case(&c.name)))).map(|c| c.name.clone());
        if let Some(name) = next {
            self.mode = Mode::Approve(Box::new(Approve { name, armed: false, shown_whole: None, ..a }));
        }
    }

    /// Record what this frame showed of `a`'s command; a different command disarms 2.
    fn mark_shown(&mut self, a: &Approve, hash: String, whole: bool) {
        if let Mode::Approve(m) = &mut self.mode
            && m.term == a.term
            && m.name == a.name
        {
            if m.shown_whole.as_ref().is_some_and(|h| *h != hash) {
                m.armed = false;
            }
            m.shown_whole = whole.then_some(hash);
        }
    }
}

/// The modal: what would run (in full, wrapped), where, from which file and commit, and
/// when; then the choices. It grows to fit a long command, as far as the screen allows.
pub(in crate::client) fn draw_approve(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, a: &Approve) {
    let Some(term) = app.snap.terms.get(&a.term).cloned() else { return };
    let Some(info) = term.checks.iter().find(|c| c.name == a.name).cloned() else { return };
    dim_all(buf, area, t);
    let changed = info.previous_command.is_some();
    let w = WIDTH.min(area.width.saturating_sub(2));
    let roomy = area.height >= ROOMY_FROM_H;
    let cols = usize::from(w.saturating_sub(4 + SIGN_COLS));
    let commands: Vec<(&str, Color, Vec<String>)> = match &info.previous_command {
        Some(old) => vec![("-", t.err, wrap_cells(&shown_command(old), cols)), ("+", t.done, wrap_cells(&shown_command(&info.command), cols))],
        None => vec![("$", t.muted, wrap_cells(&shown_command(&info.command), cols))],
    };
    let total = commands.iter().map(|(.., rows)| rows.len()).sum::<usize>();
    let extra = u16::try_from(total - commands.len()).unwrap_or(u16::MAX);
    let h = HEIGHT.saturating_add(extra).min(area.height.saturating_sub(1)).saturating_sub(u16::from(!roomy));
    let title = if changed { "THIS REPO CHANGED ITS CHECK" } else { "A REPO WANTS TO RUN A COMMAND" };
    let r = panel(app, buf, area, w, h, title, &[], t);
    let (x, right) = (r.x + 2, r.right().saturating_sub(2));
    let card = Style::default().bg(t.card);
    let bold = |c: Color| card.fg(c).add_modifier(Modifier::BOLD);
    let project = project_name(&term);
    let mut y = r.y + 2;
    let headline = if changed { "The command changed since you said always:".to_string() } else { format!("{project} ships its own check. It has never run here.") };
    put(buf, x, y, &[seg(headline, bold(t.strong))], right);
    y += 2;
    let by = r.bottom().saturating_sub(2);
    let room = usize::from(by.saturating_sub(y + ROWS_AFTER_COMMAND - u16::from(!roomy)));
    let whole = total <= room;
    let mut left = if whole { total } else { room };
    let ground = Style::default().bg(t.card2).add_modifier(Modifier::BOLD);
    for (sign, sign_color, rows) in &commands {
        for (i, row) in rows.iter().enumerate() {
            if left == 0 {
                break;
            }
            left -= 1;
            fill(buf, Rect { x, y, width: right.saturating_sub(x), height: 1 }, t.card2);
            let lead = if i == 0 { format!("{sign} ") } else { "  ".to_string() };
            put(buf, x + 1, y, &[seg(lead, ground.fg(*sign_color)), seg(row.as_str(), ground.fg(t.strong))], right);
            if left == 0 && !whole {
                put(buf, right.saturating_sub(1), y, &[seg("…", ground.fg(t.blocked))], right);
            }
            y += 1;
        }
    }
    if roomy {
        y += 1;
    }
    let file = match &info.source {
        CheckSource::Repo { file, .. } => file.clone(),
        CheckSource::User => String::new(),
    };
    let dir = if info.cwd.as_os_str().is_empty() { term.top.clone().unwrap_or(term.cwd.clone()) } else { info.cwd.clone() };
    let key = |k: &str| seg(format!("{k:<LABEL_COLS$}"), card.fg(t.muted));
    put(buf, x, y, &[key("Runs in"), seg(tilde(&dir), card.fg(t.strong))], right);
    put(buf, x, y + 1, &[key("From"), seg(file, card.fg(t.strong)), seg(origin(&info), card.fg(t.text))], right);
    put(buf, x, y + 2, &[key("When"), seg("after every agent turn, with your permissions", card.fg(t.text))], right);
    y += 4;
    put(buf, x, y, &[seg("Read it. It can do anything you can in that folder.", bold(t.blocked))], right);
    let hint = if whole { "Enter and y do nothing here. Choose with 1, 2 or 3." } else { NOT_SHOWN_WHOLE };
    put(buf, x, y + 1, &[seg(hint, card.fg(t.muted))], right);
    if a.armed {
        let tint = crate::theme::mix(t.card, t.blocked, ARMED_TINT);
        fill(buf, Rect { x, y: y + 2, width: right.saturating_sub(x), height: 1 }, tint);
        let st = Style::default().bg(tint).fg(t.strong);
        put(buf, x + 1, y + 2, &[seg("Press 2 again", st.add_modifier(Modifier::BOLD)), seg(format!(" to always run this exact command for {project}."), st)], right);
    }
    let short = w < SHORT_BELOW_W;
    let mut bx = btn(app, buf, x, by, "Run once", "1", BtnKind::Normal, ScreenHit::Approve(ApproveHit::Once), right) + 1;
    let (always, always_key, always_kind) = (if short { "Always" } else { "Always for this repo" }, if a.armed { "2 again" } else { "2" }, if a.armed { BtnKind::Primary } else { BtnKind::Normal });
    bx = btn(app, buf, bx, by, always, always_key, always_kind, ScreenHit::Approve(ApproveHit::Always), right) + 1;
    bx = btn(app, buf, bx, by, if short { "Never" } else { "Never for this repo" }, "3", BtnKind::Danger, ScreenHit::Approve(ApproveHit::Never), right) + 1;
    btn(app, buf, bx, by, "Not now", "Esc", BtnKind::Normal, ScreenHit::Approve(ApproveHit::NotNow), right);
    app.mark_shown(a, shown_hash(&info), whole);
}
