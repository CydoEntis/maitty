//! The workflow editor and run controls in the right-hand sheet.

use super::design::{BtnKind, fill, put, seg};
use super::hydra::{HyHit, btn, hints, hit};
use super::*;
use crate::workflows::{self as wf, ResolvedWorkflow, State, Workflow};
use anyhow::Context;
use ratatui::{buffer::Buffer, style::Style};

const SECTIONS: [&str; 3] = ["Profiles", "Workflows", "Runs"];
const REPORTS: [&str; 4] = ["Goal", "Plan", "Build", "Review"];
#[derive(Default)]
pub(super) struct AgentSheets {
    pub profiles: Option<Box<super::profiles::ProfilesView>>,
    pub workflows: Option<Box<WorkflowView>>,
    pub runs: Option<Box<WorkflowView>>,
}

pub(super) fn sheet_tab_key(key: &KeyEvent, tab: u8, browsing: bool) -> Option<u8> {
    let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
    let backwards = key.code == KeyCode::BackTab || key.modifiers.contains(KeyModifiers::SHIFT);
    match key.code {
        KeyCode::Tab | KeyCode::BackTab if ctrl || browsing => Some((tab + if backwards { SECTIONS.len() as u8 - 1 } else { 1 }) % SECTIONS.len() as u8),
        KeyCode::Left if browsing => Some((tab + SECTIONS.len() as u8 - 1) % SECTIONS.len() as u8),
        KeyCode::Right if browsing => Some((tab + 1) % SECTIONS.len() as u8),
        _ => None,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct WorkflowView {
    pub dir: PathBuf,
    pub entries: Option<Vec<ResolvedWorkflow>>,
    pub profiles: Vec<crate::profiles::Resolved>,
    pub runs: bool,
    pub sel: usize,
    pub scroll: usize,
    pub editor: Option<Workflow>,
    pub original: Option<String>,
    pub project: bool,
    pub field: usize,
    pub task: Option<String>,
    pub busy: bool,
    pub error: Option<String>,
    pub confirm: Option<(u64, WorkflowAction)>,
    pub setup: Option<Workflow>,
    pub run: Option<u64>,
    pub report: usize,
    pub picker: Option<usize>,
    pub cursor: Option<usize>,
}

impl WorkflowView {
    pub fn paste(&mut self, text: &str) {
        if let Some(task) = &mut self.task {
            super::text_input::insert(task, &mut self.cursor, text);
        } else if self.field == 0
            && let Some(editor) = &mut self.editor
        {
            super::text_input::insert(&mut editor.name, &mut self.cursor, text.lines().next().unwrap_or_default());
        }
    }
}

impl App {
    pub(super) fn switch_agent_sheet(&mut self, tab: u8) {
        if tab > 2 { return; }
        if tab < 2 && ipc::remote().is_some() {
            self.notify("Profile and workflow setup is local; open Seshi on the remote host.".into(), true); return;
        }
        let (dir, current, saving) = match &self.mode {
            Mode::Profiles(v) => (v.dir.clone(), 0, v.busy && v.entries.is_some()),
            Mode::Workflows(v) => (v.dir.clone(), if v.runs { 2 } else { 1 }, v.busy && v.entries.is_some()),
            _ => return,
        };
        if tab == current { return; }
        if saving { self.notify("Wait for the save to finish before changing sections.".into(), false); return; }
        match std::mem::replace(&mut self.mode, Mode::Normal) {
            Mode::Profiles(v) => self.agent_sheets.profiles = Some(v),
            Mode::Workflows(v) if v.runs => self.agent_sheets.runs = Some(v),
            Mode::Workflows(v) => self.agent_sheets.workflows = Some(v),
            _ => unreachable!(),
        }
        if tab == 0 {
            if let Some(v) = self.agent_sheets.profiles.take().filter(|v| v.dir == dir) {
                self.mode = Mode::Profiles(v);
                self.refresh_profiles(dir, false);
            } else { self.open_profiles_at(dir); }
        } else {
            let cached = if tab == 2 { &mut self.agent_sheets.runs } else { &mut self.agent_sheets.workflows };
            if let Some(v) = cached.take().filter(|v| v.dir == dir) {
                self.mode = Mode::Workflows(v);
                if tab == 1 { self.refresh_workflows(dir); }
            } else { self.open_workflows_at(dir, tab == 2); }
        }
    }

    pub(super) fn open_workflows_at(&mut self, dir: PathBuf, runs: bool) {
        if !runs && ipc::remote().is_some() {
            self.notify(
                "Workflow setup is local; run Seshi on the remote host.".into(),
                true,
            );
            return;
        }
        self.mode = Mode::Workflows(Box::new(WorkflowView {
            dir: dir.clone(),
            entries: None,
            profiles: Vec::new(),
            runs,
            sel: 0,
            scroll: 0,
            editor: None,
            original: None,
            project: false,
            field: 0,
            task: None,
            busy: !runs,
            error: None,
            confirm: None,
            setup: None,
            run: None,
            report: 0,
            picker: None,
            cursor: None,
        }));
        if !runs { self.refresh_workflows(dir); }
    }

    pub(super) fn open_workflows(&mut self, runs: bool) {
        self.open_workflows_at(self.hy_target_dir(), runs);
    }

    pub(super) fn open_workflow_run(&mut self, id: u64) {
        let Some(run) = self.snap.workflows.iter().find(|r| r.id == id) else {
            self.notify("Workflow run is no longer available.".into(), true);
            return;
        };
        let dir = run.source.clone();
        let sel = self.snap.workflows.iter().rev().position(|r| r.id == id).unwrap_or(0);
        self.open_workflows_at(dir, true);
        if let Mode::Workflows(view) = &mut self.mode { view.sel = sel; view.run = Some(id); view.report = 1; }
    }

    fn refresh_workflows(&mut self, dir: PathBuf) {
        self.spawn_bg(move || {
            let result = (|| {
                let cfg = Config::load()?;
                let workflows = wf::load(&cfg, &dir)?;
                let profiles = crate::profiles::load(&cfg, &dir)?;
                Ok::<_, anyhow::Error>((cfg, workflows, profiles))
            })()
            .map_err(|e| format!("{e:#}"));
            Bg::Then(Box::new(move |app| {
                if let Mode::Workflows(view) = &mut app.mode
                    && view.dir == dir
                {
                    // Initial loading is busy too; only an existing view can be saving.
                    if view.busy && view.entries.is_some() { return; }
                    view.busy = false;
                    match result {
                        Ok((cfg, entries, profiles)) => {
                            let selected = view.entries.as_ref().and_then(|v| v.get(view.sel)).map(|e| &e.workflow.name);
                            view.sel = selected.and_then(|name| entries.iter().position(|e| &e.workflow.name == name)).unwrap_or_else(|| view.sel.min(entries.len().saturating_sub(1)));
                            app.cfg.profiles = cfg.profiles;
                            app.cfg.workflows = cfg.workflows;
                            view.entries = Some(entries);
                            view.profiles = profiles;
                            if view.editor.is_none() && view.task.is_none() { view.error = None; }
                        }
                        Err(error) => {
                            view.error = Some(error);
                            view.entries = Some(Vec::new());
                        }
                    }
                }
            }))
        });
    }

    fn save_workflow(&mut self, view: &mut WorkflowView, starter: Option<crate::profiles::Runner>) {
        if starter.is_none() && let Some(editor) = &view.editor && let Err(error) = editor.profiles(&view.profiles) {
            view.error = Some(error.to_string()); return;
        }
        let dir = view.dir.clone();
        let project = view.project;
        let editor = view.editor.clone();
        let original = view.original.clone();
        view.busy = true;
        self.spawn_bg(move || {
            let result = (|| {
                let cfg = Config::load()?;
                if let Some(runner) = starter {
                    return wf::init(&cfg, &dir, runner, project);
                }
                let editor = editor.context("no workflow to save")?;
                editor.profiles(&crate::profiles::load(&cfg, &dir)?)?;
                let path = if project {
                    dir.join(crate::project::FILE)
                } else {
                    crate::config::workflow_path()?
                };
                wf::save(&path, original.as_deref(), &editor)
            })()
            .map_err(|e| format!("{e:#}"));
            Bg::Then(Box::new(move |app| {
                let mut reload = false;
                if let Mode::Workflows(v) = &mut app.mode
                    && v.dir == dir
                {
                    v.busy = false;
                    match result {
                        Ok(()) => {
                            v.editor = None;
                            v.original = None;
                            v.error = None;
                            reload = true;
                        }
                        Err(error) => v.error = Some(error),
                    }
                }
                if reload {
                    app.cmd(Command::ReloadConfig);
                    app.refresh_workflows(dir);
                }
            }))
        });
    }

    pub(super) fn on_workflows_key(&mut self, mut view: WorkflowView, key: &KeyEvent) {
        let browsing = view.editor.is_none() && view.task.is_none() && view.setup.is_none() && view.run.is_none() && view.confirm.is_none();
        if let Some(tab) = sheet_tab_key(key, if view.runs { 2 } else { 1 }, browsing) {
            self.switch_agent_sheet(tab);
            return;
        }
        let ctrl = key.modifiers.contains(KeyModifiers::CONTROL);
        if view.busy {
            if key.code == KeyCode::Esc && view.entries.is_none() { self.mode = Mode::Normal; }
            return;
        }
        if let Some(workflow) = &view.setup {
            match key.code {
                KeyCode::Esc => { view.setup = None; view.scroll = 0; }
                KeyCode::PageDown | KeyCode::Down => view.scroll = view.scroll.saturating_add(5),
                KeyCode::PageUp | KeyCode::Up => view.scroll = view.scroll.saturating_sub(5),
                KeyCode::Char('c') => {
                    let text = activation_instructions(workflow);
                    self.spawn_bg(move || {
                        let copied = super::copy::to_clipboard(&text);
                        Bg::Then(Box::new(move |app| app.notify(if copied {
                            "Instructions copied. Review and paste into AGENTS.md or CLAUDE.md; ask existing sessions to read them.".into()
                        } else { "Couldn't copy instructions. The preview is still available.".into() }, !copied)))
                    });
                }
                _ => {}
            }
        } else if let Some((run, action)) = view.confirm {
            view.confirm = None;
            if matches!(key.code, KeyCode::Enter | KeyCode::Char('y'))
            {
                self.cmd(Command::WorkflowControl {
                    run,
                    action,
                });
            }
        } else if let Some(id) = view.run {
            match key.code {
                KeyCode::Esc => { view.run = None; view.scroll = 0; }
                KeyCode::Tab | KeyCode::Right => { view.report = (view.report + 1) % REPORTS.len(); view.scroll = 0; }
                KeyCode::BackTab | KeyCode::Left => { view.report = (view.report + REPORTS.len() - 1) % REPORTS.len(); view.scroll = 0; }
                KeyCode::Char(c @ '1'..='4') => { view.report = (c as u8 - b'1') as usize; view.scroll = 0; }
                KeyCode::Down | KeyCode::PageDown => view.scroll = view.scroll.saturating_add(5),
                KeyCode::Up | KeyCode::PageUp => view.scroll = view.scroll.saturating_sub(5),
                KeyCode::Home => view.scroll = 0,
                KeyCode::End => view.scroll = usize::MAX,
                KeyCode::Char('a') if selected_run(self, &view).is_some_and(|r| r.state == State::Approval) => {
                    self.cmd(Command::WorkflowControl { run: id, action: WorkflowAction::Approve });
                }
                KeyCode::Char('c') if selected_run(self, &view).is_some_and(|r| !r.state.terminal()) => view.confirm = Some((id, WorkflowAction::Cancel)),
                KeyCode::Char('t') if selected_run(self, &view).is_some_and(|r| matches!(r.state, State::Failed | State::Interrupted)) => view.confirm = Some((id, WorkflowAction::Retry)),
                KeyCode::Char('o') => {
                    if let Some(term) = selected_run(self, &view).and_then(|r| r.term.or_else(|| r.finished.last().copied())) {
                        self.cmd(Command::FocusPane { term }); self.mode = Mode::Normal; return;
                    }
                }
                _ => {}
            }
        } else if let Some(selected) = &mut view.picker {
            let count = view.profiles.len();
            match key.code {
                KeyCode::Esc => view.picker = None,
                KeyCode::Down => *selected = selected.saturating_add(1).min(count.saturating_sub(1)),
                KeyCode::Up => *selected = selected.saturating_sub(1),
                KeyCode::Home => *selected = 0,
                KeyCode::End => *selected = count.saturating_sub(1),
                KeyCode::Enter => {
                    if let Some(profile) = view.profiles.get(*selected) && let Some(editor) = &mut view.editor {
                        *match view.field { 1 => &mut editor.planner, 2 => &mut editor.worker, _ => &mut editor.reviewer } = profile.profile.name.clone();
                    }
                    view.picker = None;
                }
                _ => {}
            }
        } else if let Some(task) = &mut view.task {
            match key.code {
                KeyCode::Esc => view.task = None,
                KeyCode::Enter if key.modifiers.contains(KeyModifiers::SHIFT) => super::text_input::insert(task, &mut view.cursor, "\n"),
                KeyCode::Enter => {
                    if task.trim().is_empty() || task.len() > wf::MAX_GOAL {
                        view.error = Some(format!("Enter a task of 1..{} bytes.", wf::MAX_GOAL));
                    } else if let Some(entry) = view.entries.as_ref().and_then(|v| v.get(view.sel))
                    {
                        self.cmd(Command::WorkflowStart {
                            name: entry.workflow.name.clone(),
                            cwd: view.dir.clone(),
                            goal: task.clone(),
                        });
                        view.task = None;
                        self.agent_sheets.workflows = Some(Box::new(view.clone()));
                        view.runs = true;
                        view.sel = 0;
                        view.error = None;
                    }
                }
                _ => super::text_input::key(task, &mut view.cursor, key),
            }
        } else if view.editor.is_some() {
            match key.code {
                KeyCode::Esc => {
                    view.editor = None;
                    view.error = None;
                }
                KeyCode::Char('s') if ctrl => self.save_workflow(&mut view, None),
                KeyCode::Tab | KeyCode::Down => { view.field = (view.field + 1) % 5; view.cursor = None; }
                KeyCode::BackTab | KeyCode::Up => { view.field = (view.field + 4) % 5; view.cursor = None; }
                KeyCode::Left | KeyCode::Right if (1..=3).contains(&view.field) => {
                    let names: Vec<_> = view
                        .profiles
                        .iter()
                        .map(|p| p.profile.name.clone())
                        .collect();
                    if !names.is_empty()
                        && let Some(editor) = &mut view.editor
                    {
                        let field = match view.field {
                            1 => &mut editor.planner,
                            2 => &mut editor.worker,
                            _ => &mut editor.reviewer,
                        };
                        let at = names.iter().position(|n| n == field).unwrap_or(0);
                        let at = if key.code == KeyCode::Left {
                            (at + names.len() - 1) % names.len()
                        } else {
                            (at + 1) % names.len()
                        };
                        *field = names[at].clone();
                    }
                }
                KeyCode::Enter | KeyCode::Char(' ') if (1..=3).contains(&view.field) => {
                    if let Some(editor) = &view.editor {
                        let name = match view.field { 1 => &editor.planner, 2 => &editor.worker, _ => &editor.reviewer };
                        view.picker = Some(view.profiles.iter().position(|p| &p.profile.name == name).unwrap_or(0));
                    }
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
                    if view.field == 4 && view.original.is_none() =>
                {
                    view.project = !view.project
                }
                _ if view.field == 0 => {
                    if let Some(e) = &mut view.editor { super::text_input::key(&mut e.name, &mut view.cursor, key); }
                }
                _ => {}
            }
        } else {
            let count = if view.runs {
                runs(self, &view).len()
            } else {
                view.entries.as_ref().map_or(0, Vec::len)
            };
            match key.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    return;
                }
                KeyCode::Char('p') => {
                    self.switch_agent_sheet(0);
                    return;
                }
                KeyCode::Char('w') => {
                    self.switch_agent_sheet(1); return;
                }
                KeyCode::Char('r') if !ctrl => {
                    self.switch_agent_sheet(2); return;
                }
                KeyCode::Char('r') if ctrl => {
                    let dir = view.dir.clone();
                    self.mode = Mode::Workflows(Box::new(view));
                    self.refresh_workflows(dir);
                    return;
                }
                KeyCode::Down => {
                    view.sel = (view.sel + 1).min(count.saturating_sub(1));
                    view.scroll = 0;
                }
                KeyCode::Up => {
                    view.sel = view.sel.saturating_sub(1);
                    view.scroll = 0;
                }
                KeyCode::PageDown => view.scroll = view.scroll.saturating_add(5),
                KeyCode::PageUp => view.scroll = view.scroll.saturating_sub(5),
                KeyCode::Char('s') if !view.runs => {
                    if let Some(entry) = view.entries.as_ref().and_then(|v| v.get(view.sel)) {
                        match entry.workflow.profiles(&view.profiles) {
                            Ok(_) => {
                                view.task = Some(String::new());
                                view.cursor = None;
                                view.error = None;
                            }
                            Err(e) => view.error = Some(e.to_string()),
                        }
                    }
                }
                KeyCode::Char('u') if !view.runs => {
                    if let Some(entry) = view.entries.as_ref().and_then(|v| v.get(view.sel)) {
                        match entry.workflow.profiles(&view.profiles) {
                            Ok(_) => { view.setup = Some(entry.workflow.clone()); view.scroll = 0; view.error = None; }
                            Err(error) => view.error = Some(error.to_string()),
                        }
                    }
                }
                KeyCode::Enter | KeyCode::Char('n' | 'e') if !view.runs => {
                    let entry = (key.code != KeyCode::Char('n'))
                        .then(|| view.entries.as_ref().and_then(|v| v.get(view.sel)))
                        .flatten();
                    view.original = entry.map(|e| e.workflow.name.clone());
                    view.project = entry.is_some_and(|e| e.project);
                    view.editor = Some(entry.map(|e| e.workflow.clone()).unwrap_or_else(|| {
                        let name = |i: usize| {
                            view.profiles
                                .get(i)
                                .map(|p| p.profile.name.clone())
                                .unwrap_or_default()
                        };
                        Workflow {
                            name: String::new(),
                            planner: name(0),
                            worker: name(1),
                            reviewer: name(2),
                        }
                    }));
                    view.field = 0;
                    view.cursor = None;
                    view.error = None;
                }
                KeyCode::Char('1' | '2') if !view.runs => self.save_workflow(
                    &mut view,
                    Some(if key.code == KeyCode::Char('1') {
                        crate::profiles::Runner::Claude
                    } else {
                        crate::profiles::Runner::Codex
                    }),
                ),
                KeyCode::Char('a') if view.runs => {
                    if let Some(run) = selected_run(self, &view) {
                        view.run = Some(run.id); view.report = 1; view.scroll = 0;
                    }
                }
                KeyCode::Char('c' | 't') if view.runs => {
                    if let Some(run) = selected_run(self, &view) { view.confirm = Some((run.id, if key.code == KeyCode::Char('c') {
                        WorkflowAction::Cancel
                    } else {
                        WorkflowAction::Retry
                    })); }
                }
                KeyCode::Enter if view.runs => {
                    if let Some(run) = selected_run(self, &view) {
                        view.run = Some(run.id); view.report = if run.plan.is_empty() { 0 } else { 1 }; view.scroll = 0;
                    }
                }
                KeyCode::Char('o') if view.runs => {
                    if let Some(term) = selected_run(self, &view)
                        .and_then(|r| r.term.or_else(|| r.finished.last().copied()))
                    {
                        self.cmd(Command::FocusPane { term });
                        self.mode = Mode::Normal;
                        return;
                    }
                }
                _ => {}
            }
        }
        self.mode = Mode::Workflows(Box::new(view));
    }
}

pub(super) fn activation_instructions(workflow: &Workflow) -> String {
    format!("For implementation tasks, use Seshi's `{0}` workflow. Unless already executing an assigned workflow stage, run `seshi workflow show {0}` and start it with a self-contained task using `seshi workflow run {0} -- \"<task>\"`. Monitor the returned id with `seshi workflow status <id>` and tell me when the plan needs approval. Do not approve it yourself or start duplicate runs while waiting. If assigned a stage, follow that stage instead of starting another workflow.", workflow.name)
}

pub(super) fn stage_badge(app: &App, run: &wf::Run) -> String {
    if run_label(app, run) == "needs you" { return format!("Needs your answer · {}", run.stage.label()); }
    match run.state {
        State::Approval => "Needs plan approval".into(),
        State::Running => match run.stage {
            wf::Stage::Plan => "Planning".into(),
            wf::Stage::Build => format!("Building {}/3", run.round),
            wf::Stage::Review => format!("Reviewing {}/3", run.round),
        },
        _ => format!("{} · {}", run.state.label(), run.stage.label()),
    }
}

fn runs<'a>(app: &'a App, _view: &WorkflowView) -> Vec<&'a wf::Run> {
    // The Runs tab is intentionally a machine-wide list, like the sidebar.
    app.snap.workflows.iter().rev().collect()
}

fn selected_run<'a>(app: &'a App, view: &WorkflowView) -> Option<&'a wf::Run> {
    if let Some(id) = view.run { app.snap.workflows.iter().find(|r| r.id == id) }
    else { runs(app, view).get(view.sel).copied() }
}

fn run_label(app: &App, run: &wf::Run) -> &'static str {
    if run.state == State::Running
        && run
            .term
            .and_then(|id| app.snap.terms.get(&id))
            .is_some_and(|t| t.status == Status::Blocked)
    {
        "needs you"
    } else {
        run.state.label()
    }
}

pub(super) fn draw_tabs(app: &mut App, buf: &mut Buffer, r: Rect, _t: &Theme, selected: u8) {
    let mut x = r.x + 2;
    for (i, title) in SECTIONS.iter().enumerate() {
        x = btn(
            app,
            buf,
            x,
            r.y + 1,
            title,
            "",
            if i == selected as usize {
                BtnKind::Primary
            } else {
                BtnKind::Normal
            },
            HyHit::WorkflowTab(i as u8),
            r.right().saturating_sub(1),
        ) + 1;
    }
    let note = if r.right().saturating_sub(x) >= 24 { "section" } else { "" };
    put(buf, x + 1, r.y + 1, &hints(_t, &[("Ctrl+Tab", note)]), r.right().saturating_sub(2));
}

pub(super) fn draw_workflows(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    t: &Theme,
    view: &WorkflowView,
) {
    let style = Style::default().bg(t.card);
    let (x, right) = (r.x + 2, r.right().saturating_sub(2));
    let bottom = r.bottom().saturating_sub(5);
    let mut y = r.y + 3;
    hit(app, Rect { x, y, width: right.saturating_sub(x), height: bottom.saturating_sub(y) }, HyHit::AgentSheetBody);
    if let Some(selected) = view.picker {
        let stage = match view.field { 1 => "planner", 2 => "worker", _ => "reviewer" };
        put(buf, x, y, &[seg(format!("Choose a profile for the {stage}"), style.fg(t.strong))], right); y += 2;
        let visible = bottom.saturating_sub(y + 2).max(1) as usize;
        let start = selected.saturating_sub(visible - 1);
        for (i, profile) in view.profiles.iter().enumerate().skip(start).take(visible) {
            let row = Rect { x, y, width: right.saturating_sub(x), height: 1 };
            let rs = hydra::list_row(app, buf, row, selected == i, t);
            put(buf, x + 2, y, &[seg(format!("{} · {}", profile.profile.name, profile.profile.agent.command()), rs.fg(t.strong))], right);
            hit(app, row, HyHit::WorkflowProfile(i)); y += 1;
        }
        if view.profiles.is_empty() { put(buf, x, y, &[seg("No profiles yet. Create one in Profiles first.", style.fg(t.muted))], right); }
        else if let Some(profile) = view.profiles.get(selected) {
            y += 1;
            put(buf, x, y, &[seg(profile.profile.summary(), style.fg(t.muted))], right);
        }
    } else if let Some(workflow) = &view.setup {
        let text = format!("Use in this project: {}\nPlan: {} → your approval\nBuild: {} → Review: {} (up to 3 rounds)\n\nReview and paste into AGENTS.md or CLAUDE.md:\n\n{}\n\nAsk existing sessions to read the updated instructions. Profiles keep their native permissions.\nProject: {}", workflow.name, workflow.planner, workflow.worker, workflow.reviewer, activation_instructions(workflow), view.dir.display());
        let lines = super::views::wrap(&text, right.saturating_sub(x) as usize);
        let count = bottom.saturating_sub(y) as usize;
        let start = view.scroll.min(lines.len().saturating_sub(count.max(1)));
        for line in lines.into_iter().skip(start).take(count) {
            put(buf, x, y, &[seg(line, style.fg(t.text))], right); y += 1;
        }
    } else if let Some(editor) = &view.editor {
        for (i, (name, value)) in [
            ("NAME", editor.name.as_str()),
            ("PLANNER", &editor.planner),
            ("WORKER", &editor.worker),
            ("REVIEWER", &editor.reviewer),
            (
                "SCOPE",
                if view.project {
                    "this project"
                } else {
                    "personal"
                },
            ),
        ]
        .into_iter()
        .enumerate()
        {
            let value = if i == 0 && view.field == 0 { super::text_input::window(value, view.cursor, right.saturating_sub(x + 12) as usize) } else { value.to_string() };
            let row = Rect {
                x,
                y,
                width: right.saturating_sub(x),
                height: 1,
            };
            fill(buf, row, if view.field == i { t.card2 } else { t.card });
            put(
                buf,
                x + 1,
                y,
                &[seg(
                    format!(
                        "{name:<10}{value}{}",
                        if (1..=3).contains(&i) { "  ▾ choose" } else if i == 4 && view.original.is_none() { "  ↔" } else { "" }
                    ),
                    style.fg(if view.field == i { t.accent } else { t.strong }),
                )],
                right,
            );
            hit(app, row, HyHit::WorkflowField(i));
            y += 2;
        }
    } else if let Some(task) = &view.task {
        put(
            buf,
            x,
            y,
            &[seg("Task for the selected workflow", style.fg(t.strong))],
            right,
        );
        y += 2;
        let lines = super::text_input::lines(&super::text_input::caret(task, view.cursor), right.saturating_sub(x) as usize);
        let height = bottom.saturating_sub(y).max(1) as usize;
        let caret = lines.iter().position(|line| line.contains('▏')).unwrap_or(0);
        for line in lines.into_iter().skip(caret.saturating_sub(height - 1)).take(height) {
            put(buf, x, y, &[seg(line, style.fg(t.text))], right);
            y += 1;
        }
    } else if view.runs {
        draw_runs(app, buf, r, t, view);
    } else {
        put(
            buf,
            x,
            y,
            &[seg(
                "Choose roles → approve a plan → build → review.",
                style.fg(t.muted),
            )],
            right,
        );
        y += 2;
        let entries = view.entries.as_deref().unwrap_or_default();
        if entries.is_empty() {
            for line in [
                if view.busy {
                    "Loading workflows…"
                } else {
                    "No workflows yet."
                },
                "Create a starter below, then edit its roles.",
                "Your own profiles work in every stage.",
            ] {
                put(buf, x, y, &[seg(line, style.fg(t.text))], right);
                y += 1;
            }
        } else {
            let visible = bottom.saturating_sub(y + 5).clamp(1, 5) as usize;
            let start = view.sel.saturating_sub(visible - 1);
            for (i, entry) in entries.iter().enumerate().skip(start).take(visible) {
                let row = Rect {
                    x,
                    y,
                    width: right.saturating_sub(x),
                    height: 1,
                };
                let rs = hydra::list_row(app, buf, row, i == view.sel, t);
                put(
                    buf,
                    x + 2,
                    y,
                    &[seg(
                        format!(
                            "{} · {}",
                            entry.workflow.name,
                            if entry.project { "project" } else { "personal" }
                        ),
                        rs.fg(t.strong),
                    )],
                    right,
                );
                hit(app, row, HyHit::WorkflowRow(i));
                y += 1;
            }
            if let Some(entry) = entries.get(view.sel) {
                for line in [
                    format!("Plan: {} → your approval", entry.workflow.planner),
                    format!("Build: {}", entry.workflow.worker),
                    format!("Review: {} · up to 3 rounds", entry.workflow.reviewer),
                ] {
                    y += 1;
                    if y >= bottom {
                        break;
                    }
                    put(buf, x, y, &[seg(line, style.fg(t.text))], right);
                }
            }
        }
    }
    if let Some(error) = &view.error {
        put(
            buf,
            x,
            bottom,
            &[seg(error.clone(), style.fg(t.err))],
            right,
        );
    }
    draw_footer(app, buf, r, t, view);
}

fn draw_runs(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme, view: &WorkflowView) {
    let (x, right) = (r.x + 2, r.right().saturating_sub(2));
    let style = Style::default().bg(t.card);
    let list: Vec<_> = runs(app, view).into_iter().cloned().collect();
    let mut y = r.y + 3;
    if view.run.is_some() {
        let Some(run) = selected_run(app, view).cloned() else {
            put(buf, x, y, &[seg("This run is no longer available. Esc returns to Runs.", style.fg(t.muted))], right); return;
        };
        put(buf, x, y, &[seg(format!("Run #{} · {} · {}", run.id, run.name, stage_badge(app, &run)), style.fg(t.strong))], right); y += 1;
        put(buf, x, y, &[seg(run.note.clone(), style.fg(t.muted))], right); y += 2;
        let mut bx = x;
        for (i, name) in REPORTS.iter().enumerate() {
            bx = btn(app, buf, bx, y, name, &(i + 1).to_string(), if view.report == i { BtnKind::Primary } else { BtnKind::Normal }, HyHit::WorkflowReport(i), right) + 1;
        }
        y += 2;
        let text = match view.report { 1 => &run.plan, 2 => &run.work, 3 => &run.review, _ => &run.goal };
        let empty = match view.report { 1 => "The planner has not submitted a plan yet.", 2 => "The worker has not submitted a report yet.", 3 => "The reviewer has not submitted a report yet.", _ => "No task text is available." };
        let text = if text.is_empty() { empty } else { text };
        let lines = super::views::wrap(text, right.saturating_sub(x) as usize);
        let count = r.bottom().saturating_sub(6 + y) as usize;
        let start = view.scroll.min(lines.len().saturating_sub(count.max(1)));
        if let Mode::Workflows(v) = &mut app.mode { v.scroll = start; }
        for line in lines.iter().skip(start).take(count) {
            put(buf, x, y, &[seg(line.clone(), style.fg(t.text))], right); y += 1;
        }
        put(buf, x, r.bottom().saturating_sub(6), &[seg(format!("Lines {}–{} of {} · PgUp/PgDn or wheel", start + 1, (start + count).min(lines.len()), lines.len()), style.fg(t.muted))], right);
        return;
    }
    if list.is_empty() {
        put(
            buf,
            x,
            y,
            &[seg(
                "No runs yet. Choose a workflow to start.",
                style.fg(t.muted),
            )],
            right,
        );
        return;
    }
    let visible = 3.min(r.height.saturating_sub(14) as usize).max(1);
    let start = view.sel.saturating_sub(visible - 1);
    for (i, run) in list.iter().enumerate().skip(start).take(visible) {
        let row = Rect {
            x,
            y,
            width: right.saturating_sub(x),
            height: 1,
        };
        let rs = hydra::list_row(app, buf, row, i == view.sel, t);
        put(
            buf,
            x + 2,
            y,
            &[seg(
                format!("{} · {} · {}", run.id, run.name, run_label(app, run)),
                rs.fg(t.strong),
            )],
            right,
        );
        hit(app, row, HyHit::WorkflowRow(i));
        y += 1;
    }
    if let Some(run) = list.get(view.sel) {
        let text = format!(
            "{} · {} · round {}/3\n{}\n\n{}\n{}",
            run.stage.label(),
            run_label(app, run),
            run.round,
            run.note,
            if run.state == State::Approval {
                "PLAN — read before approving"
            } else {
                "LATEST REPORT"
            },
            if run.state == State::Approval {
                &run.plan
            } else if !run.review.is_empty() {
                &run.review
            } else if !run.work.is_empty() {
                &run.work
            } else {
                &run.goal
            }
        );
        y += 1;
        let lines = super::views::wrap(&text, right.saturating_sub(x) as usize);
        let count = r.bottom().saturating_sub(5 + y) as usize;
        let start = view.scroll.min(lines.len().saturating_sub(count.max(1)));
        for line in lines.into_iter().skip(start).take(count) {
            put(buf, x, y, &[seg(line, style.fg(t.text))], right);
            y += 1;
        }
    }
}

fn draw_footer(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme, view: &WorkflowView) {
    let x = r.x + 2;
    let right = r.right().saturating_sub(2);
    let y = r.bottom().saturating_sub(4);
    let style = Style::default().bg(t.card);
    if let Some((_, action)) = view.confirm {
        put(
            buf,
            x,
            y,
            &[seg(
                if action == WorkflowAction::Cancel {
                    "Stop sequencing? Panes/files are kept."
                } else {
                    "Retry stage? Close its old pane first."
                },
                style.fg(t.strong),
            )],
            right,
        );
        let bx = btn(app, buf, x, y + 1, "Confirm", "Enter", BtnKind::Primary, HyHit::WorkflowKey('\n'), right) + 1;
        btn(app, buf, bx, y + 1, "Back", "Esc", BtnKind::Normal, HyHit::WorkflowKey('\u{1b}'), right);
    } else if view.busy {
        put(buf, x, y, &[seg("Working…", style.fg(t.muted))], right);
    } else if view.picker.is_some() {
        btn(app, buf, x, y, "Use profile", "Enter", BtnKind::Primary, HyHit::WorkflowKey('\n'), right);
        put(buf, x, y + 1, &hints(t, &[("↑↓", "choose"), ("Esc", "back")]), right);
    } else if view.setup.is_some() {
        btn(app, buf, x, y, "Copy instructions", "c", BtnKind::Primary, HyHit::WorkflowKey('c'), right);
        put(buf, x, y + 1, &hints(t, &[("PgUp/PgDn", "scroll"), ("Esc", "back")]), right);
    } else if view.editor.is_some() {
        let bx = btn(
            app,
            buf,
            x,
            y,
            "Save",
            "Ctrl+S",
            BtnKind::Primary,
            HyHit::WorkflowKey('s'),
            right,
        ) + 1;
        btn(app, buf, bx, y, "Back", "Esc", BtnKind::Normal, HyHit::WorkflowKey('\u{1b}'), right);
        put(
            buf,
            x,
            y + 1,
            &hints(t, &[("Tab", "field"), ("←→", "profile/scope")]),
            right,
        );
    } else if view.task.is_some() {
        btn(
            app,
            buf,
            x,
            y,
            "Start workflow",
            "Enter",
            BtnKind::Primary,
            HyHit::WorkflowKey('\n'),
            right,
        );
        put(buf, x, y + 1, &hints(t, &[("Shift+Enter", "newline"), ("Esc", "back")]), right);
    } else if view.runs {
        let selected = selected_run(app, view)
            .map(|r| (r.state, r.term.is_some() || !r.finished.is_empty()));
        let mut bx = x;
        if view.run.is_none() {
            if selected.is_some() { btn(app, buf, x, y, "Open run", "Enter", BtnKind::Primary, HyHit::WorkflowKey('\n'), right); }
        } else {
        if selected.is_some_and(|(s, _)| s == State::Approval) {
            bx = btn(
                app,
                buf,
                bx,
                y,
                "Approve plan",
                "a",
                BtnKind::Primary,
                HyHit::WorkflowKey('a'),
                right,
            ) + 1;
        }
        if selected.is_some_and(|(_, pane)| pane) {
            btn(
                app,
                buf,
                bx,
                y,
                "Open pane",
                "o",
                BtnKind::Normal,
                HyHit::WorkflowKey('o'),
                right,
            );
        }
        if selected.is_some_and(|(s, _)| matches!(s, State::Failed | State::Interrupted)) {
            btn(
                app,
                buf,
                x,
                y + 1,
                "Retry stage",
                "t",
                BtnKind::Normal,
                HyHit::WorkflowKey('t'),
                right,
            );
        } else if selected.is_some_and(|(s, _)| !s.terminal()) {
            btn(
                app,
                buf,
                x,
                y + 1,
                "Cancel run",
                "c",
                BtnKind::Normal,
                HyHit::WorkflowKey('c'),
                right,
            );
        }
        btn(app, buf, x + 22, y + 1, "Back to runs", "Esc", BtnKind::Normal, HyHit::WorkflowKey('\u{1b}'), right);
        }
    } else {
        let mut bx = x;
        for (title, key) in [("New", 'n'), ("Edit", 'e'), ("Setup", 'u')] {
            bx = btn(
                app,
                buf,
                bx,
                y,
                title,
                &key.to_string(),
                BtnKind::Normal,
                HyHit::WorkflowKey(key),
                right,
            ) + 1;
        }
        btn(
            app,
            buf,
            bx,
            y,
            "Run",
            "s",
            BtnKind::Primary,
            HyHit::WorkflowKey('s'),
            right,
        );
        let bx = btn(
            app,
            buf,
            x,
            y + 1,
            "Claude starter",
            "1",
            BtnKind::Normal,
            HyHit::WorkflowKey('1'),
            right,
        ) + 1;
        btn(
            app,
            buf,
            bx,
            y + 1,
            "Codex starter",
            "2",
            BtnKind::Normal,
            HyHit::WorkflowKey('2'),
            right,
        );
    }
    put(
        buf,
        x,
        y + 2,
        &hints(
            t,
            if view.run.is_some() { &[("Tab/←→", "report"), ("Ctrl+Tab", "section")] }
            else if view.editor.is_some() || view.task.is_some() || view.setup.is_some() { &[("Esc", "back"), ("Ctrl+Tab", "section")] }
            else { &[("Enter", "open"), ("Tab", "section"), ("Esc", "close")] },
        ),
        right,
    );
}
