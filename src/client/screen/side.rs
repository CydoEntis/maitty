//! The sidebar card: projects, their worktrees and sessions, each session's rows, and the
//! lines they are drawn from.

use super::*;

/// A tool's mark for a tight sidebar row: one glyph, or two letters for a tool without one.
fn tool_icon(agent: &str) -> String {
    match agent {
        "claude" => "✻".into(),
        "codex" => "◆".into(),
        "gemini" => "✦".into(),
        "opencode" => "▣".into(),
        "pi" => "π".into(),
        "shell" => "›".into(),
        other => other.chars().take(2).collect(),
    }
}

/// A line of the sidebar tree.
#[derive(Debug, Clone)]
pub(in crate::client) enum Line {
    /// The heading over the other machines' sessions (with how many).
    Section(Kind, usize),
    Proj(usize),
    /// A heading: BRANCHES or WORKTREES.
    /// An agent or shell (pi, wi, si), and how deep it sits in a fleet (0: not under anyone).
    Sess(usize, usize, usize, u8),
    /// A dim line under an agent: what it's on, its question, a subagent. Shares the
    /// agent's highlight.
    Note(String, Color, TermId),
    /// The check result under a session's row.
    Check(TermId),
    /// A legacy live helper label.
    Subagent(String, TermId),
    /// A runtime-reported child with a stable identity and retained lifecycle.
    SubagentRecord(Box<crate::protocol::SubagentActivity>, TermId, usize),
    /// The heading over a session's subagents: how many, and whether they're listed.
    Subs(TermId, usize, bool),
    /// Nothing running in a folder project.
    Empty(usize),
    Gap,
}

/// What a sidebar row shows of a session: the agent and its state on top, what it's working
/// on (or asking) underneath.
/// A session's load is said under its row from here: a fifth of the machine's CPU, or two
/// gigabytes. And in the working colour from here, where it's the reason the PC is slow.
const LOAD_SAID_CPU: f32 = 20.0;
const LOAD_SAID_MEM: u64 = 2 << 30;
const LOAD_HEAVY_CPU: f32 = 50.0;
const LOAD_HEAVY_MEM: u64 = 6 << 30;

/// "▲ 38% CPU · 2.1 GB" for a session that's taking a real share of the machine, and whether
/// it's heavy. Nothing for one that isn't.
pub(in crate::client) fn load_note(cpu: f32, mem: u64) -> Option<(String, bool)> {
    if cpu < LOAD_SAID_CPU && mem < LOAD_SAID_MEM {
        return None;
    }
    let gb = mem as f64 / f64::from(1u32 << 30);
    Some((format!("▲ {cpu:.0}% CPU · {gb:.1} GB"), cpu >= LOAD_HEAVY_CPU || mem >= LOAD_HEAVY_MEM))
}

pub(in crate::client) fn session_lines(s: &Session, subs_closed: bool, t: &Theme, out: &mut Vec<Line>) {
    let notes_c = t.muted;
    if check::shown(s).is_some() {
        out.push(Line::Check(s.term));
    }
    if let Some(q) = &s.question {
        out.push(Line::Note(q.clone(), blend(t.blocked, t.sidebar_bg, 0.25), s.term));
    }
    // What it's been told to do next, once this turn is over.
    if let Some(next) = s.queued.first() {
        let more = if s.queued.len() > 1 { format!(" (+{})", s.queued.len() - 1) } else { String::new() };
        out.push(Line::Note(format!("⏵ next: {next}{more}"), notes_c, s.term));
    }
    // Its subagents, each on a line, under a heading that folds them away.
    if !s.subagents.is_empty()
        || s.subagent_activity
            .iter()
            .any(|activity| activity.state.active())
    {
        let has_record_for_live_name = |name: &str| {
            s.subagent_activity.iter().any(|activity| {
                activity.state.active() && activity.name.eq_ignore_ascii_case(name)
            })
        };
        let legacy: Vec<_> = s.subagents.iter().filter(|name| !has_record_for_live_name(name)).collect();
        let total = s.subagent_activity.iter().filter(|activity| activity.state.active()).count() + legacy.len();
        out.push(Line::Subs(s.term, total, !subs_closed));
        if !subs_closed {
            let mut shown = 0;
            for (index, activity) in s.subagent_activity.iter().enumerate().filter(|(_, activity)| activity.state.active()) {
                if shown == SUBS_LISTED { break; }
                out.push(Line::SubagentRecord(Box::new(activity.clone()), s.term, index));
                shown += 1;
            }
            for sub in &legacy {
                if shown == SUBS_LISTED { break; }
                let stage = subagent_stage(sub, None);
                out.push(Line::Subagent(stage.to_string(), s.term));
                shown += 1;
            }
            if total > shown {
                out.push(Line::Note(format!("  +{} more", total - shown), notes_c, s.term));
            }
        }
    }
}

/// Infer a short stage only when the task or role provides recognizable evidence.
pub(super) fn subagent_stage(name: &str, task: Option<&str>) -> &'static str {
    let task = task.unwrap_or_default().to_ascii_lowercase();
    let role: Vec<_> = name
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|part| !part.is_empty())
        .map(str::to_ascii_lowercase)
        .collect();
    let has_role = |words: &[&str]| words.iter().any(|word| role.iter().any(|part| part == word));
    if ["review", "audit", "verif"].iter().any(|word| task.contains(word)) || has_role(&["review", "reviewer", "audit", "auditor", "verify", "verifier"]) {
        "Reviewing"
    } else if ["plan", "architect", "design"].iter().any(|word| task.contains(word)) || has_role(&["plan", "planner", "architect", "design", "designer"]) {
        "Planning"
    } else if ["research", "explore", "search", "investigat"].iter().any(|word| task.contains(word)) || has_role(&["research", "researcher", "explore", "explorer", "search", "investigator"]) {
        "Researching"
    } else if ["build", "implement", "code"].iter().any(|word| task.contains(word)) || has_role(&["build", "builder", "implementer", "coder"]) {
        "Building"
    } else {
        "Working"
    }
}

fn subagent_label(name: &str, task: Option<&str>, latest_activity: Option<&str>) -> String {
    if let Some(task) = task.map(str::trim).filter(|task| !task.is_empty()) {
        return task.to_owned();
    }
    if let Some(activity) = latest_activity.map(str::trim).filter(|activity| {
        !activity.is_empty() && !activity.starts_with("Runtime reported")
    }) {
        if let Some(tool) = activity.strip_prefix("PreToolUse: ") {
            return format!("Using {tool}");
        }
        if let Some(tool) = activity.strip_prefix("PostToolUse: ") {
            return format!("Used {tool}");
        }
        if !activity.starts_with("SubagentStart") && !activity.starts_with("SubagentStop") {
            return activity.to_owned();
        }
    }
    let role = name.trim();
    let generic = ["", "agent", "subagent", "worker", "delegate", "general-purpose", "general purpose", "default", "unknown"];
    if !generic.iter().any(|name| role.eq_ignore_ascii_case(name)) {
        let stage = subagent_stage(role, None);
        if stage != "Working" {
            return stage.to_owned();
        }
        return role.to_owned();
    }
    "Working".to_owned()
}

/// This many of a session's native helper agents are named before they are counted.
const SUBS_LISTED: usize = 8;

/// A helper's row steps in this far under the session that started it, per level.
const FLEET_INDENT: u16 = 2;
/// Deeper helpers stay at this level: the names need the width more than the tree does.
const FLEET_DEPTH: u8 = 2;

/// Sessions of a project, each by (worktree, session) index.
type Rows = Vec<(usize, usize)>;

pub(in crate::client) fn side_lines(app: &App, model: &[Proj], t: &Theme) -> Vec<Line> {
    let mut out = Vec::new();
    for (pi, p) in model.iter().enumerate() {
        if p.kind == Kind::Ssh && (pi == 0 || model[pi - 1].kind != p.kind) {
            let n = model.iter().filter(|q| q.kind == p.kind).map(|q| q.sessions().count()).sum();
            out.push(Line::Section(p.kind, n));
        }
        out.push(Line::Proj(pi));
        if app.screen.saved.closed.contains(&format!("p:{}", p.key)) {
            out.push(Line::Gap);
            continue;
        }
        // Just the sessions: the ones that need you first, the rest where they were (the repo
        // folder's before the worktrees' when equal).
        let every: Rows = (0..p.wts.len()).flat_map(|wi| (0..p.wts[wi].sessions.len()).map(move |si| (wi, si))).collect();
        let mut rows = every;
        // Your order (dragged) otherwise.
        let order = &app.screen.saved.session_order;
        let at = |t: TermId| order.iter().position(|x| *x == t).unwrap_or(usize::MAX);
        let sort = app.cfg.ui.attention_sort;
        rows.sort_by_key(|&(wi, si)| {
            let s = &p.wts[wi].sessions[si];
            let mine = at(s.term);
            (sort && settled(s.status), if mine == usize::MAX { !p.wts[wi].main } else { false }, mine, s.term)
        });
        // A fleet: the sessions a session started sit under it, in the order above. One that
        // needs you has sorted to the top and stays there, on its own.
        let term_of = |r: &(usize, usize)| p.wts[r.0].sessions[r.1].term;
        let commander = |r: &(usize, usize)| {
            let s = &p.wts[r.0].sessions[r.1];
            let floats = sort && !settled(s.status);
            s.started_by.filter(|by| !floats && *by != s.term && rows.iter().any(|x| term_of(x) == *by))
        };
        let mut placed: Vec<((usize, usize), u8)> = Vec::with_capacity(rows.len());
        let mut next: Vec<((usize, usize), u8)> = rows.iter().rev().filter(|r| commander(r).is_none()).map(|r| (*r, 0)).collect();
        while let Some((r, depth)) = next.pop() {
            placed.push((r, depth));
            let below = (depth + 1).min(FLEET_DEPTH);
            next.extend(rows.iter().rev().filter(|k| commander(k) == Some(term_of(&r)) && !placed.iter().any(|(x, _)| x == *k)).map(|k| (*k, below)));
        }
        // Sessions that started each other can't come from the daemon; if they did, they're
        // still listed.
        let lost: Vec<(usize, usize)> = rows.iter().filter(|r| !placed.iter().any(|(x, _)| x == *r)).copied().collect();
        placed.extend(lost.into_iter().map(|r| (r, 0)));
        let mut any = false;
        for ((wi, si), depth) in placed {
            any = true;
            out.push(Line::Sess(pi, wi, si, depth));
            let s = &p.wts[wi].sessions[si];
            session_lines(s, app.screen.subs_closed.contains(&s.term), t, &mut out);
        }
        if !any {
            out.push(Line::Empty(pi));
        }
        out.push(Line::Gap);
    }
    out
}

/// The session a sidebar line stands for, if any.
pub(in crate::client) fn line_term(model: &[Proj], l: &Line) -> Option<TermId> {
    match l {
        Line::Sess(pi, wi, si, _) => Some(model[*pi].wts[*wi].sessions[*si].term),
        _ => None,
    }
}

/// The sidebar card: projects (● in their colour, bold), their sessions under them most urgent
/// first, a question under a session that needs you; new and actions at the foot.
pub(in crate::client) fn draw_side(app: &mut App, buf: &mut Buffer, r: Rect, model: &[Proj], t: &Theme) {
    let look = Look::of(&app.cfg.ui);
    let pb = if look.flat { t.card } else { pane_bg(t) };
    let focused_side = app.mode == Mode::Side;
    let host = crate::ipc::remote().map(|h| format!("⇄ {h}")).unwrap_or_default();
    let mut c = Card::new(t, &host);
    if look.flat { c.bg = t.card; }
    if focused_side {
        c = c.lit(t.accent);
    }
    card(app, buf, r, &c, t);
    app.screen.side_rect = r;
    hit(app, r, ScreenHit::SideFocus);
    let small = buf.area.width < NARROW;
    let mut lines = side_lines(app, model, t);
    if small {
        lines.retain(|l| !matches!(l, Line::Check(_)));
    }
    let shown: Vec<TermId> = app.screen.tabs.get(app.screen.tab).map(|t| t.layout.leaves()).unwrap_or_default();
    // In a split, the split's row (its first pane's) is the open one, whichever side you're on.
    let focus = app.focused().map(|f| if shown.len() > 1 && shown.contains(&f) { shown[0] } else { f });
    // Rows: from 1 below the top border, inset 3; a blank row above the bottom border.
    let (x0, w) = (r.x, r.width);
    let top = r.y + if look.flat { 1 } else { 2 };
    let list_h = if look.flat { r.bottom().saturating_sub(top).saturating_sub(2) as usize } else { r.height.saturating_sub(8) as usize };
    let right = r.right().saturating_sub(if look.flat { 1 } else { 3 });
    // Keep the focused (or cursor) row in view when it changes; otherwise the wheel rules.
    let mut scroll = app.screen.side_scroll as usize;
    if app.screen.follow {
        let want = app.screen.cursor.or(focus).and_then(|term| lines.iter().position(|l| line_term(model, l) == Some(term)));
        if let Some(i) = want {
            if i < scroll {
                scroll = i.saturating_sub(1);
            } else if i + 2 >= scroll + list_h {
                scroll = i + 3 - list_h.min(i + 3);
            }
        }
        app.screen.follow = false;
    }
    scroll = scroll.min(lines.len().saturating_sub(list_h));
    app.screen.side_scroll = scroll as u16;

    app.screen.visible = lines.iter().filter_map(|l| line_term(model, l)).collect();
    app.screen.side_items = lines
        .iter()
        .filter_map(|l| match l {
            Line::Proj(pi) => Some(SideItem::Proj(model[*pi].key.clone())),
            Line::Sess(..) => line_term(model, l).map(SideItem::Sess),
            _ => None,
        })
        .collect();
    app.screen.row_y.clear();
    app.screen.proj_keys = model.iter().map(|p| p.key.clone()).collect();

    // Hover and keyboard browsing must never erase the session that's actually open.
    let hover_bg = if look.flat { t.hov } else { t.hover_bg() };
    let on = app.motion_on();
    let mut gliding: Option<(u16, Rect)> = None;
    let shade = |app: &App, term: TermId, row: Rect, glide: bool| -> Option<Color> {
        if !look.flat {
            // One selected row: the one under the cursor while the sidebar has the keys, else
            // the open one.
            let picked = if focused_side { app.screen.cursor == Some(term) } else { Some(term) == focus };
            return if picked { Some(t.sel_bg()) } else { hovered(app, row).then_some(hover_bg) };
        }
        if Some(term) == focus { Some(t.hov) }
        else if focused_side && app.screen.cursor == Some(term) && !glide { Some(if look.flat { t.hov } else { t.card2 }) }
        else { hovered(app, row).then_some(hover_bg) }
    };
    // The lines under a session (what it asks, its subagents) step in as far as it does.
    let mut indents: Vec<u16> = Vec::with_capacity(lines.len());
    for l in &lines {
        let last = indents.last().copied().unwrap_or(0);
        indents.push(match l {
            Line::Sess(.., depth) => u16::from(*depth) * FLEET_INDENT,
            Line::Note(..) | Line::Check(..) | Line::Subagent(..) | Line::SubagentRecord(..) | Line::Subs(..) => last,
            _ => 0,
        });
    }
    for (i, line) in lines.iter().enumerate().skip(scroll).take(list_h) {
        let y = top + (i - scroll) as u16;
        let indent = indents[i];
        let row = Rect { x: x0 + if look.flat { 1 } else { 2 }, y, width: w.saturating_sub(if look.flat { 2 } else { 4 }), height: 1 };
        match line {
            Line::Proj(pi) => {
                let p = &model[*pi];
                let open = !app.screen.saved.closed.contains(&format!("p:{}", p.key));
                let on = focused_side && app.screen.cursor_proj.as_ref() == Some(&p.key);
                let hov = hovered(app, row) || on;
                let bg = if on { if look.flat { t.card2 } else { t.sel_bg() } } else if hov { hover_bg } else { pb };
                if bg != pb {
                    row_pill(look, buf, row.x, y, row.width, bg, pb);
                }
                let s = Style::default().bg(bg);
                // Its fold arrow, in the project's colour.
                let mut left = vec![seg(if open { "▾ " } else { "▸ " }, s.fg(p.color).add_modifier(Modifier::BOLD)), seg(p.name.clone(), s.fg(t.strong).add_modifier(Modifier::BOLD))];
                if p.fresh {
                    left.push(seg(" ", s));
                    left.extend(chip("NEW", Style::default().bg(t.accent).fg(t.acc_ink).add_modifier(Modifier::BOLD)));
                }
                // Right: ● n needing you, or "no git"; folded, what's inside.
                let needs = p.sessions().filter(|x| x.status == Status::Blocked).count();
                let mut c: Vec<Seg> = Vec::new();
                if needs > 0 {
                    c.push(seg(format!("● {needs}"), s.fg(t.blocked).add_modifier(Modifier::BOLD)));
                } else if !p.git && p.kind != Kind::Ssh {
                    c.push(seg("no git", s.fg(t.muted)));
                }
                if !open {
                    let n = p.sessions().count();
                    let what = if n == 0 { "empty".to_string() } else { format!("▸ {n}") };
                    c.push(seg(format!("{}{what}", if c.is_empty() { "" } else { "  " }), s.fg(t.muted)));
                }
                let cw = segs_width(&c);
                put(buf, x0 + 3, y, &left, right.saturating_sub(cw + 1));
                hit(app, row, ScreenHit::ToggleProj(*pi));
                if hov && !small {
                    row_menu_button(app, buf, Rect { x: right.saturating_sub(1), y, width: 1, height: 1 }, bg, t, ScreenHit::RowMenuProj(*pi));
                    row_glyph(app, buf, Rect { x: right.saturating_sub(3), y, width: 1, height: 1 }, "+", bg, t.accent, t, ScreenHit::ShellIn(*pi));
                } else {
                    put(buf, right.saturating_sub(cw), y, &c, right);
                }
            }
            Line::Sess(pi, wi, si, _) => {
                let s = &model[*pi].wts[*wi].sessions[*si];
                app.screen.row_y.insert(s.term, y);
                let active = Some(s.term) == focus;
                let keyboard = focused_side && app.screen.cursor == Some(s.term);
                let glide = if keyboard && !active { app.motion.glide("side", u64::from(s.term), y, on) } else { None };
                if let Some(gy) = glide {
                    gliding = Some((gy, row));
                }
                let chk = check::shown(s);
                // Its result line under it is one row with it: one tint, one hover.
                let span = Rect { height: if matches!(lines.get(i + 1), Some(Line::Check(_))) { 2 } else { 1 }, ..row };
                let lit = shade(app, s.term, span, glide.is_some());
                let bg = lit.unwrap_or(pb);
                if lit.is_some() {
                    if look.flat {
                        row_pill(look, buf, row.x, y, row.width, bg, pb);
                    } else {
                        fill(buf, Rect { x: r.x + 1, y, width: r.width.saturating_sub(2), height: 1 }, bg);
                    }
                }
                if !look.flat {
                    // The selection bar: lime while the sidebar has the keys, dim when a pane
                    // or a sheet does.
                    let selected = lit == Some(t.sel_bg());
                    if selected || active {
                        let col = if selected && focused_side { t.accent } else { t.muted };
                        if let Some(cell) = buf.cell_mut((r.x + 1, y)) {
                            cell.set_symbol("▎").set_style(Style::default().fg(col).bg(bg));
                        }
                    }
                }
                if look.flat && lit.is_some() {
                    let bar = if s.status == Status::Blocked { t.blocked } else if focused_side && app.screen.cursor == Some(s.term) { t.accent } else { t.muted };
                    if let Some(cell) = buf.cell_mut((row.x, y)) {
                        cell.set_symbol("▎").set_style(Style::default().fg(bar).bg(bg));
                    }
                }
                if chk.is_some_and(check::failed) {
                    check::edge(buf, x0 + 1, y, bg, t);
                }
                if active || keyboard {
                    put(buf, x0 + 3 + indent, y, &[seg(if active { "▸" } else { "›" }, Style::default().bg(bg).fg(t.strong))], x0 + 5 + indent);
                }
                let sel = focused_side && app.screen.cursor == Some(s.term);
                let st = Style::default().bg(bg);
                // A state's colour on this row: the theme's own, kept readable when the
                // row is lit.
                let state = |c: Color| if lit.is_some() { t.on_lit(c) } else { c };
                let (gl, gc) = if let Some(c) = chk {
                    (check::glyph(app, c).to_string(), state(check::color(t, c)))
                } else if s.is_agent || s.status != Status::None {
                    (glyph(app, s.status), state(state_color(t, s.status)))
                } else {
                    ("›".to_string(), t.muted)
                };
                let mut gs = st.fg(gc);
                if s.status == Status::Blocked || chk.is_some_and(check::failed) {
                    gs = gs.add_modifier(Modifier::BOLD);
                }
                let open_row = Some(s.term) == focus;
                // Finished and not looked at yet: green, like its dot.
                // Every colour is the theme's own.
                let unseen = s.is_agent && s.status == Status::Done;
                let needs = s.is_agent && s.status == Status::Blocked;
                let working = s.is_agent && s.status == Status::Working;
                let name_c = if needs {
                    state(t.blocked)
                } else if unseen {
                    state(t.done)
                } else if sel || open_row {
                    t.strong
                } else {
                    t.text
                };
                let mut ns = st.fg(name_c);
                if open_row || sel || unseen || needs {
                    ns = ns.add_modifier(Modifier::BOLD);
                }
                let mut left = vec![];
                if working {
                    let working = state(t.working);
                    left.extend(shimmer(&s.name, app.spinner_frame(), working, blend(working, t.strong, 0.6), ns));
                } else {
                    left.push(seg(s.name.clone(), ns));
                }
                // Right: agent · age (amber when it needs you); a limit, asleep.
                let tail: Vec<Seg> = if small {
                    vec![]
                } else if let Some(at) = s.resume_at {
                    vec![seg(format!("⏸ resumes in {}", until(at)), st.fg(t.muted))]
                } else {
                    let col = if s.status == Status::Blocked { t.blocked } else { t.muted };
                    let first = if s.agent.is_empty() || s.agent == s.name { String::new() } else { format!("{} · ", truncate(&s.agent, 16)) };
                    // A shell has no turn to time: just say what it is.
                    let when = if s.is_agent { age(s.since) } else { String::new() };
                    vec![seg(format!("{first}{when}").trim_end_matches(" · ").to_string(), st.fg(col))]
                };
                // The name comes first: when it doesn't fit, the tool shrinks to its icon.
                let name_x = x0 + 7 + indent;
                let room = right.saturating_sub(name_x) as usize;
                let tail = if !sel && s.resume_at.is_none() && !small && segs_width(&left) as usize + segs_width(&tail) as usize + 2 > room {
                    let col = if s.status == Status::Blocked { t.blocked } else { t.muted };
                    let when = if s.is_agent { age(s.since) } else { String::new() };
                    vec![seg(format!("{} {when}", tool_icon(&s.agent)).trim_end().to_string(), st.fg(col))]
                } else {
                    tail
                };
                let tw = segs_width(&tail);
                // Pointed at: its ⋯ takes the row's last column, the right side steps in.
                let menu = hovered(app, row) && !sel;
                let end = if menu { right.saturating_sub(2) } else { right };
                put(buf, x0 + 5 + indent, y, &[seg(gl, gs)], name_x);
                put(buf, name_x, y, &left, end.saturating_sub(tw + 1));
                put(buf, end.saturating_sub(tw), y, &tail, end);
                hit(app, row, ScreenHit::Session(s.term));
                hit(app, Rect { x: x0 + 5 + indent, y, width: 2, height: 1 }, ScreenHit::Why(s.term));
                if menu {
                    row_menu_button(app, buf, Rect { x: right.saturating_sub(1), y, width: 1, height: 1 }, bg, t, ScreenHit::RowMenuSess(s.term));
                }
            }
            Line::Check(term) => {
                let Some((_, _, s)) = find(model, *term) else { continue };
                let Some(info) = check::shown(s) else { continue };
                let lit = shade(app, *term, Rect { y: y.saturating_sub(1), height: 2, ..row }, false);
                let bg = lit.unwrap_or(pb);
                if lit.is_some() {
                    if look.flat {
                        row_pill(look, buf, row.x, y, row.width, bg, pb);
                    } else {
                        fill(buf, Rect { x: r.x + 1, y, width: r.width.saturating_sub(2), height: 1 }, bg);
                    }
                }
                let x = x0 + 7 + indent;
                let text = check::words(app, info, right.saturating_sub(x) as usize);
                put(buf, x, y, &[seg(text, check::words_style(info, t, bg, lit.is_some()))], right);
                if check::failed(info) {
                    check::edge(buf, x0 + 1, y, bg, t);
                }
                hit(app, row, ScreenHit::Session(*term));
            }
            Line::Note(text, c, term) => {
                if small {
                    continue;
                }
                put(buf, x0 + 7 + indent, y, &[seg(truncate(text, w.saturating_sub(11 + indent) as usize), Style::default().bg(pb).fg(*c).add_modifier(Modifier::ITALIC))], right);
                hit(app, row, ScreenHit::Session(*term));
            }
            Line::Subagent(name, term) => {
                let base = Style::default().bg(pb).fg(t.muted).add_modifier(Modifier::DIM);
                let available = w.saturating_sub(14 + indent) as usize;
                let label = truncate(name, available);
                let mut segs = vec![seg("  ↳ ", base)];
                segs.push(seg(label, base));
                put(buf, x0 + 7 + indent, y, &segs, right);
                hit(app, row, ScreenHit::Session(*term));
            }
            Line::SubagentRecord(activity, term, index) => {
                let state_color = match activity.state {
                    crate::protocol::SubagentState::Working => t.muted,
                    crate::protocol::SubagentState::WaitingForInput | crate::protocol::SubagentState::AwaitingApproval => t.blocked,
                    crate::protocol::SubagentState::Completed => t.done,
                    crate::protocol::SubagentState::Failed | crate::protocol::SubagentState::ConnectionLost => t.err,
                    crate::protocol::SubagentState::Cancelled => t.muted,
                    crate::protocol::SubagentState::Starting => t.accent,
                };
                let state_marker = match activity.state {
                    crate::protocol::SubagentState::Working => None,
                    crate::protocol::SubagentState::WaitingForInput => Some("?"),
                    crate::protocol::SubagentState::AwaitingApproval => Some("!"),
                    crate::protocol::SubagentState::Completed => Some("✓"),
                    crate::protocol::SubagentState::Failed => Some("×"),
                    crate::protocol::SubagentState::Cancelled => Some("–"),
                    crate::protocol::SubagentState::ConnectionLost => Some("?"),
                    crate::protocol::SubagentState::Starting => Some("◷"),
                };
                let detail = subagent_label(&activity.name, activity.task.as_deref(), activity.latest_activity.as_deref());
                let label = truncate(&detail, w.saturating_sub(15 + indent) as usize);
                let st = Style::default().bg(pb).add_modifier(Modifier::DIM);
                let mut label_style = st.fg(state_color);
                if activity.state != crate::protocol::SubagentState::Working {
                    label_style = label_style.add_modifier(Modifier::BOLD);
                }
                let mut segs = vec![seg("  ↳ ", st.fg(t.muted))];
                if let Some(marker) = state_marker {
                    segs.push(seg(format!("{marker} "), st.fg(state_color)));
                }
                segs.push(seg(label, label_style));
                put(buf, x0 + 7 + indent, y, &segs, right);
                hit(app, row, ScreenHit::SubagentInfo(*term, *index));
            }
            // Nothing running: one click starts a shell there.
            Line::Empty(pi) => {
                let bg = if hovered(app, row) { t.hov } else { pb };
                if bg != pb {
                    row_pill(look, buf, row.x, y, row.width, bg, pb);
                }
                let st = Style::default().bg(bg);
                let nk = k(app, &Action::ShellHere);
                put(
                    buf,
                    x0 + 5,
                    y,
                    &[seg("empty  ", st.fg(t.muted).add_modifier(Modifier::ITALIC)), seg(nk, st.fg(t.accent).add_modifier(Modifier::BOLD)), seg(" new pane", st.fg(t.muted))],
                    right,
                );
                hit(app, row, ScreenHit::ShellIn(*pi));
            }
            // A session's subagents: the heading folds and unfolds them.
            Line::Subs(term, n, open) => {
                if small {
                    continue;
                }
                let st = Style::default().bg(pb).fg(if hovered(app, row) { t.text } else { t.muted });
                let label = format!("{} {n} subagent{}", if *open { "▾" } else { "▸" }, if *n == 1 { "" } else { "s" });
                put(buf, x0 + 7 + indent, y, &[seg(label, st)], right);
                hit(app, row, ScreenHit::ToggleSubs(*term));
            }
            // A quiet heading: "AGENTS 2 ────".
            Line::Section(kind, n) => {
                let st = Style::default().bg(pb);
                let label = format!("{} {n} ", kind.heading().to_uppercase());
                let rest = right.saturating_sub(x0 + 3 + label.width() as u16);
                put(buf, x0 + 3, y, &[seg(label, st.fg(t.muted).add_modifier(Modifier::BOLD)), seg("─".repeat(rest as usize), st.fg(t.line))], right);
            }
            Line::Gap => {}
        }
    }
    let plain = Style::default().bg(pb);
    if scroll > 0 {
        put(buf, r.right() - 2, top, &[seg("▲", plain.fg(t.muted))], r.right() - 1);
    }
    if scroll + list_h < lines.len() {
        put(buf, r.right() - 2, top + list_h as u16 - 1, &[seg("▼", plain.fg(t.muted))], r.right() - 1);
    }
    // Keep the two global entry points in one stable place at the sidebar's foot.
    if r.height >= 5 {
        let foot_y = r.bottom().saturating_sub(if look.flat { 1 } else { 2 });
        let rule_y = foot_y.saturating_sub(1);
        let left = r.x + if look.flat { 1 } else { 2 };
        let right_edge = r.right().saturating_sub(if look.flat { 1 } else { 2 });
        let rule_bg = pb;
        for x in left..right_edge {
            if let Some(cell) = buf.cell_mut((x, rule_y)) {
                cell.set_symbol("─").set_style(Style::default().fg(t.line).bg(rule_bg));
            }
        }
        let new_label = "+ new";
        let new_style = Style::default().fg(t.accent).bg(pb).add_modifier(Modifier::BOLD);
        put(buf, left, foot_y, &[seg(new_label, new_style)], right_edge);
        let new_rect = Rect { x: left, y: foot_y, width: new_label.len() as u16, height: 1 };
        hit(app, new_rect, ScreenHit::NewShell);
        let actions_key = k(app, &Action::Actions);
        let actions_label = format!("{actions_key} actions");
        let actions_width = actions_label.len() as u16;
        let actions_x = right_edge.saturating_sub(actions_width);
        put(buf, actions_x, foot_y, &[seg(actions_label, new_style)], right_edge);
        hit(app, Rect { x: actions_x, y: foot_y, width: actions_width, height: 1 }, ScreenHit::Actions);
    }
    if let Some((gy, row)) = gliding.filter(|(gy, _)| (top..top + list_h as u16).contains(gy))
        && app.screen.row_y.get(&focus.unwrap_or_default()) != Some(&gy)
    {
        tint_row(look, buf, row.x, gy, row.width, if look.flat { t.card2 } else { t.sel_bg() }, pb);
    }
}

/// The ⋯ that opens a sidebar row's menu (for terminals that keep right-clicks).
pub(in crate::client) fn row_menu_button(app: &mut App, buf: &mut Buffer, r: Rect, bg: Color, t: &Theme, h: ScreenHit) {
    row_glyph(app, buf, r, "⋯", bg, t.muted, t, h);
}

/// A small glyph button on a sidebar row (⋯, +): just the glyph on the row's own ground,
/// brightening when it's the one under the mouse.
#[allow(clippy::too_many_arguments)]
fn row_glyph(app: &mut App, buf: &mut Buffer, r: Rect, glyph: &str, bg: Color, fg: Color, t: &Theme, h: ScreenHit) {
    let mut st = Style::default().bg(bg).fg(if hovered(app, r) { t.strong } else { fg });
    if hovered(app, r) {
        st = st.add_modifier(Modifier::BOLD);
    }
    put(buf, r.x, r.y, &[seg(glyph, st)], r.right());
    hit(app, r, h);
}

#[cfg(test)]
mod subagent_tests {
    use super::{subagent_label, subagent_stage};

    #[test]
    fn helpers_show_reported_tasks_and_use_roles_only_as_a_fallback() {
        assert_eq!(subagent_stage("planner", None), "Planning");
        assert_eq!(subagent_stage("general-purpose", Some("Review the test changes")), "Reviewing");
        assert_eq!(subagent_stage("general-purpose", Some("Implement the fix")), "Building");
        assert_eq!(subagent_stage("Explore", None), "Researching");
        assert_eq!(subagent_label("general-purpose", Some(" Review the test changes "), None), "Review the test changes");
        assert_eq!(subagent_label("general-purpose", None, Some("PreToolUse: Bash")), "Using Bash");
        assert_eq!(subagent_label("planner", None, None), "Planning");
        assert_eq!(subagent_label("Tools Redesign", None, None), "Tools Redesign");
        assert_eq!(subagent_label("general-purpose", None, Some("Runtime reported child start")), "Working");
    }
}
