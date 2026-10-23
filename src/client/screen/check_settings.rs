//! Settings › Check: the checkpoints Seshi runs when an agent's turn ends in a project, each with
//! its command, timeout, latest result and where it comes from. One renderer and one set of keys,
//! two hosts: the `PROJECT · <name>` modal (`,` from the evidence sheet or the Inbox) and the
//! Check page of Settings. Your checkpoints are `[[check]]` tables in config.toml; a repo's own
//! come from its `.seshi/checks.toml` and are read-only here.
use super::check;
use super::*;
use crate::client::{Bg, modal};
use crate::config::{CHECK_TIMEOUT_S_DEFAULT, Config, same_path};
use crate::protocol::{CheckInfo, CheckSource, CheckState};
use anyhow::{Context, Result};
use toml_edit::{ArrayOfTables, Item, Table};

/// Round values the timeout steps through, in seconds.
const TIMEOUT_STEPS_S: [u64; 8] = [30, 60, 120, 300, 600, 900, 1800, 3600];
const SECS_PER_MIN: u64 = 60;
const MODAL_W: u16 = 80;
const MODAL_H: u16 = 20;
/// The label column of a row ("Command", "Timeout", …).
const LABEL_COLS: usize = 10;
const EXIT_NOT_EXECUTABLE: i32 = 126;
const EXIT_NOT_FOUND: i32 = 127;
const COMMAND_PLACEHOLDER: &str = "e.g. npm test";
const NO_CHECK: &str = "No check set.";
/// The empty state's words as the handoff breaks them; a narrower host wraps them.
const EMPTY_LINES: [&str; 2] = [
    "No check set. Seshi can run one command itself when an agent's",
    "turn ends; the session isn't done until it passes. Agents keep working.",
];
const NOT_A_LIST: &str = "`check` in config.toml is not a list of [[check]] tables";

/// The row the arrows are on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum DraftRow {
    Command,
    Timeout,
    Points,
}

/// What you have changed and not saved yet, in either host.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct CheckDraft {
    /// The repository's main checkout.
    pub project: PathBuf,
    /// Which checkpoint the rows show.
    pub point: usize,
    pub row: DraftRow,
    /// The command as typed, once you start typing.
    pub command: Option<String>,
    pub editing: bool,
    pub timeout_s: Option<u64>,
    pub saving: bool,
}

impl CheckDraft {
    pub fn new(project: PathBuf) -> CheckDraft {
        CheckDraft { project, point: 0, row: DraftRow::Command, command: None, editing: false, timeout_s: None, saving: false }
    }
}

/// A click in either host.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum DraftHit {
    Save,
    RunNow,
    Remove,
    Cancel,
    Command,
    /// ‹ (false) or › (true) beside the timeout.
    Timeout(bool),
    /// ‹ or › beside CHECKPOINTS.
    Point(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Host {
    Modal,
    Page,
}

/// One checkpoint of the project.
#[derive(Debug, Clone)]
struct Point {
    name: String,
    command: String,
    timeout_s: u64,
    source: CheckSource,
    /// Its latest result, and the session it ran in.
    result: Option<(CheckInfo, String)>,
}

impl Point {
    /// It comes from the repo's file, so only the repo changes it.
    fn repos_own(&self) -> bool {
        matches!(self.source, CheckSource::Repo { .. })
    }
}

/// The timeout one step longer (`dir` > 0) or shorter, staying at the ends. A value written by
/// hand that is not a step moves to the nearest step in that direction.
pub(in crate::client) fn step_timeout(secs: u64, dir: i64) -> u64 {
    let last = TIMEOUT_STEPS_S[TIMEOUT_STEPS_S.len() - 1];
    if dir > 0 {
        TIMEOUT_STEPS_S.iter().copied().find(|s| *s > secs).unwrap_or(last)
    } else {
        TIMEOUT_STEPS_S.iter().rev().copied().find(|s| *s < secs).unwrap_or(TIMEOUT_STEPS_S[0])
    }
}

fn timeout_label(secs: u64) -> String {
    if secs >= SECS_PER_MIN && secs.is_multiple_of(SECS_PER_MIN) { format!("{} min", secs / SECS_PER_MIN) } else { format!("{secs} s") }
}

// ---- config.toml ---------------------------------------------------------------------------

/// A change to one of the project's `[[check]]` tables in config.toml.
#[derive(Debug, Clone)]
pub(in crate::client) enum CheckEdit {
    /// The checkpoint `name` (a new one when None or not found) runs `command`.
    Set { project: PathBuf, name: Option<String>, command: String, timeout_s: u64 },
    Remove { project: PathBuf, name: String },
}

impl CheckEdit {
    pub fn write(&self) -> Result<()> {
        modal::edit_config(|doc| self.apply(doc))
    }

    pub fn apply(&self, doc: &mut toml_edit::DocumentMut) -> Result<()> {
        match self {
            CheckEdit::Set { project, name, command, timeout_s } => {
                let list = doc.entry("check").or_insert(Item::ArrayOfTables(ArrayOfTables::new())).as_array_of_tables_mut().context(NOT_A_LIST)?;
                let timeout = i64::try_from(*timeout_s).unwrap_or(i64::MAX);
                let at = name.as_deref().and_then(|n| list.iter().position(|t| is_for(t, project, n)));
                match at.and_then(|i| list.get_mut(i)) {
                    Some(table) => {
                        // Without a `name` it is named by its command: keep the name it had.
                        if let Some(n) = name.as_deref().filter(|n| !table.contains_key("name") && !named_by(command, n)) {
                            table.insert("name", toml_edit::value(n));
                        }
                        set_keeping_decor(table, "command", command.as_str().into());
                        set_keeping_decor(table, "timeout_s", timeout.into());
                    }
                    None => {
                        let mut table = Table::new();
                        table.insert("project", toml_edit::value(project.to_string_lossy().as_ref()));
                        if let Some(n) = name.as_deref().filter(|n| !named_by(command, n)) {
                            table.insert("name", toml_edit::value(n));
                        }
                        table.insert("command", toml_edit::value(command.as_str()));
                        table.insert("timeout_s", toml_edit::value(timeout));
                        list.push(table);
                    }
                }
            }
            CheckEdit::Remove { project, name } => {
                let Some(item) = doc.get_mut("check") else { return Ok(()) };
                let list = item.as_array_of_tables_mut().context(NOT_A_LIST)?;
                let at = list.iter().position(|t| is_for(t, project, name));
                if let Some(i) = at {
                    list.remove(i);
                }
                if list.is_empty() {
                    doc.remove("check");
                }
            }
        }
        Ok(())
    }
}

/// A command with no `name` beside it is called by its last word (`CheckDef::name`).
fn named_by(command: &str, name: &str) -> bool {
    command.split_whitespace().last().is_some_and(|w| w.eq_ignore_ascii_case(name))
}

/// Whether a `[[check]]` table names `project`.
pub(super) fn is_project(table: &Table, project: &Path) -> bool {
    table.get("project").and_then(Item::as_str).is_some_and(|p| same_path(Path::new(p), project))
}

fn is_for(table: &Table, project: &Path, name: &str) -> bool {
    let own = is_project(table, project);
    let named = match table.get("name").and_then(Item::as_str) {
        Some(n) => n.eq_ignore_ascii_case(name),
        None => named_by(table.get("command").and_then(Item::as_str).unwrap_or_default(), name),
    };
    own && named
}

/// Replace a value but keep the comments next to it.
pub(super) fn set_keeping_decor(table: &mut Table, key: &str, mut value: toml_edit::Value) {
    match table.get_mut(key).and_then(Item::as_value_mut) {
        Some(old) => {
            *value.decor_mut() = old.decor().clone();
            *old = value;
        }
        None => {
            table.insert(key, toml_edit::value(value));
        }
    }
}

// ---- the project's checkpoints -------------------------------------------------------------

/// The project Settings edits: the repository of the workspace you are in.
pub(super) fn project_of(app: &App) -> Option<PathBuf> {
    app.active_ws().map(|w| w.git.as_ref().map_or_else(|| w.cwd.clone(), |g| g.root.clone()))
}

fn project_of_term(t: &TermInfo) -> PathBuf {
    t.root.clone().unwrap_or_else(|| t.cwd.clone())
}

fn in_project(t: &TermInfo, project: &Path) -> bool {
    same_path(&project_of_term(t), project)
}

/// Yours from config first, in its order, then the repo's that sessions have reported.
fn points(app: &App, project: &Path) -> Vec<Point> {
    let mut out: Vec<Point> = app
        .cfg
        .check
        .iter()
        .filter(|d| same_path(&d.project, project))
        .map(|d| Point { name: d.name().to_string(), command: d.command.clone(), timeout_s: d.timeout_s, source: CheckSource::User, result: None })
        .collect();
    let terms = || app.snap.terms.values().filter(|t| in_project(t, project));
    for c in terms().flat_map(|t| &t.checks).filter(|c| matches!(c.source, CheckSource::Repo { .. })) {
        if !out.iter().any(|p| p.name.eq_ignore_ascii_case(&c.name)) {
            out.push(Point { name: c.name.clone(), command: c.command.clone(), timeout_s: CHECK_TIMEOUT_S_DEFAULT, source: c.source.clone(), result: None });
        }
    }
    for p in &mut out {
        p.result = terms()
            .flat_map(|t| t.checks.iter().map(move |c| (c, t)))
            .filter(|(c, _)| c.name.eq_ignore_ascii_case(&p.name) || (c.name.is_empty() && c.command == p.command))
            .max_by_key(|(c, _)| c.started_at)
            .map(|(c, t)| (c.clone(), app.session_name(t.id)));
    }
    out
}

fn selected(points: &[Point], d: &CheckDraft) -> Option<Point> {
    points.get(d.point.min(points.len().saturating_sub(1))).cloned()
}

/// The rows the arrows walk, top to bottom.
fn rows(points: &[Point]) -> Vec<DraftRow> {
    match points.len() {
        0 => vec![DraftRow::Command],
        1 => vec![DraftRow::Command, DraftRow::Timeout],
        _ => vec![DraftRow::Command, DraftRow::Timeout, DraftRow::Points],
    }
}

/// The session "Run now" runs the check in: the one with the latest result, else the focused
/// agent of the project, else its first agent.
fn run_target(app: &App, project: &Path) -> Option<TermId> {
    let agent = |t: &&TermInfo| t.agent.is_some() && in_project(t, project);
    app.snap
        .terms
        .values()
        .filter(|t| in_project(t, project) && !t.checks.is_empty())
        .max_by_key(|t| t.checks.iter().map(|c| c.started_at).max())
        .map(|t| t.id)
        .or_else(|| app.focused().and_then(|f| app.snap.terms.get(&f)).filter(agent).map(|t| t.id))
        .or_else(|| app.snap.terms.values().filter(agent).map(|t| t.id).min())
}

/// What you chose for a repo's checkpoint, as far as its last state tells.
// shortcut: the snapshot does not carry the choice, so a result after Run once reads as always
// until the next turn asks again.
fn decision(p: &Point) -> &'static str {
    match p.result.as_ref().map(|(c, _)| c.state) {
        Some(CheckState::Never) => "never",
        None | Some(CheckState::NeedsApproval) => "asks each time",
        Some(_) => "always",
    }
}

// ---- drawing -------------------------------------------------------------------------------

fn label(text: &str, on: bool, t: &Theme) -> Seg {
    let style = if on { Style::default().fg(t.accent).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.muted) };
    seg(format!("{text:<LABEL_COLS$}"), style.bg(t.card))
}

/// What the latest run came to, and a second line when there is something to do about it.
fn status(app: &App, t: &Theme, p: &Point) -> (Vec<Seg>, Option<&'static str>) {
    let m = glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons);
    let bold = |c: Color| Style::default().fg(c).bg(t.card).add_modifier(Modifier::BOLD);
    let Some((info, on)) = &p.result else {
        return (vec![seg(format!("{} never run", m.pend), bold(t.strong)), seg(" · runs after the next turn ends", Style::default().fg(t.muted).bg(t.card))], None);
    };
    let at = crate::clock::local_hm(info.started_at);
    let first_word = info.command.split_whitespace().next().unwrap_or_default();
    match info.state {
        CheckState::Passed => (vec![seg(format!("{} passed on {on} {at} · {}", m.done, check::duration(info.duration_ms)), bold(t.done))], None),
        CheckState::Failed => {
            let code = info.exit_code.map(|c| format!(" · exit {c}")).unwrap_or_default();
            (vec![seg(format!("{} failed on {on} {at}{code}", m.failed), bold(t.err))], None)
        }
        CheckState::CouldNotRun => {
            let why = match info.exit_code {
                Some(EXIT_NOT_FOUND) => format!("{first_word}: not found (exit {EXIT_NOT_FOUND})"),
                Some(EXIT_NOT_EXECUTABLE) => format!("{first_word}: not executable (exit {EXIT_NOT_EXECUTABLE})"),
                Some(code) => format!("exit {code}"),
                None => "it did not finish".into(),
            };
            let fix = (info.exit_code == Some(EXIT_NOT_FOUND)).then_some("Not on the worktree's PATH. Install it or change the command.");
            (vec![seg(format!("{} could not run on {on} {at} · {why}", m.couldnt), bold(t.violet()))], fix)
        }
        CheckState::Checking => (vec![seg(format!("{} checking on {on} since {at}", check::glyph(app, info)), bold(t.sky()))], None),
        CheckState::Overridden => (vec![seg(format!("{} failed, overridden on {on} {at}", m.over), bold(t.text))], None),
        CheckState::Stale => (vec![seg(format!("{} stale pass on {on} {at} · {} files changed", m.stale, info.changed_files.len()), bold(t.text))], None),
        CheckState::NeedsApproval => (vec![seg(format!("{} waiting for your approval on {on}", m.needs), bold(t.blocked))], None),
        CheckState::Never => (vec![seg(format!("{} not checked: never for this repo", m.couldnt), bold(t.violet()))], None),
        CheckState::Pending => (vec![seg(format!("{} next on {on}", m.pend), bold(t.muted))], None),
    }
}

fn source_words(p: &Point) -> String {
    match &p.source {
        CheckSource::User => "you, in settings · no approval needed".into(),
        CheckSource::Repo { file, commit, .. } => format!("{file} · {} · {}", commit.as_deref().unwrap_or("not committed"), decision(p)),
    }
}

/// A one-row input from `x` to `right`: the value, or the placeholder in italics.
fn field(buf: &mut Buffer, x: u16, y: u16, right: u16, t: &Theme, value: &str, editing: bool) {
    let ground = Style::default().bg(t.card2);
    for xx in x..right.min(buf.area.right()) {
        if let Some(c) = buf.cell_mut((xx, y)) {
            c.set_symbol(" ").set_style(ground);
        }
    }
    let mut segs = if value.is_empty() && !editing {
        vec![seg(COMMAND_PLACEHOLDER, ground.fg(t.muted).add_modifier(Modifier::ITALIC))]
    } else {
        let keep = right.saturating_sub(x + 3) as usize;
        let tail: String = value.chars().skip(value.chars().count().saturating_sub(keep)).collect();
        vec![seg(tail, ground.fg(t.strong).add_modifier(Modifier::BOLD))]
    };
    if editing {
        segs.push(seg("█", ground.fg(t.accent)));
    }
    put(buf, x + 1, y, &segs, right);
}

/// ‹ › for stepping a row, clickable; returns the x after them.
fn arrows(app: &mut App, buf: &mut Buffer, x: u16, y: u16, on: bool, t: &Theme, hit_of: fn(bool) -> DraftHit) -> u16 {
    let st = Style::default().bg(t.card).fg(if on { t.accent } else { t.muted });
    put(buf, x, y, &[seg("‹ ›", if on { st.add_modifier(Modifier::BOLD) } else { st })], x + 3);
    hit(app, Rect { x, y, width: 1, height: 1 }, ScreenHit::CheckDraft(hit_of(false)));
    hit(app, Rect { x: x + 2, y, width: 1, height: 1 }, ScreenHit::CheckDraft(hit_of(true)));
    x + 3
}

/// The content of either host from (`x`, `y`) to `right`, buttons on `buttons_y`.
fn draw_content(app: &mut App, buf: &mut Buffer, (x, y, right, buttons_y): (u16, u16, u16, u16), t: &Theme, d: &CheckDraft) {
    let card = Style::default().bg(t.card);
    let text = card.fg(t.text);
    let all = points(app, &d.project);
    let point = selected(&all, d);
    let mut y = y;
    put(buf, x, y, &[seg(" CHECK ", Style::default().bg(t.btn).fg(t.strong).add_modifier(Modifier::BOLD))], right);
    y += 2;
    let typed = d.command.clone();
    let on = |row: DraftRow| d.row == row;
    let command_at = |app: &mut App, buf: &mut Buffer, y: u16, value: &str| {
        put(buf, x, y, &[label("Command", on(DraftRow::Command), t)], right);
        let fx = x + LABEL_COLS as u16;
        field(buf, fx, y, right, t, value, d.editing);
        hit(app, Rect { x: fx, y, width: right.saturating_sub(fx), height: 1 }, ScreenHit::CheckDraft(DraftHit::Command));
    };
    let Some(p) = point else {
        let width = right.saturating_sub(x) as usize;
        let mut lines = EMPTY_LINES.map(String::from).to_vec();
        if lines.iter().any(|l| l.width() > width) {
            lines = super::popover::wrap(&EMPTY_LINES.join(" "), width);
        }
        for (i, line) in lines.iter().enumerate() {
            let segs = match line.strip_prefix(NO_CHECK) {
                Some(rest) if i == 0 => vec![seg(NO_CHECK, card.fg(t.strong).add_modifier(Modifier::BOLD)), seg(rest, text)],
                _ => vec![seg(line.as_str(), text)],
            };
            put(buf, x, y + i as u16, &segs, right);
        }
        let y = y + lines.len() as u16 + 1;
        command_at(app, buf, y, typed.as_deref().unwrap_or_default());
        put(buf, x, y + 2, &[seg("Projects without a check look exactly as they do today.", card.fg(t.muted))], right);
        return draw_buttons(app, buf, x, buttons_y, right, None);
    };
    command_at(app, buf, y, typed.as_deref().unwrap_or(&p.command));
    y += 2;
    put(buf, x, y, &[label("Runs", false, t), seg("after each turn ends · in the session's worktree", text)], right);
    y += 1;
    let tx = put(buf, x, y, &[label("Timeout", on(DraftRow::Timeout), t), seg(timeout_label(d.timeout_s.unwrap_or(p.timeout_s)), text), seg("  ", card)], right);
    arrows(app, buf, tx, y, on(DraftRow::Timeout), t, DraftHit::Timeout);
    y += 1;
    let (words, fix) = status(app, t, &p);
    put(buf, x, y, &[vec![label("Status", false, t)], words].concat(), right);
    if let Some(fix) = fix {
        y += 1;
        put(buf, x, y, &[label("", false, t), seg(fix, card.fg(t.muted))], right);
    }
    y += 1;
    put(buf, x, y, &[label("Source", false, t), seg(source_words(&p), text)], right);
    y += 2;
    let head = Style::default().bg(t.card).fg(if on(DraftRow::Points) { t.accent } else { t.muted }).add_modifier(Modifier::BOLD);
    let n = all.len();
    let at = d.point.min(n - 1) + 1;
    if n > 1 {
        let ax = put(buf, x, y, &[seg("CHECKPOINTS  ", head), seg(format!("{at} of {n} · "), card.fg(t.muted))], right);
        let ex = arrows(app, buf, ax, y, on(DraftRow::Points), t, DraftHit::Point);
        put(buf, ex, y, &[seg(" switch", card.fg(t.muted))], right);
    } else {
        put(buf, x, y, &[seg("CHECKPOINTS  ", head), seg("1 of 1 · room for more kinds later", card.fg(t.muted))], right);
    }
    draw_buttons(app, buf, x, buttons_y, right, Some(&p));
}

fn draw_buttons(app: &mut App, buf: &mut Buffer, x: u16, y: u16, right: u16, p: Option<&Point>) {
    let mut bx = btn(app, buf, x, y, "Save", "Ctrl+S", BtnKind::Primary, ScreenHit::CheckDraft(DraftHit::Save), right) + 1;
    if let Some(p) = p {
        bx = btn(app, buf, bx, y, "Run now", "r", BtnKind::Normal, ScreenHit::CheckDraft(DraftHit::RunNow), right) + 1;
        let remove = if p.repos_own() { "Forget choice" } else { "Remove" };
        bx = btn(app, buf, bx, y, remove, "Del", BtnKind::Risky, ScreenHit::CheckDraft(DraftHit::Remove), right) + 1;
    }
    btn(app, buf, bx, y, "Cancel", "Esc", BtnKind::Normal, ScreenHit::CheckDraft(DraftHit::Cancel), right);
}

/// The `PROJECT · <name>` modal.
pub(in crate::client) fn draw_project_check(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, d: &CheckDraft) {
    dim_all(buf, area, t);
    let w = MODAL_W.min(area.width.saturating_sub(2));
    let h = MODAL_H.min(area.height.saturating_sub(2));
    let r = panel(app, buf, area, w, h, &format!("PROJECT · {}", folder_name(&d.project)), &[], t);
    draw_content(app, buf, (r.x + 2, r.y + 2, r.right().saturating_sub(2), r.bottom().saturating_sub(2)), t, d);
}

/// The Check page of Settings, inside `r` (the Settings card) from row `top`.
pub(in crate::client) fn draw_page(app: &mut App, buf: &mut Buffer, r: Rect, top: u16, t: &Theme) {
    let Some(project) = project_of(app) else {
        put(buf, r.x + 4, top, &[seg("Open a session in a project to set its check.", Style::default().bg(t.card).fg(t.muted))], r.right());
        return;
    };
    let d = app.screen.check_page.clone().filter(|d| same_path(&d.project, &project)).unwrap_or_else(|| CheckDraft::new(project));
    draw_content(app, buf, (r.x + 4, top, r.right().saturating_sub(3), r.bottom().saturating_sub(2)), t, &d);
}

// ---- keys ----------------------------------------------------------------------------------

enum Outcome {
    Stay,
    Close,
}

impl App {
    /// The `PROJECT · <name>` modal for `term`'s project.
    pub(in crate::client) fn open_project_check(&mut self, term: TermId) {
        let Some(t) = self.snap.terms.get(&term) else { return };
        self.mode = Mode::ProjectCheck(Box::new(CheckDraft::new(project_of_term(t))));
    }

    pub(in crate::client) fn on_project_check_key(&mut self, mut d: CheckDraft, k: &KeyEvent) {
        self.mode = Mode::Normal;
        match self.draft_key(&mut d, k) {
            Outcome::Close => {}
            Outcome::Stay if matches!(self.mode, Mode::Normal) => self.mode = Mode::ProjectCheck(Box::new(d)),
            Outcome::Stay => {}
        }
    }

    /// Keys on the Check page of Settings. False leaves the key to Settings (Tab, Esc, o).
    pub(in crate::client) fn on_check_page_key(&mut self, k: &KeyEvent) -> bool {
        let Mode::ScreenSettings(v) = &self.mode else { return false };
        if modal::Cat::ALL.get(v.cat) != Some(&modal::Cat::Check) {
            return false;
        }
        if matches!(k.code, KeyCode::Tab | KeyCode::BackTab | KeyCode::Esc) {
            self.screen.check_page = None;
            return false;
        }
        let Some(project) = project_of(self) else {
            self.notify("open a session in a project to set its check".into(), true);
            return true;
        };
        let mut d = self.screen.check_page.take().filter(|d| same_path(&d.project, &project)).unwrap_or_else(|| CheckDraft::new(project));
        let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        if !d.editing && plain && k.code == KeyCode::Char('o') {
            self.screen.check_page = Some(d);
            return false;
        }
        match self.draft_key(&mut d, k) {
            Outcome::Stay => self.screen.check_page = Some(d),
            Outcome::Close => self.mode = Mode::Normal,
        }
        true
    }

    pub(in crate::client) fn on_check_draft_hit(&mut self, h: DraftHit) {
        let key = |code| KeyEvent::new(code, KeyModifiers::NONE);
        let k = match h {
            DraftHit::Save => KeyEvent::new(KeyCode::Char('s'), KeyModifiers::CONTROL),
            DraftHit::RunNow => key(KeyCode::Char('r')),
            DraftHit::Remove => key(KeyCode::Delete),
            DraftHit::Cancel => key(KeyCode::Esc),
            DraftHit::Command | DraftHit::Timeout(_) | DraftHit::Point(_) => return self.draft_point_at(h),
        };
        self.on_key(k);
    }

    /// A click on a row: go to it and do what its key does.
    fn draft_point_at(&mut self, h: DraftHit) {
        let (row, code) = match h {
            DraftHit::Timeout(up) => (DraftRow::Timeout, if up { KeyCode::Right } else { KeyCode::Left }),
            DraftHit::Point(up) => (DraftRow::Points, if up { KeyCode::Right } else { KeyCode::Left }),
            _ => (DraftRow::Command, KeyCode::Enter),
        };
        let d = match &mut self.mode {
            Mode::ProjectCheck(d) => Some(&mut **d),
            Mode::ScreenSettings(_) => self.screen.check_page.as_mut(),
            _ => None,
        };
        let Some(d) = d else {
            if let Some(project) = project_of(self).filter(|_| matches!(self.mode, Mode::ScreenSettings(_))) {
                self.screen.check_page = Some(CheckDraft { row, ..CheckDraft::new(project) });
                self.on_key(KeyEvent::new(code, KeyModifiers::NONE));
            }
            return;
        };
        if d.editing && row == DraftRow::Command {
            return;
        }
        (d.row, d.editing) = (row, false);
        self.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn draft_key(&mut self, d: &mut CheckDraft, k: &KeyEvent) -> Outcome {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        let all = points(self, &d.project);
        let point = selected(&all, d);
        match k.code {
            KeyCode::Esc => return Outcome::Close,
            KeyCode::Char('s') if ctrl && !d.saving => self.save_draft(d, point.as_ref()),
            _ if d.saving => {}
            _ if d.editing => edit_key(d, k),
            KeyCode::Up | KeyCode::Down => {
                let rows = rows(&all);
                let at = rows.iter().position(|r| *r == d.row).unwrap_or(0);
                let to = if k.code == KeyCode::Up { at.saturating_sub(1) } else { (at + 1).min(rows.len() - 1) };
                d.row = rows[to];
            }
            KeyCode::Left | KeyCode::Right => {
                let dir = if k.code == KeyCode::Right { 1 } else { -1 };
                match d.row {
                    DraftRow::Timeout => self.step_draft_timeout(d, point.as_ref(), dir),
                    DraftRow::Points if all.len() > 1 => {
                        let n = all.len() as i64;
                        *d = CheckDraft { point: (d.point as i64 + dir).rem_euclid(n) as usize, row: DraftRow::Points, ..CheckDraft::new(d.project.clone()) };
                    }
                    _ => {}
                }
            }
            KeyCode::Char('r') if plain && point.is_some() => self.run_point_now(&d.project, point.as_ref()),
            KeyCode::Delete => self.remove_point(d, point.as_ref()),
            KeyCode::Enter if d.row == DraftRow::Command => self.start_editing(d, point.as_ref(), None),
            KeyCode::Char(c) if plain && d.row == DraftRow::Command => self.start_editing(d, point.as_ref(), Some(c)),
            _ => {}
        }
        Outcome::Stay
    }

    fn start_editing(&mut self, d: &mut CheckDraft, p: Option<&Point>, typed: Option<char>) {
        if let Some(p) = p.filter(|p| p.repos_own()) {
            let file = match &p.source {
                CheckSource::Repo { file, .. } => file.as_str(),
                CheckSource::User => "",
            };
            return self.notify(format!("this command comes from the repo's {file}: change it there"), false);
        }
        let command = d.command.get_or_insert_with(|| p.map(|p| p.command.clone()).unwrap_or_default());
        command.extend(typed);
        d.editing = true;
    }

    fn step_draft_timeout(&mut self, d: &mut CheckDraft, p: Option<&Point>, dir: i64) {
        match p {
            None => self.notify("set a command first; the timeout belongs to it".into(), false),
            Some(p) if p.repos_own() => self.notify("this checkpoint's timeout comes from the repo's file".into(), false),
            Some(p) => d.timeout_s = Some(step_timeout(d.timeout_s.unwrap_or(p.timeout_s), dir)),
        }
    }

    fn run_point_now(&mut self, project: &Path, p: Option<&Point>) {
        let Some(p) = p else { return };
        match run_target(self, project) {
            Some(term) => self.rerun_checkpoint(term, Some(p.name.clone())),
            None => self.notify("no agent has run in this project yet: the check runs when its turn ends".into(), false),
        }
    }

    /// Del: your checkpoint's table goes; for a repo's, what you chose about it is forgotten.
    fn remove_point(&mut self, d: &mut CheckDraft, p: Option<&Point>) {
        let Some(p) = p else { return };
        if p.repos_own() {
            self.cmd(Command::ForgetCheckApproval { project: d.project.clone(), name: p.name.clone() });
            return self.notify(format!("Forgot your choice. {} asks again before running {}.", folder_name(&d.project), p.name), false);
        }
        let edit = CheckEdit::Remove { project: d.project.clone(), name: p.name.clone() };
        d.saving = true;
        self.save_check(edit, "Removed.", false);
    }

    /// Ctrl+S: write the shown checkpoint, or a first one.
    fn save_draft(&mut self, d: &mut CheckDraft, p: Option<&Point>) {
        match draft_edit(d, p) {
            Ok((edit, said)) => {
                (d.saving, d.editing) = (true, false);
                self.save_check(edit, said, true);
            }
            Err(why) => self.notify(why.into(), false),
        }
    }

    /// Write off the UI thread; `close` the host once it is saved.
    fn save_check(&mut self, edit: CheckEdit, said: &'static str, close: bool) {
        self.spawn_bg(move || {
            let result = edit.write().and_then(|()| Config::load());
            Bg::Then(Box::new(move |app| app.check_saved(result, said, close)))
        });
    }

    pub(in crate::client) fn check_saved(&mut self, result: Result<Config>, said: &str, close: bool) {
        let host = match &self.mode {
            Mode::ProjectCheck(_) => Some(Host::Modal),
            Mode::ScreenSettings(_) => Some(Host::Page),
            _ => None,
        };
        match result {
            Ok(cfg) => {
                self.apply_config(cfg);
                match host {
                    Some(Host::Modal) if close => self.mode = Mode::Normal,
                    Some(Host::Page) if close => self.screen.check_page = None,
                    _ => self.with_draft(|d| *d = CheckDraft::new(d.project.clone())),
                }
                self.notify(said.into(), false);
            }
            Err(error) => {
                self.with_draft(|d| d.saving = false);
                self.notify(format!("couldn't save the check: {error:#}"), true);
            }
        }
    }

    fn with_draft(&mut self, f: impl FnOnce(&mut CheckDraft)) {
        match &mut self.mode {
            Mode::ProjectCheck(d) => f(d),
            Mode::ScreenSettings(_) => {
                if let Some(d) = &mut self.screen.check_page {
                    f(d);
                }
            }
            _ => {}
        }
    }
}

/// What Ctrl+S writes for the shown checkpoint `p` (None: a first one) and what to say once it
/// is written; or why there is nothing to write.
fn draft_edit(d: &CheckDraft, p: Option<&Point>) -> Result<(CheckEdit, &'static str), &'static str> {
    let typed = d.command.as_deref().map(str::trim);
    let Some(p) = p else {
        let command = typed.filter(|c| !c.is_empty()).ok_or("type a command first")?;
        let edit = CheckEdit::Set { project: d.project.clone(), name: None, command: command.into(), timeout_s: CHECK_TIMEOUT_S_DEFAULT };
        return Ok((edit, "Saved. Runs after the next turn ends."));
    };
    if p.repos_own() {
        return Err("this checkpoint comes from the repo's file; nothing here to save");
    }
    let command = typed.unwrap_or(&p.command).to_string();
    let timeout_s = d.timeout_s.unwrap_or(p.timeout_s);
    if command.is_empty() {
        return Err("a check needs a command; Del removes it");
    }
    if command == p.command && timeout_s == p.timeout_s {
        return Err("nothing changed");
    }
    Ok((CheckEdit::Set { project: d.project.clone(), name: Some(p.name.clone()), command, timeout_s }, "Saved."))
}

/// Pasted text goes into the command being typed.
pub(in crate::client) fn paste_into_draft(d: &mut CheckDraft, text: &str) {
    if d.editing {
        d.command.get_or_insert_default().push_str(text.lines().next().unwrap_or_default());
    }
}

fn edit_key(d: &mut CheckDraft, k: &KeyEvent) {
    let plain = !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
    let command = d.command.get_or_insert_default();
    match k.code {
        KeyCode::Enter | KeyCode::Up | KeyCode::Down => d.editing = false,
        KeyCode::Backspace => {
            command.pop();
        }
        KeyCode::Char('u') if !plain => command.clear(),
        KeyCode::Char(c) if plain => command.push(c),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn point(name: &str, command: &str, source: CheckSource) -> Point {
        Point { name: name.into(), command: command.into(), timeout_s: CHECK_TIMEOUT_S_DEFAULT, source, result: None }
    }

    #[test]
    fn ctrl_s_writes_the_shown_checkpoint_by_name_or_a_first_one() {
        let d = |command: Option<&str>| CheckDraft { command: command.map(String::from), ..CheckDraft::new("/a".into()) };
        let unit = point("unit", "npm test", CheckSource::User);
        let Ok((CheckEdit::Set { name, command, timeout_s, .. }, said)) = draft_edit(&d(Some(" npm test -- --run ")), Some(&unit)) else { panic!("an edit") };
        assert_eq!((name.as_deref(), command.as_str(), timeout_s, said), (Some("unit"), "npm test -- --run", CHECK_TIMEOUT_S_DEFAULT, "Saved."));
        let Ok((CheckEdit::Set { name, command, .. }, said)) = draft_edit(&d(Some("cargo test")), None) else { panic!("a first one") };
        assert_eq!((name, command.as_str(), said), (None, "cargo test", "Saved. Runs after the next turn ends."));
        let timeout = CheckDraft { timeout_s: Some(900), ..d(None) };
        assert!(matches!(draft_edit(&timeout, Some(&unit)), Ok((CheckEdit::Set { timeout_s: 900, .. }, _))), "a new timeout alone is a change");
    }

    #[test]
    fn ctrl_s_says_why_when_there_is_nothing_to_write() {
        let d = |command: Option<&str>| CheckDraft { command: command.map(String::from), ..CheckDraft::new("/a".into()) };
        let unit = point("unit", "npm test", CheckSource::User);
        let repo = point("test", "npm test", CheckSource::Repo { file: ".seshi/checks.toml".into(), commit: None, committed_at: None, uncommitted: true });
        assert_eq!(draft_edit(&d(None), Some(&unit)).err(), Some("nothing changed"));
        assert_eq!(draft_edit(&d(Some("npm test")), Some(&unit)).err(), Some("nothing changed"));
        assert_eq!(draft_edit(&d(Some("  ")), Some(&unit)).err(), Some("a check needs a command; Del removes it"));
        assert_eq!(draft_edit(&d(None), None).err(), Some("type a command first"));
        assert!(draft_edit(&d(Some("rm -rf ~")), Some(&repo)).is_err(), "a repo's command is changed in the repo");
    }
}
