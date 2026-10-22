//! Every v7 preset drawn whole on the fixture's world and set beside the handoff's reference
//! frames (`v7_frames/<preset>.txt`, cut from `docs/design/v7/render-frames.mjs`; see
//! docs/TESTING.md), in Cards floating, Cards tiled and Flat at 160×45, 100×30 and 80×24, and
//! in ASCII at Flat 80×24.
//!
//! Cells the reference leaves to the app (masks):
//! - the pane's terminal text: the agent's own output;
//! - the working spinner's cell (the user's spinner, not the handoff's 3-bar one) and ages the
//!   fixture can't pin to the sample (the working session reads `0s`, not `now`; the sample's
//!   approval item is a minute younger than its own row);
//! - the handoff's sample checkpoint kinds (commits, approval, review) become the fixture's
//!   build and test.
//!
//! Where the sample cannot be the app's, the reference is changed to what the app draws, and
//! each change is a deliberate difference:
//! - a folder with a session that needs you is listed first (the app's order);
//! - quick-fox is finished and unseen, so it reads "finished · not seen", as the sample's Inbox
//!   says (its sidebar says "finished");
//! - a failed row's third line is its first failing line, the Inbox's sample line; checks7.js
//!   sets "footer shows current year" outside the failed branch, so other rows drop it;
//! - add-payments is a session of its own, not a second tab of fix-footer's pane, and the
//!   pane's mark keeps its room where checks7.js draws the tab chips over it;
//! - a row waiting on a changed command (A4) shows the new command; a project with a check
//!   that never ran (16) names it in the folder's meta;
//! - a command you said never to (A5) reads "did not run" and its source names the file's
//!   commit (the result carries no time for your choice);
//! - Why (13) does not add "1 of 59 tests failed": the app does not parse a command's output;
//! - the C2 Inbox's footnote is an annotation, not a screen line;
//! - pill caps (a Nerd Font option, on by default) are off, so buttons have square ends.

use super::screen::{GoRow, goto_rows};
use super::v7_fixture::{Layout, Stage, frame, world};
use super::*;
use crate::protocol::{CheckSource, CheckState};

/// A cell the reference leaves to the app.
const ANY: char = '\u{1}';
/// The handoff's working spinner with motion off: its third 3-bar frame, and ASCII's.
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
        "01" => include_str!("v7_frames/01.txt"),
        "02" => include_str!("v7_frames/02.txt"),
        "03" => include_str!("v7_frames/03.txt"),
        "04" => include_str!("v7_frames/04.txt"),
        "05" => include_str!("v7_frames/05.txt"),
        "06" => include_str!("v7_frames/06.txt"),
        "07" => include_str!("v7_frames/07.txt"),
        "08" => include_str!("v7_frames/08.txt"),
        "09" => include_str!("v7_frames/09.txt"),
        "10" => include_str!("v7_frames/10.txt"),
        "11" => include_str!("v7_frames/11.txt"),
        "12" => include_str!("v7_frames/12.txt"),
        "13" => include_str!("v7_frames/13.txt"),
        "14" => include_str!("v7_frames/14.txt"),
        "15" => include_str!("v7_frames/15.txt"),
        "16" => include_str!("v7_frames/16.txt"),
        "17" => include_str!("v7_frames/17.txt"),
        "A1" => include_str!("v7_frames/A1.txt"),
        "A2" => include_str!("v7_frames/A2.txt"),
        "A3" => include_str!("v7_frames/A3.txt"),
        "A4" => include_str!("v7_frames/A4.txt"),
        "A5" => include_str!("v7_frames/A5.txt"),
        "B1" => include_str!("v7_frames/B1.txt"),
        "B2" => include_str!("v7_frames/B2.txt"),
        "C1" => include_str!("v7_frames/C1.txt"),
        "C2" => include_str!("v7_frames/C2.txt"),
        _ => panic!("no frames for {id}"),
    }
}

// ---- the app's side: the fixture's world with the preset open --------------------------------

fn key(app: &mut App, c: char) {
    app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
}

fn with_check(app: &mut App, command: &str) {
    let site = app.snap.terms[&1].root.clone().expect("fix-footer is in a repo");
    app.cfg.check = vec![crate::config::CheckDef { project: site, name: None, command: command.into(), ..Default::default() }];
}

/// The sample's words where the fixture has none of its own.
fn sample_words(app: &mut App) {
    app.cfg.ui.pill_caps = false;
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
            check.source = CheckSource::Repo { file: ".seshi/checks.toml".into(), commit: Some("9f3c2a1".into()), committed_at: Some(super::v7_fixture::now() - 2 * HOUR), uncommitted: false };
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
                app.ask_override(1);
                app.on_paste("flaky legal test, tracked in #212".into());
            }
        }
        Over::Inbox => select_in_inbox(&mut app, if id == "C2" { 3 } else { 1 }),
        Over::Why => app.mode = Mode::Why { term: 1, side: true },
        Over::Settings => app.open_project_check(1),
        Over::Approve => {
            app.open_approval(1);
            if id == "A3" {
                key(&mut app, '2');
            }
        }
    }
    app
}

// ---- the reference's side: where things are, and what the app may draw its own way ------------

/// The handoff's grid for a `w`-wide frame (`render` in checks7.js).
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
    // A modal or popover, and its hard shadow a cell right and down.
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
        // checks7.js keeps the row's old command after the repo changed it.
        if id == "A4" {
            swap_tail(row, "approve check: npm test", "approve check: npm test && node scripts/seed.js", 0, end);
        }
        for sp in SPINNER {
            if let Some(i) = find(row, &format!("{sp} add-payments")) {
                let to: Vec<char> = format!("{ANY} add-payments ").chars().collect();
                splice(row, i, sp.chars().count() + " add-payments".chars().count(), &to, end);
            }
        }
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
    // quick-fox: finished and unseen.
    if let Some(i) = rows.iter().position(|r| text(r).contains("quick-fox"))
        && rows.get(i + 1).is_some_and(|r| text(r).trim() == "finished")
    {
        swap(&mut rows[i + 1], "finished", "finished · not seen", 0, end);
    }
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
    // 16's project has a check that never ran; checks7.js names it only once there is a result.
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
    // The sample's second tab is add-payments, a session of its own in the app (the sidebar
    // lists it); fix-footer has one tab.
    let flat = b.layout == Layout::Flat;
    let top = &mut want[0];
    let w = top.len();
    swap(top, " 2  add-payments ", &(if flat { " " } else { "─" }).repeat(17), 0, w);
    // checks7.js draws the tab chips over the pane's mark when they meet; the app gives the
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
        for row in want.iter_mut().take(s.bottom() as usize).skip(s.y as usize) {
            swap(row, QUESTION, "use pnpm or npm?", x0, x1);
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
                swap_tail(row, ".seshi/checks.toml · you chose never", ".seshi/checks.toml · a1b2c3d · you chose never", x0, x1);
            }
        }
    }
    // The sample's "1 of 59 tests failed" is read from vitest's output; the app does not parse
    // a command's output.
    if let Some(pop) = g.overlays.first().filter(|_| preset(id).1 == Over::Why) {
        // Its text stops a column before its right edge.
        let (x0, x1) = (pop.x as usize, pop.right() as usize - 1);
        for row in want.iter_mut().take(pop.bottom() as usize).skip(pop.y as usize) {
            swap_tail(row, " · 1 of 59", "", x0, x1);
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
    assert!(report.is_empty(), "{id}: the frames differ from the v7 reference\n{report}");
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
