//! The Add Machine dialog: one input typed like scp (`user@host`), the hosts of `~/.ssh/config`
//! to complete it from, then a checklist of what connecting does. What it does with a choice
//! (writing the config, probing, the first login) is in `machine_flow`; a machine is
//! reconnected or removed from its line in the sidebar.

use super::chips::{Chip, chip};
use super::machine_group::AppFix;
use super::*;
use crate::client::machine_config::Grant;
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
    /// Where one of your machines is, being typed again.
    Edit {
        name: String,
        target: String,
        error: Option<String>,
    },
    /// Asking before one of your machines is taken off the list.
    Remove {
        name: String,
    },
    /// ssh is being asked, without a prompt, whether the machine can be reached.
    Checking {
        target: String,
    },
    /// The machine is written and dialling: what the checklist shows now.
    Added {
        name: String,
        target: String,
        how: Outcome,
    },
}

/// How an added machine stands when the dialog says so.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// It answered; its daemon's version.
    Ready(String),
    /// ssh needs a person to log in: a popup is open for it.
    Login,
    /// It was reached, but its daemon is missing, old or not running: the connection says how
    /// to fix it.
    Fix,
}

/// A line under the lists: what just happened, or why a choice was refused.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineNotice {
    pub text: String,
    pub error: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MachinesView {
    /// `~/.ssh/config`'s hosts; None while they are being read.
    pub hosts: Option<Vec<SshHost>>,
    /// What is typed: `user@host`, or a host alias, completed from the hosts below it.
    pub query: String,
    pub host_sel: usize,
    /// The keys are on the list of hosts, not the input.
    pub in_list: bool,
    /// The keys are on your machines, above the input: the chosen one's place in `machine_rows`.
    pub machine_sel: Option<usize>,
    pub step: MachineStep,
    pub notice: Option<MachineNotice>,
    /// Whether to say what Windows ssh can and cannot remember.
    pub windows: bool,
}

impl MachinesView {
    pub fn new(windows: bool) -> MachinesView {
        MachinesView {
            hosts: None,
            query: String::new(),
            host_sel: 0,
            in_list: false,
            machine_sel: None,
            step: MachineStep::Browse,
            notice: None,
            windows,
        }
    }
}

/// One configured machine as the dialog lists it.
#[derive(Debug, Clone, PartialEq)]
pub struct MachineRow {
    pub name: String,
    pub target: String,
    pub glyph: &'static str,
    pub word: String,
    pub color: Color,
    /// Why it is down and what to do, in full.
    pub detail: Option<String>,
    /// The app is missing or another version there: `u` installs or updates it.
    pub fix: Option<AppFix>,
    /// Whether it may raise toasts here (`n`) and play alert sounds (`s`). Both are off until you turn them on.
    pub notices: bool,
    pub sounds: bool,
}

impl App {
    /// The configured machines in the order the dialog lists them: by name.
    fn listed_machines(&self) -> Vec<&MachineDef> {
        let mut defs: Vec<&MachineDef> = self.cfg.machines.iter().collect();
        defs.sort_by_key(|d| d.name.to_lowercase());
        defs
    }

    fn machine_names(&self) -> Vec<String> {
        self.listed_machines()
            .into_iter()
            .map(|def| def.name.clone())
            .collect()
    }

    /// Where a machine is in the dialog's list.
    pub(in crate::client) fn machine_listed_at(&self, name: &str) -> Option<usize> {
        self.machine_names()
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))
    }

    /// The configured machines, each with how its connection stands.
    pub(in crate::client) fn machine_rows(&self, t: &Theme) -> Vec<MachineRow> {
        self.listed_machines()
            .into_iter()
            .map(|def| self.machine_row(def, t))
            .collect()
    }

    fn machine_row(&self, def: &MachineDef, t: &Theme) -> MachineRow {
        let found = self
            .remotes
            .iter()
            .find(|(_, remote)| remote.name.eq_ignore_ascii_case(&def.name));
        let fix = found.and_then(|(id, _)| self.machine_fix(*id));
        let (glyph, word, color, detail) = match found {
            Some((id, _)) if self.refused.contains_key(id) => (
                "✕",
                "refused".to_string(),
                t.err,
                self.refused.get(id).map(|why| sanitize(why, false)),
            ),
            Some((_, remote)) => {
                let mark = machine_group::state_mark(&remote.state, self.ascii_marks(), t);
                (
                    mark.glyph,
                    mark.word,
                    mark.color,
                    state_detail(&remote.state),
                )
            }
            None => (
                "◌",
                "not started".to_string(),
                t.muted,
                Some("Reconnect starts it.".to_string()),
            ),
        };
        MachineRow {
            name: sanitize(&def.name, false),
            target: sanitize(&def.target, false),
            glyph,
            word,
            color,
            // The fix texts name the machine as `<name>`: say which one.
            detail: detail.map(|why| why.replace("<name>", &def.name)),
            fix,
            notices: def.notices,
            sounds: def.sounds,
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

/// What the input says, read as `[user@]host[:path]`.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Parsed {
    pub user: Option<String>,
    pub host: String,
    /// Typed after a `:`; opening a folder is not built, so it is refused rather than dropped.
    pub path: Option<String>,
}

/// The input as `[user@]host[:path]`, or None while there is no host.
pub fn parse_target(text: &str) -> Option<Parsed> {
    let text = text.trim();
    let (rest, path) = match text.split_once(':') {
        Some((rest, path)) => (rest, Some(path.to_string())),
        None => (text, None),
    };
    let (user, host) = match rest.split_once('@') {
        Some((user, host)) => ((!user.is_empty()).then(|| user.to_string()), host),
        None => (None, rest),
    };
    (!host.is_empty() && !host.contains(char::is_whitespace)).then(|| Parsed {
        user,
        host: host.to_string(),
        path,
    })
}

/// The `Host` entry of `~/.ssh/config` the typed host names, if it is one.
pub fn known_host<'a>(parsed: &Parsed, hosts: &'a [SshHost]) -> Option<&'a SshHost> {
    hosts
        .iter()
        .find(|h| h.alias.eq_ignore_ascii_case(&parsed.host))
}

/// The hosts that match the host part of what is typed and are not machines yet.
pub fn suggestions(v: &MachinesView, existing: &[MachineDef]) -> Vec<SshHost> {
    let query = parse_target(&v.query).map_or_else(String::new, |p| p.host.to_lowercase());
    v.hosts
        .iter()
        .flatten()
        .filter(|h| {
            !existing
                .iter()
                .any(|m| m.target.eq_ignore_ascii_case(&h.alias))
        })
        .filter(|h| matches(h, &query))
        .cloned()
        .collect()
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

/// What Enter connects to: the chosen host while the keys are on the list, else the typed text.
/// None when there is nothing to connect to.
pub fn connect_target(v: &MachinesView, existing: &[MachineDef]) -> Option<String> {
    if v.in_list {
        let list = suggestions(v, existing);
        let at = v.host_sel.min(list.len().saturating_sub(1));
        return list.get(at).map(|h| h.alias.clone());
    }
    let typed = v.query.trim();
    (!typed.is_empty()).then(|| typed.to_string())
}

fn host_line(host: &SshHost) -> String {
    match (&host.user, &host.host_name) {
        (Some(user), Some(name)) => format!("{user}@{name}"),
        (None, Some(name)) => name.clone(),
        (Some(user), None) => format!("{user}@"),
        (None, None) => String::new(),
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
            MachineStep::Edit { name, target, .. } => {
                self.machines_edit_key(&mut v, name, target, k)
            }
            MachineStep::Remove { name } => self.machines_remove_key(&mut v, &name, k),
            MachineStep::Checking { .. } => {
                if k.code == KeyCode::Esc {
                    self.machines_cancel_probe(&mut v);
                }
            }
            MachineStep::Added { .. } => {
                if matches!(k.code, KeyCode::Esc | KeyCode::Enter) {
                    self.mode = Mode::Normal;
                }
            }
        }
        if matches!(self.mode, Mode::Machines(_)) {
            self.mode = Mode::Machines(Box::new(v));
        }
    }

    /// A click in the Machines dialog: choose machine `i`, and press `key` on it when a button
    /// was clicked, so a button does exactly what its key does.
    pub(in crate::client) fn machines_pick(&mut self, i: usize, key: Option<char>) {
        // The dialog stays the mode: the key handler puts the changed view back only while it is.
        let Mode::Machines(v) = &self.mode else {
            return;
        };
        let mut v = (**v).clone();
        v.machine_sel = Some(i);
        match key {
            Some(c) => self.on_machines_key(v, &KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)),
            None => self.mode = Mode::Machines(Box::new(v)),
        }
    }

    /// The keys while they are on your machines: the arrows move (Down from the last one is back
    /// in the input), and a letter acts on the chosen machine.
    fn machines_list_key(&mut self, v: &mut MachinesView, sel: usize, k: &KeyEvent) {
        let names = self.machine_names();
        let sel = sel.min(names.len().saturating_sub(1));
        let Some(name) = names.get(sel).cloned() else {
            v.machine_sel = None;
            return;
        };
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Up => v.machine_sel = Some(sel.saturating_sub(1)),
            KeyCode::Down if sel + 1 < names.len() => v.machine_sel = Some(sel + 1),
            KeyCode::Down | KeyCode::Tab | KeyCode::BackTab => v.machine_sel = None,
            KeyCode::Enter | KeyCode::Char('r') => v.notice = Some(self.machine_retry(&name)),
            KeyCode::Char('n') => v.notice = Some(self.machine_toggle_grant(&name, Grant::Notices)),
            KeyCode::Char('s') => v.notice = Some(self.machine_toggle_grant(&name, Grant::Sounds)),
            KeyCode::Char('u') => {
                let said = self.machine_update(&name);
                // The update's popup takes the keys, so the dialog is closed: say it in a toast.
                if matches!(self.mode, Mode::Machines(_)) {
                    v.notice = Some(said);
                } else {
                    self.notify(said.text, said.error);
                }
            }
            KeyCode::Char('e') => {
                let target = self
                    .cfg
                    .machines
                    .iter()
                    .find(|m| m.name == name)
                    .map(|m| m.target.clone())
                    .unwrap_or_default();
                v.notice = None;
                v.step = MachineStep::Edit {
                    name,
                    target,
                    error: None,
                };
            }
            KeyCode::Char('x') | KeyCode::Delete => {
                v.notice = None;
                v.step = MachineStep::Remove { name };
            }
            _ => {}
        }
    }

    /// Enter takes the machine off the list; any other key leaves it where it was.
    fn machines_remove_key(&mut self, v: &mut MachinesView, name: &str, k: &KeyEvent) {
        let sel = v.machine_sel.unwrap_or(0);
        v.step = MachineStep::Browse;
        if k.code != KeyCode::Enter {
            return;
        }
        v.notice = Some(self.machines_remove(name));
        let left = self.cfg.machines.len();
        v.machine_sel = (left > 0).then(|| sel.min(left - 1));
    }

    fn machines_browse_key(&mut self, v: &mut MachinesView, k: &KeyEvent) {
        if let Some(sel) = v.machine_sel {
            return self.machines_list_key(v, sel, k);
        }
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            // Up from the input is the last of your machines.
            KeyCode::Up if !v.in_list && !self.cfg.machines.is_empty() => {
                v.machine_sel = Some(self.cfg.machines.len() - 1);
            }
            // Down walks into the list of hosts, Up from its first row is back in the input.
            KeyCode::Up if v.in_list && v.host_sel == 0 => v.in_list = false,
            KeyCode::Up => v.host_sel = v.host_sel.saturating_sub(1),
            KeyCode::Down => {
                let last = suggestions(v, &self.cfg.machines).len().saturating_sub(1);
                if v.in_list {
                    v.host_sel = (v.host_sel + 1).min(last);
                } else if !suggestions(v, &self.cfg.machines).is_empty() {
                    v.in_list = true;
                    v.host_sel = 0;
                }
            }
            KeyCode::Tab | KeyCode::BackTab => {
                v.in_list = !v.in_list && !suggestions(v, &self.cfg.machines).is_empty();
                v.host_sel = 0;
            }
            KeyCode::Enter => self.machines_add_chosen(v),
            KeyCode::Backspace => {
                v.query.pop();
                (v.in_list, v.host_sel) = (false, 0);
            }
            KeyCode::Char('u') if k.modifiers.contains(KeyModifiers::CONTROL) => {
                v.query.clear();
                (v.in_list, v.host_sel) = (false, 0);
            }
            _ => {
                if let Some(c) = typed(k) {
                    v.query.push(c);
                    (v.in_list, v.host_sel) = (false, 0);
                    v.notice = None;
                }
            }
        }
    }

    /// Typing where a machine is: Enter saves it and dials there, Esc leaves it as it was.
    fn machines_edit_key(
        &mut self,
        v: &mut MachinesView,
        name: String,
        mut target: String,
        k: &KeyEvent,
    ) {
        match k.code {
            KeyCode::Esc => v.step = MachineStep::Browse,
            KeyCode::Enter => {
                let said = self.machine_retarget(&name, &target);
                if said.error {
                    v.step = MachineStep::Edit {
                        name,
                        target,
                        error: Some(said.text),
                    };
                } else {
                    v.step = MachineStep::Browse;
                    v.machine_sel = self.machine_listed_at(&name);
                    v.notice = Some(said);
                }
            }
            KeyCode::Backspace => {
                target.pop();
                v.step = MachineStep::Edit {
                    name,
                    target,
                    error: None,
                };
            }
            _ => {
                if let Some(c) = typed(k) {
                    target.push(c);
                }
                v.step = MachineStep::Edit {
                    name,
                    target,
                    error: None,
                };
            }
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
}

// ---- drawing -------------------------------------------------------------------------------

/// Left edge of the dialog's text, in cells from the panel's.
const INDENT: u16 = 3;
const WIDTH: u16 = 96;
const HEIGHT: u16 = 26;
const MOST_SUGGESTIONS: usize = 6;
/// Your machines shown at once; the list follows the chosen one past them.
const MOST_MACHINES: usize = 4;
/// The most lines the chosen machine's fix takes before its end is cut.
const DETAIL_LINES: usize = 2;
/// Rows the dialog keeps for everything but the suggestions: the question, the input, the parse
/// line and the heading, with the gaps between.
const FIXED_ROWS: u16 = 6;
/// What the Windows hint takes under the list: a gap and its lines.
const WINDOWS_ROWS: u16 = 3;

/// A step of the checklist.
#[derive(Clone, Copy, PartialEq)]
enum Mark {
    Done,
    Running,
    Waiting,
    Pending,
    Failed,
}

struct Step {
    mark: Mark,
    text: String,
    /// Said right-aligned, faint.
    aside: String,
}

fn step(mark: Mark, text: impl Into<String>, aside: impl Into<String>) -> Step {
    Step {
        mark,
        text: text.into(),
        aside: aside.into(),
    }
}

pub(in crate::client) fn draw_machines(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    t: &Theme,
    v: &MachinesView,
) {
    let buf = f.buffer_mut();
    dim_all(app, buf, area, t);
    let title = match v.step {
        MachineStep::Added { .. } => "MACHINE ADDED",
        // Opened on one of your machines, it is about that machine, not about adding one.
        MachineStep::Browse if v.machine_sel.is_some() => "MACHINES",
        _ => "ADD MACHINE",
    };
    let r = panel(app, buf, area, WIDTH, HEIGHT, title, &[], t);
    let inner = Rect {
        x: r.x + INDENT,
        y: r.y + 2,
        width: r.width.saturating_sub(INDENT * 2),
        height: r.height.saturating_sub(5),
    };
    let notice_rows = v.notice.as_ref().map_or(0, |n| {
        popover::wrap(&sanitize(&n.text, false), inner.width as usize).len() as u16
    });
    match &v.step {
        MachineStep::Browse => draw_input(app, buf, inner, v, t),
        MachineStep::Name {
            target,
            name,
            error,
        } => {
            let ask = format!("Name for {}", sanitize(target, false));
            draw_name(buf, inner, &ask, name, error.as_deref(), t)
        }
        MachineStep::Edit {
            name,
            target,
            error,
        } => {
            let ask = format!(
                "Where is {}? Type it like scp or ssh.",
                sanitize(name, false)
            );
            draw_name(buf, inner, &ask, target, error.as_deref(), t)
        }
        MachineStep::Remove { name } => draw_remove(buf, inner, name, t),
        MachineStep::Checking { target } => {
            let who = parse_target(target).unwrap_or_default();
            let steps = checklist(&who, None);
            draw_checklist(buf, inner, (&who.host, target), &steps, t);
        }
        MachineStep::Added { name, target, how } => {
            let who = parse_target(target).unwrap_or_default();
            let steps = checklist(&who, Some(how));
            draw_checklist(buf, inner, (name, target), &steps, t);
        }
    }
    if let Some(notice) = &v.notice {
        let color = if notice.error { t.err } else { t.done };
        let lines = popover::wrap(&sanitize(&notice.text, false), inner.width as usize);
        let top = inner.bottom().saturating_sub(notice_rows);
        for (i, line) in lines.into_iter().enumerate() {
            let row = Rect {
                x: inner.x,
                y: top + i as u16,
                width: inner.width,
                height: 1,
            };
            fill(buf, row, t.card);
            put(
                buf,
                inner.x,
                row.y,
                &[seg(line, Style::default().bg(t.card).fg(color))],
                inner.right(),
            );
        }
    }
    let chosen_fix = v.machine_sel.and_then(|sel| {
        let rows = app.machine_rows(t);
        rows.get(sel.min(rows.len().saturating_sub(1)))
            .and_then(|row| row.fix)
    });
    panel_bar(buf, r, t, &bar_keys(v, chosen_fix));
}

fn heading(buf: &mut Buffer, inner: Rect, y: u16, text: &str, t: &Theme) {
    put(
        buf,
        inner.x,
        y,
        &[seg(
            text,
            Style::default()
                .fg(t.accent)
                .bg(t.card)
                .add_modifier(Modifier::BOLD),
        )],
        inner.right(),
    );
}

fn draw_input(app: &mut App, buf: &mut Buffer, inner: Rect, v: &MachinesView, t: &Theme) {
    let c = Style::default().bg(t.card);
    let notice_rows = v.notice.as_ref().map_or(0, |n| {
        popover::wrap(&sanitize(&n.text, false), inner.width as usize).len() as u16
    });
    let windows = if v.windows { WINDOWS_ROWS } else { 0 };
    let yours = draw_your_machines(app, buf, inner, v, t);
    let room = (inner
        .height
        .saturating_sub(FIXED_ROWS + yours + windows + notice_rows) as usize)
        .clamp(1, MOST_SUGGESTIONS);
    let mut y = inner.y + yours;
    put(
        buf,
        inner.x,
        y,
        &[seg(
            "Where's the machine? Type it like scp or ssh.",
            c.fg(t.strong),
        )],
        inner.right(),
    );
    y += 2;
    draw_field(
        buf,
        inner,
        y,
        &v.query,
        !v.in_list && v.machine_sel.is_none(),
        t,
    );
    y += 1;
    let hosts = v.hosts.as_deref().unwrap_or_default();
    let parsed = parse_target(&v.query);
    let known = parsed.as_ref().and_then(|p| known_host(p, hosts));
    let (user, host) = match (&parsed, known) {
        (Some(p), Some(h)) => (
            p.user.clone().or_else(|| h.user.clone()),
            Some(h.host_name.clone().unwrap_or_else(|| h.alias.clone())),
        ),
        (Some(p), None) => (p.user.clone(), Some(p.host.clone())),
        _ => (None, None),
    };
    let part = |label: &str, value: Option<String>, none: &str| -> Vec<Seg> {
        let mut out = vec![seg(format!("{label} "), c.fg(t.muted))];
        out.push(match value {
            Some(v) => seg(
                format!("{}   ", sanitize(&v, false)),
                c.fg(t.strong).add_modifier(Modifier::BOLD),
            ),
            None => seg(format!("{none}   "), c.fg(t.muted)),
        });
        out
    };
    let mut line = vec![seg("  ", c)];
    line.extend(part("user", user, "—"));
    line.extend(part("host", host, "—"));
    put(buf, inner.x, y, &line, inner.right());
    y += 2;
    let list = suggestions(v, &app.cfg.machines);
    heading(buf, inner, y, "FROM ~/.ssh/config", t);
    y += 1;
    let quiet = c.fg(t.muted).add_modifier(Modifier::ITALIC);
    if v.hosts.is_none() {
        put(
            buf,
            inner.x + 1,
            y,
            &[seg("loading hosts from ~/.ssh/config…", quiet)],
            inner.right(),
        );
    } else if list.is_empty() {
        let none = if hosts.is_empty() {
            "no hosts in ~/.ssh/config: type user@host above"
        } else {
            "nothing in ~/.ssh/config matches, or every host is already a machine"
        };
        put(buf, inner.x + 1, y, &[seg(none, quiet)], inner.right());
    }
    let sel = v.host_sel.min(list.len().saturating_sub(1));
    let start = (sel + 1).saturating_sub(room);
    let alias_w = list
        .iter()
        .map(|h| h.alias.width())
        .max()
        .unwrap_or(0)
        .clamp(6, 24);
    for (n, host) in list.iter().enumerate().skip(start).take(room) {
        let rr = Rect {
            x: inner.x,
            y: y + (n - start) as u16,
            width: inner.width,
            height: 1,
        };
        let on = v.in_list && n == sel;
        let st = list_row(app, buf, rr, on, t);
        let rest_w = (inner.width as usize).saturating_sub(alias_w + 4 + 24);
        put(
            buf,
            rr.x + 2,
            rr.y,
            &[
                seg(
                    format!(
                        "{:<alias_w$}",
                        truncate(&sanitize(&host.alias, false), alias_w)
                    ),
                    st.fg(if on { t.strong } else { t.text })
                        .add_modifier(Modifier::BOLD),
                ),
                seg(
                    format!("  {}", truncate(&sanitize(&host_line(host), false), rest_w)),
                    st.fg(t.muted),
                ),
            ],
            rr.right(),
        );
        if on {
            let hint = "⏎ connect";
            put(
                buf,
                rr.right().saturating_sub(hint.width() as u16 + 1),
                rr.y,
                &[seg(hint, st.fg(t.accent))],
                rr.right(),
            );
        }
    }
    // Windows has no agent service to remember a password in; the other systems say nothing.
    if v.windows {
        y += room as u16 + 1;
        for (i, line) in popover::wrap(WINDOWS_HINT, inner.width as usize)
            .into_iter()
            .enumerate()
        {
            put(
                buf,
                inner.x,
                y + i as u16,
                &[seg(line, c.fg(t.muted))],
                inner.right(),
            );
        }
    }
}

/// Your machines, above the input: a row each with how its connection stands, and under the
/// chosen one what to do about it. Returns the rows taken, the gap after them included; none
/// when there is no machine yet.
fn draw_your_machines(
    app: &mut App,
    buf: &mut Buffer,
    inner: Rect,
    v: &MachinesView,
    t: &Theme,
) -> u16 {
    let rows = app.machine_rows(t);
    if rows.is_empty() {
        return 0;
    }
    let mut y = inner.y;
    heading(buf, inner, y, "YOUR MACHINES", t);
    y += 1;
    let sel = v.machine_sel.map(|s| s.min(rows.len() - 1));
    let start = sel.map_or(0, |s| (s + 1).saturating_sub(MOST_MACHINES));
    let name_w = rows
        .iter()
        .map(|r| r.name.width())
        .max()
        .unwrap_or(0)
        .clamp(6, 24);
    for (n, row) in rows.iter().enumerate().skip(start).take(MOST_MACHINES) {
        let rr = Rect {
            x: inner.x,
            y,
            width: inner.width,
            height: 1,
        };
        let on = sel == Some(n);
        let st = list_row(app, buf, rr, on, t);
        let state = format!("{} {}", row.glyph, row.word);
        let state_w = state.width();
        // One button for each thing the keys do, at the row's right end.
        let ground = st.bg.unwrap_or(t.card);
        let buttons: Vec<(Vec<Seg>, char)> = [
            (row.fix.map_or("update", AppFix::verb), 'u'),
            ("reconnect", 'r'),
            ("edit", 'e'),
            ("remove", 'x'),
        ]
        .into_iter()
        .map(|(label, key)| (chip(app, label, Chip::Action, ground, t), key))
        .collect();
        let buttons_w: usize = buttons.iter().map(|(b, _)| segs_width(b) as usize + 1).sum();
        let target_w =
            (inner.width as usize).saturating_sub(name_w + state_w + buttons_w + 13);
        put(
            buf,
            rr.x + 2,
            y,
            &[
                seg(
                    format!(
                        "{} {:<name_w$}",
                        machine_group::REMOTE_ICON,
                        truncate(&row.name, name_w)
                    ),
                    st.fg(if on { t.strong } else { t.text })
                        .add_modifier(Modifier::BOLD),
                ),
                seg(
                    format!("  {}", truncate(&row.target, target_w)),
                    st.fg(t.muted),
                ),
            ],
            rr.right(),
        );
        hit(app, rr, ScreenHit::MachineRow(n));
        let mut x = rr.right().saturating_sub(buttons_w as u16 + 1);
        put(
            buf,
            x.saturating_sub(state_w as u16 + 2),
            y,
            &[seg(state, st.fg(row.color).add_modifier(Modifier::BOLD))],
            x,
        );
        for (button, key) in &buttons {
            let w = segs_width(button);
            put(buf, x, y, button, rr.right());
            hit(app, Rect { x, y, width: w, height: 1 }, ScreenHit::MachineButton(n, *key));
            x += w + 1;
        }
        y += 1;
    }
    // What the chosen machine may do to this computer, which is nothing until you say so: toasts and sounds are off by default.
    if let Some(row) = sel.map(|s| &rows[s]) {
        let word = |on: bool| if on { "on" } else { "off" };
        put(
            buf,
            inner.x + 2,
            y,
            &[seg(
                format!(
                    "Alerts from {}: toasts {} · sounds {}.   n toasts · s sounds",
                    row.name,
                    word(row.notices),
                    word(row.sounds)
                ),
                Style::default().bg(t.card).fg(t.muted),
            )],
            inner.right(),
        );
        y += 1;
    }
    if let Some(detail) = sel.and_then(|s| rows[s].detail.as_deref()) {
        let room = inner.width.saturating_sub(2) as usize;
        for line in popover::wrap_capped(detail, room, DETAIL_LINES) {
            put(
                buf,
                inner.x + 2,
                y,
                &[seg(line, Style::default().bg(t.card).fg(t.text))],
                inner.right(),
            );
            y += 1;
        }
    }
    y + 1 - inner.y
}

/// The input, its row tinted, with the cursor.
fn draw_field(buf: &mut Buffer, inner: Rect, y: u16, query: &str, on: bool, t: &Theme) {
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
        seg(sanitize(query, false), s2.fg(t.strong)),
    ];
    if on {
        segs.push(seg("█", s2.fg(t.accent)));
    }
    if query.is_empty() {
        segs.push(seg(" user@host", s2.fg(t.muted)));
    }
    put(buf, inner.x, y, &segs, inner.right());
}

/// A step that asks for one line: what it asks, the field with what is typed, and under it why
/// the last answer was refused.
fn draw_name(
    buf: &mut Buffer,
    inner: Rect,
    ask: &str,
    name: &str,
    error: Option<&str>,
    t: &Theme,
) {
    let c = Style::default().bg(t.card);
    let bold = c.fg(t.strong).add_modifier(Modifier::BOLD);
    let mut lines: Vec<Vec<Seg>> = vec![vec![seg(ask.to_string(), bold)], vec![]];
    let field_y = inner.y + 2;
    draw_field(buf, inner, field_y, name, true, t);
    if let Some(error) = error {
        for line in popover::wrap(&sanitize(error, false), inner.width as usize) {
            lines.push(vec![seg(line, c.fg(t.err))]);
        }
    }
    for (n, segs) in lines.into_iter().enumerate() {
        // The field sits on the third row; what follows it goes under.
        let row = inner.y + n as u16 + u16::from(n >= 2) * 2;
        put(buf, inner.x, row, &segs, inner.right());
    }
}

fn draw_remove(buf: &mut Buffer, inner: Rect, name: &str, t: &Theme) {
    let c = Style::default().bg(t.card);
    let ask = format!("Remove {} from this list?", sanitize(name, false));
    let said = "Its sessions keep running on it, and you can add it again.";
    put(
        buf,
        inner.x,
        inner.y,
        &[seg(ask, c.fg(t.strong).add_modifier(Modifier::BOLD))],
        inner.right(),
    );
    put(
        buf,
        inner.x,
        inner.y + 2,
        &[seg(said.to_string(), c.fg(t.muted))],
        inner.right(),
    );
}

/// The steps connecting takes, in order. `how` is None while ssh is being asked.
fn checklist(who: &Parsed, how: Option<&Outcome>) -> Vec<Step> {
    let host = sanitize(&who.host, false);
    let user = who.user.clone().unwrap_or_else(|| "you".to_string());
    match how {
        None => vec![
            step(Mark::Running, format!("reach {host}"), ""),
            step(Mark::Pending, "verify host key", ""),
            step(Mark::Pending, format!("sign in as {user}"), ""),
            step(Mark::Pending, "maitty-daemon", ""),
        ],
        Some(Outcome::Ready(version)) => vec![
            step(Mark::Done, format!("reach {host}"), ""),
            step(Mark::Done, "verify host key", ""),
            step(Mark::Done, format!("sign in as {user}"), ""),
            step(Mark::Done, "maitty-daemon", version.clone()),
        ],
        Some(Outcome::Login) => vec![
            step(Mark::Done, format!("reach {host}"), ""),
            step(
                Mark::Waiting,
                format!("sign in as {user}"),
                "log in in the popup",
            ),
            step(Mark::Pending, "maitty-daemon", ""),
        ],
        Some(Outcome::Fix) => vec![
            step(Mark::Done, format!("reach {host}"), ""),
            step(Mark::Done, format!("sign in as {user}"), ""),
            step(
                Mark::Failed,
                "maitty-daemon",
                "the connection shows the fix",
            ),
        ],
    }
}

fn draw_checklist(
    buf: &mut Buffer,
    inner: Rect,
    (name, target): (&str, &str),
    steps: &[Step],
    t: &Theme,
) {
    let c = Style::default().bg(t.card);
    let mut y = inner.y;
    put(
        buf,
        inner.x,
        y,
        &[
            seg(
                format!("{} {}", machine_group::REMOTE_ICON, sanitize(name, false)),
                c.fg(t.strong).add_modifier(Modifier::BOLD),
            ),
            seg(format!("   {}", sanitize(target, false)), c.fg(t.muted)),
        ],
        inner.right(),
    );
    y += 2;
    for s in steps {
        let (glyph, colour, bold) = match s.mark {
            Mark::Done => ("✓", t.done, false),
            Mark::Running => ("◌", t.accent, true),
            Mark::Waiting => ("●", t.blocked, true),
            Mark::Pending => ("○", t.muted, false),
            Mark::Failed => ("✕", t.err, true),
        };
        let text_style = c
            .fg(match s.mark {
                Mark::Waiting => t.blocked,
                Mark::Failed => t.err,
                Mark::Pending => t.muted,
                _ => t.text,
            })
            .add_modifier(if bold {
                Modifier::BOLD
            } else {
                Modifier::empty()
            });
        put(
            buf,
            inner.x + 2,
            y,
            &[
                seg(format!("{glyph} "), c.fg(colour)),
                seg(s.text.clone(), text_style),
            ],
            inner.right(),
        );
        if !s.aside.is_empty() {
            let w = s.aside.width() as u16;
            put(
                buf,
                inner.right().saturating_sub(w + 1),
                y,
                &[seg(s.aside.clone(), c.fg(t.muted))],
                inner.right(),
            );
        }
        y += 1;
    }
}

/// The keys under the dialog. `fix`: what `u` does for the chosen machine, when it needs it.
fn bar_keys(v: &MachinesView, fix: Option<AppFix>) -> Vec<(&'static str, &'static str)> {
    match &v.step {
        MachineStep::Browse if v.machine_sel.is_some() => vec![
            ("r", "reconnect"),
            ("u", fix.map_or("update", AppFix::verb)),
            ("n s", "alerts"),
            ("e", "edit"),
            ("x", "remove"),
            ("↑↓", "move"),
            ("Esc", "close"),
        ],
        MachineStep::Browse => vec![
            ("Enter", "connect"),
            ("↑↓", "input / hosts"),
            ("Esc", "cancel"),
        ],
        MachineStep::Name { .. } => {
            vec![("", "type a name"), ("Enter", "add"), ("Esc", "back")]
        }
        MachineStep::Edit { .. } => {
            vec![("", "type where it is"), ("Enter", "save"), ("Esc", "back")]
        }
        MachineStep::Remove { .. } => vec![("Enter", "remove"), ("Esc", "keep it")],
        MachineStep::Checking { .. } => vec![("Esc", "cancel")],
        MachineStep::Added { .. } => vec![("Enter", "done"), ("Esc", "close")],
    }
}
