//! The New session dialog on screen: opening it, its keys and clicks, the daemon's answers about
//! folders, creating the shell, and drawing the 78 by 19 box. The model is in `new_session.rs`.
//! Docs: UI.md, "New session".

use super::new_session::{
    Chip, ChipState, FolderState, Go, NewSession, RECENT_KEEP, ROOT, Row, Step, folder_name, parse_recent,
    split_folder, store_recent,
};
use super::popups::{hints, panel, sel_row};
use super::*;
use crate::protocol::{ClientMsg, FolderListing, Query};

const DIALOG_W: u16 = 78;
const DIALOG_H: u16 = 19;
/// Rows from the box's top edge to the header, the search row, the line under it and the list.
const HEADER_ROW: u16 = 1;
const SEARCH_ROW: u16 = 3;
const NOTE_ROW: u16 = 4;
const LIST_ROW: u16 = 5;
/// Rows the list always has, so the box never changes height.
const LIST_ROWS: u16 = 12;
const TITLE: &str = "NEW SESSION";
const BROWSE_LABEL: &str = "Browse…";
const BROWSE_KEY: &str = "Ctrl+O";
/// Where the folder picker opens when nothing is typed.
const PICKER_START: &str = ROOT;

/// What this computer is called in the dialog.
fn local_label() -> &'static str {
    if cfg!(target_os = "macos") {
        "This Mac"
    } else if cfg!(windows) {
        "This PC"
    } else {
        "This computer"
    }
}

impl App {
    /// The machines the dialog can start on: this computer first, then the others in sidebar order.
    pub(in crate::client) fn ns_chips(&self) -> Vec<Chip> {
        self.machines()
            .iter()
            .map(|m| {
                let state = if m.id == MachineId::LOCAL {
                    ChipState::Online
                } else {
                    match self.remotes.get(&m.id).map(|r| r.state.clone()) {
                        Some(s) if s.is_connected() => ChipState::Online,
                        Some(s) if !s.is_down() => ChipState::Connecting,
                        _ => ChipState::Offline,
                    }
                };
                Chip { id: m.id, name: m.name.to_string(), state }
            })
            .collect()
    }

    fn dialog_machine_name(&self, machine: MachineId) -> String {
        match self.remotes.get(&machine) {
            Some(r) if machine != MachineId::LOCAL => r.name.clone(),
            _ => local_label().to_string(),
        }
    }

    /// The folders offered: the ones sessions were started in, then the ones open now.
    fn ns_recents(&self) -> Vec<(MachineId, String)> {
        let chips = self.ns_chips();
        let mut out: Vec<(MachineId, String)> = Vec::new();
        let mut add = |entry: (MachineId, String)| {
            let path = entry.1.trim_end_matches('/').to_string();
            if !path.is_empty() && !path.starts_with('~') && !out.iter().any(|(m, p)| *m == entry.0 && *p == path) {
                out.push((entry.0, path));
            }
        };
        for entry in &self.screen.saved.recent_paths {
            add(parse_recent(entry, &chips));
        }
        for (term, info) in self.all_terms() {
            let dir = info.root.clone().unwrap_or_else(|| info.cwd.clone());
            add((term.machine, dir.to_string_lossy().replace('\\', "/")));
        }
        out
    }

    /// Open the dialog on `machine` (the machine of the focused session when none is given).
    pub(in crate::client) fn open_new_session(&mut self, machine: Option<MachineId>) {
        self.leave_shortcut_mode();
        self.screen.cursor = None;
        let machine = machine.unwrap_or_else(|| self.focus_machine());
        let chips = self.ns_chips();
        let machine = if chips.iter().any(|c| c.id == machine && c.state.pickable()) { machine } else { MachineId::LOCAL };
        self.mode = Mode::NewSession(Box::new(NewSession::new(machine)));
    }

    fn ns_ask_wanted(&mut self, ns: &mut NewSession) {
        if let Some((machine, folder)) = ns.wanted() {
            let tag = ns.ask(machine, folder.clone());
            self.send(machine, ClientMsg::Query(Query::ListFolders { path: folder, tag }));
        }
    }

    /// A machine answered a question about a folder.
    pub(in crate::client) fn new_session_listing(&mut self, machine: MachineId, tag: u64, listing: FolderListing) {
        if let Mode::NewSession(ns) = &mut self.mode {
            ns.arrived(machine, tag, listing);
        }
    }

    /// An error came back while folders were being asked for: stop waiting for them.
    pub(in crate::client) fn new_session_failed(&mut self) {
        if let Mode::NewSession(ns) = &mut self.mode {
            ns.give_up();
        }
    }

    pub(in crate::client) fn on_new_session_key(&mut self, mut ns: NewSession, k: &KeyEvent) {
        let chips = self.ns_chips();
        let recent = self.ns_recents();
        let rows = ns.rows(&recent);
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let word = ctrl || k.modifiers.contains(KeyModifiers::ALT);
        match k.code {
            KeyCode::Esc if ns.browsing.is_some() => {
                ns.browsing = None;
                ns.sel = 0;
            }
            KeyCode::Esc if ns.step == Step::Machine => {
                ns.step = Step::Project;
                ns.sel = 0;
            }
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Char('o') if ctrl && ns.step == Step::Project => {
                let start = if super::new_session::is_path(&ns.q) { split_folder(&ns.q).0 } else { PICKER_START.to_string() };
                ns.browsing = Some(start);
                ns.sel = 0;
            }
            KeyCode::Enter => {
                let go = ns.enter(&rows, &chips);
                self.ns_go(ns, go, &chips);
                return;
            }
            KeyCode::Up if ns.step == Step::Project => ns.step_sel(-1, rows.len()),
            KeyCode::Down if ns.step == Step::Project => ns.step_sel(1, rows.len()),
            KeyCode::Up => ns.step_machine(-1, &chips),
            KeyCode::Down => ns.step_machine(1, &chips),
            KeyCode::Tab | KeyCode::Right if ns.step == Step::Project => {
                if let Some(full) = ns.accepted() {
                    ns.q = full;
                    ns.sel = 0;
                } else if k.code == KeyCode::Tab {
                    let go = ns.enter(&rows, &chips);
                    if matches!(go, Go::Choose { .. }) {
                        self.ns_go(ns, go, &chips);
                        return;
                    }
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                ns.step = Step::Project;
                ns.sel = 0;
            }
            KeyCode::Left | KeyCode::Backspace if ns.browsing.is_some() && ns.q.is_empty() => {
                if let Some(Row::Up { to }) = rows.iter().find(|r| matches!(r, Row::Up { .. })) {
                    ns.browsing = Some(to.clone());
                    ns.sel = 0;
                }
            }
            KeyCode::Backspace if ns.step == Step::Project && word => {
                ns.q = delete_segment(&ns.q);
                ns.browsing = None;
                ns.sel = 0;
            }
            KeyCode::Backspace if ns.step == Step::Project => {
                ns.q.pop();
                ns.browsing = None;
                ns.sel = 0;
            }
            KeyCode::Char(c) if !word && ns.step == Step::Project => {
                ns.q.push(c);
                ns.browsing = None;
                ns.sel = 0;
            }
            _ => {}
        }
        // Whatever the key did, ask for the folder it now needs (the typed one, the picker's, or
        // the project on the machine just picked).
        self.ns_ask_wanted(&mut ns);
        self.mode = Mode::NewSession(Box::new(ns));
    }

    /// Pasted text goes into the query as one line.
    pub(in crate::client) fn new_session_paste(&mut self, text: &str) {
        if let Mode::NewSession(ns) = self.mode.clone() {
            let mut ns = *ns;
            if ns.step == Step::Project {
                ns.q.push_str(text.lines().next().unwrap_or("").trim());
                ns.browsing = None;
                ns.sel = 0;
                self.ns_ask_wanted(&mut ns);
            }
            self.mode = Mode::NewSession(Box::new(ns));
        }
    }

    /// A click on a row selects it; a click on the selected row does what Enter does.
    pub(in crate::client) fn new_session_row_click(&mut self, i: usize) {
        let Mode::NewSession(ns) = self.mode.clone() else { return };
        let mut ns = *ns;
        let chips = self.ns_chips();
        let rows = ns.rows(&self.ns_recents());
        let already = match ns.step {
            Step::Project => ns.sel == i,
            Step::Machine => chips.get(i).is_some_and(|c| c.id == ns.machine),
        };
        match ns.step {
            Step::Project => ns.sel = i,
            Step::Machine => {
                if let Some(c) = chips.get(i).filter(|c| c.state.pickable()) {
                    ns.machine = c.id;
                } else {
                    return;
                }
            }
        }
        if already {
            let go = ns.enter(&rows, &chips);
            self.ns_go(ns, go, &chips);
        } else {
            self.ns_ask_wanted(&mut ns);
            self.mode = Mode::NewSession(Box::new(ns));
        }
    }

    fn ns_go(&mut self, mut ns: NewSession, go: Go, chips: &[Chip]) {
        match go {
            Go::Nothing => {}
            Go::Browse(dir) | Go::Into(dir) => {
                ns.browsing = Some(dir);
                ns.sel = 0;
            }
            Go::Choose { machine, path } => {
                let several = chips.iter().filter(|c| c.state.pickable()).count() > 1;
                let up = chips.iter().any(|c| c.id == machine && c.state.pickable());
                // With one machine to start on there is no second step, so a folder remembered
                // from a machine that is down must not start a shell on it.
                if !several && !up {
                    let name = self.dialog_machine_name(machine);
                    self.notify(format!("{name} is offline: pick a folder on a machine that is up"), true);
                    self.mode = Mode::NewSession(Box::new(ns));
                    return;
                }
                ns.machine = machine;
                ns.project = Some((machine, path.clone()));
                ns.browsing = None;
                if !several {
                    self.create_shell(machine, &path);
                    return;
                }
                ns.step = Step::Machine;
                ns.sel = 0;
            }
            Go::Start { machine, path } => {
                self.create_shell(machine, &path);
                return;
            }
        }
        self.ns_ask_wanted(&mut ns);
        self.mode = Mode::NewSession(Box::new(ns));
    }

    /// The one way a session is started from the dialog and the menu: a shell in `path` on
    /// `machine`, then the terminal has the keys, the daemon focuses the new session and the
    /// sidebar scrolls it into view. The folder is remembered for the next time.
    pub(in crate::client) fn create_shell(&mut self, machine: MachineId, path: &str) {
        let chips = self.ns_chips();
        let remembered = store_recent(&chips, machine, path);
        self.screen.saved.recent_paths.retain(|p| *p != remembered);
        self.screen.saved.recent_paths.insert(0, remembered);
        self.screen.saved.recent_paths.truncate(RECENT_KEEP);
        self.screen.save();
        self.screen_new_session_on(machine, PathBuf::from(path), None, false);
        self.screen.follow = true;
        let line = format!("+ shell · {} on {}", folder_name(path), self.dialog_machine_name(machine));
        self.bar_flash = Some((line.clone(), Instant::now()));
        self.notify(line, false);
    }
}

/// Ctrl or Alt + Backspace: back over one path segment.
fn delete_segment(q: &str) -> String {
    let trimmed = q.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(i) => trimmed[..=i].to_string(),
        None => String::new(),
    }
}

// ---- drawing -------------------------------------------------------------------------------

/// A path with the home folder written `~`, for this computer's folders.
fn shown_path(machine: MachineId, path: &str) -> String {
    if machine == MachineId::LOCAL { tilde(Path::new(path)).replace('\\', "/") } else { path.to_string() }
}

pub(in crate::client) fn draw_new_session(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, ns: &NewSession) {
    let chips = app.ns_chips();
    let rows = ns.rows(&app.ns_recents());
    let machine_name = app.dialog_machine_name(ns.machine);
    let buf = f.buffer_mut();
    super::popups::dim_all(app, buf, area, t);
    let r = panel(app, buf, area, DIALOG_W, DIALOG_H, TITLE, &[], t);
    let c = Style::default().bg(t.card);
    let left = r.x + 3;
    let right = r.right().saturating_sub(3);

    // Header: Start in [project] on [machine] . opens a shell. The active step is reversed.
    let chip = |active: bool| {
        if active {
            Style::default().fg(t.card).bg(t.text).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(t.text).bg(t.btn)
        }
    };
    let project_text = match &ns.project {
        Some((_, p)) => folder_name(p).to_string(),
        None if ns.q.is_empty() => "project".to_string(),
        None => truncate(folder_name(&ns.q), 24),
    };
    put(
        buf,
        left,
        r.y + HEADER_ROW,
        &[
            seg("Start in ", c.fg(t.muted)),
            seg(format!(" {project_text} "), chip(ns.step == Step::Project)),
            seg(" on ", c.fg(t.muted)),
            seg(format!(" {machine_name} "), chip(ns.step == Step::Machine)),
            seg(" · opens a shell", c.fg(t.muted)),
        ],
        right,
    );

    // The search row (step 1) or the chosen folder (step 2).
    let search_y = r.y + SEARCH_ROW;
    let s = Style::default().bg(t.card2);
    fill(buf, Rect { x: r.x + 1, y: search_y, width: r.width - 2, height: 1 }, t.card2);
    let ghost = ns.ghost();
    let mut parts = vec![seg("⌕ ", s.fg(t.accent))];
    match (ns.step, &ns.project) {
        (Step::Machine, Some((m, path))) => parts.push(seg(shown_path(*m, path), s.fg(t.muted))),
        _ if ns.q.is_empty() && ns.browsing.is_none() => {
            parts.push(seg("project, ~/path or /path", s.fg(t.muted)));
            parts.push(seg("▏", s.fg(t.accent)));
        }
        _ => {
            parts.push(seg(ns.q.clone(), s.fg(t.strong).add_modifier(Modifier::BOLD)));
            match &ghost {
                Some(g) => parts.push(seg(g.clone(), s.fg(t.muted))),
                None => parts.push(seg("▏", s.fg(t.accent))),
            }
        }
    }
    put(buf, left, search_y, &parts, right);
    if ghost.is_some() {
        let tip = "Tab complete";
        put(buf, right.saturating_sub(tip.width() as u16), search_y, &[seg(tip, s.fg(t.muted))], r.right() - 1);
    }

    // The line under it: a folder that is not there, in red, or what is happening.
    let missing = match ns.step {
        Step::Project => ns.typed_state() == FolderState::Missing,
        Step::Machine => ns.project_state() == FolderState::Missing,
    };
    let note_y = r.y + NOTE_ROW;
    let offline = ns.step == Step::Machine
        && !chips.iter().any(|c| c.id == ns.machine && c.state.pickable());
    if offline {
        put(buf, left, note_y, &[seg(format!("✗ {machine_name} is offline: pick another machine"), c.fg(t.err))], right);
    } else if missing && ns.step == Step::Machine {
        // The project was chosen on another machine: say which folder is not here.
        let line = format!("✗ {} is not a folder on {machine_name}", ns.project.as_ref().map_or("", |(_, p)| p));
        put(buf, left, note_y, &[seg(truncate(&line, (right - left) as usize), c.fg(t.err))], right);
    } else if ns.step == Step::Machine && ns.project_state() == FolderState::Pending {
        put(buf, left, note_y, &[seg(format!("checking the folder on {machine_name}…"), c.fg(t.muted))], right);
    } else if missing {
        put(buf, left, note_y, &[seg(format!("✗ No such folder on {machine_name}"), c.fg(t.err))], right);
    } else if ns.step == Step::Project && !ns.q.is_empty() && ns.typed_state() == FolderState::Pending {
        put(buf, left, note_y, &[seg("reading…", c.fg(t.muted))], right);
    } else if let Some(dir) = &ns.browsing {
        put(buf, left, note_y, &[seg(format!("Browsing {}", shown_path(ns.machine, dir)), c.fg(t.muted))], right);
    }

    // The list: always LIST_ROWS tall.
    let first_y = r.y + LIST_ROW;
    let room = LIST_ROWS as usize;
    match ns.step {
        Step::Project => {
            let start = ns.sel.saturating_sub(room - 1);
            for (offset, (i, row)) in rows.iter().enumerate().skip(start).take(room).enumerate() {
                draw_project_row(app, buf, &r, first_y + offset as u16, row, i == ns.sel, i, ns, &chips, t);
            }
        }
        Step::Machine => {
            for (i, chip) in chips.iter().enumerate().take(room) {
                draw_machine_row(app, buf, &r, first_y + i as u16, chip, chip.id == ns.machine, i, t);
            }
        }
    }

    // The bottom border carries the keys.
    let enter = if ns.step == Step::Machine { "start" } else { "pick" };
    let back = if ns.step == Step::Machine || ns.browsing.is_some() { "back" } else { "close" };
    let bar = hints(t, &[("↑ ↓", "move"), ("Enter", enter), ("Tab", "next"), ("Esc", back)]);
    put(buf, r.x + 2, r.bottom() - 1, &bar, r.right().saturating_sub(2));
}

#[allow(clippy::too_many_arguments)]
fn draw_project_row(
    app: &mut App,
    buf: &mut Buffer,
    r: &Rect,
    y: u16,
    row: &Row,
    selected: bool,
    index: usize,
    ns: &NewSession,
    chips: &[Chip],
    t: &Theme,
) {
    let left = r.x + 4;
    let right = r.right().saturating_sub(3);
    let bg = sel_row(app, buf, *r, y, selected, t);
    let s = Style::default().bg(bg);
    let name_style = s.fg(if selected { t.strong } else { t.text }).add_modifier(Modifier::BOLD);
    match row {
        Row::Browse => {
            put(buf, left, y, &[seg("▸ ", s.fg(t.accent)), seg(BROWSE_LABEL, name_style)], right);
            put(buf, right.saturating_sub(BROWSE_KEY.width() as u16), y, &[seg(BROWSE_KEY, s.fg(t.muted))], right);
        }
        Row::Recent { machine, path } => {
            let place = machine_mark(chips, *machine);
            let place_w = place.width() as u16;
            put(buf, left, y, &[seg(folder_name(path).to_string(), name_style)], right.saturating_sub(place_w + 2));
            let at = left + 24;
            put(buf, at, y, &[seg(shown_path(*machine, path), s.fg(t.muted))], right.saturating_sub(place_w + 2));
            put(buf, right.saturating_sub(place_w), y, &[seg(place, s.fg(t.muted))], right);
        }
        Row::Typed { path, state } => {
            let (mark, color) = match state {
                FolderState::Exists => ("✓ ", t.done),
                FolderState::Missing => ("✗ ", t.err),
                FolderState::Pending | FolderState::Incomplete => ("… ", t.muted),
            };
            put(buf, left, y, &[seg(mark, s.fg(color)), seg(shown_path(ns.machine, path), name_style)], right);
        }
        Row::Child { name, git, has_children, .. } => {
            let glyph = if *has_children { "▸ " } else { "· " };
            put(buf, left, y, &[seg(glyph, s.fg(t.muted)), seg(name.clone(), name_style)], right.saturating_sub(24));
            if let Some(b) = git {
                let tag = format!("⎇ {}", truncate(b, 20));
                put(buf, right.saturating_sub(tag.width() as u16), y, &[seg(tag, s.fg(t.muted))], right);
            }
        }
        Row::UseHere { path } => {
            put(buf, left, y, &[seg("✓ ", s.fg(t.done)), seg(format!("Use {}", shown_path(ns.machine, path)), name_style)], right);
        }
        Row::Up { .. } => {
            put(buf, left, y, &[seg("↑ ", s.fg(t.muted)), seg("..", name_style)], right);
        }
    }
    hit(app, Rect { x: r.x + 1, y, width: r.width - 2, height: 1 }, ScreenHit::NsRow(index));
}

/// Where a folder is, at the right of its row: `local` or `⇄ name`.
fn machine_mark(chips: &[Chip], machine: MachineId) -> String {
    match chips.iter().find(|c| c.id == machine) {
        Some(c) if !c.name.is_empty() => format!("⇄ {}", c.name),
        _ => "local".to_string(),
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_machine_row(app: &mut App, buf: &mut Buffer, r: &Rect, y: u16, chip: &Chip, selected: bool, index: usize, t: &Theme) {
    let left = r.x + 4;
    let right = r.right().saturating_sub(3);
    let bg = sel_row(app, buf, *r, y, selected, t);
    let s = Style::default().bg(bg);
    let (mark, name) = if chip.name.is_empty() { ("⌂ ", local_label().to_string()) } else { ("⇄ ", chip.name.clone()) };
    let (note, ink) = match chip.state {
        ChipState::Online => ("connected", t.muted),
        ChipState::Connecting => ("connecting…", t.accent),
        ChipState::Offline => ("reconnecting", t.muted),
    };
    let dim = !chip.state.pickable();
    let name_style = if dim {
        s.fg(t.muted)
    } else {
        s.fg(if selected { t.strong } else { t.text }).add_modifier(Modifier::BOLD)
    };
    let mark = if dim { "┆ " } else { mark };
    put(buf, left, y, &[seg(mark, s.fg(if dim { t.muted } else { t.accent })), seg(name, name_style)], right);
    put(buf, right.saturating_sub(note.width() as u16), y, &[seg(note, s.fg(ink))], right);
    if chip.state.pickable() {
        hit(app, Rect { x: r.x + 1, y, width: r.width - 2, height: 1 }, ScreenHit::NsRow(index));
    }
}
