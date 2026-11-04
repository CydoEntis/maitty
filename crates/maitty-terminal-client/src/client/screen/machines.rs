//! The Machines dialog: the computers this window reaches over ssh, with how each connection
//! stands and what to do about it, and a picker to add another. What it does with a choice
//! (writing the config, probing, the first login) is in `machine_flow`.

use super::*;
use crate::client::ssh_hosts::SshHost;
use crate::hidden::sanitize;
use maitty_settings::config::MachineDef;

/// Said on Windows only: there is no ssh agent service to remember a password in.
pub const WINDOWS_HINT: &str =
    "Windows: use ssh keys or an agent. Password prompts work in the popup but are not remembered.";

/// What a machine may be removed with, said where the person confirms it.
pub fn removal_note(name: &str) -> String {
    format!("Removes {name} from this list. Its sessions keep running on {name}.")
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineSection {
    Machines,
    Add,
}

/// What the Add side is doing: choosing, or waiting on something it started.
#[derive(Debug, Clone, PartialEq)]
pub enum MachineStep {
    Browse,
    /// No usable name came from the target: the person types one.
    Name {
        target: String,
        name: String,
        error: Option<String>,
    },
    /// ssh is being asked, without a prompt, whether the machine can be reached.
    Checking {
        target: String,
    },
    Remove {
        name: String,
    },
}

/// A line under the lists: what just happened, or why a choice was refused.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineNotice {
    pub text: String,
    pub error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MachinesView {
    pub section: MachineSection,
    /// The selected machine of "yours".
    pub sel: usize,
    /// `~/.ssh/config`'s hosts; None while they are being read.
    pub hosts: Option<Vec<SshHost>>,
    /// What is typed in the Add field: filters the hosts, or is the host itself.
    pub query: String,
    pub host_sel: usize,
    pub step: MachineStep,
    pub notice: Option<MachineNotice>,
    /// Whether to say what Windows ssh can and cannot remember.
    pub windows: bool,
}

impl MachinesView {
    pub fn new(windows: bool) -> MachinesView {
        MachinesView {
            section: MachineSection::Add,
            sel: 0,
            hosts: None,
            query: String::new(),
            host_sel: 0,
            step: MachineStep::Browse,
            notice: None,
            windows,
        }
    }
}

/// A row of the Add list.
#[derive(Debug, Clone, PartialEq)]
pub enum AddRow {
    Host(SshHost),
    /// The typed text, taken as the host.
    Typed(String),
}

impl AddRow {
    /// What goes in `target`: the alias, so ssh's own config for it still applies.
    pub fn target(&self) -> &str {
        match self {
            AddRow::Host(host) => &host.alias,
            AddRow::Typed(text) => text,
        }
    }
}

/// The hosts that match what is typed and are not machines yet, then the typed text itself
/// when it is not already one of them.
pub fn add_rows(v: &MachinesView, existing: &[MachineDef]) -> Vec<AddRow> {
    let query = v.query.trim().to_lowercase();
    let mut rows: Vec<AddRow> = v
        .hosts
        .iter()
        .flatten()
        .filter(|h| {
            !existing
                .iter()
                .any(|m| m.target.eq_ignore_ascii_case(&h.alias))
        })
        .filter(|h| matches(h, &query))
        .cloned()
        .map(AddRow::Host)
        .collect();
    let typed = v.query.trim();
    let is_alias = rows
        .iter()
        .any(|r| matches!(r, AddRow::Host(h) if h.alias.eq_ignore_ascii_case(typed)));
    if !typed.is_empty() && !is_alias {
        rows.push(AddRow::Typed(typed.to_string()));
    }
    rows
}

fn matches(host: &SshHost, query: &str) -> bool {
    let text = format!(
        "{} {} {}",
        host.alias,
        host.host_name.as_deref().unwrap_or_default(),
        host.user.as_deref().unwrap_or_default()
    )
    .to_lowercase();
    query.split_whitespace().all(|word| text.contains(word))
}

/// One configured machine as the list shows it.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineRow {
    pub name: String,
    pub target: String,
    pub glyph: &'static str,
    pub word: String,
    pub color: Color,
    /// Why it is down and what to do, in full.
    pub detail: Option<String>,
}

impl App {
    /// The configured machines in name order, each with how its connection stands.
    pub(in crate::client) fn machine_rows(&self, t: &Theme) -> Vec<MachineRow> {
        let ascii = self.cfg.ui.glyphs == "ascii";
        let mut defs: Vec<&MachineDef> = self.cfg.machines.iter().collect();
        defs.sort_by_key(|d| d.name.to_lowercase());
        defs.into_iter()
            .map(|def| self.machine_row(def, ascii, t))
            .collect()
    }

    fn machine_row(&self, def: &MachineDef, ascii: bool, t: &Theme) -> MachineRow {
        let found = self
            .remotes
            .iter()
            .find(|(_, remote)| remote.name.eq_ignore_ascii_case(&def.name));
        let (glyph, word, color, detail) = match found {
            Some((id, _)) if self.refused.contains_key(id) => (
                if ascii { "x" } else { "✕" },
                "refused".to_string(),
                t.err,
                self.refused.get(id).map(|why| sanitize(why, false)),
            ),
            Some((_, remote)) => {
                let mark = machine_group::state_mark(&remote.state, ascii, t);
                (
                    mark.glyph,
                    mark.word,
                    mark.color,
                    state_detail(&remote.state),
                )
            }
            None if self.machine_login_waiting(&def.name) => (
                if ascii { "." } else { "◌" },
                "waiting for login".to_string(),
                t.muted,
                Some("Log in in the popup. It connects when you are done.".to_string()),
            ),
            None => (
                if ascii { "." } else { "◌" },
                "not started".to_string(),
                t.muted,
                Some("Retry starts it.".to_string()),
            ),
        };
        MachineRow {
            name: sanitize(&def.name, false),
            target: sanitize(&def.target, false),
            glyph,
            word,
            color,
            detail,
        }
    }
}

fn state_detail(state: &crate::client::link::ConnectionState) -> Option<String> {
    use crate::client::link::ConnectionState::*;
    match state {
        NeedsLogin { why } | NeedsUpdate { why } | Unreachable { why, .. } => {
            Some(sanitize(why, false))
        }
        Reconnecting { .. } => Some("The connection was lost; trying again.".to_string()),
        Connecting | Connected => None,
    }
}

// ---- keys ----------------------------------------------------------------------------------

fn typed(k: &KeyEvent) -> Option<char> {
    match k.code {
        KeyCode::Char(c)
            if !k
                .modifiers
                .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
        {
            Some(c)
        }
        _ => None,
    }
}

impl App {
    pub(in crate::client) fn on_machines_key(&mut self, mut v: MachinesView, k: &KeyEvent) {
        match v.step.clone() {
            MachineStep::Browse => self.machines_browse_key(&mut v, k),
            MachineStep::Name { target, name, .. } => {
                self.machines_name_key(&mut v, target, name, k)
            }
            MachineStep::Checking { .. } => {
                if k.code == KeyCode::Esc {
                    self.machines_cancel_probe(&mut v);
                }
            }
            MachineStep::Remove { name } => match k.code {
                KeyCode::Enter | KeyCode::Char('y') => {
                    v.step = MachineStep::Browse;
                    v.notice = Some(self.machines_remove(&name));
                    v.sel = v.sel.saturating_sub(1);
                }
                KeyCode::Esc | KeyCode::Char('n') => v.step = MachineStep::Browse,
                _ => {}
            },
        }
        if matches!(self.mode, Mode::Machines(_)) {
            self.mode = Mode::Machines(Box::new(v));
        }
    }

    fn machines_browse_key(&mut self, v: &mut MachinesView, k: &KeyEvent) {
        let machines = self.cfg.machines.len();
        match (k.code, v.section) {
            (KeyCode::Esc, _) => self.mode = Mode::Normal,
            (KeyCode::Tab | KeyCode::BackTab, _) if machines > 0 => {
                v.section = match v.section {
                    MachineSection::Machines => MachineSection::Add,
                    MachineSection::Add => MachineSection::Machines,
                };
            }
            (KeyCode::Up, MachineSection::Machines) => v.sel = v.sel.saturating_sub(1),
            (KeyCode::Down, MachineSection::Machines) => {
                v.sel = (v.sel + 1).min(machines.saturating_sub(1));
            }
            (KeyCode::Up, MachineSection::Add) => v.host_sel = v.host_sel.saturating_sub(1),
            (KeyCode::Down, MachineSection::Add) => {
                let last = add_rows(v, &self.cfg.machines).len().saturating_sub(1);
                v.host_sel = (v.host_sel + 1).min(last);
            }
            (KeyCode::Char('r') | KeyCode::Enter, MachineSection::Machines) => {
                if let Some(name) = self.machine_name_at(v.sel) {
                    v.notice = Some(self.machine_retry(&name));
                }
            }
            (KeyCode::Char('x') | KeyCode::Delete, MachineSection::Machines) => {
                if let Some(name) = self.machine_name_at(v.sel) {
                    v.step = MachineStep::Remove { name };
                }
            }
            (KeyCode::Enter, MachineSection::Add) => self.machines_add_chosen(v),
            (KeyCode::Backspace, MachineSection::Add) => {
                v.query.pop();
                v.host_sel = 0;
            }
            (KeyCode::Char('u'), MachineSection::Add)
                if k.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                v.query.clear();
                v.host_sel = 0;
            }
            (_, MachineSection::Add) => {
                if let Some(c) = typed(k) {
                    v.query.push(c);
                    v.host_sel = 0;
                    v.notice = None;
                }
            }
            _ => {}
        }
    }

    fn machines_name_key(
        &mut self,
        v: &mut MachinesView,
        target: String,
        mut name: String,
        k: &KeyEvent,
    ) {
        match k.code {
            KeyCode::Esc => v.step = MachineStep::Browse,
            KeyCode::Enter => self.machines_add_named(v, target, name),
            KeyCode::Backspace => {
                name.pop();
                v.step = MachineStep::Name {
                    target,
                    name,
                    error: None,
                };
            }
            _ => {
                if let Some(c) = typed(k) {
                    name.push(c);
                }
                v.step = MachineStep::Name {
                    target,
                    name,
                    error: None,
                };
            }
        }
    }

    /// The configured machine in row `at` of the list (name order).
    fn machine_name_at(&self, at: usize) -> Option<String> {
        let mut names: Vec<&str> = self.cfg.machines.iter().map(|m| m.name.as_str()).collect();
        names.sort_by_key(|n| n.to_lowercase());
        names.get(at).map(|n| n.to_string())
    }
}

// ---- drawing -------------------------------------------------------------------------------

/// Left edge of the dialog's text, in cells from the panel's.
const INDENT: u16 = 3;
const WIDTH: u16 = 100;
const LEAST_HEIGHT: usize = 16;
const MOST_MACHINE_ROWS: usize = 14;
const MOST_ADD_ROWS: usize = 8;
/// The rows of the Add list that stay, whatever the machines list takes.
const ADD_ROWS_KEPT: usize = 2;

/// The rows the dialog asks for: what it has to show, between a least and a most. A smaller
/// window gives it less, and the lists scroll.
fn wanted_height(
    v: &MachinesView,
    rows: &[MachineRow],
    adds: &[AddRow],
    screen_width: u16,
    t: &Theme,
) -> u16 {
    let width = WIDTH
        .min(screen_width.saturating_sub(2))
        .saturating_sub(INDENT * 2) as usize;
    let hint = if v.windows {
        popover::wrap(WINDOWS_HINT, width).len() + 1
    } else {
        0
    };
    let machines = machine_lines(rows, v, width, t)
        .len()
        .min(MOST_MACHINE_ROWS);
    let add = match v.step {
        MachineStep::Browse => 1 + adds.len().clamp(ADD_ROWS_KEPT + 1, MOST_ADD_ROWS),
        _ => 5,
    };
    let notice = v
        .notice
        .as_ref()
        .map_or(0, |n| popover::wrap(&n.text, width).len());
    // Border, gap, two headings and a gap between the lists, the bar and its gap.
    let chrome = 9;
    (chrome + hint + machines + add + notice).max(LEAST_HEIGHT) as u16
}

pub(in crate::client) fn draw_machines(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    t: &Theme,
    v: &MachinesView,
) {
    let rows = app.machine_rows(t);
    let adds = add_rows(v, &app.cfg.machines);
    let height = wanted_height(v, &rows, &adds, area.width, t);
    let buf = f.buffer_mut();
    dim_all(app, buf, area, t);
    let r = panel(app, buf, area, WIDTH, height, "Machines", &[], t);
    let inner = Rect {
        x: r.x + INDENT,
        y: r.y + 2,
        width: r.width.saturating_sub(INDENT * 2),
        height: r.height.saturating_sub(5),
    };
    let width = inner.width as usize;
    let c = Style::default().bg(t.card);
    let mut y = inner.y;
    if v.windows {
        for line in popover::wrap(WINDOWS_HINT, width) {
            put(buf, inner.x, y, &[seg(line, c.fg(t.muted))], inner.right());
            y += 1;
        }
        y += 1;
    }
    let machine_lines = machine_lines(&rows, v, width, t);
    // The rest: two headings, a gap, the field, and the Add list's own rows.
    let fixed = 4 + ADD_ROWS_KEPT;
    let room = (inner.bottom().saturating_sub(y) as usize).saturating_sub(fixed);
    let machine_room = machine_lines.len().clamp(1, room.max(1));
    heading(
        buf,
        inner,
        y,
        "YOURS",
        v.section == MachineSection::Machines,
        t,
    );
    y += 1;
    draw_machine_lines(buf, inner, y, machine_room, &machine_lines, v, t);
    y += machine_room as u16 + 1;
    heading(buf, inner, y, "ADD", v.section == MachineSection::Add, t);
    y += 1;
    let bottom = inner.bottom();
    match &v.step {
        MachineStep::Browse => {
            let field = draw_field(buf, inner, y, &v.query, v.section == MachineSection::Add, t);
            y = field + 1;
            draw_add_list(app, buf, inner, y, bottom, v, &adds, t);
        }
        step => draw_step(buf, inner, y, bottom, step, t),
    }
    if let Some(notice) = &v.notice {
        let color = if notice.error { t.err } else { t.done };
        let lines = popover::wrap(&sanitize(&notice.text, false), width);
        let top = bottom.saturating_sub(lines.len() as u16);
        for (i, line) in lines.into_iter().enumerate() {
            fill(
                buf,
                Rect {
                    x: inner.x,
                    y: top + i as u16,
                    width: inner.width,
                    height: 1,
                },
                t.card,
            );
            put(
                buf,
                inner.x,
                top + i as u16,
                &[seg(line, c.fg(color))],
                inner.right(),
            );
        }
    }
    panel_bar(buf, r, t, &bar_keys(v));
}

fn heading(buf: &mut Buffer, inner: Rect, y: u16, text: &str, on: bool, t: &Theme) {
    let c = Style::default().bg(t.card);
    let color = if on { t.accent } else { t.muted };
    put(
        buf,
        inner.x,
        y,
        &[seg(text, c.fg(color).add_modifier(Modifier::BOLD))],
        inner.right(),
    );
}

/// Each machine as its row and the lines of its detail, tagged with the machine's index.
fn machine_lines(
    rows: &[MachineRow],
    v: &MachinesView,
    width: usize,
    t: &Theme,
) -> Vec<(usize, Vec<Seg>)> {
    if rows.is_empty() {
        let quiet = Style::default()
            .bg(t.card)
            .fg(t.muted)
            .add_modifier(Modifier::ITALIC);
        return vec![(
            usize::MAX,
            vec![seg("none yet: pick a host below, or type user@host", quiet)],
        )];
    }
    let name_w = rows
        .iter()
        .map(|r| r.name.width())
        .max()
        .unwrap_or(0)
        .clamp(4, 20);
    let mut lines = Vec::new();
    for (i, row) in rows.iter().enumerate() {
        let on = i == v.sel && v.section == MachineSection::Machines;
        let bg = if on { t.sel_on(t.card) } else { t.card };
        let st = Style::default().bg(bg);
        let word_w = 18;
        let used = 2 + word_w + 1 + name_w + 2;
        let target = truncate(&row.target, width.saturating_sub(used).max(4));
        lines.push((
            i,
            vec![
                seg(if on { "▎" } else { " " }, st.fg(t.accent)),
                seg(format!("{} ", row.glyph), st.fg(row.color)),
                seg(
                    format!("{:<word_w$}", truncate(&row.word, word_w)),
                    st.fg(row.color),
                ),
                seg(" ", st),
                seg(
                    format!("{:<name_w$}", truncate(&row.name, name_w)),
                    st.fg(t.strong).add_modifier(Modifier::BOLD),
                ),
                seg(format!("  {target}"), st.fg(t.muted)),
            ],
        ));
        if let Some(detail) = &row.detail {
            for line in popover::wrap(detail, width.saturating_sub(4)) {
                lines.push((
                    i,
                    vec![seg(
                        format!("    {line}"),
                        Style::default().bg(t.card).fg(t.text),
                    )],
                ));
            }
        }
    }
    lines
}

#[allow(clippy::too_many_arguments)]
fn draw_machine_lines(
    buf: &mut Buffer,
    inner: Rect,
    y: u16,
    room: usize,
    lines: &[(usize, Vec<Seg>)],
    v: &MachinesView,
    t: &Theme,
) {
    let last_of_sel = lines
        .iter()
        .rposition(|(i, _)| *i == v.sel)
        .map_or(0, |at| at + 1);
    let first_of_sel = lines.iter().position(|(i, _)| *i == v.sel).unwrap_or(0);
    let start = last_of_sel.saturating_sub(room).min(first_of_sel);
    for (n, (_, segs)) in lines.iter().skip(start).take(room).enumerate() {
        let row = Rect {
            x: inner.x,
            y: y + n as u16,
            width: inner.width,
            height: 1,
        };
        let on = lines[start + n].0 == v.sel && v.section == MachineSection::Machines;
        fill(buf, row, if on { t.sel_on(t.card) } else { t.card });
        put(buf, inner.x, row.y, segs, inner.right());
    }
}

/// The Add field; returns its row.
fn draw_field(buf: &mut Buffer, inner: Rect, y: u16, query: &str, on: bool, t: &Theme) -> u16 {
    let row = Rect {
        x: inner.x,
        y,
        width: inner.width,
        height: 1,
    };
    strip(buf, row, t.card2);
    let s2 = Style::default().bg(t.card2);
    let mut segs = vec![
        seg(" › ", s2.fg(t.accent).add_modifier(Modifier::BOLD)),
        seg(query.to_string(), s2.fg(t.strong)),
    ];
    if on {
        segs.push(seg("█", s2.fg(t.accent)));
    }
    if query.is_empty() {
        segs.push(seg(
            " type user@host, or filter the hosts below",
            s2.fg(t.muted),
        ));
    }
    put(buf, inner.x, y, &segs, inner.right());
    y
}

#[allow(clippy::too_many_arguments)]
fn draw_add_list(
    app: &mut App,
    buf: &mut Buffer,
    inner: Rect,
    y: u16,
    bottom: u16,
    v: &MachinesView,
    adds: &[AddRow],
    t: &Theme,
) {
    let c = Style::default().bg(t.card);
    let quiet = c.fg(t.muted).add_modifier(Modifier::ITALIC);
    let notice_rows = v.notice.as_ref().map_or(0, |n| {
        popover::wrap(&n.text, inner.width as usize).len() as u16
    });
    let room = bottom.saturating_sub(y).saturating_sub(notice_rows) as usize;
    if v.hosts.is_none() {
        put(
            buf,
            inner.x + 1,
            y,
            &[seg("loading hosts from ~/.ssh/config…", quiet)],
            inner.right(),
        );
        return;
    }
    if adds.is_empty() {
        let none = if v.hosts.as_ref().is_some_and(|h| h.is_empty()) {
            "no hosts in ~/.ssh/config: type user@host above"
        } else {
            "every host in ~/.ssh/config is already a machine"
        };
        put(buf, inner.x + 1, y, &[seg(none, quiet)], inner.right());
        return;
    }
    let sel = v.host_sel.min(adds.len() - 1);
    let start = (sel + 1).saturating_sub(room);
    let alias_w = adds
        .iter()
        .map(|r| r.target().width())
        .max()
        .unwrap_or(0)
        .clamp(6, 24);
    for (n, row) in adds.iter().enumerate().skip(start).take(room) {
        let rr = Rect {
            x: inner.x,
            y: y + (n - start) as u16,
            width: inner.width,
            height: 1,
        };
        let on = n == sel && v.section == MachineSection::Add;
        let st = list_row(app, buf, rr, on, t);
        let (label, rest) = match row {
            AddRow::Host(h) => (h.alias.clone(), host_line(h)),
            AddRow::Typed(text) => (text.clone(), "add as typed".to_string()),
        };
        let rest_w = (inner.width as usize).saturating_sub(alias_w + 4);
        put(
            buf,
            rr.x + 2,
            rr.y,
            &[
                seg(
                    format!("{:<alias_w$}", truncate(&sanitize(&label, false), alias_w)),
                    st.fg(if on { t.strong } else { t.text })
                        .add_modifier(Modifier::BOLD),
                ),
                seg(
                    format!("  {}", truncate(&sanitize(&rest, false), rest_w)),
                    st.fg(t.muted),
                ),
            ],
            rr.right(),
        );
    }
}

fn host_line(host: &SshHost) -> String {
    match (&host.user, &host.host_name) {
        (Some(user), Some(name)) => format!("{user}@{name}"),
        (None, Some(name)) => name.clone(),
        (Some(user), None) => format!("{user}@"),
        (None, None) => String::new(),
    }
}

fn draw_step(buf: &mut Buffer, inner: Rect, y: u16, bottom: u16, step: &MachineStep, t: &Theme) {
    let c = Style::default().bg(t.card);
    let width = inner.width as usize;
    let mut lines: Vec<Vec<Seg>> = Vec::new();
    let bold = c.fg(t.strong).add_modifier(Modifier::BOLD);
    match step {
        MachineStep::Checking { target } => {
            lines.push(vec![seg(
                format!("Checking {}…", sanitize(target, false)),
                bold,
            )]);
            lines.push(vec![seg(
                "Esc cancels. Nothing is added until ssh answers.",
                c.fg(t.muted),
            )]);
        }
        MachineStep::Remove { name } => {
            let name = sanitize(name, false);
            lines.push(vec![seg(format!("Remove {name}?"), bold)]);
            for line in popover::wrap(&removal_note(&name), width) {
                lines.push(vec![seg(line, c.fg(t.text))]);
            }
            lines.push(vec![seg("Enter removes it. Esc keeps it.", c.fg(t.muted))]);
        }
        MachineStep::Name {
            target,
            name,
            error,
        } => {
            lines.push(vec![seg(
                format!("Name for {}", sanitize(target, false)),
                bold,
            )]);
            lines.push(vec![
                seg("› ", c.fg(t.accent)),
                seg(name.clone(), c.fg(t.strong)),
                seg("█", c.fg(t.accent)),
            ]);
            if let Some(error) = error {
                for line in popover::wrap(&sanitize(error, false), width) {
                    lines.push(vec![seg(line, c.fg(t.err))]);
                }
            }
        }
        MachineStep::Browse => {}
    }
    for (n, segs) in lines.into_iter().enumerate() {
        let row = y + n as u16;
        if row < bottom {
            put(buf, inner.x, row, &segs, inner.right());
        }
    }
}

fn bar_keys(v: &MachinesView) -> Vec<(&'static str, &'static str)> {
    match (&v.step, v.section) {
        (MachineStep::Browse, MachineSection::Machines) => vec![
            ("↑↓", "choose"),
            ("r", "retry"),
            ("x", "remove"),
            ("Tab", "add"),
            ("Esc", "close"),
        ],
        (MachineStep::Browse, MachineSection::Add) => vec![
            ("", "type to filter"),
            ("↑↓", "choose"),
            ("Enter", "add"),
            ("Tab", "yours"),
            ("Esc", "close"),
        ],
        (MachineStep::Remove { .. }, _) => vec![("Enter", "remove"), ("Esc", "keep")],
        (MachineStep::Name { .. }, _) => {
            vec![("", "type a name"), ("Enter", "add"), ("Esc", "back")]
        }
        (MachineStep::Checking { .. }, _) => vec![("Esc", "cancel")],
    }
}
