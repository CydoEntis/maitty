//! The client's tests: rendering through TestBackend, keys and clicks.

use super::*;

mod basics {
    use super::*;
    use ratatui::style::Color;

    #[test]
    fn blend_mixes_rgb() {
        let c = render::blend(Color::Rgb(0, 0, 0), Color::Rgb(200, 100, 50), 0.1);
        assert_eq!(c, Color::Rgb(20, 10, 5));
        assert_eq!(render::blend(Color::Indexed(4), Color::Rgb(1, 2, 3), 0.1), Color::Indexed(4));
    }

}

mod design_tests {
    use super::*;
    use crate::layout::{Dir, Node};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn entry(path: &str, branch: &str, main: bool) -> WorktreeEntry {
        WorktreeEntry { path: PathBuf::from(path), branch: branch.into(), main, repo: "shop-api".into() }
    }

    fn term(id: TermId, agent: Option<&str>, status: Status, cwd: &str) -> TermInfo {
        TermInfo {
            status_why: String::new(),
            why_hook: String::new(),
            why_hook_at: 0,
            why_screen: String::new(),
            why_screen_at: 0,
            why_process_at: 0,
            pid: 0,
            name: String::new(),
            label: String::new(),
            model: String::new(),
            mem: 0,
            cpu: 0.0,
            bell: false,
            id,
            cols: 80,
            rows: 20,
            title: String::new(),
            process: if agent.is_some() { "node".into() } else { "pwsh".into() },
            agent: agent.map(String::from),
            status,
            cwd: PathBuf::from(cwd),
            summary: String::new(),
            said: String::new(),
            branch: None,
            linked: false,
            subagents: Vec::new(),
            subagent_activity: Vec::new(),
            session: None,
            started_by: None,
            queued: Vec::new(),
            root: None,
            top: None,
            since: 0,
            asleep: false,
            win32_input: false,
            remote: None,
            popup: false,
            usage: Default::default(),
            resume_at: None,
        }
    }

    /// The main screen with a few projects and agents, rendered as text.
    pub(super) fn render_with(w: u16, h: u16) -> (String, App) {
        let (tx, _rx) = mpsc::unbounded_channel();
        let (bg_tx, _bg_rx) = mpsc::unbounded_channel();
        let cfg = Config::default();
        let mut app = App::new(cfg, tx, None, bg_tx);
        app.splash = false;
        let layout = Node::Leaf(1);
        // A project folder in this platform's own style (tests run on Windows and Linux).
        let base = std::path::Path::new(if cfg!(windows) { r"C:\code" } else { "/code" });
        let root_buf = base.join("shop-api");
        let rate_buf = root_buf.join(".wt").join("rate");
        let orders_buf = root_buf.join(".wt").join("orders");
        let root = root_buf.to_str().unwrap();
        app.snap.workspaces.push(WorkspaceInfo {
            id: 10,
            name: "shop-api".into(),
            cwd: PathBuf::from(root),
            tabs: vec![
                TabInfo { id: 11, name: String::new(), layout, focus: 1 },
                TabInfo { id: 12, name: String::new(), layout: Node::Leaf(2), focus: 2 },
                TabInfo { id: 13, name: String::new(), layout: Node::Leaf(3), focus: 3 },
            ],
            active_tab: 11,
            git: Some(GitInfo {
                repo: "shop-api".into(),
                branch: "main".into(),
                dirty: 2,
                linked: false,
                root: PathBuf::from(root),
                worktrees: vec![
                    entry(root, "main", true),
                    entry(rate_buf.to_str().unwrap(), "rate-limit", false),
                    entry(orders_buf.to_str().unwrap(), "orders-migration", false),
                ],
                ahead: 0,
            }),
            worktree: false,
            color: 0,
            is_new: false,
            group: None,
        });
        app.snap.active_ws = Some(10);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let rate = rate_buf.to_str().unwrap();
        let mut claude = term(1, Some("claude"), Status::Blocked, root);
        claude.branch = Some("main".into());
        claude.subagents = vec!["Explore".into(), "planner".into(), "builder".into(), "reviewer".into()];
        claude.summary = "Fix flaky checkout test".into();
        claude.root = Some(PathBuf::from(root));
        claude.top = Some(PathBuf::from(root));
        claude.since = now - 180;
        let mut codex = term(2, Some("codex"), Status::Working, rate);
        codex.branch = Some("rate-limit".into());
        codex.linked = true;
        codex.summary = "Rate limit /login".into();
        codex.root = Some(PathBuf::from(root));
        codex.top = Some(PathBuf::from(rate));
        codex.since = now - 120;
        let mut shell = term(3, None, Status::None, root);
        shell.root = Some(PathBuf::from(root));
        shell.top = Some(PathBuf::from(root));
        shell.branch = Some("main".into());
        app.snap.terms.insert(1, claude);
        app.snap.terms.insert(2, codex);
        app.snap.terms.insert(3, shell);
        app.screen_sync();
        let mut p = vt100::Parser::new(20, 80, 0);
        p.process(b"> fix the flaky checkout test\r\n\r\nRun npm test -- checkout?\r\n\x1b[1m\xe2\x9d\xaf 1. Yes\x1b[0m\r\n  2. Yes, and always allow\r\n  3. No");
        app.parsers.insert(1, p);
        app.parsers.insert(2, vt100::Parser::new(20, 80, 0));
        app.parsers.insert(3, vt100::Parser::new(20, 80, 0));
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut out = String::new();
        for y in 0..h {
            let mut line = String::new();
            for x in 0..w {
                line.push_str(buf[(x, y)].symbol());
            }
            out.push_str(line.trim_end());
            out.push('\n');
        }
        (out, app)
    }

    #[test]
    fn sync_restores_nested_split_layout_from_daemon_snapshot() {
        let (_, mut app) = render_with(160, 45);
        let mut layout = Node::Leaf(1);
        layout.split(1, Dir::Right, 2);
        layout.split(2, Dir::Down, 3);
        app.snap.workspaces[0].tabs = vec![TabInfo { id: 11, name: String::new(), layout, focus: 1 }];
        app.screen.tabs.clear();
        app.screen.last_focus = None;
        app.screen_sync();
        let tab = &app.screen.tabs[app.screen.tab];
        assert_eq!(tab.layout.leaves(), vec![1, 2, 3], "the UI must retain all restored panes");
        let rects = tab.layout.rects(ratatui::layout::Rect::new(0, 0, 120, 40));
        let rect = |id| rects.iter().find(|(term, _)| *term == id).unwrap().1;
        assert_eq!(rect(3).x, rect(2).x, "split-down stays in pane 2's column");
        assert!(rect(3).y > rect(2).y, "split-down places pane 3 below pane 2");
        let mut terminal = Terminal::new(TestBackend::new(160, 45)).unwrap();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        let rendered = |id| app.screen.leaf_rects.iter().find(|(term, _)| *term == id).unwrap().1;
        assert_eq!(rendered(3).x, rendered(2).x, "the screen renderer preserves the nested column");
        assert!(rendered(3).y > rendered(2).y, "the screen renderer places the pane below its sibling");
    }
}

mod screen_tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect()
    }

    fn show(s: &str) {
        if std::env::var("SESHI_SHOW").is_ok() {
            println!("{s}");
        }
    }

    #[test]
    fn all_keys_lists_aliases_globals_and_custom_bindings_without_disabled_keys() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let rows = shortcuts::entries(&app);
        for (key, action) in &app.keymap.prefixed {
            assert!(rows.contains(&("After leader".into(), key.to_string(), action.describe())), "leader {key} is discoverable");
        }
        for (key, action) in &app.keymap.global {
            assert!(rows.contains(&("Without leader".into(), key.to_string(), action.describe())), "global {key} is discoverable");
        }
        for alias in ["p", "v", "|", ":", "Space", "C-v"] {
            assert!(rows.iter().any(|(scope, key, _)| scope == "After leader" && key == alias), "alias {alias} is not hidden");
        }
        app.cfg.prefix = "ctrl+a".into();
        app.cfg.keys.prefix.insert("v".into(), "none".into());
        app.cfg.keys.prefix.insert("g".into(), "workflows".into());
        app.cfg.keys.global.insert("alt+0".into(), "spawn-tab:my-agent --review".into());
        app.keymap = app.cfg.keymap();
        let rows = shortcuts::entries(&app);
        assert_eq!(rows[0].1, "C-a");
        assert!(!rows.iter().any(|(scope, key, _)| scope == "After leader" && key == "v"));
        assert!(rows.iter().any(|(scope, key, label)| scope == "After leader" && key == "g" && label == "Command palette"));
        assert!(rows.iter().any(|(scope, key, label)| scope == "Without leader" && key == "M-0" && label.contains("my-agent --review")));
        app.cfg.keys.replace_defaults = true;
        app.keymap = app.cfg.keymap();
        assert!(!shortcuts::entries(&app).iter().any(|(scope, key, _)| scope == "After leader" && key == "j"));
    }

    #[test]
    fn all_keys_opens_searches_scrolls_and_returns_without_running_commands() {
        for (w, h) in [(80, 24), (100, 30), (160, 45)] {
            let (_, mut app) = super::design_tests::render_with(w, h);
            let (tx, mut rx) = mpsc::unbounded_channel(); app.out = tx;
            app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
            app.on_key(KeyEvent::new(KeyCode::Char('?'), KeyModifiers::NONE));
            assert!(matches!(app.mode, Mode::Shortcuts { .. }));
            let output = draw(&mut app, w, h); show(&output);
            assert!(output.contains("All shortcuts") && output.contains("AFTER LEADER") && output.contains("Esc"));
            app.on_key(KeyEvent::new(KeyCode::End, KeyModifiers::NONE));
            let output = draw(&mut app, w, h);
            assert!(output.contains("Complete current"), "the end of the reference is reachable at {w} columns");
            app.on_key(KeyEvent::new(KeyCode::Home, KeyModifiers::NONE));
            draw(&mut app, w, h);
            app.on_mouse(MouseEvent { kind: MouseEventKind::ScrollDown, column: w / 2, row: h / 2, modifiers: KeyModifiers::NONE });
            assert!(matches!(app.mode, Mode::Shortcuts { scroll: 1, .. }));
            for (query, expected) in [("settings", "SETTINGS"), ("sidebar", "SIDEBAR"),
                ("copy mode", "COPY MODE"), ("without leader", "WITHOUT LEADER"),
                ("Ctrl+Space", "START HERE"), ("Alt+1", "WITHOUT LEADER"), ("no match zzqq", "No shortcuts match")] {
                app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
                app.on_paste(query.into());
                let output = draw(&mut app, w, h); show(&output);
                assert!(output.contains(expected));
            }
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert_eq!(app.mode, Mode::Normal);
            assert!(rx.try_recv().is_err(), "help must not run or send any command");
            app.mode = Mode::Actions { sel: 0 };
            app.actions_run(screen::action_items().iter().position(|(_, a)| *a == Action::Help).unwrap());
            assert!(matches!(app.mode, Mode::Shortcuts { .. }), "Actions → All keys reaches the full reference too");
        }
    }

    #[test]
    fn all_keys_renders_bindings_as_keycaps_separate_from_actions() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        app.open_shortcuts();
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).unwrap();
        terminal.draw(|frame| render::draw(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let keycap = (0..30).any(|y| {
            (0..100).any(|x| {
                let cell = &buffer[(x, y)];
                cell.symbol() == "j" && cell.bg == app.theme.btn
            })
        });
        assert!(keycap, "the Inbox binding should be visually rendered on a raised keycap");
    }

    #[test]
    fn all_keys_places_shortcut_sections_in_three_columns_on_wide_screens() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.open_shortcuts();
        let mut terminal = Terminal::new(TestBackend::new(160, 45)).unwrap();
        terminal.draw(|frame| render::draw(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let output = (0..45)
            .map(|y| {
                (0..160)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        show(&output);
        let heading_x = |heading: &str| {
            (0..45).find_map(|y| {
                let row = (0..160)
                    .map(|x| buffer[(x, y)].symbol())
                    .collect::<String>();
                row.find(heading)
            })
        };
        let positions = ["START HERE", "AFTER LEADER", "WITHOUT LEADER"]
            .map(|heading| heading_x(heading).expect("configured bindings are visible"));
        assert_ne!(positions[0], positions[1], "sections should occupy separate columns");
        assert_ne!(positions[1], positions[2], "sections should occupy separate columns");
        assert_ne!(positions[0], positions[2], "sections should occupy separate columns");
    }

    #[test]
    fn collapsed_sidebar_keeps_status_focus_mouse_access_and_overflow_attention() {
        for (w, h) in [(80, 24), (100, 30), (160, 45)] {
            for position in ["left", "right"] {
                let (_, mut app) = super::design_tests::render_with(w, h);
                app.cfg.ui.sidebar_position = position.into();
                app.sidebar = false;
                app.snap.terms.get_mut(&2).unwrap().label = "Review payments".into();
                app.screen_fresh();
                app.cfg.ui.motion = false;
                app.screen_focus(1);
                app.mode = Mode::Normal;
                let (tx, mut rx) = mpsc::unbounded_channel(); app.out = tx;
                let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
                terminal.draw(|f| render::draw(&mut app, f)).unwrap();
                let rail = app.screen.side_rect;
                assert_eq!(rail.width, 7);
                assert_eq!(app.screen.row_y.len(), 3);
                let y = app.screen.row_y[&1];
                let hover_y = app.screen.row_y[&2];
                let active_bg = terminal.backend().buffer()[(rail.x + 3, y)].bg;
                assert_eq!(terminal.backend().buffer()[(rail.x + 1, y)].fg, app.theme.blocked);
                assert_eq!(terminal.backend().buffer()[(rail.x + 3, y)].fg, app.theme.blocked);
                app.on_mouse(MouseEvent { kind: MouseEventKind::Moved, column: rail.x + 3, row: hover_y, modifiers: KeyModifiers::NONE });
                terminal.draw(|f| render::draw(&mut app, f)).unwrap();
                assert_eq!(terminal.backend().buffer()[(rail.x + 3, y)].bg, active_bg);
                assert_ne!(terminal.backend().buffer()[(rail.x + 3, hover_y)].bg, active_bg);
                assert_eq!(app.focused(), Some(1));
                let screen = draw(&mut app, w, h); show(&screen);
                assert!(screen.contains("Review payments"), "hover reveals the full name");
                for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
                    app.on_mouse(MouseEvent { kind, column: rail.x + 3, row: hover_y, modifiers: KeyModifiers::NONE });
                }
                assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, ClientMsg::Command(Command::FocusPane { term: 2 }))));
                // More sessions than fit: the blocked total remains visible off screen.
                for id in 4..40 { let mut t = app.snap.terms[&2].clone(); t.id = id; app.snap.terms.insert(id, t);
                    app.snap.workspaces[0].tabs.push(TabInfo { id: 100 + id, name: String::new(), layout: crate::layout::Node::Leaf(id), focus: id });
                }
                app.screen.side_scroll = 30; app.screen.follow = false;
                let screen = draw(&mut app, w, h);
                assert!(screen.contains("\u{25cf}1"));
                assert!(!app.screen.row_y.contains_key(&1));
                let (expand, _) = app.hits.iter().find(|(_, h)| *h == Hit::Screen(screen::ScreenHit::ExpandSidebar)).unwrap();
                app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: expand.x, row: expand.y, modifiers: KeyModifiers::NONE });
                assert!(app.sidebar);
                draw(&mut app, w, h);
                assert!(app.screen.side_rect.width > rail.width);
            }
        }
    }

    #[test]
    fn active_sidebar_selection_survives_hover_and_keyboard_browsing() {
        let (_, mut app) = super::design_tests::render_with(120, 30);
        let (tx, mut rx) = mpsc::unbounded_channel(); app.out = tx;
        app.screen_focus(1);
        app.mode = Mode::Normal;
        let mut terminal = Terminal::new(TestBackend::new(120, 30)).unwrap();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        let x = app.screen.side_rect.x;
        let active_y = app.screen.row_y[&1];
        let hover_y = app.screen.row_y[&2];
        let active_bg = terminal.backend().buffer()[(x + 8, active_y)].bg;
        for (column, row) in [(x + 10, hover_y), (x + 4, 3), (x + 4, 27), (80, 20)] {
            app.on_mouse(MouseEvent { kind: MouseEventKind::Moved, column, row, modifiers: KeyModifiers::NONE });
            terminal.draw(|f| render::draw(&mut app, f)).unwrap();
            assert_eq!(app.focused(), Some(1), "hover never changes the active pane");
            assert_eq!(terminal.backend().buffer()[(x + 8, active_y)].bg, active_bg);
            assert_eq!(terminal.backend().buffer()[(x + 1, active_y)].symbol(), "▎");
        }
        app.on_mouse(MouseEvent { kind: MouseEventKind::Moved, column: x + 10, row: hover_y, modifiers: KeyModifiers::NONE });
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        assert_ne!(terminal.backend().buffer()[(x + 8, hover_y)].bg, active_bg, "hover has its own quieter style");
        app.screen_side_set(screen::SideItem::Sess(2));
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = terminal.backend().buffer();
        assert_eq!(buf[(x + 1, active_y)].symbol(), "▎", "the open row keeps a bar");
        assert_eq!(buf[(x + 1, hover_y)].symbol(), "▎", "and so does the one under the keyboard");
        assert_eq!((buf[(x + 1, hover_y)].fg, buf[(x + 1, active_y)].fg), (app.theme.accent, app.theme.muted), "lime for the cursor, dim for the open one");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: x + 10, row: hover_y, modifiers: KeyModifiers::NONE });
        app.on_mouse(MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column: x + 10, row: hover_y, modifiers: KeyModifiers::NONE });
        assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, ClientMsg::Command(Command::FocusPane { term: 2 }))));
        app.snap.workspaces[0].tabs[0].focus = 2;
        app.screen_sync();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        assert_eq!(app.focused(), Some(2));
        assert_eq!(terminal.backend().buffer()[(x + 1, hover_y)].symbol(), "▎");
        app.snap.terms.get_mut(&2).unwrap().asleep = true;
        app.screen_fresh();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        assert_eq!(terminal.backend().buffer()[(x + 1, hover_y)].symbol(), "▎", "sleep glyphs leave the selection bar visible");
    }

    #[test]
    fn installing_an_update_keeps_the_window_and_sessions_running() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, mut rx) = mpsc::unbounded_channel(); app.out = tx;
        let focused = app.focused();
        app.got_state = true;
        app.snap.daemon_version = "0.36.11".into();
        app.updating = true;
        app.update_available = Some("9.9.9".into());
        app.update_installed("9.9.9".into());
        assert!(!app.updating && app.update_available.is_none());
        assert_eq!(app.installed_version, Some(Ok("9.9.9".into())));
        assert!(app.quit.is_none() && app.restart_pending());
        assert_eq!(app.focused(), focused);
        assert!(rx.try_recv().is_err(), "install completion sends no session commands");
        assert!(app.notice.as_ref().is_some_and(|(text, ..)| text.contains("restart automatically when no sessions are open")));
    }

    #[test]
    fn installing_an_update_restarts_the_daemon_when_no_sessions_are_open() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.got_state = true;
        app.snap.terms.clear();
        app.snap.daemon_version = "0.36.11".into();
        app.update_installed("0.36.12".into());

        assert!(matches!(rx.try_recv(), Ok(ClientMsg::Command(Command::KillServer { forget: false }))));
    }

    #[test]
    fn daemon_update_waits_until_the_last_session_closes() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.installed_version = Some(Ok("0.36.12".into()));
        app.snap.daemon_version = "0.36.11".into();
        assert!(rx.try_recv().is_err(), "open sessions must not be interrupted");

        let mut snapshot = app.snap.clone();
        snapshot.terms.clear();
        app.on_server(ServerMsg::State(snapshot));

        assert!(matches!(rx.try_recv(), Ok(ClientMsg::Command(Command::KillServer { forget: false }))));
    }

    #[test]
    fn a_matching_daemon_version_completes_the_automatic_restart() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.installed_version = Some(Ok(env!("CARGO_PKG_VERSION").into()));
        app.daemon_restart_requested = true;
        app.daemon_restart_reconnect = true;
        let mut snapshot = app.snap.clone();
        snapshot.daemon_version = env!("CARGO_PKG_VERSION").into();

        app.on_server(ServerMsg::State(snapshot));

        assert!(!app.daemon_restart_requested && !app.daemon_restart_reconnect);
        assert!(rx.try_recv().is_err(), "a current daemon must not be restarted again");
        assert!(app.notice.as_ref().is_some_and(|(text, ..)| text.contains("daemon restarted with the update")));
    }
    #[test]
    fn planned_daemon_restart_keeps_the_window_open_for_reconnect() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        app.daemon_restart_requested = true;

        app.on_server(ServerMsg::Bye);

        assert!(app.quit.is_none());
        assert!(app.daemon_restart_reconnect);
    }

    #[test]
    fn versions_distinguish_installed_window_and_running_daemon() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        app.installed_version = Some(Ok("9.9.9".into()));
        app.snap.daemon_version = "0.20.0".into();
        app.act(Action::Settings);
        let output = draw(&mut app, 100, 30); show(&output);
        assert!(output.contains("Installed here: 9.9.9") && output.contains("This window:"));
        assert!(output.contains("Running daemon: 0.20.0") && output.contains("Restart pending"));
        app.update_available = Some("9.9.9".into());
        app.update_checked_now(true, Ok(None), Vec::new());
        assert!(app.update_available.is_none(), "a release already installed is not offered again");
        assert!(app.restart_pending());
        app.installed_version = Some(Ok(env!("CARGO_PKG_VERSION").into()));
        app.snap.daemon_version = env!("CARGO_PKG_VERSION").into();
        assert!(!app.restart_pending());
        app.installed_version = Some(Err("unreadable executable".into()));
        assert!(draw(&mut app, 80, 24).contains("unavailable"));
    }

    #[test]
    fn clicking_status_opens_plain_language_evidence() {
        let (_, mut app) = super::design_tests::render_with(120, 30);
        for pane_tag in [false, true] {
            app.mode = if pane_tag { Mode::Normal } else { Mode::Side };
            draw(&mut app, 120, 30);
            let hits: Vec<_> = app.hits.iter().filter_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::Why(1))).then_some(*r)).collect();
            assert!(hits.len() >= 2, "the pane tag and sidebar status are clickable");
            let at = if pane_tag { hits[0] } else { *hits.last().unwrap() };
            app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.x, row: at.y, modifiers: KeyModifiers::NONE });
            assert!(matches!(app.mode, Mode::Why { term: 1, .. }));
            let output = draw(&mut app, 120, 30); show(&output);
            assert!(output.contains("Status uncertain") && output.contains("Last confirmation: none"));
            assert!(!output.contains("process  shell running  now"));
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert_eq!(matches!(app.mode, Mode::Side), !pane_tag);
        }
        let info = app.snap.terms.get_mut(&1).unwrap();
        info.status_why = "screen: queued question".into(); info.why_screen_at = 1;
        app.mode = Mode::Why { term: 1, side: false };
        assert!(draw(&mut app, 120, 30).contains("Waiting for your answer"));
    }

    #[test]
    #[ignore = "opt-in release-mode performance measurement; no wall-clock CI threshold"]
    fn session_interactions_under_heavy_output() {
        const PANES: u32 = 8;
        const CYCLES: usize = 120;
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let base_ws = app.snap.workspaces[0].clone();
        let base_term = app.snap.terms[&2].clone();
        app.snap.workspaces.clear(); app.snap.terms.clear(); app.parsers.clear(); app.screen.tabs.clear();
        for id in 1..=PANES {
            let mut info = base_term.clone(); info.id = id; info.status = Status::Working;
            let mut ws = base_ws.clone(); ws.id = 100 + id; ws.tabs.truncate(1);
            ws.tabs[0].id = 200 + id; ws.tabs[0].layout = crate::layout::Node::Leaf(id); ws.tabs[0].focus = id; ws.active_tab = 200 + id;
            app.snap.workspaces.push(ws); app.snap.terms.insert(id, info);
            app.parsers.insert(id, app.new_parser(30, 100));
        }
        app.snap.active_ws = Some(101); app.screen_fresh(); app.screen_focus(1);
        let mut terminal = Terminal::new(TestBackend::new(160, 45)).unwrap();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap(); app.sync_sizes();
        let mut timings: [Vec<f64>; 4] = std::array::from_fn(|_| Vec::new());
        let mut bytes = 0;
        let mut scrolled = false;
        for cycle in 0..CYCLES {
            let payload = format!("\x1b[32mstress-{cycle:03}\x1b[0m output with enough text to wrap at narrow widths\r\n").repeat(64).into_bytes();
            let began = Instant::now();
            for id in 1..=PANES {
                bytes += payload.len();
                app.on_server(ServerMsg::Output { term: id, data: payload.clone() });
            }
            timings[3].push(began.elapsed().as_secs_f64() * 1000.0);
            app.mode = Mode::Normal;
            let began = Instant::now();
            let action = cycle % 3;
            let wanted = (cycle / 3) as u32 % PANES + 1;
            match action {
                0 => {
                    app.screen_focus(wanted);
                    let mut acknowledged = app.snap.clone();
                    acknowledged.active_ws = Some(100 + wanted);
                    app.on_server(ServerMsg::State(acknowledged));
                }
                1 => {
                    let r = app.panes[0].1;
                    app.on_mouse(MouseEvent { kind: MouseEventKind::ScrollUp, column: r.x + 1, row: r.y + 1, modifiers: KeyModifiers::NONE });
                    scrolled |= app.focused().is_some_and(|id| app.history(id).0 > 0);
                }
                _ => {
                    let (width, height) = if cycle % 2 == 0 { (100, 30) } else { (160, 45) };
                    terminal.backend_mut().resize(width, height);
                    app.on_event(event::Event::Resize(width, height));
                }
            }
            terminal.draw(|f| render::draw(&mut app, f)).unwrap(); app.sync_sizes();
            timings[action].push(began.elapsed().as_secs_f64() * 1000.0);
            if action == 0 { assert_eq!(app.focused(), Some(wanted)); }
            let area = terminal.backend().buffer().area;
            assert!(app.panes.iter().all(|(_, r)| r.right() <= area.right() && r.bottom() <= area.bottom()));
        }
        assert!(scrolled, "the benchmark really exercised scrollback");
        assert!(app.parsers[&PANES].screen().contents().contains("stress-"), "output survived switching and rewrapping");
        println!("{PANES} panes, {} output messages, {bytes} bytes", CYCLES * PANES as usize);
        for (label, mut samples) in ["switch", "scroll", "resize", "output burst"].into_iter().zip(timings) {
            samples.sort_by(f64::total_cmp);
            println!("{label}: p50={:.3}ms p95={:.3}ms max={:.3}ms", samples[samples.len() / 2], samples[(samples.len() * 95 / 100).min(samples.len() - 1)], samples[samples.len() - 1]);
        }
    }

    #[test]
    fn main_screen_follows_the_handoff() {
        let (text, mut app) = super::design_tests::render_with(160, 45);
        show(&text);
        let lines: Vec<&str> = text.lines().collect();
        assert!(!text.contains(">_ seshi"), "no logo");
        // The floating grid: a row of margin, the sidebar card the full height a column in, the
        // tab pills on its top row, the cards directly under them.
        assert!(lines[0].trim().is_empty(), "a row of margin on top");
        assert!(lines[1].starts_with("  ▁▁"), "the sidebar card, a column in: {}", lines[1]);
        assert!(text.contains("Fix flaky checkout test") && text.contains("claude") && text.contains("needs you"), "the active pane keeps its session title and state");
        assert!(text.contains("+ new") && text.contains("actions"), "New and Actions stay in the sidebar foot");
        assert!(lines[1].contains(" 1 ") && !lines[1].contains("claude") && lines[1].contains(" + ") && !lines[1].contains('▯'), "an unnamed tab is its number; +; no layout switch: {}", lines[1]);
        // Projects and their sessions, nothing in between.
        assert!(text.contains("▾ shop-api") && text.contains("● 1"), "a project: its fold arrow and name, what needs you on the right");
        assert!(text.contains("AGENTS 2 ─") && text.contains("TERMINALS 1 ─") && !text.contains("BRANCHES") && !text.contains("WORKTREES"), "a heading per section");
        assert!(!text.contains("main folder"), "no 'main folder' wording");
        assert!(!text.contains("+ open a project"), "opening a project is in the header now");
        // Rows: state and name, branch · age on the right, its question underneath.
        assert!(text.contains("● claude") && text.contains(" 3m") && !text.contains("main · 3m"), "state, then the name; age on the right (the agent too when the name isn't it), no branch");
        assert!(text.contains("Run npm test -- checkout?"), "the question under the agent");
        assert!(text.contains("Researching"), "the explorer is shown by its stage");
        assert!(text.contains("\u{25be} 4 subagents") && text.contains("Planning") && text.contains("Building") && text.contains("Reviewing"), "live helpers use stage labels instead of provider role labels: {text}");
        assert!(text.contains("⣾ rate") && text.contains(" 2m"), "a worktree's session is named after it");
        assert!(!text.contains("Rate limit /login"), "under a session only its question, as in the redesign");
        assert!(text.contains("› shell") && !text.contains("shell 2"), "a shell in the project's folder is just 'shell', no age");
        assert!(!text.contains("session"), "no 'session' wording on screen");
        assert!(!text.contains("● answer") && !text.contains(" Yes 1 "), "an agent's own question is answered in its own prompt: no answer bar");
        // Overlays are centred over a dimmed screen.
        for (mode, needle) in [
            (Mode::GoTo { query: String::new(), sel: 1 }, "NEEDS YOU"),
            (Mode::ScreenPane(screen::NewPaneHy::new(0, false)), "claude gets its own new worktree in shop-api"),
            (Mode::ScreenPane(screen::NewPaneHy { place: Some(1), ..screen::NewPaneHy::new(0, false) }), "Switches shop-api to a new branch"),
            (Mode::KeyMap(Box::new(screen::KeyMap { query: String::new(), searching: false, step: None, sel: 0 })), "inbox ● 1"),
        ] {
            app.mode = mode;
            let o = draw(&mut app, 160, 45);
            show(&o);
            assert!(o.contains(needle), "overlay shows {needle}");
        }
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 1, sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(" General ") && o.contains(" Sessions ") && o.contains("Sort sidebar by attention") && o.contains(" on "));
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 2, sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Card edges") && o.contains("PREVIEW") && o.contains("focused") && o.contains("dimmed"), "the pane settings, with a preview");
        // The splash: the SESHI wordmark, its tagline, what happened, buttons.
        app.mode = Mode::Normal;
        app.splash = true;
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("███████ ███████ ███████ ██   ██ ██") && o.contains("every session, one calm place") && !o.contains("⣿"), "the block wordmark, no art");
        assert!(o.contains("while you were away") && o.contains("1 need you") && o.contains("Resume where you left off") && o.contains("New shell here"));
        // Small windows keep it all.
        let o = draw(&mut app, 100, 30);
        assert!(!o.contains("⣿") && o.contains("New shell here"));
    }

    #[test]
    fn completed_subagents_leave_the_live_list_but_running_ones_are_inspectable() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let term = 1;
        app.snap.terms.get_mut(&term).unwrap().subagents.clear();
        app.snap.terms.get_mut(&term).unwrap().subagent_activity = vec![SubagentActivity {
            id: "child-done".into(), name: "Reviewer".into(), parent: None,
            state: SubagentState::Completed, task: Some("review the old flow".into()),
            model: Some("claude-opus".into()), effort: None, started_at: Some(100), updated_at: 130,
            latest_activity: Some("review returned".into()), result: Some("old finding".into()),
        }, SubagentActivity {
            id: "child-live".into(), name: "general-purpose".into(), parent: None,
            state: SubagentState::Working, task: Some("review the bing flow".into()),
            model: Some("claude-opus".into()), effort: None, started_at: Some(100), updated_at: 130,
            latest_activity: Some("review returned".into()), result: Some("one blocking finding".into()),
        }];
        app.screen_fresh();
        app.screen_side_set(screen::SideItem::Sess(term));
        app.on_key(KeyEvent::new(KeyCode::Char('s'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::SubagentInfo { term: found, index: 0 } if found == term), "keyboard users can open live subagent details");
        app.mode = Mode::Normal;
        let sidebar = draw(&mut app, 160, 45);
        assert!(sidebar.contains("1 subagent") && sidebar.contains("review the bi"), "only the active child and its task are shown: {sidebar}");
        assert!(!sidebar.contains("review the old flow") && !sidebar.contains("Finished"), "completed child is removed from the live list: {sidebar}");
        let target = app.hits.iter().find_map(|(_, hit)| match hit {
            Hit::Screen(screen::ScreenHit::SubagentInfo(found, index)) if *found == term && *index == 0 => Some(screen::ScreenHit::SubagentInfo(*found, *index)),
            _ => None,
        }).expect("the active subagent row is a mouse target");
        app.on_hy_hit(target, false);
        let inspector = draw(&mut app, 160, 45);
        assert!(inspector.contains("review the bing flow") && inspector.contains("claude-opus") && inspector.contains("one blocking finding"), "details show runtime data: {inspector}");
        assert!(inspector.contains("Not reported"), "unknown fields are labelled instead of inherited from the parent");
    }

    #[test]
    fn inactive_subagents_leave_no_empty_live_heading() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let pane = app.snap.terms.get_mut(&1).unwrap();
        pane.subagents.clear();
        pane.subagent_activity = vec![SubagentActivity {
            id: "child-finished".into(),
            name: "Reviewer".into(),
            parent: None,
            state: SubagentState::ConnectionLost,
            task: Some("review the old flow".into()),
            model: None,
            effort: None,
            started_at: Some(100),
            updated_at: 130,
            latest_activity: Some("completion was not reported".into()),
            result: None,
        }];
        app.screen_fresh();

        let sidebar = draw(&mut app, 160, 45);

        assert!(
            !sidebar.contains("subagent") && !sidebar.contains("review the old flow"),
            "inactive lifecycle history is not part of the live sidebar: {sidebar}"
        );
    }

    #[test]
    fn active_subagent_shows_its_task_as_quiet_secondary_text() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let term = 1;
        let pane = app.snap.terms.get_mut(&term).unwrap();
        pane.subagents.clear();
        pane.subagent_activity = vec![SubagentActivity {
            id: "child-general".into(),
            name: "delegate".into(),
            parent: None,
            state: SubagentState::Working,
            task: Some("Inspect sidebar titles".into()),
            model: None,
            effort: None,
            started_at: None,
            updated_at: 0,
            latest_activity: None,
            result: None,
        }];
        app.screen_fresh();
        app.screen_side_set(screen::SideItem::Sess(term));

        let sidebar = draw(&mut app, 160, 45);

        assert!(sidebar.contains("Inspect sidebar"), "the reported task is visible in the live list: {sidebar}");
        assert!(!sidebar.contains("general-purpose"), "provider-internal role names stay out of the sidebar: {sidebar}");
        assert!(!sidebar.contains("Active"), "the live list needs no redundant Active count: {sidebar}");

        let mut terminal = Terminal::new(TestBackend::new(160, 45)).unwrap();
        terminal.draw(|frame| render::draw(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let child_y = app.hits.iter().find_map(|(rect, hit)| {
            (*hit == Hit::Screen(screen::ScreenHit::SubagentInfo(term, 0))).then_some(rect.y)
        }).expect("the working child row is rendered");
        assert!((0..160).all(|x| !app.cfg.ui.spinner.iter().any(|frame| frame == buffer[(x, child_y)].symbol())), "working children have no animated spinner");
        let task_start = (0..129).find(|x| {
            (0.."Inspect".len()).all(|offset| buffer[(x + offset as u16, child_y)].symbol() == &"Inspect"[offset..offset + 1])
        }).expect("the reported task is rendered");
        assert_eq!(buffer[(task_start, child_y)].fg, app.theme.muted, "the child task is neutral gray");
        assert!(!buffer[(task_start, child_y)].modifier.contains(ratatui::style::Modifier::BOLD), "the child task is not bold");
        assert!(buffer[(task_start, child_y)].modifier.contains(ratatui::style::Modifier::DIM), "the child task is visually quieter than its parent");
        assert!((0..160).all(|x| buffer[(x, child_y)].symbol() != "⚙"), "the generic gear icon is removed");

        let narrow_sidebar = draw(&mut app, 100, 30);
        assert!(narrow_sidebar.contains("Inspect"), "helper tasks remain visible in a narrow sidebar: {narrow_sidebar}");
    }

    #[test]
    fn a_working_subagent_does_not_animate_when_its_parent_is_idle() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        for term in app.snap.terms.values_mut() {
            term.status = Status::None;
        }
        let pane = app.snap.terms.get_mut(&1).unwrap();
        pane.subagent_activity = vec![SubagentActivity {
            id: "child-live".into(), name: "worker".into(), parent: None,
            state: SubagentState::Working, task: None, model: None, effort: None,
            started_at: None, updated_at: 0, latest_activity: None, result: None,
        }];
        assert!(!app.any_working(), "child rows have no spinner to animate when the parent is idle");
    }


    #[test]
    fn no_answer_buttons_without_a_question_on_screen() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // Still "needs you", but what asked is gone (auto mode settled it).
        let mut p = vt100::Parser::new(40, 120, 0);
        p.process(b"* Waiting for 1 dynamic workflow to finish");
        app.parsers.insert(1, p);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("needs you") && !o.contains("● answer") && !o.contains(" Yes 1 "), "no buttons that would only type a digit");
    }

    #[test]
    fn the_inbox_answers_from_where_you_are() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // claude (pane 1) asks a numbered question (on its screen in the fixture); codex finished.
        let t = app.snap.terms.get_mut(&2).unwrap();
        (t.status, t.said) = (Status::Done, "All 14 tests pass now.".into());
        app.screen_fresh();
        app.act(Action::Jump);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("▁ Inbox") && o.contains("NEEDS YOU 1 ─") && o.contains("JUST FINISHED 1 ─") && !o.contains("EVERYTHING"), "the Inbox sheet: what needs you, what finished");
        assert!(o.contains("Run npm test -- checkout?") && o.contains("Yes 1") && o.contains("Always 2") && o.contains("No 3"), "the question and its answers");
        assert!(o.contains("“All 14 tests pass now.”"), "what the finished one said, quoted");
        assert!(o.contains(" move") && o.contains(" closes"), "the sheet's status bar");
        // It's docked beside the panes, not over them: the session's card is still there.
        assert!(o.contains("▁ claude"), "the panes reflow beside it");
        // 2 answers the selected question without going there.
        app.on_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::GoTo { .. }), "still in the Inbox");
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.contains("Always")), "{:?}", app.notice);
        // Typing finds any session instead (numbers too, once you're typing).
        for ch in "rate".chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("FOUND 1") && !o.contains("NEEDS YOU"), "searching: just what matches");
        // j closes it again, and opens it afresh.
        app.act(Action::Jump);
        assert!(app.mode == Mode::Normal, "j closes the Inbox");
        app.act(Action::Jump);
        assert!(matches!(&app.mode, Mode::GoTo { query, .. } if query.is_empty()));
    }



    #[test]
    fn splash_menus_and_resizing() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        // The splash waits for a button: other keys do nothing.
        app.splash = true;
        app.on_key(key(KeyCode::Char('x')));
        app.on_key(key(KeyCode::Esc));
        assert!(app.splash, "random keys don't leave the splash");
        app.on_key(key(KeyCode::Down));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("↑↓ choose   Enter open"));
        app.on_key(key(KeyCode::Enter));
        assert!(!app.splash && matches!(app.mode, Mode::Normal), "Enter picks the selected one (New: a shell, straight away)");
        app.mode = Mode::Normal;

        // Right-click menus.
        app.menu_for_session(1, (10, 10));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Rename") && o.contains("Close"));
        let Mode::ScreenMenu(m) = &app.mode else { panic!("a menu") };
        let rename = m.items.iter().position(|(l, _)| l == "Rename").unwrap();
        app.menu_pick(rename);
        assert!(matches!(app.mode, Mode::Prompt { kind: PromptKind::RenamePane(1), .. }), "picking an item does it");
        app.mode = Mode::Normal;
        app.menu_for_project(0, (5, 5));
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Rename") && o.contains("Close") && o.contains("New worktree") && o.contains("Open worktree…"), "herdr's project menu");
        app.mode = Mode::Normal;

        // herdr's pane menu, and closing asks first.
        app.menu_for_pane(1, (40, 10));
        let o = draw(&mut app, 160, 45);
        show(&o);
        for item in ["Rename pane", "Split right", "Split down", "Send right-clicks to pane", "Close pane"] {
            assert!(o.contains(item), "pane menu has {item}");
        }
        let Mode::ScreenMenu(m) = &app.mode else { panic!("a menu") };
        let close = m.items.iter().position(|(l, _)| l == "Close pane").unwrap();
        app.menu_pick(close);
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.title == "Close pane"), "asks before closing");
        let o = draw(&mut app, 160, 45);
        show(&o);
        // Buttons are the same pills everywhere: each starts with a pill's end, not a flat block.
        let cap = screen::ends(app.theme.btn).0.0.to_string();
        assert!(o.contains(&format!("{cap}Close Enter")) && o.contains(&format!("{cap}Cancel Esc")), "a red Close and a plain Cancel");
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(app.mode, Mode::Normal), "Esc keeps it");
        // The sidebar: sessions right under their project, worktree sessions tagged.
        let o = draw(&mut app, 160, 45);
        assert!(!o.contains("WORKTREES") && !o.contains("BRANCHES"), "no folder headings");
        assert!(o.contains("rate") && o.contains(" · 2m"), "a worktree session is named after it, with its agent and age");

        // Resizing the sidebar, within its limits.
        let before = app.screen.side_rect.width;
        app.act(Action::Resize(crate::layout::Dir::Right));
        draw(&mut app, 160, 45);
        assert!(app.screen.side_rect.width > before, "wider");
        for _ in 0..40 {
            app.act(Action::Resize(crate::layout::Dir::Right));
        }
        assert_eq!(app.screen.saved.side_w, Some(screen::SIDE_MAX), "but not past the max");
        for _ in 0..40 {
            app.act(Action::Resize(crate::layout::Dir::Left));
        }
        assert_eq!(app.screen.saved.side_w, Some(screen::SIDE_MIN), "nor under the min");
    }

    #[test]
    fn bright_black_backgrounds_are_a_quiet_panel() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // Claude draws pasted text and its diff panel on palette colour 8.
        let mut p = vt100::Parser::new(20, 80, 0);
        p.process(b"\x1b[100mpasted text\x1b[0m plain");
        app.parsers.insert(1, p);
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let (_, inner) = app.panes[0];
        let buf = term.backend().buffer();
        assert_eq!(buf[(inner.x - 1, inner.y)].bg, ratatui::style::Color::Reset, "a small contrasting panel does not recolor the card padding");
        assert_eq!(buf[(inner.x, inner.y)].bg, app.theme.card2, "not a light grey slab");
        assert_eq!(buf[(inner.x + 13, inner.y)].bg, ratatui::style::Color::Reset, "the rest on the card's ground");
    }

    #[test]
    fn a_terminal_surface_background_fills_card_padding() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let surface = ratatui::style::Color::Rgb(8, 9, 10);
        let mut parser = vt100::Parser::new(20, 80, 0);
        parser.process(b"\x1b[48;2;8;9;10m\x1b[2J");
        app.parsers.insert(1, parser);

        let mut terminal =
            ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 45)).unwrap();
        terminal.draw(|frame| render::draw(&mut app, frame)).unwrap();

        let (_, inner) = app.panes[0];
        let buffer = terminal.backend().buffer();
        assert_eq!(buffer[(inner.x - 1, inner.y)].bg, surface, "the padding shares the terminal's solid surface");
        assert_eq!(buffer[(inner.x, inner.y)].bg, surface, "the terminal canvas keeps its background");
        assert_eq!(buffer[(inner.x + inner.width, inner.y)].bg, surface, "the opposite padding stays visible without a seam");
        show(&draw(&mut app, 160, 45));
    }

    #[test]
    fn flat_terminal_background_matches_its_theme_surface_and_keeps_ansi_colors() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.layout = "flat".into();
        let mut parser = vt100::Parser::new(20, 80, 0);
        parser.process(b"\x1b[40mX\x1b[49mY");
        app.parsers.insert(1, parser);

        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 45)).unwrap();
        terminal.draw(|frame| render::draw(&mut app, frame)).unwrap();

        let (_, inner) = app.panes[0];
        let buffer = terminal.backend().buffer();
        assert_eq!(
            buffer[(inner.x, inner.y)].bg,
            ratatui::style::Color::Indexed(0),
            "explicit ANSI black remains the program's choice"
        );
        assert_eq!(
            buffer[(inner.x + 1, inner.y)].bg,
            app.theme.card,
            "the default terminal ground matches the flat pane surface"
        );
        assert_eq!(
            buffer[(inner.x + 2, inner.y)].bg,
            app.theme.card,
            "empty terminal cells keep the same surface"
        );
        show(&draw(&mut app, 160, 45));
    }

    #[test]
    fn wheel_scrolls_history() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The terminal is as big as the card's inside (seshi sizes it so).
        let (_, inside) = app.panes[0];
        let mut p = vt100::Parser::new(inside.height, inside.width, 1000);
        for i in 1..=100 {
            p.process(format!("line {i}
").as_bytes());
        }
        app.parsers.insert(1, p);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("line 100") && !o.contains("line 40 "), "bottom first");
        let (_, inner) = app.panes[0];
        for _ in 0..10 {
            app.on_mouse(MouseEvent { kind: MouseEventKind::ScrollUp, column: inner.x + 5, row: inner.y + 5, modifiers: KeyModifiers::NONE });
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(!o.contains("line 100"), "scrolled up: {:?}", app.history(1));
        assert!(o.contains("↑ 30 lines up"), "and it says so");
        app.on_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::SHIFT));
        app.on_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::SHIFT));
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("line 100") && !o.contains("lines up"), "Shift+PageDown back to the bottom");
    }

    #[test]
    fn wheel_scrolls_history_even_when_the_program_takes_the_mouse() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // An agent printing into the normal screen (its history above) that also asked for
        // the mouse: the wheel is still for scrolling back.
        let mut p = vt100::Parser::new(40, 120, 1000);
        for i in 1..=100 {
            p.process(format!("line {i}\r\n").as_bytes());
        }
        p.process(b"\x1b[?1000h\x1b[?1006h");
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        let (_, inner) = app.panes[0];
        for _ in 0..5 {
            app.on_mouse(MouseEvent { kind: MouseEventKind::ScrollUp, column: inner.x + 5, row: inner.y + 5, modifiers: KeyModifiers::NONE });
        }
        assert_eq!(app.history(1).0, 15, "seshi scrolled its history");
        // A full-screen program (no history of its own here) gets the wheel itself.
        let mut p = vt100::Parser::new(40, 120, 1000);
        p.process(b"\x1b[?1049h\x1b[?1000h\x1b[?1006h");
        app.parsers.insert(1, p);
        app.on_mouse(MouseEvent { kind: MouseEventKind::ScrollUp, column: inner.x + 5, row: inner.y + 5, modifiers: KeyModifiers::NONE });
        assert_eq!(app.history(1).0, 0, "the program scrolls itself");
    }

    #[test]
    fn a_normal_screen_agent_without_scrollback_receives_mouse_wheel() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let mut p = vt100::Parser::new(40, 120, 1000);
        p.process(b"\x1b[?1000h\x1b[?1006h");
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        let (_, pane) = app.panes[0];

        app.on_mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: pane.x + 5,
            row: pane.y + 5,
            modifiers: KeyModifiers::NONE,
        });

        assert!(matches!(rx.try_recv(), Ok(crate::protocol::ClientMsg::Input { term: 1, data }) if data == b"\x1b[<64;6;6M"));
    }

    #[test]
    fn a_normal_screen_agent_pages_on_wheel_when_mouse_capture_is_off_and_history_is_empty() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        app.parsers.insert(1, vt100::Parser::new(40, 120, 1000));
        draw(&mut app, 160, 45);
        let (_, pane) = app.panes[0];

        app.on_mouse(MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: pane.x + 5,
            row: pane.y + 5,
            modifiers: KeyModifiers::NONE,
        });

        assert!(matches!(rx.try_recv(), Ok(crate::protocol::ClientMsg::Input { term: 1, data }) if data == b"\x1b[5~"));
    }

    #[test]
    fn page_up_reaches_a_normal_screen_agent_when_history_is_empty() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        app.parsers.insert(1, vt100::Parser::new(40, 120, 1000));

        app.on_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));

        assert!(matches!(rx.try_recv(), Ok(crate::protocol::ClientMsg::Input { term: 1, data }) if data == b"\x1b[5~"));
    }

    #[test]
    fn behaves_like_a_terminal() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut p = vt100::Parser::new(40, 120, 1000);
        for i in 1..=100 {
            p.process(format!("line {i}\r\n").as_bytes());
        }
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        // PageUp at a prompt scrolls history; a scrollbar shows where you are.
        app.on_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        assert!(app.history(1).0 > 10, "PageUp scrolled: {:?}", app.history(1));
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("┃"), "scrollbar thumb");
        app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert_eq!(app.history(1).0, 0, "typing goes back to the bottom");
        // A full-screen program keeps PageUp for itself.
        if let Some(p) = app.parsers.get_mut(&1) {
            p.process(b"\x1b[?1049h");
        }
        app.on_key(KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE));
        assert_eq!(app.history(1).0, 0, "PageUp went to the program");
        // Synchronized output holds drawing until it ends.
        app.on_server(ServerMsg::Output { term: 1, data: b"\x1b[?2026hhalf a frame".to_vec() });
        assert!(app.sync_hold_until().is_some(), "held mid update");
        app.on_server(ServerMsg::Output { term: 1, data: b"rest\x1b[?2026l".to_vec() });
        assert!(app.sync_hold_until().is_none(), "drawn once it's whole");
        // Focus reporting.
        app.on_server(ServerMsg::Output { term: 1, data: b"\x1b[?1004h".to_vec() });
        assert!(app.focus_report.contains(&1));
        // A program's clipboard copy reaches the user (and says so).
        app.on_server(ServerMsg::Clipboard { term: 1, text: "hello".into() });
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.contains("copied 5 characters")));
    }

    #[test]
    fn rewraps_on_resize_and_follows_the_cursor_style() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // Narrow pane: a long line wraps over several rows.
        let long = "word ".repeat(40);
        app.sizes.insert(1, (40, 20));
        app.parsers.insert(1, vt100::Parser::new(20, 40, 1000));
        app.on_server(ServerMsg::Output { term: 1, data: format!("{long}\r\nend\x1b[5 q").into_bytes() });
        let rows_of = |app: &App| {
            let sc = app.parsers[&1].screen();
            sc.rows(0, sc.size().1).filter(|l| l.contains("word")).count()
        };
        let rows_narrow = rows_of(&app);
        assert!(rows_narrow >= 5, "wrapped narrow: {rows_narrow}");
        // Wider: the same text re-wraps into fewer rows instead of being cut.
        app.panes = vec![(1, Rect { x: 0, y: 0, width: 120, height: 20 })];
        app.sync_sizes();
        let rows_wide = rows_of(&app);
        assert!(rows_wide < rows_narrow && rows_wide >= 1, "re-wrapped: {rows_wide} rows (was {rows_narrow})");
        assert!(app.parsers[&1].screen().contents().contains("end"));
        // The program asked for a blinking bar.
        assert_eq!(app.cursor_style.get(&1), Some(&5));
    }

    #[test]
    fn saves_pasted_images_as_png() {
        let p = std::env::temp_dir().join(format!("seshi-paste-{}.png", std::process::id()));
        let px: Vec<u8> = (0..4 * 3 * 2).map(|i| i as u8).collect();
        super::write_png(&p, 3, 2, &px).unwrap();
        let bytes = std::fs::read(&p).unwrap();
        assert_eq!(&bytes[..8], &[0x89, b'P', b'N', b'G', 13, 10, 26, 10], "a real PNG");
        let _ = std::fs::remove_file(p);
    }

    #[test]
    fn task_box_starts_an_agent_on_a_task() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        let cmd = super::screen::np_command(&app, "claude", 1, "fix the login bug").unwrap();
        assert!(cmd.starts_with("claude --model opus ") && cmd.contains("fix the login bug"), "{cmd}");
        assert_eq!(super::screen::np_command(&app, "claude", 0, "  ").as_deref(), Some("claude"));
        assert_eq!(super::screen::np_command(&app, "shell", 0, "x"), None);
        app.screen_new(0, false);
        for c in "add tests".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Down));
        app.on_key(key(KeyCode::Right));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("TASK") && o.contains("add tests"), "the task row shows what you typed");
        assert!(o.contains("Runs: claude --model opus"), "and what will run");
        app.on_key(key(KeyCode::Esc));
        app.screen_new(0, false);
        assert!(matches!(&app.mode, Mode::ScreenPane(np) if np.task == "add tests"), "Esc keeps the task as a draft");
    }

    #[test]
    fn switching_sessions_is_not_held_by_the_previous_panes_update() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let old = app.focused().unwrap();
        app.sync_hold.insert(old, Instant::now());
        assert!(app.sync_hold_until().is_some());
        app.snap.workspaces[0].tabs[0].focus = 2;
        app.snap.active_ws = Some(10);
        app.panes.retain(|(id, _)| *id == old);
        assert!(app.sync_hold_until().is_none(), "navigation draws immediately instead of waiting for the old pane");
        app.snap.workspaces[0].tabs[0].focus = old;
        app.mode = Mode::Actions { sel: 0 };
        assert!(app.sync_hold_until().is_none(), "opening a control is not delayed by the child's frame either");
    }

    #[test]
    fn agent_rows_show_name_model_and_latest_prompt() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let id = *app.snap.terms.iter().find(|(_, t)| t.agent.as_deref() == Some("claude")).unwrap().0;
        let t = app.snap.terms.get_mut(&id).unwrap();
        t.name = "Fix the login flow".into();
        t.summary = "now add a test for it".into();
        t.model = "opus 4.5".into();
        t.status = Status::Working;
        app.screen_fresh();
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("shop-api") && !o.contains("opus 4.5"), "the repo in the card's title; the model is the agent's own to show");
        assert!(o.contains("Fix the login flow"), "its name stays");
        assert!(!o.contains("› now add a test for it"), "one line under a row, no more");
    }


    #[test]
    fn history_keeps_status_changes_without_terminal_bells() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut next = app.snap.clone();
        let (a, b) = {
            let mut ids = next.terms.iter().filter(|(_, t)| t.status != Status::Done).map(|(id, _)| *id);
            (ids.next().unwrap(), ids.next().unwrap())
        };
        next.terms.get_mut(&a).unwrap().status = Status::Done;
        next.terms.get_mut(&a).unwrap().said = "Fixed it.\nMore".into();
        next.terms.get_mut(&b).unwrap().bell = true;
        next.terms.get_mut(&b).unwrap().status = Status::Blocked;
        app.got_state = true;
        app.on_server(ServerMsg::State(next));
        let texts: Vec<&str> = app.history.iter().map(|h| h.3.as_str()).collect();
        assert!(texts.iter().any(|t| t.ends_with("finished: Fixed it.")), "{texts:?}");
        assert!(texts.iter().any(|t| t.ends_with("needs you")), "{texts:?}");
        assert!(!texts.iter().any(|t| t.ends_with("rang the bell")), "bell output is not an attention event: {texts:?}");
        app.act(Action::History);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("What happened") && o.contains("needs you") && !o.contains('♪'));
    }

    #[test]
    fn any_number_of_splits_and_tabs() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        app.cfg.ui.layout = "cards".into();
        app.cfg.ui.panes = "floating".into();
        app.cfg.ui.gap = "2".into();
        let a = app.focused().unwrap();
        let others: Vec<TermId> = app.snap.terms.keys().copied().filter(|t| *t != a).collect();
        let (b, c) = (others[0], others[1]);
        app.screen.tabs.clear();
        app.screen_place(a, None);
        app.screen.pending_split = Some((a, Instant::now()));
        app.screen_place(b, Some(a));
        app.screen.pending_split = Some((b, Instant::now()));
        app.screen.split_dir = Some(crate::layout::Dir::Down);
        app.screen_place(c, Some(b));
        assert_eq!(app.screen.tabs.len(), 1);
        assert_eq!(app.screen.tabs[0].layout.leaves().len(), 3, "all three panes share the tab");
        // Focus stays on a (the snapshot's focus), so draw shows all three.
        app.screen.tabs[0].focus = a;
        let o = draw(&mut app, 200, 50);
        show(&o);
        assert_eq!(app.screen.leaf_rects.len(), 3);
        let rect = |id| app.screen.leaf_rects.iter().find(|(term, _)| *term == id).unwrap().1;
        assert_eq!(rect(c).x, rect(b).x, "an explicit down split stays in the target pane's column");
        assert!(rect(c).y > rect(b).y, "an explicit down split appears below its target");
        let card = |id| app.pane_frames.iter().find(|(term, _)| *term == id).unwrap().1;
        assert_eq!(card(b).x - card(a).right(), 2, "the configured column gap separates cards");
        assert_eq!(card(c).y - card(b).bottom(), 2, "the configured row gap separates cards");
        // Close one: two left, with a line between them you can drag.
        assert!(app.screen_unshow(c));
        assert_eq!(app.screen.tabs[0].layout.leaves().len(), 2);
        draw(&mut app, 200, 50);
        assert_eq!(app.screen.dividers.len(), 1, "a divider between the two");
        let before = app.screen.leaf_rects[0].1.width;
        let path = app.screen.dividers[0].2.clone();
        app.screen.tabs[0].layout.set_ratio(&path, 0.3);
        draw(&mut app, 200, 50);
        assert!(app.screen.leaf_rects[0].1.width < before, "dragging moves it");
        // A new tab for c in this session; the bar shows both.
        app.screen.new_tab = Some((Instant::now(), app.screen.tabs[0].owner));
        app.screen_place(c, Some(a));
        assert_eq!(app.screen.tabs.len(), 2);
        assert_eq!(app.screen.tabs[1].owner, app.screen.tabs[0].owner, "a tab of the same session");
        app.screen.tab = 0;
        let o = draw(&mut app, 200, 50);
        show(&o);
        assert!(o.contains(" 1 ") && o.contains(" 2 "), "a tab bar");
        assert!(app.hits.iter().any(|(_, h)| *h == Hit::Screen(screen::ScreenHit::TabNew)), "with a + for another tab");
    }

    #[test]
    fn new_agent_defaults_to_its_own_worktree() {
        let pane = screen::NewPaneHy::new(0, false);
        assert_eq!(pane.place_for("claude", true), 0, "agents default to a new worktree");
        assert_eq!(pane.place_for("codex", true), 0, "the default applies across providers");
        assert_eq!(pane.place_for("shell", true), 2, "plain shells stay in the project checkout");
        assert_eq!(pane.place_for("claude", false), 2, "the opt-out setting is respected");
        assert_eq!(pane.place_for("claude", true), 0, "starting a new chooser does not retain a prior location");
    }

    #[test]
    fn split_intent_survives_the_focus_change_to_its_target_pane() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.screen.tabs.clear();
        app.screen_place(1, None);

        app.menu_do(menu::Act::Split(2, crate::layout::Dir::Down));
        app.screen_place(2, Some(1));
        assert!(app.screen.pending_split.is_some(), "focusing the pane chosen in the context menu must not consume the pending split");

        app.snap.terms.insert(4, app.snap.terms[&3].clone());
        app.snap.terms.get_mut(&4).unwrap().id = 4;
        app.screen_place(4, Some(2));
        let layout = &app.screen.tabs[app.screen.tab].layout;
        assert!(matches!(layout, crate::layout::Node::Split { horizontal: false, .. }), "Split down must remain vertical after the focus event");
        assert_eq!(layout.leaves(), vec![2, 4]);
    }

    #[test]
    fn a_session_has_tabs_of_its_own() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // claude (1), codex (2), and a shell (3).
        let rows = |app: &App| app.screen_model().iter().flat_map(|p| p.sessions().map(|s| s.term).collect::<Vec<_>>()).collect::<Vec<_>>();
        app.screen.tabs.clear();
        app.screen_place(1, None);
        draw(&mut app, 160, 45);
        assert!(app.hits.iter().filter(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_)))).count() == 1 && app.hits.iter().any(|(_, h)| *h == Hit::Screen(screen::ScreenHit::TabNew)), "the tab row, with +, even with one tab");
        // Ctrl+Space t in claude's session: a shell where you are, straight away, as a tab of
        // claude's (no card to fill in first).
        let key = |app: &mut App, c: KeyCode| app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.act(Action::NewTab);
        assert_eq!(app.mode, Mode::Normal, "nothing to answer first");
        assert!(matches!(app.screen.new_tab, Some((_, 1))), "the next session is a tab of claude's: {:?}", app.screen.new_tab);
        app.screen_place(3, Some(1));
        app.screen_fresh();
        assert!(!rows(&app).contains(&3), "the shell is one of claude's tabs, not a row of its own: {:?}", rows(&app));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert_eq!(app.session_tabs().len(), 2);
        assert!(app.hits.iter().filter(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_)))).count() == 2 && !o.contains("2 shell"), "claude's two tabs, unnamed ones shown by number");
        // Rename uses the same centered text editor as pane and project names.
        app.act(Action::RenameTab);
        for c in "tests".chars() {
            key(&mut app, KeyCode::Char(c));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Rename tab") && o.contains("tests█"), "the dialog shows the draft: {o}");
        key(&mut app, KeyCode::Enter);
        // The daemon owns the name: play its part by applying the command to the snapshot.
        let (rws, rtab, rname) = std::iter::from_fn(|| rx.try_recv().ok())
            .find_map(|m| match m {
                crate::protocol::ClientMsg::Command(crate::protocol::Command::RenameTab { ws, tab, name }) => Some((ws, tab, name)),
                _ => None,
            })
            .expect("the rename is sent to the daemon");
        for t in app.snap.workspaces.iter_mut().filter(|w| w.id == rws).flat_map(|w| w.tabs.iter_mut()).filter(|t| t.id == rtab) {
            t.name = rname.clone();
        }
        app.screen_sync();
        let o = draw(&mut app, 160, 45);
        assert!(o.contains(" tests ") && !o.contains("2 tests"), "renamed: the pill is its name");
        // Views get rebuilt (another session reuses the slot, the client restarts): the name stays.
        app.screen.tabs.iter_mut().for_each(|t| t.name.clear());
        app.screen_sync();
        let o = draw(&mut app, 160, 45);
        assert!(o.contains(" tests "), "the name comes back from the daemon: {o}");
        // Another session: its own view, with its own rail.
        app.screen_place(2, Some(3));
        draw(&mut app, 160, 45);
        assert_eq!(app.session_tabs().len(), 1);
        assert!(app.hits.iter().filter(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_)))).count() == 1, "codex has one tab");
        // Back to claude: the tab you were last on there (the shell).
        app.screen_place(1, Some(2));
        assert_eq!(app.screen.tabs[app.screen.tab].focus, 3, "back on the shell tab");
        // Closing claude's own tab while claude runs takes a second click.
        let mine = app.screen.tabs.iter().position(|t| t.layout.contains(1)).unwrap();
        app.close_tab(mine);
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.contains("close it again")), "{:?}", app.notice);
    }

    #[test]
    fn the_wheel_scrolls_an_agents_full_screen_view() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        let (term, r) = app.pane_frames[0];
        let at = (r.x + 10, r.y + 10);
        let mouse = |app: &mut App, kind: MouseEventKind| app.on_mouse(MouseEvent { kind, column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
        // A drag whose release never came is over once the mouse moves with no button held.
        app.screen.drag = Some(screen::Drag::Session(2, false));
        mouse(&mut app, MouseEventKind::Moved);
        assert!(app.screen.drag.is_none(), "a stuck drag doesn't keep the mouse from the panes");
        // Claude Code's full-screen view, not taking the mouse: the wheel pages it.
        assert!(app.snap.terms[&term].agent.is_some());
        app.feed(term, b"\x1b[?1049h");
        assert!(app.parsers[&term].screen().alternate_screen());
        mouse(&mut app, MouseEventKind::ScrollUp);
        assert!(app.wheel_page.is_some(), "paged (PgUp), not arrow keys into its prompt history");
    }

    #[test]
    fn codex_output_above_its_input_box_can_be_scrolled_back() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        let (term, _) = app.pane_frames[0];
        // A pane's screen as the app makes it (the fixture's keep no history).
        let p = app.new_parser(40, 100);
        app.parsers.insert(term, p);
        // Codex: a scroll region over the rows above its input box, and each finished line
        // printed at the region's bottom, pushing the ones above it up and off the top.
        let mut out = b"\x1b[1;30r\x1b[30;1H".to_vec();
        for i in 0..60 {
            out.extend(format!("\r\nline {i}").bytes());
        }
        out.extend(b"\x1b[r");
        app.feed(term, &out);
        app.scroll_by(term, 20);
        assert!(app.history(term).0 > 0, "lines pushed off the top are history");
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("line 10") && !o.contains("line 59"), "and show when scrolled back (20 lines up)");
    }

    #[test]
    fn pane_info_says_how_a_pane_scrolls() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        let (term, _) = app.pane_frames[0];
        let mut p = app.new_parser(40, 100);
        p.process(b"\x1b[?1000h\x1b[?1006h");
        app.parsers.insert(term, p);
        let empty = app.pane_info(term).unwrap();
        assert!(empty.contains("normal screen") && empty.contains("wheel goes to the program") && empty.contains("0 lines of history"), "{empty}");
        app.feed(term, &b"line\r\n".repeat(100));
        let info = app.pane_info(term).unwrap();
        assert!(info.contains("normal screen") && info.contains("the wheel goes to seshi") && info.contains("61 lines of history"), "{info}");
        app.feed(term, b"\x1b[?1049h\x1b[?1000h\x1b[?1006h");
        let info = app.pane_info(term).unwrap();
        assert!(info.contains("full-screen") && info.contains("the wheel goes to the program"), "{info}");
    }

    #[test]
    fn a_pane_rewraps_once_a_drag_ends() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        app.sync_sizes();
        let (term, _) = app.pane_frames[0];
        app.keep_raw(term, b"some output\r\n", true);
        // Mid-drag, a new width just resizes the screen; it re-wraps after.
        app.screen.drag = Some(screen::Drag::Side);
        app.screen.saved.side_w = Some(50);
        draw(&mut app, 160, 45);
        app.sync_sizes();
        assert!(app.rewrap.contains(&term), "re-wrap waits for the drag to end");
        app.screen.drag = None;
        app.sync_sizes();
        assert!(!app.rewrap.contains(&term), "and happens once it has");
    }

    #[test]
    fn renaming_names_the_row() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let id = *app.snap.terms.iter().find(|(_, t)| t.agent.as_deref() == Some("codex")).unwrap().0;
        app.snap.terms.get_mut(&id).unwrap().label = "Doing something".into();
        app.screen_fresh();
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Doing something"), "the name is on the row itself");
        assert_eq!(o.matches("Doing something").count(), 1, "once: on the row, not in a second line");
    }

    #[test]
    fn sidebar_by_keyboard() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        draw(&mut app, 160, 45);
        // Clicking a project puts the keys in the sidebar, on that project.
        app.on_hy_hit(screen::ScreenHit::ToggleProj(0), false);
        app.on_hy_hit(screen::ScreenHit::ToggleProj(0), false);
        assert!(matches!(app.mode, Mode::Side) && app.screen.cursor_proj.is_some(), "on the project row");
        draw(&mut app, 160, 45);
        // ← folds it, → opens it again.
        app.on_key(key(KeyCode::Left));
        assert!(app.screen.saved.closed.iter().any(|k| k.starts_with("p:")), "folded");
        app.on_key(key(KeyCode::Right));
        assert!(!app.screen.saved.closed.iter().any(|k| k.starts_with("p:")), "open");
        draw(&mut app, 160, 45);
        // ↓ onto its first session; ← back up to the project.
        app.on_key(key(KeyCode::Down));
        let first = app.screen.cursor.expect("on a session");
        app.on_key(key(KeyCode::Left));
        assert!(app.screen.cursor_proj.is_some() && app.screen.cursor.is_none(), "← goes to its project");
        app.on_key(key(KeyCode::Down));
        assert_eq!(app.screen.cursor, Some(first));
        draw(&mut app, 160, 45);
        // A row's letter does what its menu says: x asks to close it.
        app.on_key(key(KeyCode::Char('x')));
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.title == "Close pane"), "x closes (after asking)");
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(app.mode, Mode::Side) && app.screen.cursor == Some(first), "saying no goes back to the sidebar");
        let next = app.side_after_close();
        app.on_key(key(KeyCode::Char('x')));
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Side), "after closing, the keys stay in the sidebar");
        assert_eq!(next.map(|n| n != screen::SideItem::Sess(first)), Some(true));
        // The leader key's close, from the sidebar: the cursor's row, and the keys stay too.
        let on = app.screen.cursor.or_else(|| app.screen.side_items.iter().find_map(|i| match i { screen::SideItem::Sess(t) => Some(*t), _ => None })).unwrap();
        app.screen_side_set(screen::SideItem::Sess(on));
        app.mode = Mode::Prefix { since: std::time::Instant::now() };
        app.act(Action::ClosePane);
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.act == crate::client::menu::Act::End(vec![on])), "it asks about the row the cursor is on");
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Side), "and afterwards you're still in the sidebar");
        app.screen_side_set(screen::SideItem::Sess(first));
        // Enter opens it and gives the keys back to the pane.
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Normal) && app.screen.cursor.is_none());
        // The leader in the sidebar is the leader, not "message this one".
        app.act(Action::BrowseTree);
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        assert!(matches!(app.mode, Mode::Prefix { .. }), "Ctrl+Space waits for a key: {:?}", std::mem::discriminant(&app.mode));
        app.mode = Mode::Normal;
        // Typing in the sidebar goes to the pane instead.
        app.act(Action::BrowseTree);
        assert!(matches!(app.mode, Mode::Side));
        app.on_key(key(KeyCode::Char('q')));
        assert!(matches!(app.mode, Mode::Normal), "a letter leaves the sidebar (and is typed into the pane)");
    }

    #[test]
    fn changes_outside_git_offers_git_init() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let dir = std::env::temp_dir().join(format!("seshi-nogit-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        app.open_changes(dir.clone());
        let o = draw(&mut app, 160, 45);
        show(&o);
        let cap = screen::ends(app.theme.btn).0.0.to_string();
        assert!(o.contains("isn't a git repository yet.") && o.contains(&format!("{cap}Make it a git repo g")) && o.contains(&format!("{cap}Cancel Esc")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_grouped_like_the_redesign() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.act(Action::Settings);
        let o = draw(&mut app, 160, 45);
        show(&o);
        for g in ["INPUT", "LAYOUT", "SHELL"] {
            assert!(o.contains(g), "General is grouped: {g}");
        }
        let input = o.find("INPUT").unwrap();
        let layout = o.find("LAYOUT").unwrap();
        assert!(input < layout, "INPUT before LAYOUT");
        assert!(o.contains("› Leader key"), "the first row is selected");
        // Appearance has one theme control that opens the searchable browser.
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("PANES") && o.contains("Card edges") && o.contains("PREVIEW"), "the pane settings first, with their preview");
        let theme_row = design::settings_rows(modal::Cat::Appearance).iter().position(|r| matches!(r, design::SRow::Setting(s) if s.path == "theme")).unwrap();
        for _ in 0..theme_row {
            app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("THEME") && o.contains("Enter browse"), "then the theme browser control");
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Themes") && o.contains("Seshi Walnut"));
        app.on_paste("Monokai".into());
        assert!(draw(&mut app, 160, 45).contains("Monokai"));
        app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        app.on_paste("Tokyo Night".into());
        assert!(draw(&mut app, 160, 45).contains("Tokyo Night"));
    }

    #[test]
    fn a_program_seshi_doesnt_know_goes_by_its_name_and_can_be_taught() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let t = app.snap.terms.get_mut(&3).unwrap();
        (t.process, t.title) = ("dst".into(), "✦ 🐋 deepseek harness".into());
        app.screen_fresh();
        let model = app.screen_model();
        let row = model.iter().flat_map(|p| p.sessions().cloned().collect::<Vec<_>>()).find(|s| s.term == 3).unwrap();
        assert_eq!(row.name, "dst", "the row says what runs there, not 'shell'");
        let (title, items) = app.session_items(3).unwrap();
        let labels: Vec<&str> = items.iter().map(|(l, _)| l.as_str()).collect();
        assert!(labels.contains(&"dst is an agent…"), "{labels:?}");
        assert!(!labels.iter().any(|l| l.starts_with("Message")), "no messaging a program that isn't an agent: {labels:?}");
        assert!(!title.contains('🐋'), "titled by the program, not its window title: {title}");
    }

    #[test]
    fn jumps_between_commands_in_history() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let term = 1;
        app.parsers.insert(term, vt100::Parser::new(20, 80, 1000));
        app.marks.remove(&term);
        // Three commands, each with output long enough to scroll away.
        for n in 1..=3 {
            app.feed(term, format!("\x1b]133;A\x07$ cmd{n}\r\n").as_bytes());
            for i in 0..30 {
                app.feed(term, format!("cmd{n} line {i}\r\n").as_bytes());
            }
        }
        app.feed(term, b"\x1b]133;A\x07$ ");
        let top = |app: &App| {
            let p = app.parsers.get(&term).unwrap();
            let (_, cols) = p.screen().size();
            p.screen().rows(0, cols).next().unwrap_or_default().trim_end().to_string()
        };
        app.jump_prompt(term, true);
        assert_eq!(top(&app), "$ cmd3", "back to the last command");
        app.jump_prompt(term, true);
        assert_eq!(top(&app), "$ cmd2");
        app.jump_prompt(term, false);
        assert_eq!(top(&app), "$ cmd3", "and forward again");
        // A history that fills up and drops its oldest lines: still the right command.
        app.parsers.insert(term, vt100::Parser::new(20, 80, 50));
        app.marks.remove(&term);
        for n in 1..=4 {
            app.feed(term, format!("\x1b]133;A\x07$ job{n}\r\n").as_bytes());
            for i in 0..30 {
                app.feed(term, format!("job{n} line {i}\r\n").as_bytes());
            }
        }
        app.jump_prompt(term, true);
        assert_eq!(top(&app), "$ job4", "found though the history shifted");
        app.jump_prompt(term, true);
        assert_eq!(top(&app), "$ job3");
    }

    #[test]
    fn a_popup_floats_over_everything_and_takes_the_keys() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut pop = app.snap.terms[&3].clone();
        (pop.id, pop.popup, pop.process) = (9, true, "fzf".into());
        app.snap.terms.insert(9, pop);
        let mut p = vt100::Parser::new(30, 120, 0);
        p.process(b"> pick a file");
        app.parsers.insert(9, p);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("> pick a file"), "drawn over the rest");
        assert_eq!(app.typing_to(), Some(9), "typing goes to it");
        assert_ne!(app.focused(), Some(9), "the pane you're on stays yours underneath");
        app.snap.terms.remove(&9);
        assert_eq!(app.typing_to(), app.focused(), "closed: typing goes back");
    }

    #[test]
    fn drag_copies_what_is_under_the_mouse_after_output_while_scrolled_up() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        let term = app.focused().unwrap();
        let (_, r) = *app.panes.iter().find(|(t, _)| *t == term).unwrap();
        let mut p = vt100::Parser::new(r.height, r.width, 1000);
        for i in 0..80 {
            p.process(format!("line {i}\r\n").as_bytes());
        }
        app.parsers.insert(term, p);
        draw(&mut app, 160, 45);
        // Scrolled up, then the agent prints more: vt100 keeps the view where it was.
        app.scroll_by(term, 10);
        app.parsers.get_mut(&term).unwrap().process(b"more 1\r\nmore 2\r\nmore 3\r\n");
        draw(&mut app, 160, 45);
        let rows: Vec<String> = app.parsers[&term].screen().rows(0, r.width).collect();
        let row_of = |s: &str| rows.iter().position(|l| l == s).unwrap() as u16;
        let (from, to) = (row_of("line 61"), row_of("line 68"));
        let mouse = |app: &mut App, kind: MouseEventKind, y: u16| {
            app.on_mouse(MouseEvent { kind, column: r.x, row: r.y + y, modifiers: KeyModifiers::NONE });
            draw(app, 160, 45);
        };
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), from + 1);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), to);
        let Mode::Copy(c) = &app.mode else { panic!("dragging selects") };
        let picked = c.selected_text();
        assert!(picked.starts_with("line 61\n") && picked.ends_with("\nl"), "{picked:?}");
    }

    #[test]
    fn the_leader_lights_up_and_its_key_map_runs_or_searches() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.which_key = false;
        let lead = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        let key = |app: &mut App, c: KeyCode| app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
        // Armed: the sky pill in the tab row, the focused card's border sky too.
        app.on_key(lead);
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let y = (0..45).find(|&y| (0..160).any(|x| buf[(x, y)].symbol() == "✕")).unwrap();
        let x = (0..160).rev().find(|&x| buf[(x, y)].symbol() == "✕").unwrap();
        assert_eq!(buf[(x + 2, y)].fg, app.theme.sky(), "the focused border turns sky");
        // The delayed overview groups common commands; ? opens the full reference instead.
        app.open_keymap(None);
        assert!(matches!(app.mode, Mode::KeyMap(_)));
        let o = draw(&mut app, 160, 45);
        show(&o);
        for g in ["AGENTS", "PANES", "TABS", "PROJECT", "SESHI"] {
            assert!(o.contains(g), "the key map has {g}");
        }
        assert!(o.contains("inbox ● 1") && o.contains("worktrees  ›"), "counts on their keys, steps marked");
        // Tab, then words: a search over every leader key.
        key(&mut app, KeyCode::Tab);
        for c in "sidebar".chars() {
            key(&mut app, KeyCode::Char(c));
        }
        let o = draw(&mut app, 160, 45);
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.query == "sidebar") && o.contains("sidebar"), "a search for the sidebar");
        // A key with › opens its second step; Backspace goes back.
        app.mode = Mode::Normal;
        app.on_key(lead);
        key(&mut app, KeyCode::Char('w'));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.step == Some(screen::Step::Worktrees)), "w opens the worktrees step");
        assert!(o.contains("WORKTREES IN SHOP-API") && o.contains("new worktree") && o.contains("merge into main") && o.contains("the path so far"));
        key(&mut app, KeyCode::Backspace);
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.step.is_none()), "back to the key map");
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn actions_list_every_command_with_its_key() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("actions"));
        let at = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::Actions)).then_some((r.x, r.y))).expect("a actions is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
        assert!(matches!(app.mode, Mode::Actions { sel: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("▁ Actions") && o.contains("New shell here") && o.contains("Inbox ● 1") && !o.contains("Open project") && o.contains("+ key, anywhere"));
        // Its key runs it: b hides the sidebar.
        assert!(app.sidebar);
        app.on_key(KeyEvent::new(KeyCode::Char('b'), KeyModifiers::NONE));
        assert!(!app.sidebar && app.mode == Mode::Normal, "b: the sidebar hides");
    }

    #[test]
    fn mouse_can_reach_agent_sheets_and_every_palette_command_without_sidebar() {
        for (w, h) in [(80, 24), (100, 30), (160, 45)] {
            let (_, mut app) = super::design_tests::render_with(w, h);
            app.sidebar = false;
            draw(&mut app, w, h);
            let click = |app: &mut App, hit| {
                let (r, _) = app.hits.iter().rev().find(|(_, target)| *target == Hit::Screen(hit)).expect("visible mouse control");
                let (column, row) = (r.x, r.y);
                app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column, row, modifiers: KeyModifiers::NONE });
            };
            click(&mut app, screen::ScreenHit::Actions);
            assert!(matches!(app.mode, Mode::Actions { .. }));
            let screen = draw(&mut app, w, h);
            show(&screen);
            assert!(screen.contains("Show sidebar"));
            for action in [Action::Palette, Action::AgentStatusSetup] {
                let index = screen::action_items().iter().position(|(_, a)| *a == action).expect("agent features and all commands are discoverable");
                assert!(app.hits.iter().any(|(_, target)| *target == Hit::Screen(screen::ScreenHit::ActionRow(index))));
            }
            let index = screen::action_items().iter().position(|(_, a)| *a == Action::Palette).unwrap();
            click(&mut app, screen::ScreenHit::ActionRow(index));
            let commands = app.pick_items("", true);
            for index in 0..commands.len() {
                app.mode = Mode::Picker { query: String::new(), sel: index, commands: true };
                draw(&mut app, w, h);
                assert!(app.hits.iter().any(|(_, target)| *target == Hit::Screen(screen::ScreenHit::PickRow(index))), "every command can be scrolled into view");
            }
            let index = commands.iter().position(|item| matches!(item.target, PickTarget::Command(Action::Help))).unwrap();
            app.mode = Mode::Picker { query: String::new(), sel: index, commands: true };
            draw(&mut app, w, h);
            click(&mut app, screen::ScreenHit::PickRow(index));
            assert!(matches!(app.mode, Mode::Shortcuts { .. }));
            app.act(Action::AgentStatusSetup);
            let screen = draw(&mut app, w, h); show(&screen);
            assert!(screen.contains("Set up Codex status hooks") && screen.contains("Set up Pi status hooks") && screen.contains("Set up OpenCode status hooks"));
            for index in 0..2 {
                assert!(app.hits.iter().any(|(_, hit)| *hit == Hit::Screen(screen::ScreenHit::MenuPick(index))));
            }
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert_eq!(app.mode, Mode::Normal);
        }
    }

    #[test]
    fn the_new_button_and_leader_n_start_the_same_plain_shell() {
        for cursor in [None, Some(2)] {
            for by_mouse in [false, true] {
                let (_, mut app) = super::design_tests::render_with(100, 30);
                let (tx, mut rx) = mpsc::unbounded_channel();
                app.out = tx;
                app.screen.cursor = cursor;
                app.mode = if cursor.is_some() { Mode::Side } else { Mode::Normal };
                let selected = &app.snap.terms[&cursor.or(app.focused()).unwrap()];
                let expected = selected.root.clone().unwrap_or_else(|| selected.cwd.clone());
                let screen = draw(&mut app, 100, 30);
                show(&screen);
                if by_mouse {
                    assert!(screen.contains("+ new"), "a visible new button");
                    let at = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::NewShell)).then_some((r.x + 1, r.y))).expect("+ new is clickable");
                    app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
                } else {
                    app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
                    app.on_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
                }
                assert_eq!(app.mode, Mode::Normal, "new opens a shell immediately");
                let commands: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).filter_map(|m| match m { ClientMsg::Command(c) => Some(c), _ => None }).collect();
                assert!(matches!(commands.as_slice(), [Command::NewWorkspace { cwd: Some(cwd), name: None, cmd: None }] if cwd == &expected), "one plain shell in the selected project: {commands:?}");
                assert!(app.screen.pending_split.is_none(), "the new session opens full screen");
            }
        }
    }

    #[test]
    fn caps_lock_n_after_the_leader_starts_a_shell_not_history() {
        use ratatui::crossterm::event::{KeyEventKind, KeyEventState};

        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        let caps_state = if cfg!(windows) {
            KeyEventState::NONE
        } else {
            KeyEventState::CAPS_LOCK
        };
        app.on_key(KeyEvent::new_with_kind_and_state(
            KeyCode::Char('N'),
            KeyModifiers::NONE,
            KeyEventKind::Press,
            caps_state,
        ));

        assert_eq!(app.mode, Mode::Normal, "Caps Lock must not open What happened");
        assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|message| matches!(
            message,
            ClientMsg::Command(Command::NewWorkspace { .. })
        )), "leader + Caps Lock + n opens a new shell");
    }

    #[test]
    fn caps_lock_x_closes_the_sidebar_session_instead_of_typing() {
        use ratatui::crossterm::event::{KeyEventKind, KeyEventState};

        let (_, mut app) = super::design_tests::render_with(100, 30);
        app.act(Action::BrowseTree);
        let term = app.screen.cursor.expect("sidebar is on a session");
        let caps_state = if cfg!(windows) {
            KeyEventState::NONE
        } else {
            KeyEventState::CAPS_LOCK
        };
        app.on_key(KeyEvent::new_with_kind_and_state(
            KeyCode::Char('X'),
            KeyModifiers::NONE,
            KeyEventKind::Press,
            caps_state,
        ));

        assert!(
            matches!(&app.mode, Mode::Confirm(confirm) if confirm.act == menu::Act::End(vec![term])),
            "Caps Lock + x must ask to close the selected session"
        );
    }

    #[test]
    fn settings_open_from_the_actions_menu_by_mouse_and_keyboard() {
        for by_mouse in [false, true] {
            let (_, mut app) = super::design_tests::render_with(100, 30);
            app.act(Action::Actions);
            let screen = draw(&mut app, 100, 30);
            show(&screen);
            assert!(screen.contains("Settings"), "Settings is visible in Actions");
            let index = screen::action_items().iter().position(|(_, action)| *action == Action::Settings).unwrap();
            if by_mouse {
                let at = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::ActionRow(index))).then_some((r.x, r.y))).expect("Settings is clickable");
                app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
            } else {
                for _ in 0..index { app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)); }
                app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            }
            assert!(matches!(app.mode, Mode::ScreenSettings(_)), "Settings opens from Actions");
            let screen = draw(&mut app, 100, 30);
            show(&screen);
            assert!(screen.contains("General") && screen.contains("Appearance"));
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert_eq!(app.mode, Mode::Normal);
        }
    }

    #[test]
    fn settings_open_as_a_centered_modal_without_reflow_and_tabs_are_mouse_reachable() {
        for (width, height) in [(80, 24), (100, 30), (160, 45)] {
            let (_, mut app) = super::design_tests::render_with(width, height);
            let before = screen::grid(&app, Rect::new(0, 0, width, height));
            app.act(Action::Settings);
            let screen = draw(&mut app, width, height);
            show(&screen);
            let grid = screen::grid(&app, Rect::new(0, 0, width, height));
            assert_eq!(grid.sheet.width, 0, "Settings does not create a side sheet at {width}x{height}");
            assert_eq!(grid.panes, before.panes, "Settings leaves the workspace geometry unchanged at {width}x{height}");
            assert!(screen.contains("Settings") && screen.contains("General"), "Settings renders in the centered modal at {width}x{height}");
        }

        let (_, mut app) = super::design_tests::render_with(100, 30);
        app.act(Action::Settings);
        draw(&mut app, 100, 30);
        let (x, y) = app.hits.iter().find_map(|(r, hit)| (*hit == Hit::Screen(screen::ScreenHit::SetTab(2))).then_some((r.x, r.y))).expect("Appearance tab is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: x, row: y, modifiers: KeyModifiers::NONE });
        assert!(matches!(&app.mode, Mode::ScreenSettings(view) if view.cat == 2), "mouse opens Appearance without closing the modal");
    }

    #[test]
    fn tiled_square_and_faded() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let a = app.focused().unwrap();
        let b = app.snap.terms.keys().copied().find(|t| *t != a).unwrap();
        app.screen.tabs.clear();
        app.screen_place(a, None);
        app.screen.pending_split = Some((a, Instant::now()));
        app.screen_place(b, Some(a));
        for t in [a, b] {
            let mut p = vt100::Parser::new(20, 60, 0);
            p.process(b"\x1b[31mred text\x1b[0m");
            app.parsers.insert(t, p);
        }
        // The one you're not in fades toward its ground; the one you're in doesn't.
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let focus = app.focused().unwrap();
        let fg_at = |term: TermId| {
            let (_, inner) = *app.panes.iter().find(|(t, _)| *t == term).unwrap();
            buf[(inner.x, inner.y)].fg
        };
        let other = if focus == a { b } else { a };
        assert_ne!(fg_at(focus), fg_at(other), "the unfocused pane's text is faded");
        // Tiled: no margin, cards edge to edge; square corners when asked.
        app.cfg.ui.panes = "tiled".into();
        app.cfg.ui.corners = "flush".into();
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.lines().next().unwrap().starts_with(" ▁"), "tiled starts at the very corner, flush: {}", o.lines().next().unwrap());
        assert!(!o.contains('╭'), "no rounded corners");
    }

    #[test]
    fn a_small_window_tightens() {
        let (o, _) = super::design_tests::render_with(100, 30);
        show(&o);
        let lines: Vec<&str> = o.lines().collect();
        // A 26-column sidebar card, no right-hand meta or question lines.
        assert!(lines[1].starts_with("  ▁") && lines[1].trim().chars().filter(|c| *c == '▁').count() == 24, "a slim sidebar: {}", lines[1]);
        let side: String = lines.iter().map(|l| l.chars().take(30).collect::<String>() + "\n").collect();
        assert!(!side.contains("main · 3m") && !side.contains("Run npm test"), "no meta or questions in the slim sidebar: {side}");
        assert!(o.contains("+ new") && o.contains("actions"), "both controls fit at the sidebar foot");
        assert!(!o.contains("? keys") && !o.contains("inbox"), "there is no full-width status footer");
    }

    #[test]
    fn flat_layout_keeps_the_same_sessions_and_exposes_focus_zone_controls() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.layout = "flat".into();
        let main = draw(&mut app, 160, 45);
        show(&main);
        assert!(main.lines().next().unwrap().starts_with("▎"), "flat has no outside margin and keeps a zone bar");
        assert!(main.contains("claude") && main.contains("AGENTS 2"), "the existing sessions remain visible");
        assert!(main.contains("+ new") && main.contains("actions"), "the sidebar keeps its two mouse-accessible controls");
        assert!(!main.lines().last().unwrap_or_default().contains("PANE"), "the full-width footer is gone");
        assert!(!main.contains('╭') && !main.contains('╮') && !main.contains('╰') && !main.contains('╯'), "flat uses square zones without card borders");
        assert!(app.hits.iter().any(|(_, hit)| *hit == Hit::Screen(screen::ScreenHit::NewShell)), "New is clickable from the sidebar foot");
        assert!(app.hits.iter().any(|(_, hit)| *hit == Hit::Screen(screen::ScreenHit::Actions)), "Actions is clickable from the sidebar foot");

        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Side, "Tab focuses the sidebar");
        let sidebar = draw(&mut app, 160, 45);
        assert!(sidebar.contains("+ new") && sidebar.contains("actions"), "sidebar navigation stays visible while focused");
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal, "Tab returns focus to the pane");

        app.act(Action::Settings);
        let settings = draw(&mut app, 160, 45);
        assert!(settings.contains("Settings") && settings.contains("Esc close"), "the Settings modal has its square status row");
        assert!(app.hits.iter().any(|(rect, hit)| rect.y > 0 && *hit == Hit::Screen(screen::ScreenHit::Close)), "the flat modal close control is mouse reachable");
    }

    #[test]
    fn start_a_fleet_asks_for_a_goal_and_starts_its_commander() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        let started = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::protocol::ClientMsg>| {
            std::iter::from_fn(|| rx.try_recv().ok()).find_map(|m| match m {
                crate::protocol::ClientMsg::Command(crate::protocol::Command::NewWorkspace { cwd, cmd, .. }) => Some((cwd, cmd)),
                crate::protocol::ClientMsg::Command(crate::protocol::Command::NewWorktree { .. }) => panic!("a commander works from the project's folder, not a worktree"),
                _ => None,
            })
        };
        assert!(screen::action_items().iter().any(|(name, a)| *name == "Start a fleet" && *a == Action::StartFleet), "in the actions list");
        let c: crate::keys::KeySpec = "c".parse().unwrap();
        assert_eq!(app.keymap.prefixed.get(&c), Some(&Action::StartFleet), "with a leader key of its own: c");
        app.screen.saved.draft = "a task left from before".into();
        app.act(Action::StartFleet);
        assert!(matches!(&app.mode, Mode::ScreenPane(np) if np.fleet && np.task.is_empty()), "the goal box, empty");
        for (w, h) in [(160, 45), (100, 30)] {
            let o = draw(&mut app, w, h);
            show(&o);
            assert!(o.contains("Start a fleet") && o.contains("GOAL") && o.contains("is the commander"), "at {w}x{h}");
        }
        // No goal: nothing starts, the box stays.
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(&app.mode, Mode::ScreenPane(np) if np.fleet) && started(&mut rx).is_none());
        // A goal, quotes and all, goes to the commander as it was typed.
        let goal = r#"fix `login` & "signup" for $5, it's urgent"#;
        for c in goal.chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        app.on_key(key(KeyCode::Enter));
        assert_eq!(app.mode, Mode::Normal);
        let (cwd, cmd) = started(&mut rx).expect("the commander starts");
        let said = crate::brief::commander_prompt(goal);
        assert_eq!(cmd.as_deref(), Some(format!("claude {}", app.cfg.quote_for_shell(&said)).as_str()));
        assert!(cwd.is_some_and(|d| d.ends_with("shop-api")), "in the project's own folder");
        assert_eq!(app.screen.saved.draft, "a task left from before", "your task draft is untouched");
        // Esc closes it and starts nothing; a shell can't command one.
        app.act(Action::StartFleet);
        app.on_key(key(KeyCode::Char('x')));
        app.on_key(key(KeyCode::Esc));
        assert!(app.mode == Mode::Normal && started(&mut rx).is_none());
        app.act(Action::StartFleet);
        let shell = screen::np_agents(&app, 0).iter().position(|a| a == "shell").unwrap();
        if let Mode::ScreenPane(np) = &mut app.mode {
            np.task = "a goal".into();
            np.a = shell;
        }
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(&app.mode, Mode::ScreenPane(_)) && started(&mut rx).is_none(), "a fleet needs an agent");
    }

    #[test]
    fn a_goal_survives_each_shells_quoting() {
        let said = crate::brief::commander_prompt("fix `login` & \"signup\" for $5, it's urgent\nthen the tests");
        for (shell, open, apostrophe) in [("pwsh.exe", "'", "''"), ("bash", "'", r"'\''")] {
            let cfg = Config { shell: Some(shell.into()), ..Config::default() };
            let q = cfg.quote_for_shell(&said);
            // Single-quoted, so $, backticks, double quotes and the newline are literal; only
            // the apostrophe needs its escape.
            assert!(q.starts_with(open) && q.ends_with(open), "{shell}: {q}");
            assert_eq!(q[1..q.len() - 1].replace(apostrophe, "'"), said, "{shell}: back to what was typed");
        }
    }

    #[test]
    fn the_sidebar_foot_keeps_new_and_actions_without_a_global_footer() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.screen_side_set(screen::SideItem::Sess(2));
        let o = draw(&mut app, 160, 45);
        show(&o);
        // A row's keys are in its menu; the status line has the global ones.
        let rows: Vec<&str> = o.lines().collect();
        assert!(!rows[2].contains("+ new") && o.contains("+ new") && o.contains("actions"), "new and Actions are at the foot");
        let actions_y = app.hits.iter().find_map(|(rect, hit)| (*hit == Hit::Screen(screen::ScreenHit::Actions)).then_some(rect.y)).unwrap();
        assert!(actions_y > 35, "Actions stays at the bottom of the sidebar");
        assert!(!o.lines().last().unwrap_or_default().contains("keys"), "no full-width status footer");
    }

    #[test]
    fn an_agents_row_menu_makes_it_a_commander() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let typed = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::protocol::ClientMsg>| {
            std::iter::from_fn(|| rx.try_recv().ok()).find_map(|m| match m {
                crate::protocol::ClientMsg::Input { term, data } => Some((term, String::from_utf8_lossy(&data).into_owned())),
                _ => None,
            })
        };
        let offered = |app: &App, t: TermId| app.session_items(t).unwrap().1.into_iter().find(|(l, _)| l == "Fleet commander").map(|(_, a)| a);
        let act = offered(&app, 1).expect("on an agent's row");
        assert!(offered(&app, 3).is_none(), "not on a shell");
        // Its key is f: not a (the foot teaches a for actions), nor one the row already uses.
        let items = app.session_items(1).unwrap().1;
        let key = crate::client::menu::menu_keys(&items)[items.iter().position(|(l, _)| l == "Fleet commander").unwrap()];
        assert_eq!(key, Some('f'));
        // claude is on a prompt: typing there would answer it, so there's nothing to confirm.
        app.menu_act(act.clone());
        assert!(app.mode == Mode::Normal && typed(&mut rx).is_none(), "nothing typed into a prompt");
        // Otherwise it asks first: a slip of a key types nothing.
        app.snap.terms.get_mut(&1).unwrap().status = Status::Idle;
        app.screen_fresh();
        app.menu_act(act.clone());
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.title == "Fleet commander" && !c.danger) && typed(&mut rx).is_none(), "asks before typing");
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Make it commander Enter") && o.contains("Cancel Esc"));
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.mode == Mode::Normal && typed(&mut rx).is_none(), "Esc: nothing sent");
        app.menu_act(act);
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let (term, text) = typed(&mut rx).expect("told to take command");
        assert_eq!((term, text.as_str()), (1, crate::brief::commander_prompt("").as_str()));
        assert!(text.contains("seshi brief") && text.contains("ask me for the goal"));
        app.snap.terms.get_mut(&1).unwrap().asleep = true;
        app.screen_fresh();
        assert!(offered(&app, 1).is_none(), "not while it's asleep");
    }

    #[test]
    fn tabs_close_rename_and_reorder_with_the_mouse() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        app.screen.tabs.clear();
        app.screen_place(1, None);
        app.screen.new_tab = Some((Instant::now(), 1));
        app.screen_place(3, Some(1));
        app.screen.new_tab = Some((Instant::now(), 1));
        app.screen_place(2, Some(3));
        assert_eq!(app.session_tabs().len(), 3, "claude's session with three tabs");
        draw(&mut app, 160, 45);
        let pill = |app: &App, i: usize| app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::TabPick(i))).then_some((r.x + 2, r.y))).unwrap();
        let mouse = |app: &mut App, kind: MouseEventKind, (x, y): (u16, u16)| {
            app.on_mouse(MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE });
            draw(app, 160, 45);
        };
        // The current tab carries a ✕.
        assert!(app.hits.iter().any(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabClose(_)))), "a ✕ on the current tab");
        // Double-click: rename it in its pill.
        let first = app.session_tabs()[0];
        let at = pill(&app, first);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), at);
        let at = pill(&app, first);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), at);
        let at = pill(&app, first);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), at);
        let at = pill(&app, first);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), at);
        assert!(matches!(&app.mode, Mode::RenameTab(nt) if nt.tab == first), "renaming the first tab");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Normal, "Escape closes the rename dialog");
        draw(&mut app, 160, 45);
        // Drag the first onto the third: it moves there, and stays the one you're on.
        let (a, c) = (app.session_tabs()[0], app.session_tabs()[2]);
        let moved = app.screen.tabs[a].focus;
        let at = pill(&app, a);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), at);
        let at = pill(&app, c);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), at);
        let at = pill(&app, c);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), at);
        let order: Vec<TermId> = app.session_tabs().iter().map(|i| app.screen.tabs[*i].focus).collect();
        assert_eq!(order.last(), Some(&moved), "dragged to the end: {order:?}");
        assert_eq!(app.screen.tabs[app.screen.tab].focus, moved, "still the tab you're on");
        // A middle click closes a tab (a shell: no second click needed).
        let shell_tab = app.session_tabs().into_iter().find(|i| app.screen.tabs[*i].focus == 3).unwrap();
        let at = pill(&app, shell_tab);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Middle), at);
        let closed = std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, crate::protocol::ClientMsg::Command(crate::protocol::Command::ClosePane { term: 3 })));
        assert!(closed, "the shell's pane is closed");
    }

    #[test]
    fn clicking_the_sidebar_gives_it_the_keys() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        let side = app.screen.side_rect;
        // Empty space near the bottom of the sidebar card, above its foot.
        let at = (side.x + 4, side.bottom() - 7);
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
        assert_eq!(app.mode, Mode::Side, "the sidebar has the keys");
    }

    #[test]
    fn copy_from_several_panes_at_once() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let (a, b) = (1, 2);
        app.screen.tabs.clear();
        app.screen_place(a, None);
        app.screen.pending_split = Some((a, Instant::now()));
        app.screen_place(b, Some(a));
        for (t, word) in [(a, "alpha"), (b, "beta")] {
            let mut p = vt100::Parser::new(20, 80, 100);
            p.process(format!("ERROR {word} failed\r\nok\r\n").as_bytes());
            app.parsers.insert(t, p);
        }
        draw(&mut app, 200, 50);
        assert!(app.enter_copy(a));
        let key = |app: &mut App, c: KeyCode| app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
        // Search, select the line, Tab to the other pane: the selection is kept and the
        // same search runs there.
        key(&mut app, KeyCode::Char('/'));
        for ch in "ERROR".chars() {
            key(&mut app, KeyCode::Char(ch));
        }
        key(&mut app, KeyCode::Enter);
        key(&mut app, KeyCode::Char('V'));
        key(&mut app, KeyCode::Tab);
        assert_eq!(app.copy_set.len(), 1, "a's piece kept");
        assert!(app.copy_set[0].1.contains("ERROR alpha failed"), "{:?}", app.copy_set);
        let Mode::Copy(c) = &app.mode else { panic!("still copying") };
        assert_eq!((c.term, c.query.as_deref()), (b, Some("ERROR")), "on b, the same search");
        assert_eq!(c.lines[c.cur.0].trim_end(), "ERROR beta failed", "at its match");
        // How it reads once copied together.
        let joined = copy::join_pieces(&[("claude".into(), "ERROR alpha failed".into()), ("codex".into(), "ERROR beta failed\n".into())]);
        assert_eq!(joined, "── claude ──\nERROR alpha failed\n\n── codex ──\nERROR beta failed");
        // Esc: nothing kept.
        key(&mut app, KeyCode::Esc);
        assert!(app.copy_set.is_empty());
    }

    #[test]
    fn a_session_keeps_its_row_unless_it_needs_you() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let rows = |app: &mut App, codex: Status| {
            app.snap.terms.get_mut(&1).unwrap().status = Status::Idle;
            app.snap.terms.get_mut(&2).unwrap().status = codex;
            app.screen_fresh();
            let model = app.screen_model();
            screen::side_lines(app, &model, &app.theme).iter().filter_map(|l| screen::line_term(&model, l)).take(2).collect::<Vec<_>>()
        };
        // claude (1) and codex (2), agents in one group.
        assert_eq!(rows(&mut app, Status::Idle), [1, 2]);
        assert_eq!(rows(&mut app, Status::Working), [1, 2], "starting work doesn't move it");
        assert_eq!(rows(&mut app, Status::Done), [1, 2], "nor does finishing");
        assert_eq!(rows(&mut app, Status::Blocked), [2, 1], "needing you does");
        app.cfg.ui.attention_sort = false;
        assert_eq!(rows(&mut app, Status::Blocked), [1, 2], "sorting off: nothing moves");
    }

    /// claude (1) as a commander: codex (2) is its helper, 4 is codex's, 5 is 4's.
    fn fleet() -> App {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let tab = app.snap.workspaces[0].tabs.iter_mut().find(|tab| tab.layout.contains(2)).unwrap();
        for (id, by) in [(4, 2), (5, 4)] {
            let mut t = app.snap.terms[&2].clone();
            t.id = id;
            app.snap.terms.insert(id, t);
            tab.layout.split(2, crate::layout::Dir::Down, id);
            app.snap.terms.get_mut(&id).unwrap().started_by = Some(by);
        }
        app.snap.terms.get_mut(&2).unwrap().started_by = Some(1);
        for t in app.snap.terms.values_mut() {
            t.status = Status::Idle;
        }
        app.screen_fresh();
        app
    }

    /// The agents' rows, top to bottom, with how deep each sits.
    fn fleet_rows(app: &mut App) -> Vec<(TermId, u8)> {
        app.screen_fresh();
        let model = app.screen_model();
        screen::side_lines(app, &model, &app.theme)
            .iter()
            .filter_map(|l| match l {
                screen::Line::Sess(.., depth) => screen::line_term(&model, l).map(|t| (t, *depth)),
                _ => None,
            })
            .filter(|(t, _)| *t != 3)
            .collect()
    }

    #[test]
    fn helpers_sit_under_the_session_that_started_them() {
        let mut app = fleet();
        assert_eq!(fleet_rows(&mut app), [(1, 0), (2, 1), (4, 2), (5, 2)], "each under its commander, no deeper than two steps");
        // One that needs you goes to the top, and takes its own helpers with it.
        app.snap.terms.get_mut(&2).unwrap().status = Status::Blocked;
        assert_eq!(fleet_rows(&mut app), [(2, 0), (4, 1), (5, 2), (1, 0)]);
        // Sorting off: nothing moves, the fleet stays together.
        app.cfg.ui.attention_sort = false;
        assert_eq!(fleet_rows(&mut app), [(1, 0), (2, 1), (4, 2), (5, 2)]);
        // Its commander closed: an ordinary row, its own helpers still under it.
        app.cfg.ui.attention_sort = true;
        app.snap.terms.get_mut(&2).unwrap().status = Status::Idle;
        app.snap.terms.get_mut(&2).unwrap().started_by = Some(999);
        assert_eq!(fleet_rows(&mut app), [(1, 0), (2, 0), (4, 1), (5, 2)]);
    }

    #[test]
    fn a_fleet_shows_in_the_sidebar() {
        let mut app = fleet();
        // The shell is in another group: started by claude or not, it's an ordinary row there.
        app.snap.terms.get_mut(&3).unwrap().started_by = Some(1);
        app.screen_fresh();
        for (w, h) in [(160, 45), (100, 30)] {
            let o = draw(&mut app, w, h);
            show(&o);
            // Where a name starts, in the sidebar's columns (names show in pane titles too).
            let col = |name: &str| {
                let side = |l: &str| l.chars().take(30).collect::<String>();
                o.lines().find_map(|l| side(l).find(name).map(|i| side(l)[..i].chars().count())).unwrap_or_else(|| panic!("{name} is in the sidebar at {w}x{h}"))
            };
            let model = app.screen_model();
            let name = |t: TermId| model.iter().flat_map(|p| p.sessions()).find(|s| s.term == t).unwrap().name.clone();
            assert_eq!(col(&name(2)), col(&name(1)) + 2, "a helper steps in under its commander");
            assert_eq!(col(&name(3)), col(&name(1)), "a session in another group doesn't");
        }
    }

    #[test]
    fn live_helpers_show_compact_pipeline_stages_without_stage_icons() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.screen_fresh();
        let mut terminal = Terminal::new(TestBackend::new(160, 45)).unwrap();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = terminal.backend().buffer().clone();
        let at = |name: &str| {
            let row = |y| (0..160).map(|x| buf[(x, y)].symbol()).collect::<String>();
            let y = (0..45).find(|&y| row(y).contains(name)).unwrap_or_else(|| panic!("{name} helper row"));
            let x = row(y)[..row(y).find(name).unwrap()].chars().count() as u16;
            (buf[(x - 2, y)].symbol().to_string(), buf[(x - 2, y)].fg, buf[(x, y)].fg, buf[(x - 1, y)].symbol().to_string())
        };
        let (planner, builder, reviewer) = (at("Planning"), at("Building"), at("Reviewing"));
        assert_eq!([planner.0.as_str(), builder.0.as_str(), reviewer.0.as_str()], ["↳"; 3], "the hierarchy marker is the only icon");
        let neutral = app.theme.muted;
        assert_eq!([planner.2, builder.2, reviewer.2], [neutral; 3], "stage labels use the secondary neutral color");
        assert_eq!([planner.1, builder.1, reviewer.1], [neutral; 3], "active helpers use a neutral connector");
        assert_eq!([planner.3.as_str(), builder.3.as_str(), reviewer.3.as_str()], [" "; 3], "leave one clear cell between the tree marker and label");
        assert!((0..45).all(|y| (0..160).all(|x| buf[(x, y)].symbol() != "⚙")), "the generic gear is absent");
    }

    #[test]
    fn sessions_drag_within_their_group_and_click_to_open() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        for t in app.snap.terms.values_mut() {
            t.status = Status::Idle;
        }
        app.screen_fresh();
        draw(&mut app, 160, 45);
        let rows = |app: &mut App| {
            let model = app.screen_model();
            screen::side_lines(app, &model, &app.theme).iter().filter_map(|l| screen::line_term(&model, l)).collect::<Vec<_>>()
        };
        let before = rows(&mut app);
        // claude and codex: agents in one group (the shell is in the terminals section).
        let (first, last) = (before[0], before[1]);
        let at = |app: &App, t: TermId| app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::Session(t))).then_some((r.x + 6, r.y))).unwrap();
        let mouse = |app: &mut App, kind: MouseEventKind, (x, y): (u16, u16)| {
            app.on_mouse(MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE });
            draw(app, 160, 45);
        };
        let (from, to) = (at(&app, last), at(&app, first));
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), to);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), to);
        assert_eq!(rows(&mut app)[0], last, "dragged to the top of its group: {:?}", rows(&mut app));
        // Not across sections: the shell can't be dragged among the agents.
        let shell = *before.last().unwrap();
        assert!(!app.move_session(shell, first), "a terminal stays in its section");
        // A click (no move) opens it.
        let p = at(&app, first);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), p);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), p);
        assert!(app.screen.drag.is_none(), "a click opens it and leaves nothing being dragged");
    }

    #[test]
    fn sessions_drag_into_another_group() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // codex works in another folder (no git): a second agents group.
        let t = app.snap.terms.get_mut(&2).unwrap();
        (t.root, t.top, t.cwd) = (None, None, std::path::PathBuf::from("/work/web"));
        app.screen_fresh();
        draw(&mut app, 160, 45);
        let groups = |app: &App| app.screen_model().iter().map(|p| (p.name.clone(), p.sessions().map(|s| s.term).collect::<Vec<_>>())).collect::<Vec<_>>();
        let header = |app: &App, name: &str| {
            let pi = app.screen_model().iter().position(|p| p.name == name && p.kind == screen::Kind::Agents).unwrap();
            app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::ToggleProj(pi))).then_some((r.x + 4, r.y))).unwrap()
        };
        let row = |app: &App, t: TermId| app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::Session(t))).then_some((r.x + 6, r.y))).unwrap();
        let mouse = |app: &mut App, kind: MouseEventKind, (x, y): (u16, u16)| {
            app.on_mouse(MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE });
            draw(app, 160, 45);
        };
        // claude dragged onto web's name joins web.
        let (from, to) = (row(&app, 1), header(&app, "web"));
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), to);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), to);
        let g = groups(&app);
        assert!(g.iter().any(|(n, ts)| n == "web" && ts.contains(&1) && ts.contains(&2)), "claude is in web now: {g:?}");
        assert!(app.screen.drag.is_none());
        // An agent can't go into the terminals section.
        let shell_group = app.screen_model().iter().position(|p| p.kind == screen::Kind::Terminals).unwrap();
        assert!(!app.place_session(1, shell_group), "an agent stays among the agents");
        // It stays when the agent reports its folder again.
        app.screen_fresh();
        assert!(groups(&app).iter().any(|(n, ts)| n == "web" && ts.contains(&1)));
    }

    #[test]
    fn usage_limits_and_a_wait_for_one_show() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        app.snap.limits = vec![("claude".into(), vec![crate::protocol::Limit { name: "5h".into(), used: 82.0, resets_at: now + 3600 + 1200 }])];
        app.snap.spent_today = 4.2;
        let agent = app.snap.terms.values().find(|t| t.agent.is_some()).map(|t| t.id).unwrap();
        let ti = app.snap.terms.get_mut(&agent).unwrap();
        ti.usage = crate::protocol::Usage { context: Some(76.0), cost: Some(1.2) };
        app.screen_focus(agent);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(!o.contains("claude 5h"), "no plan limits on screen");
        assert!(!o.contains("ctx 76%") && !o.contains("$1.20") && !o.contains("76% "), "context and cost are the agent's own to show");
        app.snap.terms.get_mut(&agent).unwrap().resume_at = Some(now + 600);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("resumes in 10m") || o.contains("resumes in 9m"), "waiting on a limit, on its row");
    }



    #[test]
    fn an_update_appears_in_the_tab_bar_only_when_available() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let initial = draw(&mut app, 160, 45);
        assert!(!initial.contains("Update") && initial.contains("actions"), "actions stay in the sidebar, no update when current");
        assert!(app.hits.iter().any(|(_, hit)| *hit == Hit::Screen(screen::ScreenHit::Actions)));
        app.update_available = Some("9.9.9".into());
        app.update_notes = vec!["Fixed: copying".into(), "New: update button".into()];
        let updated = draw(&mut app, 160, 45);
        assert!(updated.contains("Update 9.9.9"), "the top row announces an available update");
        assert!(app.hits.iter().any(|(_, hit)| *hit == Hit::Screen(screen::ScreenHit::Update)), "the update is clickable");
        assert!(app.palette_commands().contains(&Action::Update), "and remains in the command palette");
        let (x, y) = app.hits.iter().find_map(|(rect, hit)| (*hit == Hit::Screen(screen::ScreenHit::Update)).then_some((rect.x, rect.y))).unwrap();
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: x, row: y, modifiers: KeyModifiers::NONE });
        let confirm = draw(&mut app, 160, 45);
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.act == menu::Act::Update));
        assert!(confirm.contains("9.9.9") && confirm.contains("What's new") && confirm.contains("Update now") && confirm.contains("Cancel"));
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.mode == Mode::Normal && !app.updating, "cancel does not install");
    }

    #[test]
    fn x_closes_and_x_confirms() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.menu_act(menu::Act::End(vec![3]));
        assert!(matches!(app.mode, Mode::Confirm(_)), "closing asks first");
        app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Normal), "x (as you asked to close) says yes");
        app.menu_act(menu::Act::End(vec![3]));
        app.on_key(KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Normal), "so does Delete");
    }

    #[test]
    fn groups_drag_to_a_new_place_and_click_to_fold() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // A second group of terminals: the shell (pane 3) in another folder, like "notes".
        let mut notes = app.snap.terms[&3].clone();
        (notes.id, notes.root, notes.top, notes.branch) = (4, None, None, None);
        notes.cwd = PathBuf::from(if cfg!(windows) { r"C:\notes" } else { "/notes" });
        app.snap.terms.insert(4, notes);
        let mut other = app.snap.terms[&3].clone();
        (other.id, other.root, other.top, other.branch) = (5, None, None, None);
        other.cwd = PathBuf::from(if cfg!(windows) { r"C:\api" } else { "/api" });
        app.snap.terms.insert(5, other);
        for (id, t) in [(30, 4), (40, 5)] {
            let mut ws = app.snap.workspaces[0].clone();
            ws.id = id;
            ws.tabs = vec![crate::protocol::TabInfo { id: id + 1, name: String::new(), layout: crate::layout::Node::Leaf(t), focus: t }];
            ws.active_tab = id + 1;
            app.snap.workspaces.push(ws);
        }
        app.screen_fresh();
        draw(&mut app, 160, 45);
        let names = |app: &mut App| app.screen_model().iter().filter(|p| p.kind == screen::Kind::Terminals).map(|p| p.name.clone()).collect::<Vec<_>>();
        let before = names(&mut app);
        assert_eq!(before.len(), 3, "shop-api's shell, notes, api: {before:?}");
        let header = |app: &App, name_idx: usize| {
            app.hits.iter().find_map(|(r, h)| matches!(h, Hit::Screen(screen::ScreenHit::ToggleProj(i)) if app.screen.proj_keys.get(*i).is_some_and(|k| app.screen_model().iter().any(|p| p.key == *k && p.name == before[name_idx]))).then_some((r.x + 4, r.y)))
        };
        let (from, to) = (header(&app, 2).unwrap(), header(&app, 1).unwrap());
        let mouse = |app: &mut App, kind: MouseEventKind, (x, y): (u16, u16)| {
            app.on_mouse(MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE });
            draw(app, 160, 45);
        };
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), to);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), to);
        assert_eq!(names(&mut app), vec![before[0].clone(), before[2].clone(), before[1].clone()], "dragged above the other");
        assert!(!app.screen.saved.closed.iter().any(|k| k.starts_with("p:")), "a drag doesn't fold");
        // A click (no move) folds it.
        let at = header(&app, 0).unwrap();
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), at);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), at);
        assert!(app.screen.saved.closed.iter().any(|k| k.starts_with("p:")), "a click folds");
    }

    #[test]
    fn the_sidebar_has_a_section_per_kind_of_session() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The shell ssh's into a machine; a second shell sits in another folder.
        let t = app.snap.terms.get_mut(&3).unwrap();
        (t.process, t.remote) = ("ssh".into(), Some("build-box".into()));
        let mut notes = app.snap.terms[&3].clone();
        (notes.id, notes.process, notes.remote, notes.root, notes.top, notes.branch) = (4, "bash".into(), None, None, None, None);
        notes.cwd = PathBuf::from(if cfg!(windows) { r"C:\notes" } else { "/notes" });
        app.snap.terms.insert(4, notes);
        let mut ws = app.snap.workspaces[0].clone();
        ws.id = 30;
        ws.tabs = vec![crate::protocol::TabInfo { id: 31, name: String::new(), layout: crate::layout::Node::Leaf(4), focus: 4 }];
        ws.active_tab = 31;
        app.snap.workspaces.push(ws);
        app.screen_fresh();
        let model = app.screen_model();
        let kinds: Vec<(screen::Kind, String)> = model.iter().map(|p| (p.kind, p.name.clone())).collect();
        assert_eq!(
            kinds,
            vec![(screen::Kind::Agents, "shop-api".into()), (screen::Kind::Terminals, "notes".into()), (screen::Kind::Ssh, "build-box".into())],
            "agents, then terminals, then other machines, each grouped by where"
        );
        let o = draw(&mut app, 160, 45);
        show(&o);
        let at = |s: &str| o.find(s).unwrap_or_else(|| panic!("{s} in the sidebar"));
        assert!(at("▾ shop-api") < at("▾ notes") && at("▾ notes") < at("▾ build-box"), "agents, then terminals, then other machines");
        assert!(at("AGENTS 2") < at("TERMINALS 1") && at("TERMINALS 1") < at("SSH 1"), "a heading per section, in order");
    }

    #[test]
    fn a_session_is_grouped_by_where_it_is_now() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The shell cd's out of shop-api into a folder that isn't a repo.
        let elsewhere = PathBuf::from(if cfg!(windows) { r"C:\notes" } else { "/notes" });
        let t = app.snap.terms.get_mut(&3).unwrap();
        (t.cwd, t.root, t.top, t.branch) = (elsewhere.clone(), None, None, None);
        app.screen_fresh();
        let model = app.screen_model();
        let group_of = |term: TermId| model.iter().find(|p| p.sessions().any(|s| s.term == term)).map(|p| p.name.clone());
        assert_eq!(group_of(3).as_deref(), Some("notes"), "it moved to where it is");
        assert_eq!(group_of(1).as_deref(), Some("shop-api"), "the others stay");
    }

    #[test]
    fn a_note_about_a_session_is_a_way_there() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let term = *app.snap.terms.keys().next().unwrap();
        app.notify("claude finished".into(), false);
        app.notice_term = Some(term);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("claude finished") && o.contains("click to open"));
        assert!(app.hits.iter().any(|(_, h)| *h == Hit::Screen(screen::ScreenHit::Session(term))), "clicking it opens the session");
        // Any other note isn't.
        app.notify("saved".into(), false);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("saved") && !o.contains("click to open"));
    }

    #[test]
    fn a_split_is_one_session_and_others_open_full_size() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let a = app.focused().unwrap();
        let others: Vec<TermId> = app.snap.terms.keys().copied().filter(|t| *t != a).collect();
        let (b, c) = (others[0], others[1]);
        app.screen.tabs.clear();
        app.screen_place(a, None);
        app.screen.pending_split = Some((a, Instant::now()));
        app.screen_place(b, Some(a));
        app.screen_fresh();
        let rows: Vec<TermId> = app.screen_model().iter().flat_map(|p| p.sessions().map(|s| s.term).collect::<Vec<_>>()).collect();
        assert!(rows.contains(&a) && !rows.contains(&b), "the pane beside a isn't a session of its own: {rows:?}");
        // Its title bar still says what it is.
        let o = draw(&mut app, 200, 50);
        let name = app.snap.terms[&b].agent.clone().unwrap_or_else(|| "shell".into());
        let bar = o.lines().find(|l| l.contains('✕') && l.matches('✕').count() == 2).unwrap_or_default();
        assert!(bar.matches(name.as_str()).count() >= 1, "the split pane's title names it ({name}): {bar}");
        // b needs you: a's row says so.
        app.snap.terms.get_mut(&b).unwrap().status = Status::Blocked;
        app.screen_fresh();
        let row = app.screen_model().iter().flat_map(|p| p.sessions().cloned().collect::<Vec<_>>()).find(|s| s.term == a).unwrap();
        assert_eq!(row.status, Status::Blocked);
        // Picking another session shows it alone; the split is still there to go back to.
        app.screen_place(c, Some(b));
        assert_eq!(app.screen.tabs[app.screen.tab].layout, crate::layout::Node::Leaf(c), "full size");
        assert_eq!(app.session_tabs().len(), 1, "no tab bar: the split is kept as its session's, not made a tab of this one");
        assert!(app.screen.tabs.iter().any(|t| t.layout.leaves() == vec![a, b]), "the split is kept: {:?}", app.screen.tabs);
        app.screen_place(a, Some(c));
        assert_eq!(app.screen.tabs[app.screen.tab].layout.leaves(), vec![a, b], "its row brings the split back");
    }

    #[test]
    fn closing_a_split_pane_leaves_the_other() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let a = app.focused().unwrap();
        let others: Vec<TermId> = app.snap.terms.keys().copied().filter(|t| *t != a).collect();
        let (b, c) = (others[0], others[1]);
        app.screen.tabs.clear();
        app.screen_place(a, None);
        app.screen.pending_split = Some((a, Instant::now()));
        app.screen_place(b, Some(a));
        assert_eq!(app.screen.tabs[0].layout.leaves().len(), 2);
        // b is closed; the server then focuses some other session, c.
        app.snap.terms.remove(&b);
        app.screen_place(c, Some(b));
        assert_eq!(app.screen.tabs[0].layout, crate::layout::Node::Leaf(a), "a takes the room; c doesn't slide into b's place");
        // Closing from the ✕ takes it out of the split first.
        app.screen.pending_split = Some((a, Instant::now()));
        app.snap.terms.insert(b, app.snap.terms[&c].clone());
        app.screen_place(b, Some(a));
        app.menu_do(menu::Act::End(vec![b]));
        assert_eq!(app.screen.tabs[0].layout, crate::layout::Node::Leaf(a));
    }

    #[test]
    fn copying_pops_a_toast() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.notify("Copied".into(), false);
        let o = draw(&mut app, 160, 45);
        show(&o);
        let lines: Vec<&str> = o.lines().collect();
        let at = lines.iter().position(|l| l.contains("✓ Copied")).expect("a toast");
        assert!(at < lines.len() - 2, "over the panes, above the bottom bar");
        assert!(!lines.last().unwrap().contains("Copied"), "not in the bottom bar");
    }

    #[test]
    fn idle_and_completed_sessions_stay_visible_without_snooze_actions() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.snap.terms.get_mut(&2).unwrap().status = Status::Idle;
        app.snap.terms.get_mut(&3).unwrap().status = Status::Done;
        let model = app.screen_model();
        let rows = screen::side_lines(&app, &model, &app.theme)
            .iter()
            .filter_map(|line| screen::line_term(&model, line))
            .collect::<Vec<_>>();
        assert!(rows.contains(&2) && rows.contains(&3), "resting sessions remain in the sidebar");
        let screen = draw(&mut app, 160, 45);
        assert!(!screen.contains("settled ?") && !screen.contains("Snooze"));
        for term in [2, 3] {
            let items = app.session_items(term).unwrap().1;
            assert!(!items.iter().any(|(label, _)| label.contains("Snooze") || label == "Bring back"));
        }
    }

    #[test]
    fn a_snapshot_is_restored_from_the_list_only_once_you_say_so() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let dir = std::path::PathBuf::from("/code/dsa-tool");
        let kept: Vec<crate::snapshot::Snapshot> =
            (0..12).map(|i| crate::snapshot::Snapshot { commit: format!("c{i}"), at: now - 120 * (i + 1), why: "before claude's turn".into() }).collect();
        // None yet: it says when there will be, and opens nothing.
        app.snapshots_menu(dir.clone(), 1, Vec::new(), (30, 6));
        assert!(!matches!(app.mode, Mode::ScreenMenu(_)));
        // Newest first, numbered, and no more than have a number to press.
        app.snapshots_menu(dir.clone(), 1, kept, (30, 6));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("snapshots · dsa-tool") && o.contains("1  2m ago · before claude's turn") && o.contains("9  18m ago") && !o.contains("20m ago"), "the list");
        // Picking one asks first, and says what happens to what's there now.
        app.on_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        let Mode::Confirm(c) = &app.mode else { panic!("it asks: {:?}", app.mode) };
        assert_eq!(c.act, crate::client::menu::Act::Restore(dir, "c1".into(), "4m ago · before claude's turn".into(), 1));
        assert!(c.note.contains("kept as a snapshot first") && c.yes == "Restore");
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Restore snapshot") && o.contains("dsa-tool"));
    }

    #[test]
    fn a_follow_up_to_a_working_agent_waits_its_turn() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let key = |c: KeyCode, m: KeyModifiers| KeyEvent::new(c, m);
        let sent = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::protocol::ClientMsg>| std::iter::from_fn(|| rx.try_recv().ok()).collect::<Vec<_>>();
        let write = |app: &mut App, text: &str| {
            app.start_compose(2, false, None);
            for c in text.chars() {
                app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            }
        };
        // codex (2) is working: Enter leaves it with the server.
        write(&mut app, "then run the tests");
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("send when it's done") && o.contains("Ctrl+Enter"), "the box says what Enter does now");
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        let got = sent(&mut rx);
        assert!(got.iter().any(|m| matches!(m, crate::protocol::ClientMsg::Command(crate::protocol::Command::QueueFollowUp { term: 2, text }) if text == "then run the tests")), "queued");
        assert!(!got.iter().any(|m| matches!(m, crate::protocol::ClientMsg::Input { term: 2, .. })), "nothing typed into it now");
        // Ctrl+Enter sends it now all the same.
        write(&mut app, "stop");
        app.on_key(key(KeyCode::Enter, KeyModifiers::CONTROL));
        assert!(sent(&mut rx).iter().any(|m| matches!(m, crate::protocol::ClientMsg::Input { term: 2, .. })));
        // Its turn over: Enter sends, as it always did.
        app.snap.terms.get_mut(&2).unwrap().status = Status::Idle;
        write(&mut app, "next thing");
        app.on_key(key(KeyCode::Enter, KeyModifiers::NONE));
        let got = sent(&mut rx);
        assert!(got.iter().any(|m| matches!(m, crate::protocol::ClientMsg::Input { term: 2, .. })) && !got.iter().any(|m| matches!(m, crate::protocol::ClientMsg::Command(crate::protocol::Command::QueueFollowUp { .. }))));
        // What's waiting shows under its row, and can be dropped from its menu.
        app.snap.terms.get_mut(&2).unwrap().queued = vec!["then run the tests".into(), "and open a PR".into()];
        app.mode = Mode::Normal;
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("⏵ next: then run the t"), "under the row: {}", o.lines().find(|l| l.contains("⏵")).unwrap_or(""));
        assert!(app.session_items(2).unwrap().1.iter().any(|(l, a)| l == "Drop queued follow-ups (2)" && *a == crate::client::menu::Act::ClearQueue(2)));
    }

    #[test]
    fn legacy_sleep_state_has_no_special_sidebar_or_preview_ui() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.got_state = true;
        app.snap.terms.get_mut(&2).unwrap().asleep = true;
        app.screen.tabs.clear();
        app.screen_place(1, None);
        app.screen_side_set(screen::SideItem::Sess(2));
        let o = draw(&mut app, 160, 45);
        show(&o);
        let legacy_ui = o.lines().find(|line| line.contains("asleep") || line.contains("zzZ") || line.contains("waking up"));
        assert!(legacy_ui.is_none(), "legacy status UI is absent: {legacy_ui:?}");
        assert!(!o.contains("the last thing codex said"), "sidebar selection doesn't replace the focused pane");
        assert!(!app.panes.iter().any(|(term, _)| *term == 2), "legacy pane preview is removed");
    }

    #[test]
    fn a_session_taking_much_of_the_machine_says_so() {
        let gb = |n: u64| n << 30;
        assert_eq!(screen::load_note(5.0, gb(1)), None, "an ordinary session: nothing");
        assert_eq!(screen::load_note(38.0, gb(1) / 2), Some(("▲ 38% CPU · 0.5 GB".to_string(), false)), "a fifth of the CPU or more");
        assert_eq!(screen::load_note(3.0, gb(3)), Some(("▲ 3% CPU · 3.0 GB".to_string(), false)), "or gigabytes of memory");
        assert!(screen::load_note(60.0, gb(1)).unwrap().1 && screen::load_note(1.0, gb(7)).unwrap().1, "heavy from half the CPU, or six gigabytes");
        // Beside its state in the pane's title, in the working colour when heavy; not in the sidebar.
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let on = app.focused().unwrap();
        (app.snap.terms.get_mut(&on).unwrap().cpu, app.snap.terms.get_mut(&on).unwrap().mem) = (62.0, gb(5));
        let o = draw(&mut app, 160, 45);
        show(&o);
        let title = o.lines().find(|l| l.contains("▲ 62% CPU · 5.0 GB")).expect("said in the pane's title");
        assert!(title.contains('✕'), "on the title border, beside the state: {title}");
        let model = app.screen_model();
        assert!(!screen::side_lines(&app, &model, &app.theme).iter().any(|l| matches!(l, screen::Line::Note(text, ..) if text.starts_with("▲"))), "and no longer under its sidebar row");
        // Its subagents fold away from the heading, and from its menu.
        let with_subs = app.snap.terms.iter().find(|(_, i)| !i.subagents.is_empty()).map(|(id, _)| *id).unwrap();
        assert!(o.contains("▾ 4 subagents"));
        app.on_hy_hit(screen::ScreenHit::ToggleSubs(with_subs), false);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("▸ 4 subagents") && !o.contains("planner"), "folded");
        assert!(app.session_items(with_subs).unwrap().1.iter().any(|(l, a)| l == "Show subagents" && *a == crate::client::menu::Act::ToggleSubs(with_subs)));
    }

    #[test]
    fn a_opens_actions_with_the_keys_in_the_sidebar_too() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.screen_side_set(screen::SideItem::Sess(2));
        assert_eq!(app.mode, Mode::Side);
        app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Actions { .. }), "a is actions here, as the foot says everywhere else");
        // And no row's menu item is ever handed a key the sidebar already uses.
        let items: Vec<(String, crate::client::menu::Act)> = ["Archive", "Ask", "Mark", "Inspect", "Jump", "Keep"].iter().map(|l| (l.to_string(), crate::client::menu::Act::RenamePane(1))).collect();
        let keys = crate::client::menu::menu_keys(&items);
        assert!(keys.iter().flatten().all(|k| !['a', 'm', 'i', 'j', 'k'].contains(k)), "{keys:?}");
    }

    #[test]
    fn the_selected_row_still_says_its_state_in_every_theme() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The colour of a row's name in the sidebar (its first letter).
        let name_fg = |app: &mut App, name: &str| {
            app.screen_fresh();
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            let buf = term.backend().buffer().clone();
            let letters: Vec<String> = name.chars().map(|c| c.to_string()).collect();
            let (x, y) = (0..45u16)
                .flat_map(|y| (0..30u16).map(move |x| (x, y)))
                .find(|&(x, y)| letters.iter().enumerate().all(|(i, l)| buf[(x + i as u16, y)].symbol() == l))
                .unwrap_or_else(|| panic!("{name} is in the sidebar"));
            (buf[(x, y)].fg, buf[(x, y)].bg)
        };
        for theme in crate::theme::BUILTIN.iter().filter(|n| **n != "mono") {
            app.cfg.theme = theme.to_string();
            app.theme = app.cfg.theme();
            let t = app.theme.clone();
            // claude (1) is the selected row, in each state in turn.
            app.screen_side_set(screen::SideItem::Sess(1));
            for (status, want) in [(Status::Blocked, t.on_lit(t.blocked)), (Status::Done, t.on_lit(t.done))] {
                app.snap.terms.get_mut(&1).unwrap().status = status;
                let (fg, bg) = name_fg(&mut app, "claude");
                assert_eq!(bg, t.sel_bg(), "{theme}: the row is lit");
                assert_eq!(fg, want, "{theme}: {status:?} keeps its colour on the lit row");
                assert!(crate::theme::contrast(fg, bg) >= 3.0, "{theme}: and reads there");
            }
            // Working is neutral: never the amber of needs-you or the green of done.
            app.snap.terms.get_mut(&1).unwrap().status = Status::Working;
            let (fg, bg) = name_fg(&mut app, "claude");
            assert!(fg != t.blocked && fg != t.done, "{theme}: working isn't a state colour on the lit row");
            assert!(crate::theme::contrast(fg, bg) >= 3.0, "{theme}: and reads there");
            // No state to show: bright, as before.
            app.snap.terms.get_mut(&1).unwrap().status = Status::Idle;
            assert_eq!(name_fg(&mut app, "claude").0, t.strong, "{theme}: idle is plain bright text");
        }
    }

    #[test]
    fn finished_is_only_said_once_it_has_stayed_finished() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let attention = |app: &mut App, term: TermId, status: Status| {
            app.snap.terms.get_mut(&term).unwrap().status = status;
            app.on_server(crate::protocol::ServerMsg::Attention { term, status });
        };
        let due = |app: &mut App, term: TermId| {
            app.settling.get_mut(&term).unwrap().1 = Instant::now() - Duration::from_millis(1);
            app.settle_alerts();
        };
        // codex (2), not the session you're on.
        app.notice = None;
        attention(&mut app, 2, Status::Done);
        assert!(app.notice.is_none() && app.settling.contains_key(&2), "finished: nothing yet, it waits");
        // It went back to work by itself: nothing is ever said.
        app.snap.terms.get_mut(&2).unwrap().status = Status::Working;
        due(&mut app, 2);
        assert!(app.notice.is_none() && !app.alerted.contains_key(&2), "it carried on: no note, no sound");
        // Finished and still finished when the wait is over: now it's said.
        attention(&mut app, 2, Status::Done);
        due(&mut app, 2);
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.ends_with("finished")) && app.alerted.contains_key(&2) && app.last_alert.is_some_and(|(kind, _)| kind == crate::alert::Kind::Done), "a background completion uses the soft sound");
        // A stable question replaces the pending finish, without ringing twice.
        app.notice = None;
        attention(&mut app, 2, Status::Done);
        attention(&mut app, 2, Status::Blocked);
        assert!(app.notice.is_none());
        due(&mut app, 2);
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.ends_with("needs you")) && app.alerted.contains_key(&2) && !app.settling.contains_key(&2));
        // A session another agent started: its finish is that agent's news, not yours.
        app.notice = None;
        app.snap.terms.get_mut(&2).unwrap().started_by = Some(1);
        attention(&mut app, 2, Status::Done);
        assert!(app.notice.is_none() && !app.settling.contains_key(&2), "a helper's finish is never said");
        // But it still says when it needs you.
        app.alerted.clear();
        app.last_alert = None;
        attention(&mut app, 2, Status::Blocked);
        due(&mut app, 2);
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.ends_with("needs you")));
        // Its starter gone: it's yours again.
        app.notice = None;
        app.snap.terms.get_mut(&2).unwrap().started_by = Some(999);
        attention(&mut app, 2, Status::Done);
        assert!(app.settling.contains_key(&2));
    }

    #[test]
    fn transient_or_stale_blockers_never_ring_while_working() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        app.notice = None;
        app.snap.terms.get_mut(&2).unwrap().status = Status::Blocked;
        app.on_server(ServerMsg::Attention { term: 2, status: Status::Blocked });
        assert!(!app.alerted.contains_key(&2) && app.notice.is_none(), "a transient ask must settle before ringing");
        let mut working = app.snap.clone();
        working.terms.get_mut(&2).unwrap().status = Status::Working;
        app.on_server(ServerMsg::State(working));
        app.settle_alerts();
        assert!(!app.alerted.contains_key(&2) && app.notice.is_none());
        // Attention can arrive after a newer working snapshot.
        app.on_server(ServerMsg::Attention { term: 2, status: Status::Blocked });
        let due = app.settling.get_mut(&2).unwrap();
        due.1 = Instant::now() - Duration::from_millis(1);
        app.settle_alerts();
        assert!(!app.alerted.contains_key(&2) && app.notice.is_none(), "stale attention must agree with the snapshot");
    }

    #[test]
    fn focused_blockers_ring_and_a_new_turn_can_ask_again() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let id = app.focused().unwrap();
        app.window_focused = true;
        app.on_server(ServerMsg::Attention { term: id, status: Status::Blocked });
        app.settling.get_mut(&id).unwrap().1 = Instant::now() - Duration::from_millis(1);
        app.settle_alerts();
        assert!(app.alerted.contains_key(&id), "a focused blocking question still needs its request sound");
        app.last_alert = Some((crate::alert::Kind::Needs, Instant::now() - Duration::from_secs(5)));
        let mut next = app.snap.clone();
        next.terms.get_mut(&id).unwrap().status = Status::Working;
        app.on_server(ServerMsg::State(next));
        app.on_server(ServerMsg::Attention { term: id, status: Status::Blocked });
        let mut blocked = app.snap.clone();
        blocked.terms.get_mut(&id).unwrap().status = Status::Blocked;
        app.on_server(ServerMsg::State(blocked));
        app.settling.get_mut(&id).unwrap().1 = Instant::now() - Duration::from_millis(1);
        app.settle_alerts();
        assert!(app.alerted[&id].1.elapsed() < Duration::from_secs(1), "a new turn's question is new news");
    }

    #[test]
    fn worktrees_are_cleaned_up_from_one_list() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        let standing = |dirty: usize, merged: bool| crate::tidy::Standing { dirty, branch: Some("b".into()), merged, last_commit: "3 days ago".into() };
        let row = |name: &str, sessions: usize, s: crate::tidy::Standing| screen::TidyRow { path: PathBuf::from(format!("/wt/{name}")), name: name.into(), branch: format!("feat/{name}"), sessions, ticked: sessions == 0 && s.safe(), standing: s };
        // It opens from the project's menu and the palette, and says it's looking.
        let pi = app.screen_model().iter().position(|p| p.git).unwrap();
        assert!(app.project_items(pi).unwrap().1.iter().any(|(l, a)| l == "Clean up worktrees…" && *a == crate::client::menu::Act::TidyOpen(pi)));
        assert!(app.palette_commands().contains(&Action::TidyWorktrees));
        app.mode = Mode::Tidy(Box::new(screen::TidyView { repo: PathBuf::from("/repo"), name: "shop-api".into(), rows: None, sel: 0 }));
        assert!(draw(&mut app, 160, 45).contains("looking at each worktree"));
        // Looked at: one safe, one never merged, one with edits, one in use.
        app.tidy_loaded(PathBuf::from("/repo"), vec![row("done", 0, standing(0, true)), row("parked", 0, standing(0, false)), row("messy", 0, standing(2, false)), row("busy", 1, standing(0, true))]);
        let ticked = |app: &App| match &app.mode {
            Mode::Tidy(v) => v.rows.as_ref().unwrap().iter().filter(|r| r.ticked).map(|r| r.name.clone()).collect::<Vec<_>>(),
            m => panic!("the list is open: {:?}", std::mem::discriminant(m)),
        };
        assert_eq!(ticked(&app), ["done"], "only what holds nothing of its own comes ticked");
        for (w, h) in [(160, 45), (100, 30)] {
            let o = draw(&mut app, w, h);
            show(&o);
            assert!(o.contains("Clean up worktrees") && o.contains("✓ done") && o.contains("○ parked") && o.contains("· busy") && o.contains("Remove 1"), "at {w}x{h}");
        }
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("merged, nothing uncommitted") && o.contains("not merged anywhere") && o.contains("2 uncommitted changes") && o.contains("in use: 1 session"));
        // One in use can't be ticked; the others are yours to tick.
        for _ in 0..3 {
            app.on_key(key(KeyCode::Down));
        }
        app.on_key(key(KeyCode::Char(' ')));
        assert_eq!(ticked(&app), ["done"], "busy stays");
        app.on_key(key(KeyCode::Up));
        app.on_key(key(KeyCode::Char(' ')));
        assert_eq!(ticked(&app), ["done", "messy"]);
        // Enter asks, and says what would be lost.
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.danger && c.what == "2 worktrees" && c.note.contains("1 has uncommitted changes, which will be lost")), "asks first");
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("messy (2 uncommitted)"));
        assert!(rx.try_recv().is_err(), "nothing sent before you confirm");
        app.on_key(key(KeyCode::Enter));
        let sent = std::iter::from_fn(|| rx.try_recv().ok()).find_map(|m| match m {
            crate::protocol::ClientMsg::Command(crate::protocol::Command::TidyWorktrees { items }) => Some(items),
            _ => None,
        });
        assert_eq!(sent, Some(vec![(PathBuf::from("/wt/done"), false), (PathBuf::from("/wt/messy"), true)]), "only the edited one is forced");
        // Nothing ticked: Enter says so and the list stays.
        app.mode = Mode::Tidy(Box::new(screen::TidyView { repo: PathBuf::from("/repo"), name: "shop-api".into(), rows: Some(vec![row("parked", 0, standing(0, false))]), sel: 0 }));
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Tidy(_)));
        // a ticks every safe one; n unticks all.
        app.tidy_loaded(PathBuf::from("/repo"), vec![row("done", 0, standing(0, true)), row("parked", 0, standing(0, false))]);
        app.on_key(key(KeyCode::Char('n')));
        assert!(ticked(&app).is_empty());
        app.on_key(key(KeyCode::Char('a')));
        assert_eq!(ticked(&app), ["done"]);
    }

    #[test]
    fn you_can_ask_for_an_update_check_and_are_told_what_it_found() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let said = |app: &App| app.notice.as_ref().map(|(m, _, err)| (m.clone(), *err)).unwrap();
        assert!(app.palette_commands().contains(&Action::CheckUpdate), "in the palette");
        // Settings: beside the version, when no newer one is known.
        app.act(Action::Settings);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(&format!("seshi {}", env!("CARGO_PKG_VERSION"))) && o.contains("up to date") && o.contains("u check now"));
        assert!(app.hits.iter().any(|(_, h)| *h == Hit::Screen(screen::ScreenHit::CheckUpdate)), "it can be clicked");
        // While it's asking, the button says so and can't be pressed twice.
        app.update_checking = true;
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("checking…") && !app.hits.iter().any(|(_, h)| *h == Hit::Screen(screen::ScreenHit::CheckUpdate)));
        // Nothing newer: you asked, so it says so. A check seshi made itself stays quiet.
        app.notice = None;
        app.update_checked_now(false, Ok(None), Vec::new());
        assert!(app.notice.is_none(), "its own checks say nothing when there's nothing new");
        app.update_checked_now(true, Ok(None), Vec::new());
        assert!(!app.update_checking && said(&app) == (format!("Installed seshi {} is the latest", env!("CARGO_PKG_VERSION")), false));
        app.update_checked_now(true, Err("couldn't reach GitHub: timed out".into()), Vec::new());
        assert!(said(&app).1 && said(&app).0.contains("Couldn't check for updates"), "{:?}", said(&app));
        // A newer one: the Update button takes the place of the check.
        app.update_checked_now(true, Ok(Some("9.9.9".into())), vec!["New: something".into()]);
        assert_eq!(app.update_available.as_deref(), Some("9.9.9"));
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Update now") && !o.contains("Check for updates"));
    }

    #[test]
    fn legacy_sleep_recovery_keeps_the_saved_screen_without_a_wake_overlay() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.got_state = true;
        app.snap.terms.get_mut(&1).unwrap().asleep = true;
        app.parsers.get_mut(&1).unwrap().process(b"Saved terminal contents\r\n");
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Saved terminal contents"), "the last screen remains readable");
        assert!(!o.contains("waking up") && !o.contains("to wake it"), "there is no wake screen or animation");
    }

    #[test]
    fn a_toast_wears_the_colour_of_its_news() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The fill and ink under the first letter of `word`, and the mark in front of it.
        let toast = |app: &mut App, word: &str| {
            app.screen_fresh();
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            let buf = term.backend().buffer().clone();
            let first = word.chars().next().unwrap().to_string();
            let (x, y) = (0..45u16)
                .flat_map(|y| (0..160u16).map(move |x| (x, y)))
                .find(|&(x, y)| buf[(x, y)].symbol() == first && (0..word.chars().count() as u16).all(|i| Some(buf[(x + i, y)].symbol()) == word.chars().nth(i as usize).map(|c| c.to_string()).as_deref()))
                .unwrap_or_else(|| panic!("{word} is on screen"));
            (buf[(x, y)].bg, buf[(x, y)].fg, buf[(x - 2, y)].symbol().to_string())
        };
        let t = app.theme.clone();
        app.notify("Copied".into(), false);
        assert_eq!(toast(&mut app, "Copied"), (t.accent, t.ink_on(t.accent), "✓".to_string()), "a plain note: the accent");
        app.notify("Couldn't save".into(), true);
        assert_eq!(toast(&mut app, "Couldn't"), (t.err, t.ink_on(t.err), "✕".to_string()), "an error");
        // claude (1) is on a prompt; codex (2) isn't.
        app.notify("pane-one needs you".into(), false);
        app.notice_term = Some(1);
        assert_eq!(toast(&mut app, "pane-one"), (t.blocked, t.ink_on(t.blocked), "●".to_string()), "needs you: its state's colour");
        app.notify("pane-two finished".into(), false);
        app.notice_term = Some(2);
        assert_eq!(toast(&mut app, "pane-two"), (t.done, t.ink_on(t.done), "✓".to_string()), "finished");
        for fill in [t.accent, t.err, t.blocked, t.done] {
            assert!(fill != t.card2 && fill != t.card, "never the card's own grey");
        }
    }

    #[test]
    fn go_to_a_project_or_session() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        app.act(Action::Jump);
        let o = draw(&mut app, 160, 45);
        show(&o);
        // claude needs you in the fixture: the Inbox, and a search for any session.
        assert!(o.contains("▁ Inbox") && o.contains("type to find any session"));
        for c in "rate".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("FOUND 1 ─") && o.contains("rate  codex · shop-api"), "typing finds what matches");
        let Mode::GoTo { query, sel } = app.mode.clone() else { panic!("still open") };
        let rows = screen::goto_rows(&app.screen_model(), &query, 0);
        assert!(matches!(rows[sel], screen::GoRow::Sess(..)), "it lands on the matching session");
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Normal), "Enter goes there");
    }

    #[test]
    fn palette_and_keys_look_like_the_app() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.act(Action::Palette);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Command palette") && o.contains("Inbox: what needs you") && !o.contains("workspace ·"), "commands only, in a panel");
        for c in "split".chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Split right") && !o.contains("Settings"), "typing narrows it");
        app.mode = Mode::KeyMap(Box::new(screen::KeyMap { query: String::new(), searching: false, step: None, sel: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        let a = o.lines().find(|l| l.contains("inbox ● 1")).unwrap();
        let b = o.lines().find(|l| l.contains("new shell here")).unwrap();
        let column = |line: &str, label: &str| line.find(label).map(|byte| line[..byte].chars().count());
        assert_eq!(column(a, "inbox"), column(b, "new shell here"), "labels line up by terminal column");
    }

    #[test]
    fn settings_keys_tab_is_current() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let cat = modal::Cat::ALL.iter().position(|c| *c == modal::Cat::Keys).unwrap();
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat, sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("GET AROUND") && o.contains("Inbox: what needs you") && o.contains("Command palette"));
    }

    #[test]
    fn folders_and_programs_are_picked_not_typed() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let rows = design::settings_rows(modal::Cat::General);
        let row_of = |p: &str| rows.iter().position(|r| matches!(r, design::SRow::Setting(s) if s.path == p)).unwrap();
        // Editor and shell: chips of what's installed, "default" first.
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: row_of("editor"), editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        let line = o.lines().find(|l| l.contains("Editor")).unwrap();
        assert!(line.contains("default"), "a choice, not a text box: {line}");
        let editor = modal::SETTINGS.iter().find(|s| s.path == "editor").unwrap();
        let opts = modal::program_options(modal::EDITORS, "");
        if opts.len() > 1 {
            assert_eq!(modal::step(&app.cfg, editor, 1).and_then(|v| v.as_str().map(String::from)).as_deref(), Some(opts[1].as_str()), "→ picks the next installed one");
        }
        // The start folder: Enter opens the folder browser for it; Esc goes back to the row.
        let sf = row_of("ui.start_dir");
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: sf, editing: None, capturing: false, scroll: 0 }));
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(&app.mode, Mode::Finder(fd) if fd.for_setting == Some("ui.start_dir")), "the folder browser, picking for the setting");
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Choose the start folder"));
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(&app.mode, Mode::ScreenSettings(v) if v.sel == sf), "Esc: back to Settings, on that row");
    }

    #[test]
    fn working_names_shimmer() {
        use ratatui::style::{Color, Style};
        let (base, bright) = (Color::Rgb(200, 160, 0), Color::Rgb(255, 255, 255));
        let at = |frame| screen::shimmer("claude", frame, base, bright, Style::default()).iter().map(|(_, s)| s.fg).collect::<Vec<_>>();
        assert_eq!(at(0).len(), 6, "one colour per letter");
        assert_ne!(at(4), at(7), "the bright band moves");
        assert!(at(7).contains(&Some(base)), "the rest stays the working colour");
    }

    #[test]
    fn the_inbox_takes_the_column_when_narrow_and_changes_is_a_sheet() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        app.act(Action::Jump);
        let o = draw(&mut app, 100, 30);
        show(&o);
        assert!(o.contains("▁ Inbox") && !o.contains("▁ claude"), "narrow: the sheet covers the pane column");
        app.mode = Mode::Normal;
        // Changes of a real repo.
        let repo = std::env::temp_dir().join(format!("seshi-sheet-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        crate::proc::git(&repo, &["init", "-q"]).unwrap();
        app.open_changes(repo.clone());
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("▁ Changes") && o.contains("▁ claude"), "Changes docks beside the panes");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn a_follow_up_and_why_open_beside_the_row() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |app: &mut App, c: KeyCode, m: KeyModifiers| app.on_key(KeyEvent::new(c, m));
        // claude's row (it needs you): m writes to it from the sidebar.
        app.mode = Mode::Side;
        app.screen.cursor = Some(1);
        key(&mut app, KeyCode::Char('m'), KeyModifiers::NONE);
        assert!(matches!(&app.mode, Mode::Compose(c) if c.term == 1 && c.side), "m: a follow-up to the row's agent");
        for ch in "use the fake clock".chars() {
            key(&mut app, KeyCode::Char(ch), KeyModifiers::NONE);
        }
        key(&mut app, KeyCode::Enter, KeyModifiers::SHIFT);
        key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("message claude") && o.contains("Run npm test -- checkout?") && o.contains("use the fake clock") && o.contains("5 words"), "the question, then the text, its word count");
        assert!(o.contains("◂"), "it points at the row");
        key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.mode == Mode::Side, "sent: back on the sidebar");
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.contains("Sent to claude")), "{:?}", app.notice);
        // i: why it has its status.
        let info = app.snap.terms.get_mut(&1).unwrap();
        (info.status_why, info.why_hook, info.why_hook_at) = ("hook: Notification".into(), "Notification".into(), 1);
        key(&mut app, KeyCode::Char('i'), KeyModifiers::NONE);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("why ") && o.contains("hook     Notification") && o.contains("most specific source wins"), "the evidence, the winner marked");
        key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(app.mode == Mode::Side);
        // In the Inbox, m opens the box inside the row.
        app.mode = Mode::Normal;
        app.act(Action::Jump);
        key(&mut app, KeyCode::Char('m'), KeyModifiers::NONE);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(matches!(&app.mode, Mode::Compose(c) if c.inbox.is_some()) && o.contains("▁ Inbox") && o.contains("Next prompt for claude"), "the box in the Inbox row");
        key(&mut app, KeyCode::Esc, KeyModifiers::NONE);
        assert!(matches!(app.mode, Mode::GoTo { .. }), "Esc: back in the Inbox");
    }

    #[test]
    fn overlapping_files_stay_in_the_inbox_instead_of_cluttering_the_sidebar() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let model = app.screen_model();
        let p = model.iter().find(|p| p.git && p.wts.len() >= 2).expect("a repo with a worktree").clone();
        let names: Vec<String> = p.wts.iter().map(|w| w.name.clone()).collect();
        let shared = |n: usize| (0..n).map(|i| crate::client::overlap::Overlap { file: format!("src/file{i}.ts"), checkouts: names.clone() }).collect::<Vec<_>>();
        // Sixty overlapping changes remain available in the Inbox without crowding the sidebar.
        app.screen.overlaps.insert(p.path.clone(), shared(60));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(!o.contains("shared files") && !o.contains("file0.ts"), "sidebar stays focused on sessions");
        let lines = screen::side_lines(&app, &app.screen_model(), &app.theme);
        assert!(lines.len() < 20, "the sidebar stays short: {} lines", lines.len());
        // A couple are still available through the Inbox.
        app.screen.overlaps.insert(p.path.clone(), shared(2));
        let o = draw(&mut app, 160, 45);
        assert!(!o.contains("file0.ts") && !o.contains("file1.ts"));
        // In the Inbox they come after what needs you and what finished, however many.
        app.screen.overlaps.insert(p.path.clone(), shared(60));
        let rows = screen::goto_rows(&app.screen_model(), "", screen::heads_up(&app).len());
        let at = |want: &str| rows.iter().position(|r| matches!(r, screen::GoRow::Head(h) if *h == want)).unwrap_or_else(|| panic!("{want} is a section"));
        assert!(at("NEEDS YOU") < at("JUST FINISHED") && at("JUST FINISHED") < at("HEADS UP"));
    }

    #[test]
    fn heads_up_and_review_from_the_inbox() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |app: &mut App, c: char| app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        // The fixture's repo and its two checkouts (the main folder, rate-limit).
        let model = app.screen_model();
        let p = model.iter().find(|p| p.git && p.wts.len() >= 2).expect("a repo with a worktree").clone();
        let names: Vec<String> = p.wts.iter().map(|w| w.name.clone()).collect();
        let o = crate::client::overlap::Overlap { file: "src/checkout.ts".into(), checkouts: names.clone() };
        app.screen.overlaps.insert(p.path.clone(), vec![o]);
        app.screen_fresh();
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(!o.contains("checkout.ts"), "worktree overlap labels stay out of the sidebar");
        // In the Inbox: HEADS UP, with what you can do about it.
        app.act(Action::Jump);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("HEADS UP") && o.contains("both changed checkout.ts") && o.contains("both diffs") && o.contains(" dismiss"), "the heads-up row");
        let Mode::GoTo { query, .. } = app.mode.clone() else { panic!("the Inbox") };
        let rows = screen::goto_rows(&app.screen_model(), &query, 1);
        let at = rows.iter().position(|r| matches!(r, screen::GoRow::Heads(_))).unwrap();
        app.mode = Mode::GoTo { query: String::new(), sel: at };
        // d: both diffs in the sheet.
        key(&mut app, 'd');
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(matches!(app.view, Some(View::Both(_))) && o.contains("▁ Changes") && o.contains("reading both diffs"), "both diffs, docked");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.view.is_none());
        // k: dismissed (and the tag goes with it).
        app.mode = Mode::GoTo { query: String::new(), sel: at };
        key(&mut app, 'k');
        app.mode = Mode::Normal;
        let o = draw(&mut app, 160, 45);
        assert!(!o.contains("⇆ checkout.ts"), "dismissed: not said again");
        // A finished worktree: merge is asked in its row; Esc backs out.
        let t = app.snap.terms.get_mut(&2).unwrap();
        t.status = Status::Done;
        app.screen_fresh();
        app.act(Action::Jump);
        let rows = screen::goto_rows(&app.screen_model(), "", 0);
        let done = rows.iter().position(|r| matches!(r, screen::GoRow::Done(_))).expect("a finished row");
        app.mode = Mode::GoTo { query: String::new(), sel: done };
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(" merge") && o.contains("throw away"), "what you can do with what it finished");
        key(&mut app, 'M');
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Merge into main, then remove the worktree and branch?") && o.contains("Cancel"), "asked in the row");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.screen.inbox_confirm.is_none() && matches!(app.mode, Mode::GoTo { .. }), "Esc: not merged, still in the Inbox");
    }

    #[test]
    fn the_sheet_and_sidebar_draw_at_every_width_a_slide_passes() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.act(Action::Jump);
        let t = app.theme.clone();
        let model = app.screen_model();
        let col = Rect { x: 40, y: 4, width: 118, height: 40 };
        for step in 0..=20 {
            let frac = step as f32 / 20.0;
            let (sheet, panes) = screen::split_for_sheet(160, col, 2, frac);
            assert!(panes.width + sheet.width <= col.width, "they share the column at {frac}");
            let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 160, 45));
            if sheet.width >= screen::SHEET_DRAWN_FROM {
                screen::draw_sheet(&mut app, &mut buf, sheet, screen::SheetKind::Inbox, &t);
            }
        }
        for w in 12..=40 {
            let mut buf = ratatui::buffer::Buffer::empty(Rect::new(0, 0, 160, 45));
            screen::draw_side(&mut app, &mut buf, Rect { x: 1, y: 1, width: w, height: 43 }, &model, &t);
        }
    }

    #[test]
    fn the_background_setting_picks_the_terminals_or_a_painted_ground() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let gap = |app: &mut App| {
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            term.backend().buffer()[(0, 0)].bg
        };
        assert_eq!(gap(&mut app), ratatui::style::Color::Reset, "terminal: SGR 49 under the chrome");
        app.cfg.ui.background = "solid".into();
        assert_eq!(gap(&mut app), app.theme.bg, "solid: the theme's own ground");
    }

    #[test]
    fn the_full_width_status_footer_is_removed_in_every_layout() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let cards = draw(&mut app, 160, 45);
        assert!(!cards.lines().last().unwrap_or_default().contains("inbox"));
        assert!(!cards.lines().last().unwrap_or_default().contains("settings"));
        app.cfg.ui.layout = "flat".into();
        let flat = draw(&mut app, 160, 45);
        assert!(!flat.lines().last().unwrap_or_default().contains("PANE"));
        assert!(!flat.lines().last().unwrap_or_default().contains("settings"));
        assert!(flat.contains("+ new") && flat.contains("actions"), "sidebar controls remain in the sidebar");
    }

    #[test]
    fn general_settings_are_in_sections_under_one_version_line() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("up to date") && o.contains("u check now"), "one version line");
        assert!(!o.contains("Installed here") && !o.contains("This window"), "the detail lines only show when something differs");
        let rows = design::settings_rows(modal::Cat::General);
        let sections: Vec<(&str, &str)> = rows.iter().filter_map(|r| match r { design::SRow::Setting(s) => Some((design::settings_group(r), s.label)), _ => None }).collect();
        let pos = |label: &str| sections.iter().position(|(_, l)| *l == label).unwrap_or_else(|| panic!("{label} is on the General page"));
        assert_eq!(sections[pos("Leader key")].0, "INPUT");
        assert_eq!(sections[pos("Layout")].0, "LAYOUT");
        assert_eq!(sections[pos("Editor")].0, "SHELL");
        assert!(pos("Leader key") < pos("Mouse") && pos("Mouse") < pos("Keys after a pause") && pos("Keys after a pause") < pos("Layout"), "input first");
        assert!(pos("Layout") < pos("Background") && pos("Background") < pos("Sidebar side") && pos("Sidebar side") < pos("Splash screen"), "then layout, background, sidebar side, splash");
        assert!(pos("Splash screen") < pos("Editor") && pos("Editor") < pos("Shell"), "then the shell");
        assert!(o.contains("INPUT ─"), "a heading has its rule");
    }

    #[test]
    fn a_finished_note_says_the_name_you_gave_the_session() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.snap.terms.get_mut(&2).unwrap().label = "Checkout rewrite".into();
        app.screen_fresh();
        app.snap.terms.get_mut(&2).unwrap().status = Status::Done;
        app.on_server(crate::protocol::ServerMsg::Attention { term: 2, status: Status::Done });
        app.settling.get_mut(&2).unwrap().1 = Instant::now() - Duration::from_millis(1);
        app.settle_alerts();
        let said = app.notice.as_ref().map(|(m, ..)| m.clone()).unwrap_or_default();
        assert!(said.starts_with("Checkout rewrite") && said.ends_with("finished"), "the renamed session, not its agent: {said}");
    }

    #[test]
    fn only_what_has_the_keyboard_is_lit() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let lit_edges = |app: &mut App| {
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            let buf = term.backend().buffer().clone();
            let (_, pane) = app.pane_frames[0];
            let accent = app.theme.accent;
            // The pane card's left edge and any sheet edge to its right.
            let pane_lit = (pane.y + 1..pane.bottom() - 1).any(|y| buf[(pane.x, y)].fg == accent);
            let sheet_lit = (0..45u16).any(|y| (pane.right()..160u16).any(|x| buf[(x, y)].fg == accent && matches!(buf[(x, y)].symbol(), "▕" | "▏" | "│")));
            (pane_lit, sheet_lit)
        };
        assert_eq!(lit_edges(&mut app), (true, false), "the pane has the keys: its edge is lit");
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: 1, editing: None, capturing: false, scroll: 0 }));
        assert_eq!(lit_edges(&mut app), (false, false), "the modal dims the workspace and overlays it without creating a sheet edge");
    }

    #[test]
    fn a_card_is_filled_up_to_its_line() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        // Flush edges: the sidebar card's left edge is a line on the cell's inner side, on the
        // desk's ground; the card's colour starts in the very next cell (no strip, no spill).
        let side = app.screen.side_rect;
        let y = side.y + side.height / 2;
        assert_eq!(buf[(side.x, y)].symbol(), "▕", "the line hugs the card");
        assert_eq!(buf[(side.x, y)].bg, buf[(side.x + 1, y)].bg, "one ground: the line, not a second grey, marks the card");
        assert_eq!(buf[(side.x, side.y)].symbol(), " ", "the corner is where the two lines meet");
        // Rounded keeps the centred line.
        app.cfg.ui.corners = "rounded".into();
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        assert_eq!(buf[(side.x, y)].symbol(), "│");
        assert_eq!(buf[(side.x, side.y)].symbol(), "╭");
    }

    #[test]
    fn agent_names_wear_their_state() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // Nothing lit by the keyboard, and the window on neither of them.
        app.screen.cursor = None;
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        // The first letter of a sidebar row's name (the sidebar only, not the card titles).
        let side = app.screen.side_rect.right();
        let name_at = |name: &str| {
            (0..45u16).find_map(|y| {
                let row: String = (0..side).map(|x| buf[(x, y)].symbol().to_string()).collect();
                let col = row.find(&format!(" {name} "))?;
                Some(buf[(row[..col + 1].chars().count() as u16, y)].fg)
            })
        };
        let t = &app.theme;
        assert_eq!(name_at("claude"), Some(t.blocked), "needs you: its colour");
        let codex = name_at("rate-limit").or_else(|| name_at("rate")).expect("the working codex in the sidebar (named for its worktree)");
        assert!(codex != t.blocked && codex != t.done, "working is neutral, never amber or green: {codex:?}");
    }

    #[test]
    fn the_split_line_drags_again_and_again() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let a = app.focused().unwrap();
        let b = app.snap.terms.keys().copied().find(|t| *t != a).unwrap();
        app.screen.tabs.clear();
        app.screen_place(a, None);
        app.screen.pending_split = Some((a, Instant::now()));
        app.screen_place(b, Some(a));
        let mouse = |app: &mut App, kind: MouseEventKind, x: u16, y: u16| {
            app.on_mouse(MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE });
            draw(app, 200, 50);
        };
        // The grab area is the gutter, the line its middle column.
        let line_x = |app: &App| app.hits.iter().find_map(|(r, h)| matches!(h, Hit::Screen(screen::ScreenHit::Divider(_))).then_some(r.x + 1));
        // Both programs want the mouse (full-screen agents do).
        for t in [a, b] {
            let mut p = vt100::Parser::new(40, 90, 0);
            p.process(b"\x1b[?1002h\x1b[?1006h");
            app.parsers.insert(t, p);
        }
        draw(&mut app, 200, 50);
        let mut at = line_x(&app).expect("a divider");
        // Grab it on the line, just right of it, just left of it: all of the gutter takes it.
        for (n, (to, off)) in [(at - 30, 0i32), (at + 20, 1), (at - 10, -1)].into_iter().enumerate() {
            let y = 20;
            mouse(&mut app, MouseEventKind::Down(MouseButton::Left), (at as i32 + off) as u16, y);
            mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), to, y);
            mouse(&mut app, MouseEventKind::Up(MouseButton::Left), to, y);
            let now = line_x(&app).expect("still a divider");
            assert_eq!(now, to, "drag {n}: the line is under the pointer");
            at = now;
        }
    }

    #[test]
    fn status_icons_ignore_terminal_bells() {
        let (_, mut app) = super::design_tests::render_with(120, 30);
        for status in [Status::Working, Status::Blocked, Status::Idle, Status::Done] {
            let t = app.snap.terms.get_mut(&2).unwrap();
            (t.status, t.bell) = (status, true);
            app.screen_fresh();
            let o = draw(&mut app, 120, 30); show(&o);
            let row = o.lines().find(|line| line.contains("rate")).unwrap();
            assert!(!row.contains('♪'), "an agent keeps its {status:?} status when it rings: {row}");
        }
        let t = app.snap.terms.get_mut(&3).unwrap();
        (t.status, t.bell) = (Status::None, true);
        let o = draw(&mut app, 120, 30);
        assert!(o.lines().any(|line| line.contains("shell") && line.contains('›') && !line.contains('♪')), "a plain terminal keeps its normal icon when it rings");
        app.snap.terms.get_mut(&3).unwrap().status = Status::Blocked;
        let o = draw(&mut app, 120, 30);
        assert!(o.lines().any(|line| line.contains("shell") && line.contains(&app.cfg.icons.blocked) && !line.contains('♪')), "a terminal asking a question keeps its needs-you icon too");
    }

    #[test]
    fn a_finished_agent_is_a_green_dot_even_if_it_rang() {
        let (_, mut app) = super::design_tests::render_with(120, 30);
        // codex finished and rang the bell (it does both), and you're on another session.
        let t = app.snap.terms.get_mut(&2).unwrap();
        (t.status, t.bell) = (Status::Done, true);
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let row = |y: u16| (0..40u16).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>();
        let y = (0..30).find(|&y| row(y).contains("rate")).expect("codex's row");
        assert!(row(y).contains('●') && !row(y).contains('♪'), "the done dot, not the bell: {:?}", row(y));
        let dot = (0..40u16).find(|&x| buf[(x, y)].symbol() == "●").unwrap();
        assert_eq!(buf[(dot, y)].fg, app.theme.done, "a green dot");
        let name = (0..40u16).find(|&x| buf[(x, y)].symbol() == "r").unwrap();
        assert_eq!(buf[(name, y)].fg, app.theme.done, "and green text");
    }

    #[test]
    fn only_the_part_with_the_keys_looks_focused() {
        let (_, mut app) = super::design_tests::render_with(120, 30);
        // The colour of the focused card's border just right of its ✕, and the sidebar
        // card's left edge.
        let borders = |app: &mut App| {
            let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            let buf = term.backend().buffer().clone();
            let y = (0..30).find(|&y| (0..120).any(|x| buf[(x, y)].symbol() == "✕")).unwrap();
            let x = (0..120).rev().find(|&x| buf[(x, y)].symbol() == "✕").unwrap();
            let side = (1..29).filter(|&y| buf[(1u16, y)].fg == app.theme.accent).count();
            (buf[(x + 2, y)].fg, side)
        };
        let (pane, side) = borders(&mut app);
        assert_eq!(pane, app.theme.accent, "the focused pane's border is the accent");
        assert_eq!(side, 0, "the sidebar's isn't");
        app.act(Action::BrowseTree);
        let (pane, side) = borders(&mut app);
        assert_ne!(pane, app.theme.accent, "not while the sidebar has the keys");
        assert!(side > 10, "the sidebar card is lit instead: {side} rows");
    }

    #[test]
    fn leader_then_a_key_does_it() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.which_key = false;
        let lead = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        for (c, what) in [('j', "inbox"), ('w', "worktrees"), ('?', "keys"), (',', "settings")] {
            app.mode = Mode::Normal;
            app.on_key(lead);
            assert!(matches!(app.mode, Mode::Prefix { .. }), "leader waits");
            let o = draw(&mut app, 160, 45);
            assert!(o.contains("⌨ CTRL+SPACE"), "the leader pill at the end of the tab row");
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            assert!(!matches!(app.mode, Mode::Normal | Mode::Prefix { .. }), "leader + {c} opens {what}: {:?}", std::mem::discriminant(&app.mode));
        }
    }

    #[test]
    fn every_popup_fits_a_tiny_window() {
        use crate::layout::Dir;
        let actions = [
            Action::Palette,
            Action::Help,
            Action::Settings,
            Action::NewPane,
            Action::Jump,
            Action::Files,
            Action::Find(0),
            Action::Find(1),
            Action::Changes,
            Action::Branches,
            Action::History,
            Action::RenameWorkspace,
            Action::BrowseTree,
            Action::SplitRight,
            Action::Focus(Dir::Left),
        ];
        for (w, h) in [(20, 6), (1, 1), (80, 3), (40, 10), (40, 15), (60, 12)] {
            for a in &actions {
                let (_, mut app) = super::design_tests::render_with(160, 45);
                app.act(a.clone());
                let _ = draw(&mut app, w, h);
                // And a right-click menu, and a question.
                app.mode = Mode::Normal;
                app.menu_for_session(1, (w.saturating_sub(1), h.saturating_sub(1)));
                let _ = draw(&mut app, w, h);
                app.menu_act(menu::Act::End(vec![1]));
                let _ = draw(&mut app, w, h);
            }
        }
    }

    #[test]
    fn borrowed_screens_look_like_the_default() {
        for (a, title) in [(Action::RenameWorkspace, "Rename"), (Action::NewWorktree(None), "Worktrees")] {
            let (_, mut app) = super::design_tests::render_with(120, 34);
            app.act(a);
            let o = draw(&mut app, 120, 34);
            show(&o);
            assert!(o.contains("▁ ") && o.contains(" ✕ ▁"), "{title}: a seshi card (title in its border, ✕ to close)");
        }
    }


    #[test]
    fn options_come_from_the_screen() {
        let mut p = vt100::Parser::new(10, 60, 0);
        p.process(b"Question: use WebKit or Safari?\r\n  1. WebKit build\r\n  2. Safari driver");
        assert_eq!(screen::options(Some(&p)), vec!["WebKit build".to_string(), "Safari driver".to_string()]);
        assert_eq!(screen::options(None), vec!["Yes", "Always", "No"]);
    }
}

mod settings_splash_tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn draw(app: &mut App) -> String {
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..45).map(|y| (0..160).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect()
    }

    #[test]
    fn splash_and_settings_render() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.splash = true;
        let splash = draw(&mut app);
        app.splash = false;
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: 1, editing: None, capturing: false, scroll: 0 }));
        let settings = draw(&mut app);
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 2, sel: 8, editing: None, capturing: false, scroll: 0 }));
        let keys = draw(&mut app);
        if std::env::var("SESHI_SHOW").is_ok() {
            println!("{splash}\n{settings}\n{keys}");
        }
        assert!(splash.contains("███████") && splash.contains("every session, one calm place"));
        assert!(splash.contains("Resume where you left off") && splash.contains("New shell here") && !splash.contains("Open a folder"));
        for page in ["General", "Sessions", "Appearance", "Agents", "Keys"] {
            assert!(settings.contains(page), "page {page}");
        }
        assert!(settings.contains("Leader key") && settings.contains("Splash screen"));
        assert!(keys.contains("Theme") && keys.contains("Enter browse"), "the theme control opens the browser");
    }
}

#[cfg(test)]
mod shots {
    //! Not a check: writes the screens the README shows, cell by cell, for the image step.
    //! `SESHI_SHOTS=<dir> cargo test write_the_readme_screens -- --ignored`
    use super::design_tests::render_with;
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    fn cells(app: &mut App, w: u16, h: u16, name: &str, dir: &std::path::Path) {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let mut rows = Vec::new();
        for y in 0..h {
            let row: Vec<serde_json::Value> = (0..w)
                .map(|x| {
                    let c = &buf[(x, y)];
                    let col = |c: ratatui::style::Color| match c {
                        ratatui::style::Color::Rgb(r, g, b) => format!("#{r:02x}{g:02x}{b:02x}"),
                        _ => String::new(),
                    };
                    serde_json::json!([c.symbol(), col(c.fg), col(c.bg), c.modifier.contains(ratatui::style::Modifier::BOLD)])
                })
                .collect();
            rows.push(row);
        }
        std::fs::write(dir.join(format!("{name}.json")), serde_json::to_string(&rows).unwrap()).unwrap();
    }

    #[test]
    #[ignore]
    fn write_the_readme_screens() {
        let dir = std::path::PathBuf::from(std::env::var("SESHI_SHOTS").expect("set SESHI_SHOTS"));
        std::fs::create_dir_all(&dir).unwrap();
        let (_, mut app) = render_with(160, 45);
        cells(&mut app, 160, 45, "02-main", &dir);
        // The agent beside a shell in the same tab.
        let (_, mut app) = render_with(160, 45);
        let mut split = crate::layout::Node::Leaf(1);
        split.split(1, crate::layout::Dir::Right, 3);
        app.screen.tabs[0].layout = split;
        app.screen.tabs[0].focus = 1;
        cells(&mut app, 160, 45, "05-split", &dir);
        let (_, mut app) = render_with(160, 45);
        app.mode = Mode::GoTo { query: String::new(), sel: 0 };
        cells(&mut app, 160, 45, "03-go-to", &dir);
        let (_, mut app) = render_with(160, 45);
        app.mode = Mode::Actions { sel: 0 };
        cells(&mut app, 160, 45, "04-palette", &dir);
        let (_, mut app) = render_with(160, 45);
        app.act(crate::keys::Action::Settings);
        cells(&mut app, 160, 45, "07-settings", &dir);
        let (_, mut app) = render_with(160, 45);
        app.open_keymap(None);
        cells(&mut app, 160, 45, "06-keys", &dir);
        let (_, mut app) = render_with(160, 45);
        app.splash = true;
        cells(&mut app, 160, 45, "01-splash", &dir);
    }
}
