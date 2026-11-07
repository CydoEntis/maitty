//! The sidebar (fleets above, ships below), the collapsed column, the status glyphs, the tab row
//! and the workspace frames, held to the TUI design (v27). `MAITTY_SHOW=1 cargo test <name> -- --nocapture` prints the frames.
use super::status_mark::State;
use super::*;
use crate::client::reference_fixture::{self, Stage};
use crate::layout::{Dir, Node};
use crate::protocol::ClientMsg;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseButton, MouseEvent, MouseEventKind};
use ratatui::Terminal;
use ratatui::backend::TestBackend;
use tokio::sync::mpsc;

fn cells(app: &mut App, w: u16, h: u16) -> Buffer {
    app.cfg.ui.gap = "1".into();
    app.screen_fresh();
    let mut term = Terminal::new(TestBackend::new(w, h)).expect("a test terminal");
    term.draw(|f| crate::client::render::draw(app, f))
        .expect("a frame");
    term.backend().buffer().clone()
}

/// `app` drawn at `w`×`h` as rows of text, trailing spaces trimmed.
fn text(app: &mut App, w: u16, h: u16) -> Vec<String> {
    let buf = cells(app, w, h);
    (0..h)
        .map(|y| {
            (0..w)
                .map(|x| buf[(x, y)].symbol())
                .collect::<String>()
                .trim_end()
                .to_string()
        })
        .collect()
}

fn show(rows: &[String], title: &str) {
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("---- {title}");
        for row in rows {
            println!("{row}");
        }
    }
}

fn key(app: &mut App, code: KeyCode) {
    app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
}

/// The sidebar's columns of row `y` of a frame with the full sidebar.
fn side(rows: &[String], y: usize) -> String {
    rows[y].chars().take(46).collect()
}

fn click(app: &mut App, x: u16, y: u16) {
    for kind in [
        MouseEventKind::Down(MouseButton::Left),
        MouseEventKind::Up(MouseButton::Left),
    ] {
        app.on_mouse(MouseEvent {
            kind,
            column: x,
            row: y,
            modifiers: KeyModifiers::NONE,
        });
    }
}

/// The world with its first session split into `n` panes (2 to 4).
fn split_world(n: usize) -> App {
    let mut app = reference_fixture::world(Stage::None);
    let focus = app.focused().expect("a focused pane");
    let tab = app.screen.tab;
    let layout = &mut app.screen.tabs[tab].layout;
    layout.split(focus.term, Dir::Right, 2);
    if n > 2 {
        layout.split(2, Dir::Down, 3);
    }
    if n > 3 {
        layout.split(focus.term, Dir::Down, 4);
    }
    app
}

// ---- the sidebar ---------------------------------------------------------------------------

#[test]
fn the_sidebar_lists_machines_and_fleets_above_and_agent_ships_below() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    show(&rows, "140x36");
    // The top half: the machine, then its fleets, the one that needs you with its count.
    assert!(side(&rows, 4).contains("FLEETS"), "{}", side(&rows, 4));
    assert!(side(&rows, 6).contains("▾ ⌂ This computer"), "{}", side(&rows, 6));
    assert!(side(&rows, 6).trim_end().ends_with("●  + │"), "{}", side(&rows, 6));
    for (y, name, tail) in [
        (7, "affiliate-site", "✻ 2"),
        (8, "pixls", "✻ 1"),
        (9, "orbit", "✻ 2"),
    ] {
        let line = side(&rows, y);
        assert!(line.contains(name) && line.contains(tail), "row {y}: {line}");
    }
    assert!(side(&rows, 8).contains("1 pixls"), "the amber count: {}", side(&rows, 8));
    assert!(side(&rows, 7).contains("◆ affiliate-site"), "{}", side(&rows, 7));
    assert!(side(&rows, 7).starts_with("│▌"), "the current fleet wears the bar");
    // The divider at the middle, and the ships grouped under their fleets.
    assert!(side(&rows, 17).contains("SHIPS 5"), "{}", side(&rows, 17));
    assert!(side(&rows, 19).contains("▾ ◆ affiliate-site"));
    assert!(side(&rows, 20).contains("fix-footer") && side(&rows, 20).trim_end().ends_with(">_ │"));
    assert!(side(&rows, 21).contains("⎇ fix-footer") && side(&rows, 21).trim_end().ends_with("worktree │"));
    assert!(side(&rows, 24).contains("▾ ◆ pixls"));
    assert!(side(&rows, 25).contains("warm-finch"));
    assert!(
        side(&rows, 26).contains("pixls") && !side(&rows, 26).contains('⎇'),
        "no repo: the folder: {}",
        side(&rows, 26)
    );
}

#[test]
fn each_half_clips_at_its_own_divider_and_the_divider_sits_at_the_middle() {
    for h in [24u16, 30, 36, 50] {
        let mut app = reference_fixture::world(Stage::None);
        let rows = text(&mut app, 140, h);
        let bar = usize::from(h) - 1;
        let ships = rows.iter().position(|r| side_of(r).contains("SHIPS")).expect("a divider");
        let fleets = rows.iter().position(|r| side_of(r).contains("FLEETS")).expect("a divider");
        // The sidebar is h-1 rows tall (the status bar has the last): MID = floor((4 + H - 5) / 2).
        let sidebar_h = usize::from(h) - 1;
        if sidebar_h >= 14 {
            assert_eq!(fleets, 4, "{h}");
            assert_eq!(ships, (4 + sidebar_h - 5) / 2, "{h}");
        }
        assert!(ships > fleets && ships < bar, "{h}");
        // Nothing of a ship's two lines hangs above its divider, nothing of a fleet below it.
        for (y, row) in rows.iter().enumerate().take(bar) {
            let line = side_of(row);
            if y > fleets && y < ships {
                assert!(!line.contains("⎇") && !line.contains("▾ ◆"), "{h} row {y}: {line}");
            }
            if y > ships {
                assert!(!line.contains("▾ ⌂") && !line.contains("✻"), "{h} row {y}: {line}");
            }
        }
    }
}

fn side_of(row: &str) -> String {
    row.chars().take(46).collect()
}

#[test]
fn folding_a_machine_or_a_fleets_ship_group_hides_its_rows_and_keeps_the_count() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, _rx) = mpsc::unbounded_channel();
    app.out = tx;
    text(&mut app, 140, 36);
    // Click the ship group heading of pixls (needs you): it folds and says so.
    let y = 24;
    click(&mut app, 12, y);
    assert!(app.screen.saved.closed.iter().any(|k| k.starts_with("c:local/")), "{:?}", app.screen.saved.closed);
    let rows = text(&mut app, 140, 36);
    show(&rows, "pixls folded");
    assert!(side(&rows, 24).contains("▸ ◆ pixls") && side(&rows, 24).contains("1 need you"), "{}", side(&rows, 24));
    assert!(!rows.iter().any(|r| side_of(r).contains("warm-finch")), "its ship is folded away");
    // The machine heading folds its fleets and shows how many.
    click(&mut app, 12, 6);
    let rows = text(&mut app, 140, 36);
    assert!(side(&rows, 6).contains("▸ ⌂ This computer  3"), "{}", side(&rows, 6));
    assert!(!side(&rows, 7).contains("affiliate-site"), "{}", side(&rows, 7));
    assert!(app.screen.saved.closed.iter().any(|k| k == "m:local"));
}

#[test]
fn up_and_down_walk_the_one_ordered_list_and_skip_what_is_folded_away() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, _rx) = mpsc::unbounded_channel();
    app.out = tx;
    text(&mut app, 140, 36);
    app.screen_fold("c:local/20", Some(false));
    text(&mut app, 140, 36);
    let items = app.screen.side_items.clone();
    assert!(items.contains(&SideItem::Proj("m:local".into())));
    assert!(items.contains(&SideItem::Proj("c:local/20".into())));
    assert!(!items.contains(&SideItem::Sess(app.session_key(ClientTerm::local(3)))), "folded away");
    // Fleets come before ships, whatever the order of their ids.
    let first_ship = items.iter().position(|i| matches!(i, SideItem::Sess(_))).unwrap();
    let last_fleet = items.iter().rposition(|i| matches!(i, SideItem::Fleet(_))).unwrap();
    assert!(last_fleet < first_ship);
    app.screen_side_set(items[0].clone());
    for expected in &items[1..] {
        key(&mut app, KeyCode::Down);
        let at = match expected {
            SideItem::Proj(k) => app.screen.cursor_proj.as_ref() == Some(k),
            SideItem::Fleet(f) => app.screen.cursor_fleet == Some(*f),
            SideItem::Sess(s) => app.screen.cursor == Some(*s),
        };
        assert!(at, "{expected:?}");
    }
}

#[test]
fn enter_on_a_fleet_opens_its_last_active_session_and_on_a_heading_folds_it() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    text(&mut app, 140, 36);
    // Remember that add-payments (2) was the one you were last on in affiliate-site.
    app.note_fleet_active(ClientTerm::local(2));
    let fleet = app.fleet_of(ClientTerm::local(1)).unwrap();
    app.screen_side_set(SideItem::Fleet(fleet));
    key(&mut app, KeyCode::Enter);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|o| matches!(&o.msg, ClientMsg::Command(Command::FocusPane { term: 2 }))),
        "{} messages, none a focus on pane 2",
        sent.len()
    );
    app.screen_side_set(SideItem::Proj("m:local".into()));
    key(&mut app, KeyCode::Enter);
    assert!(app.screen.saved.closed.iter().any(|k| k == "m:local"));
}

#[test]
fn a_row_keeps_its_place_whatever_its_status() {
    let mut app = reference_fixture::world(Stage::None);
    let order = |app: &mut App| {
        let model = app.screen_model();
        side_keys(app, &model)
    };
    let before = order(&mut app);
    for status in [Status::Working, Status::Blocked, Status::Done, Status::Idle] {
        app.snap.terms.get_mut(&5).unwrap().status = status;
        app.screen_fresh();
        assert_eq!(order(&mut app), before, "{status:?}");
    }
}

#[test]
fn a_ship_row_ends_in_its_view_mark_and_shows_no_diff_counts() {
    let mut app = reference_fixture::world(Stage::None);
    let info = app.snap.terms.get_mut(&1).unwrap();
    (info.diff_added, info.diff_removed) = (104, 61);
    let rows = text(&mut app, 140, 36);
    let line = side(&rows, 20);
    assert!(!line.contains("+104") && !line.contains("−61"), "{line}");
    assert!(line.trim_end().ends_with(">_ │"), "the terminal view mark: {line}");
}

#[test]
fn idle_names_are_dimmed_and_the_open_session_has_the_raised_ground() {
    let mut app = reference_fixture::world(Stage::None);
    let buf = cells(&mut app, 140, 50);
    let t = app.theme.clone();
    let y = |term: u32| app.screen.row_y[&ClientTerm::local(term)];
    // old-moth (5) is idle: its name is muted, not the strong ink.
    assert_eq!(buf[(6, y(5))].fg, t.muted);
    assert_ne!(buf[(6, y(1))].fg, t.muted, "a session with work keeps the bright name");
    // fix-footer (1) is the one open in the workspace: raised ground and the bar on both lines.
    for dy in 0..2 {
        assert_eq!(buf[(10, y(1) + dy)].bg, t.card);
        assert_eq!(buf[(1, y(1) + dy)].symbol(), "▌");
    }
    assert_eq!(buf[(10, y(3))].bg, t.bg, "the others sit on the ground");
}

#[test]
fn the_current_fleet_has_the_raised_ground_and_a_needs_count_is_amber() {
    let mut app = reference_fixture::world(Stage::None);
    let buf = cells(&mut app, 140, 36);
    let t = app.theme.clone();
    assert_eq!(buf[(20, 7)].bg, t.card, "affiliate-site is the current fleet");
    assert_eq!(buf[(1, 7)].symbol(), "▌");
    assert_eq!(buf[(20, 9)].bg, t.bg);
    assert_eq!((buf[(4, 8)].symbol(), buf[(4, 8)].fg), ("1", t.blocked), "pixls needs you");
    assert_eq!(buf[(4, 9)].symbol(), "◆");
}

#[test]
fn the_top_border_search_field_and_footer_carry_the_designs_chips() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    let top = side(&rows, 0);
    assert!(
        top.contains(" « ") && top.contains(" + ") && top.contains(" inbox 1 "),
        "{top}"
    );
    assert!(top.find(" « ").unwrap() < top.find(" + ").unwrap());
    assert!(side(&rows, 2).contains("⌕ Filter sessions") && side(&rows, 2).contains(" / "), "{}", side(&rows, 2));
    let foot = side(&rows, 33);
    assert!(
        foot.contains(" + New ") && foot.contains(" Menu "),
        "{foot}"
    );
    assert!(side(&rows, 32).contains('─'), "a rule over the buttons");
    assert!(
        side(&rows, 31).contains("Maitty"),
        "the version, dim: {}",
        side(&rows, 31)
    );
}

#[test]
fn the_filter_matches_fleets_and_sessions_and_the_x_chip_and_esc_clear_it() {
    let mut app = reference_fixture::world(Stage::None);
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(1))));
    key(&mut app, KeyCode::Char('/'));
    for c in "pixls".chars() {
        key(&mut app, KeyCode::Char(c));
    }
    let rows = text(&mut app, 140, 36);
    show(&rows, "filtered");
    assert!(side(&rows, 2).contains("pixls") && side(&rows, 2).contains(" x "), "{}", side(&rows, 2));
    assert!(side(&rows, 7).contains("pixls") && !side(&rows, 8).contains("orbit"), "only the fleet: {}", side(&rows, 8));
    assert!(rows.iter().any(|r| side_of(r).contains("warm-finch")));
    assert!(!rows.iter().any(|r| side_of(r).contains("fix-footer")), "only the match");
    key(&mut app, KeyCode::Esc);
    let rows = text(&mut app, 140, 36);
    assert!(side(&rows, 2).contains("Filter sessions") && side(&rows, 2).contains(" / "));
    assert!(rows.iter().any(|r| side_of(r).contains("fix-footer")));
    // A session's folder matches too, and the x chip clears what is typed.
    app.screen.filter = "affiliate".into();
    let rows = text(&mut app, 140, 36);
    assert!(rows.iter().any(|r| side_of(r).contains("add-payments")));
    let x = side(&rows, 2).chars().collect::<Vec<_>>().windows(3).position(|w| w == [' ', 'x', ' ']).unwrap() as u16;
    click(&mut app, x + 1, 2);
    assert!(app.screen.filter.is_empty());
}

#[test]
fn the_default_width_is_46_and_a_dragged_one_stays_between_24_and_60() {
    let mut app = reference_fixture::world(Stage::None);
    cells(&mut app, 140, 36);
    assert_eq!(app.screen.side_rect.width, 46);
    app.screen.saved.side_w = Some(80);
    cells(&mut app, 140, 36);
    assert_eq!(app.screen.side_rect.width, 60);
    app.screen.saved.side_w = Some(10);
    cells(&mut app, 140, 36);
    assert_eq!(app.screen.side_rect.width, 24);
}

#[test]
fn clicking_a_ship_focuses_that_sessions_pane_wherever_the_daemon_holds_it() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    cells(&mut app, 140, 36);
    // warm-finch (3): its first line.
    let y = app.screen.row_y[&ClientTerm::local(3)];
    click(&mut app, 8, y);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|o| matches!(&o.msg, ClientMsg::Command(Command::FocusPane { term: 3 }))),
        "{} messages, none a focus on pane 3",
        sent.len()
    );
}

#[test]
fn prefix_dot_on_a_ship_opens_its_row_menu_and_anywhere_else_the_menu() {
    let mut app = reference_fixture::world(Stage::None);
    text(&mut app, 140, 36);
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(3))));
    app.act(Action::Menu);
    let Mode::ScreenMenu(menu) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    assert!(menu.title.contains("warm-finch"), "{}", menu.title);
    let mut app = reference_fixture::world(Stage::None);
    text(&mut app, 140, 36);
    app.act(Action::Menu);
    assert!(matches!(app.mode, Mode::Actions { .. }));
}

#[test]
fn the_plus_on_a_machine_heading_starts_a_shell_in_its_first_fleet() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    let rows = text(&mut app, 140, 36);
    let plus = side(&rows, 6).chars().collect::<Vec<_>>().iter().rposition(|c| *c == '+').unwrap() as u16;
    click(&mut app, plus, 6);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|o| matches!(&o.msg, ClientMsg::Command(Command::NewTab { ws: 10, cmd: None, .. }))),
        "{} messages",
        sent.len()
    );
    let flash = app.notice.as_ref().map(|(m, _, _)| m.clone()).unwrap_or_default();
    assert!(flash.starts_with("New shell in affiliate-site · "), "{flash}");
}

#[test]
fn a_machine_that_is_offline_gets_a_warning_and_no_shell() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    app.service_lost();
    app.new_ship_on(MachineId::LOCAL);
    assert!(std::iter::from_fn(|| rx.try_recv().ok()).next().is_none());
    assert!(app.notice.as_ref().is_some_and(|(m, _, err)| *err && m.contains("offline")));
}

// ---- the collapsed column ------------------------------------------------------------------

#[test]
fn below_110_columns_the_sidebar_is_the_ten_column_column_with_initials() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 100, 30);
    show(&rows, "100x30");
    let col = |y: usize| rows[y].chars().take(10).collect::<String>();
    assert_eq!(col(0), "┌── » ───┐");
    assert!(
        col(1).starts_with("│   1    │"),
        "the inbox count: {}",
        col(1)
    );
    assert_eq!(col(2), "├─ FLT ──┤");
    // The fleets: a needs count or a diamond, then initials.
    for (y, mark, initials) in [(3, "◆", "AS"), (4, "1", "PI"), (5, "◆", "OR")] {
        assert!(col(y).contains(mark) && col(y).contains(initials), "{}", col(y));
    }
    assert!(
        col(3).starts_with("│▌"),
        "the current fleet wears the bar: {}",
        col(3)
    );
    // The agent ships under SHP at the middle.
    let shp = (0..30).find(|y| col(*y).contains("SHP")).expect("the SHP divider");
    for (i, initials) in ["FF", "AP", "WF", "QF", "OM"].iter().enumerate() {
        assert!(col(shp + 1 + i).contains(initials), "{}", col(shp + 1 + i));
    }
    assert!(
        col(26).contains('+') && col(27).contains('≡'),
        "{} {}",
        col(26),
        col(27)
    );
}

#[test]
fn an_offline_machines_fleets_show_the_dashed_mark_in_the_column() {
    let mut app = reference_fixture::world(Stage::None);
    app.service_lost();
    let rows = text(&mut app, 100, 30);
    let col = |y: usize| rows[y].chars().take(10).collect::<String>();
    assert!(col(3).contains('┆') && col(4).contains('┆'), "{} {}", col(3), col(4));
}

#[test]
fn at_110_columns_and_up_it_is_the_full_sidebar() {
    let mut app = reference_fixture::world(Stage::None);
    assert!(text(&mut app, 109, 30)[0].starts_with("┌── » ───┐"));
    assert!(side(&text(&mut app, 110, 30), 4).contains(" FLEETS "));
}

#[test]
fn the_toggle_opens_the_full_sidebar_over_the_workspace_when_narrow_and_picking_closes_it() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, _rx) = mpsc::unbounded_channel();
    app.out = tx;
    text(&mut app, 100, 30);
    app.act(Action::ToggleSidebar);
    let rows = text(&mut app, 100, 30);
    show(&rows, "overlay 100x30");
    assert!(app.screen.overlay_open);
    assert!(
        side(&rows, 4).contains(" FLEETS "),
        "the full sidebar is over the workspace"
    );
    assert_eq!(app.screen.side_rect.width, 46);
    // Picking a ship puts it away again.
    let y = app.screen.row_y[&ClientTerm::local(3)];
    click(&mut app, 8, y);
    text(&mut app, 100, 30);
    assert!(!app.screen.overlay_open);
    // And the toggle closes it by itself.
    app.act(Action::ToggleSidebar);
    text(&mut app, 100, 30);
    assert!(app.screen.overlay_open);
    app.act(Action::ToggleSidebar);
    text(&mut app, 100, 30);
    assert!(!app.screen.overlay_open);
}

#[test]
fn the_toggle_collapses_and_expands_a_wide_screen() {
    let mut app = reference_fixture::world(Stage::None);
    text(&mut app, 140, 36);
    app.act(Action::ToggleSidebar);
    let rows = text(&mut app, 140, 36);
    assert!(rows[0].starts_with("┌── » ───┐"), "{}", rows[0]);
    app.act(Action::ToggleSidebar);
    assert!(side(&text(&mut app, 140, 36), 4).contains(" FLEETS "));
}

// ---- the status glyphs ---------------------------------------------------------------------

#[test]
fn by_default_every_state_is_a_dot_told_apart_by_colour() {
    let app = reference_fixture::world(Stage::None);
    let t = app.theme.clone();
    let marks: Vec<_> = [State::Needs, State::Done, State::Problem, State::Idle]
        .into_iter()
        .map(|s| app.state_mark(s, &t))
        .collect();
    assert!(marks.iter().all(|m| m.glyph == "●"));
    let colours: Vec<_> = marks.iter().map(|m| m.color).collect();
    assert_eq!(colours, [t.blocked, t.done, t.err, t.idle]);
    assert_eq!(app.state_mark(State::Unknown, &t).glyph, "┆");
}

#[test]
fn while_the_daemon_is_away_every_row_shows_the_unknown_mark() {
    let mut app = reference_fixture::world(Stage::None);
    let unknown_rows = |rows: &[String]| {
        (0..rows.len())
            .filter(|y| side(rows, *y).trim_start_matches(['│', ' ']).starts_with('┆'))
            .count()
    };
    let live = text(&mut app, 140, 30);
    assert_eq!(unknown_rows(&live), 0, "a live daemon has no unknown marks:\n{}", live.join("\n"));
    app.service_lost();
    let away = text(&mut app, 140, 30);
    show(&away, "the daemon is away");
    assert!(unknown_rows(&away) >= 2, "every session's mark is the dashed one:\n{}", away.join("\n"));
}

#[test]
fn the_unicode_setting_gives_each_state_a_shape_of_its_own() {
    let mut app = reference_fixture::world(Stage::None);
    app.cfg.ui.glyphs = "unicode".into();
    let t = app.theme.clone();
    let mut glyphs: Vec<String> = [
        State::Needs,
        State::Done,
        State::Problem,
        State::Idle,
        State::Unknown,
    ]
    .into_iter()
    .map(|s| app.state_mark(s, &t).glyph)
    .collect();
    glyphs.sort();
    glyphs.dedup();
    assert_eq!(glyphs.len(), 5, "{glyphs:?}");
}

#[test]
fn a_failed_command_is_the_problem_state_and_its_row_says_so_in_red() {
    let mut app = reference_fixture::world(Stage::None);
    app.snap.terms.get_mut(&5).unwrap().exit = Some(crate::protocol::ExitInfo {
        code: Some(1),
        at: 1,
    });
    let buf = cells(&mut app, 140, 50);
    let t = app.theme.clone();
    let y = app.screen.row_y[&ClientTerm::local(5)];
    assert_eq!(buf[(4, y)].symbol(), "●");
    assert_eq!(buf[(4, y)].fg, t.err);
}

#[test]
fn the_working_spinner_is_still_with_reduce_motion_and_turns_with_it() {
    let braille: Vec<String> = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".chars().map(String::from).collect();
    assert_eq!(status_mark::frame_at(&braille, 10_000, false), "⠋");
    assert_eq!(status_mark::frame_at(&braille, 80, true), "⠙");
}

// ---- the workspace -------------------------------------------------------------------------

#[test]
fn a_lone_pane_has_no_header_and_the_top_border_is_the_tab_row() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 200, 40);
    show(&rows, "lone pane");
    let top: String = rows[0].chars().skip(47).collect();
    assert!(top.starts_with("┌"), "{top}");
    // One tab per ship of the fleet: status glyph, title, ×; no app icon. Then + and the tools.
    for chip in ["fix-footer ×", "add-payments ×", " + ", " Files ", " Changes "] {
        assert!(top.contains(chip), "{chip} in {top}");
    }
    let (first, second, plus, files, changes) = (
        top.find("fix-footer").unwrap(),
        top.find("add-payments").unwrap(),
        top.find(" + ").unwrap(),
        top.find(" Files ").unwrap(),
        top.find(" Changes ").unwrap(),
    );
    assert!(first < second && second < plus && plus < files && files < changes, "{top}");
    assert!(top.trim_end().ends_with("┐"), "{top}");
    // The program starts on the row under the border: no header.
    let second_row: String = rows[1].chars().skip(47).collect();
    assert!(second_row.contains("rename the footer copyright"), "{second_row}");
}

#[test]
fn the_active_tab_is_the_accent_when_the_workspace_has_the_keys_and_the_chip_ground_when_not() {
    let mut app = reference_fixture::world(Stage::None);
    let t = app.theme.clone();
    let buf = cells(&mut app, 200, 40);
    let tab = app
        .hits
        .iter()
        .find_map(|(r, h)| matches!(h, Hit::Screen(ScreenHit::Tab(s)) if s.term == 1).then_some(*r))
        .expect("fix-footer's tab");
    assert_eq!(buf[(tab.x + 3, 0)].bg, t.accent);
    assert_eq!(buf[(tab.x + 3, 0)].fg, t.acc_ink);
    let other = app
        .hits
        .iter()
        .find_map(|(r, h)| matches!(h, Hit::Screen(ScreenHit::Tab(s)) if s.term == 2).then_some(*r))
        .expect("add-payments' tab");
    assert_eq!(buf[(other.x + 3, 0)].bg, t.btn);
    assert_eq!(buf[(other.x + 3, 0)].fg, t.text);
    // The sidebar has the keys: the workspace is not focused, so no tab is the accent.
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(1))));
    let buf = cells(&mut app, 200, 40);
    assert_eq!(buf[(tab.x + 3, 0)].bg, t.btn);
}

#[test]
fn clicking_a_tab_opens_it_the_cross_closes_it_and_right_click_opens_its_menu() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    cells(&mut app, 200, 40);
    let find = |app: &App, term: u32, close: bool| {
        app.hits.iter().find_map(|(r, h)| match h {
            Hit::Screen(ScreenHit::Tab(s)) if !close && s.term == term => Some(*r),
            Hit::Screen(ScreenHit::TabClose(s)) if close && s.term == term => Some(*r),
            _ => None,
        })
    };
    let tab = find(&app, 2, false).expect("a tab");
    click(&mut app, tab.x + 3, 0);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|o| matches!(&o.msg, ClientMsg::Command(Command::FocusPane { term: 2 }))),
        "it opens the ship"
    );
    // The × closes the session (with an undo), or asks again when an agent is running in it.
    let cross = find(&app, 2, true).expect("its ×");
    click(&mut app, cross.x + 1, 0);
    click(&mut app, cross.x + 1, 0);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter().any(|o| matches!(
            &o.msg,
            ClientMsg::Command(Command::CloseSession { term: 2 })
        )),
        "the second press closes the running agent: {} messages",
        sent.len()
    );
    app.mode = Mode::Normal;
    app.on_mouse(MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Right),
        column: tab.x + 3,
        row: 0,
        modifiers: KeyModifiers::NONE,
    });
    assert!(matches!(app.mode, Mode::ScreenMenu(_)), "the ship's menu");
}

#[test]
fn a_tab_with_two_or_more_panes_shows_the_count_and_too_many_tabs_overflow() {
    let mut app = split_world(3);
    let rows = text(&mut app, 200, 40);
    let top: String = rows[0].chars().skip(47).collect();
    assert!(top.contains("fix-footer 3 ×"), "{top}");
    // Six more ships in the fleet, on a narrow workspace: the active tab stays, the rest fold
    // into ` …N `.
    let mut app = reference_fixture::world(Stage::None);
    for id in 20..26u32 {
        let mut extra = app.snap.terms[&2].clone();
        extra.id = id;
        extra.label = format!("ship-{id}");
        app.snap.terms.insert(id, extra);
        let tab = id + 100;
        app.snap.workspaces[0].tabs.push(crate::protocol::TabInfo {
            id: tab,
            name: String::new(),
            layout: Node::Leaf(id),
            focus: id,
        });
    }
    app.screen_fresh();
    let rows = text(&mut app, 120, 30);
    show(&rows, "overflow");
    let top: String = rows[0].chars().skip(11).collect();
    assert!(top.contains("fix-footer") && top.contains(" …"), "{top}");
    assert!(top.contains(" + ") && top.contains(" Files "), "{top}");
}

#[test]
fn the_files_and_changes_chips_open_the_project_panels_and_light_while_open() {
    let mut app = reference_fixture::world(Stage::None);
    cells(&mut app, 140, 36);
    let find = |app: &App, f: fn(&ScreenHit) -> bool| {
        app.hits.iter().find_map(|(r, h)| match h {
            Hit::Screen(h) if f(h) => Some(*r),
            _ => None,
        })
    };
    let files = find(&app, |h| matches!(h, ScreenHit::FilesChip)).expect("the Files chip");
    assert_eq!(files.y, 0, "on the top border");
    let t = app.theme.clone();
    let buf = cells(&mut app, 140, 36);
    assert_eq!(buf[(files.x + 1, 0)].bg, t.btn, "quiet while closed");
    click(&mut app, files.x + 2, 0);
    assert!(matches!(app.view, Some(crate::client::View::Files(_))), "it opens Files");
    let buf = cells(&mut app, 140, 36);
    let files = find(&app, |h| matches!(h, ScreenHit::FilesChip)).expect("the Files chip");
    assert_eq!(buf[(files.x + 1, 0)].bg, t.accent, "the accent while open");
    click(&mut app, files.x + 2, 0);
    assert!(app.view.is_none(), "a second click closes it");
}

#[test]
fn the_project_tools_open_as_a_panel_over_the_workspace_sixty_columns_wide() {
    let mut app = reference_fixture::world(Stage::None);
    cells(&mut app, 140, 36);
    let panes_before = grid(&app, Rect::new(0, 0, 140, 36)).panes;
    app.act(Action::Branches);
    cells(&mut app, 140, 36);
    let g = grid(&app, Rect::new(0, 0, 140, 36));
    assert_eq!(g.sheet.width, 60);
    assert_eq!(
        g.sheet.right(),
        g.panes.right(),
        "over the right side of the workspace"
    );
    assert_eq!(g.panes, panes_before, "the workspace is not narrowed");
}

#[test]
fn a_split_is_one_frame_cut_by_single_lines_with_proper_joints() {
    let mut app = split_world(4);
    let rows = text(&mut app, 140, 36);
    show(&rows, "4 panes");
    let all: String = rows.concat();
    for joint in ['┬', '┴', '├', '┤', '┼'] {
        assert!(all.contains(joint), "{joint}");
    }
    // One outer frame: a single top left corner in the workspace column.
    let corners: usize = rows
        .iter()
        .map(|r| r.chars().skip(47).filter(|c| *c == '┌').count())
        .sum();
    assert_eq!(corners, 1);
}

#[test]
fn each_pane_of_a_split_has_a_two_row_header_with_its_buttons() {
    let mut app = split_world(2);
    let rows = text(&mut app, 140, 36);
    assert!(
        rows[1].contains('⋯') && rows[1].contains('×'),
        "{}",
        rows[1]
    );
    assert!(rows[1].contains("fix-footer") && rows[1].contains("add-payments"));
    assert!(
        rows[2].contains('⎇'),
        "the path and the branch: {}",
        rows[2]
    );
    // Each is under 60 columns wide: no split or zoom buttons yet.
    assert!(!rows[1].contains('⊞'));
    let mut app = split_world(2);
    let wide = text(&mut app, 200, 40);
    assert!(
        wide[1].contains('⊞') && wide[1].contains('⊟') && wide[1].contains('⤢'),
        "{}",
        wide[1]
    );
}

#[test]
fn exactly_one_pane_has_the_accent_border() {
    let mut app = split_world(4);
    let buf = cells(&mut app, 140, 36);
    let t = app.theme.clone();
    let line = |c: &ratatui::buffer::Cell| "│─┌┐└┘├┤┬┴┼".contains(c.symbol());
    let area = Rect::new(47, 0, 93, 35);
    let accent: Vec<(u16, u16)> = area
        .positions()
        .filter(|p| line(&buf[(p.x, p.y)]) && buf[(p.x, p.y)].fg == t.accent)
        .map(|p| (p.x, p.y))
        .collect();
    assert!(!accent.is_empty(), "the focused pane's lines");
    // The focused pane is the top left one: nothing past the middle is accent, but the cross.
    let (mid_x, mid_y) = (area.x + area.width / 2 + 2, area.height / 2 + 2);
    assert!(
        accent.iter().all(|(x, y)| *x < mid_x && *y < mid_y),
        "{accent:?}"
    );
    assert_ne!(buf[(area.right() - 1, area.bottom() - 1)].fg, t.accent);
    // Focus in the sidebar: the workspace has none, the sidebar frame is the accent one.
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(1))));
    let buf = cells(&mut app, 140, 36);
    assert!(
        area.positions()
            .all(|p| !line(&buf[(p.x, p.y)]) || buf[(p.x, p.y)].fg != t.accent)
    );
    assert_eq!(buf[(0, 0)].fg, t.accent);
}

#[test]
fn a_view_holds_four_panes_and_the_split_commands_say_which_side_the_new_one_goes() {
    assert_eq!(split_dir(&Action::SplitUp), Some(Dir::Up));
    assert_eq!(split_dir(&Action::SplitDown), Some(Dir::Down));
    assert_eq!(split_dir(&Action::SplitLeft), Some(Dir::Left));
    assert_eq!(split_dir(&Action::SplitRight), Some(Dir::Right));
    // Up and left insert before the pane, down and right after it.
    let mut node = Node::Leaf(1);
    node.split(1, Dir::Left, 2);
    assert_eq!(node.leaves(), [2, 1]);
    let mut node = Node::Leaf(1);
    node.split(1, Dir::Right, 2);
    assert_eq!(node.leaves(), [1, 2]);

    let mut app = split_world(4);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    text(&mut app, 140, 36);
    app.act(Action::SplitRight);
    assert!(
        std::iter::from_fn(|| rx.try_recv().ok()).next().is_none(),
        "no fifth pane is asked for"
    );
    assert!(
        app.notice
            .as_ref()
            .is_some_and(|(m, _, _)| m.contains("4 panes"))
    );
}

#[test]
fn the_row_menu_lists_what_the_design_does() {
    let mut app = reference_fixture::world(Stage::None);
    text(&mut app, 140, 36);
    // Not the open session, so it can open in a split.
    let (_, items) = app.session_items(ClientTerm::local(3)).expect("a menu");
    let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(
        labels,
        [
            "Rename",
            "Pin",
            "Open in split right",
            "Open in split down",
            "Copy path",
            "Close session"
        ]
    );
}

#[test]
fn the_status_bar_says_where_the_keys_go_and_who_needs_you() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    let bar = &rows[35];
    assert!(
        bar.contains("TYPING") && bar.contains("keys go to"),
        "{bar}"
    );
    assert!(bar.contains("needs you") && bar.contains("CPU"), "{bar}");
}
