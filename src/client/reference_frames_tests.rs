//! Every reference preset drawn whole on the fixture's world and set beside the reference
//! frames (`reference_frames/<preset>.txt`, fixed fixtures; see docs/TESTING.md), in Cards floating,
//! Cards tiled and Flat at 160×45, 100×30 and 80×24, and in ASCII at Flat 80×24.
//!
//! Cells the reference leaves to the app (masks):
//! - the pane's terminal text: the agent's own output;
//! - the working spinner's cell (the user's spinner, not the reference's 3-bar one) and ages the
//!   fixture can't pin to the sample (the working session reads `0s`, not `now`; the sample's
//!   approval item is a minute younger than its own row);
//! - the reference's sample checkpoint kinds (commits, approval, review) become the fixture's
//!   build and test.
//!
//! Where the sample cannot be the app's, the reference is changed to what the app draws, and
//! each change is a deliberate difference:
//! - a folder with a session that needs you is listed first (the app's order);
//! - quick-fox is finished and unseen, so it reads "finished · not seen", as the sample's Inbox
//!   says (its sidebar says "finished");
//! - a failed row's third line is its first failing line, the Inbox's sample line; the reference
//!   sets "footer shows current year" outside the failed branch, so other rows drop it;
//! - add-payments is a session of its own, not a second tab of fix-footer's pane, so fix-footer's
//!   lone tab has no number chip, and the pane's mark keeps its room where the reference draws
//!   the tab chips over it;
//! - a row waiting on a changed command (A4) shows the new command; a project with a check
//!   that never ran (16) names it in the folder's meta;
//! - a command you said never to (A5) reads "did not run" and its source names the file's
//!   commit (the result carries no time for your choice);
//! - Why (13) does not add "1 of 59 tests failed": the app does not parse a command's output;
//! - the C2 Inbox's footnote is an annotation, not a screen line;
//! - a failure's buttons end in Send to agent, which the reference predates: under 100 columns
//!   it takes the evidence sheet's Open session, and the Inbox has no room for it;
//! - pill caps (a Nerd Font option, on by default) are off, so buttons have square ends;
//! - the mode bar along the bottom (ADR 0024) is not the reference's: `reference_fixture::cells` draws
//!   a row taller and leaves out that row, the bar's.

use super::screen::{CAP_L, CAP_R, GoRow, goto_rows};
use super::reference_fixture::{Layout, Stage, cells, frame, world};
use super::*;
use crate::protocol::{CheckSource, CheckState};

/// A cell the reference leaves to the app.
const ANY: char = '\u{1}';
/// The reference's working spinner with motion off: its third 3-bar frame, and ASCII's.
const SPINNER: [&str; 2] = ["\u{28f6}\u{2846}", "O "];
/// The sample's first failing line, as the Inbox shows it.
const FAILING: &str = "✗ legal.test.tsx › footer shows current year";
const QUESTION: &str = "Use pnpm or npm for the workspace?";
const OUT_FAIL: &str = "> affiliate-site@1.4.0 test\n> vitest run\n\n ✓ src/components/Footer.test.tsx (3)\n ✓ src/lib/price.test.ts (9)\n ✗ src/pages/legal.test.tsx > footer shows current year\n   AssertionError: expected '© 2025 Affiliate Co.'\n                   to equal '© 2026 Affiliate Co.'\n    ❯ src/pages/legal.test.tsx:18:32\n\n Test Files  1 failed | 11 passed (12)\n      Tests  1 failed | 58 passed (59)\n   Duration  14.21s";
const OUT_PASS: &str = "> vitest run\n ✓ 12 test files · 59 tests\n   Duration  14.02s";
const LONG_LINES: u64 = 1284;
const HOUR: u64 = 3600;

/// One reference frame.
struct Block {
    layout: Layout,
    w: u16,
    h: u16,
    glyphs: String,
    rows: Vec<Vec<char>>,
}

fn blocks(file: &str) -> Vec<Block> {
    let mut out: Vec<Block> = Vec::new();
    for line in file.lines() {
        if let Some(head) = line.strip_prefix("===== ") {
            let parts: Vec<&str> = head.split(" | ").collect();
            let layout = match parts[0] {
                "floating" => Layout::Floating,
                "tiled" => Layout::Tiled,
                _ => Layout::Flat,
            };
            let (w, h) = parts[1].split_once('x').map(|(w, h)| (w.parse().unwrap(), h.parse().unwrap())).unwrap();
            out.push(Block { layout, w, h, glyphs: parts[2].into(), rows: Vec::new() });
        } else if let Some(b) = out.last_mut() {
            let mut row: Vec<char> = line.replace('\\', std::path::MAIN_SEPARATOR_STR).chars().collect();
            row.resize(b.w as usize, ' ');
            b.rows.push(row);
        }
    }
    out
}

/// What a preset opens over the world.
#[derive(Clone, Copy, PartialEq)]
enum Over {
    Rows,
    Evidence,
    Inbox,
    Override,
    Why,
    Settings,
    Approve,
}

/// A preset by its file's name: its stage and what it opens.
fn preset(id: &str) -> (Stage, Over) {
    match id {
        "01" => (Stage::None, Over::Rows),
        "02" => (Stage::Checking, Over::Rows),
        "03" => (Stage::Passed, Over::Rows),
        "04" => (Stage::Failed, Over::Rows),
        "05" => (Stage::Over, Over::Rows),
        "06" => (Stage::Couldnt, Over::Rows),
        "07" | "08" | "09" => (Stage::Failed, Over::Evidence),
        "10" => (Stage::Passed, Over::Evidence),
        "11" => (Stage::Failed, Over::Inbox),
        "12" => (Stage::Failed, Over::Override),
        "13" => (Stage::Failed, Over::Why),
        "14" | "16" => (Stage::None, Over::Settings),
        "15" => (Stage::Passed, Over::Settings),
        "17" => (Stage::Couldnt, Over::Settings),
        "A1" => (Stage::Approve, Over::Inbox),
        "A2" | "A3" | "A4" => (Stage::Approve, Over::Approve),
        "A5" => (Stage::Never, Over::Evidence),
        "B1" => (Stage::Stale, Over::Rows),
        "B2" => (Stage::Stale, Over::Evidence),
        "C1" => (Stage::Multi, Over::Rows),
        "C2" => (Stage::Multi, Over::Inbox),
        _ => panic!("no preset {id}"),
    }
}

/// The preset whose sidebar shows `stage` with nothing over it.
fn rows_preset(stage: Stage) -> &'static str {
    match stage {
        Stage::None => "01",
        Stage::Checking => "02",
        Stage::Passed => "03",
        Stage::Failed => "04",
        Stage::Over => "05",
        Stage::Couldnt => "06",
        Stage::Approve => "A1",
        Stage::Never => "A5",
        Stage::Stale => "B1",
        Stage::Multi => "C1",
    }
}

fn frames(id: &str) -> &'static str {
    match id {
        "01" => include_str!("reference_frames/01.txt"),
        "02" => include_str!("reference_frames/02.txt"),
        "03" => include_str!("reference_frames/03.txt"),
        "04" => include_str!("reference_frames/04.txt"),
        "05" => include_str!("reference_frames/05.txt"),
        "06" => include_str!("reference_frames/06.txt"),
        "07" => include_str!("reference_frames/07.txt"),
        "08" => include_str!("reference_frames/08.txt"),
        "09" => include_str!("reference_frames/09.txt"),
        "10" => include_str!("reference_frames/10.txt"),
        "11" => include_str!("reference_frames/11.txt"),
        "12" => include_str!("reference_frames/12.txt"),
        "13" => include_str!("reference_frames/13.txt"),
        "14" => include_str!("reference_frames/14.txt"),
        "15" => include_str!("reference_frames/15.txt"),
        "16" => include_str!("reference_frames/16.txt"),
        "17" => include_str!("reference_frames/17.txt"),
        "A1" => include_str!("reference_frames/A1.txt"),
        "A2" => include_str!("reference_frames/A2.txt"),
        "A3" => include_str!("reference_frames/A3.txt"),
        "A4" => include_str!("reference_frames/A4.txt"),
        "A5" => include_str!("reference_frames/A5.txt"),
        "B1" => include_str!("reference_frames/B1.txt"),
        "B2" => include_str!("reference_frames/B2.txt"),
        "C1" => include_str!("reference_frames/C1.txt"),
        "C2" => include_str!("reference_frames/C2.txt"),
        _ => panic!("no frames for {id}"),
    }
}

// ---- the app's side: the fixture's world with the preset open --------------------------------

fn with_check(app: &mut App, command: &str) {
    let site = app.snap.terms[&1].root.clone().expect("fix-footer is in a repo");
    app.cfg.check = vec![crate::config::CheckDef { project: site, name: None, command: command.into(), ..Default::default() }];
}

/// The sample's words where the fixture has none of its own.
fn sample_words(app: &mut App) {
    app.cfg.ui.rounded = crate::config::Rounding::Off;
    let footer = app.snap.terms.get_mut(&1).unwrap();
    footer.said = "tests pass".into();
    for c in &mut footer.checks {
        if c.state == CheckState::Failed {
            c.first_failing_line = FAILING.into();
        }
    }
    app.screen_sync();
}

fn select_in_inbox(app: &mut App, term: TermId) {
    let rows = goto_rows(&app.screen_model(), "", 0);
    let sel = rows.iter().position(|r: &GoRow| r.term() == Some(term)).expect("the item is listed");
    app.mode = Mode::GoTo { query: String::new(), sel };
}

fn open(id: &str) -> App {
    let (stage, over) = preset(id);
    let mut app = world(stage);
    sample_words(&mut app);
    match id {
        "15" | "16" => with_check(&mut app, "npm test"),
        "17" => with_check(&mut app, "pnpm test"),
        "A4" => {
            let check = &mut app.snap.terms.get_mut(&1).unwrap().checks[0];
            check.previous_command = Some("npm test".into());
            check.command = "npm test && node scripts/seed.js".into();
            check.source = CheckSource::Repo { file: concat!(".", env!("APP_COMMAND"), "/checks.toml").into(), commit: Some("9f3c2a1".into()), committed_at: Some(super::reference_fixture::now() - 2 * HOUR), uncommitted: false };
        }
        _ => {}
    }
    match over {
        Over::Rows => {
            app.mode = Mode::Side;
            app.screen.cursor = Some(1);
        }
        Over::Evidence | Over::Override => {
            let (output, lines) = match id {
                "08" => (OUT_FAIL, Some(LONG_LINES)),
                "09" | "10" | "A5" => ("", Some(0)),
                "B2" => (OUT_PASS, None),
                _ => (OUT_FAIL, None),
            };
            if let Some(n) = lines {
                app.snap.terms.get_mut(&1).unwrap().checks[0].line_count = n;
            }
            app.open_check_sheet(1);
            app.check_output_arrived(output.into());
            if over == Over::Override {
                app.ask_override(1, None);
                app.on_paste("flaky legal test, tracked in #212".into());
            }
        }
        Over::Inbox => select_in_inbox(&mut app, if id == "C2" { 3 } else { 1 }),
        Over::Why => app.mode = Mode::Why { term: 1, side: true },
        Over::Settings => app.open_project_check(1),
        Over::Approve => {
            app.open_approval(1);
            // The first 2, pressed once the command was on screen.
            if id == "A3"
                && let Mode::Approve(a) = &mut app.mode
            {
                a.armed = true;
            }
        }
    }
    app
}

// ---- the reference's side: where things are, and what the app may draw its own way ------------

/// The reference's grid for a `w`-wide frame.
struct Geometry {
    pane: Option<Rect>,
    /// Where the pane's title row's right-hand mark ends (exclusive).
    pane_end: u16,
    /// The sheet's inside, between its side borders (or its left rule and the edge in Flat).
    sheet: Option<Rect>,
    overlays: Vec<Rect>,
    side_inner: Rect,
}

fn geometry(id: &str, b: &Block) -> Geometry {
    let (_, over) = preset(id);
    let (w, h) = (b.w, b.h);
    let bordered = b.layout != Layout::Flat;
    let side_w = if w >= 200 { 40 } else if w >= 140 { 34 } else { 26 };
    let gut = u16::from(b.layout == Layout::Floating);
    let ov = u16::from(!bordered);
    let sheet = matches!(over, Over::Evidence | Over::Inbox | Over::Override);
    let beside = sheet && w >= 140;
    let px = side_w + gut - ov;
    let pw = w - px;
    let pane_w = if beside { pw - 72 - gut + ov } else { pw };
    let pane = (!(sheet && !beside)).then(|| {
        if bordered {
            Rect { x: px + 1, y: 1, width: pane_w - 2, height: h - 2 }
        } else {
            let right = if beside { px + pane_w - 1 } else { w };
            Rect { x: px + 1, y: 1, width: right - px - 1, height: h - 1 }
        }
    });
    // A modal or popover, and the reference's hard shadow a cell right and down.
    let with_shadow = |r: Rect| vec![r, Rect { x: r.x + 1, y: r.y + 1, ..r }];
    let modal = |mw: u16, mh: u16| with_shadow(Rect { x: (w - mw) / 2, y: (h - mh) / 2, width: mw, height: mh });
    let overlays = match over {
        Over::Override => modal((w - 2).min(70), 15),
        Over::Approve => modal((w - 2).min(84), (h - 1).min(20) - u16::from(h < 30)),
        Over::Settings => modal((w - 2).min(80), (h - 2).min(20)),
        Over::Why => {
            let x = (side_w + 2).min(w - 40);
            with_shadow(Rect { x, y: if h < 30 { 3 } else { 5 }, width: 60.min(w - x - 2), height: 8 })
        }
        _ => Vec::new(),
    };
    let side_inner = if bordered { Rect { x: 1, y: 1, width: side_w - 2, height: h - 2 } } else { Rect { x: 0, y: 1, width: side_w - 1, height: h - 1 } };
    let sheet = sheet.then(|| {
        let (sx, sw) = if beside { (px + pane_w + gut - ov, 72) } else { (px, pw) };
        // A sheet's content keeps the Cards box in Flat too.
        Rect { x: sx + 1, y: 1, width: sw - 2, height: h - 2 }
    });
    Geometry { pane, pane_end: px + pane_w - 1, sheet, overlays, side_inner }
}

fn covered(g: &Geometry, x: u16, y: u16) -> bool {
    g.overlays.iter().any(|r| r.contains(ratatui::layout::Position { x, y }))
}

/// `from` in `row` starting at `at`, replaced by `to`, the rest of `row` shifted to keep its
/// width (padding at `end`).
fn splice(row: &mut Vec<char>, at: usize, from_len: usize, to: &[char], end: usize) {
    let tail: Vec<char> = row[at + from_len..end].to_vec();
    let mut mid: Vec<char> = to.to_vec();
    mid.extend(tail);
    mid.resize(end - at, ' ');
    row.splice(at..end, mid);
}

fn find(row: &[char], needle: &str) -> Option<usize> {
    let n: Vec<char> = needle.chars().collect();
    (0..row.len().saturating_sub(n.len() - 1)).find(|&i| row[i..i + n.len()] == n[..])
}

/// Replace `from` with `to` in `row` within `[start, end)`, keeping the columns after `end`.
fn swap(row: &mut Vec<char>, from: &str, to: &str, start: usize, end: usize) -> bool {
    let Some(i) = find(&row[start..end], from).map(|i| i + start) else { return false };
    splice(row, i, from.chars().count(), &to.chars().collect::<Vec<_>>(), end);
    true
}

/// From `marker` to `end`, `to` in place of whatever the reference had (it clips there).
fn swap_tail(row: &mut Vec<char>, marker: &str, to: &str, start: usize, end: usize) -> bool {
    let Some(i) = find(&row[start..end], marker).map(|i| i + start) else { return false };
    let mut new: Vec<char> = to.chars().collect();
    new.truncate(end - i);
    new.resize(end - i, ' ');
    row.splice(i..end, new);
    true
}

/// The sidebar's inner rows as the app lists the sample: folder order, quick-fox unseen, a
/// failed row's third line, and the fixture's checkpoints.
fn app_sidebar(id: &str, stage: Stage, rows: Vec<Vec<char>>, inner_w: usize, ascii: bool) -> Vec<Vec<char>> {
    let (glyph_ok, glyph_bad, glyph_need) = if ascii { ('+', 'x', '!') } else { ('✓', '✕', '●') };
    let end = inner_w;
    let mut rows = rows;
    for row in &mut rows {
        // The reference keeps the row's old command after the repo changed it.
        if id == "A4" {
            swap_tail(row, "approve check: npm test", "approve check: npm test && node scripts/seed.js", 0, end);
        }
        for sp in SPINNER {
            if let Some(i) = find(row, &format!("{sp} add-payments")) {
                let to: Vec<char> = format!("{ANY} add-payments ").chars().collect();
                splice(row, i, sp.chars().count() + " add-payments".chars().count(), &to, end);
            }
        }
        // The app leaves the agent's name off a working line; it is in the pane's title.
        swap(row, "claude · working", "working", 0, end);
        if find(row, "add-payments").is_some()
            && let Some(i) = find(row, " now")
        {
            row[i + 1..i + 4].fill(ANY);
        }
        if stage == Stage::Multi {
            if find(row, "affiliate-site").is_some() {
                swap(row, "· 4 ", "· 2 ", 0, end);
            }
            swap_tail(row, "failed: test", &format!("failed: test {glyph_ok}{glyph_bad}"), 0, end);
            swap_tail(row, "needs you: approval", &format!("needs you: test {glyph_ok}{glyph_need}"), 0, end);
            if find(row, &format!("test {glyph_bad}")).is_some() {
                swap_tail(row, "commits", &format!("build {glyph_ok} · test {glyph_bad}"), 0, end);
            } else {
                swap_tail(row, "commits", &format!("build {glyph_ok} · test {glyph_need}"), 0, end);
            }
        }
    }
    let text = |r: &Vec<char>| r.iter().collect::<String>();
    // Line 3: the first failing line on a failed row, nothing on any other.
    if let Some(i) = rows.iter().position(|r| text(r).contains("footer shows current year")) {
        if stage == Stage::Failed {
            let at = find(&rows[i], "footer shows current year").unwrap();
            swap_tail(&mut rows[i], "footer shows current year", FAILING, at, end);
        } else {
            rows.remove(i);
            rows.insert(rows.len() - 1, vec![' '; inner_w]);
        }
    }
    // Sessions are one line now, each with its agent's mark (ADR 0021).
    let n = rows.len();
    let foot_row = rows.pop().expect("the foot");
    let mut rows = super::reference_fixture::as_one_line_rows(rows, ascii, ANY);
    for r in &mut rows {
        r.resize(inner_w, ' ');
    }
    rows.resize(n - 1, vec![' '; inner_w]);
    rows.push(foot_row);
    // Folders: the one with a session asking you goes first.
    let foot = rows.len() - 1;
    let mut groups: Vec<Vec<Vec<char>>> = Vec::new();
    let mut cur: Vec<Vec<char>> = Vec::new();
    for r in &rows[1..foot] {
        if text(r).trim().is_empty() {
            if !cur.is_empty() {
                groups.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(r.clone());
        }
    }
    if !cur.is_empty() {
        groups.push(cur);
    }
    groups.sort_by_key(|g| !g.iter().any(|r| text(r).contains("warm-finch")));
    let mut out = vec![rows[0].clone()];
    for g in groups {
        out.extend(g);
        out.push(vec![' '; inner_w]);
    }
    while out.len() < foot {
        out.push(vec![' '; inner_w]);
    }
    out.truncate(foot);
    out.push(rows[foot].clone());
    out
}

/// A row of key hints (`Esc close`) is drawn as keycaps in an action bar, which spaces its words
/// out (and, in the Why popover, adds the Enter key); its words are tested on their own.
fn mask_key_bar(row: &mut [char], x0: usize, x1: usize) {
    let inside: String = row[x0..x1].iter().collect();
    if inside.contains("Esc ") && ["select", "re-run", "evidence", "back", "close"].iter().any(|w| inside.contains(w)) {
        row[x0..x1].fill(ANY);
    }
}

/// A modal's title row in `[x0, x1)` as the app draws it since modals took a pane's frame (the
/// user's choice, 2026-10-24, docs/UI.md): the reference's accent bar becomes the top border with
/// the corners of its `bottom` row, its title a chip two in and `Esc close` a chip a `─` before
/// the corner. Every other cell of the modal is still the reference's.
fn as_bordered_header(row: &mut [char], bottom: &[char], x0: usize, x1: usize) {
    let bar: String = row[x0..x1].iter().collect();
    let title = bar.trim().strip_suffix("Esc close").unwrap_or_else(|| panic!("the reference's title bar: {bar:?}")).trim_end();
    let top = |c: char| match c {
        '└' => '┌',
        '┘' => '┐',
        c => c,
    };
    let tail: Vec<char> = format!(" Esc close ─{}", top(bottom[x1 - 1])).chars().collect();
    let mut header: Vec<char> = format!("{}─ {title} ", top(bottom[x0])).chars().collect();
    header.resize(x1 - x0 - tail.len(), '─');
    header.extend(tail);
    row[x0..x1].copy_from_slice(&header);
}

/// The reference frame as the app should draw it on the fixture: masks and sample swaps.
fn expected(id: &str, b: &Block) -> Vec<Vec<char>> {
    let (stage, _) = preset(id);
    let g = geometry(id, b);
    let mut want = b.rows.clone();
    let source = blocks(frames(rows_preset(stage)))
        .into_iter()
        .find(|s| s.layout == b.layout && (s.w, s.h) == (b.w, b.h) && s.glyphs == b.glyphs)
        .expect("the stage's rows");
    let si = g.side_inner;
    let ascii = b.glyphs == "ascii";
    let inner_of = |rows: &[Vec<char>]| -> Vec<Vec<char>> { (si.y..si.bottom()).map(|y| rows[y as usize][si.x as usize..si.right() as usize].to_vec()).collect() };
    let mut side = app_sidebar(id, stage, inner_of(&source.rows), si.width as usize, ascii);
    // 16's project has a check that never ran; the reference names it only once there is a result.
    if id == "16" {
        let passed = blocks(frames("03")).into_iter().find(|s| s.layout == b.layout && (s.w, s.h) == (b.w, b.h) && s.glyphs == b.glyphs).expect("03");
        let head = |rows: &[Vec<char>]| rows.iter().position(|r| find(r, "affiliate-site").is_some()).expect("the folder");
        let with_meta = inner_of(&passed.rows);
        let i = head(&side);
        side[i] = with_meta[head(&with_meta)].clone();
    }
    for (dy, row) in side.iter().enumerate() {
        let y = si.y + dy as u16;
        for (dx, c) in row.iter().enumerate() {
            let x = si.x + dx as u16;
            if !covered(&g, x, y) {
                want[y as usize][x as usize] = *c;
            }
        }
    }
    if let Some(p) = g.pane {
        for y in p.y..p.bottom() {
            for x in p.x..p.right() {
                if !covered(&g, x, y) {
                    want[y as usize][x as usize] = ANY;
                }
            }
        }
    }
    // A modal's title row is a border now; it holds no key bar, so no mask passes over it.
    let modal = matches!(preset(id).1, Over::Override | Over::Approve | Over::Settings);
    let header = g.overlays.first().filter(|_| modal).map(|m| {
        let bottom = want[m.bottom() as usize - 1].clone();
        as_bordered_header(&mut want[m.y as usize], &bottom, m.x as usize, m.right() as usize);
        m.y as usize
    });
    // The sample's second tab is add-payments, a session of its own in the app (the sidebar
    // lists it); fix-footer has one tab, so its title has no tab number either.
    let flat = b.layout == Layout::Flat;
    let fill = if flat { " " } else { "─" };
    let top = &mut want[0];
    let w = top.len();
    swap(top, " 2  add-payments ", &fill.repeat(17), 0, w);
    swap(top, " 1  fix-footer ", &format!(" fix-footer {}", fill.repeat(3)), 0, w);
    // The reference draws the tab chips over the pane's mark when they meet; the app gives the
    // mark its room.
    if g.pane.is_some() {
        let mark = pane_mark(stage, b.glyphs == "ascii");
        let mark: Vec<char> = (if flat { mark } else { format!(" {mark} ") }).chars().collect();
        let end = g.pane_end as usize;
        top[end - mark.len()..end].copy_from_slice(&mark);
    }
    if let Some(s) = g.sheet {
        // Text inside a sheet stops a column before its right edge.
        let (x0, x1) = (s.x as usize, s.right() as usize - 1);
        let room = s.width as usize - 4;
        let (ok, bad, need) = if ascii { ('+', 'x', '!') } else { ('✓', '✕', '●') };
        let line = |word: &str, last: char| {
            let full = format!("{word} · build {ok} · test {last}");
            if full.chars().count() <= room { full } else { format!("{word} {ok}{last}") }
        };
        for (y, row) in want.iter_mut().enumerate().take(s.bottom() as usize).skip(s.y as usize) {
            if Some(y) != header {
                mask_key_bar(row, x0, x1);
            }
            swap(row, QUESTION, "use pnpm or npm?", x0, x1);
            // A failure offers Send to agent after Override. At 80 columns it takes the
            // evidence sheet's room for Open session, and the Inbox has no room for it.
            if b.w >= 100 {
                swap(row, "Override… O", "Override… O   Send to agent m", x0, x1);
            } else {
                swap(row, "Override… O   Open session o", "Override… O   Send to agent m", x0, x1);
            }
            // The sample's approval item is a minute younger than its row; the app reads one age.
            if stage == Stage::Approve && find(&row[x0..x1], "fix-footer").is_some() && let Some(i) = find(&row[x0..x1], "1m") {
                row[x0 + i..x0 + i + 2].fill(ANY);
            }
            if stage == Stage::Multi {
                swap_tail(row, "needs you: approval", &line("needs you: test", need), x0, x1);
                swap_tail(row, "failed: test · commits", &line("failed: test", bad), x0, x1);
                swap_tail(row, &format!("failed: test {ok}{bad}"), &line("failed: test", bad), x0, x1);
                swap_tail(row, "Several checkpoints read as one line", "", x0, x1);
            }
            // A command you said never to never ran: no run time, and its source names the
            // file's commit (when you chose is not part of the result).
            if stage == Stage::Never {
                swap_tail(row, "14:22:05 · took", "did not run · after turn 3 ended 14:21:51", x0, x1);
                swap_tail(row, concat!(".", env!("APP_COMMAND"), "/checks.toml · "), concat!(".", env!("APP_COMMAND"), "/checks.toml · a1b2c3d · you chose never"), x0, x1);
            }
        }
    }
    // The sample's "1 of 59 tests failed" is read from vitest's output; the app does not parse
    // a command's output.
    for pop in &g.overlays {
        for (y, row) in want.iter_mut().enumerate().take(pop.bottom() as usize).skip(pop.y as usize) {
            if Some(y) != header {
                mask_key_bar(row, pop.x as usize, pop.right() as usize - 1);
            }
        }
    }
    if let Some(pop) = g.overlays.first().filter(|_| preset(id).1 == Over::Why) {
        // Its text stops a column before its right edge.
        let (x0, x1) = (pop.x as usize, pop.right() as usize - 1);
        for row in want.iter_mut().take(pop.bottom() as usize).skip(pop.y as usize) {
            swap_tail(row, " · 1 of 59", "", x0, x1);
        }
    }
    // The reference casts a hard shadow; the app casts none, so what shows there is the app's.
    if let [frame, shadow] = g.overlays[..] {
        for y in shadow.y..shadow.bottom().min(b.h) {
            for x in shadow.x..shadow.right().min(b.w) {
                if !frame.contains(ratatui::layout::Position { x, y }) {
                    want[y as usize][x as usize] = ANY;
                }
            }
        }
    }
    want
}

/// fix-footer's mark on its pane: glyph, word and the close.
fn pane_mark(stage: Stage, ascii: bool) -> String {
    let (u, a, word) = match stage {
        Stage::None | Stage::Passed => ("✓", "+", "done"),
        Stage::Checking => ("◜", "-", "checking"),
        Stage::Failed | Stage::Multi => ("✕", "x", "failed"),
        Stage::Over => ("⊘", "/", "done · overridden"),
        Stage::Couldnt | Stage::Never => ("?", "?", "not checked"),
        Stage::Approve => ("●", "!", "needs you"),
        Stage::Stale => ("◷", "~", "done · stale"),
    };
    let (g, close) = if ascii { (a, "x") } else { (u, "✕") };
    format!("{g} {word}  {close} ")
}

fn got_rows(app: &mut App, b: &Block) -> Vec<Vec<char>> {
    let text = frame(app, b.layout, b.w, b.h, &b.glyphs);
    text.lines()
        .map(|l| {
            let mut r: Vec<char> = l.chars().collect();
            r.resize(b.w as usize, ' ');
            r
        })
        .collect()
}

/// Every frame of preset `id` against its reference; the report shows each row that differs.
fn conforms(id: &str) {
    let mut report = String::new();
    for b in blocks(frames(id)) {
        let mut app = open(id);
        let got = got_rows(&mut app, &b);
        let want = expected(id, &b);
        let mut diff = String::new();
        for (y, (gr, wr)) in got.iter().zip(&want).enumerate() {
            let same = gr.len() == wr.len() && gr.iter().zip(wr).all(|(a, w)| a == w || *w == ANY);
            if !same {
                let shown = |r: &Vec<char>| r.iter().map(|c| if *c == ANY { '░' } else { *c }).collect::<String>().trim_end().to_string();
                diff.push_str(&format!("≠{y:>2} got  |{}\n    want |{}\n", shown(gr), shown(wr)));
            }
        }
        if !diff.is_empty() {
            report.push_str(&format!("===== {id} | {:?} | {}x{} | {}\n{diff}", b.layout, b.w, b.h, b.glyphs));
        }
    }
    assert!(report.is_empty(), "{id}: the frames differ from the reference\n{report}");
}

#[test]
fn preset_01_no_check_set() {
    conforms("01");
}

#[test]
fn preset_02_row_checking() {
    conforms("02");
}

#[test]
fn preset_03_row_passed() {
    conforms("03");
}

#[test]
fn preset_04_row_failed() {
    conforms("04");
}

#[test]
fn preset_05_row_overridden() {
    conforms("05");
}

#[test]
fn preset_06_row_could_not_run() {
    conforms("06");
}

#[test]
fn preset_07_evidence_short_output() {
    conforms("07");
}

#[test]
fn preset_08_evidence_long_output() {
    conforms("08");
}

#[test]
fn preset_09_evidence_no_output() {
    conforms("09");
}

#[test]
fn preset_10_evidence_printed_nothing() {
    conforms("10");
}

#[test]
fn preset_11_inbox() {
    conforms("11");
}

#[test]
fn preset_12_override() {
    conforms("12");
}

#[test]
fn preset_13_why() {
    conforms("13");
}

#[test]
fn preset_14_settings_empty() {
    conforms("14");
}

#[test]
fn preset_15_settings_set() {
    conforms("15");
}

#[test]
fn preset_16_settings_never_run() {
    conforms("16");
}

#[test]
fn preset_17_settings_command_missing() {
    conforms("17");
}

#[test]
fn preset_a1_repo_command_waits() {
    conforms("A1");
}

#[test]
fn preset_a2_approve_read_first() {
    conforms("A2");
}

#[test]
fn preset_a3_approve_confirm_always() {
    conforms("A3");
}

#[test]
fn preset_a4_approve_command_changed() {
    conforms("A4");
}

#[test]
fn preset_a5_never_for_this_repo() {
    conforms("A5");
}

#[test]
fn preset_b1_stale_row() {
    conforms("B1");
}

#[test]
fn preset_b2_stale_evidence() {
    conforms("B2");
}

#[test]
fn preset_c1_several_checkpoints_rows() {
    conforms("C1");
}

#[test]
fn preset_c2_several_checkpoints_inbox() {
    conforms("C2");
}

#[test]
fn the_sidebar_keeps_its_focus_while_its_why_popover_is_open() {
    let border = |app: &mut App| {
        (app.cfg.ui.layout, app.cfg.ui.panes) = ("cards".into(), "floating".into());
        app.screen_fresh();
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 45)).expect("a test terminal");
        term.draw(|f| render::draw(app, f)).expect("a frame");
        let r = app.screen.side_rect;
        let buf = term.backend().buffer();
        (r.y..r.bottom()).map(|y| buf[(r.x, y)].fg).collect::<Vec<_>>()
    };
    let mut app = open("13");
    assert!(matches!(app.mode, Mode::Why { side: true, .. }));
    let why = border(&mut app);
    app.mode = Mode::Side;
    assert_eq!(why, border(&mut app), "lit as when the sidebar has the keys");
    app.mode = Mode::Normal;
    assert_ne!(why, border(&mut app), "and not as when the pane has them");
}

// ---- colour roles: the cells that carry a meaning, and the Flat ground ----------------------

/// Where `needle` starts on screen, read cell by cell.
fn at(buf: &ratatui::buffer::Buffer, needle: &str) -> (u16, u16) {
    let want: Vec<String> = needle.chars().map(String::from).collect();
    let a = buf.area;
    for y in a.top()..a.bottom() {
        let row: Vec<&str> = (a.left()..a.right()).map(|x| buf[(x, y)].symbol()).collect();
        if let Some(x) = (0..row.len().saturating_sub(want.len() - 1)).find(|&x| row[x..x + want.len()].iter().zip(&want).all(|(g, w)| *g == w)) {
            return (x as u16, y);
        }
    }
    panic!("{needle:?} is not on screen");
}

#[test]
fn flat_draws_its_zones_on_the_ground_under_a_card_strip() {
    for id in ["01", "04", "C1"] {
        let mut app = open(id);
        let buf = cells(&mut app, Layout::Flat, 160, 45, "unicode");
        let t = &app.theme;
        let side = app.screen.side_rect;
        let mid = side.x + side.width / 2;
        assert_eq!(buf[(mid, 0)].bg, t.card, "{id}: row 0 is the header strip");
        for y in [1, side.bottom() - 2] {
            // The ground, or the terminal's own background standing for it.
            let bg = buf[(mid, y)].bg;
            assert!(bg == t.bg || bg == ratatui::style::Color::Reset, "{id}: the sidebar's body at row {y} is the ground, not {bg:?}");
        }
    }
}

#[test]
fn a_failed_check_reads_in_err_on_its_row_its_sheet_and_its_pane() {
    let mut app = open("07");
    let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
    let t = &app.theme;
    let (x, y) = at(&buf, "✕ fix-footer");
    assert_eq!(buf[(x, y)].fg, t.err, "the row's glyph");
    assert_eq!(buf[(x - 2, y)].fg, t.accent, "the open row's bar is the accent, as any selected row's");
    let (x, y) = at(&buf, "✕ failed   exit 1");
    assert_eq!(buf[(x, y)].bg, t.err, "the sheet's result chip");
    let (x, y) = at(&buf, "✕ failed  ✕");
    assert_eq!(buf[(x, y)].fg, t.err, "the pane's mark");
}

#[test]
fn a_modal_title_is_on_the_accent() {
    for id in ["12", "A2"] {
        let mut app = open(id);
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let title = if id == "12" { "OVERRIDE A FAILED CHECK" } else { "A REPO WANTS TO RUN A COMMAND" };
        let (x, y) = at(&buf, title);
        assert_eq!(buf[(x, y)].bg, app.theme.accent, "{id}");
    }
}

/// Buttons have square ends by default, as the reference draws them.
/// Turned on, they only put a cap where a square end had a blank of the button's colour.
#[test]
fn pill_caps_only_round_the_buttons_ends() {
    for id in ["12", "A2"] {
        let mut square = open(id);
        let flat = cells(&mut square, Layout::Floating, 160, 45, "unicode");
        let mut round = open(id);
        round.cfg.ui.rounded = crate::config::Rounding::Full;
        let capped = cells(&mut round, Layout::Floating, 160, 45, "unicode");
        let caps = [CAP_L, CAP_R];
        let mut changed = 0;
        let width = flat.area.width as usize;
        for (n, (a, b)) in flat.content().iter().zip(capped.content()).enumerate() {
            let title_row = n / width == 0;
            // Round tabs reflow the title row and a round session row gives up its last cell to its
            // cap; this test is about the buttons, which only gain caps where a blank was.
            if a.symbol() != b.symbol() && !title_row && caps.contains(&b.symbol()) {
                changed += 1;
            }
        }
        assert!(changed > 0, "{id}: the buttons have caps");
    }
}

/// Flat honours `ui.rounded` as Cards does: off, no cap is drawn; on, caps appear and each
/// one sits on the ground of the cell beside it (the zone's, not the theme's `bg`), so an
/// end shows no notch of another colour.
#[test]
fn flat_rounds_its_pill_ends_on_the_ground_around_them() {
    let caps = [CAP_L, CAP_R];
    let mut total = 0;
    for id in ["01", "04", "11", "12", "A2", "C1"] {
        let mut square = open(id);
        let plain = cells(&mut square, Layout::Flat, 160, 45, "unicode");
        assert!(!plain.content().iter().any(|c| caps.contains(&c.symbol())), "{id}: pill_caps off draws no cap");

        let mut round = open(id);
        round.cfg.ui.rounded = crate::config::Rounding::Full;
        let buf = cells(&mut round, Layout::Flat, 160, 45, "unicode");
        let area = buf.area;
        let mut seen = 0;
        for y in area.top()..area.bottom() {
            for x in area.left()..area.right() {
                let cell = &buf[(x, y)];
                let outside = match cell.symbol() {
                    s if s == CAP_L => x.checked_sub(1),
                    s if s == CAP_R => Some(x + 1).filter(|n| *n < area.right()),
                    _ => continue,
                };
                seen += 1;
                // A session row's cap stands on the sidebar's own ground, which can be the terminal's.
                assert!(cell.bg != ratatui::style::Color::Reset || cell.fg != ratatui::style::Color::Reset, "{id}: the cap at {x},{y} has a ground");
                // A tab chip on a title of its own colour has a cap that reads as a blank.
                assert!(y == 0 || cell.bg != cell.fg, "{id}: the cap at {x},{y} is invisible");
                if let Some(o) = outside {
                    let beside = &buf[(o, y)];
                    if !caps.contains(&beside.symbol()) && beside.symbol() == " " {
                        assert_eq!(cell.bg, beside.bg, "{id}: the cap at {x},{y} sits on another colour than the cell beside it");
                    }
                }
            }
        }
        total += seen;
    }
    assert!(total > 0, "Flat's buttons have round ends with pill_caps on");
}

/// Flat has no card edges, so rounded corners reach only a modal there: its box, all four.
#[test]
fn flat_corners_round_only_a_modal() {
    let mut app = open("12");
    let square = frame(&mut app, Layout::Flat, 160, 45, "unicode");
    assert!(!square.contains(['╭', '╮', '╰', '╯']), "square is the default\n{square}");
    app.cfg.ui.rounded = crate::config::Rounding::Full;
    let rounded = frame(&mut app, Layout::Flat, 160, 45, "unicode");
    for corner in ['╭', '╮', '╰', '╯'] {
        assert_eq!(rounded.matches(corner).count(), 1, "the modal's {corner}, and nothing else is a box\n{rounded}");
    }
}

fn press(app: &mut App, code: crossterm::event::KeyCode) {
    app.on_key(crossterm::event::KeyEvent::new(code, crossterm::event::KeyModifiers::NONE));
}

/// The reference keeps the Inbox under a modal opened from it; Esc goes back to
/// it, on the same item.
#[test]
fn esc_from_a_modal_opened_in_the_inbox_goes_back_to_the_inbox() {
    use crossterm::event::KeyCode;
    for (id, opens) in [("11", KeyCode::Char('O')), ("A1", KeyCode::Enter)] {
        let mut app = open(id);
        let Mode::GoTo { sel: before, .. } = app.mode else { panic!("{id}: the Inbox is open") };
        press(&mut app, opens);
        assert!(matches!(app.mode, Mode::Confirm(_) | Mode::Approve(_)), "{id}: a modal: {:?}", app.mode);
        assert!(frame(&mut app, Layout::Floating, 160, 45, "unicode").contains("INBOX"), "{id}: the Inbox stays under it");
        press(&mut app, KeyCode::Esc);
        assert!(matches!(app.mode, Mode::GoTo { sel, .. } if sel == before), "{id}: back on its item: {:?}", app.mode);
        assert!(frame(&mut app, Layout::Floating, 160, 45, "unicode").contains("INBOX"), "{id}");
        press(&mut app, KeyCode::Esc);
        assert!(!matches!(app.mode, Mode::GoTo { .. }), "{id}: and Esc closes the Inbox after");
    }
}

#[test]
fn finished_lists_the_session_with_a_check_result_first_then_the_newest() {
    for stage in [Stage::Stale, Stage::Over, Stage::Never, Stage::Passed] {
        let app = world(stage);
        let done: Vec<TermId> = goto_rows(&app.screen_model(), "", 0).iter().filter_map(|r| if let GoRow::Done(t) = r { Some(*t) } else { None }).collect();
        assert_eq!(done, [1, 4], "{stage:?}: fix-footer (2m, checked) before quick-fox (9h)");
    }
}

#[test]
fn not_now_keeps_its_last_cell_beside_an_armed_always() {
    let mut app = open("A3");
    let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
    let (x, y) = at(&buf, "Not now Esc");
    assert_eq!(x + 11, 120, "where the reference puts its end");
    assert_eq!(buf[(120, y)].bg, app.theme.btn, "the button's trailing space is drawn");
}

/// The reference makes the whole inside of a pane's card its focus region, padding and all; the
/// terminal inside it still gets its own clicks.
#[test]
fn a_click_anywhere_inside_a_panes_card_is_the_pane() {
    for (id, layout) in [("01", Layout::Floating), ("07", Layout::Floating), ("07", Layout::Tiled), ("01", Layout::Flat)] {
        let mut app = open(id);
        app.mode = Mode::Normal;
        let _ = cells(&mut app, layout, 160, 45, "unicode");
        let (_, card) = app.pane_frames[0];
        let (_, term) = app.panes[0];
        let top = |x: u16, y: u16| app.hits.iter().rev().find(|(r, _)| r.contains(ratatui::layout::Position { x, y })).map(|(_, h)| *h);
        for (x, y) in [(card.x + 1, card.y + 1), (term.x - 1, term.y), (card.right() - 2, card.bottom() - 2), (term.x, term.y)] {
            assert_eq!(top(x, y), Some(Hit::Pane(1)), "{id} {layout:?}: ({x},{y})");
        }
    }
}

/// After an override the evidence sheet shows the evidence the override stands on.
#[test]
fn overriding_from_the_inbox_opens_the_evidence() {
    use crossterm::event::KeyCode;
    let mut app = open("11");
    press(&mut app, KeyCode::Char('O'));
    press(&mut app, KeyCode::Enter);
    assert!(matches!(&app.view, Some(View::Check(v)) if v.term == 1), "the evidence sheet");
    assert!(frame(&mut app, Layout::Floating, 160, 45, "unicode").contains("CHECK · fix-footer › test"));
}

/// A working row speaks in the working colour: its spinner and its name, in every theme.
#[test]
fn a_working_row_s_mark_and_name_are_the_working_colour() {
    for &name in crate::theme::BUILTIN {
        let mut app = open("01");
        app.theme = crate::theme::Theme::named(name);
        let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
        let work = app.theme.accent;
        let (x, y) = at(&buf, "add-payments");
        assert_eq!(buf[(x - 2, y)].fg, work, "{name}: the spinner");
        assert_eq!(buf[(x, y)].fg, work, "{name}: the name (motion off holds it still)");
    }
}

/// Needs you and failed are two colours in the app-dark, as in the reference (`needs`, `err`):
/// the Inbox's group chips, the row's glyph and question, and the failed result.
#[test]
fn needs_you_is_amber_and_failed_red_in_app_dark() {
    let mut app = open("11");
    app.theme = crate::theme::Theme::named("night");
    let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
    let t = &app.theme;
    assert_ne!(t.blocked, t.err);
    let (x, y) = at(&buf, "NEEDS YOU");
    assert_eq!(buf[(x, y)].bg, t.blocked, "the Inbox's needs-you chip");
    let (x, y) = at(&buf, "FAILED CHECKS");
    assert_eq!(buf[(x, y)].bg, t.err, "the Inbox's failed chip");
    let (x, y) = at(&buf, "warm-finch");
    assert_eq!(buf[(x - 2, y)].fg, t.blocked, "the asking row's glyph");
    let (x, y) = at(&buf, "asks:");
    assert_eq!(buf[(x, y)].fg, t.muted, "the question is quiet: the glyph says it needs you");
}

/// A result that could not run is violet, apart from failed red, needs-you amber and working.
#[test]
fn a_check_that_could_not_run_is_violet_not_red_or_amber() {
    let mut app = open("06");
    app.theme = crate::theme::Theme::named("night");
    let buf = cells(&mut app, Layout::Floating, 160, 45, "unicode");
    let t = &app.theme;
    let (x, y) = at(&buf, "fix-footer        ");
    assert_eq!(buf[(x - 2, y)].fg, t.violet(), "the row's glyph");
    assert!(![t.err, t.blocked, t.accent].contains(&t.violet()));
}

#[test]
/// Backspace in the Inbox edits its search and never closes it or the app, whatever it holds.
fn backspace_in_the_inbox_only_edits_its_search() {
    for stage in reference_fixture::STAGES {
        for (w, h) in reference_fixture::SIZES {
            for keys in ["", "a", "zz"] {
                let mut app = reference_fixture::world(stage);
                app.cfg.ui.padding = "1".into();
                app.open_goto();
                let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
                for c in keys.chars() {
                    app.on_key(crate::client::KeyEvent::new(crate::client::KeyCode::Char(c), crate::client::KeyModifiers::NONE));
                }
                for _ in 0..3 {
                    term.draw(|f| crate::client::render::draw(&mut app, f)).unwrap();
                    app.on_key(crate::client::KeyEvent::new(crate::client::KeyCode::Backspace, crate::client::KeyModifiers::NONE));
                    assert!(app.quit.is_none(), "{stage:?}");
                }
                term.draw(|f| crate::client::render::draw(&mut app, f)).unwrap();
            }
        }
    }
}

