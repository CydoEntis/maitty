//! The reference sample world (its sessions and their check evidence) as an `App`, for each
//! of its check stages, and a frame of it as text to set beside the reference frames.
//!
//! - affiliate-site ⎇ main: fix-footer (codex, 2m), whose check each stage changes, and
//!   add-payments (claude, working)
//! - pixls (no git): warm-finch asking "use pnpm or npm?"
//! - orbit ⎇ dev: quick-fox done 9h ago, old-moth asleep for 2d
//!
//! The clock is pinned (`NOW`, 14:24 local) so ages and clock times read the same every run.

use super::screen::FIXED_NOW;
use super::*;
use crate::layout::Node;
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use std::path::Path;

/// The reference's `stage`: what fix-footer's check came to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Stage {
    None,
    Checking,
    Passed,
    Failed,
    Over,
    Couldnt,
    Approve,
    Never,
    Stale,
    /// Several checkpoints: build and test.
    Multi,
}

pub(super) const STAGES: [Stage; 10] =
    [Stage::None, Stage::Checking, Stage::Passed, Stage::Failed, Stage::Over, Stage::Couldnt, Stage::Approve, Stage::Never, Stage::Stale, Stage::Multi];

/// The reference's three frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Layout {
    Floating,
    Tiled,
    Flat,
}

pub(super) const LAYOUTS: [Layout; 3] = [Layout::Floating, Layout::Tiled, Layout::Flat];

/// The reference's frame sizes, widest first.
pub(super) const SIZES: [(u16, u16); 3] = [(160, 45), (100, 30), (80, 24)];

impl Layout {
    /// `[ui] layout` and `[ui] panes`.
    fn config(self) -> (&'static str, &'static str) {
        match self {
            Layout::Floating => ("cards", "floating"),
            Layout::Tiled => ("cards", "tiled"),
            Layout::Flat => ("flat", "tiled"),
        }
    }
}

/// 2026-10-22 00:00 UTC, the reference's day.
const DAY: u64 = 1_792_627_200;
const HOUR: u64 = 3600;

/// The unix time that reads `h:m:s` on this machine's clock on the reference's day.
pub(super) fn at(h: u64, m: u64, s: u64) -> u64 {
    let want = (h * HOUR + m * 60 + s) as i64;
    let guess = DAY + want as u64;
    let shown: Vec<i64> = crate::clock::local_hms(guess).split(':').map(|p| p.parse().unwrap()).collect();
    let got = shown[0] * HOUR as i64 + shown[1] * 60 + shown[2];
    let day = 24 * HOUR as i64;
    let diff = (want - got + day / 2).rem_euclid(day) - day / 2;
    (guess as i64 + diff) as u64
}

/// The pinned now: 14:24, two minutes after fix-footer's turn ended.
pub(super) fn now() -> u64 {
    at(14, 24, 0)
}

fn home() -> PathBuf {
    directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf()).unwrap_or_else(std::env::temp_dir).join("dev")
}

pub(super) fn term(id: TermId, label: &str, agent: &str, status: Status, cwd: &Path, since: u64) -> TermInfo {
    TermInfo {
        id,
        cols: 80,
        rows: 20,
        title: String::new(),
        process: "node".into(),
        remote: None,
        popup: false,
        agent: Some(agent.into()),
        status,
        cwd: cwd.to_path_buf(),
        summary: String::new(),
        name: String::new(),
        label: label.into(),
        model: String::new(),
        mem: 0,
        cpu: 0.0,
        bell: false,
        said: String::new(),
        branch: None,
        linked: false,
        subagents: Vec::new(),
        subagent_activity: Vec::new(),
        session: None,
        started_by: None,
            helper_ask: Vec::new(),
        queued: Vec::new(),
        root: None,
        top: None,
        since,
        status_why: String::new(),
        why_hook: String::new(),
        why_hook_at: 0,
        why_screen: String::new(),
        why_screen_at: 0,
        why_process_at: 0,
        quiet_since: None,
        pid: 0,
        asleep: false,
        win32_input: false,
        usage: Default::default(),
        resume_at: None,
        checks: Vec::new(),
    }
}

/// A pane in a checkout of `root` on `branch`.
pub(super) fn in_repo(mut t: TermInfo, root: &Path, branch: &str) -> TermInfo {
    t.linked = t.cwd != root;
    (t.root, t.top, t.branch) = (Some(root.to_path_buf()), Some(t.cwd.clone()), Some(branch.into()));
    t
}

pub(super) fn entry(path: &Path, branch: &str, main: bool, repo: &str) -> WorktreeEntry {
    WorktreeEntry { path: path.to_path_buf(), branch: branch.into(), main, repo: repo.into() }
}

pub(super) fn workspace(id: WsId, name: &str, cwd: &Path, terms: &[TermId], git: Option<GitInfo>, color: u8) -> WorkspaceInfo {
    let tabs: Vec<TabInfo> = terms.iter().enumerate().map(|(i, &t)| TabInfo { id: id + 1 + i as TabId, name: String::new(), layout: Node::Leaf(t), focus: t }).collect();
    let active_tab = tabs[0].id;
    WorkspaceInfo { id, name: name.into(), cwd: cwd.to_path_buf(), tabs, active_tab, git, worktree: false, color, is_new: false, group: None }
}

pub(super) fn git(root: &Path, branch: &str, worktrees: Vec<WorktreeEntry>) -> GitInfo {
    let repo = super::screen::folder_name(root);
    GitInfo { repo, branch: branch.into(), dirty: 0, linked: false, root: root.to_path_buf(), worktrees, ahead: 0 }
}

const OUT_FAIL_LINES: u64 = 13;
const OUT_PASS_LINES: u64 = 3;
const FIRST_FAILING: &str = "✗ src/pages/legal.test.tsx > footer shows current year";

/// The repo file a repo-supplied command comes from, added three days ago.
fn from_repo() -> CheckSource {
    CheckSource::Repo { file: concat!(".", env!("APP_COMMAND"), "/checks.toml").into(), commit: Some("a1b2c3d".into()), committed_at: Some(now() - 3 * 24 * HOUR), uncommitted: false }
}

/// fix-footer's checkpoints at `stage`, as the evidence sheet reads them.
fn fix_footer_checks(stage: Stage, cwd: &Path) -> Vec<CheckInfo> {
    let ran = CheckInfo {
        name: "test".into(),
        command: "npm test".into(),
        cwd: cwd.to_path_buf(),
        turn: 3,
        started_at: at(14, 22, 5),
        duration_ms: 14_200,
        after_turn_ended_at: at(14, 21, 51),
        ..Default::default()
    };
    let passed = CheckInfo { state: CheckState::Passed, exit_code: Some(0), line_count: OUT_PASS_LINES, ..ran.clone() };
    let failed = CheckInfo { state: CheckState::Failed, exit_code: Some(1), line_count: OUT_FAIL_LINES, first_failing_line: FIRST_FAILING.into(), ..ran.clone() };
    let unrun = |state| CheckInfo { state, started_at: 0, duration_ms: 0, source: from_repo(), ..ran.clone() };
    match stage {
        Stage::None => vec![],
        Stage::Checking => vec![CheckInfo { state: CheckState::Checking, started_at: now() - 6, duration_ms: 0, ..ran }],
        Stage::Passed => vec![passed],
        Stage::Failed => vec![failed],
        Stage::Over => vec![CheckInfo {
            state: CheckState::Overridden,
            overridden_at: Some(at(14, 30, 0)),
            overridden_by: "you".into(),
            override_reason: "flaky legal test, tracked in #212".into(),
            ..failed
        }],
        Stage::Couldnt => vec![CheckInfo { state: CheckState::CouldNotRun, command: "pnpm test".into(), exit_code: Some(127), duration_ms: 100, line_count: 1, ..ran }],
        Stage::Approve => vec![unrun(CheckState::NeedsApproval)],
        Stage::Never => vec![unrun(CheckState::Never)],
        Stage::Stale => vec![CheckInfo {
            state: CheckState::Stale,
            stale_since: Some(at(14, 24, 0)),
            changed_files: [('M', "src/components/Footer.tsx"), ('M', "src/pages/legal.tsx"), ('A', "tests/legal.test.tsx")]
                .map(|(kind, path)| ChangedFile { kind, path: path.into() })
                .into(),
            ..passed
        }],
        Stage::Multi => vec![build_passed(&ran), failed],
    }
}

fn build_passed(ran: &CheckInfo) -> CheckInfo {
    CheckInfo { name: "build".into(), command: "npm run build".into(), state: CheckState::Passed, exit_code: Some(0), line_count: OUT_PASS_LINES, ..ran.clone() }
}

/// fix-footer's screen: codex's own transcript of the turn.
fn fix_footer_screen() -> vt100::Parser {
    let mut p = vt100::Parser::new(20, 80, 0);
    let lines = [
        "\x1b[2m› \x1b[0;1mrename the footer copyright to 2026\x1b[0m",
        "",
        "\x1b[32m• \x1b[0;1mEdit \x1b[0;36msrc/components/Footer.tsx\x1b[0m",
        "\x1b[31m  - © 2025 Affiliate Co.\x1b[0m",
        "\x1b[32m  + © 2026 Affiliate Co.\x1b[0m",
        "",
        "\x1b[32m• \x1b[0mRan \x1b[34mnpm test -- Footer\x1b[0m",
        "\x1b[2m  └ 3 passed\x1b[0m",
        "",
        "\x1b[32m• \x1b[0mDone. Footer updated and its tests pass.",
        "",
        "\x1b[2mWorked for 48s\x1b[0m",
        "",
        "› \x1b[2mAsk Codex to do anything\x1b[0m",
    ];
    p.process(lines.join("\r\n").as_bytes());
    p
}

/// The reference sample world with fix-footer's check at `stage`. Pins this thread's clock.
pub(super) fn world(stage: Stage) -> App {
    FIXED_NOW.with(|n| n.set(Some(now())));
    let (tx, _rx) = mpsc::unbounded_channel();
    let (bg_tx, _bg_rx) = mpsc::unbounded_channel();
    // The reference has no padding round the screen; its frames are compared cell for cell.
    let cfg = Config { ui: crate::config::Ui { padding: "0".into(), ..Default::default() }, ..Default::default() };
    let mut app = App::new(cfg, tx, None, bg_tx);
    app.splash = false;
    let dev = home();
    let (site, pixls, orbit) = (dev.join("affiliate-site"), dev.join("pixls"), dev.join("orbit"));
    let trees = dev.join("affiliate-site-worktrees");
    let (footer_dir, payments_dir) = (trees.join("fix-footer"), trees.join("add-payments"));

    let mut footer = in_repo(term(1, "fix-footer", "codex", Status::Done, &footer_dir, at(14, 21, 51)), &site, "fix-footer");
    footer.checks = fix_footer_checks(stage, &footer_dir);
    let mut payments = in_repo(term(2, "add-payments", "claude", Status::Working, &payments_dir, now()), &site, "add-payments");
    if stage == Stage::Multi {
        let base = &footer.checks[0];
        let approve = CheckInfo { name: "test".into(), state: CheckState::NeedsApproval, started_at: 0, duration_ms: 0, exit_code: None, line_count: 0, source: from_repo(), ..base.clone() };
        (payments.status, payments.since, payments.checks) = (Status::Done, now() - 40, vec![build_passed(base), approve]);
    }
    let finch = term(3, "warm-finch", "codex", Status::Blocked, &pixls, now() - 3 * 60);
    let fox = in_repo(term(4, "quick-fox", "claude", Status::Done, &orbit, now() - 9 * HOUR), &orbit, "dev");
    let mut moth = in_repo(term(5, "old-moth", "codex", Status::Idle, &orbit, now() - 2 * 24 * HOUR), &orbit, "dev");
    moth.asleep = true;

    let site_trees = vec![entry(&site, "main", true, "affiliate-site"), entry(&footer_dir, "fix-footer", false, "affiliate-site"), entry(&payments_dir, "add-payments", false, "affiliate-site")];
    app.snap.workspaces = vec![
        workspace(10, "affiliate-site", &site, &[1, 2], Some(git(&site, "main", site_trees)), 0),
        workspace(20, "pixls", &pixls, &[3], None, 1),
        workspace(30, "orbit", &orbit, &[4, 5], Some(git(&orbit, "dev", vec![entry(&orbit, "dev", true, "orbit")])), 2),
    ];
    app.snap.active_ws = Some(10);
    app.snap.questions = vec![HumanQuestion { id: 1, term: 3, text: "use pnpm or npm?".into(), options: vec!["pnpm".into(), "npm".into()] }];
    for t in [footer, payments, finch, fox, moth] {
        app.parsers.insert(t.id, if t.id == 1 { fix_footer_screen() } else { vt100::Parser::new(20, 80, 0) });
        app.snap.terms.insert(t.id, t);
    }
    app.screen_sync();
    app
}

/// The world drawn in `layout` at `w`×`h`, every cell with its colours. The reference has no mode
/// bar (ADR 0024), which takes the bottom row: the world is drawn a row taller and that row, the
/// bar's and nothing else, is left out, so the screen above it is laid out as at `w`×`h`.
pub(super) fn cells(app: &mut App, layout: Layout, w: u16, h: u16, glyphs: &str) -> ratatui::buffer::Buffer {
    FIXED_NOW.with(|n| n.set(Some(now())));
    let (l, p) = layout.config();
    (app.cfg.ui.layout, app.cfg.ui.panes, app.cfg.ui.glyphs) = (l.into(), p.into(), glyphs.into());
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h + 1)).expect("a test terminal");
    term.draw(|f| render::draw(app, f)).expect("a frame");
    let full = term.backend().buffer();
    assert_eq!(super::screen::mode_bar_row(app, full.area).y, h, "the bar is the row left out");
    let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, w, h));
    for (x, y) in (0..h).flat_map(|y| (0..w).map(move |x| (x, y))) {
        buf[(x, y)] = full[(x, y)].clone();
    }
    buf
}

/// `app` drawn in `layout` at `w`×`h` with the `glyphs` set (`unicode`, `ascii`, `nerd`), as
/// text: one line per row, trailing spaces trimmed. `<APP>_SHOW=1` prints it under a heading
/// like the reference frames.
/// Row 0 as the reference draws it: its tabs end without the `✕ ` the app gives each, and its frames
/// have no `+` chip after them (the app keeps one a cell clear of them, so a tab opens with a
/// click). What the app adds
/// reads as the title's own fill, so everything after it keeps its column.
fn without_new_tab_chip(row: &str) -> String {
    let chars: Vec<char> = row.chars().collect();
    let mut out = String::new();
    let (mut cut, mut cut_at) = (0, 0);
    let mut i = 0;
    while i < chars.len() {
        let tab_close = chars[i] == '✕'
            && i >= 2
            && chars[i - 1] == ' '
            && chars[i - 2].is_alphanumeric()
            && chars.get(i + 1) == Some(&' ')
            && !chars[i + 1..].iter().find(|c| **c != ' ').is_some_and(|c| "┐╮│".contains(*c));
        if tab_close {
            cut += 2;
            cut_at = out.len();
            i += 2;
        } else {
            out.push(chars[i]);
            i += 1;
        }
    }
    // The `+` chip and the cell the app keeps clear before it.
    const CHIP: &str = "  + ";
    // The chip is followed by the title's fill; ASCII's done mark (`+ done`) is not.
    let Some(at) = out.match_indices(CHIP).map(|(i, _)| i).find(|&i| out[i + CHIP.len()..].starts_with(['─', ' '])) else {
        let fill = out[cut_at..].chars().next().unwrap_or(' ');
        out.insert_str(cut_at, &fill.to_string().repeat(cut));
        return out;
    };
    let rest = &out[at + CHIP.len()..];
    let fill = rest.chars().next().unwrap_or(' ');
    format!("{}{}{rest}", &out[..at], fill.to_string().repeat(CHIP.len() + cut))
}

/// The sidebar as the reference frames draw it, from what v8 draws (ADR 0014). `rows` is the
/// whole screen; the sidebar's rectangle and its rows' places come from `app`. What v8 changes
/// on purpose, put back:
/// - the foot sits a row higher in Flat and has a divider above it; its `new` and `menu` chips
///   read `+ new` and `a actions` (the app's `+ New` and `Menu`), the reference's words at the same edges;
///
/// A second line that only names the state is gone from the app; `as_one_line_rows` drops it
/// from the reference.
fn as_the_reference_sidebar(rows: &mut [Vec<char>], buf: &ratatui::buffer::Buffer, app: &App, flat: bool) {
    let side = app.screen.side_rect;
    if side.width < 4 || side.height < 6 {
        return;
    }
    let (left, right) = (side.x as usize + 1, side.right() as usize - 1);
    let foot_y = side.bottom() as usize - 2;
    // The foot: the divider over it goes (only its own run of `─`, a modal may cross that row).
    let divider_from = left + usize::from(!flat);
    for x in divider_from..right {
        // A modal's bottom border can lie on this row: it is on the modal's ground, not the sidebar's.
        let modal = [app.theme.card, app.theme.card2].contains(&buf[(x as u16, (foot_y - 1) as u16)].bg);
        if rows[foot_y - 1][x] != '─' || modal {
            break;
        }
        rows[foot_y - 1][x] = ' ';
    }
    let mut foot: Vec<char> = rows[foot_y][left..right].to_vec();
    let chars = |s: &str| s.chars().collect::<Vec<char>>();
    let find = |row: &[char], needle: &str| row.windows(needle.chars().count()).position(|w| w == chars(needle).as_slice());
    // A chip is its word with a cell either side (a space, or a cap with `ui.rounded`).
    if let Some(at) = find(&foot, "+ New").filter(|at| *at > 0) {
        foot.splice(at - 1..at + 6, chars("+ new  "));
    }
    if let Some(at) = find(&foot, "Menu").filter(|at| *at > 0) {
        let end = at + 5;
        foot.splice(at - 1..end, vec![' '; 6]);
        let start = (end + 1).saturating_sub(9).min(foot.len() - 9);
        foot.splice(start..start + 9, chars("a actions"));
    }
    // Lifted a row, the foot can sit under a modal that ends where the reference's foot is.
    if flat && find(&foot, "+ new").is_none() {
        let (nx, mend) = (side.x as usize + 2 - left, right - left);
        let mut text = vec![' '; mend];
        text[nx..nx + 5].copy_from_slice(&chars("+ new"));
        text[mend - 9..].copy_from_slice(&chars("a actions"));
        foot = text;
    }
    foot.truncate(right - left);
    rows[foot_y][left..right].fill(' ');
    let to = if flat { foot_y + 1 } else { foot_y };
    rows[to][left..right].copy_from_slice(&foot);
}

/// A project row's `−` / `+` button, in the state bars' column, read as the caret the reference
/// draws a column further in.
fn with_the_reference_carets(rows: &mut [Vec<char>], app: &App, glyphs: &str) {
    let (open, closed) = if glyphs == "ascii" { ('v', '>') } else { ('▾', '›') };
    for (r, h) in &app.hits {
        if !matches!(h, Hit::Screen(super::screen::ScreenHit::ToggleProj(_))) {
            continue;
        }
        let (x, row) = (r.x as usize, &mut rows[r.y as usize]);
        if row.get(x + 1) == Some(&' ') && (row[x] == '−' || row[x] == '+') {
            row[x + 1] = if row[x] == '−' { open } else { closed };
            row[x] = ' ';
        }
    }
}

/// The app bars only the selected row; the reference also bars a failed check's row in red.
fn with_the_reference_failed_bars(rows: &mut [Vec<char>], app: &App, flat: bool, glyphs: &str) {
    let side = app.screen.side_rect;
    let bar_x = (side.x + if flat { 1 } else { 2 }) as usize;
    let bar = if glyphs == "ascii" { '|' } else { '▎' };
    let folders: Vec<usize> = app.hits.iter().filter(|(_, h)| matches!(h, Hit::Screen(super::screen::ScreenHit::ToggleProj(_)))).map(|(r, _)| r.y as usize).collect();
    let failed = |term: &crate::protocol::TermId| app.snap.terms.get(term).is_some_and(|i| i.reported_check().is_some_and(|c| c.state == crate::protocol::CheckState::Failed));
    let mut owners: Vec<(usize, crate::protocol::TermId)> = app.screen.row_y.iter().map(|(t, y)| (*y as usize, *t)).collect();
    owners.sort();
    for (y, row) in rows.iter_mut().enumerate().take((side.bottom() as usize).saturating_sub(2)) {
        // A line belongs to the session row that began at most two lines above it.
        let owner = owners.iter().rev().find(|(oy, _)| *oy <= y && y - oy <= 2).map(|(_, t)| t);
        let has_text = row.iter().skip(bar_x + 1).take(side.width as usize).any(|c| *c != ' ' && *c != '│');
        if owner.is_some_and(failed) && !folders.contains(&y) && has_text && row.get(bar_x) == Some(&' ') {
            row[bar_x] = bar;
        }
    }
}

/// `app` drawn as `frame` does but as the app draws it: no reference normalisation, rows in full.
pub(super) fn raw_text(app: &mut App, layout: Layout, w: u16, h: u16, glyphs: &str) -> String {
    let buf = &cells(app, layout, w, h, glyphs);
    (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end().to_string() + "\n").collect()
}

pub(super) fn frame(app: &mut App, layout: Layout, w: u16, h: u16, glyphs: &str) -> String {
    let buf = &cells(app, layout, w, h, glyphs);
    let mut rows: Vec<Vec<char>> = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' ')).collect()).collect();
    with_the_reference_carets(&mut rows, app, glyphs);
    with_the_reference_failed_bars(&mut rows, app, layout == Layout::Flat, glyphs);
    as_the_reference_sidebar(&mut rows, buf, app, layout == Layout::Flat);
    let text: String = rows
        .iter()
        .enumerate()
        .map(|(y, row)| {
            let row: String = row.iter().collect();
            let row = if y == 0 { without_new_tab_chip(&row) } else { row };
            row.trim_end().to_string() + "\n"
        })
        .collect();
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("===== {layout:?} | {w}x{h} | {glyphs}\n{text}");
    }
    text
}

mod tests {
    use super::*;

    #[test]
    fn every_stage_draws_in_every_layout_and_size() {
        for stage in STAGES {
            let mut app = world(stage);
            for layout in LAYOUTS {
                for (w, h) in SIZES {
                    let text = frame(&mut app, layout, w, h, "unicode");
                    assert_eq!(text.lines().count(), h as usize, "{stage:?} {layout:?} {w}x{h}");
                    assert!(text.contains("affiliate-site"), "{stage:?} {layout:?} {w}x{h}: the project is listed\n{text}");
                }
            }
        }
    }

    #[test]
    fn the_world_holds_the_reference_s_sessions_and_fix_footer_s_check() {
        let app = world(Stage::Failed);
        let names: Vec<&str> = app.snap.terms.values().map(|t| t.label.as_str()).collect();
        assert_eq!(names, ["fix-footer", "add-payments", "warm-finch", "quick-fox", "old-moth"]);
        let footer = &app.snap.terms[&1];
        let check = footer.check().expect("a check");
        assert_eq!((check.name.as_str(), check.command.as_str(), check.state, check.exit_code), ("test", "npm test", CheckState::Failed, Some(1)));
        assert_eq!((crate::clock::local_hms(check.started_at), crate::clock::local_hm(now())), ("14:22:05".to_string(), "14:24".to_string()));
        assert_eq!(super::super::screen::age(footer.since), "2m");
        let multi = world(Stage::Multi);
        let names = |id: TermId| multi.snap.terms[&id].checks.iter().map(|c| (c.name.clone(), c.state)).collect::<Vec<_>>();
        assert_eq!(names(1), [("build".to_string(), CheckState::Passed), ("test".to_string(), CheckState::Failed)]);
        assert_eq!(names(2), [("build".to_string(), CheckState::Passed), ("test".to_string(), CheckState::NeedsApproval)]);
        assert!(world(Stage::None).snap.terms[&1].checks.is_empty());
    }
}

/// The fixture's sessions and the mark of the agent each runs (Unicode, then ASCII).
const AGENT_MARKS: [(&str, char, char); 5] = [("fix-footer", '◆', 'X'), ("add-payments", '✻', 'C'), ("warm-finch", '◆', 'X'), ("quick-fox", '✻', 'C'), ("old-moth", '◆', 'X')];

/// The second lines that only name a session's state, which the app no longer draws.
const STATE_ONLY_LINES: [&str; 5] = ["working", "finished", "finished · not seen", "needs you", "idle"];

/// The reference's sidebar rows as the app draws them on purpose since sessions became one
/// line: a second line that only names the state is gone (the mark and the name's colour say
/// it), and each session's row carries its agent's mark two spaces before its age, or where the
/// age would end when there is none. A moving age (`any` in the reference) is any width, so the
/// mark's place before it is `any` too. A name that would run into the mark is cut with `…`.
pub(super) fn as_one_line_rows(rows: Vec<Vec<char>>, ascii: bool, any: char) -> Vec<Vec<char>> {
    let text = |r: &[char]| r.iter().collect::<String>();
    let mut rows: Vec<Vec<char>> = rows.into_iter().filter(|r| !STATE_ONLY_LINES.contains(&text(r).trim_start_matches([' ', '▎', '|']).trim_end())).collect();
    let has = |r: &[char], needle: &str| text(r).contains(needle);
    let last_of = |r: &[char]| r.iter().rposition(|c| *c != ' ');
    let right_edge = rows.iter().find(|r| has(r, "pixls")).and_then(|r| last_of(r)).unwrap_or(0);
    for row in &mut rows {
        let Some((_, shape, letter)) = AGENT_MARKS.iter().find(|(name, ..)| has(row, name)) else { continue };
        let Some(last) = last_of(row) else { continue };
        let start = row[..=last].iter().rposition(|c| *c == ' ').map_or(0, |i| i + 1);
        let token: String = row[start..=last].iter().collect();
        let moving = !token.is_empty() && token.chars().all(|c| c == any);
        let is_age = moving || token == "now" || token.trim_end_matches(['s', 'm', 'h', 'd']).parse::<u32>().is_ok();
        let at = if is_age { start.saturating_sub(3) } else { right_edge };
        if row.len() <= at + 1 {
            row.resize(at + 2, ' ');
        }
        if moving {
            // The name may or may not be cut, as the age is wider or narrower.
            row[at.saturating_sub(2)..=at + 1].fill(any);
            continue;
        }
        if at >= 2 && row[at - 1] != ' ' {
            row[at - 2] = '…';
            row[at - 1] = ' ';
        }
        row[at] = if ascii { *letter } else { *shape };
    }
    rows
}
