//! Popovers beside a sidebar row (the screen isn't dimmed): a quick follow-up to an agent
//! (`m`), and why it has the status it has (`i`). A `◂` points at the row.

use super::*;
use crate::protocol::{CheckInfo, CheckState, ClientMsg};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::time::{Duration, Instant};

/// A follow-up being written: to `term`, from the sidebar (`side`: back to its cursor after)
/// or from an Inbox row (`inbox`: the Inbox's query and row, to go back to).
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct Compose {
    pub term: TermId,
    pub text: String,
    pub side: bool,
    pub inbox: Option<(String, usize)>,
}

/// The Inbox row's text area grows to this many rows.
pub(in crate::client) const INBOX_ROWS: u16 = 6;
/// The popovers' widths, when there's room.
const WHY_W: u16 = 64;
const WHY_H: u16 = 12;
/// The why popover of a session with a check: its three lines and the keys.
const CHECK_WHY_W: u16 = 60;
const CHECK_WHY_H: u16 = 8;
/// Width of the time column in the check timeline (`14:21:51 `).
const TIME_COL: usize = 9;

/// Word-wrapped to `width`, keeping blank lines.
pub(super) fn wrap(text: &str, width: usize) -> Vec<String> {
    let width = width.max(1);
    let mut out = Vec::new();
    for para in text.split('\n') {
        let mut cur = String::new();
        for word in para.split(' ') {
            if !cur.is_empty() && cur.width() + 1 + word.width() > width {
                out.push(std::mem::take(&mut cur));
            } else if !cur.is_empty() {
                cur.push(' ');
            }
            cur.push_str(word);
            while cur.width() > width {
                let head: String = cur.chars().take(width).collect();
                cur = cur.chars().skip(width).collect();
                out.push(head);
            }
        }
        out.push(cur);
    }
    out
}

/// The rows a text area `w` wide takes (its box and the keys under it).
pub(in crate::client) fn compose_rows(c: &Compose, w: u16, max: u16) -> u16 {
    (wrap(&c.text, w.saturating_sub(6) as usize).len() as u16).clamp(3, max) + 3
}

/// The text area at `at` (its top left and width), on `outer`: a rounded box of 3 to `max` rows that grows
/// with the text and scrolls past that (`↑ n more` in its top border), a word count top
/// right, and the keys under it. Returns the rows it took.
pub(in crate::client) fn compose_box(app: &mut App, buf: &mut Buffer, at: Rect, c: &Compose, max: u16, outer: Color, t: &Theme) -> u16 {
    let (x, y, w) = (at.x, at.y, at.width);
    let lines = wrap(&c.text, w.saturating_sub(6) as usize);
    let rows = (lines.len() as u16).clamp(3, max);
    let hidden = lines.len().saturating_sub(rows as usize);
    let more = if hidden > 0 { format!("↑ {hidden} more") } else { String::new() };
    let mut card_ = Card::new(t, &more);
    card_.border = t.accent;
    card_.bold = true;
    card_.title_fg = t.muted;
    card_.bg = t.card2;
    let r = Rect { x, y, width: w, height: rows + 2 };
    card(app, buf, r, &card_, t);
    let words = c.text.split_whitespace().count();
    if words > 0 {
        let wc = format!(" {words} word{} ", if words == 1 { "" } else { "s" });
        put(buf, r.right().saturating_sub(wc.width() as u16 + 2), y, &[seg(wc, Style::default().fg(t.muted).bg(outer))], r.right());
    }
    let s2 = Style::default().bg(t.card2);
    if c.text.is_empty() {
        let name = app.snap.terms.get(&c.term).map(|i| i.display_name()).unwrap_or_default();
        put(buf, x + 3, y + 1, &[seg(format!("Next prompt for {name}…"), s2.fg(t.muted).add_modifier(Modifier::ITALIC)), seg(" █", s2.fg(t.accent))], r.right() - 2);
    }
    let shown = &lines[hidden..];
    for (i, l) in shown.iter().enumerate().filter(|_| !c.text.is_empty()) {
        let mut segs = vec![seg(l.clone(), s2.fg(t.strong))];
        if i + 1 == shown.len() {
            segs.push(seg("█", s2.fg(t.accent)));
        }
        put(buf, x + 3, y + 1 + i as u16, &segs, r.right() - 2);
    }
    // To an agent that's working, Enter queues it for when the turn is over.
    let working = app.snap.terms.get(&c.term).is_some_and(|i| i.status == Status::Working);
    let keys = if working {
        cap_hints(t, outer, &[("Enter", "send when it's done"), ("Ctrl+Enter", "now"), ("Shift+Enter", "new line"), ("Esc", "cancel")])
    } else {
        cap_hints(t, outer, &[("Enter", "send"), ("Shift+Enter", "new line"), ("Esc", "cancel")])
    };
    put(buf, x + 1, y + rows + 2, &keys, x + w);
    rows + 3
}

/// The follow-up popover's widest width: you are replying, not writing a document.
const POPOVER_W: u16 = 68;
/// Below this width the hints and the working line use their short words.
const POPOVER_NARROW_W: u16 = 60;
/// The most of a message a receipt on a row repeats.
const RECEIPT_TEXT_MAX: usize = 44;
/// A receipt stays at least this long, so an agent that starts working at once doesn't wipe
/// it before it is read, and at most this long.
const RECEIPT_MIN_SHOWN: Duration = Duration::from_secs(2);
const RECEIPT_MAX_SHOWN: Duration = Duration::from_secs(6);

/// What a row says about the last message sent to its session.
#[derive(Debug, Clone)]
pub(in crate::client) struct Receipt {
    text: String,
    /// Waiting for the agent's turn to end (`◷ queued`), else delivered (`› sent`).
    queued: bool,
    /// The server has listed it among the session's queued follow-ups, so an empty list
    /// afterwards means it was delivered.
    seen_queued: bool,
    /// When it was made, or when it was delivered.
    since: Instant,
    /// The session's status then: a change clears a delivered receipt.
    status: Status,
}

/// Quote lines and input rows the popover allows at a screen height: 160x45, 100x30, 80x24.
fn size_tier(screen_h: u16) -> (usize, usize) {
    if screen_h >= 45 {
        (4, 6)
    } else if screen_h >= 30 {
        (3, 4)
    } else {
        (2, 3)
    }
}

/// The popover's top row for a session row at `anchor`: its top border a line above the row,
/// else (opening upward) its bottom border just under the row, else pinned to the bottom of
/// the screen. Under the row, not on its last line, so the `◂` stays on the row.
fn place(screen: Rect, anchor: Rect, h: u16) -> u16 {
    let mut top = anchor.y.saturating_sub(1);
    if top + h > screen.bottom() {
        top = (anchor.bottom() + 1).saturating_sub(h);
        if top + h > screen.bottom() {
            top = screen.bottom().saturating_sub(h);
        }
    }
    top.max(screen.y)
}

/// A popover's frame right of the sidebar, at `(x, width, height)` as far as the screen allows,
/// beside `term`'s row: the screen dimmed except that row, a full accent border on `card`, and
/// a `◂` on the row. Returns the frame and the row's rect.
pub(in crate::client) fn frame_beside_row(app: &mut App, buf: &mut Buffer, screen: Rect, side: Rect, term: TermId, (x0, w, h): (u16, u16, u16), t: &Theme) -> (Rect, Rect) {
    let ascii = app.cfg.ui.glyphs == "ascii";
    let hit_row = app.hits.iter().find_map(|(r, hit)| match hit {
        crate::client::Hit::Screen(ScreenHit::Session(t)) if *t == term && r.right() <= x0 => Some(*r),
        _ => None,
    });
    let anchor = hit_row.unwrap_or(Rect { x: side.x, y: screen.y + 1, width: side.width, height: 1 });
    let top = place(screen, anchor, h);
    let r = Rect { x: x0, y: top, width: w, height: h };

    let kept = hit_row.map(|row| (row, (row.y..row.bottom()).flat_map(|y| (row.x..row.right()).map(move |x| (x, y))).map(|p| (p, buf[p].clone())).collect::<Vec<_>>()));
    dim_all(app, buf, screen, t);
    if let Some((_, cells)) = kept {
        for (p, cell) in cells {
            buf[p] = cell;
        }
    }

    let look = Look::of(&app.cfg.ui);
    fill(buf, r, t.card);
    let edge = Style::default().fg(t.accent).bg(t.card);
    let pointer = Style::default().fg(t.accent).bg(t.card);
    let [tl, tr, bl, br] = corners(look);
    for xx in r.x + 1..r.right() - 1 {
        buf[(xx, r.y)].set_symbol("─").set_style(edge);
        buf[(xx, r.bottom() - 1)].set_symbol("─").set_style(edge);
    }
    for yy in r.y + 1..r.bottom() - 1 {
        buf[(r.x, yy)].set_symbol("│").set_style(edge);
        buf[(r.right() - 1, yy)].set_symbol("│").set_style(edge);
    }
    for (xx, yy, s) in [(r.x, r.y, tl), (r.right() - 1, r.y, tr), (r.x, r.bottom() - 1, bl), (r.right() - 1, r.bottom() - 1, br)] {
        buf[(xx, yy)].set_symbol(s).set_style(edge);
    }
    hit(app, r, ScreenHit::Noop);
    if anchor.y > r.y && anchor.y < r.bottom() - 1 {
        put(buf, r.x, anchor.y, &[seg(if ascii { "<" } else { "◂" }, pointer.add_modifier(Modifier::BOLD))], r.x + 1);
    }

    (r, anchor)
}

/// The follow-up popover, beside its sidebar row with the rest of the screen dimmed.
pub(in crate::client) fn draw_compose_pop(app: &mut App, buf: &mut Buffer, screen: Rect, side: Rect, c: &Compose, t: &Theme) {
    let Some(info) = app.snap.terms.get(&c.term).cloned() else { return };
    let ascii = app.cfg.ui.glyphs == "ascii";
    let (quote_max, input_max) = size_tier(screen.height);
    let x0 = side.right() + 1;
    let w = POPOVER_W.min(screen.right().saturating_sub(x0 + 1));
    if w < 20 {
        return;
    }
    let (iw, x) = (w as usize - 4, x0 + 2);
    let narrow = w < POPOVER_NARROW_W;

    let agent = info.agent.clone().unwrap_or_else(|| "agent".into());
    let blocked = info.status == Status::Blocked;
    let working = info.status == Status::Working;
    let said = if info.said.trim().is_empty() { quote_fallback(app, c.term) } else { info.said.clone() };
    let quote: Vec<String> = wrap(said.trim(), iw - 2).into_iter().filter(|l| !l.trim().is_empty()).collect();
    let shown = &quote[quote.len().saturating_sub(quote_max)..];
    let cut = quote.len() > shown.len();
    let input = wrap(&c.text, iw - 2);
    let more = if input.len() > input_max { input.len() - (input_max - 1) } else { 0 };
    let visible = if more > 0 { &input[input.len() - (input_max - 1)..] } else { &input[..] };
    let input_rows = if more > 0 { input_max } else { input.len() } as u16;
    let quote_rows = if shown.is_empty() { 0 } else { shown.len() as u16 + 1 };
    let h = 2 + quote_rows + 1 + input_rows + u16::from(working) + 1;

    let (r, _) = frame_beside_row(app, buf, screen, side, c.term, (x0, w, h), t);

    let name = app.session_name(c.term);
    let title = vec![
        seg(format!(" MESSAGE {name} "), Style::default().fg(t.acc_ink).bg(t.accent).add_modifier(Modifier::BOLD)),
        seg(format!("· {agent} "), Style::default().fg(t.muted).bg(t.card)),
    ];
    put(buf, r.x + 2, r.y, &title.clone(), r.right() - 2);
    let bold = Modifier::BOLD;
    let state = if blocked {
        vec![seg(format!(" {} needs you ", if ascii { "!" } else { "●" }), Style::default().fg(t.blocked).bg(t.card).add_modifier(bold))]
    } else if working {
        vec![seg(format!(" {}working ", glyph(app, Status::Working)), Style::default().fg(t.accent).bg(t.card).add_modifier(bold))]
    } else {
        vec![seg(format!(" {} done ", if ascii { "o" } else { "○" }), Style::default().fg(t.muted).bg(t.card))]
    };
    let state_x = (r.right() - 2).saturating_sub(segs_width(&state));
    if state_x > r.x + 2 + segs_width(&title) {
        put(buf, state_x, r.y, &state, r.right() - 2);
    }

    let ground = Style::default().bg(t.card);
    let mut y = r.y + 1;
    if !shown.is_empty() {
        let verb = if blocked { "asked" } else { "said" };
        let tail = if cut { format!(" · last {} of {} lines", shown.len(), quote.len()) } else { String::new() };
        put(buf, x, y, &[seg(format!("{agent} {verb}"), ground.fg(t.muted).add_modifier(bold)), seg(tail, ground.fg(t.muted))], x + iw as u16);
        y += 1;
        for l in shown {
            put(buf, x, y, &[seg(if ascii { "| " } else { "▏ " }, ground.fg(t.muted)), seg(l.clone(), ground.fg(t.text))], x + iw as u16);
            y += 1;
        }
    }
    for xx in r.x + 1..r.right() - 1 {
        buf[(xx, y)].set_symbol("─").set_style(ground.fg(t.line));
    }
    y += 1;
    let well = Style::default().bg(t.bg);
    fill(buf, Rect { x: x - 1, y, width: iw as u16 + 2, height: input_rows }, t.bg);
    if more > 0 {
        let up = if ascii { "^" } else { "↑" };
        put(buf, x, y, &[seg(format!("{up} {more} more line{}", if more == 1 { "" } else { "s" }), well.fg(t.muted))], x + iw as u16);
        y += 1;
    }
    for (i, l) in visible.iter().enumerate() {
        let first = more == 0 && i == 0;
        let mut segs = vec![seg(if first { if ascii { "> " } else { "› " } } else { "  " }, well.fg(t.accent).add_modifier(bold))];
        if c.text.is_empty() {
            segs.push(seg("█", well.fg(t.accent)));
            segs.push(seg(format!(" Reply to {agent}…"), well.fg(t.muted)));
        } else {
            segs.push(seg(l.clone(), well.fg(t.strong)));
            if i + 1 == visible.len() {
                segs.push(seg("█", well.fg(t.accent)));
            }
        }
        put(buf, x, y, &segs, x + iw as u16);
        y += 1;
    }
    if working {
        let rest = if narrow { "Enter queues it." } else { "Enter queues this until the turn ends." };
        put(buf, x, y, &[seg(format!("{agent} is mid-turn. "), ground.fg(t.text)), seg(rest, ground.fg(t.muted))], x + iw as u16);
        y += 1;
    }
    let keys = if working {
        cap_hints_gap(t, t.card, if narrow { "  " } else { "   " }, &[("Enter", "queue"), ("Ctrl+Enter", if narrow { "now" } else { "send now" }), ("Esc", "cancel")])
    } else {
        cap_hints_gap(t, t.card, if narrow { "  " } else { "   " }, &[("Enter", "send"), ("Shift+Enter", if narrow { "line" } else { "new line" }), ("Esc", "cancel")])
    };
    let keys_end = put(buf, x, y, &keys, x + iw as u16);
    let words = c.text.split_whitespace().count();
    if words > 0 {
        let wc = format!("{words} word{}", if words == 1 { "" } else { "s" });
        let wx = (x + iw as u16).saturating_sub(wc.width() as u16);
        if wx > keys_end + 1 {
            put(buf, wx, y, &[seg(wc, ground.fg(t.muted))], x + iw as u16);
        }
    }
}

/// What a blocked agent asks, when it has said nothing else the popover can quote.
fn quote_fallback(app: &App, term: TermId) -> String {
    let model = app.screen_model();
    find(&model, term).and_then(|(_, _, s)| s.question.clone()).unwrap_or_default()
}

/// Drop receipts that have run their time, and turn queued ones into sent once the server has
/// handed the follow-up to the agent. Called once a frame.
pub(in crate::client) fn refresh_receipts(app: &mut App) {
    let now = Instant::now();
    let snap = &app.snap.terms;
    app.screen.receipts.retain(|term, r| {
        let Some(info) = snap.get(term) else { return false };
        if r.queued {
            r.seen_queued |= !info.queued.is_empty();
            if info.queued.is_empty() && (r.seen_queued || info.status != Status::Working) {
                (r.queued, r.since, r.status) = (false, now, info.status);
            }
            return true;
        }
        let age = now.saturating_duration_since(r.since);
        age < RECEIPT_MAX_SHOWN && !(info.status != r.status && age >= RECEIPT_MIN_SHOWN)
    });
}

/// A row's second line while it carries a receipt: `◷ queued "…"` dim, or `› sent "…"` accent.
pub(in crate::client) fn receipt_segs(app: &App, term: TermId, t: &Theme) -> Option<Vec<Seg>> {
    let r = app.screen.receipts.get(&term)?;
    let ascii = app.cfg.ui.glyphs == "ascii";
    let (mark, word, color) = if r.queued { (if ascii { "~" } else { "◷" }, "queued", t.muted) } else { (if ascii { ">" } else { "›" }, "sent", t.accent) };
    Some(vec![seg(format!("{mark} {word} "), Style::default().fg(color).add_modifier(Modifier::BOLD)), seg(format!("\"{}\"", r.text), Style::default().fg(t.text))])
}

/// Where a status came from: "hook", "screen" or "process" (the rest: you).
fn source_of(why: &str) -> &'static str {
    match why.split(':').next().unwrap_or("") {
        "hook" => "hook",
        "screen" => "screen",
        "process" => "process",
        _ => "",
    }
}

/// Why a session has its status: the hook, screen and process evidence, each with its age,
/// the one that set it bold and marked.
pub(in crate::client) fn draw_why_pop(app: &mut App, buf: &mut Buffer, screen: Rect, side: Rect, term: TermId, t: &Theme) {
    let Some(info) = app.snap.terms.get(&term).cloned() else { return };
    if let Some(check) = info.check() {
        let r = popover_at(app, buf, popover_rect(screen, side, CHECK_WHY_W, CHECK_WHY_H), &check_why_title(app, &info), t);
        hit(app, r, ScreenHit::CheckEvidence(term));
        draw_check_why(app, buf, r, &info, check, t);
        return;
    }
    let title = format!("why {} {}", glyph(app, info.status), state_label(info.status));
    let r = popover_at(app, buf, popover_rect(screen, side, WHY_W, WHY_H), &title, t);
    let ago = |at: u64| if at == 0 { String::new() } else { format!("{} ago", age(at)) };
    let process = match (info.agent.as_deref(), info.pid) {
        (Some(a), 0) => format!("{a} running"),
        (Some(a), pid) => format!("{a} running · pid {pid}"),
        (None, _) => format!("{} running", if info.process.is_empty() { "shell" } else { &info.process }),
    };
    let won = source_of(&info.status_why);
    let rows = [
        ("hook", if info.why_hook.is_empty() { "none since it started".to_string() } else { info.why_hook.clone() }, ago(info.why_hook_at)),
        ("screen", if info.why_screen.is_empty() { "nothing matched yet".to_string() } else { info.why_screen.clone() }, ago(info.why_screen_at)),
        ("process", process, ago(info.why_process_at)),
    ];
    let c = Style::default().bg(t.card);
    let confirmed = info.why_hook_at.max(info.why_screen_at);
    let summary = if let (Status::Working, Some(at)) = (info.status, info.quiet_since) {
        format!("No output or report for {} — it may be stuck.", age(at))
    } else if info.agent.is_some() && confirmed == 0 && info.status_why.is_empty() {
        "Status uncertain — no report or screen confirmation.".into()
    } else {
        match info.status {
            Status::Blocked => "Waiting for your answer or permission.",
            Status::Done => "Turn finished; ready for you to review.",
            Status::Idle => "Ready for your next instruction.",
            Status::Working if won == "hook" => "The agent reported that it is working.",
            Status::Working if info.status_why.contains("one of its own") => "Waiting for its helpers to finish.",
            Status::Working if info.why_screen_at == 0 => "Status uncertain — no turn confirmation yet.",
            Status::Working => "Working — estimated from the screen.",
            Status::None => "No agent status is available for this terminal.",
        }
        .into()
    };
    put(buf, r.x + 3, r.y + 2, &[seg(summary, c.fg(t.text))], r.right() - 2);
    let confirmation = if confirmed == 0 { "Last confirmation: none".into() } else { format!("Last confirmation: {}", ago(confirmed)) };
    put(buf, r.x + 3, r.y + 3, &[seg(confirmation, c.fg(t.muted))], r.right() - 2);
    for (i, (src, detail, when)) in rows.iter().enumerate() {
        let y = r.y + 5 + i as u16;
        let win = *src == won;
        let name = if win { c.fg(t.strong).add_modifier(Modifier::BOLD) } else { c.fg(t.muted) };
        let right = vec![seg(when.clone(), c.fg(t.muted)), seg(if win { "  ◂" } else { "   " }, c.fg(t.accent).add_modifier(Modifier::BOLD))];
        let rw = segs_width(&right);
        put(buf, r.x + 3, y, &[seg(format!("{src:<9}"), name), seg(detail.clone(), c.fg(if win { t.text } else { t.muted }))], r.right().saturating_sub(rw + 4));
        put(buf, r.right().saturating_sub(rw + 3), y, &right, r.right() - 1);
    }
    let note = if won.is_empty() && !info.status_why.is_empty() {
        format!("Reason: {}.", info.status_why)
    } else {
        "The most specific source wins: hook, then screen, then process.".to_string()
    };
    put(buf, r.x + 3, r.y + 9, &[seg(truncate(&note, r.width.saturating_sub(6) as usize), c.fg(t.muted).add_modifier(Modifier::ITALIC))], r.right() - 2);
}

/// " WHY · fix-footer reads failed ": the session and the word it reads as.
pub(in crate::client) fn check_why_title(app: &App, info: &TermInfo) -> String {
    let word = super::glyphs::Kind::of(info.status, info.check(), info.asleep).word();
    format!("WHY · {} reads {word}", app.session_name(info.id))
}

/// What the check's run came to, as the last line: its words and colour. A result that isn't
/// a plain pass or failure says what its row says.
fn check_result(app: &App, check: &CheckInfo, t: &Theme) -> (String, Color) {
    let g = super::check::glyph(app, check);
    let exit = check.exit_code.map(|c| format!("exit {c}")).unwrap_or_default();
    let took = super::check::duration(check.duration_ms);
    let name = if check.name.is_empty() { &check.command } else { &check.name };
    let words = match check.state {
        CheckState::Passed | CheckState::Failed => format!("{exit} after {took}"),
        CheckState::Overridden => format!("failed, overridden: {name}"),
        CheckState::Stale => format!("stale pass: {name} · {} files changed", check.changed_files.len()),
        CheckState::Never => "not checked: never for this repo".into(),
        CheckState::NeedsApproval => format!("approve check: {}", check.command),
        CheckState::CouldNotRun => format!("{name} could not run{}", super::check::why_not_run(check.exit_code)),
        CheckState::Checking => format!("checking {name}"),
        CheckState::Pending => format!("next: {name}"),
    };
    let color = super::glyphs::Kind::of(Status::Done, Some(check), false).color(t);
    (format!("{g} {words}"), color)
}

/// The popover's keys, only those that act on this result.
fn check_keys(state: CheckState) -> Vec<(&'static str, &'static str)> {
    let mut keys = vec![("⏎", "evidence")];
    if state != CheckState::Checking {
        keys.push(("r", "re-run"));
    }
    if state == CheckState::Failed {
        keys.push(("O", "override"));
    }
    keys.push(("Esc", "close"));
    keys
}

/// Why a session with a check reads as it does: the turn that ended, the app running the
/// command, and what came of it, one line each with its time, inside the popover `r`.
fn draw_check_why(app: &App, buf: &mut Buffer, r: Rect, info: &TermInfo, check: &CheckInfo, t: &Theme) {
    let c = Style::default().bg(t.card);
    let (x, right) = (r.x + 2, r.right().saturating_sub(1));
    let time = |at: u64| if at == 0 { " ".repeat(TIME_COL) } else { format!("{} ", crate::clock::local_hms(at)) };
    let said = info.said.lines().rev().map(str::trim).find(|l| !l.is_empty());
    let mut lines: Vec<(u64, Vec<Seg>)> = Vec::new();
    if check.after_turn_ended_at > 0 {
        let quote = said.map(|s| format!(" · it said \"{s}\"")).unwrap_or_default();
        lines.push((check.after_turn_ended_at, vec![seg(format!("agent's turn ended{quote}"), c.fg(t.text))]));
    }
    if check.started_at > 0 {
        lines.push((check.started_at, vec![seg(concat!(env!("APP_NAME"), " ran "), c.fg(t.text)), seg(check.command.clone(), c.fg(t.strong).add_modifier(Modifier::BOLD)), seg(" in its worktree", c.fg(t.text))]));
    }
    let (result, color) = check_result(app, check, t);
    let finished = if check.state == CheckState::Checking || check.started_at == 0 { 0 } else { check.started_at + check.duration_ms / 1000 };
    lines.push((finished, vec![seg(result, c.fg(color).add_modifier(Modifier::BOLD))]));
    for (i, (at, body)) in lines.into_iter().enumerate() {
        let mut row = vec![seg(time(at), c.fg(t.muted))];
        row.extend(body);
        put(buf, x, r.y + 2 + i as u16, &row, right);
    }
    put(buf, x, r.y + 6, &cap_hints_fit(t, t.card, &check_keys(check.state), right.saturating_sub(x)), right);
}

impl App {
    /// Start a follow-up to `term`; `inbox`: from that Inbox place (query, row). A draft kept
    /// with Esc comes back.
    pub(in crate::client) fn start_compose(&mut self, term: TermId, side: bool, inbox: Option<(String, usize)>) {
        let agent = self.snap.terms.get(&term).is_some_and(|t| t.agent.is_some());
        if !agent {
            self.notify("only an agent takes a follow-up".into(), true);
            return;
        }
        let text = self.screen.drafts.get(&term).cloned().unwrap_or_default();
        self.mode = Mode::Compose(Box::new(Compose { term, text, side, inbox }));
    }

    /// Back where the follow-up was started from.
    fn compose_back(&mut self, c: &Compose) {
        self.mode = match &c.inbox {
            Some((query, sel)) => Mode::GoTo { query: query.clone(), sel: *sel },
            None if c.side => Mode::Side,
            None => Mode::Normal,
        };
    }

    pub(in crate::client) fn on_compose_key(&mut self, mut c: Compose, k: &KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let newline = k.modifiers.intersects(KeyModifiers::SHIFT | KeyModifiers::ALT);
        match k.code {
            KeyCode::Esc => {
                if c.text.trim().is_empty() {
                    self.screen.drafts.remove(&c.term);
                } else {
                    self.screen.drafts.insert(c.term, c.text.clone());
                }
                return self.compose_back(&c);
            }
            KeyCode::Enter if newline => c.text.push('\n'),
            KeyCode::Enter => {
                let text = c.text.trim_end().to_string();
                if !text.trim().is_empty() {
                    self.screen.drafts.remove(&c.term);
                    // Typed into a working agent it would interrupt, or be lost: it waits its turn.
                    let working = self.snap.terms.get(&c.term).is_some_and(|t| t.status == Status::Working);
                    if working && !ctrl {
                        self.queue_follow_up(c.term, &text);
                    } else {
                        self.type_follow_up(c.term, &text);
                        self.note_receipt(c.term, &text, false);
                    }
                }
                return self.compose_back(&c);
            }
            KeyCode::Backspace => {
                c.text.pop();
            }
            KeyCode::Char('u') if ctrl => c.text.clear(),
            KeyCode::Char(ch) if !ctrl => c.text.push(ch),
            _ => {}
        }
        self.mode = Mode::Compose(Box::new(c));
    }

    /// Show on `term`'s row that `text` was sent now, or is queued for the end of the turn.
    fn note_receipt(&mut self, term: TermId, text: &str, queued: bool) {
        let first = text.lines().find(|l| !l.trim().is_empty()).unwrap_or_default().trim();
        let status = self.snap.terms.get(&term).map(|t| t.status).unwrap_or(Status::None);
        let receipt = Receipt { text: truncate(first, RECEIPT_TEXT_MAX), queued, seen_queued: false, since: Instant::now(), status };
        self.screen.receipts.insert(term, receipt);
    }

    /// Leave `text` with the server for the agent: it's typed in when this turn is over.
    pub(in crate::client) fn queue_follow_up(&mut self, term: TermId, text: &str) {
        self.cmd(Command::QueueFollowUp { term, text: text.to_string() });
        self.note_receipt(term, text, true);
    }

    /// Type `text` into the agent's prompt and press Enter (a moment later, once it has taken
    /// the text); several lines go in as one paste.
    fn type_follow_up(&mut self, term: TermId, text: &str) {
        let bracketed = self.parsers.get(&term).is_some_and(|p| p.screen().bracketed_paste());
        let data = if text.contains('\n') && bracketed { format!("\x1b[200~{}\x1b[201~", text.replace('\n', "\r")) } else { text.replace('\n', " ") };
        self.send(ClientMsg::Input { term, data: data.into_bytes() });
        let out = self.out.clone();
        std::thread::spawn(move || {
            std::thread::sleep(ENTER_AFTER);
            let _ = out.send(ClientMsg::Input { term, data: b"\r".to_vec() });
        });
    }

    /// `i` again or Esc closes it; any other key closes it and does its own thing.
    pub(in crate::client) fn on_why_key(&mut self, term: TermId, side: bool, k: &KeyEvent) {
        self.mode = if side { Mode::Side } else { Mode::Normal };
        // A check's timeline shows the keys that act; the ones it doesn't show are not passed on
        // to the pane behind it.
        if let Some(info) = self.snap.terms.get(&term).and_then(|t| t.check().cloned()) {
            let key = match k.code {
                KeyCode::Enter => Some(('o', true)),
                KeyCode::Char('r') => Some(('r', check::settled(&info))),
                KeyCode::Char('O') => Some(('O', check::failed(&info))),
                _ => None,
            };
            if let Some((key, shown)) = key {
                if shown {
                    self.do_check_key(term, None, key);
                }
                return;
            }
        }
        if !matches!(k.code, KeyCode::Esc | KeyCode::Char('i')) {
            self.on_key(*k);
        }
    }
}

/// How long after the text the follow-up's Enter goes, so the agent has taken the text.
const ENTER_AFTER: std::time::Duration = std::time::Duration::from_millis(60);

#[cfg(test)]
mod tests {
    use super::wrap;

    #[test]
    fn the_text_area_wraps_on_words_and_keeps_blank_lines() {
        assert_eq!(wrap("one two three", 7), vec!["one two", "three"]);
        assert_eq!(wrap("a\n\nb", 10), vec!["a", "", "b"]);
        assert_eq!(wrap("abcdefghij", 4), vec!["abcd", "efgh", "ij"], "a word longer than the line breaks");
    }
}

#[cfg(test)]
mod why_tests {
    use super::super::reference::{assert_same, layout_name, reference, text};
    use super::*;
    use crate::client::reference_fixture::{LAYOUTS, SIZES, Stage, world};

    fn why_world(stage: Stage) -> App {
        let mut app = world(stage);
        app.cfg.ui.glyphs = "unicode".into();
        app.snap.terms.get_mut(&1).unwrap().said = "tests pass".into();
        app
    }

    /// The popover drawn in the rectangle the reference's inside is under.
    fn draw(app: &App, area: Rect, w: u16, h: u16) -> Vec<String> {
        let t = Theme::named(crate::theme::DEFAULT);
        let mut buf = Buffer::empty(Rect { x: 0, y: 0, width: w, height: h });
        let info = app.snap.terms[&1].clone();
        let r = Rect { y: area.y - 1, height: CHECK_WHY_H, ..area };
        draw_check_why(app, &mut buf, r, &info, info.check().unwrap(), &t);
        text(&buf, area)
    }

    /// The reference's sample counts the tests that failed, which a check's result does not carry.
    #[test]
    fn why_reads_the_turn_the_run_and_what_it_came_to() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut want = reference("13 Why? (i)", layout_name(layout), w, h);
                want.set_row(3, "  14:22:19 ✕ exit 1 after 14.2s");
                let app = why_world(Stage::Failed);
                assert_same(&draw(&app, want.area, w, h), &want, &format!("13 {layout:?} {w}x{h}"));
            }
        }
    }

    #[test]
    fn the_title_says_what_the_session_reads_as() {
        let title = |stage| {
            let app = why_world(stage);
            check_why_title(&app, &app.snap.terms[&1])
        };
        assert_eq!(title(Stage::Failed), "WHY · fix-footer reads failed");
        assert_eq!(title(Stage::Stale), "WHY · fix-footer reads done · stale");
        assert_eq!(title(Stage::Never), "WHY · fix-footer reads not checked");
    }

    #[test]
    fn a_result_that_is_not_a_plain_pass_or_failure_says_its_row_words() {
        let lines = |stage| draw(&why_world(stage), Rect { x: 0, y: 1, width: 60, height: 7 }, 60, 9).join("\n");
        assert!(lines(Stage::Over).contains("⊘ failed, overridden: test"), "{}", lines(Stage::Over));
        assert!(lines(Stage::Stale).contains("◷ stale pass: test · 3 files changed"));
        assert!(lines(Stage::Never).contains("? not checked: never for this repo"));
        assert!(lines(Stage::Approve).contains("● approve check: npm test"));
        assert!(lines(Stage::Passed).contains("✓ exit 0 after 14.2s"));
    }

    #[test]
    fn a_click_on_it_opens_the_evidence() {
        let mut app = why_world(Stage::Failed);
        app.mode = Mode::Why { term: 1, side: false };
        crate::client::reference_fixture::frame(&mut app, crate::client::reference_fixture::Layout::Floating, 100, 30, "unicode");
        assert!(app.hits.iter().any(|(_, h)| *h == crate::client::Hit::Screen(ScreenHit::CheckEvidence(1))));
        app.on_hy_hit(ScreenHit::CheckEvidence(1), false);
        assert!(matches!(&app.view, Some(crate::client::View::Check(v)) if v.term == 1));
    }

    #[test]
    fn its_keys_are_only_those_that_act() {
        let keys = |state| check_keys(state).iter().map(|(k, _)| *k).collect::<Vec<_>>();
        assert_eq!(keys(CheckState::Failed), ["⏎", "r", "O", "Esc"]);
        assert_eq!(keys(CheckState::Passed), ["⏎", "r", "Esc"]);
        assert_eq!(keys(CheckState::Checking), ["⏎", "Esc"]);
    }
}

#[cfg(test)]
mod message_tests {
    use super::*;
    use crate::client::reference_fixture::{LAYOUTS, SIZES, Stage, frame, world};

    const QUESTION: &str = "Both lockfiles are in the repo.\nCI runs pnpm install, but the README says npm.\nTwo scripts call npx directly.\nUse pnpm or npm for the workspace?\nTell me which.";

    fn open(term: TermId, text: &str) -> App {
        let mut app = world(Stage::Failed);
        app.snap.terms.get_mut(&term).unwrap().said = QUESTION.into();
        app.start_compose(term, true, None);
        if let Mode::Compose(c) = &mut app.mode {
            c.text = text.into();
        }
        app
    }

    fn type_keys(app: &mut App, text: &str) {
        for ch in text.chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
    }

    #[test]
    fn it_names_the_session_quotes_the_tail_and_points_at_the_row_in_every_layout_and_size() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut app = open(3, "");
                let o = frame(&mut app, layout, w, h, "unicode");
                let (quote, _) = size_tier(h);
                assert!(o.contains("MESSAGE warm-finch · codex"), "{layout:?} {w}x{h}\n{o}");
                assert!(o.contains(&format!("codex asked · last {quote} of 5 lines")) || quote >= 5, "{layout:?} {w}x{h}\n{o}");
                assert!(o.contains("▏ Tell me which.") && !o.contains("Both lockfiles"), "the tail, not the head\n{o}");
                assert!(o.contains("◂") && o.contains("Reply to codex…") && o.contains("● needs you"), "{layout:?} {w}x{h}\n{o}");
                let corner = o.lines().filter(|l| l.contains("MESSAGE")).any(|l| l.contains("┌─ MESSAGE") || l.contains("╭─ MESSAGE") || l.contains("┌ MESSAGE") || l.contains(" MESSAGE"));
                assert!(corner);
                let top = o.lines().find(|l| l.contains("MESSAGE")).unwrap();
                assert!(top.contains('┐') || top.contains('╮'), "a full accent border in {layout:?}: {top}");
            }
        }
    }

    #[test]
    fn it_grows_a_row_a_line_then_scrolls() {
        let rows_of = |lines: usize, w: u16, h: u16| {
            let mut app = open(3, &(1..=lines).map(|i| format!("line {i}")).collect::<Vec<_>>().join("\n"));
            let o = frame(&mut app, crate::client::reference_fixture::Layout::Floating, w, h, "unicode");
            (o.lines().filter(|l| l.contains("│ › line") || l.contains("│   line ")).count(), o)
        };
        assert_eq!(rows_of(1, 160, 45).0, 1);
        assert_eq!(rows_of(3, 160, 45).0, 3);
        let (n, o) = rows_of(9, 160, 45);
        assert!(n == 5 && o.contains("↑ 4 more lines") && o.contains("line 9█"), "{o}");
        let (n, o) = rows_of(9, 100, 30);
        assert!(n == 3 && o.contains("↑ 6 more lines"), "{o}");
        let (n, o) = rows_of(9, 80, 24);
        assert!(n == 2 && o.contains("↑ 7 more lines"), "{o}");
    }

    #[test]
    fn it_opens_upward_then_pins_when_neither_side_has_room() {
        let screen = Rect { x: 0, y: 0, width: 100, height: 30 };
        assert_eq!(place(screen, Rect { x: 0, y: 5, width: 30, height: 2 }, 9), 4, "a line above the row");
        let top = place(screen, Rect { x: 0, y: 26, width: 30, height: 2 }, 9);
        assert_eq!(top + 9, 29, "its bottom border just under the row: {top}");
        assert_eq!(place(screen, Rect { x: 0, y: 28, width: 30, height: 1 }, 40), 0, "taller than the screen: pinned");
        assert_eq!(place(Rect { height: 20, ..screen }, Rect { x: 0, y: 19, width: 30, height: 1 }, 12), 8);
    }

    #[test]
    fn a_working_agent_queues_and_the_row_says_so_then_says_sent() {
        let mut app = open(2, "");
        let o = frame(&mut app, crate::client::reference_fixture::Layout::Floating, 160, 45, "unicode");
        assert!(o.contains("codex is mid-turn") || o.contains("claude is mid-turn"), "{o}");
        assert!(o.contains("Ctrl") && o.contains("Enter") && o.contains("send now"), "{o}");
        type_keys(&mut app, "also test a duplicate");
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.mode == Mode::Side, "closed");
        let o = frame(&mut app, crate::client::reference_fixture::Layout::Floating, 160, 45, "unicode");
        assert!(o.contains("◷ queued \"also test a"), "{o}");
        let info = app.snap.terms.get_mut(&2).unwrap();
        info.queued = vec!["also test a duplicate".into()];
        refresh_receipts(&mut app);
        assert!(app.screen.receipts[&2].queued, "still waiting");
        let info = app.snap.terms.get_mut(&2).unwrap();
        (info.queued, info.status) = (Vec::new(), Status::Idle);
        let o = frame(&mut app, crate::client::reference_fixture::Layout::Floating, 160, 45, "unicode");
        assert!(o.contains("› sent \"also test a"), "{o}");
    }

    #[test]
    fn a_sent_receipt_clears_on_a_status_change_after_its_minimum_and_by_its_maximum() {
        let mut app = open(3, "");
        type_keys(&mut app, "pnpm");
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!app.screen.receipts[&3].queued);
        app.snap.terms.get_mut(&3).unwrap().status = Status::Working;
        refresh_receipts(&mut app);
        assert!(app.screen.receipts.contains_key(&3), "too soon to clear");
        app.screen.receipts.get_mut(&3).unwrap().since = Instant::now() - RECEIPT_MIN_SHOWN - Duration::from_millis(1);
        refresh_receipts(&mut app);
        assert!(!app.screen.receipts.contains_key(&3), "the status changed");
        app.note_receipt(3, "x", false);
        app.screen.receipts.get_mut(&3).unwrap().since = Instant::now() - RECEIPT_MAX_SHOWN - Duration::from_millis(1);
        refresh_receipts(&mut app);
        assert!(!app.screen.receipts.contains_key(&3), "timed out");
    }

    #[test]
    fn esc_keeps_the_draft_for_that_session_and_an_empty_enter_only_closes() {
        let mut app = open(3, "");
        type_keys(&mut app, "half a thought");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.mode == Mode::Side && app.screen.receipts.is_empty());
        app.start_compose(1, true, None);
        assert!(matches!(&app.mode, Mode::Compose(c) if c.text.is_empty()), "another session's box is empty");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.start_compose(3, true, None);
        assert!(matches!(&app.mode, Mode::Compose(c) if c.text == "half a thought"), "its own draft is back");
        for _ in 0.."half a thought".len() {
            app.on_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        }
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(app.mode == Mode::Side && app.screen.receipts.is_empty(), "nothing sent");
    }
}
