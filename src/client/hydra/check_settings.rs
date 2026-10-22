//! Settings > Check: the command Seshi runs when an agent's turn ends in the current project,
//! its timeout, and how the latest run came out. The entry is a `[[check]]` table keyed by
//! project, so these rows have their own editor and are not in `modal::SETTINGS`.
use super::check;
use super::*;
use crate::client::{Bg, modal};
use crate::config::{CHECK_TIMEOUT_S_DEFAULT, CheckDef, Config, same_path};
use crate::protocol::{CheckInfo, CheckState};
use anyhow::{Context, Result};
use toml_edit::{ArrayOfTables, Item, Table};

/// Round values the timeout steps through, in seconds.
const TIMEOUT_STEPS_S: [u64; 8] = [30, 60, 120, 300, 600, 900, 1800, 3600];
const SECS_PER_MIN: u64 = 60;
const COMMAND_PLACEHOLDER: &str = "e.g. npm test";
const RUNS_WHEN: &str = "after each turn ends · in the session's worktree";
const RUNS_WHEN_SHORT: &str = "after each turn · in its worktree";
const NEVER_RUN: &str = "never run · runs after the next turn ends";
/// Columns the cursor and a little padding take from an edited command.
const EDIT_CHROME_COLS: usize = 3;
/// Columns a command chip's padding takes.
const CHIP_PAD_COLS: usize = 2;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum CheckField {
    Command,
    Timeout,
    Runs,
    Status,
    Project,
    RunNow,
}

impl CheckField {
    pub const ALL: [CheckField; 6] = [CheckField::Command, CheckField::Timeout, CheckField::Runs, CheckField::Status, CheckField::Project, CheckField::RunNow];

    pub fn label(self) -> &'static str {
        match self {
            CheckField::Command => "Command",
            CheckField::Timeout => "Timeout",
            CheckField::Runs => "Runs",
            CheckField::Status => "Status",
            CheckField::Project => "Project",
            CheckField::RunNow => "Run now",
        }
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

/// A change to the project's `[[check]]` table in config.toml.
#[derive(Debug, Clone)]
pub(in crate::client) enum CheckEdit {
    Set { project: PathBuf, command: String, timeout_s: u64 },
    Remove { project: PathBuf },
}

impl CheckEdit {
    pub fn write(&self) -> Result<()> {
        modal::edit_config(|doc| self.apply(doc))
    }

    pub fn apply(&self, doc: &mut toml_edit::DocumentMut) -> Result<()> {
        match self {
            CheckEdit::Set { project, command, timeout_s } => {
                let list = doc.entry("check").or_insert(Item::ArrayOfTables(ArrayOfTables::new())).as_array_of_tables_mut().context(NOT_A_LIST)?;
                let at = list.iter().position(|t| is_for(t, project));
                match at.and_then(|i| list.get_mut(i)) {
                    Some(table) => {
                        set_keeping_decor(table, "command", command.as_str().into());
                        set_keeping_decor(table, "timeout_s", i64::try_from(*timeout_s).unwrap_or(i64::MAX).into());
                    }
                    None => {
                        let mut table = Table::new();
                        table.insert("project", toml_edit::value(project.to_string_lossy().as_ref()));
                        table.insert("command", toml_edit::value(command.as_str()));
                        table.insert("timeout_s", toml_edit::value(i64::try_from(*timeout_s).unwrap_or(i64::MAX)));
                        list.push(table);
                    }
                }
            }
            CheckEdit::Remove { project } => {
                let Some(item) = doc.get_mut("check") else { return Ok(()) };
                let list = item.as_array_of_tables_mut().context(NOT_A_LIST)?;
                let at = list.iter().position(|t| is_for(t, project));
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

const NOT_A_LIST: &str = "`check` in config.toml is not a list of [[check]] tables";

fn is_for(table: &Table, project: &Path) -> bool {
    table.get("project").and_then(Item::as_str).is_some_and(|p| same_path(Path::new(p), project))
}

/// Replace a value but keep the comments next to it.
fn set_keeping_decor(table: &mut Table, key: &str, mut value: toml_edit::Value) {
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

/// The project Settings edits: the repository of the workspace you are in.
fn project_of(app: &App) -> Option<PathBuf> {
    app.active_ws().map(|w| w.git.as_ref().map_or_else(|| w.cwd.clone(), |g| g.root.clone()))
}

fn entry<'a>(app: &'a App, project: &Path) -> Option<&'a CheckDef> {
    app.cfg.check.iter().find(|c| same_path(&c.project, project))
}

fn in_project(t: &TermInfo, project: &Path) -> bool {
    same_path(t.root.as_deref().unwrap_or(&t.cwd), project)
}

/// The most recently started result among the project's sessions, with the session's name.
fn latest<'a>(app: &'a App, project: &Path) -> Option<(&'a CheckInfo, String)> {
    latest_term(app, project).and_then(|t| t.check.as_ref().map(|c| (c, t.display_name())))
}

fn latest_term<'a>(app: &'a App, project: &Path) -> Option<&'a TermInfo> {
    app.snap.terms.values().filter(|t| in_project(t, project)).filter(|t| t.check.is_some()).max_by_key(|t| t.check.as_ref().map_or(0, |c| c.started_at))
}

/// The session "Run now" runs the check in: the one with the latest result, else the focused
/// agent of the project, else its first agent.
fn run_target(app: &App, project: &Path) -> Option<TermId> {
    let agent = |t: &&TermInfo| t.agent.is_some() && in_project(t, project);
    latest_term(app, project)
        .map(|t| t.id)
        .or_else(|| app.focused().and_then(|f| app.snap.terms.get(&f)).filter(agent).map(|t| t.id))
        .or_else(|| app.snap.terms.values().filter(agent).map(|t| t.id).min())
}

fn project_name(app: &App, project: &Path) -> String {
    app.active_ws()
        .and_then(|w| w.git.as_ref().map(|g| g.repo.clone()))
        .or_else(|| project.file_name().map(|n| n.to_string_lossy().into_owned()))
        .unwrap_or_default()
}

/// The words after the status glyph: the result, then what it took to know, with the session
/// last so a narrow row cuts that first.
fn result_words(info: &CheckInfo, name: &str) -> String {
    let ago = if info.started_at == 0 { String::new() } else { format!(" · {} ago", age(info.started_at)) };
    let code = info.exit_code.map(|c| format!(" · exit {c}")).unwrap_or_default();
    match info.state {
        CheckState::Checking => format!("checking · {name}"),
        CheckState::Passed => format!("passed{ago} · {name}"),
        CheckState::Failed => format!("failed{code}{ago} · {name}"),
        CheckState::Overridden => format!("overridden by you{ago} · {name}"),
        CheckState::CouldNotRun => format!("could not run{}{ago} · {name}", check::why_not_run(info.exit_code)),
    }
}

/// What the selected row does, shown under the list. Two lines wide.
pub(in crate::client) fn help(app: &App, field: CheckField) -> String {
    let set = project_of(app).is_some_and(|p| entry(app, &p).is_some());
    match field {
        CheckField::Command if set => "Runs through your shell when an agent's turn ends. Enter edits it, Del removes the check. Agents keep working while it runs.".into(),
        CheckField::Command => "No check set. Seshi can run one command itself when an agent's turn ends; the session isn't done until it passes. Agents keep working.".into(),
        CheckField::Timeout => "A run longer than this is stopped and reads as could not run. ← → step it.".into(),
        CheckField::Runs => "In the folder of the session whose turn ended, so a worktree is checked as it is.".into(),
        CheckField::Status => "The latest run among this project's sessions. A failed one keeps the session from reading done.".into(),
        CheckField::Project => "The repository's main checkout. Its worktrees share this one check.".into(),
        CheckField::RunNow => "Runs the check now in the session above. If its agent is working it waits for the turn to end.".into(),
    }
}

/// The value column of a Check row, in `room` columns.
pub(in crate::client) fn control(app: &App, t: &Theme, field: CheckField, v: &SettingsView, selected: bool, room: usize) -> Vec<Seg> {
    let project = project_of(app);
    let def = project.as_deref().and_then(|p| entry(app, p));
    let muted = Style::default().fg(t.muted);
    match field {
        CheckField::Command => command_control(t, def, v.editing.as_deref().filter(|_| selected), room),
        CheckField::Timeout => {
            let arrow = Style::default().fg(if selected { t.accent } else { t.muted }).add_modifier(Modifier::BOLD);
            let secs = def.map_or(CHECK_TIMEOUT_S_DEFAULT, |d| d.timeout_s);
            let value = Style::default().fg(if def.is_some() { t.strong } else { t.muted }).add_modifier(Modifier::BOLD);
            vec![seg("‹ ", arrow), seg(timeout_label(secs), value), seg(" ›", arrow)]
        }
        CheckField::Runs => vec![seg(truncate(if RUNS_WHEN.width() <= room { RUNS_WHEN } else { RUNS_WHEN_SHORT }, room), muted)],
        CheckField::Status => status_control(app, t, project.as_deref(), def.is_some(), room),
        CheckField::Project => match &project {
            Some(p) => vec![seg(truncate(&format!("{} · {}", project_name(app, p), tilde(p)), room), muted)],
            None => vec![seg(truncate("none open", room), muted)],
        },
        CheckField::RunNow => {
            let look = Look::of(&app.cfg.ui);
            let bg = if selected { t.sel_on(t.card) } else { t.card };
            button_pill(look, t, "Run now", "r", false, false, bg)
        }
    }
}

fn command_control(t: &Theme, def: Option<&CheckDef>, draft: Option<&str>, room: usize) -> Vec<Seg> {
    if let Some(draft) = draft {
        let keep = room.saturating_sub(EDIT_CHROME_COLS);
        let skip = draft.chars().count().saturating_sub(keep);
        let tail: String = draft.chars().skip(skip).collect();
        return vec![seg(format!(" {tail}"), Style::default().bg(t.card2).fg(t.strong)), seg("█ ", Style::default().bg(t.card2).fg(t.accent))];
    }
    match def {
        Some(d) => chip(&truncate(&d.command, room.saturating_sub(CHIP_PAD_COLS)), Style::default().bg(t.btn).fg(t.strong)),
        None => chip(COMMAND_PLACEHOLDER, Style::default().bg(t.btn).fg(t.muted)),
    }
}

fn status_control(app: &App, t: &Theme, project: Option<&Path>, has_check: bool, room: usize) -> Vec<Seg> {
    let muted = Style::default().fg(t.muted);
    if !has_check {
        return vec![seg(truncate("no check set", room), muted)];
    }
    let Some((info, name)) = project.and_then(|p| latest(app, p)) else {
        return vec![seg(truncate(&format!("… {NEVER_RUN}"), room), muted)];
    };
    let style = Style::default().fg(check::color(t, info)).add_modifier(if check::failed(info) { Modifier::BOLD } else { Modifier::empty() });
    let text = format!("{} {}", check::glyph(app, info), result_words(info, &name));
    vec![seg(truncate(&text, room), style)]
}

impl App {
    /// Keys on a Check row that the generic Settings keys do not cover. True when it took the key.
    pub(in crate::client) fn on_check_row_key(&mut self, v: &mut SettingsView, field: CheckField, k: &KeyEvent) -> bool {
        match (field, k.code) {
            (CheckField::Command, KeyCode::Enter | KeyCode::Char(' ')) => match project_of(self) {
                Some(p) => v.editing = Some(entry(self, &p).map(|d| d.command.clone()).unwrap_or_default()),
                None => self.notify("open a session in a project to set its check".into(), true),
            },
            (CheckField::Timeout, KeyCode::Left | KeyCode::Char('h')) => self.step_check_timeout(-1),
            (CheckField::Timeout, KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter | KeyCode::Char(' ')) => self.step_check_timeout(1),
            (CheckField::RunNow, KeyCode::Enter | KeyCode::Char(' ')) | (_, KeyCode::Char('r')) => self.run_check_now(),
            (_, KeyCode::Delete) => self.remove_check(),
            _ => return false,
        }
        true
    }

    /// Enter in the command field: save it, or remove the check when it is empty.
    pub(in crate::client) fn submit_check_command(&mut self, draft: String) {
        let Some(project) = project_of(self) else { return };
        let command = draft.trim().to_string();
        let current = entry(self, &project).map(|d| (d.command.clone(), d.timeout_s));
        let edit = match (command.is_empty(), current) {
            (true, Some(_)) => CheckEdit::Remove { project },
            (true, None) => return,
            (false, Some((old, _))) if old == command => return,
            (false, current) => CheckEdit::Set { project, command, timeout_s: current.map_or(CHECK_TIMEOUT_S_DEFAULT, |(_, t)| t) },
        };
        self.save_check(edit, Some(draft));
    }

    fn step_check_timeout(&mut self, dir: i64) {
        let Some(project) = project_of(self) else { return };
        let Some(def) = entry(self, &project) else {
            self.notify("set a command first; the timeout belongs to it".into(), false);
            return;
        };
        let timeout_s = step_timeout(def.timeout_s, dir);
        if timeout_s != def.timeout_s {
            let edit = CheckEdit::Set { project, command: def.command.clone(), timeout_s };
            self.save_check(edit, None);
        }
    }

    /// Settings' Run now: the project's check, in the session that last ran it.
    fn run_check_now(&mut self) {
        let Some(project) = project_of(self) else {
            self.notify("open a session in a project to run its check".into(), true);
            return;
        };
        if entry(self, &project).is_none() {
            self.notify("set a command first; there is nothing to run".into(), false);
            return;
        }
        match run_target(self, &project) {
            Some(term) => self.rerun_check(term),
            None => self.notify("no agent has run in this project yet: the check runs when its turn ends".into(), false),
        }
    }

    fn remove_check(&mut self) {
        if let Some(project) = project_of(self).filter(|p| entry(self, p).is_some()) {
            self.save_check(CheckEdit::Remove { project }, None);
        }
    }

    /// Write off the UI thread; `draft` is what to put back in the field if it fails.
    fn save_check(&mut self, edit: CheckEdit, draft: Option<String>) {
        self.spawn_bg(move || {
            let result = edit.write().and_then(|()| Config::load());
            Bg::Then(Box::new(move |app| app.check_saved(result, draft)))
        });
    }

    pub(in crate::client) fn check_saved(&mut self, result: Result<Config>, draft: Option<String>) {
        match result {
            Ok(cfg) => self.apply_config(cfg),
            Err(error) => {
                if let (Mode::HySettings(v), Some(draft)) = (&mut self.mode, draft) {
                    v.editing = Some(draft);
                }
                self.notify(format!("couldn't save the check: {error:#}"), true);
            }
        }
    }
}
