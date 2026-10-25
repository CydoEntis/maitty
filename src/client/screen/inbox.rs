//! The Inbox sheet: what needs you, failed checks, checks that could not run, what finished,
//! then what two agents both changed. Typing finds any session instead.

use super::check_inbox::{self, Item, FAILED_HEAD, FINISHED_HEAD, NEEDS_HEAD, NOT_RUN_HEAD};
use super::*;
use crate::theme::mix;

const HEADS_HEAD: &str = "HEADS UP";
/// How much of its colour a group's count chip keeps against the ground.
const COUNT_FADE: f32 = 0.25;
/// A button is drawn only with this much room after its label.
const BUTTON_ROOM: u16 = 6;
/// The age at the right of an item keeps this much from its name.
const AGE_GAP: u16 = 4;

/// The groups the Inbox title counts: everything but what finished and the heads-ups.
const COUNTED: [&str; 3] = [NEEDS_HEAD, FAILED_HEAD, NOT_RUN_HEAD];

/// The Inbox's rows with nothing typed, in its fixed order; an empty group is left out.
pub(super) fn inbox_rows(model: &[Proj], heads: usize) -> Vec<GoRow> {
    let waiting = jump_list(model);
    let asks = waiting.iter().filter(|(s, ..)| s.status == Status::Blocked).map(|(s, ..)| GoRow::Ask(s.term));
    let mut quiet: Vec<&Session> = model.iter().flat_map(Proj::sessions).filter(|s| s.status == Status::Working && s.quiet_since.is_some() && !s.quiet_seen).collect();
    quiet.sort_by_key(|s| (s.quiet_since, s.term));
    let needs: Vec<GoRow> = asks
        .chain(check_inbox::rows(model, |r| matches!(r, GoRow::Approve(_))))
        .chain(quiet.into_iter().map(|s| GoRow::Quiet(s.term)))
        .collect();
    // Finished: a session with a check result first, then the newest (the reference's order).
    let mut done: Vec<&Session> = waiting.iter().map(|(s, ..)| s).filter(|s| check::finished(s)).collect();
    done.sort_by_key(|s| (s.checks.is_empty(), std::cmp::Reverse(s.since)));
    let finished = done.into_iter().map(|s| GoRow::Done(s.term)).collect();
    let mut out = Vec::new();
    for (head, rows) in [
        (NEEDS_HEAD, needs),
        (FAILED_HEAD, check_inbox::rows(model, |r| matches!(r, GoRow::Failed(_)))),
        (NOT_RUN_HEAD, check_inbox::rows(model, |r| matches!(r, GoRow::NotRun(_)))),
        (FINISHED_HEAD, finished),
        // Last: there can be dozens (every file two checkouts both changed), and they must
        // never stand between you and an agent that needs you or has finished.
        (HEADS_HEAD, (0..heads).map(GoRow::Heads).collect()),
    ] {
        if !rows.is_empty() {
            out.push(GoRow::Head(head));
            out.extend(rows);
        }
    }
    out
}

/// The Inbox title's number: items that are not just finished.
pub(super) fn inbox_count(app: &App) -> usize {
    let rows = inbox_rows(&app.screen_model(), heads_up(app).len());
    let mut head = "";
    rows.iter()
        .filter(|r| match r {
            GoRow::Head(h) => {
                head = h;
                false
            }
            _ => COUNTED.contains(&head),
        })
        .count()
}

/// Where the Inbox is (its query and row), while it's open.
pub(super) fn inbox_place(app: &App) -> Option<(String, usize)> {
    match &app.mode {
        Mode::GoTo { query, sel } => Some((query.clone(), *sel)),
        Mode::Compose(c) => c.inbox.clone(),
        Mode::Confirm(_) | Mode::Approve(_) | Mode::Helpers { .. } => app.screen.inbox_return.clone(),
        _ => None,
    }
}

/// The Inbox's selected row, while it has the keys.
pub(super) fn inbox_selected(app: &App) -> Option<GoRow> {
    let Mode::GoTo { query, sel } = &app.mode else { return None };
    goto_rows(&app.screen_model(), query, heads_up(app).len()).get(*sel).cloned()
}

/// " NEEDS YOU " on its colour, then its count on a fainter chip.
fn chip_head(buf: &mut Buffer, (x, y, right): (u16, u16, u16), name: &str, color: Color, n: Option<usize>, t: &Theme) {
    let ink = if color == t.btn { t.strong } else { t.ink_on(color) };
    let b = Style::default().fg(ink).add_modifier(Modifier::BOLD);
    let mut segs = vec![seg(format!(" {name} "), b.bg(color))];
    if let Some(n) = n {
        segs.push(seg(format!(" {n} "), b.bg(mix(color, t.bg, COUNT_FADE))));
    }
    put(buf, x, y, &segs, right);
}

fn head_color(name: &str, t: &Theme) -> Color {
    match name {
        NEEDS_HEAD => t.blocked,
        FAILED_HEAD => t.err,
        NOT_RUN_HEAD => t.violet(),
        FINISHED_HEAD => t.done,
        HEADS_HEAD => blend(t.sky(), t.text, 0.3),
        _ => t.btn,
    }
}

/// What a row adds under an item's lines while it's being asked about or written to.
#[derive(Clone, Copy, PartialEq)]
enum Extra {
    None,
    /// A follow-up box of this many rows.
    Compose(u16),
    /// Merge or throw away, asked: the question and its buttons.
    Asking,
    /// Merging or throwing away: a spinner.
    Busy,
}

/// How many rows a row of the Inbox takes, its gap included.
fn row_height(row: &GoRow, item: Option<&Item>, selected: bool, extra: Extra) -> u16 {
    match (row, item) {
        (GoRow::Head(_), _) => 1,
        (GoRow::Heads(_), _) => 4,
        (_, Some(it)) => {
            let buttons = u16::from(selected && !it.buttons.is_empty() && extra == Extra::None);
            let more = match extra {
                Extra::None => 0,
                Extra::Compose(rows) => rows,
                Extra::Asking => 2,
                Extra::Busy => 1,
            };
            2 + u16::from(it.third.is_some()) + buttons + more + 1
        }
        _ => 1,
    }
}

/// An item's lines at `y`: its head line and the lines under it, on `bg`.
fn draw_item(buf: &mut Buffer, (x, y, right): (u16, u16, u16), it: &Item, bg: Color, t: &Theme) {
    let st = Style::default().bg(bg);
    let age_w = it.age.width() as u16;
    let head = [seg(format!("{} ", it.glyph), st.fg(it.color).add_modifier(Modifier::BOLD)), seg(it.name.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)), seg(format!(" · {}", it.folder), st.fg(t.muted))];
    put(buf, x, y, &head, right.saturating_sub(AGE_GAP));
    put(buf, right.saturating_sub(age_w), y, &[seg(it.age.clone(), st.fg(t.muted))], right);
    let words = |w: &[check_inbox::Words]| -> Vec<Seg> { w.iter().map(|(s, c, b)| seg(s.clone(), if *b { st.fg(*c).add_modifier(Modifier::BOLD) } else { st.fg(*c) })).collect() };
    let mut line = words(&it.line);
    if let Some(strip) = &it.strip
        && segs_width(&line) > right.saturating_sub(x + 2)
    {
        line = words(strip);
    }
    put(buf, x + 2, y + 1, &line, right);
    if let Some(third) = &it.third {
        put(buf, x + 2, y + 2, &[seg(third.clone(), st.fg(t.muted))], right);
    }
}

/// The selected item's buttons from `x`; each is a click on the key it shows.
fn draw_buttons(app: &mut App, buf: &mut Buffer, (x, y, right): (u16, u16, u16), buttons: &[(&str, &str, char)], t: &Theme) {
    let mut at = x;
    for &(label, key, press) in buttons {
        if at + label.width() as u16 + BUTTON_ROOM >= right {
            break;
        }
        let kind = if press == check_inbox::OVERRIDE.2 { BtnKind::Danger } else { BtnKind::Normal };
        let Some(r) = sheet_button(buf, at, y, right, (label, key), kind, t) else { break };
        hit(app, r, ScreenHit::InboxKey(press));
        at = r.right() + 1;
    }
}

/// Merge or throw away being asked about, or under way, in a finished item.
fn draw_asking(app: &mut App, buf: &mut Buffer, (x, y, right): (u16, u16, u16), name: &str, (merge, busy): (bool, bool), ground: Color, t: &Theme) {
    let c = Style::default().bg(ground);
    if busy {
        let what = if merge { format!("Merging {name} into main…") } else { "Removing the worktree…".to_string() };
        let spin = app.cfg.ui.spinner.get(app.spinner_frame() as usize % app.cfg.ui.spinner.len().max(1)).cloned().unwrap_or_default();
        put(buf, x, y, &[seg(format!("{spin} "), c.fg(t.accent)), seg(what, c.fg(t.text))], right);
        return;
    }
    let ask = if merge { "Merge into main, then remove the worktree and branch?" } else { "Throw it away and remove the worktree?" };
    put(buf, x, y, &[seg(ask, c.fg(t.strong))], right);
    let yes = if merge { "Merge" } else { "Throw away" };
    draw_buttons(app, buf, (x, y + 1, right), &[(yes, "Enter", '\n'), ("Cancel", "Esc", '\x1b')], t);
}

/// A heads-up: two checkouts changed the same file.
fn draw_heads(buf: &mut Buffer, (x, y, right): (u16, u16, u16), o: &crate::client::overlap::Overlap, bg: Color, ground: Color, t: &Theme) {
    let st = Style::default().bg(bg);
    let conf = head_color(HEADS_HEAD, t);
    let mut left = vec![seg("⇆ ", st.fg(conf).add_modifier(Modifier::BOLD))];
    match o.checkouts.as_slice() {
        [a, b] => left.extend([seg(a.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)), seg(" and ", st.fg(t.text)), seg(b.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD))]),
        many => left.push(seg(format!("{} checkouts", many.len()), st.fg(t.strong).add_modifier(Modifier::BOLD))),
    }
    left.push(seg(" both changed ", st.fg(t.text)));
    left.push(seg(o.file.rsplit('/').next().unwrap_or(&o.file).to_string(), st.fg(t.sky())));
    put(buf, x, y, &left, right);
    let c = Style::default().bg(ground);
    put(buf, x + 2, y + 1, &[seg("Different branches, same file: it may conflict when they merge.", c.fg(t.muted))], right);
    let tell = format!("tell {}", o.checkouts.first().cloned().unwrap_or_default());
    put(buf, x + 2, y + 2, &cap_hints(t, ground, &[("d", "both diffs"), ("m", &tell), ("k", "dismiss")]), right);
}

/// A session found by typing: glyph, name, what runs and where.
fn draw_found(app: &App, buf: &mut Buffer, (x, y, right): (u16, u16, u16), (proj, s): (&Proj, &Session), bg: Color, t: &Theme) {
    let st = Style::default().bg(bg);
    let (gl, gc) = if s.is_agent { (glyph(app, s.status), t.status(s.status)) } else { (app.cfg.icons.shell.clone(), t.muted) };
    let what = if s.is_agent { s.agent.clone() } else { "shell".into() };
    put(buf, x, y, &[seg(format!("{gl} "), st.fg(gc)), seg(s.name.clone(), st.fg(t.strong).add_modifier(Modifier::BOLD)), seg(format!(" · {what} · {}", proj.name), st.fg(t.muted))], right);
}

/// The selected row's tint across the sheet, and its bar; a failure keeps a red bar unselected.
fn mark_row(buf: &mut Buffer, inside: Rect, (y, h): (u16, u16), (selected, failed): (bool, bool), (bar, ground): (&str, Color), t: &Theme) {
    let bg = if selected { t.sel_on(ground) } else { ground };
    if selected {
        fill(buf, Rect { x: inside.x, y, width: inside.width, height: h }, bg);
    }
    if selected || failed {
        for yy in y..y + h {
            put(buf, inside.x, yy, &[seg(bar, Style::default().bg(bg).fg(if failed { t.err } else { t.accent }))], inside.x + 1);
        }
    }
}

/// The search row, once something is typed.
fn draw_query(buf: &mut Buffer, (x, y, right): (u16, u16, u16), query: &str, t: &Theme) {
    let s2 = Style::default().bg(t.card2);
    fill(buf, Rect { x, y, width: right.saturating_sub(x), height: 1 }, t.card2);
    put(buf, x + 1, y, &[seg("› ", s2.fg(t.accent).add_modifier(Modifier::BOLD)), seg(query.to_string(), s2.fg(t.strong)), seg("█", s2.fg(t.accent))], right);
}

/// The Inbox inside `inside`, on `ground`: its groups, the selected item with its buttons, and
/// the keys on the last row.
pub(super) fn draw_inbox(app: &mut App, buf: &mut Buffer, inside: Rect, ground: Color, t: &Theme) {
    let Some((query, sel)) = inbox_place(app) else { return };
    let compose = match &app.mode {
        Mode::Compose(c) => Some((**c).clone()),
        _ => None,
    };
    let model = app.screen_model();
    let heads = heads_up(app);
    let rows = goto_rows(&model, &query, heads.len());
    // A selection from an older list may point past this one.
    let sel = sel.min(rows.len().saturating_sub(1));
    let items: Vec<Option<Item>> = rows.iter().map(|r| check_inbox::item(app, &model, r, t)).collect();
    let bar = super::glyphs::chosen(&app.cfg.ui.glyphs, &app.cfg.icons).bar;
    let (x, right) = (inside.x + 1, inside.right().saturating_sub(1));
    let mut y = inside.y + 1;
    if !query.is_empty() {
        draw_query(buf, (x, y, right), &query, t);
        y += 2;
    }
    let bottom = inside.bottom().saturating_sub(1);
    let box_rows = compose.as_ref().map(|c| compose_rows(c, right.saturating_sub(x + 2), INBOX_ROWS)).unwrap_or(0);
    let confirm = app.screen.inbox_confirm;
    let extra = |r: &GoRow| match (r.term(), confirm) {
        (Some(term), _) if compose.as_ref().is_some_and(|c| c.term == term) => Extra::Compose(box_rows),
        (Some(term), Some((ct, _, busy))) if ct == term => if busy { Extra::Busy } else { Extra::Asking },
        _ => Extra::None,
    };
    let height = |i: usize| row_height(&rows[i], items[i].as_ref(), i == sel, extra(&rows[i]));
    // Scrolled so the selected row is whole on screen.
    let avail = bottom.saturating_sub(y);
    let mut start = 0;
    while start < sel && (start..=sel.min(rows.len().saturating_sub(1))).map(height).sum::<u16>() > avail + 1 {
        start += 1;
    }
    if rows.is_empty() {
        put(buf, x, y, &[seg("✓ ", Style::default().bg(ground).fg(t.done)), seg("Nothing is waiting on you.", Style::default().bg(ground).fg(t.text))], right);
    }
    for (i, row) in rows.iter().enumerate().skip(start) {
        let hgt = height(i);
        // The gap under the last row may fall on the hints.
        if y + hgt.saturating_sub(1) > bottom {
            break;
        }
        let selected = i == sel && row.pickable();
        let mark_h = hgt.saturating_sub(1).max(1);
        if let GoRow::Head(name) = row {
            let n = rows[i + 1..].iter().take_while(|r| !matches!(r, GoRow::Head(_))).count();
            let count = (*name != HEADS_HEAD).then_some(n);
            chip_head(buf, (x, y, right), name, head_color(name, t), count, t);
            y += 1;
            continue;
        }
        mark_row(buf, inside, (y, mark_h), (selected, items[i].as_ref().is_some_and(|it| it.failed)), (bar, ground), t);
        // Before its buttons, so they are clicks of their own.
        hit(app, Rect { x: inside.x, y, width: inside.width, height: mark_h }, ScreenHit::GoPick(i));
        let bg = if selected { t.sel_on(ground) } else { ground };
        match (row, &items[i]) {
            (GoRow::Heads(k), _) => {
                if let Some((_, o)) = heads.get(*k) {
                    draw_heads(buf, (x, y, right), o, bg, ground, t);
                }
            }
            (GoRow::Sess(pi, term), _) => {
                if let Some((_, _, s)) = find(&model, *term) {
                    draw_found(app, buf, (x, y, right), (&model[*pi], s), bg, t);
                }
            }
            (_, Some(it)) => {
                draw_item(buf, (x, y, right), it, bg, t);
                let under = y + 2 + u16::from(it.third.is_some());
                match extra(row) {
                    Extra::Compose(_) => {
                        if let Some(cm) = &compose {
                            compose_box(app, buf, Rect { x: x + 2, y: under, width: right.saturating_sub(x + 2), height: 0 }, cm, INBOX_ROWS, ground, t);
                        }
                    }
                    Extra::Asking | Extra::Busy => {
                        if let Some((_, merge, busy)) = confirm {
                            draw_asking(app, buf, (x + 2, under, right), &it.name, (merge, busy), ground, t);
                        }
                    }
                    Extra::None if selected => draw_buttons(app, buf, (x + 2, under, right), &it.buttons, t),
                    Extra::None => {}
                }
            }
            _ => {}
        }
        y += hgt;
    }
    let keys: &[(&str, &str)] = if query.is_empty() {
        &[("↑↓", "select"), ("⏎", "open"), ("r", "re-run"), ("O", "override"), ("Esc", "close")]
    } else {
        &[("↑↓", "select"), ("⏎", "go to it"), ("Esc", "close")]
    };
    action_bar(buf, Rect { y: bottom, height: 1, ..inside }, t, t.card, keys);
}

#[cfg(test)]
mod tests {
    use super::super::reference::{Ref, assert_same, layout_name, reference, text};
    use super::*;
    use crate::client::reference_fixture::{LAYOUTS, SIZES, Stage, now, world};
    use crate::client::View;
    use crate::protocol::{CheckState, ClientMsg, Command};

    /// The reference's sample question and first failing line, in place of the fixture's.
    const QUESTION: &str = "Use pnpm or npm for the workspace?";
    const FAILING: &str = "✗ legal.test.tsx › footer shows current year";

    /// The world at `stage` with the reference's sample words, the Inbox open on `term`'s item.
    fn inbox_world(stage: Stage, term: TermId) -> App {
        let mut app = world(stage);
        app.cfg.ui.glyphs = "unicode".into();
        app.snap.questions[0].text = QUESTION.into();
        for c in &mut app.snap.terms.get_mut(&1).unwrap().checks {
            if c.state == CheckState::Failed {
                c.first_failing_line = FAILING.into();
            }
        }
        app.screen_sync();
        select(&mut app, term);
        app
    }

    fn select(app: &mut App, term: TermId) {
        let rows = goto_rows(&app.screen_model(), "", 0);
        let sel = rows.iter().position(|r| r.term() == Some(term)).expect("the item is listed");
        app.mode = Mode::GoTo { query: String::new(), sel };
    }

    fn draw(app: &mut App, area: Rect, w: u16, h: u16) -> Vec<String> {
        let t = Theme::named(crate::theme::DEFAULT);
        let mut buf = Buffer::empty(Rect { x: 0, y: 0, width: w, height: h });
        draw_inbox(app, &mut buf, area, t.bg, &t);
        text(&buf, area)
    }

    fn matches_reference(preset: &str, stage: Stage, term: TermId, tweak: fn(&mut App), fix: fn(&mut Ref)) {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut want = reference(preset, layout_name(layout), w, h);
                want.offers_send_to_agent(w);
                fix(&mut want);
                let mut app = inbox_world(stage, term);
                tweak(&mut app);
                let got = draw(&mut app, want.area, w, h);
                assert_same(&got, &want, &format!("{preset} {layout:?} {w}x{h}"));
            }
        }
    }

    /// The fixture's working session, gone quiet twelve minutes ago.
    fn quiet_world() -> (App, TermId) {
        let mut app = world(Stage::Failed);
        let term = app.snap.terms.iter().filter(|(_, t)| t.status == Status::Working).map(|(id, _)| *id).min().expect("a working session");
        app.snap.terms.get_mut(&term).unwrap().quiet_since = Some(now() - 12 * 60);
        app.screen_fresh();
        (app, term)
    }

    #[test]
    fn a_quiet_agent_is_listed_under_needs_you_after_questions() {
        let (app, term) = quiet_world();
        let rows = inbox_rows(&app.screen_model(), 0);
        let at = |r: &GoRow| rows.iter().position(|x| x == r);
        let quiet = at(&GoRow::Quiet(term)).expect("listed");
        let ask = rows.iter().position(|r| matches!(r, GoRow::Ask(_))).expect("a question");
        let failed = at(&GoRow::Head(FAILED_HEAD)).expect("failed checks");
        assert!(at(&GoRow::Head(NEEDS_HEAD)) < Some(ask) && ask < quiet && quiet < failed, "{rows:?}");
        let before = { let (mut a, _) = quiet_world(); a.snap.terms.get_mut(&term).unwrap().quiet_since = None; a.screen_fresh(); inbox_count(&a) };
        assert_eq!(inbox_count(&app), before + 1, "counted in the title");
    }

    #[test]
    fn a_quiet_item_says_how_long_and_opens_its_session() {
        let (mut app, term) = quiet_world();
        select(&mut app, term);
        let page = draw(&mut app, Rect { x: 0, y: 0, width: 100, height: 30 }, 100, 30).join("\n");
        assert!(page.contains("no output for 12m · may be stuck"), "{page}");
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Normal), "Enter opens the session");
    }

    #[test]
    fn delete_hides_a_quiet_agent_until_it_goes_quiet_again() {
        let (mut app, term) = quiet_world();
        select(&mut app, term);
        app.on_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert!(!inbox_rows(&app.screen_model(), 0).contains(&GoRow::Quiet(term)), "seen");
        app.snap.terms.get_mut(&term).unwrap().quiet_since = Some(now() - 60);
        app.screen_fresh();
        assert!(inbox_rows(&app.screen_model(), 0).contains(&GoRow::Quiet(term)), "a new quiet spell is listed again");
    }

    #[test]
    fn the_inbox_lists_needs_you_then_failed_checks_then_finished_as_the_reference_draws_it() {
        matches_reference("11 Inbox", Stage::Failed, 1, |_| {}, |_| {});
    }

    /// The reference's sample asked a minute ago; the fixture's turn ended two minutes ago.
    #[test]
    fn a_repo_command_waits_as_needs_you_with_one_way_to_read_it() {
        let a_minute_ago = |app: &mut App| {
            app.snap.terms.get_mut(&1).unwrap().since = now() - 60;
            app.screen_fresh();
        };
        matches_reference("A1 Repo command waits", Stage::Approve, 1, a_minute_ago, |_| {});
    }

    /// The fixture's checkpoints are build and test; the reference's sample kinds (commits,
    /// approval, review) are not built, and nor is its footnote.
    #[test]
    fn several_checkpoints_read_as_one_word_then_each_checkpoint() {
        matches_reference("C2 Several checkpoints · Inbox", Stage::Multi, 3, |_| {}, |want| {
            for (i, row) in want.rows.clone().iter().enumerate() {
                if row.contains("needs you: approval") {
                    want.set_row(i, "   needs you: test · build ✓ · test ●");
                } else if row.contains("failed: test ") {
                    want.set_row(i, "▎  failed: test · build ✓ · test ✕");
                } else if row.contains("Several checkpoints read as one line") {
                    want.set_row(i, "");
                }
            }
        });
    }

    #[test]
    fn the_title_counts_what_is_not_just_finished() {
        assert_eq!(inbox_count(&inbox_world(Stage::Failed, 1)), 2);
        assert_eq!(inbox_count(&inbox_world(Stage::Multi, 3)), 3);
        assert_eq!(inbox_count(&inbox_world(Stage::Passed, 3)), 1);
    }

    fn heads(app: &App) -> Vec<&'static str> {
        goto_rows(&app.screen_model(), "", 0).iter().filter_map(|r| if let GoRow::Head(h) = r { Some(*h) } else { None }).collect()
    }

    #[test]
    fn every_group_comes_in_its_fixed_order_and_a_question_is_never_below_a_check() {
        let mut app = world(Stage::Failed);
        let mut extra = |id: TermId, state: CheckState, approve: bool| {
            let mut t = app.snap.terms[&1].clone();
            (t.id, t.label, t.since) = (id, format!("s{id}"), now() - u64::from(id));
            t.checks[0].state = state;
            if approve {
                t.checks[0].exit_code = None;
            }
            app.snap.terms.insert(id, t);
            app.snap.workspaces[0].tabs.push(crate::protocol::TabInfo { id: 100 + id, name: String::new(), layout: crate::layout::Node::Leaf(id), focus: id });
            app.parsers.insert(id, vt100::Parser::new(20, 80, 0));
        };
        extra(40, CheckState::CouldNotRun, false);
        extra(41, CheckState::NeedsApproval, true);
        extra(42, CheckState::Stale, false);
        extra(43, CheckState::Never, false);
        app.screen_sync();
        assert_eq!(heads(&app), [NEEDS_HEAD, FAILED_HEAD, NOT_RUN_HEAD, FINISHED_HEAD]);
        let rows = goto_rows(&app.screen_model(), "", 0);
        let at = |want: &GoRow| rows.iter().position(|r| r == want).unwrap_or_else(|| panic!("{want:?} in {rows:?}"));
        assert!(at(&GoRow::Ask(3)) < at(&GoRow::Approve(41)), "the question first");
        assert!(at(&GoRow::Approve(41)) < at(&GoRow::Failed(1)) && at(&GoRow::Failed(1)) < at(&GoRow::NotRun(40)));
        assert!(at(&GoRow::NotRun(40)) < at(&GoRow::Done(42)) && rows.contains(&GoRow::Done(43)) && rows.contains(&GoRow::Done(4)), "stale, never and no check finished");
    }

    #[test]
    fn an_empty_group_is_not_drawn() {
        let app = inbox_world(Stage::Passed, 3);
        assert_eq!(heads(&app), [NEEDS_HEAD, FINISHED_HEAD]);
    }

    /// A stale pass stays in FINISHED, and its first button is Re-run.
    #[test]
    fn a_stale_pass_is_finished_with_re_run_first() {
        let mut app = inbox_world(Stage::Stale, 1);
        let area = reference("11 Inbox", "floating", 160, 45).area;
        let rows = draw(&mut app, area, 160, 45);
        let at = rows.iter().position(|r| r.contains("◷ fix-footer · affiliate-site")).unwrap_or_else(|| panic!("{rows:#?}"));
        assert!(rows[..at].iter().any(|r| r.contains("FINISHED  2")), "{rows:#?}");
        assert_eq!(rows[at + 1], "▎  stale pass: test · 3 files changed since");
        assert_eq!(rows[at + 2], "▎   Re-run r   Output ⏎");
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(sent.iter().any(|m| matches!(m, ClientMsg::Command(Command::RerunCheck { term: 1, .. }))), "Enter re-runs: {sent:?}");
        assert!(matches!(app.mode, Mode::GoTo { .. }), "and the Inbox stays");
    }

    fn enter(stage: Stage, term: TermId) -> (App, Vec<ClientMsg>) {
        let mut app = inbox_world(stage, term);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let sent = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        (app, sent)
    }

    fn focused(sent: &[ClientMsg], term: TermId) -> bool {
        sent.iter().any(|m| matches!(m, ClientMsg::Command(Command::FocusPane { term: t }) if *t == term))
    }

    #[test]
    fn enter_runs_each_item_s_first_button() {
        let (app, _) = enter(Stage::Failed, 3);
        assert!(matches!(&app.mode, Mode::Compose(c) if c.term == 3 && c.inbox.is_some()), "a question: Reply");
        let (app, _) = enter(Stage::Failed, 1);
        assert!(matches!(&app.view, Some(View::Check(v)) if v.term == 1) && app.mode == Mode::Normal, "a failure: Output");
        let (app, _) = enter(Stage::Couldnt, 1);
        assert!(matches!(&app.view, Some(View::Check(v)) if v.term == 1), "could not run: Output");
        let (app, _) = enter(Stage::Approve, 1);
        assert!(matches!(&app.mode, Mode::Approve(a) if a.term == 1), "a command waiting: read it");
        let (app, sent) = enter(Stage::Passed, 1);
        assert!(app.mode == Mode::Normal && app.view.is_none() && focused(&sent, 1), "no buttons: the session");
        let (app, sent) = enter(Stage::Failed, 4);
        assert!(app.mode == Mode::Normal && focused(&sent, 4), "finished, no check: the session");
    }

    #[test]
    fn o_opens_the_session_of_any_item() {
        for (stage, term) in [(Stage::Failed, 3), (Stage::Failed, 1), (Stage::Couldnt, 1), (Stage::Approve, 1), (Stage::Stale, 1), (Stage::Failed, 4)] {
            let mut app = inbox_world(stage, term);
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            app.out = tx;
            app.on_key(KeyEvent::new(KeyCode::Char('o'), KeyModifiers::NONE));
            let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
            assert!(app.mode == Mode::Normal && focused(&sent, term), "{stage:?} {term}");
        }
    }

    #[test]
    fn a_failing_line_is_marked_in_the_chosen_glyph_set() {
        for (glyphs, mark) in [("unicode", "✗"), ("ascii", "x")] {
            let mut app = inbox_world(Stage::Failed, 1);
            app.cfg.ui.glyphs = glyphs.into();
            app.snap.terms.get_mut(&1).unwrap().checks[0].first_failing_line = "FAIL footer shows current year".into();
            app.screen_fresh();
            let area = reference("11 Inbox", "floating", 160, 45).area;
            let rows = draw(&mut app, area, 160, 45);
            assert!(rows.iter().any(|r| r.contains(&format!("{mark} FAIL footer shows current year"))), "{glyphs}: {rows:#?}");
        }
    }

    #[test]
    fn a_button_is_a_click_of_its_key() {
        let mut app = inbox_world(Stage::Couldnt, 1);
        let area = reference("11 Inbox", "floating", 100, 30).area;
        draw(&mut app, area, 100, 30);
        assert!(app.hits.iter().any(|(_, h)| *h == Hit::Screen(ScreenHit::InboxKey(','))), "Settings ,");
        app.on_hy_hit(ScreenHit::InboxKey(','), false);
        assert!(matches!(&app.mode, Mode::ProjectCheck(_)), "opens PROJECT · <name>");
    }

    #[test]
    fn typing_finds_a_session_and_shows_what_was_typed() {
        let mut app = inbox_world(Stage::Failed, 1);
        app.mode = Mode::GoTo { query: String::new(), sel: 0 };
        for c in "quick".chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        let area = reference("11 Inbox", "floating", 100, 30).area;
        let rows = draw(&mut app, area, 100, 30);
        assert!(rows[1].contains("› quick█"), "{rows:#?}");
        assert!(rows.iter().any(|r| r.contains("quick-fox")), "{rows:#?}");
        let mut app = inbox_world(Stage::Failed, 1);
        assert!(!draw(&mut app, area, 100, 30).iter().any(|r| r.contains("type to find")), "no search row until you type");
    }
}
