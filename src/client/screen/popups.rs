//! Popups: shared panel helpers, jump, open a folder, palette, go to, history, memory.

use super::*;

// ---- overlays ------------------------------------------------------------------------------

/// What a popup does to everything behind it: text toward its ground, all of it toward black.
const BEHIND_TEXT: f32 = 0.6;
const BEHIND_DARK: f32 = 0.45;

/// Dim everything already drawn behind a popup: text toward its ground and all of it darker.
/// Pill ends keep their shape (their colour darkens like the pill's).
pub(in crate::client) fn dim_all(buf: &mut Buffer, area: Rect, t: &Theme) {
    let black = Color::Rgb(0, 0, 0);
    let area = area.intersection(buf.area);
    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let c = &mut buf[(x, y)];
            let bg = if c.bg == Color::Reset { t.bg } else { c.bg };
            let fg = if c.fg == Color::Reset { t.fg } else { c.fg };
            let cap = c.symbol() == CAP_L || c.symbol() == CAP_R;
            c.fg = if cap { blend(fg, black, BEHIND_DARK) } else { blend(blend(fg, bg, BEHIND_TEXT), black, BEHIND_DARK) };
            c.bg = blend(bg, black, BEHIND_DARK);
        }
    }
}

/// The one size every tool window opens at (files, search, changes, pull requests).
pub(in crate::client) fn tool_rect(area: Rect) -> Rect {
    let w = area.width.saturating_sub(8).min(170);
    let h = area.height.saturating_sub(4).min(48);
    Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h }
}

const ESC_CLOSE: &str = "Esc close";

/// A centred modal over the dimmed screen: `panel_at` in the middle of `area`, at most
/// `w`×`h`. A click outside it closes it.
#[allow(clippy::too_many_arguments)]
pub(in crate::client) fn panel(app: &mut App, buf: &mut Buffer, area: Rect, w: u16, h: u16, title: &str, right: &[Seg], t: &Theme) -> Rect {
    let w = w.min(area.width.saturating_sub(2));
    let h = h.min(area.height.saturating_sub(2));
    let r = Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h };
    hit(app, area, ScreenHit::Close);
    panel_at(app, buf, r, title, right, t)
}

/// A modal's frame at `r`: a body of `card`, a title row filled with the accent
/// (the title two in, bold, and `Esc close` at the right), `│` sides and a `─` bottom in
/// `line` with corners by the setting. `right` sits in the title row before `Esc close`.
/// Returns `r`; its contents start two in and two down.
pub(in crate::client) fn panel_at(app: &mut App, buf: &mut Buffer, r: Rect, title: &str, right: &[Seg], t: &Theme) -> Rect {
    let r = r.intersection(buf.area);
    if r.width < 4 || r.height < 2 {
        return r;
    }
    let look = Look::of(&app.cfg.ui);
    let behind = ends_behind(buf, Rect { height: 1, ..r });
    fill(buf, r, t.card);
    let edge = Style::default().fg(t.line).bg(t.card);
    let (right_x, bottom) = (r.right() - 1, r.bottom() - 1);
    for y in r.y + 1..bottom {
        buf[(r.x, y)].set_symbol("│").set_style(edge);
        buf[(right_x, y)].set_symbol("│").set_style(edge);
    }
    for x in r.x..r.right() {
        buf[(x, bottom)].set_symbol("─").set_style(edge);
    }
    let [_, _, bl, br] = corners(Look { rounded: look.rounded || look.caps, ..look });
    buf[(r.x, bottom)].set_symbol(bl).set_style(edge);
    buf[(right_x, bottom)].set_symbol(br).set_style(edge);
    fill(buf, Rect { height: 1, ..r }, t.accent);
    if look.caps {
        soften_block(buf, Rect { height: 1, ..r }, &behind);
    }
    let ink = Style::default().fg(t.acc_ink).bg(t.accent);
    let esc_x = r.right().saturating_sub(ESC_CLOSE.len() as u16 + 2);
    put(buf, esc_x, r.y, &[seg(ESC_CLOSE, ink)], r.right());
    let segs: Vec<Seg> = right.iter().map(|(s, st)| (s.clone(), st.bg(t.accent))).collect();
    let rx = esc_x.saturating_sub(segs_width(&segs) + 2);
    put(buf, rx, r.y, &segs, esc_x.saturating_sub(2));
    put(buf, r.x + 2, r.y, &[seg(title, ink.add_modifier(Modifier::BOLD))], rx.saturating_sub(1));
    hit(app, r, ScreenHit::Noop);
    hit(app, Rect { x: esc_x, y: r.y, width: ESC_CLOSE.len() as u16, height: 1 }, ScreenHit::Close);
    r
}

/// A popover keeps at least this many columns from its left edge to the screen's.
const POPOVER_ROOM: u16 = 40;
/// Below this many rows a popover sits higher.
const POPOVER_SHORT: u16 = 30;

/// Where a popover goes: beside the sidebar's rows (two right of it, five down, three on a
/// short screen), narrower rather than over the sidebar when the screen is small.
pub(in crate::client) fn popover_rect(screen: Rect, side: Rect, w: u16, h: u16) -> Rect {
    let x = (side.right() + 2).min(screen.right().saturating_sub(POPOVER_ROOM)).max(screen.x);
    let w = w.min(screen.right().saturating_sub(x + 2));
    let h = h.min(screen.height.saturating_sub(1));
    let y = (screen.y + if screen.height < POPOVER_SHORT { 3 } else { 5 }).min(screen.bottom().saturating_sub(h + 1));
    Rect { x, y, width: w, height: h }
}

/// A popover's frame at `r`: a body of `card` and a title row of `btn` with
/// ` TITLE ` in `strong` bold, a column in; no border.
pub(in crate::client) fn popover_at(app: &mut App, buf: &mut Buffer, r: Rect, title: &str, t: &Theme) -> Rect {
    let r = r.intersection(buf.area);
    fill(buf, r, t.card);
    fill(buf, Rect { height: 1, ..r }, t.btn);
    put(buf, r.x + 1, r.y, &[seg(format!(" {title} "), Style::default().fg(t.strong).bg(t.btn).add_modifier(Modifier::BOLD))], r.right());
    hit(app, r, ScreenHit::Noop);
    r
}

pub(in crate::client) fn sel_row(app: &App, buf: &mut Buffer, r: Rect, y: u16, sel: bool, t: &Theme) -> Color {
    let row = Rect { x: r.x + 1, y, width: r.width.saturating_sub(2), height: 1 };
    if sel {
        let bg = t.sel_on(t.card);
        row_pill(Look::of(&app.cfg.ui), buf, row.x, y, row.width, bg, t.card);
        put(buf, r.x + 2, y, &[seg("▎", Style::default().fg(t.accent).bg(bg).add_modifier(Modifier::BOLD))], r.right());
        bg
    } else {
        t.card
    }
}

pub(in crate::client) fn hints(t: &Theme, pairs: &[(&str, &str)]) -> Vec<Seg> {
    cap_hints(t, t.card, pairs)
}

/// A panel's key hints in a bar of their own: one row on a ground a step above the panel's,
/// the full width of `bar` with a column of padding, keycaps then what each does. It is the
/// last row of its panel so the actions are in the same place everywhere. `ground` is `card`
/// for a sheet on the app's background and `card2` inside a modal.
pub(in crate::client) fn action_bar(buf: &mut Buffer, bar: Rect, t: &Theme, ground: Color, keys: &[(&str, &str)]) {
    let row = Rect { height: 1, ..bar };
    fill(buf, row, ground);
    let room = row.width.saturating_sub(2);
    put(buf, row.x + 1, row.y, &cap_hints_fit(t, ground, keys, room), row.right().saturating_sub(1));
}

/// The action bar of a bordered modal at `panel`: the row above its bottom border, between its sides.
pub(in crate::client) fn panel_bar(buf: &mut Buffer, panel: Rect, t: &Theme, keys: &[(&str, &str)]) {
    let bar = Rect { x: panel.x + 1, y: panel.bottom().saturating_sub(2), width: panel.width.saturating_sub(2), height: 1 };
    action_bar(buf, bar, t, t.card2, keys);
}

// Jump ----------------------------------------------------------------------------------------

pub(in crate::client) fn jump_list(model: &[Proj]) -> Vec<(Session, String, String, Color)> {
    let mut out = Vec::new();
    for st in [Status::Blocked, Status::Done] {
        let mut these = Vec::new();
        for p in model {
            for w in &p.wts {
                for s in w.sessions.iter().filter(|s| s.status == st) {
                    these.push((s.clone(), p.name.clone(), w.name.clone(), p.color));
                }
            }
        }
        // Waiting longest first.
        these.sort_by_key(|(s, ..)| s.since);
        out.extend(these);
    }
    // Only what needs you or finished: finding any session is Go to's job.
    out
}

// Open a folder ------------------------------------------------------------------------------

impl Finder {
    pub fn new(start: &Path) -> Finder {
        let sep = std::path::MAIN_SEPARATOR;
        let mut f = Finder { q: format!("{}{sep}", tilde(start)), sel: 0, dir: PathBuf::new(), list: Vec::new(), for_setting: None };
        f.refresh();
        f
    }

    /// The typed text as (folder, the part being typed in it).
    pub(in crate::client) fn split(&self) -> (PathBuf, String) {
        let sep = std::path::MAIN_SEPARATOR;
        let mut q = self.q.replace(['/', '\\'], &sep.to_string());
        if let Some(rest) = q.strip_prefix("cd ") {
            q = rest.to_string();
        }
        let home = directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()).unwrap_or_default();
        let full = if let Some(rest) = q.strip_prefix('~') {
            format!("{}{rest}", home.display())
        } else if Path::new(&q).is_absolute() || q.get(1..2) == Some(":") {
            q
        } else {
            format!("{}{sep}{q}", home.display())
        };
        let (dir, part) = match full.rfind(sep) {
            Some(i) => (&full[..=i], full[i + 1..].to_string()),
            None => (full.as_str(), String::new()),
        };
        (normalize(Path::new(dir)), part)
    }

    /// Re-list the folder being typed in.
    pub fn refresh(&mut self) {
        let (dir, part) = self.split();
        if dir != self.dir {
            self.dir = dir.clone();
            let mut list: Vec<(String, bool)> = std::fs::read_dir(&dir)
                .map(|rd| {
                    rd.flatten()
                        .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
                        .map(|e| e.file_name().to_string_lossy().into_owned())
                        .filter(|n| !n.starts_with('.') && !n.starts_with('$'))
                        .map(|n| {
                            let repo = dir.join(&n).join(".git").exists();
                            (n, repo)
                        })
                        .collect()
                })
                .unwrap_or_default();
            list.sort_by_key(|(n, _)| n.to_lowercase());
            self.list = list;
        }
        let _ = part;
        self.sel = self.sel.min(self.rows().len().saturating_sub(1));
    }

    /// Rows shown: "." (open this folder), ".." (up), then matching subfolders.
    pub fn rows(&self) -> Vec<(String, PathBuf, bool)> {
        let (dir, part) = self.split();
        let p = part.to_lowercase();
        let fuzzy = |n: &str| {
            let mut it = p.chars().peekable();
            for c in n.to_lowercase().chars() {
                if it.peek() == Some(&c) {
                    it.next();
                }
            }
            it.peek().is_none()
        };
        let mut out = Vec::new();
        if part.is_empty() {
            out.push((".".to_string(), dir.clone(), dir.join(".git").exists()));
        }
        if dir.parent().is_some() && (part.is_empty() || "..".starts_with(&part)) {
            out.push(("..".to_string(), dir.parent().map(Path::to_path_buf).unwrap_or_default(), false));
        }
        let mut kids: Vec<_> = self.list.iter().filter(|(n, _)| part == ".." || fuzzy(n)).collect();
        kids.sort_by_key(|(n, _)| !n.to_lowercase().starts_with(&p));
        out.extend(kids.into_iter().map(|(n, repo)| (n.clone(), dir.join(n), *repo)));
        out
    }

    /// The rest of the selected name, shown greyed after what's typed.
    pub(in crate::client) fn ghost(&self) -> String {
        let (_, part) = self.split();
        let rows = self.rows();
        match rows.get(self.sel) {
            Some((n, ..)) if !part.is_empty() && n.to_lowercase().starts_with(&part.to_lowercase()) => n[part.len()..].to_string(),
            _ => String::new(),
        }
    }
}

/// Resolve `.` and `..` without touching the disk.
pub(in crate::client) fn normalize(p: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for c in p.components() {
        match c {
            std::path::Component::ParentDir => {
                if out.parent().is_some() {
                    out.pop();
                }
            }
            std::path::Component::CurDir => {}
            c => out.push(c.as_os_str()),
        }
    }
    out
}

pub(in crate::client) fn draw_finder(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, fd: &Finder) {
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    let title = if fd.for_setting.is_some() { "Choose the start folder" } else { "Open a folder" };
    let r = panel(app, buf, area, 92, 24, title, &[], t);
    let input = Rect { x: r.x + 1, y: r.y + 2, width: r.width - 2, height: 1 };
    strip(buf, input, t.card2);
    let s = Style::default().bg(t.card2);
    // A long path shows its end, where you're typing.
    let room = r.width.saturating_sub(10) as usize;
    let ghost = fd.ghost();
    let q = if fd.q.width() + ghost.width() > room {
        let tail: String = fd.q.chars().rev().take(room.saturating_sub(ghost.width() + 1)).collect::<Vec<_>>().into_iter().rev().collect();
        format!("…{tail}")
    } else {
        fd.q.clone()
    };
    put(
        buf,
        r.x + 3,
        input.y,
        &[seg("› ", s.fg(t.accent).add_modifier(Modifier::BOLD)), seg(q, s.fg(t.strong)), seg(ghost, s.fg(t.muted)), seg("█", s.fg(t.accent))],
        r.right() - 1,
    );
    let c = Style::default().bg(t.card);
    put(
        buf,
        r.x + 3,
        r.y + 3,
        &[seg("type a path, ", c.fg(t.muted)), seg("cd ..", c.fg(t.text)), seg(" works too · fuzzy: ", c.fg(t.muted)), seg("~\\c\\sa", c.fg(t.text)), seg(" finds ~\\code\\shop-api", c.fg(t.muted))],
        r.right(),
    );
    put(buf, r.x + 3, r.y + 5, &[seg(tilde(&fd.dir), c.fg(t.muted).add_modifier(Modifier::BOLD))], r.right());
    let model = app.screen_model();
    let rows = fd.rows();
    let max = (r.height.saturating_sub(9)) as usize;
    let start = fd.sel.saturating_sub(max.saturating_sub(1));
    for (i, (n, path, repo)) in rows.iter().enumerate().skip(start).take(max) {
        let y = r.y + 6 + (i - start) as u16;
        let sel = i == fd.sel;
        let bg = sel_row(app, buf, r, y, sel, t);
        let st = Style::default().bg(bg);
        let open = model.iter().any(|p| path_key(&p.path) == path_key(path));
        let (icon, label) = match n.as_str() {
            "." => ("◇ ", format!("open {} here", folder_name(path))),
            ".." => ("↰ ", format!(".. (up to {})", tilde(path))),
            _ => (if *repo { "◆ " } else { "▸ " }, n.clone()),
        };
        let mut ls = st.fg(if sel { t.strong } else { t.fg });
        if sel {
            ls = ls.add_modifier(Modifier::BOLD);
        }
        put(buf, r.x + 3, y, &[seg(icon, st.fg(if *repo || n == "." { t.accent } else { t.muted })), seg(label, ls)], r.right() - 30);
        let tag = if open {
            "has sessions"
        } else if n == "." {
            if *repo { "git repo · opens as a project" } else { "opens as a project" }
        } else if *repo {
            "git repo · opens as a project"
        } else if n == ".." {
            ""
        } else {
            "folder"
        };
        put(buf, r.right().saturating_sub(3 + tag.width() as u16), y, &[seg(tag, st.fg(if open { t.accent } else { t.muted }))], r.right());
        hit(app, Rect { x: r.x + 1, y, width: r.width - 2, height: 1 }, ScreenHit::FinderPick(i));
    }
    if rows.is_empty() {
        put(buf, r.x + 3, r.y + 6, &[seg("nothing matches here", c.fg(t.muted).add_modifier(Modifier::ITALIC))], r.right());
    }
    panel_bar(buf, r, t, &[("Tab", "complete"), ("Enter", "open"), ("↑↓", "choose"), ("Backspace", "edit"), ("Esc", "close")]);
}

// New pane ----------------------------------------------------------------------------------

pub(in crate::client) fn np_agents(app: &App, _project: usize) -> Vec<String> {
    let mut v: Vec<String> = app.cfg.quick.agents.iter().map(|a| a.name.clone()).collect();
    for a in ["claude", "codex", "gemini"] {
        if !v.iter().any(|x| x == a) {
            v.push(a.into());
        }
    }
    v.push("shell".into());
    v.extend(app.cfg.recipes.iter().filter(|r| !r.run.is_empty()).map(|r| format!("⚙ {}", r.name)));
    v.extend(app.cfg.presets.iter().map(|p| format!("★ {}", p.name)));

    v
}

pub(in crate::client) fn preset_of<'a>(app: &'a App, run: &str) -> Option<&'a crate::config::Preset> {
    let name = run.strip_prefix("★ ")?;
    app.cfg.presets.iter().find(|p| p.name == name)
}

/// Where a new agent can go in a git project: the three kinds, then its worktrees by name.
pub(in crate::client) fn np_places(p: &Proj) -> Vec<String> {
    let mut v: Vec<String> = ["new worktree", "new branch", "this branch"].map(String::from).to_vec();
    v.extend(p.wts.iter().filter(|w| !w.main).map(|w| format!("⌥ {}", w.name)));
    v
}

/// A popup with a query row and a scrolling list under it: dims the screen, draws the
/// panel and the query (with `hint` while it's empty), and returns where the rows go and
/// the first one on screen (so `sel` stays visible).
#[allow(clippy::too_many_arguments)]
pub(in crate::client) fn query_list(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, title: &str, w: u16, rows: usize, query: &str, hint: &str, sel: usize) -> (Rect, Rect, usize) {
    dim_all(buf, area, t);
    let h = (rows as u16 + 8).max(12).min(area.height.saturating_sub(4));
    let r = panel(app, buf, area, w, h, title, &[], t);
    let q = Rect { x: r.x + 1, y: r.y + 2, width: r.width.saturating_sub(2), height: 1 };
    strip(buf, q, t.card2);
    let s2 = Style::default().bg(t.card2);
    let mut qs = vec![seg("› ", s2.fg(t.accent).add_modifier(Modifier::BOLD)), seg(query.to_string(), s2.fg(t.strong)), seg("█", s2.fg(t.accent))];
    if query.is_empty() {
        qs.push(seg(format!(" {hint}"), s2.fg(t.muted)));
    }
    put(buf, r.x + 3, q.y, &qs, r.right().saturating_sub(2));
    let list = Rect { x: r.x + 1, y: r.y + 4, width: r.width.saturating_sub(2), height: r.height.saturating_sub(7) };
    let start = sel.saturating_sub(list.height.saturating_sub(1) as usize);
    (r, list, start)
}

/// One row of a `query_list`: its background (highlighted when selected) and the selection
/// marker. Returns the style to draw its text with.
pub(in crate::client) fn list_row(app: &App, buf: &mut Buffer, rr: Rect, on: bool, t: &Theme) -> Style {
    let bg = if on { t.sel_on(t.card) } else { t.card };
    if bg == t.card {
        fill(buf, rr, bg);
    } else {
        row_pill(Look::of(&app.cfg.ui), buf, rr.x, rr.y, rr.width, bg, t.card);
    }
    let st = Style::default().bg(bg);
    if on {
        put(buf, rr.x + 1, rr.y, &[seg("▎", st.fg(t.accent).add_modifier(Modifier::BOLD))], rr.right());
    }
    st
}

/// A working agent's name: the working colour with a bright band sweeping across it.
pub(in crate::client) fn shimmer(text: &str, frame: u64, base: Color, bright: Color, st: Style) -> Vec<Seg> {
    const WIDTH: f32 = 3.0;
    let chars: Vec<char> = text.chars().collect();
    let span = chars.len() as u64 + 8;
    let pos = (frame % span) as f32 - 4.0;
    chars
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            let k = (1.0 - (i as f32 - pos).abs() / WIDTH).max(0.0);
            seg(ch.to_string(), st.fg(crate::client::render::blend(base, bright, k * 0.8)))
        })
        .collect()
}

/// Go to's rows. With nothing typed it opens on what needs you (the Inbox): agents asking,
/// ones that finished, pull requests that need you; then every session by project. Typing
/// finds any session.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) enum GoRow {
    /// A section's heading (not a row you pick).
    Head(&'static str),
    /// An agent asking you: its question and answers on the line.
    Ask(TermId),
    /// One that finished and you haven't looked at: what it said.
    Done(TermId),
    /// Two checkouts changed the same file (index into `heads_up`).
    Heads(usize),
    /// One whose check failed: the result and the first failing output line.
    Failed(TermId),
    /// One whose check could not run: the work was never checked.
    NotRun(TermId),
    /// One whose repo-supplied check command waits for you to allow it.
    Approve(TermId),
    /// A working agent gone silent: it may be stuck.
    Quiet(TermId),
    Sess(usize, TermId),
}

impl GoRow {
    pub fn term(&self) -> Option<TermId> {
        match self {
            GoRow::Ask(t) | GoRow::Done(t) | GoRow::Failed(t) | GoRow::NotRun(t) | GoRow::Approve(t) | GoRow::Quiet(t) | GoRow::Sess(_, t) => Some(*t),
            GoRow::Head(_) | GoRow::Heads(_) => None,
        }
    }

    pub fn pickable(&self) -> bool {
        !matches!(self, GoRow::Head(_))
    }
}

pub(in crate::client) fn goto_rows(model: &[Proj], q: &str, heads: usize) -> Vec<GoRow> {
    let q = q.trim();
    let mut out = Vec::new();
    if q.is_empty() {
        return super::inbox::inbox_rows(model, heads);
    }
    // Typed: every session it matches (by name, agent, title or folder), most urgent first.
    let hit = |text: &str| crate::client::files::fuzzy(q, text).is_some();
    let mut found: Vec<(usize, &Session)> = model
        .iter()
        .enumerate()
        .flat_map(|(pi, p)| p.sessions().map(move |s| (pi, s)))
        .filter(|(pi, s)| hit(&model[*pi].name) || hit(&format!("{} {} {} {}", model[*pi].name, s.name, s.agent, s.title)))
        .collect();
    found.sort_by_key(|(_, s)| (rank(s.status), s.term));
    found.dedup_by_key(|(_, s)| s.term);
    out.push(GoRow::Head("FOUND"));
    out.extend(found.into_iter().map(|(pi, s)| GoRow::Sess(pi, s.term)));
    out
}

pub(in crate::client) fn draw_history(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, sel: usize) {
    let items: Vec<(u64, Option<TermId>, char, String)> = app.history.iter().rev().cloned().collect();
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    let h = (items.len() as u16 + 7).max(10).min(area.height.saturating_sub(4));
    let r = panel(app, buf, area, 104, h, "What happened", &[], t);
    let c = Style::default().bg(t.card);
    let list = Rect { x: r.x + 1, y: r.y + 2, width: r.width - 2, height: r.height.saturating_sub(5) };
    if items.is_empty() {
        put(buf, list.x + 2, list.y, &[seg("nothing yet: agents finishing, asking and ringing show up here", c.fg(t.muted))], list.right());
    }
    let start = sel.saturating_sub(list.height.saturating_sub(1) as usize);
    for (i, (at, term, kind, text)) in items.iter().enumerate().skip(start).take(list.height as usize) {
        let y = list.y + (i - start) as u16;
        let row = Rect { y, height: 1, ..list };
        let on = i == sel;
        let bg = if on { t.sel_on(t.card) } else { t.card };
        fill(buf, row, bg);
        let st = Style::default().bg(bg);
        if on {
            put(buf, row.x, y, &[seg(">", st.fg(t.accent).add_modifier(Modifier::BOLD))], row.right());
        }
        let (g, gc) = match kind {
            '!' => ("●", t.blocked),
            '✓' => ("✓", t.done),
            'x' => ("✕", t.err),
            _ => ("·", t.muted),
        };
        let gone = term.is_some_and(|tm| !app.snap.terms.contains_key(&tm));
        put(
            buf,
            row.x + 2,
            y,
            &[
                seg(format!("{:>4}  ", age(*at)), st.fg(t.muted)),
                seg(format!("{g} "), st.fg(gc).add_modifier(Modifier::BOLD)),
                seg(truncate(text, (row.width as usize).saturating_sub(18)), st.fg(if gone { t.muted } else { t.text })),
            ],
            row.right() - 1,
        );
        hit(app, row, ScreenHit::HistRow(i));
    }
    panel_bar(buf, r, t, &[("Enter", "go to it"), ("c", "clear"), ("Esc", "close")]);
}


