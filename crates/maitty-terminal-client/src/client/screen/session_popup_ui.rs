//! The New Session popup on screen: opening it, its keys and clicks, the answers to its questions
//! about other machines' folders, and how it is drawn. The model is in `session_popup.rs`.

use super::popups::{dim_all, panel, panel_bar, sel_row};
use super::session_popup::{
    Chip, ChipState, Go, Listing, NewSession, ProjectRow, Row, Scope, delete_segment, ghost, host_of, scope_of,
};
use super::*;
use crate::protocol::{ClientMsg, FolderListing, Query};

const POPUP_W: u16 = 92;
const POPUP_H: u16 = 28;
/// Rows from the popup's top edge to the first list row, and from its bottom edge up to the last.
const LIST_TOP: u16 = 7;
const LIST_BOTTOM_GAP: u16 = 6;
/// The branch is cut at this many characters.
const BRANCH_MAX: usize = 20;
/// The title chip's text.
const TITLE: &str = "NEW SESSION";

/// The glyphs the popup draws whose width differs between terminals, and their plain-ASCII twins.
struct Marks {
    prompt: &'static str,
    cursor: &'static str,
    find: &'static str,
    start: &'static str,
    project: &'static str,
    needs: &'static str,
    folder: &'static str,
    leaf: &'static str,
    local: &'static str,
    remote: &'static str,
    branch: &'static str,
    ellipsis: &'static str,
    rule: &'static str,
    online: &'static str,
    connecting: &'static str,
    offline: &'static str,
    switch: &'static str,
    move_keys: &'static str,
}

const UNICODE_MARKS: Marks = Marks {
    prompt: "❯ ",
    cursor: "█",
    find: "⌕ ",
    start: "▶ ",
    project: "◆",
    needs: "●",
    folder: "▸",
    leaf: "·",
    local: "⌂",
    remote: "⇄",
    branch: "⎇ ",
    ellipsis: "…",
    rule: "─",
    online: "●",
    connecting: "◌",
    offline: "○",
    switch: "← →  switch machine",
    move_keys: "↑↓",
};

const ASCII_MARKS: Marks = Marks {
    prompt: "> ",
    cursor: "_",
    find: "",
    start: "> ",
    project: "*",
    needs: "!",
    folder: ">",
    leaf: "-",
    local: "~",
    remote: "@",
    branch: "git:",
    ellipsis: "...",
    rule: "-",
    online: "*",
    connecting: "~",
    offline: "o",
    switch: "Left/Right  switch machine",
    move_keys: "Up/Down",
};

/// ASCII marks when `[ui] glyphs` says `ascii`, or `MAITTY_ASCII` is set. The setting's own
/// choices (`dots`, `unicode`) are both Unicode, and a saved `ascii` is rewritten to `dots` on load,
/// so the variable is how a terminal that draws these glyphs at the wrong width asks for plain ones.
fn marks(app: &App) -> &'static Marks {
    if app.cfg.ui.glyphs == "ascii" || std::env::var_os("MAITTY_ASCII").is_some_and(|v| !v.is_empty()) {
        &ASCII_MARKS
    } else {
        &UNICODE_MARKS
    }
}

impl App {
    /// The machines the popup can start on: this computer first, then the others in sidebar order.
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
                Chip {
                    id: m.id,
                    name: m.name.to_string(),
                    state,
                }
            })
            .collect()
    }

    /// The open projects of every machine, this computer's first.
    fn ns_projects(&self) -> Vec<ProjectRow> {
        let mut out: Vec<ProjectRow> = self
            .screen_model()
            .iter()
            .map(|p| ProjectRow {
                machine: p.machine,
                path: if p.machine == MachineId::LOCAL {
                    // This computer's home folder is written `~`, as it is typed.
                    tilde(&p.path).replace('\\', "/")
                } else {
                    p.path.to_string_lossy().replace('\\', "/")
                },
                branch: p
                    .wts
                    .iter()
                    .find(|w| w.main)
                    .map(|w| w.branch.clone())
                    .filter(|b| !b.is_empty()),
                sessions: p.sessions().count(),
                needs: p.sessions().any(|s| s.status == Status::Blocked),
            })
            .collect();
        out.sort_by_key(|p| p.machine);
        out
    }

    /// Open the popup with `prefill` already typed.
    pub(in crate::client) fn screen_open_session_popup(&mut self, prefill: &str) {
        self.leave_shortcut_mode();
        self.screen.cursor = None;
        let mut ns = NewSession::new(prefill);
        let chips = self.ns_chips();
        let projects = self.ns_projects();
        for (machine, folder) in NewSession::index_folders(&chips, &projects) {
            self.ns_ask(&mut ns, machine, folder);
        }
        self.ns_ask_wanted(&mut ns, &chips);
        self.mode = Mode::NewSession(Box::new(ns));
    }

    fn ns_ask(&mut self, ns: &mut NewSession, machine: MachineId, folder: String) {
        if ns.cache.contains_key(&(machine, folder.clone())) {
            return;
        }
        let tag = ns.ask(machine, folder.clone());
        self.send(
            machine,
            ClientMsg::Query(Query::ListFolders { path: folder, tag }),
        );
    }

    fn ns_ask_wanted(&mut self, ns: &mut NewSession, chips: &[Chip]) {
        if let Some((machine, folder)) = ns.wanted(chips) {
            self.ns_ask(ns, machine, folder);
        }
    }

    /// A machine answered a question about a folder.
    pub(in crate::client) fn session_popup_listing(
        &mut self,
        machine: MachineId,
        tag: u64,
        listing: FolderListing,
    ) {
        if let Mode::NewSession(ns) = &mut self.mode {
            ns.arrived(machine, tag, listing);
        }
    }

    /// An error came back while folders were being asked for: stop waiting for them.
    pub(in crate::client) fn session_popup_failed(&mut self) {
        if let Mode::NewSession(ns) = &mut self.mode {
            let waiting: Vec<_> = ns.asked.values().cloned().collect();
            ns.asked.clear();
            for key in waiting {
                let missing = FolderListing::missing(&key.1);
                ns.cache.insert(key, Listing::Ready(missing));
            }
        }
    }

    pub(in crate::client) fn on_session_popup_key(&mut self, mut ns: NewSession, k: &KeyEvent) {
        let chips = self.ns_chips();
        let projects = self.ns_projects();
        let recent = self.screen.saved.recent_paths.clone();
        let rows = ns.rows(&chips, &projects, &recent);
        let word = k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT);
        let mut edited = false;
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Enter => {
                let go = ns.enter(&rows, &chips);
                self.ns_go(ns, go, &chips);
                return;
            }
            KeyCode::Up => ns.step(&rows, false),
            KeyCode::Down => ns.step(&rows, true),
            KeyCode::Left => ns.step_chip(&chips, false),
            KeyCode::Right => {
                let on_row = matches!(ns.selected(&rows), Some((_, r)) if !matches!(r, Row::StartHere { .. }));
                match ns.tab(&rows).filter(|_| on_row) {
                    Some(fill) => {
                        ns.q = fill;
                        edited = true;
                    }
                    None => ns.step_chip(&chips, true),
                }
            }
            KeyCode::Tab => {
                if let Some(fill) = ns.tab(&rows) {
                    ns.q = fill;
                    edited = true;
                }
            }
            KeyCode::Backspace if word => {
                ns.q = delete_segment(&ns.q);
                edited = true;
            }
            KeyCode::Backspace => {
                ns.q.pop();
                edited = true;
            }
            KeyCode::Char(c) if !word => {
                ns.q.push(c);
                edited = true;
            }
            _ => {}
        }
        if edited {
            ns.sel = None;
            ns.chip = None;
            self.ns_ask_wanted(&mut ns, &chips);
        }
        self.mode = Mode::NewSession(Box::new(ns));
    }

    /// Pasted text goes into the query as one line.
    pub(in crate::client) fn session_popup_paste(&mut self, text: &str) {
        let chips = self.ns_chips();
        if let Mode::NewSession(ns) = self.mode.clone() {
            let mut ns = *ns;
            ns.q.push_str(text.lines().next().unwrap_or("").trim());
            ns.sel = None;
            ns.chip = None;
            self.ns_ask_wanted(&mut ns, &chips);
            self.mode = Mode::NewSession(Box::new(ns));
        }
    }

    /// A click on a list row selects it; a click on the selected row does what Enter does.
    pub(in crate::client) fn session_popup_row_click(&mut self, i: usize) {
        let Mode::NewSession(ns) = self.mode.clone() else { return };
        let mut ns = *ns;
        let chips = self.ns_chips();
        let rows = ns.rows(&chips, &self.ns_projects(), &self.screen.saved.recent_paths.clone());
        let already = ns.selected(&rows).is_some_and(|(sel, _)| sel == i);
        ns.chip = None;
        ns.sel = Some(i);
        if already {
            let go = ns.enter(&rows, &chips);
            self.ns_go(ns, go, &chips);
        } else {
            self.mode = Mode::NewSession(Box::new(ns));
        }
    }

    pub(in crate::client) fn session_popup_chip_click(&mut self, i: usize) {
        let Mode::NewSession(ns) = self.mode.clone() else { return };
        let mut ns = *ns;
        let chips = self.ns_chips();
        ns.chip = Some(i);
        let go = ns.enter(&[], &chips);
        self.ns_go(ns, go, &chips);
    }

    fn ns_go(&mut self, mut ns: NewSession, go: Go, chips: &[Chip]) {
        match go {
            Go::Nothing => self.mode = Mode::NewSession(Box::new(ns)),
            Go::Fill(path) => {
                ns.q = path;
                ns.sel = None;
                ns.chip = None;
                self.ns_ask_wanted(&mut ns, chips);
                self.mode = Mode::NewSession(Box::new(ns));
            }
            Go::Reconnect(machine) => {
                let name = chips.iter().find(|c| c.id == machine).map(|c| c.name.clone()).unwrap_or_default();
                let notice = self.machine_retry(&name);
                self.notify(notice.text, false);
                self.mode = Mode::NewSession(Box::new(ns));
            }
            Go::Start { machine, path } => self.create_session(machine, &path),
        }
    }
}

// ---- drawing -------------------------------------------------------------------------------

fn card(t: &Theme) -> Style {
    Style::default().bg(t.card)
}

/// `host:` and the path after it, when the text starts with a machine's name.
fn split_host(shown: &str) -> (&str, &str) {
    match host_of(shown) {
        Some((host, _)) if !shown.starts_with(['~', '/', '.']) => shown.split_at(host.len() + 1),
        _ => ("", shown),
    }
}

fn branch_text(branch: &str, m: &Marks) -> String {
    let b: String = branch.chars().take(BRANCH_MAX).collect();
    let more = if branch.chars().count() > BRANCH_MAX { m.ellipsis } else { "" };
    format!("{}{b}{more}", m.branch)
}

pub(in crate::client) fn draw_session_popup(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, ns: &NewSession) {
    let chips = app.ns_chips();
    let projects = app.ns_projects();
    let recent = app.screen.saved.recent_paths.clone();
    let rows = ns.rows(&chips, &projects, &recent);
    let scope = scope_of(&ns.q, &chips);
    let selected = ns.selected(&rows);
    let sel_fill = selected.and_then(|(_, row)| row.fill());
    let ghost_text = sel_fill.as_deref().and_then(|fill| ghost(&ns.q, fill));
    let enter_label = match ns.enter(&rows, &chips) {
        Go::Start { .. } => match selected.map(|(_, r)| r) {
            Some(Row::StartHere { .. }) => "start here",
            _ => "start",
        },
        Go::Fill(_) if ns.chip.is_some() => "browse",
        Go::Fill(_) => "open folder",
        Go::Reconnect(_) => "reconnect",
        Go::Nothing => "",
    };
    let loading = ns.loading(&chips);
    let m = marks(app);
    let buf = f.buffer_mut();
    dim_all(app, buf, area, t);
    // Browsing one machine says so: NEW SESSION ON KINGDOM, or ON LOCAL for this computer.
    let title = match &scope {
        Scope::Path { machine, .. } => {
            let name = chips.iter().find(|c| c.id == *machine).map_or("", |c| c.name.as_str());
            format!("{TITLE} ON {}", if name.is_empty() { "LOCAL".to_string() } else { name.to_uppercase() })
        }
        Scope::Plain => TITLE.to_string(),
    };
    let r = panel(app, buf, area, POPUP_W, POPUP_H, &title, &[], t);
    let inner_left = r.x + 3;
    let inner_right = r.right().saturating_sub(3);
    let c = card(t);

    // The input row.
    let input = Rect { x: r.x + 1, y: r.y + 2, width: r.width - 2, height: 1 };
    strip(buf, input, t.card2);
    let s = Style::default().bg(t.card2);
    let mut parts = vec![seg(m.prompt, s.fg(t.accent).add_modifier(Modifier::BOLD))];
    if ns.q.is_empty() {
        parts.push(seg("project, folder, ~/path or kingdom:~/path", s.fg(t.muted)));
        parts.push(seg(m.cursor, s.fg(t.accent)));
    } else {
        let remote = matches!(scope, Scope::Path { machine, .. } if machine != MachineId::LOCAL);
        let (host, rest) = if remote { split_host(&ns.q) } else { ("", ns.q.as_str()) };
        if !host.is_empty() {
            parts.push(seg(host, s.fg(t.cyan()).add_modifier(Modifier::BOLD)));
        }
        parts.push(seg(rest, s.fg(t.strong).add_modifier(Modifier::BOLD)));
        match &ghost_text {
            Some(g) => {
                let mut chars = g.chars();
                let first = chars.next().map(String::from).unwrap_or_default();
                parts.push(seg(first, Style::default().fg(t.card2).bg(t.muted)));
                parts.push(seg(chars.collect::<String>(), s.fg(t.muted)));
            }
            None => parts.push(seg(m.cursor, s.fg(t.accent))),
        }
    }
    put(buf, r.x + 3, input.y, &parts, inner_right);
    if ghost_text.is_some() {
        let hint = "tab fill";
        put(buf, inner_right.saturating_sub(hint.width() as u16), input.y, &[seg(hint, s.fg(t.muted))], r.right() - 1);
    }

    // The breadcrumb or scope row.
    let crumb_y = r.y + 4;
    let online = chips.iter().filter(|m| m.state == ChipState::Online).count();
    let right_note = match &scope {
        Scope::Plain => format!("{online} machines online"),
        Scope::Path { .. } => {
            let n = rows.iter().filter(|r| matches!(r, Row::Folder { .. })).count();
            if loading { format!("reading{}", m.ellipsis) } else if n == 1 { "1 folder".to_string() } else { format!("{n} folders") }
        }
    };
    match &scope {
        Scope::Plain => {
            let text = if ns.q.trim().is_empty() {
                format!("{}everything open, plus recent paths", m.find)
            } else {
                format!("{}matches on every machine", m.find)
            };
            put(buf, inner_left, crumb_y, &[seg(text, c.fg(t.muted))], inner_right);
        }
        Scope::Path { machine, path } => {
            let name = chips.iter().find(|m| m.id == *machine).map_or("", |m| m.name.as_str());
            let badge = if name.is_empty() { format!(" {} local ", m.local) } else { format!(" {} {name} ", m.remote) };
            let (folder, _) = super::session_popup::split_folder(path);
            let segments: Vec<&str> = folder.split('/').filter(|p| !p.is_empty()).collect();
            let mut crumb = vec![seg(badge, Style::default().fg(t.text).bg(t.btn)), seg("  ", c)];
            for (i, part) in segments.iter().enumerate() {
                if i > 0 {
                    crumb.push(seg(" / ", c.fg(t.muted)));
                }
                let last = i + 1 == segments.len();
                let style = if last { c.fg(t.accent).add_modifier(Modifier::BOLD) } else { c.fg(t.text) };
                crumb.push(seg(*part, style));
            }
            put(buf, inner_left, crumb_y, &crumb, inner_right);
        }
    }
    put(buf, inner_right.saturating_sub(right_note.width() as u16), crumb_y, &[seg(right_note, c.fg(t.muted))], r.right() - 1);
    let rule = m.rule.repeat(r.width.saturating_sub(6) as usize);
    put(buf, r.x + 3, r.y + 5, &[seg(rule.clone(), c.fg(t.line))], r.right() - 1);

    // The list.
    let first_y = r.y + LIST_TOP;
    let room = r.height.saturating_sub(LIST_TOP + LIST_BOTTOM_GAP) as usize;
    let sel_at = selected.map_or(0, |(i, _)| i);
    let start = sel_at.saturating_sub(room.saturating_sub(1));
    for (offset, (i, row)) in rows.iter().enumerate().skip(start).take(room).enumerate() {
        let y = first_y + offset as u16;
        let is_sel = selected.is_some_and(|(s, _)| s == i);
        draw_row(app, buf, &r, y, row, is_sel, i, &chips, &scope, t, m);
    }

    // The machine strip.
    let strip_y = r.bottom().saturating_sub(4);
    put(buf, r.x + 3, strip_y.saturating_sub(1), &[seg(m.rule.repeat(r.width.saturating_sub(6) as usize), c.fg(t.line))], r.right() - 1);
    let mut x = r.x + 3;
    x = put(buf, x, strip_y, &[seg("on ", c.fg(t.muted))], inner_right);
    let browsing = match &scope {
        Scope::Path { machine, .. } => Some(*machine),
        Scope::Plain => None,
    };
    for (i, chip) in chips.iter().enumerate() {
        let label = if chip.name.is_empty() { format!(" {} local ", m.local) } else { format!(" {} {} ", m.remote, chip.name) };
        let style = if ns.chip == Some(i) {
            Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD)
        } else if browsing == Some(chip.id) {
            Style::default().fg(t.accent).bg(t.btn)
        } else if chip.state == ChipState::Offline {
            Style::default().fg(t.muted).bg(t.card2)
        } else {
            Style::default().fg(t.text).bg(t.btn)
        };
        let (glyph, color) = match chip.state {
            ChipState::Online => (m.online, t.done),
            ChipState::Connecting => (m.connecting, t.accent),
            ChipState::Offline => (m.offline, t.muted),
        };
        let start = x;
        x = put(buf, x, strip_y, &[seg(label, style), seg(" ", c), seg(glyph, c.fg(color)), seg("   ", c)], inner_right);
        hit(app, Rect { x: start, y: strip_y, width: x - start, height: 1 }, ScreenHit::NsChip(i));
    }
    let switch = m.switch;
    put(buf, inner_right.saturating_sub(switch.width() as u16), strip_y, &[seg(switch, c.fg(t.muted))], r.right() - 1);

    let mut bar: Vec<(&str, &str)> = Vec::new();
    if !enter_label.is_empty() {
        bar.push(("Enter", enter_label));
    }
    bar.extend([("Tab", "fill"), (m.move_keys, "move"), ("Esc", "close")]);
    panel_bar(buf, r, t, &bar);
}

#[allow(clippy::too_many_arguments)]
fn draw_row(app: &mut App, buf: &mut Buffer, r: &Rect, y: u16, row: &Row, selected: bool, index: usize, chips: &[Chip], scope: &Scope, t: &Theme, m: &Marks) {
    let c = card(t);
    let left = r.x + 3;
    let right = r.right().saturating_sub(3);
    match row {
        Row::Section { label, note } => {
            put(buf, left, y, &[seg(*label, c.fg(t.text)), seg(if note.is_empty() { String::new() } else { format!("  {note}") }, c.fg(t.muted))], right);
        }
        Row::Blank => {}
        Row::Note(text) => {
            put(buf, left, y, &[seg(*text, c.fg(t.muted).add_modifier(Modifier::ITALIC))], right);
        }
        Row::StartHere { shown, branch, .. } => {
            let bg = sel_row(app, buf, *r, y, selected, t);
            let s = Style::default().bg(bg);
            let (host, rest) = split_host(shown);
            let mut parts = vec![
                seg(format!("{}start here", m.start), s.fg(t.accent).add_modifier(if selected { Modifier::BOLD } else { Modifier::empty() })),
                seg("  ", s),
            ];
            if !host.is_empty() {
                parts.push(seg(host, s.fg(t.cyan())));
            }
            parts.push(seg(rest, s.fg(t.text)));
            put(buf, left, y, &parts, right.saturating_sub(24));
            let tag = branch.as_deref().map_or("no git".to_string(), |b| branch_text(b, m));
            put(buf, right.saturating_sub(tag.width() as u16), y, &[seg(tag, s.fg(t.muted))], r.right() - 1);
            hit(app, Rect { x: r.x + 1, y, width: r.width - 2, height: 1 }, ScreenHit::NsRow(index));
        }
        Row::Project { shown, branch, sessions, needs, .. } => {
            let bg = sel_row(app, buf, *r, y, selected, t);
            let s = Style::default().bg(bg);
            let glyph = if *needs { (m.needs, t.blocked) } else { (m.project, t.accent) };
            let mut parts = vec![seg(glyph.0, s.fg(glyph.1)), seg(" ", s)];
            parts.extend(path_parts(shown, 0, &[], selected, s, t, false));
            put(buf, left, y, &parts, right.saturating_sub(34));
            let mut tail = Vec::new();
            tail.push(seg(format!("{sessions} open"), s.fg(if *needs { t.blocked } else { t.muted })));
            if let Some(b) = branch {
                tail.push(seg("   ", s));
                tail.push(seg(branch_text(b, m), s.fg(if selected { t.text } else { t.muted })));
            }
            let width = segs_width(&tail);
            put(buf, right.saturating_sub(width), y, &tail, r.right() - 1);
            hit(app, Rect { x: r.x + 1, y, width: r.width - 2, height: 1 }, ScreenHit::NsRow(index));
        }
        Row::Folder { shown, typed, matched, branch, has_children, open, needs, .. } => {
            let bg = sel_row(app, buf, *r, y, selected, t);
            let s = Style::default().bg(bg);
            let glyph = if *has_children { m.folder } else { m.leaf };
            let mut parts = vec![seg(glyph, s.fg(t.muted)), seg(" ", s)];
            let browse = matches!(scope, Scope::Path { .. });
            parts.extend(path_parts(shown, *typed, matched, selected, s, t, browse));
            put(buf, left, y, &parts, right.saturating_sub(34));
            let mut tail = Vec::new();
            if *open > 0 {
                tail.push(seg(format!("{open} open"), s.fg(if *needs { t.blocked } else { t.muted })));
            }
            if let Some(b) = branch {
                if !tail.is_empty() {
                    tail.push(seg("   ", s));
                }
                tail.push(seg(branch_text(b, m), s.fg(if selected { t.text } else { t.muted })));
            } else if *has_children {
                if !tail.is_empty() {
                    tail.push(seg("   ", s));
                }
                tail.push(seg("folder", s.fg(t.muted)));
            }
            let width = segs_width(&tail);
            put(buf, right.saturating_sub(width), y, &tail, r.right() - 1);
            hit(app, Rect { x: r.x + 1, y, width: r.width - 2, height: 1 }, ScreenHit::NsRow(index));
        }
    }
    let _ = chips;
}

/// A path as the row shows it: the host in cyan when selected, the parents dim, and the last
/// segment normal or bold. `typed` characters at the front are what is already typed (dim, in
/// browse mode); `matched` are indices into the last segment that a search matched.
fn path_parts(shown: &str, typed: usize, matched: &[usize], selected: bool, s: Style, t: &Theme, browse: bool) -> Vec<Seg> {
    let (host, rest) = split_host(shown);
    let mut out = Vec::new();
    if !host.is_empty() {
        out.push(seg(host, s.fg(if selected { t.cyan() } else { t.muted })));
    }
    let body = rest.trim_end_matches('/');
    let split = body.rfind('/').map_or(0, |i| i + 1);
    let (parents, last) = body.split_at(split);
    let weight = if selected { Modifier::BOLD } else { Modifier::empty() };
    if browse {
        let done = typed.saturating_sub(host.chars().count());
        let all: Vec<char> = rest.chars().collect();
        let (dim, fresh): (String, String) = (all.iter().take(done).collect(), all.iter().skip(done).collect());
        out.push(seg(dim, s.fg(t.muted)));
        out.push(seg(fresh, s.fg(if selected { t.strong } else { t.text }).add_modifier(weight)));
        return out;
    }
    out.push(seg(parents, s.fg(t.muted)));
    if matched.is_empty() {
        out.push(seg(format!("{last}/"), s.fg(if selected { t.strong } else { t.text }).add_modifier(weight)));
    } else {
        for (i, ch) in last.chars().enumerate() {
            let hit_char = matched.contains(&i);
            let style = if hit_char {
                s.fg(t.accent).add_modifier(Modifier::BOLD)
            } else {
                s.fg(if selected { t.text } else { t.muted })
            };
            out.push(seg(ch.to_string(), style));
        }
        out.push(seg("/", s.fg(t.muted)));
    }
    out
}
