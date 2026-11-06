//! Starting a session without the popup (`n`), the scoped ` + ` chips and the leader's picker key.

use super::*;
use crate::client::ids::{MachineId, Remote};
use crate::client::reference_fixture::{Stage, world};
use crate::client::screen::quick_new::{QuickTarget, Selected, quick_target_of};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::layout::Rect;

const BUILD: MachineId = MachineId(1);
const GARAGE: MachineId = MachineId(2);

fn machines() -> (App, mpsc::UnboundedReceiver<Outgoing>) {
    let mut app = world(Stage::None);
    let (tx, rx) = mpsc::unbounded_channel();
    app.out = tx;
    let remote = |name: &str, state| Remote {
        name: name.into(),
        state,
        ..Remote::default()
    };
    app.remotes.insert(BUILD, remote("build", crate::client::link::ConnectionState::Connected));
    app.remotes.insert(
        GARAGE,
        remote(
            "pi-garage",
            crate::client::link::ConnectionState::Unreachable {
                since: std::time::Instant::now(),
                why: "no route".into(),
            },
        ),
    );
    app.screen_fresh();
    (app, rx)
}

fn frame(app: &mut App, w: u16, h: u16) -> (Vec<String>, ratatui::buffer::Buffer) {
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
    term.draw(|f| render::draw(app, f)).unwrap();
    let buf = term.backend().buffer().clone();
    let rows = (0..h)
        .map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>())
        .collect();
    (rows, buf)
}

fn sent(rx: &mut mpsc::UnboundedReceiver<Outgoing>) -> Vec<Outgoing> {
    std::iter::from_fn(|| rx.try_recv().ok()).collect()
}

fn started(out: &[Outgoing]) -> usize {
    out.iter()
        .filter(|o| matches!(&o.msg, ClientMsg::Command(Command::NewWorkspace { .. })))
        .count()
}

fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
    app.on_key(KeyEvent::new(code, mods));
}

fn local_project(app: &App) -> screen::Proj {
    app.screen_model()
        .into_iter()
        .find(|p| p.machine == MachineId::LOCAL)
        .expect("a local project")
}

/// The rects of every ` + ` hit that opens the picker on `machine`.
fn chip_rects(app: &App, machine: MachineId) -> Vec<Rect> {
    app.hits
        .iter()
        .filter(|(_, h)| *h == Hit::Screen(screen::ScreenHit::NsMachine(machine)))
        .map(|(r, _)| *r)
        .collect()
}

fn click(app: &mut App, r: Rect) {
    app.on_mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: r.x + 1,
        row: r.y,
        modifiers: KeyModifiers::NONE,
    });
}

// ---- quick_target: every row of the table ------------------------------------------------------

#[test]
fn quick_target_follows_the_table_row_by_row() {
    let (app, _rx) = machines();
    let p = local_project(&app);
    let main = p.wts.iter().find(|w| w.main).expect("main checkout");

    // A session, a shell tab or a project header: that project, on its machine, on its branch.
    assert_eq!(
        quick_target_of(Selected::Project(&p)),
        QuickTarget::Start {
            machine: MachineId::LOCAL,
            project: main.path.clone(),
            branch: Some(main.branch.clone()).filter(|b| !b.is_empty()),
        }
    );
    let mut remote = p.clone();
    remote.machine = BUILD;
    assert!(matches!(
        quick_target_of(Selected::Project(&remote)),
        QuickTarget::Start { machine: BUILD, .. }
    ));
    // A project with no checkout of its own starts in its folder, with no branch.
    let mut bare = p.clone();
    bare.wts.clear();
    assert_eq!(
        quick_target_of(Selected::Project(&bare)),
        QuickTarget::Start {
            machine: MachineId::LOCAL,
            project: bare.path.clone(),
            branch: None
        }
    );
    // A machine with nothing running asks where; one that is down is dialled again.
    assert_eq!(
        quick_target_of(Selected::Machine { id: BUILD, down: false }),
        QuickTarget::Scoped(BUILD)
    );
    assert_eq!(
        quick_target_of(Selected::Machine { id: GARAGE, down: true }),
        QuickTarget::Reconnect(GARAGE)
    );
    // Nothing selected asks where, on this computer.
    assert_eq!(quick_target_of(Selected::Nothing), QuickTarget::Scoped(MachineId::LOCAL));
}

#[test]
fn the_apps_quick_target_reads_the_selection_even_when_the_sidebar_has_no_keys() {
    let (mut app, _rx) = machines();
    let p = local_project(&app);
    // The keys are in a pane; the sidebar's project cursor still names the project.
    app.mode = Mode::Normal;
    app.screen.cursor_proj = Some(p.key.clone());
    assert!(matches!(app.quick_target(), QuickTarget::Start { machine: MachineId::LOCAL, .. }));
    // A session of the project says the same.
    app.screen.cursor_proj = None;
    app.screen.cursor = p.sessions().next().map(|s| s.key);
    assert!(matches!(app.quick_target(), QuickTarget::Start { machine: MachineId::LOCAL, .. }));
    // Nothing selected and nothing open: pick a folder here.
    app.screen.cursor = None;
    app.screen.proj = None;
    assert_eq!(app.quick_target(), QuickTarget::Scoped(MachineId::LOCAL));
}

#[test]
fn a_selection_on_a_machine_that_went_down_reconnects_it() {
    let (mut app, _rx) = machines();
    let mut p = local_project(&app);
    p.machine = GARAGE;
    // The project the sidebar names lives on a machine whose line is down.
    let mut model = app.screen_model();
    model.retain(|q| q.key != p.key);
    model.push(p.clone());
    *app.screen.model_cache.borrow_mut() = Some(model);
    app.screen.cursor_proj = Some(p.key.clone());
    assert_eq!(app.quick_target(), QuickTarget::Reconnect(GARAGE));
    assert_eq!(app.quick_hint(), "reconnect pi-garage");
}

// ---- the live hint -----------------------------------------------------------------------------------

#[test]
fn the_hint_says_what_n_will_do() {
    let (mut app, _rx) = machines();
    let p = local_project(&app);
    app.screen.cursor_proj = Some(p.key.clone());
    assert_eq!(app.quick_hint(), format!("new in {}", p.name));
    app.screen.cursor_proj = None;
    app.screen.cursor = None;
    app.screen.proj = None;
    assert_eq!(app.quick_hint(), "new on local…");

    // A project on a connected machine names the machine too.
    let mut remote = p.clone();
    remote.machine = BUILD;
    let mut model = app.screen_model();
    model.retain(|q| q.key != p.key);
    model.push(remote);
    *app.screen.model_cache.borrow_mut() = Some(model);
    app.screen.cursor_proj = Some(p.key.clone());
    assert_eq!(app.quick_hint(), format!("new in {} · build", p.name));
}

#[test]
fn the_hint_is_in_the_sidebars_mode_line() {
    let (mut app, _rx) = machines();
    let p = local_project(&app);
    app.screen.cursor_proj = Some(p.key.clone());
    app.mode = Mode::Side;
    let (rows, _) = frame(&mut app, 160, 45);
    let bar = rows.iter().find(|r| r.contains("SIDEBAR")).expect("the mode line");
    assert!(bar.contains(&format!("n new in {}", p.name)), "{bar}");
    assert!(bar.contains("Esc back"), "{bar}");
}

// ---- n, N and the new chip ------------------------------------------------------------------------

#[test]
fn n_in_the_sidebar_starts_at_once_and_flashes_what_started() {
    let (mut app, mut rx) = machines();
    sent(&mut rx);
    let p = local_project(&app);
    app.screen.cursor_proj = Some(p.key.clone());
    app.mode = Mode::Side;

    press(&mut app, KeyCode::Char('n'), KeyModifiers::NONE);

    assert!(!matches!(app.mode, Mode::NewSession(_)), "no popup");
    assert_eq!(started(&sent(&mut rx)), 1, "one session started");
    let (flash, _) = app.bar_flash.clone().expect("the bar says what started");
    assert!(flash.starts_with("+ "), "{flash}");
    assert!(flash.contains(&format!("· {} on local", p.name)), "{flash}");
    assert_eq!(app.screen.cursor_proj, None, "the new session is the selected one");
    assert!(app.screen.follow, "and is scrolled into view");
    assert_eq!(app.screen.saved.recent_paths.len(), 1, "the folder is remembered");

    let (rows, buf) = frame(&mut app, 160, 45);
    let at = rows.iter().rposition(|r| r.contains(&flash)).expect("the flash is on the bar");
    let x = rows[at].find(&flash).map(|b| rows[at][..b].chars().count()).unwrap() as u16;
    assert_eq!(buf[(x, at as u16)].fg, app.theme.done, "in green");
}

#[test]
fn n_with_nothing_selected_asks_where_and_capital_n_always_opens_the_picker() {
    let (mut app, mut rx) = machines();
    sent(&mut rx);
    app.screen.cursor = None;
    app.screen.cursor_proj = None;
    app.screen.proj = None;
    app.mode = Mode::Side;

    press(&mut app, KeyCode::Char('n'), KeyModifiers::NONE);
    let Mode::NewSession(ns) = &app.mode else { panic!("the scoped picker should open") };
    assert_eq!(ns.q, "~/");
    assert_eq!(started(&sent(&mut rx)), 0);

    app.mode = Mode::Side;
    app.screen.cursor_proj = Some(local_project(&app).key);
    press(&mut app, KeyCode::Char('N'), KeyModifiers::SHIFT);
    let Mode::NewSession(ns) = &app.mode else { panic!("the full picker should open") };
    assert_eq!(ns.q, "", "the full picker starts empty");
    assert_eq!(started(&sent(&mut rx)), 0);
}

// ---- the scoped chips -------------------------------------------------------------------------------------

#[test]
fn local_and_each_online_machine_have_a_plus_chip_and_an_offline_one_has_none() {
    let (mut app, _rx) = machines();
    let (rows, buf) = frame(&mut app, 120, 40);
    let side = app.screen.side_rect;

    let local = chip_rects(&app, MachineId::LOCAL);
    assert_eq!(local.len(), 1, "{local:?}");
    assert_eq!((local[0].x, local[0].y), (side.x + 12, side.y), "after the fold arrow and the LOCAL title on the border");
    assert_eq!(rows[local[0].y as usize].chars().skip(local[0].x as usize).take(3).collect::<String>(), " + ");
    let cell = &buf[(local[0].x + 1, local[0].y)];
    assert_eq!((cell.fg, cell.bg), (app.theme.accent, app.theme.btn), "orange on the chip ground");

    let build_row = rows.iter().position(|r| r.contains("⇄ build")).expect("build's badge row");
    let on_row: Vec<_> = chip_rects(&app, BUILD).into_iter().filter(|r| r.y as usize == build_row).collect();
    assert_eq!(on_row.len(), 1, "one chip on build's badge row: {rows:#?}");
    let line: Vec<char> = rows[build_row].chars().collect();
    let badge_end = line.iter().collect::<String>().find("⇄ build").map(|b| rows[build_row][..b].chars().count() + "⇄ build ".chars().count()).unwrap();
    assert_eq!(on_row[0].x as usize, badge_end + 1, "one space after the badge");

    let garage_row = rows.iter().position(|r| r.contains("⇄ pi-garage")).expect("pi-garage's badge row");
    assert!(
        chip_rects(&app, GARAGE).iter().all(|r| r.y as usize != garage_row),
        "no chip on an offline machine"
    );
}

#[test]
fn a_chip_opens_the_picker_in_path_mode_on_that_machine_with_its_title() {
    for (machine, query, title) in [
        (MachineId::LOCAL, "~/", "NEW SESSION ON LOCAL"),
        (BUILD, "build:~/", "NEW SESSION ON BUILD"),
    ] {
        let (mut app, _rx) = machines();
        let (rows, _) = frame(&mut app, 120, 40);
        let row = match machine {
            MachineId::LOCAL => app.screen.side_rect.y,
            _ => rows.iter().position(|r| r.contains("⇄ build")).unwrap() as u16,
        };
        let chip = chip_rects(&app, machine).into_iter().find(|r| r.y == row).expect("the chip");
        click(&mut app, chip);
        let Mode::NewSession(ns) = &app.mode else { panic!("no picker for {query}") };
        assert_eq!(ns.q, query);
        let (rows, _) = frame(&mut app, 120, 40);
        assert!(rows.iter().any(|r| r.contains(title)), "{title}:\n{}", rows.join("\n"));
    }
}

#[test]
fn the_full_picker_keeps_the_plain_title() {
    let (mut app, _rx) = machines();
    app.screen_open_session_popup("");
    let (rows, _) = frame(&mut app, 120, 40);
    assert!(rows.iter().any(|r| r.contains("NEW SESSION ") && !r.contains(" ON ")), "plain title");
}

#[test]
fn the_remote_rule_adds_a_machine_with_m_add() {
    let (mut app, _rx) = machines();
    let (rows, _) = frame(&mut app, 120, 40);
    let rule = rows.iter().find(|r| r.contains("REMOTE")).expect("the remote rule");
    assert!(rule.contains(" m add"), "{rule}");
    assert!(!rule.contains("+ Add"), "{rule}");
}

// ---- the leader --------------------------------------------------------------------------------------------

#[test]
fn leader_o_opens_the_full_picker_from_a_terminal_pane() {
    let (mut app, _rx) = machines();
    app.mode = Mode::Normal;
    press(&mut app, KeyCode::Char(' '), KeyModifiers::CONTROL);
    assert!(matches!(app.mode, Mode::Prefix { .. }), "Ctrl+Space is the leader from a pane");
    press(&mut app, KeyCode::Char('o'), KeyModifiers::NONE);
    let Mode::NewSession(ns) = &app.mode else { panic!("the picker should open") };
    assert_eq!(ns.q, "");
}

// ---- the help popup -----------------------------------------------------------------------------------------

#[test]
fn all_keys_lists_the_new_session_keys() {
    let (mut app, _rx) = machines();
    app.open_key_view(screen::KeyView::All);
    let (rows, _) = frame(&mut app, 200, 60);
    let shown = rows.join("\n");
    assert!(shown.contains("New session anywhere"), "{shown}");
    assert!(shown.contains("Pick anywhere") && shown.contains("New here"), "{shown}");
}
