//! The "workspaces" layout from the design handoff: a sidebar of workspaces (one per repo)
//! → their worktrees → the agent in each; borderless panes with one-row title bars; an
//! answer bar on panes that need you; a crumb in the top bar and counts in the status line.
//! Plus the seshi-native views that replace the pane area (Changes, Files), the
//! talk-to-a-worktree modal and the + Pane menu.

use super::render::{status_icon, truncate};
use super::{App, Hit};
use crate::protocol::{Status, TermInfo};

use crate::theme::Theme;
use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use std::path::Path;
use unicode_width::UnicodeWidthStr;

// ---- sidebar model -------------------------------------------------------------------------

/// Case- and separator-insensitive form of a path, for comparisons.
/// A path for comparing: one separator, no trailing one, and case folded only where the
/// file system ignores case (Windows).
pub(super) fn path_key(p: &Path) -> String {
    let s = p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_string();
    if cfg!(windows) { s.to_lowercase() } else { s }
}

/// The question an agent is asking, read off the bottom of its screen.
pub(super) fn question(parser: &vt100::Parser) -> Option<String> {
    static QUEUED: std::sync::LazyLock<regex::Regex> = std::sync::LazyLock::new(|| regex::Regex::new(crate::config::CODEX_QUEUED_QUESTION_PATTERN).unwrap());
    let screen = parser.screen();
    let (rows, cols) = screen.size();
    let lines: Vec<String> = screen.rows(0, cols).collect();
    let tidy = |l: &str| l.trim().trim_matches(|c: char| "│╭╮╰╯─┃ ".contains(c)).trim().to_string();
    let start = lines.len().saturating_sub(24.min(rows as usize));
    lines[start..]
        .iter()
        .rev()
        .map(|l| tidy(l))
        .filter(|l| !l.is_empty())
        .filter(|l| !l.starts_with('❯') && !l.starts_with('›') && !l.chars().next().is_some_and(|c| c.is_ascii_digit()))
        .find(|l| l.ends_with('?') || l.starts_with("Do you want") || l.starts_with("Allow"))
        .or_else(|| lines[start..].iter().any(|l| QUEUED.is_match(l)).then(|| "Unanswered Codex question — open its question list".into()))
}

impl App {

}

/// `~\code\shop-api` style path.
pub(super) fn tilde(p: &Path) -> String {
    // "C:\dev\" and "C:\dev" are the same place; show it one way (but keep "C:\").
    let text = p.to_string_lossy();
    let trimmed = text.trim_end_matches(['\\', '/']);
    let p = if trimmed.len() > 2 && trimmed.len() < text.len() { Path::new(trimmed) } else { p };
    if let Some(home) = directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())
        && let Ok(rel) = p.strip_prefix(&home)
    {
        let sep = std::path::MAIN_SEPARATOR;
        return if rel.as_os_str().is_empty() { "~".into() } else { format!("~{sep}{}", rel.display()) };
    }
    p.display().to_string()
}

/// A pane's name in its title bar: the agent, or what runs in the shell.
pub(super) fn pane_name(t: &TermInfo) -> String {
    match &t.agent {
        Some(a) => a.clone(),
        None if t.is_shell() => "shell".into(),
        None => t.display_name(),
    }
}

// ---- drawing primitives ----------------------------------------------------------------------

/// A styled run of text.
pub(super) type Seg = (String, Style);

pub(super) fn seg(t: impl Into<String>, s: Style) -> Seg {
    (t.into(), s)
}

pub(super) fn segs_width(s: &[Seg]) -> u16 {
    s.iter().map(|(t, _)| t.width() as u16).sum()
}

/// Write segments from (x, y), not past `max_x`; returns the x after the last cell written.
pub(super) fn put(buf: &mut Buffer, x: u16, y: u16, segs: &[Seg], max_x: u16) -> u16 {
    // Off the screen (a popup taller or wider than a small window): draw nothing.
    let area = buf.area;
    if y < area.top() || y >= area.bottom() || x < area.left() {
        return x;
    }
    let max_x = max_x.min(area.right());
    let mut x = x;
    for (text, style) in segs {
        if x >= max_x {
            break;
        }
        let (nx, _) = buf.set_stringn(x, y, text, (max_x - x) as usize, *style);
        x = nx;
    }
    x
}

pub(super) fn fill(buf: &mut Buffer, r: Rect, bg: Color) {
    let r = r.intersection(buf.area);
    buf.set_style(r, Style::reset().bg(bg));
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            if let Some(px) = buf.cell_mut((x, y)) {
                px.set_symbol(" ");
            }
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum BtnKind {
    Primary,
    /// The one that ends or removes something, in a confirmation that is about nothing else.
    Danger,
    /// A choice in a check's modal or sheet that cannot be taken back (Override, Never,
    /// Remove): the plain ground, its words in the failure colour.
    Risky,
    Normal,
    Ghost,
}

/// The design's button: " Label " + "key " on a filled ground. The pointer's tint is laid
/// over every clickable region after drawing (`render`).
pub(super) fn button(t: &Theme, label: &str, key: &str, kind: BtnKind) -> Vec<Seg> {
    let (bg, fg, kfg) = match kind {
        BtnKind::Primary => (t.accent, t.acc_ink, t.acc_ink),
        BtnKind::Danger => (t.danger_fill(), t.ink_on(t.danger_fill()), t.ink_on(t.danger_fill())),
        BtnKind::Normal => (t.btn, t.strong, t.accent),
        BtnKind::Risky => (t.btn, t.err, t.accent),
        BtnKind::Ghost => (t.sidebar_bg, t.text, t.accent),
    };
    let bold = if kind == BtnKind::Ghost { Modifier::empty() } else { Modifier::BOLD };
    // A pill: the same width as " Label key " was.
    let (l, r) = super::screen::ends(bg);
    let mut v = vec![l];
    if key.is_empty() {
        v.push(seg(label.to_string(), Style::default().bg(bg).fg(fg).add_modifier(bold)));
    } else {
        v.push(seg(format!("{label} "), Style::default().bg(bg).fg(fg).add_modifier(bold)));
        v.push(seg(key.to_string(), Style::default().bg(bg).fg(kfg).add_modifier(Modifier::BOLD)));
    }
    v.push(r);
    v
}

/// A key drawn as a keycap: ` key ` raised on `btn`, strong and bold. A combo (`Ctrl+Space`) is
/// one cap per part with a dim `+` between. The one place keys are drawn as caps; its ends
/// follow `ui.rounded`.
pub(super) fn keycap(t: &Theme, key: &str) -> Vec<Seg> {
    let cap = Style::default().bg(t.btn).fg(t.strong).add_modifier(Modifier::BOLD);
    let parts: Vec<&str> = if key.chars().count() > 1 && key.contains('+') { key.split('+').filter(|p| !p.is_empty()).collect() } else { vec![key] };
    let mut out = Vec::new();
    for (i, part) in parts.into_iter().enumerate() {
        if i > 0 {
            out.push(seg("+", Style::default().fg(t.muted)));
        }
        out.extend(super::screen::chip(part, cap));
    }
    out
}

/// Keys as keycaps. Keys separated by two spaces are different sets; a set of four or more
/// single keys shares one cap (` h j k l `), otherwise every key gets its own.
pub(super) fn keycaps(t: &Theme, keys: &str, gap_bg: Color) -> Vec<Seg> {
    let mut out = Vec::new();
    for (gi, group) in keys.split("  ").filter(|g| !g.is_empty()).enumerate() {
        let ks: Vec<&str> = group.split(' ').filter(|k| !k.is_empty()).collect();
        let caps: Vec<String> = if ks.len() >= 4 && ks.iter().all(|k| k.chars().count() == 1) {
            vec![ks.join(" ")]
        } else {
            ks.iter().map(|k| k.to_string()).collect()
        };
        for (ci, c) in caps.iter().enumerate() {
            if gi > 0 || ci > 0 {
                out.push(seg(" ", Style::default().bg(gap_bg)));
            }
            out.extend(keycap(t, c));
        }
    }
    out
}

/// "Enter open   Tab switch" style hints, with keycaps, `gap` between the pairs.
pub(super) fn cap_hints_gap(t: &Theme, bg: Color, gap: &str, pairs: &[(&str, &str)]) -> Vec<Seg> {
    let mut out = Vec::new();
    for (i, (k, what)) in pairs.iter().enumerate() {
        if i > 0 {
            out.push(seg(gap, Style::default().bg(bg)));
        }
        out.extend(keycap(t, k));
        out.push(seg(format!(" {what}"), Style::default().bg(bg).fg(t.text)));
    }
    out
}

pub(super) fn cap_hints(t: &Theme, bg: Color, pairs: &[(&str, &str)]) -> Vec<Seg> {
    cap_hints_gap(t, bg, "   ", pairs)
}

/// The hints that fit in `width` columns: first with the gap between them closed up, then
/// without the one before the last (the last is `Esc close`, which must stay), and the one
/// before that next.
pub(super) fn cap_hints_fit(t: &Theme, bg: Color, pairs: &[(&str, &str)], width: u16) -> Vec<Seg> {
    let mut keep = pairs.to_vec();
    loop {
        for gap in ["   ", "  ", " "] {
            let segs = cap_hints_gap(t, bg, gap, &keep);
            if segs_width(&segs) <= width || keep.len() == 1 && gap == " " {
                return segs;
            }
        }
        keep.remove(keep.len() - 2);
    }
}

/// The key bound to an action after the leader, for button labels.
pub(super) fn key_of(app: &App, a: &crate::keys::Action) -> String {
    app.keymap
        .prefixed_order
        .iter()
        .find(|k| app.keymap.prefixed.get(k) == Some(a))
        .map(|k| k.to_string())
        .unwrap_or_default()
}

pub(super) fn glyph(app: &App, s: Status) -> String {
    match s {
        Status::None => app.cfg.icons.idle.clone(),
        s => status_icon(app, s),
    }
}

pub(super) fn state_label(s: Status) -> &'static str {
    match s {
        Status::Blocked => "needs you",
        Status::Working => "working",
        Status::Done => "done",
        _ => "idle",
    }
}

// ---- the layout ------------------------------------------------------------------------------

/// A title bar: name + location on the left, the given segments on the right.
pub(super) fn title_bar(buf: &mut Buffer, r: Rect, t: &Theme, left: &[Seg], right: &[Seg], focus: bool) {
    let bg = t.bg;
    fill(buf, Rect { height: 1, ..r }, bg);
    let ink = |segs: &[Seg], c: Color| -> Vec<Seg> { segs.iter().map(|(s, st)| (s.clone(), st.bg(bg).fg(c))).collect() };
    let rw = segs_width(right);
    put(buf, r.x + 1, r.y, &ink(left, if focus { t.accent } else { t.strong }), r.right().saturating_sub(rw + 2));
    put(buf, r.right().saturating_sub(rw), r.y, &ink(right, t.muted), r.right());
}

// ---- views that replace the pane area ------------------------------------------------------

/// The bottom action row of a view.
fn action_row(app: &mut App, buf: &mut Buffer, r: Rect, t: &Theme, buttons: &[(&str, &str, BtnKind, Hit)], tail: &[Seg]) {
    let y = r.bottom().saturating_sub(1);
    super::screen::strip(buf, Rect { y, height: 1, ..r }, t.card2);
    // Keycap first: in a tool window you just press the key (no Ctrl+Space).
    let mut x = r.x + 2;
    for (label, key, kind, hit) in buttons {
        let cap = format!(" {key} ");
        let text = format!(" {label} ");
        let w = (cap.width() + text.width()) as u16;
        let br = Rect { x, y, width: w.min(r.right().saturating_sub(x)), height: 1 };
        let cap_st = if *kind == BtnKind::Primary {
            Style::default().bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD)
        } else {
            Style::default().bg(t.btn).fg(t.accent).add_modifier(Modifier::BOLD)
        };
        let text_st = Style::default().bg(t.card2).fg(t.strong);
        let mut segs = super::screen::chip(key, cap_st);
        segs.push(seg(text, text_st));
        x = put(buf, x, y, &segs, r.right()) + 2;
        app.hits.push((br, *hit));
    }
    let tail: Vec<Seg> = tail.iter().map(|(s, st)| (s.clone(), st.bg(t.card2))).collect();
    put(buf, x + 2, y, &tail, r.right());
}

pub(super) fn diff_line(t: &Theme, l: &str, old: &mut u32, new: &mut u32) -> Vec<Seg> {
    let (red, green, cyan, gray) = (t.err, t.done, Color::Rgb(0x3d, 0xd6, 0xc0), t.muted);
    if let Some(rest) = l.strip_prefix("@@") {
        // @@ -a,b +c,d @@ context
        let nums: Vec<u32> = rest
            .split_whitespace()
            .take(2)
            .filter_map(|p| p.trim_start_matches(['-', '+']).split(',').next()?.parse().ok())
            .collect();
        if nums.len() == 2 {
            *old = nums[0];
            *new = nums[1];
        }
        return vec![seg(l.to_string(), Style::default().fg(cyan))];
    }
    if let Some(body) = l.strip_prefix('+') {
        *new += 1;
        return vec![seg("      + ", Style::default().fg(green)), seg(body.to_string(), Style::default().fg(green))];
    }
    if let Some(body) = l.strip_prefix('-') {
        let n = *old;
        *old += 1;
        return vec![seg(format!("{n:>4}  - "), Style::default().fg(red)), seg(body.to_string(), Style::default().fg(red))];
    }
    let body = l.strip_prefix(' ').unwrap_or(l);
    let n = *new;
    *old += 1;
    *new += 1;
    vec![seg(format!("{n:>4}    "), Style::default().fg(gray)), seg(body.to_string(), Style::default().fg(t.fg))]
}

pub(super) fn draw_changes(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, v: &super::views::ChangesView) {
    use super::views::ChangesRow;
    fill(buf, area, t.bg);
    let (green, red, yellow) = (t.done, t.err, Color::Rgb(0xe8, 0xc5, 0x65));
    // The sheet's title says "Changes" and has the ✕: this line is the summary.
    let mut left: Vec<Seg> = Vec::new();
    let right: Vec<Seg> = Vec::new();
    if let Some(r) = &v.review {
        let added: i64 = r.files.iter().map(|f| f.added).sum();
        let removed: i64 = r.files.iter().map(|f| f.removed).sum();
        let agent = if v.agent.is_empty() { String::new() } else { format!("{} · ", v.agent) };
        left.push(seg(format!("{} · {agent}", r.task.branch), Style::default().fg(t.muted)));
        left.push(seg(format!("+{added}"), Style::default().fg(green)));
        left.push(seg(format!(" −{removed}"), Style::default().fg(red)));
        left.push(seg(format!(" · {} files", r.files.len()), Style::default().fg(t.muted)));
        if !v.reviewed.is_empty() {
            let all = v.reviewed.len() == r.files.len();
            left.push(seg(format!(" · {} of {} reviewed", v.reviewed.len(), r.files.len()), Style::default().fg(if all { t.done } else { t.muted })));
        }
    }
    title_bar(buf, area, t, &left, &right, true);

    let body = Rect { y: area.y + 1, height: area.height.saturating_sub(2), ..area };
    let Some(r) = &v.review else {
        let msg = v.error.clone().unwrap_or_else(|| "reading changes…".into());
        put(buf, body.x + 2, body.y + 1, &[seg(msg, Style::default().fg(t.muted))], body.right());
        return;
    };
    let fw: u16 = 34.min(body.width / 2);
    // File tree
    let rows = v.rows();
    let mut y = body.y + 1;
    for (i, row) in rows.iter().enumerate() {
        if y >= body.bottom().saturating_sub(8) {
            break;
        }
        let rr = Rect { x: body.x + 1, y, width: fw - 1, height: 1 };
        let sel = matches!(row, ChangesRow::File(fi, _) if *fi == r.sel);
        if sel {
            fill(buf, rr, t.sel_on(t.bg));
            put(buf, body.x + 1, y, &[seg("▎", Style::default().fg(t.accent).bg(t.sel_on(t.bg)).add_modifier(Modifier::BOLD))], body.right());
        }
        let bg = if sel { t.sel_on(t.bg) } else { t.bg };
        let s = Style::default().bg(bg);
        match row {
            ChangesRow::Reviewed(n) => {
                put(buf, body.x + 2, y, &[seg(format!("REVIEWED · {n}"), s.fg(t.muted).add_modifier(Modifier::BOLD))], body.x + fw);
            }
            ChangesRow::Dir(name, depth) => {
                put(buf, body.x + 2, y, &[seg(format!("{}▾ {name}", "  ".repeat(*depth)), s.fg(t.text))], body.x + fw);
            }
            ChangesRow::File(fi, depth) => {
                let f = &r.files[*fi];
                let done = v.reviewed.contains(&f.path);
                let name = Path::new(&f.path).file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();
                let name = if done { format!("✓ {name}") } else { name };
                let mut ns = s.fg(if sel { t.strong } else if done { t.muted } else { t.fg });
                if sel {
                    ns = ns.add_modifier(Modifier::BOLD);
                }
                put(buf, body.x + 2, y, &[seg(format!("{}  ", "  ".repeat(*depth)), s.fg(t.muted)), seg(name, ns)], body.x + fw - 10);
                let counts = if f.untracked { format!("+{}", f.added) } else { format!("+{} −{}", f.added, f.removed) };
                let (letter, lc) = if f.untracked { ("A", green) } else { ("M", yellow) };
                let mark = vec![seg(counts, s.fg(t.muted)), seg(format!(" {letter}"), s.fg(lc).add_modifier(Modifier::BOLD))];
                let mw = segs_width(&mark);
                put(buf, body.x + fw - mw - 1, y, &mark, body.x + fw);
            }
        }
        app.hits.push((rr, Hit::Button(super::Btn::Row(i))));
        y += 1;
    }
    if r.files.is_empty() {
        put(buf, body.x + 2, body.y + 1, &[seg("No changes yet.", Style::default().fg(t.muted))], body.x + fw);
    }
    // What the agent said about it.
    if !v.said.is_empty() {
        let mut sy = y + 1;
        let who = if v.agent.is_empty() { "AGENT".to_string() } else { v.agent.to_uppercase() };
        put(buf, body.x + 2, sy, &[seg(format!("{who} SAYS"), Style::default().fg(t.muted).add_modifier(Modifier::BOLD))], body.x + fw);
        sy += 1;
        for line in super::views::wrap(&v.said, (fw - 3) as usize) {
            if sy >= body.bottom() {
                break;
            }
            put(buf, body.x + 2, sy, &[seg(line, Style::default().fg(t.text))], body.x + fw);
            sy += 1;
        }
    }
    for yy in body.top()..body.bottom() {
        if let Some(px) = buf.cell_mut((body.x + fw, yy)) {
            px.set_symbol("│").set_style(Style::default().fg(t.line).bg(t.bg));
        }
    }
    // Diff of the selected file
    let dx = body.x + fw + 3;
    if let Some(f) = r.files.get(r.sel) {
        let header = vec![
            seg(f.path.clone(), Style::default().fg(t.strong).add_modifier(Modifier::BOLD)),
            seg(if f.untracked { format!("   +{} new", f.added) } else { format!("   +{} −{}", f.added, f.removed) }, Style::default().fg(t.muted)),
        ];
        put(buf, dx, body.y + 1, &header, body.right());
    }
    let (mut old, mut new) = (1u32, 1u32);
    let lines: Vec<Vec<Seg>> = r.diff.iter().map(|l| diff_line(t, l, &mut old, &mut new)).collect();
    for (k, l) in lines.iter().skip(r.scroll as usize).enumerate() {
        let yy = body.y + 3 + k as u16;
        if yy >= body.bottom() {
            break;
        }
        put(buf, dx, yy, l, body.right().saturating_sub(1));
    }
    // Actions
    let who = if v.agent.is_empty() { "agent".to_string() } else { v.agent.clone() };
    let reply = format!("Reply to {who}");
    let base = r.task.base.clone();
    let merge = format!("Merge into {base}");
    if let Some((q, _)) = &v.confirm {
        action_row(
            app,
            buf,
            area,
            t,
            &[("Yes", "y", BtnKind::Primary, Hit::Button(super::Btn::ViewKey('y'))), ("No", "n", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('n')))],
            &[seg(q.clone(), Style::default().fg(t.strong).add_modifier(Modifier::BOLD))],
        );
    } else if v.linked {
        action_row(
            app,
            buf,
            area,
            t,
            &[
                ("Commit", "c", BtnKind::Primary, Hit::Button(super::Btn::ViewKey('c'))),
                (&merge, "m", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('m'))),
                ("Open PR", "p", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('p'))),
                ("Editor", "e", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('e'))),
                ("Reviewed", "x", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('x'))),
                (&reply, "r", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('r'))),
            ],
            &[seg("d", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)), seg(" discard", Style::default().fg(t.err))],
        );
        app.hits.push((Rect { x: area.right().saturating_sub(1), y: area.bottom() - 1, width: 1, height: 1 }, Hit::Button(super::Btn::ViewKey('d'))));
    } else {
        action_row(
            app,
            buf,
            area,
            t,
            &[
                ("Commit", "c", BtnKind::Primary, Hit::Button(super::Btn::ViewKey('c'))),
                ("Open PR", "p", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('p'))),
                ("Editor", "e", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('e'))),
                ("Reviewed", "x", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('x'))),
                (&reply, "r", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('r'))),
            ],
            &[seg("Esc close", Style::default().fg(t.muted))],
        );
    }
}

pub(super) fn draw_files(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, v: &super::views::FilesTree) {
    fill(buf, area, t.bg);
    let (green, yellow) = (t.done, Color::Rgb(0xe8, 0xc5, 0x65));
    let branch = v.branch.clone().map(|b| format!(" · {b}")).unwrap_or_default();
    let left = vec![
        seg("files  ", Style::default().fg(t.strong).add_modifier(Modifier::BOLD)),
        seg(format!("{}{branch}", tilde(&v.root)), Style::default().fg(t.muted)),
    ];
    let filter = if v.filtering || !v.filter.is_empty() { format!("/{}  ", v.filter) } else { "/ filter  ".into() };
    let mode = if v.recent { "Tab project  " } else { "Tab recent  " };
    let right = vec![
        seg(mode, Style::default().fg(t.muted)),
        seg(filter, Style::default().fg(if v.filtering { t.accent } else { t.muted })),
        seg("✕ ", Style::default().fg(t.muted)),
    ];
    title_bar(buf, area, t, &left, &right, true);
    app.hits.push((Rect { x: area.right().saturating_sub(2), y: area.y, width: 2, height: 1 }, Hit::Button(super::Btn::CloseView)));

    let body = Rect { y: area.y + 1, height: area.height.saturating_sub(2), ..area };
    let fw: u16 = 44.min(body.width / 2);
    let rows = v.visible();
    let list_h = body.height.saturating_sub(4) as usize;
    let start = v.sel.saturating_sub(list_h.saturating_sub(1));
    for (k, (i, e)) in rows.iter().skip(start).take(list_h).enumerate() {
        let y = body.y + 1 + (k as u16);
        let sel = *i == v.sel;
        let rr = Rect { x: body.x + 1, y, width: fw - 1, height: 1 };
        let bg = if sel { t.sel_on(t.bg) } else { t.bg };
        if sel {
            fill(buf, rr, bg);
        }
        if sel {
            put(buf, body.x + 1, y, &[seg(">", Style::default().fg(t.accent).bg(bg).add_modifier(Modifier::BOLD))], body.right());
        }
        let s = Style::default().bg(bg);
        let lead = if e.is_dir { if v.expanded.contains(&e.path) { "▾ " } else { "▸ " } } else { "  " };
        let mut ns = s.fg(if e.is_dir { t.text } else if sel { t.strong } else { t.fg });
        if sel {
            ns = ns.add_modifier(Modifier::BOLD);
        }
        put(buf, body.x + 2, y, &[seg(format!("{}{lead}", "  ".repeat(e.depth)), s.fg(t.muted)), seg(e.label.clone(), ns)], body.x + fw - 5);
        let mut marks: Vec<Seg> = Vec::new();
        if v.editing.contains_key(&e.rel) {
            marks.push(seg(app.cfg.icons.blocked.clone(), s.fg(t.blocked)));
        }
        if let Some(m) = v.git.get(&e.rel) {
            marks.push(seg(format!(" {m}"), s.fg(if *m == 'A' { green } else { yellow }).add_modifier(Modifier::BOLD)));
        }
        let mw = segs_width(&marks);
        put(buf, body.x + fw - mw - 1, y, &marks, body.x + fw);
        app.hits.push((rr, Hit::Button(super::Btn::Row(*i))));
    }
    if rows.is_empty() {
        let msg = if v.loading { "reading files…" } else if v.recent { "Nothing new in Downloads, Desktop, Documents or here lately." } else { "No files match." };
        put(buf, body.x + 2, body.y + 1, &[seg(msg, Style::default().fg(t.muted))], body.x + fw);
    }
    if !v.recent {
        let legend = vec![
            seg("M", Style::default().fg(yellow).add_modifier(Modifier::BOLD)),
            seg(" changed   ", Style::default().fg(t.muted)),
            seg("A", Style::default().fg(green).add_modifier(Modifier::BOLD)),
            seg(" new   ", Style::default().fg(t.muted)),
            seg(app.cfg.icons.blocked.clone(), Style::default().fg(t.blocked)),
            seg(" an agent is editing", Style::default().fg(t.muted)),
        ];
        put(buf, body.x + 2, body.bottom().saturating_sub(2), &legend, body.x + fw);
    }
    for yy in body.top()..body.bottom() {
        if let Some(px) = buf.cell_mut((body.x + fw, yy)) {
            px.set_symbol("│").set_style(Style::default().fg(t.line).bg(t.bg));
        }
    }
    // Preview
    let px = body.x + fw + 3;
    if let Some((path, lines)) = &v.preview {
        let rel = path.strip_prefix(&v.root).map(|p| p.display().to_string()).unwrap_or_else(|_| tilde(path));
        #[allow(unused_assignments)]
        let mut header = vec![seg(rel, Style::default().fg(t.strong).add_modifier(Modifier::BOLD))];
        if let Some(who) = v.editing_by(path) {
            header.push(seg(format!("   {who} is editing this file"), Style::default().fg(t.muted).add_modifier(Modifier::ITALIC)));
        }
        let prect = Rect { x: px, y: body.y + 3, width: body.right().saturating_sub(px + 1), height: body.bottom().saturating_sub(body.y + 3) };
        app.screen.preview_rect = prect;
        match &v.edit {
            Some(ed) => {
                header = vec![
                    seg(ed.path.strip_prefix(&v.root).map(|p| p.display().to_string()).unwrap_or_default(), Style::default().fg(t.strong).add_modifier(Modifier::BOLD)),
                    seg(if ed.dirty { "  ● unsaved" } else { "  editing" }, Style::default().fg(t.accent)),
                    seg("   Ctrl+S save · Esc done", Style::default().fg(t.muted)),
                ];
                put(buf, px, body.y + 1, &header, body.right());
                for (k, l) in ed.lines.iter().enumerate().skip(v.scroll).take(prect.height as usize) {
                    let yy = prect.y + (k - v.scroll) as u16;
                    let num = Style::default().fg(if k == ed.row { t.accent } else { t.muted });
                    let mut segs = vec![seg(format!("{:>4}  ", k + 1), num)];
                    segs.extend(super::views::highlight(&l.replace('\t', "    "), t));
                    put(buf, px, yy, &segs, prect.right());
                    if k == ed.row {
                        let cx = px + 6 + ed.cursor_x() as u16;
                        if cx < prect.right() {
                            let cell = &mut buf[(cx, yy)];
                            let sym = if cell.symbol().trim().is_empty() { " ".to_string() } else { cell.symbol().to_string() };
                            cell.set_symbol(&sym).set_style(Style::default().bg(t.accent).fg(t.acc_ink));
                        }
                    }
                }
            }
            None => {
                if v.in_preview {
                    header.push(seg("   ↑↓ PgUp PgDn read · i edit · ← back", Style::default().fg(t.accent)));
                }
                put(buf, px, body.y + 1, &header, body.right());
                // The side the arrows are on gets the accent line.
                if v.in_preview {
                    for yy in body.top()..body.bottom() {
                        if let Some(px) = buf.cell_mut((body.x + fw, yy)) {
                            px.set_symbol("┃").set_style(Style::default().fg(t.accent).bg(t.bg));
                        }
                    }
                }
                for (k, l) in lines.iter().enumerate().skip(v.scroll).take(prect.height as usize) {
                    let yy = prect.y + (k - v.scroll) as u16;
                    let mut segs = vec![seg(format!("{:>4}  ", k + 1), Style::default().fg(t.muted))];
                    segs.extend(super::views::highlight(l, t));
                    put(buf, px, yy, &segs, prect.right());
                }
                if lines.len() > prect.height as usize {
                    let at = format!(" {}–{} of {} ", v.scroll + 1, (v.scroll + prect.height as usize).min(lines.len()), lines.len());
                    put(buf, body.right().saturating_sub(at.width() as u16 + 1), body.y + 1, &[seg(at, Style::default().fg(t.muted))], body.right());
                }
            }
        }
    }
    let who = app.focused().and_then(|id| app.snap.terms.get(&id)).map(pane_name).unwrap_or_else(|| "the pane".into());
    let insert = format!("Insert path into {who}");
    action_row(
        app,
        buf,
        area,
        t,
        &[
            (&insert, "Enter", BtnKind::Primary, Hit::Button(super::Btn::ViewKey('\n'))),
            ("Edit here", "i", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('i'))),
            ("Editor", "e", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('e'))),
            ("Open", "o", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('o'))),
            ("Copy path", "y", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('y'))),
            ("Show diff", "d", BtnKind::Normal, Hit::Button(super::Btn::ViewKey('d'))),
        ],
        &[seg("PgUp PgDn / wheel scroll", Style::default().fg(t.muted))],
    );
}

// ---- modals: talk, + Pane -----------------------------------------------------------------

/// What one key of a set does, for the Keys page where each is changed on its own.
fn single_label(a: &crate::keys::Action) -> &'static str {
    use crate::keys::Action as A;
    use crate::layout::Dir::*;
    match a {
        A::Focus(Left) => "Focus the pane left",
        A::Focus(Up) => "Focus the pane above",
        A::Focus(Down) => "Focus the pane below",
        A::Focus(Right) => "Focus the pane right",
        A::Resize(Left) => "Resize left",
        A::Resize(Down) => "Resize down",
        A::Resize(Up) => "Resize up",
        A::Resize(Right) => "Resize right",
        A::ScrollUp => "Scroll up",
        A::ScrollDown => "Scroll down",
        A::PrevPrompt => "Previous command",
        A::NextPrompt => "Next command",
        _ => "",
    }
}

/// The shortcut rows by group: (group, [(what it does, the actions it covers)]). The key map's
/// own list, with a row that covers a set of keys (the arrows) split into one per key so each
/// can be changed on its own.
#[allow(clippy::type_complexity)]
pub(super) fn key_rows() -> Vec<(&'static str, Vec<(&'static str, Vec<crate::keys::Action>)>)> {
    super::screen::key_groups()
        .into_iter()
        .map(|(group, rows)| {
            let rows = rows
                .into_iter()
                .flat_map(|r| {
                    if r.acts.len() > 1 {
                        r.acts.into_iter().map(|a| (single_label(&a), vec![a])).collect::<Vec<_>>()
                    } else {
                        vec![(r.label, r.acts)]
                    }
                })
                .collect();
            (group, rows)
        })
        .collect()
}

// ---- settings view -------------------------------------------------------------------------

/// A row of a settings page: a setting, or (on the Keys page) a shortcut.
#[derive(Debug, Clone)]
pub(super) enum SRow {
    Setting(&'static super::modal::Setting),
    Bind { label: &'static str, acts: Vec<crate::keys::Action> },
    /// A line of the Agents page that is not a setting: what seshi can tell about an agent.
    Agent(AgentRow),
}

/// The AGENT STATUS rows of Settings > Agents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AgentRow {
    Claude,
    Codex,
    /// Opens the Connect agents popup.
    SetUp,
}

pub(super) const AGENT_STATUS_GROUP: &str = "AGENT STATUS";

impl AgentRow {
    pub(super) fn label(self) -> &'static str {
        match self {
            AgentRow::Claude => "Claude",
            AgentRow::Codex => "Codex",
            AgentRow::SetUp => "Set up another agent…",
        }
    }

    pub(super) fn help(self) -> &'static str {
        match self {
            AgentRow::Claude => "Seshi reads working, needs you and done from Claude's hooks. Missing? Run seshi integrate claude.",
            AgentRow::Codex => "No hooks needed: seshi watches the Codex process.",
            AgentRow::SetUp => "Connect another agent so it reports working, needs you and done itself. Enter opens the list.",
        }
    }
}

/// The small caps heading a settings row sits under.
pub(super) fn settings_group(row: &SRow) -> &'static str {
    match row {
        SRow::Bind { label, .. } => key_rows().into_iter().find(|(_, items)| items.iter().any(|(l, _)| l == label)).map(|(g, _)| g).unwrap_or("KEYS"),
        SRow::Agent(_) => AGENT_STATUS_GROUP,
        SRow::Setting(s) => match s.path {
            "sessions.agent" | "sessions.where" => "NEW SESSIONS",
            "sessions.helpers" | "sessions.helpers_at_once" => "HELPERS",
            "ui.subagent_count" => "SUBAGENTS",
            "theme" => "THEME",
            "ui.spinner" | "ui.glyphs" => "STATUS INDICATORS",
            "prefix" | "ui.mouse" | "ui.which_key" | "ui.alt_arrows" => "INPUT",
            "ui.layout" | "ui.background" | "ui.sidebar_position" | "ui.motion" => "LAYOUT",
            "ui.splash" => "STARTUP",
            "ui.update_check" => "UPDATES",
            "shell" | "editor" | "shell_integration" | "ui.start_dir" => "SHELL",
            "ui.attention_sort" => "SIDEBAR",
            "ui.panes" | "ui.corners" | "ui.gap" | "ui.dim" | "ui.focus_border" | "ui.rounded" => "PANES",
            p if p.starts_with("notify.") => "ALERTS",
            "worktree.delete_with_last" | "worktree.per_agent" | "worktree.command" => "WORKTREES",
            p if p.starts_with("restore.") => "RESTARTS",
            "scrollback" => "HISTORY",
            p if p.starts_with("mcp.") => "AGENTS TALKING TO AGENTS",
            p if p.starts_with("detection.") => "STATUS DETECTION",
            _ => "OTHER",
        },
    }
}

/// A theme's name as people say it.
pub(super) fn theme_label(name: &str) -> String {
    if let Some((_, l)) = crate::theme::DESIGN.iter().find(|(n, _)| *n == name) {
        return l.to_string();
    }
    name.split('-').map(|w| {
        let mut c = w.chars();
        c.next().map(|f| f.to_uppercase().collect::<String>() + c.as_str()).unwrap_or_default()
    }).collect::<Vec<_>>().join(" ")
}

/// The rows of one settings page.
pub(super) fn settings_rows(cat: super::modal::Cat) -> Vec<SRow> {
    let mut rows: Vec<SRow> = Vec::new();
    if cat == super::modal::Cat::Agents {
        rows.extend([AgentRow::Claude, AgentRow::Codex, AgentRow::SetUp].map(SRow::Agent));
    }
    rows.extend(super::modal::in_cat(cat).into_iter().map(SRow::Setting));
    // Grouped under their headings, in the order the headings first come.
    let order: Vec<&str> = rows.iter().map(settings_group).fold(Vec::new(), |mut v, g| {
        if !v.contains(&g) {
            v.push(g);
        }
        v
    });
    const FIRST: [&str; 8] = ["INPUT", "STARTUP", "LAYOUT", "SHELL", "NEW SESSIONS", "HELPERS", AGENT_STATUS_GROUP, "SUBAGENTS"];
    let order: Vec<&str> = FIRST.into_iter().filter(|g| order.contains(g)).chain(order.iter().copied().filter(|g| !FIRST.contains(g))).collect();
    rows.sort_by_key(|r| order.iter().position(|g| *g == settings_group(r)).unwrap_or(99));
    if cat == super::modal::Cat::Keys {
        for (_, items) in key_rows() {
            for (label, acts) in items {
                rows.push(SRow::Bind { label, acts });
            }
        }
    }
    rows
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SettingsView {
    pub cat: usize,
    pub sel: usize,
    pub editing: Option<String>,
    /// Waiting for a key: the leader, or a new key for this row's action.
    pub capturing: bool,
    pub scroll: u16,
}

pub(super) fn control(app: &App, t: &Theme, row: &SRow, v: &SettingsView, selected: bool) -> Vec<Seg> {
    use super::modal::Kind;
    let cfg = &app.cfg;
    if selected && v.capturing {
        return super::screen::chip("press the new key…", Style::default().bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD));
    }
    match row {
        SRow::Bind { acts, .. } => {
            let keys = key_text(app, acts);
            if keys.is_empty() {
                vec![seg("not set", Style::default().fg(t.muted))]
            } else {
                keycaps(t, &keys, t.bg)
            }
        }
        SRow::Agent(row) => agent_control(app, t, *row),
        SRow::Setting(s) => {
            if s.path == "theme" {
                let name = if cfg.theme.is_empty() { crate::theme::DEFAULT } else { &cfg.theme };
                return vec![seg(format!(" {} ", theme_label(name)), Style::default().fg(t.strong)), seg("Enter browse", Style::default().fg(t.accent))];
            }
            let cur = super::modal::current(cfg, s.path);
            match s.kind {
                Kind::Spinner => vec![
                    seg(format!(" {} ", super::modal::display(cfg, s)), Style::default().fg(t.strong)),
                    seg("Enter browse", Style::default().fg(t.accent)),
                ],
                Kind::Bool => {
                    if cur.and_then(|v| v.as_bool()).unwrap_or(false) {
                        super::screen::chip("● on ", Style::default().bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD))
                    } else {
                        super::screen::chip("○ off", Style::default().bg(t.btn).fg(t.muted).add_modifier(Modifier::BOLD))
                    }
                }
                Kind::Number { min, max, .. } => {
                    let val = cur.and_then(|v| v.as_float()).unwrap_or(min);
                    let filled = (((val - min) / (max - min)) * 10.0).round() as usize;
                    vec![
                        seg("▰".repeat(filled.min(10)), Style::default().fg(t.accent)),
                        seg("▱".repeat(10 - filled.min(10)), Style::default().fg(t.line)),
                        seg(format!(" {val:.2}"), Style::default().fg(t.strong)),
                    ]
                }
                Kind::Key => {
                    keycap(t, &super::modal::display(cfg, s).replace("C-", "Ctrl+"))
                }
                Kind::Text | Kind::Folder | Kind::Program(_) => {
                    if selected && let Some(e) = &v.editing {
                        vec![seg(format!(" {e}"), Style::default().bg(t.card2).fg(t.strong)), seg("█ ", Style::default().bg(t.card2).fg(t.accent))]
                    } else {
                        let shown = super::modal::display(cfg, s);
                        let shown = if shown.is_empty() || shown == "(shell)" { "(default)".to_string() } else { shown };
                        super::screen::chip(&truncate(&shown, 28), Style::default().bg(t.btn).fg(t.strong))
                    }
                }
                Kind::Choice(_) | Kind::Int { .. } => {
                    let a = Style::default().fg(if selected { t.accent } else { t.muted }).add_modifier(Modifier::BOLD);
                    vec![seg("‹ ", a), seg(super::modal::display(cfg, s), Style::default().fg(t.strong).add_modifier(Modifier::BOLD)), seg(" ›", a)]
                }
            }
        }
    }
}

/// What seshi can tell about an agent, as the value of its AGENT STATUS row.
fn agent_control(app: &App, t: &Theme, row: AgentRow) -> Vec<Seg> {
    let ok = Style::default().fg(t.done);
    match row {
        AgentRow::Claude => match app.claude_hooks {
            Some(true) => vec![seg("✓ hooks installed", ok)],
            Some(false) => vec![seg("hooks not installed", Style::default().fg(t.muted))],
            None => vec![seg("checking…", Style::default().fg(t.muted))],
        },
        AgentRow::Codex => vec![seg("✓ read from the process", ok)],
        AgentRow::SetUp => vec![seg("Enter", Style::default().fg(t.accent))],
    }
}

/// The current keys for a shortcut row (sets separated by two spaces, as keycaps expects).
pub(super) fn key_text(app: &App, acts: &[crate::keys::Action]) -> String {
    let per: Vec<Vec<String>> = acts
        .iter()
        .map(|a| app.keymap.prefixed_order.iter().filter(|k| app.keymap.prefixed.get(k) == Some(a)).map(pretty_key).collect())
        .collect();
    let mut sets = Vec::new();
    for rank in 0..per.iter().map(Vec::len).max().unwrap_or(0) {
        let set: Vec<String> = per.iter().filter_map(|l| l.get(rank).cloned()).collect();
        if !set.is_empty() {
            sets.push(set.join(" "));
        }
    }
    sets.join("  ")
}

pub(super) fn pretty_key(k: &crate::keys::KeySpec) -> String {
    match k.to_string().as_str() {
        "Left" => "←".into(),
        "Right" => "→".into(),
        "Up" => "↑".into(),
        "Down" => "↓".into(),
        "PageUp" => "PgUp".into(),
        "PageDown" => "PgDn".into(),
        other => other.to_string(),
    }
}

/// A key with its modifiers spelled out: `Alt+1`, `Ctrl+Space`.
pub(super) fn long_key(k: &crate::keys::KeySpec) -> String {
    use crossterm::event::KeyModifiers as M;
    let mut out = String::new();
    for (m, name) in [(M::CONTROL, "Ctrl+"), (M::ALT, "Alt+"), (M::SHIFT, "Shift+")] {
        if k.mods.contains(m) {
            out.push_str(name);
        }
    }
    let code = match k.code {
        crossterm::event::KeyCode::Char(' ') => "Space".to_string(),
        crossterm::event::KeyCode::Char(c) => c.to_string(),
        _ => {
            let bare = crate::keys::KeySpec { mods: M::NONE, ..*k };
            pretty_key(&bare)
        }
    };
    out + &code
}
