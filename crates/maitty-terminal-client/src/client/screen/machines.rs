//! The Add Machine dialog: one input typed like scp (`user@host`), the hosts of `~/.ssh/config`
//! to complete it from, then a checklist of what connecting does. What it does with a choice
//! (writing the config, probing, the first login) is in `machine_flow`; a machine is
//! reconnected or removed from its line in the sidebar.

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
            step: MachineStep::Browse,
            notice: None,
            windows,
        }
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

    fn machines_browse_key(&mut self, v: &mut MachinesView, k: &KeyEvent) {
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
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
        } => draw_name(buf, inner, target, name, error.as_deref(), t),
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
    panel_bar(buf, r, t, &bar_keys(&v.step));
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
    let room = (inner
        .height
        .saturating_sub(FIXED_ROWS + windows + notice_rows) as usize)
        .clamp(1, MOST_SUGGESTIONS);
    let mut y = inner.y;
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
    draw_field(buf, inner, y, &v.query, !v.in_list, t);
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

fn draw_name(
    buf: &mut Buffer,
    inner: Rect,
    target: &str,
    name: &str,
    error: Option<&str>,
    t: &Theme,
) {
    let c = Style::default().bg(t.card);
    let bold = c.fg(t.strong).add_modifier(Modifier::BOLD);
    let mut lines: Vec<Vec<Seg>> = vec![
        vec![seg(format!("Name for {}", sanitize(target, false)), bold)],
        vec![],
    ];
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

fn bar_keys(step: &MachineStep) -> Vec<(&'static str, &'static str)> {
    match step {
        MachineStep::Browse => vec![
            ("Enter", "connect"),
            ("↑↓", "input / hosts"),
            ("Esc", "cancel"),
        ],
        MachineStep::Name { .. } => {
            vec![("", "type a name"), ("Enter", "add"), ("Esc", "back")]
        }
        MachineStep::Checking { .. } => vec![("Esc", "cancel")],
        MachineStep::Added { .. } => vec![("Enter", "done"), ("Esc", "close")],
    }
}
