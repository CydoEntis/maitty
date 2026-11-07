//! The flat sidebar, the collapsed column, the status glyphs and the workspace frames, held to
//! the TUI design (v26). `MAITTY_SHOW=1 cargo test <name> -- --nocapture` prints the frames.
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
fn the_sidebar_is_one_flat_list_two_lines_a_session_in_creation_order() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    show(&rows, "140x36");
    // The names in id order, two rows apart, each over its project and machine.
    let names = [
        "fix-footer",
        "add-payments",
        "warm-finch",
        "quick-fox",
        "old-moth",
    ];
    let projects = [
        "affiliate-site",
        "affiliate-site",
        "pixls",
        "orbit",
        "orbit",
    ];
    for (i, (name, project)) in names.iter().zip(projects).enumerate() {
        let y = 2 + 2 * i;
        assert!(side(&rows, y).contains(name), "row {y}: {}", side(&rows, y));
        let second = side(&rows, y + 1);
        assert!(
            second.contains(project) && second.trim_end().ends_with("local │"),
            "{second}"
        );
    }
    // No group headers: nothing but the sessions between the filter and the footer.
    assert!(
        !rows
            .iter()
            .any(|r| r.contains("LOCAL") || r.contains("REMOTE"))
    );
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
fn diff_counts_sit_at_the_right_of_the_first_line_green_and_red() {
    let mut app = reference_fixture::world(Stage::None);
    let info = app.snap.terms.get_mut(&1).unwrap();
    (info.diff_added, info.diff_removed) = (104, 61);
    let buf = cells(&mut app, 140, 36);
    let line: String = (0..46).map(|x| buf[(x, 2)].symbol()).collect();
    assert!(line.contains("+104 −61"), "{line}");
    let t = app.theme.clone();
    let plus = line.chars().position(|c| c == '+').unwrap() as u16;
    assert_eq!(buf[(plus, 2)].fg, t.done, "+N in the done green");
    let minus = line.chars().position(|c| c == '−').unwrap() as u16;
    assert_eq!(buf[(minus, 2)].fg, t.err, "−N in the danger red");
    assert!(line.trim_end().ends_with("−61 │"), "right-aligned: {line}");
}

#[test]
fn idle_names_are_dimmed_and_the_open_session_has_the_raised_ground() {
    let mut app = reference_fixture::world(Stage::None);
    let buf = cells(&mut app, 140, 36);
    let t = app.theme.clone();
    // old-moth (5) is idle: its name is muted, not the strong ink.
    assert_eq!(buf[(4, 10)].fg, t.muted);
    assert_ne!(
        buf[(4, 2)].fg,
        t.muted,
        "a session with work keeps the bright name"
    );
    // fix-footer (1) is the one open in the workspace: raised ground on both lines.
    assert_eq!(buf[(10, 2)].bg, t.card);
    assert_eq!(buf[(10, 3)].bg, t.card);
    assert_eq!(buf[(10, 4)].bg, t.bg, "the others sit on the ground");
}

#[test]
fn the_top_border_and_footer_carry_the_designs_chips() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 140, 36);
    let top = side(&rows, 0);
    assert!(
        top.contains(" SESSIONS ") && top.contains(" inbox 1 ") && top.contains(" « "),
        "{top}"
    );
    assert!(side(&rows, 1).contains("/ filter"));
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
fn the_filter_matches_name_folder_process_and_machine_and_esc_clears_it() {
    let mut app = reference_fixture::world(Stage::None);
    app.screen_side_set(SideItem::Sess(app.session_key(ClientTerm::local(1))));
    key(&mut app, KeyCode::Char('/'));
    for c in "pixls".chars() {
        key(&mut app, KeyCode::Char(c));
    }
    let rows = text(&mut app, 140, 36);
    assert!(side(&rows, 1).contains("/ pixls"), "{}", side(&rows, 1));
    assert!(side(&rows, 2).contains("warm-finch"));
    assert!(
        !side(&rows, 4).contains("fix-footer"),
        "only the match: {}",
        side(&rows, 4)
    );
    key(&mut app, KeyCode::Esc);
    let rows = text(&mut app, 140, 36);
    assert!(side(&rows, 1).contains("/ filter") && side(&rows, 2).contains("fix-footer"));
    // The project on a row filters to that project when clicked.
    click(&mut app, 5, 3);
    assert_eq!(app.screen.filter, "affiliate-site");
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
fn clicking_a_row_focuses_that_sessions_pane_wherever_the_daemon_holds_it() {
    let mut app = reference_fixture::world(Stage::None);
    let (tx, mut rx) = mpsc::unbounded_channel();
    app.out = tx;
    cells(&mut app, 140, 36);
    // warm-finch (3) is the third row: its first line.
    click(&mut app, 8, 6);
    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
    assert!(
        sent.iter()
            .any(|o| matches!(&o.msg, ClientMsg::Command(Command::FocusPane { term: 3 }))),
        "{} messages, none a focus on pane 3",
        sent.len()
    );
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
    assert_eq!(col(2), "├────────┤");
    for (y, initials) in [(3, "FF"), (4, "AP"), (5, "WF"), (6, "QF"), (7, "OM")] {
        assert!(col(y).contains(initials), "{}", col(y));
    }
    assert!(
        col(3).starts_with("│▌"),
        "the current session wears the bar: {}",
        col(3)
    );
    assert!(
        col(26).contains('+') && col(27).contains('≡'),
        "{} {}",
        col(26),
        col(27)
    );
}

#[test]
fn at_110_columns_and_up_it_is_the_full_sidebar() {
    let mut app = reference_fixture::world(Stage::None);
    assert!(text(&mut app, 109, 30)[0].starts_with("┌── » ───┐"));
    assert!(text(&mut app, 110, 30)[0].contains(" SESSIONS "));
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
        rows[0].contains(" SESSIONS "),
        "the full sidebar is over the workspace"
    );
    assert_eq!(app.screen.side_rect.width, 46);
    // Picking a session puts it away again.
    click(&mut app, 8, 6);
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
    assert!(text(&mut app, 140, 36)[0].contains(" SESSIONS "));
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
    let buf = cells(&mut app, 140, 36);
    let t = app.theme.clone();
    assert_eq!(buf[(2, 10)].symbol(), "●");
    assert_eq!(buf[(2, 10)].fg, t.err);
}

#[test]
fn the_working_spinner_is_still_with_reduce_motion_and_turns_with_it() {
    let braille: Vec<String> = "⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏".chars().map(String::from).collect();
    assert_eq!(status_mark::frame_at(&braille, 10_000, false), "⠋");
    assert_eq!(status_mark::frame_at(&braille, 80, true), "⠙");
}

// ---- the workspace -------------------------------------------------------------------------

#[test]
fn a_lone_pane_has_no_header_and_wears_its_status_title_and_chips_on_the_border() {
    let mut app = reference_fixture::world(Stage::None);
    let rows = text(&mut app, 200, 40);
    show(&rows, "lone pane");
    let top: String = rows[0].chars().skip(47).collect();
    assert!(top.starts_with("┌ "), "{top}");
    for chip in ["fix-footer", "▾ Project", "▢ ", "⎇ fix-footer", "⌂ This"] {
        assert!(top.contains(chip), "{chip} in {top}");
    }
    let (project, branch, machine, close) = (
        top.find("▾ Project").unwrap(),
        top.find('⎇').unwrap(),
        top.find('⌂').unwrap(),
        top.find('×').unwrap(),
    );
    assert!(
        project < branch && branch < machine && machine < close,
        "{top}"
    );
    assert!(top.trim_end().ends_with("× ┐"), "{top}");
    // The program starts on the row under the border: no header, no tab row.
    let second: String = rows[1].chars().skip(47).collect();
    assert!(second.contains("rename the footer copyright"), "{second}");
}

#[test]
fn the_path_chips_drop_first_when_the_border_is_short() {
    let mut app = reference_fixture::world(Stage::None);
    let wide: String = text(&mut app, 200, 40)[0].chars().skip(47).collect();
    assert!(wide.contains('▢'));
    let mut app = reference_fixture::world(Stage::None);
    let mid: String = text(&mut app, 140, 36)[0].chars().skip(47).collect();
    assert!(
        !mid.contains('▢') && mid.contains('⎇'),
        "the folder goes before the branch: {mid}"
    );
    let mut app = reference_fixture::world(Stage::None);
    let narrow: String = text(&mut app, 80, 24)[0].chars().skip(11).collect();
    assert!(!narrow.contains('▢'), "{narrow}");
    assert!(
        narrow.contains("▾ Project") && narrow.contains('×'),
        "{narrow}"
    );
}

#[test]
fn the_red_close_sits_on_the_border_and_the_project_chip_opens_the_tools() {
    let mut app = reference_fixture::world(Stage::None);
    cells(&mut app, 140, 36);
    let find = |app: &App, f: fn(&ScreenHit) -> bool| {
        app.hits.iter().find_map(|(r, h)| match h {
            Hit::Screen(h) if f(h) => Some(*r),
            _ => None,
        })
    };
    let close = find(&app, |h| matches!(h, ScreenHit::CloseSplit(_))).expect("the close chip");
    assert_eq!(close.y, 0);
    let project = find(&app, |h| matches!(h, ScreenHit::ProjectMenu(_))).expect("the Project chip");
    click(&mut app, project.x + 2, 0);
    let Mode::ScreenMenu(menu) = &app.mode else {
        panic!("{:?}", app.mode)
    };
    let labels: Vec<&str> = menu.items.iter().map(|(l, _)| l.as_str()).collect();
    assert_eq!(
        &labels[..5],
        ["Files", "Changes", "Branches", "History", "References"]
    );
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
