//! Settings > Team: the family a commander's jobs run on, for everyone and for the project you
//! are in, and the jobs themselves. `[[team.projects]]` and `[[team.jobs]]` are lists, so these
//! rows have their own editor and are not in `modal::SETTINGS`.
use super::check_settings::{is_for, project_of};
use super::*;
use crate::client::{Bg, modal};
use crate::config::{Config, Family, same_path};
use anyhow::{Context, Result};
use toml_edit::{ArrayOfTables, Item, Table};

/// The heading over the family rows (the global setting and this project's).
pub(in crate::client) const FAMILY_GROUP: &str = "TEAM";
pub(in crate::client) const JOBS_GROUP: &str = "JOBS";
pub(in crate::client) const EMPTY: &str = "No team set. Add jobs (planner, builder, reviewer) to give a commander a model for each.";
/// How an empty model reads: the agent picks its own.
const DEFAULT_MODEL: &str = "default";
const GLOBAL: &str = "global";
const NOT_A_TABLE: &str = "`team` in config.toml is not a table";
const NOT_A_LIST: &str = "`team.projects` in config.toml is not a list of [[team.projects]] tables";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum TeamRow {
    /// The current project's own family, or the global one.
    Project,
    Job(usize),
}

pub(in crate::client) fn rows(cfg: &Config) -> Vec<TeamRow> {
    std::iter::once(TeamRow::Project).chain((0..cfg.team.jobs.len()).map(TeamRow::Job)).collect()
}

pub(in crate::client) fn group(row: TeamRow) -> &'static str {
    match row {
        TeamRow::Project => FAMILY_GROUP,
        TeamRow::Job(_) => JOBS_GROUP,
    }
}

pub(in crate::client) fn label(cfg: &Config, row: TeamRow) -> String {
    match row {
        TeamRow::Project => "This project".into(),
        TeamRow::Job(i) => cfg.team.jobs.get(i).map_or_else(String::new, |j| j.name.clone()),
    }
}

/// The project's own family, if it has one.
fn override_of(cfg: &Config, project: &Path) -> Option<Family> {
    cfg.team.projects.iter().find(|p| same_path(&p.project, project)).map(|p| p.family)
}

/// The family the jobs run on where you are: the project's own, else the team's.
fn active(app: &App) -> Family {
    project_of(app).map_or(app.cfg.team.family, |p| crate::team::active_family(&app.cfg.team, &p))
}

/// The project row's next value: global, claude, codex, round again.
pub(in crate::client) fn step_override(cur: Option<Family>, dir: i64) -> Option<Family> {
    let choices = [None, Some(Family::Claude), Some(Family::Codex)];
    let at = choices.iter().position(|c| *c == cur).unwrap_or(0) as i64;
    choices[(at + dir).rem_euclid(choices.len() as i64) as usize]
}

/// A change to `[team]` in config.toml.
#[derive(Debug, Clone)]
pub(in crate::client) enum TeamEdit {
    /// The project's own family; `None` removes its entry so it follows the team's.
    Family { project: PathBuf, family: Option<Family> },
}

impl TeamEdit {
    pub fn write(&self) -> Result<()> {
        modal::edit_config(|doc| self.apply(doc))
    }

    pub fn apply(&self, doc: &mut toml_edit::DocumentMut) -> Result<()> {
        match self {
            TeamEdit::Family { project, family } => set_project_family(doc, project, *family),
        }
    }
}

/// The `[team]` table, made (without a header of its own) when there is none.
fn team_table(doc: &mut toml_edit::DocumentMut) -> Result<&mut Table> {
    let mut new = Table::new();
    new.set_implicit(true);
    doc.entry("team").or_insert(Item::Table(new)).as_table_mut().context(NOT_A_TABLE)
}

fn set_project_family(doc: &mut toml_edit::DocumentMut, project: &Path, family: Option<Family>) -> Result<()> {
    if family.is_none() && doc.get("team").is_none() {
        return Ok(());
    }
    let team = team_table(doc)?;
    let list = team.entry("projects").or_insert(Item::ArrayOfTables(ArrayOfTables::new())).as_array_of_tables_mut().context(NOT_A_LIST)?;
    let at = list.iter().position(|t| is_for(t, project));
    match (family, at.and_then(|i| list.get_mut(i))) {
        (Some(f), Some(table)) => super::check_settings::set_keeping_decor(table, "family", f.agent().into()),
        (Some(f), None) => {
            let mut table = Table::new();
            table.insert("project", toml_edit::value(project.to_string_lossy().as_ref()));
            table.insert("family", toml_edit::value(f.agent()));
            list.push(table);
        }
        (None, _) => {
            if let Some(i) = at {
                list.remove(i);
            }
            if list.is_empty() {
                team.remove("projects");
            }
        }
    }
    if team.is_empty() && team.is_implicit() {
        doc.remove("team");
    }
    Ok(())
}

/// What the selected row does, shown under the list. Two lines wide.
pub(in crate::client) fn help(app: &App, row: TeamRow) -> String {
    match row {
        TeamRow::Project => format!(
            "The family this project's jobs run on. {GLOBAL} follows Family above ({}); ← → pick one for this project only.",
            app.cfg.team.family.agent()
        ),
        TeamRow::Job(i) => {
            let Some(job) = app.cfg.team.jobs.get(i) else { return String::new() };
            let family = active(app);
            let model = job.model(family);
            let on = if model.is_empty() { "its default model".to_string() } else { format!("model {model}") };
            format!("In this project a commander starts {} on {}, {on}.", job.name, family.agent())
        }
    }
}

/// The value column of a Team row, in `room` columns.
pub(in crate::client) fn control(app: &App, t: &Theme, row: TeamRow, selected: bool, room: usize) -> Vec<Seg> {
    let muted = Style::default().fg(t.muted);
    match row {
        TeamRow::Project => {
            let Some(project) = project_of(app) else { return vec![seg(truncate("no project open", room), muted)] };
            let own = override_of(&app.cfg, &project);
            let arrow = Style::default().fg(if selected { t.accent } else { t.muted }).add_modifier(Modifier::BOLD);
            let value = Style::default().fg(if own.is_some() { t.strong } else { t.text }).add_modifier(Modifier::BOLD);
            let mut out = vec![seg("‹ ", arrow), seg(own.map_or(GLOBAL, Family::agent), value), seg(" ›", arrow)];
            if own.is_none() {
                out.push(seg(format!("  {}", app.cfg.team.family.agent()), muted));
            }
            out
        }
        TeamRow::Job(i) => match app.cfg.team.jobs.get(i) {
            Some(job) => job_models(t, job, active(app), room),
            None => Vec::new(),
        },
    }
}

/// `claude opus · codex gpt-6.1-sol`, the active family's model emphasised.
fn job_models(t: &Theme, job: &crate::config::TeamJob, active: Family, room: usize) -> Vec<Seg> {
    let muted = Style::default().fg(t.muted);
    let mut out = Vec::new();
    for (k, family) in Family::ALL.into_iter().enumerate() {
        if k > 0 {
            out.push(seg(" · ", muted));
        }
        let model = Some(job.model(family)).filter(|m| !m.is_empty()).unwrap_or(DEFAULT_MODEL);
        let style = if family == active { Style::default().fg(t.strong).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.muted) };
        out.push(seg(format!("{} ", family.agent()), muted));
        out.push(seg(model.to_string(), style));
    }
    fit(out, room)
}

/// Cut a row of segments to `room` columns.
fn fit(segs: Vec<Seg>, room: usize) -> Vec<Seg> {
    let mut left = room;
    let mut out = Vec::new();
    for (text, style) in segs {
        if left == 0 {
            break;
        }
        let w = text.width();
        out.push(if w <= left { (text.clone(), style) } else { (truncate(&text, left), style) });
        left = left.saturating_sub(w);
    }
    out
}

impl App {
    /// Keys on a Team row that the generic Settings keys do not cover. True when it took the key.
    pub(in crate::client) fn on_team_row_key(&mut self, row: TeamRow, k: &KeyEvent) -> bool {
        match (row, k.code) {
            (TeamRow::Project, KeyCode::Left | KeyCode::Char('h')) => self.step_project_family(-1),
            (TeamRow::Project, KeyCode::Right | KeyCode::Char('l') | KeyCode::Enter | KeyCode::Char(' ')) => self.step_project_family(1),
            _ => return false,
        }
        true
    }

    fn step_project_family(&mut self, dir: i64) {
        let Some(project) = project_of(self) else {
            self.notify("open a session in a project to give it its own family".into(), true);
            return;
        };
        let family = step_override(override_of(&self.cfg, &project), dir);
        self.save_team(TeamEdit::Family { project, family });
    }

    /// Write off the UI thread.
    fn save_team(&mut self, edit: TeamEdit) {
        self.spawn_bg(move || {
            let result = edit.write().and_then(|()| Config::load());
            Bg::Then(Box::new(move |app| app.team_saved(result)))
        });
    }

    pub(in crate::client) fn team_saved(&mut self, result: Result<Config>) {
        match result {
            Ok(cfg) => self.apply_config(cfg),
            Err(error) => self.notify(format!("couldn't save the team: {error:#}"), true),
        }
    }
}
