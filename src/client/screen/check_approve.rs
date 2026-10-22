//! Approving a command a repository ships in its `.seshi/checks.toml`. It never pops up over
//! your work: Enter on the waiting Inbox item or sidebar row opens it. It is hard to approve by
//! reflex: there is no default, Enter, y and Space do nothing, and Always takes 2 twice.
use super::*;
use crate::protocol::{ApprovalChoice, CheckInfo, CheckSource, CheckState};

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

/// The modal for one checkpoint of one session; the next waiting one opens after a choice.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct Approve {
    pub term: TermId,
    pub name: String,
    /// 2 was pressed once: the next 2 means Always.
    pub armed: bool,
    /// Checkpoints of this session already answered here.
    pub answered: Vec<String>,
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
            self.mode = Mode::Approve(Box::new(Approve { term, name, armed: false, answered: Vec::new() }));
        }
    }

    pub(in crate::client) fn on_approve_key(&mut self, mut a: Approve, k: &KeyEvent) {
        let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        self.mode = Mode::Approve(Box::new(a.clone()));
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

    fn approve(&mut self, mut a: Approve, choice: ApprovalChoice) {
        let project = self.snap.terms.get(&a.term).map(project_name).unwrap_or_default();
        self.cmd(Command::ApproveCheck { term: a.term, name: a.name.clone(), choice });
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
            self.mode = Mode::Approve(Box::new(Approve { name, armed: false, ..a }));
        }
    }
}

/// The modal: what would run, where, from which file and commit, and when; then the choices.
pub(in crate::client) fn draw_approve(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, a: &Approve) {
    let Some(term) = app.snap.terms.get(&a.term).cloned() else { return };
    let Some(info) = term.checks.iter().find(|c| c.name == a.name).cloned() else { return };
    dim_all(buf, area, t);
    let changed = info.previous_command.is_some();
    let w = WIDTH.min(area.width.saturating_sub(2));
    let roomy = area.height >= ROOMY_FROM_H;
    let h = HEIGHT.min(area.height.saturating_sub(1)) - u16::from(!roomy);
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
    let mut command_row = |buf: &mut Buffer, sign: &str, sign_color: Color, command: &str| {
        fill(buf, Rect { x, y, width: right.saturating_sub(x), height: 1 }, t.card2);
        let ground = Style::default().bg(t.card2).add_modifier(Modifier::BOLD);
        put(buf, x + 1, y, &[seg(format!("{sign} "), ground.fg(sign_color)), seg(command, ground.fg(t.strong))], right);
        y += 1;
    };
    match &info.previous_command {
        Some(old) => {
            command_row(buf, "-", t.err, old);
            command_row(buf, "+", t.done, &info.command);
        }
        None => command_row(buf, "$", t.muted, &info.command),
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
    put(buf, x, y + 1, &[seg("Enter and y do nothing here. Choose with 1, 2 or 3.", card.fg(t.muted))], right);
    if a.armed {
        let tint = crate::theme::mix(t.card, t.blocked, ARMED_TINT);
        fill(buf, Rect { x, y: y + 2, width: right.saturating_sub(x), height: 1 }, tint);
        let st = Style::default().bg(tint).fg(t.strong);
        put(buf, x + 1, y + 2, &[seg("Press 2 again", st.add_modifier(Modifier::BOLD)), seg(format!(" to always run this exact command for {project}."), st)], right);
    }
    let short = w < SHORT_BELOW_W;
    let by = r.bottom().saturating_sub(2);
    let mut bx = btn(app, buf, x, by, "Run once", "1", BtnKind::Normal, ScreenHit::Approve(ApproveHit::Once), right) + 1;
    let (always, always_key, always_kind) = (if short { "Always" } else { "Always for this repo" }, if a.armed { "2 again" } else { "2" }, if a.armed { BtnKind::Primary } else { BtnKind::Normal });
    bx = btn(app, buf, bx, by, always, always_key, always_kind, ScreenHit::Approve(ApproveHit::Always), right) + 1;
    bx = btn(app, buf, bx, by, if short { "Never" } else { "Never for this repo" }, "3", BtnKind::Danger, ScreenHit::Approve(ApproveHit::Never), right) + 1;
    btn(app, buf, bx, by, "Not now", "Esc", BtnKind::Normal, ScreenHit::Approve(ApproveHit::NotNow), right);
}
