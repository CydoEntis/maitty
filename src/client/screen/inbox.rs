//! The Inbox sheet: a search pill, then what needs you, failed checks, what two agents both
//! changed, and what finished; typing finds any session instead.

use super::*;

/// "NEEDS YOU 2 ──────": the name, its count, and a rule to the edge.
fn section(buf: &mut Buffer, x: u16, y: u16, w: u16, name: &str, n: Option<usize>, t: &Theme) {
    let c = Style::default().bg(t.card);
    let mut segs = vec![seg(name, c.fg(t.muted).add_modifier(Modifier::BOLD))];
    segs.push(seg(n.map(|n| format!(" {n} ")).unwrap_or_else(|| " ".into()), c.fg(t.text)));
    let end = put(buf, x, y, &segs, x + w);
    for xx in end..x + w {
        buf[(xx, y)].set_symbol("─").set_style(c.fg(t.line));
    }
}

/// Where the Inbox is (its query and row), while it's open.
pub(super) fn inbox_place(app: &App) -> Option<(String, usize)> {
    match &app.mode {
        Mode::GoTo { query, sel } => Some((query.clone(), *sel)),
        Mode::Compose(c) => c.inbox.clone(),
        _ => None,
    }
}

/// How many rows a row of the Inbox takes; the one with a follow-up open grows by its box.
fn row_height(row: &GoRow, compose: Option<&Compose>, box_rows: u16, asking: Option<TermId>, lit: bool) -> u16 {
    let writing = |term: &TermId| compose.is_some_and(|c| c.term == *term);
    match row {
        // A merge or throw-away being asked: its buttons go under the question.
        GoRow::Done(t) if asking == Some(*t) => 5,
        GoRow::Ask(t) if writing(t) => 3 + box_rows,
        GoRow::Done(t) if writing(t) => 3 + box_rows,
        GoRow::Ask(_) => 4,
        GoRow::Heads(_) => 4,
        GoRow::Done(_) => 4,
        GoRow::Failed(_) => check_inbox::FAILED_ROWS + u16::from(lit),
        GoRow::NotRun(_) => check_inbox::NOT_RUN_ROWS,

        _ => 1,
    }
}

/// The Inbox: a search pill, then what needs you (with its answers), what two agents both
/// changed, and what finished; typing finds any session instead.
pub(super) fn draw_inbox(app: &mut App, buf: &mut Buffer, inside: Rect, t: &Theme) {
    let Some((query, sel)) = inbox_place(app) else { return };
    let compose = match &app.mode {
        Mode::Compose(c) => Some((**c).clone()),
        _ => None,
    };
    let model = app.screen_model();
    let motion = app.motion_on();
    let mut gliding: Option<(u16, Rect)> = None;
    let heads = heads_up(app);
    let rows = goto_rows(&model, &query, heads.len());
    let look = Look::of(&app.cfg.ui);
    let c = Style::default().bg(t.card);
    let (x, w) = (inside.x + 2, inside.width.saturating_sub(4));
    let mut y = inside.y + 1;
    // The search pill.
    row_pill(look, buf, x - 1, y, w + 2, t.card2, t.card);
    let s2 = Style::default().bg(t.card2);
    let q: Vec<Seg> = if query.is_empty() {
        vec![seg("› ", s2.fg(t.accent).add_modifier(Modifier::BOLD)), seg("type to find any session", s2.fg(t.muted).add_modifier(Modifier::ITALIC))]
    } else {
        vec![seg("› ", s2.fg(t.accent).add_modifier(Modifier::BOLD)), seg(query.clone(), s2.fg(t.strong)), seg("█", s2.fg(t.accent))]
    };
    put(buf, x + 1, y, &q, x + w);
    y += 2;
    let bottom = inside.bottom().saturating_sub(2);
    // Scrolled so the selected row is whole on screen.
    let avail = bottom.saturating_sub(y);
    let box_rows = compose.as_ref().map(|c| compose_rows(c, w.saturating_sub(1), INBOX_ROWS)).unwrap_or(0);
    let asking = app.screen.inbox_confirm.filter(|(.., busy)| !busy).map(|(t, ..)| t);
    let height = |r: &GoRow, lit: bool| row_height(r, compose.as_ref(), box_rows, asking, lit);
    let mut start = 0;
    while start < sel && rows[start..=sel.min(rows.len().saturating_sub(1))].iter().enumerate().map(|(k, r)| height(r, start + k == sel)).sum::<u16>() > avail {
        start += 1;
    }
    let empty_note = |buf: &mut Buffer, y: u16, text: &str, tick: bool| {
        let mut segs = Vec::new();
        if tick {
            segs.push(seg("✓ ", c.fg(t.done)));
        }
        segs.push(seg(text.to_string(), c.fg(if tick { t.text } else { t.muted }).add_modifier(if tick { Modifier::empty() } else { Modifier::ITALIC })));
        put(buf, x, y, &segs, x + w);
    };
    for (i, row) in rows.iter().enumerate().skip(start) {
        let hgt = height(row, i == sel);
        if y + hgt > bottom + 1 {
            break;
        }
        let head = Rect { x: x - 1, y, width: w + 2, height: 1 };
        // The selected row's highlight glides there from the last one.
        let selected = i == sel && row.pickable();
        let glide = if selected { app.motion.glide("inbox", i as u64, y, motion) } else { None };
        if let Some(gy) = glide {
            gliding = Some((gy, head));
        }
        let on = selected && glide.is_none();
        let hov = hovered(app, head);
        let bg = if on { t.sel_on(t.card) } else if hov { t.hover_on(t.card) } else { t.card };
        if on || hov {
            row_pill(look, buf, head.x, y, head.width, bg, t.card);
        }
        let st = Style::default().bg(bg);
        // A row's head line: glyph, name, where; something right-aligned.
        let head_line = |buf: &mut Buffer, left: Vec<Seg>, right: Vec<Seg>| {
            let rw = segs_width(&right);
            put(buf, x + 1, y, &left, (x + w).saturating_sub(rw + 1));
            if rw > 0 {
                put(buf, (x + w).saturating_sub(rw), y, &right, x + w);
            }
        };
        match row {
            GoRow::Head(name) => {
                let n = rows[i + 1..].iter().take_while(|r| !matches!(r, GoRow::Head(_))).count();
                section(buf, x, y, w, name, if *name == "HEADS UP" { None } else { Some(n) }, t);
                y += 1;
                if n == 0 {
                    match *name {
                        "NEEDS YOU" => empty_note(buf, y, "Nothing is waiting on you.", true),
                        "JUST FINISHED" => empty_note(buf, y, "Nothing finished since you last looked.", false),
                        "FOUND" => empty_note(buf, y, "No session matches. Esc clears.", false),
                        _ => {}
                    }
                    y += 1;
                }
                continue;
            }
            GoRow::Ask(term) => {
                let Some((p, _, s)) = find(&model, *term) else { continue };
                head_line(
                    buf,
                    vec![
                        seg(format!("{} ", glyph(app, Status::Blocked)), st.fg(t.blocked).add_modifier(Modifier::BOLD)),
                        seg(s.name.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)),
                        seg(format!("  {} · {}", s.agent, p.name), st.fg(t.muted)),
                    ],
                    vec![seg(age(s.since), st.fg(t.blocked))],
                );
                let q = s.question.clone().unwrap_or_else(|| "waiting on you".into());
                put(buf, x + 3, y + 1, &[seg(truncate(&q, w.saturating_sub(4) as usize), c.fg(t.blocked).add_modifier(Modifier::ITALIC))], x + w);
                // Writing a follow-up here: the box in place of the answers.
                if let Some(cm) = compose.as_ref().filter(|cm| cm.term == *term) {
                    compose_box(app, buf, Rect { x: x + 1, y: y + 2, width: w.saturating_sub(1), height: 0 }, cm, INBOX_ROWS, t.card, t);
                    y += hgt;
                    continue;
                }
                // Its answers, as buttons: a number answers it; m writes something else.
                let mut ax = x + 3;
                for (n, label) in app.answer_options(*term).iter().enumerate().take(4) {
                    let key = char::from(b'1' + n as u8);
                    let segs = button_pill(look, t, label, &key.to_string(), n == 0, false, t.card);
                    let bw = segs_width(&segs);
                    if ax + bw >= x + w {
                        break;
                    }
                    put(buf, ax, y + 2, &segs, x + w);
                    hit(app, Rect { x: ax, y: y + 2, width: bw, height: 1 }, ScreenHit::InboxAnswer(*term, key));
                    ax += bw + 1;
                }
                put(buf, ax + 1, y + 2, &cap_hints(t, t.card, &[("m", "follow-up")]), x + w);
            }
            GoRow::Heads(k) => {
                let Some((_, o)) = heads.get(*k) else { continue };
                let conf = blend(t.blocked, t.text, 0.45);
                let who = match o.checkouts.as_slice() {
                    [a, b] => vec![seg(a.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)), seg(" and ", st.fg(t.text)), seg(b.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD))],
                    many => vec![seg(format!("{} checkouts", many.len()), st.fg(t.strong).add_modifier(Modifier::BOLD))],
                };
                let mut left = vec![seg("⇆ ", st.fg(conf).add_modifier(Modifier::BOLD))];
                left.extend(who);
                left.push(seg(" both changed ", st.fg(t.text)));
                left.push(seg(o.file.rsplit('/').next().unwrap_or(&o.file).to_string(), st.fg(t.sky())));
                head_line(buf, left, vec![]);
                put(buf, x + 3, y + 1, &[seg(truncate("Different branches, same file: it may conflict when they merge.", w.saturating_sub(4) as usize), c.fg(t.muted).add_modifier(Modifier::ITALIC))], x + w);
                let tell = format!("tell {}", o.checkouts.first().cloned().unwrap_or_default());
                put(buf, x + 3, y + 2, &cap_hints(t, t.card, &[("d", "both diffs"), ("m", &tell), ("k", "dismiss")]), x + w);
            }
            GoRow::Done(term) => {
                let Some((p, wt, s)) = find(&model, *term) else { continue };
                // What it changed, worked out once per finish (off the UI thread).
                let size = match app.screen.change_sizes.get(&wt.path) {
                    Some((since, size)) if *since == s.since => size.clone(),
                    _ => {
                        app.screen.change_sizes.insert(wt.path.clone(), (s.since, String::new()));
                        let (dir, since, base) = (wt.path.clone(), s.since, (!wt.main).then(|| crate::gitfs::main_branch(&p.path)).flatten());
                        app.spawn_bg(move || crate::client::Bg::ChangeSize(dir.clone(), since, crate::client::overlap::change_size(&dir, base.as_deref())));
                        String::new()
                    }
                };
                head_line(
                    buf,
                    vec![
                        seg(format!("{} ", glyph(app, Status::Done)), st.fg(t.done).add_modifier(Modifier::BOLD)),
                        seg(s.name.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)),
                        seg(format!("  {} · {}", s.agent, p.name), st.fg(t.muted)),
                    ],
                    vec![if size.is_empty() { seg(age(s.since), st.fg(t.muted)) } else { seg(size, st.fg(t.text)) }],
                );
                let said = app.snap.terms.get(term).map(|i| i.said.lines().rev().find(|l| !l.trim().is_empty()).unwrap_or_default().trim().to_string()).unwrap_or_default();
                let said = if said.is_empty() { "finished".to_string() } else { format!("“{said}”") };
                put(buf, x + 3, y + 1, &[seg(truncate(&said, w.saturating_sub(4) as usize), c.fg(t.text).add_modifier(Modifier::ITALIC))], x + w);
                if let Some(cm) = compose.as_ref().filter(|cm| cm.term == *term) {
                    compose_box(app, buf, Rect { x: x + 1, y: y + 2, width: w.saturating_sub(1), height: 0 }, cm, INBOX_ROWS, t.card, t);
                } else {
                    // Merge and throw-away are asked here, then run with a spinner.
                    match app.screen.inbox_confirm.filter(|(ct, ..)| ct == term) {
                        Some((_, merge, true)) => {
                            let what = if merge { format!("Merging {} into main…", s.name) } else { "Removing the worktree…".to_string() };
                            let spin = app.cfg.ui.spinner.get(app.spinner_frame() as usize % app.cfg.ui.spinner.len().max(1)).cloned().unwrap_or_default();
                            put(buf, x + 3, y + 2, &[seg(format!("{spin} "), c.fg(t.accent)), seg(what, c.fg(t.text))], x + w);
                        }
                        Some((_, merge, false)) => {
                            let ask = if merge { "Merge into main, then remove the worktree and branch?  " } else { "Throw it away and remove the worktree?  " };
                            put(buf, x + 3, y + 2, &[seg(ask.trim_end(), c.fg(t.strong))], x + w);
                            let mut bx = x + 3;
                            for (label, key, primary) in [(if merge { "Merge" } else { "Throw away" }, "Enter", true), ("Cancel", "Esc", false)] {
                                let segs = button_pill(look, t, label, key, primary, false, t.card);
                                bx = put(buf, bx, y + 3, &segs, x + w) + 1;
                            }
                        }
                        None => {
                            let keys: &[(&str, &str)] = if wt.main { &[("d", "diff"), ("m", "follow-up")] } else { &[("d", "diff"), ("M", "merge"), ("x", "throw away"), ("m", "follow-up")] };
                            put(buf, x + 3, y + 2, &cap_hints(t, t.card, keys), x + w);
                        }
                    }
                }
            }
            GoRow::Failed(term) | GoRow::NotRun(term) => {
                let Some((p, _, s)) = find(&model, *term) else { continue };
                check_inbox::draw(app, buf, (x, y, w), bg, (p, s), i == sel, t);
            }
            GoRow::Sess(pi, term) => {
                let Some((_, _, s)) = find(&model, *term) else { continue };
                let (gl, gc) = if s.is_agent { (glyph(app, s.status), t.status(s.status)) } else { (app.cfg.icons.shell.clone(), t.muted) };
                let what = if s.is_agent { s.agent.clone() } else { "shell".into() };
                head_line(
                    buf,
                    vec![seg(format!("{gl} "), st.fg(gc)), seg(s.name.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)), seg(format!("  {what} · {}", model[*pi].name), st.fg(t.muted))],
                    if on { vec![seg("Enter go", st.fg(t.muted))] } else { vec![] },
                );
            }
        }
        if row.pickable() {
            // A lit failed row keeps its button line out of the row's own click.
            let pick_h = hgt.saturating_sub(if i == sel && matches!(row, GoRow::Failed(_)) { 2 } else { 1 });
            hit(app, Rect { x: x - 1, y, width: w + 2, height: pick_h.max(1) }, ScreenHit::GoPick(i));
        }
        y += hgt;
    }
    if let Some((gy, head)) = gliding.filter(|(gy, _)| *gy < bottom) {
        tint_row(look, buf, head.x, gy, head.width, t.sel_on(t.card), t.card);
    }
    let keys: &[(&str, &str)] = if query.is_empty() { &[("↑↓", "move"), ("1 2 3", "answer"), ("Enter", "go"), ("Del", "seen")] } else { &[("↑↓", "move"), ("Enter", "go to it"), ("Esc", "clear")] };
    let note = vec![seg("j", Style::default().fg(t.accent).add_modifier(Modifier::BOLD)), seg(" closes", Style::default().fg(t.muted))];
    status_bar(buf, inside, t, keys, if query.is_empty() { &note } else { &[] });
}
