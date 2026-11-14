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
    Passed,
    Failed,
    Couldnt,
    Approve,
    Never,
    Stale,
    /// Several checkpoints: build and test.
    Multi,
}

/// The reference's boxed frames: Floating, and Tiled (Floating with no gaps). Its Flat frames
/// are of a look the app no longer has.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Layout {
    Floating,
    Tiled,
}

pub(super) const LAYOUTS: [Layout; 2] = [Layout::Floating, Layout::Tiled];

/// The reference's frame sizes, widest first.
pub(super) const SIZES: [(u16, u16); 3] = [(160, 45), (100, 30), (80, 24)];

impl Layout {
    /// `[ui] gap`.
    fn gap(self) -> &'static str {
        match self {
            Layout::Floating => "1",
            Layout::Tiled => "0",
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
    let shown: Vec<i64> = crate::clock::local_hms(guess)
        .split(':')
        .map(|p| p.parse().unwrap())
        .collect();
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
    directories::BaseDirs::new()
        .map(|d| d.home_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir)
        .join("dev")
}

pub(super) fn term(
    id: TermId,
    label: &str,
    agent: &str,
    status: Status,
    cwd: &Path,
    since: u64,
) -> TermInfo {
    TermInfo {
        conversation_id: None,
        id,
        cols: 80,
        rows: 20,
        title: String::new(),
        process: "node".into(),
        remote: None,
        popup: false,
        agent: Some(agent.into()),
        hooked: true,
        status,
        cwd: cwd.to_path_buf(),
        summary: String::new(),
        name: String::new(),
        label: label.into(),
        pinned: false,
        archived: false,
        snoozed_until: None,
        model: String::new(),
        mem: 0,
        cpu: 0.0,
        bell: false,
        said: String::new(),
        last_lines: String::new(),
        branch: None,
        linked: false,
        diff_added: 0,
        diff_removed: 0,
        subagents: Vec::new(),
        subagent_activity: Vec::new(),
        workflows: Vec::new(),
        session: None,
        effort: None,
        started_by: None,
        tab_of: None,
        helper_ask: Vec::new(),
        pr: None,
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
        repeats: None,
        pid: 0,
        asleep: false,
        win32_input: false,
        usage: Default::default(),
        resume_at: None,
        checks: Vec::new(),
        asking: String::new(),
        interrupted: None,
        exit: None,
        ask: None,
        timeline: Vec::new(),
    }
}

/// A pane in a checkout of `root` on `branch`.
/// What quick-fox, the sample's finished agent, last said: its Inbox item says it where the
/// reference frames say `finished · not seen`.
pub(super) const QUICK_FOX_SAID: &str = "Trimmed the cache; 14 tests pass.";

pub(super) fn in_repo(mut t: TermInfo, root: &Path, branch: &str) -> TermInfo {
    t.linked = t.cwd != root;
    (t.root, t.top, t.branch) = (
        Some(root.to_path_buf()),
        Some(t.cwd.clone()),
        Some(branch.into()),
    );
    t
}

pub(super) fn entry(path: &Path, branch: &str, main: bool, repo: &str) -> WorktreeEntry {
    WorktreeEntry {
        path: path.to_path_buf(),
        branch: branch.into(),
        main,
        repo: repo.into(),
    }
}

pub(super) fn workspace(
    id: WsId,
    name: &str,
    cwd: &Path,
    terms: &[TermId],
    git: Option<GitInfo>,
    color: u8,
) -> WorkspaceInfo {
    let tabs: Vec<TabInfo> = terms
        .iter()
        .enumerate()
        .map(|(i, &t)| TabInfo {
            id: id + 1 + i as TabId,
            name: String::new(),
            layout: Node::Leaf(t),
            focus: t,
        })
        .collect();
    let active_tab = tabs[0].id;
    WorkspaceInfo {
        id,
        name: name.into(),
        cwd: cwd.to_path_buf(),
        tabs,
        active_tab,
        git,
        worktree: false,
        color,
        is_new: false,
        group: None,
    }
}

/// Focus pane `term` as the daemon would: its workspace and its tab become the active ones.
pub(super) fn focus(app: &mut App, term: TermId) {
    for w in &mut app.snap.workspaces {
        if let Some(tab) = w.tabs.iter_mut().find(|t| t.layout.contains(term)) {
            tab.focus = term;
            w.active_tab = tab.id;
            app.snap.active_ws = Some(w.id);
        }
    }
    app.screen_sync();
}

pub(super) fn git(root: &Path, branch: &str, worktrees: Vec<WorktreeEntry>) -> GitInfo {
    let repo = super::screen::folder_name(root);
    GitInfo {
        repo,
        branch: branch.into(),
        dirty: 0,
        linked: false,
        root: root.to_path_buf(),
        worktrees,
        ahead: 0,
    }
}

const OUT_FAIL_LINES: u64 = 13;
const OUT_PASS_LINES: u64 = 3;
const FIRST_FAILING: &str = "✗ src/pages/legal.test.tsx > footer shows current year";

/// The repo file a repo-supplied command comes from, added three days ago.
fn from_repo() -> CheckSource {
    CheckSource::Repo {
        file: concat!(".", env!("APP_COMMAND"), "/checks.toml").into(),
        commit: Some("a1b2c3d".into()),
        committed_at: Some(now() - 3 * 24 * HOUR),
        uncommitted: false,
    }
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
    let passed = CheckInfo {
        state: CheckState::Passed,
        exit_code: Some(0),
        line_count: OUT_PASS_LINES,
        ..ran.clone()
    };
    let failed = CheckInfo {
        state: CheckState::Failed,
        exit_code: Some(1),
        line_count: OUT_FAIL_LINES,
        first_failing_line: FIRST_FAILING.into(),
        ..ran.clone()
    };
    let unrun = |state| CheckInfo {
        state,
        started_at: 0,
        duration_ms: 0,
        source: from_repo(),
        ..ran.clone()
    };
    match stage {
        Stage::None => vec![],
        Stage::Passed => vec![passed],
        Stage::Failed => vec![failed],
        Stage::Couldnt => vec![CheckInfo {
            state: CheckState::CouldNotRun,
            command: "pnpm test".into(),
            exit_code: Some(127),
            duration_ms: 100,
            line_count: 1,
            ..ran
        }],
        Stage::Approve => vec![unrun(CheckState::NeedsApproval)],
        Stage::Never => vec![unrun(CheckState::Never)],
        Stage::Stale => vec![CheckInfo {
            state: CheckState::Stale,
            stale_since: Some(at(14, 24, 0)),
            changed_files: [
                ('M', "src/components/Footer.tsx"),
                ('M', "src/pages/legal.tsx"),
                ('A', "tests/legal.test.tsx"),
            ]
            .map(|(kind, path)| ChangedFile {
                kind,
                path: path.into(),
            })
            .into(),
            ..passed
        }],
        Stage::Multi => vec![build_passed(&ran), failed],
    }
}

fn build_passed(ran: &CheckInfo) -> CheckInfo {
    CheckInfo {
        name: "build".into(),
        command: "npm run build".into(),
        state: CheckState::Passed,
        exit_code: Some(0),
        line_count: OUT_PASS_LINES,
        ..ran.clone()
    }
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
    let cfg = Config {
        ui: crate::config::Ui {
            padding: "0".into(),
            // The reference has no tool rail, and its sheets sit beside the panes.
            ..Default::default()
        },
        ..Default::default()
    };
    let mut app = App::new(cfg, tx, None, bg_tx);
    app.splash = false;
    let dev = home();
    let (site, pixls, orbit) = (
        dev.join("affiliate-site"),
        dev.join("pixls"),
        dev.join("orbit"),
    );
    let trees = dev.join("affiliate-site-worktrees");
    let (footer_dir, payments_dir) = (trees.join("fix-footer"), trees.join("add-payments"));

    let mut footer = in_repo(
        term(
            1,
            "fix-footer",
            "codex",
            Status::Done,
            &footer_dir,
            at(14, 21, 51),
        ),
        &site,
        "fix-footer",
    );
    footer.checks = fix_footer_checks(stage, &footer_dir);
    let mut payments = in_repo(
        term(
            2,
            "add-payments",
            "claude",
            Status::Working,
            &payments_dir,
            now(),
        ),
        &site,
        "add-payments",
    );
    if stage == Stage::Multi {
        let base = &footer.checks[0];
        let approve = CheckInfo {
            name: "test".into(),
            state: CheckState::NeedsApproval,
            started_at: 0,
            duration_ms: 0,
            exit_code: None,
            line_count: 0,
            source: from_repo(),
            ..base.clone()
        };
        (payments.status, payments.since, payments.checks) =
            (Status::Done, now() - 40, vec![build_passed(base), approve]);
    }
    let finch = term(
        3,
        "warm-finch",
        "codex",
        Status::Blocked,
        &pixls,
        now() - 3 * 60,
    );
    let mut fox = in_repo(
        term(
            4,
            "quick-fox",
            "claude",
            Status::Done,
            &orbit,
            now() - 9 * HOUR,
        ),
        &orbit,
        "dev",
    );
    fox.said = QUICK_FOX_SAID.into();
    let mut moth = in_repo(
        term(
            5,
            "old-moth",
            "codex",
            Status::Idle,
            &orbit,
            now() - 2 * 24 * HOUR,
        ),
        &orbit,
        "dev",
    );
    moth.asleep = true;

    let site_trees = vec![
        entry(&site, "main", true, "affiliate-site"),
        entry(&footer_dir, "fix-footer", false, "affiliate-site"),
        entry(&payments_dir, "add-payments", false, "affiliate-site"),
    ];
    app.snap.workspaces = vec![
        workspace(
            10,
            "affiliate-site",
            &site,
            &[1, 2],
            Some(git(&site, "main", site_trees)),
            0,
        ),
        workspace(20, "pixls", &pixls, &[3], None, 1),
        workspace(
            30,
            "orbit",
            &orbit,
            &[4, 5],
            Some(git(
                &orbit,
                "dev",
                vec![entry(&orbit, "dev", true, "orbit")],
            )),
            2,
        ),
    ];
    app.snap.active_ws = Some(10);
    app.snap.questions = vec![HumanQuestion {
        id: 1,
        term: 3,
        text: "use pnpm or npm?".into(),
        options: vec!["pnpm".into(), "npm".into()],
    }];
    // The reference lists the project that asks a question first. The app no longer moves a
    // project when its sessions need you: it keeps the order you first saw them in.
    app.screen.saved.order = [&pixls, &site, &orbit]
        .iter()
        .map(|p| super::design::path_key(p))
        .collect();
    for t in [footer, payments, finch, fox, moth] {
        app.parsers.insert(
            ClientTerm::local(t.id),
            if t.id == 1 {
                fix_footer_screen()
            } else {
                vt100::Parser::new(20, 80, 0)
            },
        );
        app.snap.terms.insert(t.id, t);
    }
    app.screen_sync();
    app
}

/// The world drawn in `layout` at `w`×`h`, every cell with its colours. The reference has no mode
/// bar, which takes the bottom row: the world is drawn a row taller and that row, the
/// bar's and nothing else, is left out, so the screen above it is laid out as at `w`×`h`.
pub(super) fn cells(
    app: &mut App,
    layout: Layout,
    w: u16,
    h: u16,
    glyphs: &str,
) -> ratatui::buffer::Buffer {
    FIXED_NOW.with(|n| n.set(Some(now())));
    (app.cfg.ui.layout, app.cfg.ui.gap, app.cfg.ui.glyphs) =
        ("floating".into(), layout.gap().into(), glyphs.into());
    app.screen_fresh();
    // The bar hides while keys go to the panes, but the reference is laid out with its row and the
    // gap above it, so a flash holds the bar open for this one frame.
    let flash = app.bar_flash.replace((String::new(), Instant::now()));
    let mut term = Terminal::new(TestBackend::new(w, h + 1)).expect("a test terminal");
    term.draw(|f| render::draw(app, f)).expect("a frame");
    let full = term.backend().buffer();
    assert_eq!(
        super::screen::mode_bar_row(app, full.area).y,
        h,
        "the bar is the row left out"
    );
    app.bar_flash = flash;
    let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, w, h));
    for (x, y) in (0..h).flat_map(|y| (0..w).map(move |x| (x, y))) {
        buf[(x, y)] = full[(x, y)].clone();
    }
    buf
}

/// `app` drawn as `frame` does but as the app draws it: no reference normalisation, rows in full.
pub(super) fn raw_text(app: &mut App, layout: Layout, w: u16, h: u16, glyphs: &str) -> String {
    let buf = &cells(app, layout, w, h, glyphs);
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
                + "\n"
        })
        .collect()
}

mod tests {
    use super::*;

    #[test]
    fn the_world_holds_the_reference_s_sessions_and_fix_footer_s_check() {
        let app = world(Stage::Failed);
        let names: Vec<&str> = app.snap.terms.values().map(|t| t.label.as_str()).collect();
        assert_eq!(
            names,
            [
                "fix-footer",
                "add-payments",
                "warm-finch",
                "quick-fox",
                "old-moth"
            ]
        );
        let footer = &app.snap.terms[&1];
        let check = footer.check().expect("a check");
        assert_eq!(
            (
                check.name.as_str(),
                check.command.as_str(),
                check.state,
                check.exit_code
            ),
            ("test", "npm test", CheckState::Failed, Some(1))
        );
        assert_eq!(
            (
                crate::clock::local_hms(check.started_at),
                crate::clock::local_hm(now())
            ),
            ("14:22:05".to_string(), "14:24".to_string())
        );
        assert_eq!(super::super::screen::age(footer.since), "2m");
        let multi = world(Stage::Multi);
        let names = |id: TermId| {
            multi.snap.terms[&id]
                .checks
                .iter()
                .map(|c| (c.name.clone(), c.state))
                .collect::<Vec<_>>()
        };
        assert_eq!(
            names(1),
            [
                ("build".to_string(), CheckState::Passed),
                ("test".to_string(), CheckState::Failed)
            ]
        );
        assert_eq!(
            names(2),
            [
                ("build".to_string(), CheckState::Passed),
                ("test".to_string(), CheckState::NeedsApproval)
            ]
        );
        assert!(world(Stage::None).snap.terms[&1].checks.is_empty());
    }
}

/// `app` drawn in `layout` at `w`×`h` with the `glyphs` set (`unicode`, `ascii`, `nerd`), as
/// text: one line per row, trailing spaces trimmed, a wide character as its first cell.
/// `<APP>_SHOW=1` prints it under a heading.
pub(super) fn frame(app: &mut App, layout: Layout, w: u16, h: u16, glyphs: &str) -> String {
    let buf = &cells(app, layout, w, h, glyphs);
    let text: String = (0..h)
        .map(|y| {
            let row: String = (0..w)
                .map(|x| buf[(x, y)].symbol().chars().next().unwrap_or(' '))
                .collect();
            row.trim_end().to_string() + "\n"
        })
        .collect();
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("===== {layout:?} | {w}x{h} | {glyphs}\n{text}");
    }
    text
}
