//! A machine's place in the sidebar: the header that names it and says whether its line is up,
//! the notes under it (why it is down, what its daemon reports), and the grey a down machine's
//! rows are drawn in. A window with no other machine draws none of this.
//!
//! Everything is derived from the machine's `Remote` each frame, so a machine that comes back
//! is drawn normally again with nothing to reset.

use super::side::Line;
use super::*;
use crate::client::link::ConnectionState;
use crate::hidden::sanitize;
use std::time::Duration;
use unicode_width::UnicodeWidthChar;

/// What the header calls the machine the window runs on, once there is another to tell it from.
pub(super) const THIS_COMPUTER: &str = "this computer";

/// A name keeps at least this many cells before the state's word gives way to its glyph.
const NAME_KEPT: usize = 8;

/// The most lines a down machine's reason takes before its end is cut with an ellipsis.
const REASON_LINES: usize = 3;

/// The mark that leads a remote machine's name.
const LEAD: &str = "⇄ ";
const LEAD_ASCII: &str = "<> ";

/// A connection state as the header says it: a glyph, a word, and the role that colours both.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct StateMark {
    pub glyph: &'static str,
    pub word: String,
    pub color: Color,
}

pub(super) fn state_mark(state: &ConnectionState, ascii: bool, t: &Theme) -> StateMark {
    let pick = |unicode, plain| if ascii { plain } else { unicode };
    let (glyph, word, color) = match state {
        ConnectionState::Connecting => (pick("◌", "."), "connecting".to_string(), t.muted),
        ConnectionState::Connected => (pick("●", "+"), "connected".to_string(), t.done),
        ConnectionState::Reconnecting { attempt, .. } => {
            (pick("↻", "~"), format!("reconnecting #{attempt}"), t.muted)
        }
        ConnectionState::NeedsLogin { .. } => (pick("✕", "x"), "needs login".to_string(), t.err),
        ConnectionState::NeedsUpdate { .. } => (pick("✕", "x"), "needs update".to_string(), t.err),
        ConnectionState::Unreachable { .. } => (pick("✕", "x"), "unreachable".to_string(), t.err),
    };
    StateMark { glyph, word, color }
}

impl App {
    pub(super) fn machine_down(&self, machine: MachineId) -> bool {
        self.remotes
            .get(&machine)
            .is_some_and(|remote| remote.state.is_down())
    }

    /// Whether two folders of the sidebar (by model index) are on one machine.
    pub(in crate::client) fn projects_share_machine(&self, a: usize, b: usize) -> bool {
        let model = self.screen_model();
        matches!((model.get(a), model.get(b)), (Some(x), Some(y)) if x.machine == y.machine)
    }

    fn machine_label(&self, machine: MachineId) -> String {
        match self.remotes.get(&machine) {
            Some(remote) => sanitize(&remote.name, false),
            None => THIS_COMPUTER.to_string(),
        }
    }

    /// The name a remote machine's items carry; empty for this computer's, which carry none.
    pub(super) fn machine_label_of(&self, machine: MachineId) -> String {
        if machine == MachineId::LOCAL {
            String::new()
        } else {
            self.machine_label(machine)
        }
    }

    /// The mark that stands for another computer: a laptop with the Nerd glyph set, a plain
    /// screen otherwise, `[]` in ASCII.
    pub(super) fn machine_icon(&self) -> &'static str {
        let set = glyphs::chosen(&self.cfg.ui.glyphs, &self.cfg.icons);
        if set.close == "\u{f00d}" {
            "\u{f0322}"
        } else if self.ascii_marks() {
            "[]"
        } else {
            "▭"
        }
    }

    /// The machine's connection as one dot: `done` when it is up, amber while it connects or
    /// reconnects, `err` when it cannot be reached (it needs a login or an update, or is down).
    pub(super) fn machine_dot(&self, machine: MachineId, t: &Theme) -> (&'static str, Color) {
        let colour = match self.remotes.get(&machine).map(|r| &r.state) {
            Some(ConnectionState::Connected) => t.done,
            Some(ConnectionState::Connecting | ConnectionState::Reconnecting { .. }) => t.blocked,
            _ => t.err,
        };
        (if self.ascii_marks() { "*" } else { "●" }, colour)
    }

    fn ascii_marks(&self) -> bool {
        glyphs::chosen(&self.cfg.ui.glyphs, &self.cfg.icons)
            .needs
            .is_ascii()
    }

    pub(super) fn machine_mark(&self, machine: MachineId, t: &Theme) -> Option<StateMark> {
        let remote = self.remotes.get(&machine)?;
        Some(state_mark(&remote.state, self.ascii_marks(), t))
    }
}

/// The lines that open a machine's group: its header, why it is down, what its daemon reports.
pub(super) fn lines(app: &App, machine: MachineId, t: &Theme) -> Vec<Line> {
    let mut out = vec![Line::Machine(machine)];
    match app.remotes.get(&machine) {
        Some(remote) if remote.state.is_down() => out.extend(down_notes(remote, note_room(app), t)),
        _ => out.extend(
            usage_notes(app, machine)
                .into_iter()
                .map(|text| Line::MachineNote(vec![text], t.muted)),
        ),
    }
    out
}

/// The cells a note has in the sidebar the last frame drew (the narrowest one before the first),
/// as `draw_note` is given them: the Floating card keeps two columns each side, Hairline one.
fn note_room(app: &App) -> usize {
    let width = match app.screen.side_rect.width {
        0 => SIDE_MIN,
        drawn => drawn,
    };
    let margin = if Look::of(&app.cfg.ui).hairline { 2 } else { 4 };
    usize::from(width.saturating_sub(margin)).max(1)
}

fn down_notes(remote: &crate::client::ids::Remote, room: usize, t: &Theme) -> Vec<Line> {
    let down_for = remote.down_since.map_or(Duration::ZERO, |at| at.elapsed());
    let when = age(now_secs().saturating_sub(down_for.as_secs()));
    let why = match &remote.state {
        ConnectionState::NeedsLogin { why }
        | ConnectionState::NeedsUpdate { why }
        | ConnectionState::Unreachable { why, .. } => sanitize(why, false),
        _ => "the connection was lost; trying again".to_string(),
    };
    let since = vec![
        format!("disconnected since {when} ago"),
        format!("disconnected {when} ago"),
        format!("down {when}"),
    ];
    // One row per line, since the rows of the sidebar are what scrolling and clicks go by.
    let reason = popover::wrap_capped(&why, room, REASON_LINES)
        .into_iter()
        .map(|line| Line::MachineNote(vec![line], t.text));
    std::iter::once(Line::MachineNote(since, t.muted))
        .chain(reason)
        .collect()
}

/// What the machine's own daemon reports, never added to another's: a line of plan limits, and
/// one of today's spend and its version when that is not this window's.
fn usage_notes(app: &App, machine: MachineId) -> Vec<String> {
    let Some(snap) = app.snap_of(machine) else {
        return Vec::new();
    };
    let limits: Vec<String> = snap
        .limits
        .iter()
        .filter(|(_, windows)| !windows.is_empty())
        .map(|(agent, windows)| {
            let used: Vec<String> = windows
                .iter()
                .map(|w| format!("{} {}%", w.name, w.used.clamp(0.0, 100.0).round() as u8))
                .collect();
            format!("{agent} {}", used.join(" "))
        })
        .collect();
    let mut spent = Vec::new();
    if snap.spent_today > 0.0 {
        spent.push(format!("${:.2} today", snap.spent_today));
    }
    if machine != MachineId::LOCAL
        && !snap.daemon_version.is_empty()
        && snap.daemon_version != env!("APP_VERSION")
    {
        spent.push(format!("daemon {}", snap.daemon_version));
    }
    [limits.join(" · "), spent.join(" · ")]
        .into_iter()
        .filter(|line| !line.is_empty())
        .collect()
}

/// `text` cut to `max` cells with a `…`, whatever the width of its characters.
fn fit(text: &str, max: usize) -> String {
    if text.width() <= max {
        return text.to_string();
    }
    let mut out = String::new();
    let mut used = 0;
    for c in text.chars() {
        let w = c.width().unwrap_or(0);
        if used + w + 1 > max {
            break;
        }
        out.push(c);
        used += w;
    }
    if max > 0 {
        out.push('…');
    }
    out
}

/// A machine's header in `room` cells: its name at the left and its state at the right. The
/// state gives way to its glyph before the name is cut, and the name is cut before it vanishes.
pub(super) fn header_segs(
    (lead, name): (&str, &str),
    mark: Option<&StateMark>,
    down: bool,
    room: usize,
    t: &Theme,
    st: Style,
) -> Vec<Seg> {
    let kept = NAME_KEPT.min(name.width());
    // The state's word outlasts the machine mark, and the glyph outlasts the word.
    let chip_text = |word: bool| {
        mark.map(|m| {
            if word {
                format!("{} {}", m.glyph, m.word)
            } else {
                m.glyph.to_string()
            }
        })
    };
    let plan = [(true, true), (false, true), (true, false), (false, false)]
        .into_iter()
        .find(|&(with_lead, word)| {
            let lead_w = if with_lead { lead.width() } else { 0 };
            lead_w + kept + chip_text(word).map_or(0, |c| c.width() + 1) <= room
        });
    let (lead, chip) = match plan {
        Some((with_lead, word)) => (if with_lead { lead } else { "" }, chip_text(word)),
        None => ("", None),
    };
    let chip_w = chip.as_ref().map_or(0, |c| c.width() + 1);
    let name = fit(name, room.saturating_sub(lead.width() + chip_w));
    let name_style = if down {
        st.fg(t.muted)
    } else {
        st.fg(t.strong)
    };
    let mut out = vec![
        seg(lead, st.fg(t.muted)),
        seg(name.clone(), name_style.add_modifier(Modifier::BOLD)),
    ];
    if let (Some(chip), Some(m)) = (chip, mark) {
        let gap = room.saturating_sub(lead.width() + name.width() + chip.width());
        out.push(seg(" ".repeat(gap), st));
        out.push(seg(chip, st.fg(m.color).add_modifier(Modifier::BOLD)));
    }
    out
}

/// A machine's header row: from `x` to `end` on `y`.
pub(super) fn draw_header(
    app: &App,
    buf: &mut Buffer,
    (x, end): (u16, u16),
    y: u16,
    machine: MachineId,
    ground: Color,
    t: &Theme,
) {
    let lead = match (machine, app.ascii_marks()) {
        (MachineId::LOCAL, _) => "",
        (_, true) => LEAD_ASCII,
        (_, false) => LEAD,
    };
    let segs = header_segs(
        (lead, &app.machine_label(machine)),
        app.machine_mark(machine, t).as_ref(),
        app.machine_down(machine),
        end.saturating_sub(x) as usize,
        t,
        Style::default().bg(ground),
    );
    put(buf, x, y, &segs, end);
}

/// A line under a header: the longest of its forms that fits, else the last one, cut.
pub(super) fn draw_note(
    buf: &mut Buffer,
    (x, end): (u16, u16),
    y: u16,
    (forms, color): (&[String], Color),
    ground: Color,
) {
    let room = end.saturating_sub(x) as usize;
    let form = forms
        .iter()
        .find(|f| f.width() <= room)
        .or(forms.last())
        .map_or("", String::as_str);
    let text = fit(form, room);
    put(
        buf,
        x,
        y,
        &[seg(text, Style::default().fg(color).bg(ground))],
        end,
    );
}

/// The collapsed sidebar's mark for a machine: its state glyph and its first letter.
pub(super) fn rail_segs(app: &App, machine: MachineId, ground: Color, t: &Theme) -> Vec<Seg> {
    let Some(mark) = app.machine_mark(machine, t) else {
        return Vec::new();
    };
    let initial = app
        .machine_label(machine)
        .chars()
        .find(|c| !c.is_whitespace())
        .map_or_else(|| "·".to_string(), |c| c.to_uppercase().collect());
    let st = Style::default().bg(ground).add_modifier(Modifier::BOLD);
    vec![
        seg(" ", st),
        seg(mark.glyph, st.fg(mark.color)),
        seg(initial, st.fg(t.muted)),
    ]
}

/// A down machine's row, drawn again in the quiet colour: whatever it said is the last report,
/// so none of it may read as live, and amber stays for what needs you now.
pub(super) fn grey(buf: &mut Buffer, r: Rect, t: &Theme) {
    let r = r.intersection(buf.area);
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            let cell = &mut buf[(x, y)];
            if !cell.symbol().trim().is_empty() {
                cell.set_style(Style::default().fg(t.muted).remove_modifier(Modifier::BOLD));
            }
        }
    }
}

/// The working mark of a row: the spinner, held still on a row that is not live.
pub(super) fn working_mark(app: &App, down: bool) -> String {
    if down {
        app.cfg.icons.working.clone()
    } else {
        glyph(app, Status::Working)
    }
}
