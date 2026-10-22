//! Settings > Team: the family a commander's jobs run on, for everyone and for the project you
//! are in, and the jobs themselves. `[[team.projects]]` and `[[team.jobs]]` are lists, so these
//! rows have their own editor and are not in `modal::SETTINGS`.
use super::check_settings::{is_for, project_of};
use super::*;
use crate::client::{Bg, modal};
use crate::config::{Config, Family, Team, TeamJob, same_path, validate_team};
use anyhow::{Context, Result};
use toml_edit::{ArrayOfTables, Item, Table};

/// The heading over the family rows (the global setting and this project's).
pub(in crate::client) const FAMILY_GROUP: &str = "TEAM";
pub(in crate::client) const JOBS_GROUP: &str = "JOBS";
pub(in crate::client) const EMPTY: &str = "No team set. Add jobs (planner, builder, reviewer) to give a commander a model for each.";
/// How an empty model reads: the agent picks its own.
const DEFAULT_MODEL: &str = "default";
const GLOBAL: &str = "global";
const ADD_LABEL: &str = "+ add job";
const NOT_A_TABLE: &str = "`team` in config.toml is not a table";
const NOT_A_LIST: &str = "`team.projects` in config.toml is not a list of [[team.projects]] tables";
const NOT_A_JOB_LIST: &str = "`team.jobs` in config.toml is not a list of [[team.jobs]] tables";
/// Settings rows on the Team page above the first `TeamRow`: the global family.
const TEAM_ROWS_BEFORE: usize = 1;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum TeamRow {
    /// The current project's own family, or the global one.
    Project,
    Job(usize),
    /// Starts the editor on a new job.
    Add,
}

pub(in crate::client) fn rows(cfg: &Config) -> Vec<TeamRow> {
    std::iter::once(TeamRow::Project).chain((0..cfg.team.jobs.len()).map(TeamRow::Job)).chain([TeamRow::Add]).collect()
}

pub(in crate::client) fn group(row: TeamRow) -> &'static str {
    match row {
        TeamRow::Project => FAMILY_GROUP,
        TeamRow::Job(_) | TeamRow::Add => JOBS_GROUP,
    }
}

pub(in crate::client) fn label(cfg: &Config, row: TeamRow) -> String {
    match row {
        TeamRow::Project => "This project".into(),
        TeamRow::Job(i) => cfg.team.jobs.get(i).map_or_else(String::new, |j| j.name.clone()),
        TeamRow::Add => ADD_LABEL.into(),
    }
}

/// The field of a job the editor is on; Enter moves to the next and saves after the last.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum JobField {
    Name,
    Claude,
    Codex,
}

impl JobField {
    fn next(self) -> Option<JobField> {
        match self {
            JobField::Name => Some(JobField::Claude),
            JobField::Claude => Some(JobField::Codex),
            JobField::Codex => None,
        }
    }

    fn label(self) -> &'static str {
        match self {
            JobField::Name => "name",
            JobField::Claude => "claude model",
            JobField::Codex => "codex model",
        }
    }

    fn help(self) -> &'static str {
        match self {
            JobField::Name => "Step 1 of 3: the job's name, in letters, digits, - and _. Enter goes on, Esc cancels.",
            JobField::Claude => "Step 2 of 3: the Claude model, passed to claude unchanged. Empty is its default.",
            JobField::Codex => "Step 3 of 3: the Codex model, passed to codex unchanged. Enter saves, Esc cancels.",
        }
    }

    fn get(self, job: &TeamJob) -> &str {
        match self {
            JobField::Name => &job.name,
            JobField::Claude => &job.claude,
            JobField::Codex => &job.codex,
        }
    }

    fn set(self, job: &mut TeamJob, text: &str) {
        let slot = match self {
            JobField::Name => &mut job.name,
            JobField::Claude => &mut job.claude,
            JobField::Codex => &mut job.codex,
        };
        *slot = text.to_string();
    }
}

/// A job being added (`old_name` none) or edited, one field at a time.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct JobDraft {
    pub old_name: Option<String>,
    pub job: TeamJob,
    pub field: JobField,
}

impl JobDraft {
    /// The team as it would be with this draft saved.
    fn applied_to(&self, team: &Team) -> Team {
        let mut team = team.clone();
        match self.old_name.as_ref().and_then(|n| team.jobs.iter().position(|j| j.name == *n)) {
            Some(i) => team.jobs[i] = self.job.clone(),
            None => team.jobs.push(self.job.clone()),
        }
        team
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
    /// Add a job, or replace the one named `old_name` where it stands.
    SetJob { old_name: Option<String>, job: TeamJob },
    RemoveJob { name: String },
}

impl TeamEdit {
    pub fn write(&self) -> Result<()> {
        modal::edit_config(|doc| self.apply(doc))
    }

    pub fn apply(&self, doc: &mut toml_edit::DocumentMut) -> Result<()> {
        match self {
            TeamEdit::Family { project, family } => set_project_family(doc, project, *family),
            TeamEdit::SetJob { old_name, job } => set_job(doc, old_name.as_deref(), job),
            TeamEdit::RemoveJob { name } => remove_job(doc, name),
        }
    }
}

/// The `[team]` table, made (without a header of its own) when there is none.
fn team_table(doc: &mut toml_edit::DocumentMut) -> Result<&mut Table> {
    let mut new = Table::new();
    new.set_implicit(true);
    doc.entry("team").or_insert(Item::Table(new)).as_table_mut().context(NOT_A_TABLE)
}

/// A `[team]` that was only made to hold a list goes with the list's last entry.
fn drop_if_empty(doc: &mut toml_edit::DocumentMut) {
    if doc.get("team").and_then(Item::as_table).is_some_and(|t| t.is_empty() && t.is_implicit()) {
        doc.remove("team");
    }
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
    drop_if_empty(doc);
    Ok(())
}

fn jobs_list(doc: &mut toml_edit::DocumentMut) -> Result<&mut ArrayOfTables> {
    team_table(doc)?.entry("jobs").or_insert(Item::ArrayOfTables(ArrayOfTables::new())).as_array_of_tables_mut().context(NOT_A_JOB_LIST)
}

fn named(table: &Table, name: &str) -> bool {
    table.get("name").and_then(Item::as_str) == Some(name)
}

fn set_job(doc: &mut toml_edit::DocumentMut, old_name: Option<&str>, job: &TeamJob) -> Result<()> {
    let list = jobs_list(doc)?;
    let at = old_name.and_then(|n| list.iter().position(|t| named(t, n)));
    let fields = [("name", &job.name), ("claude", &job.claude), ("codex", &job.codex)];
    match at.and_then(|i| list.get_mut(i)) {
        Some(table) => {
            for (key, value) in fields {
                super::check_settings::set_keeping_decor(table, key, value.as_str().into());
            }
        }
        None => {
            let mut table = Table::new();
            for (key, value) in fields {
                table.insert(key, toml_edit::value(value.as_str()));
            }
            list.push(table);
        }
    }
    Ok(())
}

fn remove_job(doc: &mut toml_edit::DocumentMut, name: &str) -> Result<()> {
    if doc.get("team").is_none() {
        return Ok(());
    }
    let list = jobs_list(doc)?;
    let at = list.iter().position(|t| named(t, name));
    if let Some(i) = at {
        list.remove(i);
    }
    if list.is_empty()
        && let Some(team) = doc.get_mut("team").and_then(Item::as_table_mut)
    {
        team.remove("jobs");
    }
    drop_if_empty(doc);
    Ok(())
}

/// What the selected row does, shown under the list. Two lines wide.
pub(in crate::client) fn help(app: &App, row: TeamRow, v: &SettingsView) -> String {
    if let Some(draft) = v.job.as_ref().filter(|_| v.editing.is_some()) {
        return draft.field.help().into();
    }
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
            format!("In this project a commander starts {} on {}, {on}. Enter edits it, Del removes it.", job.name, family.agent())
        }
        TeamRow::Add => "A job (planner, builder, reviewer, …) with a Claude and a Codex model. Enter or a adds one.".into(),
    }
}

/// The value column of a Team row, in `room` columns.
pub(in crate::client) fn control(app: &App, t: &Theme, row: TeamRow, v: &SettingsView, selected: bool, room: usize) -> Vec<Seg> {
    let muted = Style::default().fg(t.muted);
    if let (true, Some(draft), Some(text)) = (selected, &v.job, &v.editing) {
        let label = format!("{} ", draft.field.label());
        let mut out = vec![seg(label.clone(), muted)];
        out.extend(super::check_settings::draft_segs(t, text, room.saturating_sub(label.width())));
        return out;
    }
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
        TeamRow::Add => vec![seg(truncate("Enter or a", room), muted)],
    }
}

/// `claude opus · codex gpt-6.1-sol`, the active family's model emphasised.
fn job_models(t: &Theme, job: &TeamJob, active: Family, room: usize) -> Vec<Seg> {
    let muted = Style::default().fg(t.muted);
    let mut out = Vec::new();
    for (k, family) in Family::ALL.into_iter().enumerate() {
        if k > 0 {
            out.push(seg(" · ", muted));
        }
        let style = if family == active { Style::default().fg(t.strong).add_modifier(Modifier::BOLD) } else { Style::default().fg(t.muted) };
        out.push(seg(format!("{} ", family.agent()), muted));
        out.push(seg(model_or_default(job, family).to_string(), style));
    }
    fit(out, room)
}

fn model_or_default(job: &TeamJob, family: Family) -> &str {
    Some(job.model(family)).filter(|m| !m.is_empty()).unwrap_or(DEFAULT_MODEL)
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
        out.push(if w <= left { (text, style) } else { (truncate(&text, left), style) });
        left = left.saturating_sub(w);
    }
    out
}

impl App {
    /// Keys on the Team page that the generic Settings keys do not cover. `row` is the selected
    /// row when it is a Team row. True when it took the key.
    pub(in crate::client) fn on_team_page_key(&mut self, v: &mut SettingsView, row: Option<TeamRow>, k: &KeyEvent) -> bool {
        let enter = matches!(k.code, KeyCode::Enter | KeyCode::Char(' '));
        match (row, k.code) {
            (Some(TeamRow::Project), KeyCode::Left | KeyCode::Char('h')) => self.step_project_family(-1),
            (Some(TeamRow::Project), KeyCode::Right | KeyCode::Char('l')) => self.step_project_family(1),
            (Some(TeamRow::Project), _) if enter => self.step_project_family(1),
            (Some(TeamRow::Job(i)), _) if enter => self.start_job_edit(v, Some(i)),
            (Some(TeamRow::Job(i)), KeyCode::Delete) => self.ask_remove_job(v, i),
            (Some(TeamRow::Add), _) if enter => self.start_job_edit(v, None),
            (_, KeyCode::Char('a')) => self.start_job_edit(v, None),
            _ => return false,
        }
        true
    }

    /// Open the editor on job `at`, or on a new one on the add row.
    fn start_job_edit(&mut self, v: &mut SettingsView, at: Option<usize>) {
        let job = at.and_then(|i| self.cfg.team.jobs.get(i)).cloned();
        if at.is_none() {
            let add = rows(&self.cfg).iter().position(|r| *r == TeamRow::Add);
            v.sel = add.map_or(v.sel, |i| i + TEAM_ROWS_BEFORE);
        }
        let draft = JobDraft { old_name: job.as_ref().map(|j| j.name.clone()), job: job.unwrap_or_default(), field: JobField::Name };
        v.editing = Some(draft.job.name.clone());
        v.job = Some(draft);
    }

    /// Enter in the job editor: keep the field and go on, or save after the last. A value the
    /// team would refuse stays in the field with the reason in a toast; nothing is written.
    pub(in crate::client) fn submit_job_field(&mut self, v: &mut SettingsView, text: String) {
        let Some(mut draft) = v.job.take() else { return };
        draft.field.set(&mut draft.job, text.trim());
        if let Err(why) = validate_team(&draft.applied_to(&self.cfg.team)) {
            self.notify(format!("{why:#}"), true);
            v.editing = Some(text);
            v.job = Some(draft);
            return;
        }
        match draft.field.next() {
            Some(next) => {
                draft.field = next;
                v.editing = Some(next.get(&draft.job).to_string());
                v.job = Some(draft);
            }
            None if self.cfg.team.jobs.contains(&draft.job) => {}
            None => {
                let edit = TeamEdit::SetJob { old_name: draft.old_name.clone(), job: draft.job.clone() };
                self.save_team(edit, Some(draft));
            }
        }
    }

    fn ask_remove_job(&mut self, v: &SettingsView, i: usize) {
        let Some(job) = self.cfg.team.jobs.get(i) else { return };
        let models = Family::ALL.map(|f| format!("{} {}", f.agent(), model_or_default(job, f))).join(" · ");
        let name = job.name.clone();
        self.screen.settings_return = Some(Box::new(v.clone()));
        self.mode = Mode::Confirm(Box::new(crate::client::menu::Confirm {
            title: "Remove job".into(),
            sub: String::new(),
            what: name.clone(),
            detail: models,
            note: "A commander can no longer start helpers for this job.".into(),
            list: Vec::new(),
            yes: "Remove".into(),
            key: '\n',
            danger: true,
            act: crate::client::menu::Act::RemoveTeamJob(name),
        }));
    }

    pub(in crate::client) fn remove_team_job(&mut self, name: String) {
        self.save_team(TeamEdit::RemoveJob { name }, None);
    }

    fn step_project_family(&mut self, dir: i64) {
        let Some(project) = project_of(self) else {
            self.notify("open a session in a project to give it its own family".into(), true);
            return;
        };
        let family = step_override(override_of(&self.cfg, &project), dir);
        self.save_team(TeamEdit::Family { project, family }, None);
    }

    /// Write off the UI thread; `draft` is the job to put back in the editor if it fails.
    fn save_team(&mut self, edit: TeamEdit, draft: Option<JobDraft>) {
        self.spawn_bg(move || {
            let result = edit.write().and_then(|()| Config::load());
            Bg::Then(Box::new(move |app| app.team_saved(result, draft)))
        });
    }

    pub(in crate::client) fn team_saved(&mut self, result: Result<Config>, draft: Option<JobDraft>) {
        match result {
            Ok(cfg) => self.apply_config(cfg),
            Err(error) => {
                if let (Mode::ScreenSettings(v), Some(draft)) = (&mut self.mode, draft) {
                    v.editing = Some(draft.field.get(&draft.job).to_string());
                    v.job = Some(draft);
                }
                self.notify(format!("couldn't save the team: {error:#}"), true);
            }
        }
    }
}
