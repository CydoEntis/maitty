//! Profile setup, preview, and choosing a role for a fresh session.

use super::design::{BtnKind, fill, put, seg};
use super::hydra::{HyHit, btn, hints, hit};
use super::*;
use crate::profiles::{Profile, Resolved, Runner};
use ratatui::{buffer::Buffer, layout::Rect, style::Style};

const EDIT_FIELDS: usize = 7;
const INSTRUCTIONS_FIELD: usize = 6;
const PROFILE_PREFIX: &str = "◇ ";

#[derive(Debug, Clone, PartialEq)]
pub(super) struct Editor {
    pub original: Option<String>,
    pub profile: Profile,
    pub skills: String,
    pub project: bool,
    pub field: usize,
}

impl Editor {
    fn new(profile: Profile, project: bool, original: Option<String>) -> Self {
        Self {
            skills: profile.skills.join(", "),
            profile,
            project,
            original,
            field: 0,
        }
    }

    pub fn input(&mut self) -> Option<&mut String> {
        match self.field {
            0 => Some(&mut self.profile.name),
            2 => Some(&mut self.profile.model),
            3 => Some(&mut self.profile.effort),
            4 => Some(&mut self.skills),
            INSTRUCTIONS_FIELD => Some(&mut self.profile.instructions),
            _ => None,
        }
    }

    fn value(&self, field: usize) -> String {
        match field {
            0 => self.profile.name.clone(),
            1 => self.profile.agent.command().into(),
            2 => self.profile.model.clone(),
            3 => self.profile.effort.clone(),
            4 => self.skills.clone(),
            5 => if self.project {
                "this project"
            } else {
                "personal"
            }
            .into(),
            _ => self.profile.instructions.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ProfilesView {
    pub dir: PathBuf,
    pub entries: Option<Vec<Resolved>>,
    pub error: Option<String>,
    pub sel: usize,
    pub scroll: usize,
    pub editor: Option<Editor>,
    pub busy: bool,
    pub import_path: Option<String>,
}

pub(super) fn label(profile: &Profile) -> String {
    format!("{PROFILE_PREFIX}{}", profile.name)
}

impl App {
    pub(super) fn profile_dir(&self, project: usize) -> Option<PathBuf> {
        let model = self.hy_model();
        let p = model.get(project)?;
        if let Mode::Profiles(view) = &self.mode
            && (p.path == view.dir || p.wts.iter().any(|w| w.path == view.dir))
        {
            return Some(view.dir.clone());
        }
        if let Mode::HyPane(np) = &self.mode
            && np.p == project
            && let Some(place) = np.place.filter(|place| *place >= 3)
            && let Some(w) = p.wts.iter().filter(|w| !w.main).nth(place as usize - 3)
        {
            return Some(w.path.clone());
        }
        Some(
            p.wts
                .iter()
                .find(|w| w.main)
                .map(|w| w.path.clone())
                .unwrap_or_else(|| p.path.clone()),
        )
    }

    pub(super) fn profiles_for(&self, project: usize) -> Result<Vec<Resolved>, String> {
        let dir = self.profile_dir(project);
        let local = match dir.as_ref().and_then(|d| self.profile_projects.get(d)) {
            Some(Ok(profiles)) => profiles.as_slice(),
            Some(Err(e)) => return Err(e.clone()),
            None => &[],
        };
        crate::profiles::resolve(&self.cfg.profiles, local).map_err(|e| format!("{e:#}"))
    }

    pub(super) fn load_profile_project(&mut self, project: usize) {
        if let Some(dir) = self.profile_dir(project) {
            self.refresh_profiles(dir, false);
        }
    }

    fn refresh_profiles(&mut self, dir: PathBuf, reload_config: bool) {
        let cfg = self.cfg.clone();
        self.spawn_bg(move || {
            let cfg = if reload_config {
                Config::load()
            } else {
                Ok(cfg)
            };
            let local = crate::profiles::project_profiles(&dir).map_err(|e| format!("{e:#}"));
            let resolved = cfg.as_ref().map_err(|e| format!("{e:#}")).and_then(|cfg| {
                local.as_ref().map_err(Clone::clone).and_then(|local| {
                    crate::profiles::resolve(&cfg.profiles, local).map_err(|e| format!("{e:#}"))
                })
            });
            Bg::Then(Box::new(move |app| {
                // Preserve the chosen role when the project list arrives or changes.
                let selected = if let Mode::HyPane(np) = &app.mode {
                    super::hydra::np_agents(app, np.p).get(np.a).cloned()
                } else {
                    None
                };
                if let Ok(cfg) = cfg {
                    app.cfg.profiles = cfg.profiles;
                }
                app.profile_projects.insert(dir.clone(), local);
                if let Mode::HyPane(np) = &app.mode {
                    let p = np.p;
                    let names = super::hydra::np_agents(app, p);
                    if let Mode::HyPane(np) = &mut app.mode {
                        np.a = selected
                            .and_then(|s| names.iter().position(|n| n == &s))
                            .unwrap_or(0);
                    }
                }
                if let Mode::Profiles(view) = &mut app.mode
                    && view.dir == dir
                {
                    view.busy = false;
                    match resolved {
                        Ok(entries) => {
                            view.sel = view.sel.min(entries.len().saturating_sub(1));
                            view.entries = Some(entries);
                            view.error = None;
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

    pub(super) fn open_profiles(&mut self) {
        if crate::ipc::remote().is_some() {
            self.notify(
                "Agent profile setup isn't available over SSH yet.".into(),
                true,
            );
            return;
        }
        let project = if let Mode::Workflows(v) = &self.mode {
            Some(v.dir.clone())
        } else if let Mode::HyPane(np) = &self.mode {
            self.hy.saved.draft = np.task.clone();
            self.profile_dir(np.p)
        } else {
            Some(self.hy_target_dir())
        };
        let dir = project.unwrap_or_else(|| self.here_dir());
        self.mode = Mode::Profiles(Box::new(ProfilesView {
            dir: dir.clone(),
            entries: None,
            error: None,
            sel: 0,
            scroll: 0,
            editor: None,
            busy: false,
            import_path: None,
        }));
        self.refresh_profiles(dir, false);
    }

    fn save_profile_editor(&mut self, view: &mut ProfilesView) {
        let Some(mut editor) = view.editor.clone() else {
            return;
        };
        editor.profile.skills = editor
            .skills
            .split(',')
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .map(String::from)
            .collect();
        if let Err(e) = editor.profile.validate() {
            view.error = Some(e.to_string());
            return;
        }
        let dir = view.dir.clone();
        view.busy = true;
        self.spawn_bg(move || {
            let result = (|| {
                let path = if editor.project {
                    dir.join(crate::project::FILE)
                } else {
                    crate::config::profile_path()?
                };
                crate::profiles::save(&path, editor.original.as_deref(), &editor.profile)
            })();
            Bg::Then(Box::new(move |app| {
                let mut reload = false;
                if let Mode::Profiles(view) = &mut app.mode
                    && view.dir == dir
                {
                    view.busy = false;
                    match result {
                        Ok(()) => {
                            view.editor = None;
                            view.error = None;
                            reload = true;
                        }
                        Err(e) => view.error = Some(format!("{e:#}")),
                    }
                }
                if reload {
                    app.refresh_profiles(dir, true);
                    app.notify(
                        "Profile saved. Export it to make it available to native workflows.".into(),
                        false,
                    );
                }
            }))
        });
    }

    fn export_profile(&mut self, view: &mut ProfilesView) {
        let Some(entry) = view.entries.as_ref().and_then(|e| e.get(view.sel)).cloned() else {
            return;
        };
        view.busy = true;
        let dir = view.dir.clone();
        self.spawn_bg(move || {
            let result = if entry.project {
                crate::profiles::export(&entry.profile, &dir)
            } else {
                crate::profiles::export_personal(&entry.profile)
            };
            Bg::Then(Box::new(move |app| {
                if let Mode::Profiles(view) = &mut app.mode
                    && view.dir == dir
                {
                    view.busy = false;
                }
                match result {
                    Ok(path) => app.notify(format!("Exported {}", path.display()), false),
                    Err(e) => app.notify(format!("{e:#}"), true),
                }
            }))
        });
    }

    pub(super) fn on_profiles_key(&mut self, mut view: ProfilesView, k: &KeyEvent) {
        if view.busy {
            self.mode = Mode::Profiles(Box::new(view));
            return;
        }
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        if let Some(path) = &mut view.import_path {
            match k.code {
                KeyCode::Esc => view.import_path = None,
                KeyCode::Backspace => {
                    path.pop();
                }
                KeyCode::Char(c) if !ctrl => path.push(c),
                KeyCode::Enter => {
                    let path = PathBuf::from(path.trim());
                    view.busy = true;
                    let dir = view.dir.clone();
                    self.spawn_bg(move || {
                        let result = crate::profiles::import(&path);
                        Bg::Then(Box::new(move |app| {
                            if let Mode::Profiles(view) = &mut app.mode
                                && view.dir == dir
                            {
                                view.busy = false;
                                match result {
                                    Ok(profile) => {
                                        view.import_path = None;
                                        view.editor = Some(Editor::new(profile, false, None));
                                        view.error = None;
                                    }
                                    Err(e) => view.error = Some(format!("{e:#}")),
                                }
                            }
                        }))
                    });
                }
                _ => {}
            }
        } else if let Some(editor) = &mut view.editor {
            match k.code {
                KeyCode::Esc => {
                    view.editor = None;
                    view.error = None;
                }
                KeyCode::Char('s') if ctrl => self.save_profile_editor(&mut view),
                KeyCode::Tab | KeyCode::Down => editor.field = (editor.field + 1) % EDIT_FIELDS,
                KeyCode::BackTab | KeyCode::Up => {
                    editor.field = (editor.field + EDIT_FIELDS - 1) % EDIT_FIELDS
                }
                KeyCode::Left | KeyCode::Right | KeyCode::Char(' ')
                    if matches!(editor.field, 1 | 5) =>
                {
                    if editor.field == 1 {
                        editor.profile.agent = if editor.profile.agent == Runner::Claude {
                            Runner::Codex
                        } else {
                            Runner::Claude
                        };
                    } else if editor.original.is_none() {
                        editor.project = !editor.project;
                    }
                }
                KeyCode::Enter if editor.field == INSTRUCTIONS_FIELD => {
                    editor.profile.instructions.push('\n')
                }
                KeyCode::Backspace => {
                    if let Some(input) = editor.input() {
                        input.pop();
                    }
                }
                KeyCode::Char('u') if ctrl => {
                    if let Some(input) = editor.input() {
                        input.clear();
                    }
                }
                KeyCode::Char(c) if !ctrl => {
                    if let Some(input) = editor.input() {
                        input.push(c);
                    }
                }
                _ => {}
            }
        } else {
            let entries = view.entries.as_deref().unwrap_or_default();
            match k.code {
                KeyCode::Esc => {
                    self.mode = Mode::Normal;
                    return;
                }
                KeyCode::Down => {
                    view.sel = (view.sel + 1).min(entries.len().saturating_sub(1));
                    view.scroll = 0;
                }
                KeyCode::Up => {
                    view.sel = view.sel.saturating_sub(1);
                    view.scroll = 0;
                }
                KeyCode::PageDown => view.scroll = view.scroll.saturating_add(3),
                KeyCode::PageUp => view.scroll = view.scroll.saturating_sub(3),
                KeyCode::Char('n') => {
                    view.editor = Some(Editor::new(Profile::default(), false, None))
                }
                KeyCode::Char('w') => { self.open_workflows_at(view.dir, false); return; }
                KeyCode::Char('r') if !ctrl => { self.open_workflows_at(view.dir, true); return; }
                KeyCode::Char('i') => view.import_path = Some(String::new()),
                KeyCode::Char(c @ '1'..='6') => {
                    let i = (c as u8 - b'1') as usize;
                    view.editor = Some(Editor::new(
                        crate::profiles::starter(
                            if i < 3 { Runner::Claude } else { Runner::Codex },
                            i % 3,
                        ),
                        false,
                        None,
                    ));
                }
                KeyCode::Char('e' | 'p') => {
                    if let Some(entry) = entries.get(view.sel) {
                        let project = entry.project || k.code == KeyCode::Char('p');
                        let original = if project == entry.project {
                            Some(entry.profile.name.clone())
                        } else {
                            None
                        };
                        view.editor = Some(Editor::new(entry.profile.clone(), project, original));
                    }
                }
                KeyCode::Char('x') => self.export_profile(&mut view),
                KeyCode::Char('r') if ctrl => {
                    let dir = view.dir.clone();
                    self.mode = Mode::Profiles(Box::new(view));
                    self.refresh_profiles(dir, true);
                    return;
                }
                KeyCode::Enter => {
                    if let Some(entry) = entries.get(view.sel) {
                        let model = self.hy_model();
                        let p = model.iter().position(|p| {
                            p.path == view.dir || p.wts.iter().any(|w| w.path == view.dir)
                        });
                        if let Some(p) = p {
                            let mut np = super::hydra::NewPaneHy::new(p, false);
                            np.task = self.hy.saved.draft.clone();
                            if let Some(w) = model[p]
                                .wts
                                .iter()
                                .filter(|w| !w.main)
                                .position(|w| w.path == view.dir)
                            {
                                np.place = Some((w + 3) as u8);
                            }
                            self.mode = Mode::HyPane(np);
                            let a = super::hydra::np_agents(self, p)
                                .iter()
                                .position(|a| *a == label(&entry.profile))
                                .unwrap_or(0);
                            if let Mode::HyPane(np) = &mut self.mode {
                                np.a = a;
                            }
                            return;
                        }
                        view.error =
                            Some("Open a folder in Seshi before launching a profile.".into());
                    }
                }
                _ => {}
            }
        }
        self.mode = Mode::Profiles(Box::new(view));
    }
}

pub(super) fn launch_command(
    cfg: &Config,
    profile: &Profile,
    task: &str,
) -> anyhow::Result<String> {
    let exe = std::env::current_exe()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "seshi".into());
    let shell = cfg.shell_command();
    let powershell = shell.first().is_some_and(|s| {
        std::path::Path::new(s)
            .file_stem()
            .is_some_and(|s| s.eq_ignore_ascii_case("pwsh") || s.eq_ignore_ascii_case("powershell"))
    });
    let encoded = crate::profiles::encode_task(task)?;
    let task = if encoded.is_empty() {
        String::new()
    } else {
        format!(" --encoded-task {encoded}")
    };
    Ok(format!(
        "{}{} profile run {}{task}",
        if powershell { "& " } else { "" },
        cfg.quote_for_shell(&exe),
        cfg.quote_for_shell(&profile.name)
    ))
}

pub(super) fn selected(app: &App, project: usize, run: &str) -> Option<Profile> {
    let name = run.strip_prefix(PROFILE_PREFIX)?;
    app.profiles_for(project)
        .ok()?
        .into_iter()
        .find(|p| p.profile.name == name)
        .map(|p| p.profile)
}

pub(super) fn draw_profiles(
    app: &mut App,
    buf: &mut Buffer,
    r: Rect,
    t: &Theme,
    view: &ProfilesView,
) {
    let y = if let Some(editor) = &view.editor {
        draw_profile_editor(app, buf, r, t, editor)
    } else if let Some(path) = &view.import_path {
        put(
            buf,
            r.x + 3,
            r.y + 3,
            &[seg(
                "Native agent file to import (.md / .toml)",
                Style::default().bg(t.card).fg(t.strong),
            )],
            r.right().saturating_sub(3),
        );
        put(
            buf,
            r.x + 3,
            r.y + 4,
            &[seg(
                format!("{path}█"),
                Style::default().bg(t.card).fg(t.accent),
            )],
            r.right().saturating_sub(3),
        );
        r.y + 6
    } else {
        let y = draw_profile_rows(app, buf, r, t, view);
        draw_profile_starters(app, buf, r, t, y)
    };
    if let Some(error) = &view.error {
        for (i, line) in super::views::wrap(error, r.width.saturating_sub(6) as usize)
            .into_iter()
            .take(2)
            .enumerate()
        {
            put(
                buf,
                r.x + 3,
                y.min(r.bottom().saturating_sub(5)) + i as u16,
                &[seg(line, Style::default().bg(t.card).fg(t.err))],
                r.right().saturating_sub(3),
            );
        }
    }
    draw_profile_footer(app, buf, r, t, view);
}

fn draw_profile_editor(
    app: &mut App,
    buf: &mut ratatui::buffer::Buffer,
    r: Rect,
    t: &Theme,
    editor: &Editor,
) -> u16 {
    let mut y = r.y + 3;
    let stride = if r.height >= 25 { 2 } else { 1 };
    for (field, title) in [
        "NAME",
        "AGENT",
        "MODEL",
        "EFFORT",
        "SKILLS",
        "SCOPE",
        "INSTRUCTIONS",
    ]
    .iter()
    .enumerate()
    {
        let on = field == editor.field;
        let row = Rect {
            x: r.x + 2,
            y,
            width: r.width.saturating_sub(4),
            height: 1,
        };
        let background = if on { t.card2 } else { t.card };
        fill(buf, row, background);
        let st = Style::default().bg(background);
        let text = editor.value(field).replace('\n', " ↵ ");
        let room = r.width.saturating_sub(20) as usize;
        let text = if on && text.chars().count() > room {
            text.chars()
                .skip(text.chars().count().saturating_sub(room))
                .collect()
        } else {
            text
        };
        put(
            buf,
            r.x + 3,
            y,
            &[
                seg(
                    format!("{title:<13}"),
                    st.fg(if on { t.accent } else { t.muted }),
                ),
                seg(text, st.fg(t.strong)),
                seg(if on { "█" } else { "" }, st.fg(t.accent)),
            ],
            r.right().saturating_sub(3),
        );
        hit(app, row, HyHit::ProfileField(field));
        y += stride;
    }
    for line in [
        "Blank model/effort uses the agent default.",
        "Skills: installed names, separated by commas.",
    ] {
        put(
            buf,
            r.x + 3,
            y,
            &[seg(line, Style::default().bg(t.card).fg(t.muted))],
            r.right().saturating_sub(3),
        );
        y += 1;
    }
    y
}

fn draw_profile_rows(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme, view: &ProfilesView) -> u16 {
    let st = Style::default().bg(t.card);
    let right = r.right().saturating_sub(2);
    for (i, line) in ["Saved roles for fresh agent sessions.", "Enter chooses a task; this session stays as is."].iter().enumerate() {
        put(buf, r.x + 2, r.y + 3 + i as u16, &[seg(*line, st.fg(t.muted))], right);
    }
    let mut y = r.y + 6;
    let entries = view.entries.as_deref().unwrap_or_default();
    let visible = (r.height.saturating_sub(19) as usize).clamp(1, 5);
    let start = view.sel.saturating_sub(visible - 1);
    for (i, entry) in entries.iter().enumerate().skip(start).take(visible) {
        let row = Rect { x: r.x + 2, y, width: r.width.saturating_sub(4), height: 1 };
        let rs = super::hydra::list_row(app, buf, row, i == view.sel, t);
        put(buf, r.x + 4, y, &[seg(format!("{} ? {}", entry.profile.name, if entry.project { "project" } else { "personal" }), rs.fg(t.strong))], right);
        hit(app, row, HyHit::ProfileRow(i)); y += 1;
    }
    if entries.is_empty() {
        put(buf, r.x + 2, y, &[seg(if view.entries.is_none() { "Loading profiles?" } else { "Choose a starter below or press n." }, st.fg(t.muted))], right); y += 1;
    }
    if let Some(entry) = entries.get(view.sel) {
        y += 1;
        put(buf, r.x + 2, y, &[seg(entry.profile.summary(), st.fg(t.strong))], right); y += 1;
        let lines = super::views::wrap(&entry.profile.instructions_for_runner(), r.width.saturating_sub(4) as usize);
        let start = view.scroll.min(lines.len().saturating_sub(2));
        for line in lines.into_iter().skip(start).take(2) {
            put(buf, r.x + 2, y, &[seg(line, st.fg(t.text))], right); y += 1;
        }
    }
    y + 1
}

fn draw_profile_starters(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme, mut y: u16) -> u16 {
    for (runner, offset) in [("Claude", 0), ("Codex", 3)] {
        put(buf, r.x + 2, y, &[seg(runner, Style::default().bg(t.card).fg(t.muted))], r.right().saturating_sub(2));
        let mut x = r.x + 9;
        for (i, role) in ["Planner", "Worker", "Reviewer"].iter().enumerate() {
            let key = char::from(b'1' + (offset + i) as u8);
            x = btn(app, buf, x, y, role, &key.to_string(), BtnKind::Normal, HyHit::ProfileKey(key), r.right().saturating_sub(2)) + 1;
        }
        y += 1;
    }
    y
}

fn draw_profile_footer(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme, view: &ProfilesView) {
    let y = r.bottom().saturating_sub(4);
    let right = r.right().saturating_sub(2);
    if view.busy {
        put(buf, r.x + 2, y, &[seg("Working?", Style::default().bg(t.card).fg(t.muted))], right);
    } else if view.editor.is_some() {
        btn(app, buf, r.x + 2, y, "Save", "Ctrl+S", BtnKind::Primary, HyHit::ProfileKey('s'), right);
        put(buf, r.x + 2, y + 1, &hints(t, &[("Tab", "field"), ("??", "choose"), ("Enter", "newline")]), right);
    } else if view.import_path.is_some() {
        put(buf, r.x + 2, y, &hints(t, &[("Enter", "import"), ("Esc", "cancel")]), right);
    } else {
        let mut x = r.x + 2;
        for (title, key) in [("New", 'n'), ("Edit", 'e'), ("Project", 'p')] {
            x = btn(app, buf, x, y, title, &key.to_string(), BtnKind::Normal, HyHit::ProfileKey(key), right) + 1;
        }
        let mut x = r.x + 2;
        for (title, key) in [("Export", 'x'), ("Import", 'i'), ("Launch", '\n')] {
            x = btn(app, buf, x, y + 1, title, if key == '\n' { "Enter" } else if key == 'x' { "x" } else { "i" }, BtnKind::Normal, HyHit::ProfileKey(key), right) + 1;
        }
    }
    put(buf, r.x + 2, y + 2, &hints(t, &[("??", "choose"), ("PgUp/Dn", "details"), ("Esc", "close")]), right);
}
