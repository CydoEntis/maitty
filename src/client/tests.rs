//! The client's tests: rendering through TestBackend, keys and clicks.

use super::*;
use crate::brand::{APP_COMMAND};

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
            quiet_since: None,
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
            helper_ask: Vec::new(),
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
            checks: Vec::new(),
        }
    }

    /// The main screen with a few projects and agents, rendered as text.
    pub(super) fn render_with(w: u16, h: u16) -> (String, App) {
        let (tx, _rx) = mpsc::unbounded_channel();
        let (bg_tx, _bg_rx) = mpsc::unbounded_channel();
        let mut cfg = Config::default();
        // The reference's own glyphs: its tests read a state by shape. `dots` is the default.
        cfg.ui.glyphs = "unicode".into();
        // The reference has no padding round the screen; its frames are compared cell for cell.
        cfg.ui.padding = "0".into();
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
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("{s}");
        }
    }

    /// Every key of the overlay's map, as (label, key text), for the current bindings.
    fn all_rows(app: &App) -> Vec<(String, String)> {
        screen::keymap_all_rows(app)
    }

    #[test]
    fn all_keys_lists_every_binding_and_custom_bindings_without_disabled_keys() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        for (key, action) in &app.keymap.prefixed {
            let keys = all_rows(&app).into_iter().filter(|(_, k)| k.split_whitespace().any(|p| p == design::pretty_key(key))).count();
            assert!(keys > 0 || matches!(action, Action::None | Action::SendPrefix), "leader {key} is listed ({})", action.describe());
        }
        // Duplicate keys are gone: one key per action.
        for dropped in ["M", "S", "Z", "H", "J", "K", "L", "X", "F", "B", "W", "R", "V", "N", "{", "}", "|", ":"] {
            let spec: crate::keys::KeySpec = dropped.parse().unwrap();
            assert!(!app.keymap.prefixed.contains_key(&spec), "{dropped} is no longer a leader key");
        }
        app.cfg.prefix = "ctrl+a".into();
        app.cfg.keys.prefix.insert("v".into(), "split-right".into());
        app.cfg.keys.prefix.insert("g".into(), "workflows".into());
        app.cfg.keys.global.insert("alt+0".into(), "spawn-tab:my-agent --review".into());
        app.keymap = app.cfg.keymap();
        assert_eq!(app.keymap.prefixed.get(&"v".parse().unwrap()), Some(&Action::SplitRight), "a user's own binding still loads");
        let rows = all_rows(&app);
        assert!(rows.iter().any(|(label, keys)| label == "Search commands" && keys.split("  ").any(|set| set.split(' ').any(|k| k == "g"))), "{rows:?}");
        assert!(rows.iter().any(|(label, keys)| label.contains("my-agent --review") && keys == "Alt+0"), "{rows:?}");
        app.cfg.keys.replace_defaults = true;
        app.keymap = app.cfg.keymap();
        assert!(!all_rows(&app).iter().any(|(label, keys)| label == "Inbox" && !keys.is_empty()), "no default key survives replace_defaults");
    }

    #[test]
    fn the_key_overlay_opens_as_a_map_searches_lists_all_and_runs_keys() {
        for (w, h) in [(80, 24), (100, 30), (160, 45)] {
            let (_, mut app) = super::design_tests::render_with(w, h);
            let (tx, mut rx) = mpsc::unbounded_channel(); app.out = tx;
            let key = |app: &mut App, c: KeyCode| app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
            // Ctrl+Space, then a pause: the map, with its banner and groups.
            app.open_keymap(None);
            let o = draw(&mut app, w, h); show(&o);
            assert!(o.contains("goes after the leader") && o.contains("= split") && o.contains("Esc"), "{w}x{h}\n{o}");
            for g in ["SESSIONS", "PANES", "TABS", "PROJECT", env!("APP_ENV")] {
                assert!(o.contains(g), "{w}x{h}: the map has {g}");
            }
            assert!(o.contains("run it") && o.contains("show all") && !o.contains("[Esc]"), "{o}");
            // ? lists every key, with the keys that need no leader and the commands with none.
            key(&mut app, KeyCode::Char('?'));
            assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::All));
            let mut seen = String::new();
            for _ in 0..80 {
                seen += &draw(&mut app, w, h);
                app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
            }
            show(&seen);
            assert!(seen.contains("All keys") && seen.contains("WITHOUT THE LEADER") && seen.contains("press directly") && seen.contains("SEARCH ONLY") && seen.contains("Clean up worktrees"), "{w}x{h}: scrolling reaches every part of the list");
            // Typing from there searches; Tab goes back to the map.
            key(&mut app, KeyCode::Char('s'));
            assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::Search && km.query == "s"));
            for c in "idebar".chars() {
                key(&mut app, KeyCode::Char(c));
            }
            let o = draw(&mut app, w, h); show(&o);
            assert!(o.contains("Collapse sidebar") && o.contains("Hide sidebar") && o.contains("Panes"), "{w}x{h}\n{o}");
            key(&mut app, KeyCode::Tab);
            assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::Map));
            key(&mut app, KeyCode::Esc);
            assert_eq!(app.mode, Mode::Normal);
            assert!(rx.try_recv().is_err(), "looking at keys sends nothing");
        }
    }

    #[test]
    fn the_key_overlay_lays_out_three_columns_wide_and_two_narrow() {
        let columns = |w: u16, h: u16| {
            let (_, mut app) = super::design_tests::render_with(w, h);
            app.open_keymap(None);
            let o = draw(&mut app, w, h);
            show(&o);
            let overlay: Vec<&str> = o.lines().skip_while(|l| !l.contains("goes after the leader")).collect();
            let x = |heading: &str| overlay.iter().find_map(|l| l.find(heading).map(|i| l[..i].chars().count())).unwrap_or_else(|| panic!("{heading} at {w}x{h}\n{o}"));
            let mut xs = vec![x("SESSIONS"), x("PANES"), x("TABS"), x("PROJECT"), x(env!("APP_ENV"))];
            xs.sort();
            xs.dedup();
            xs.len()
        };
        assert_eq!(columns(160, 45), 3);
        assert_eq!(columns(100, 30), 2);
    }

    #[test]
    fn space_searches_every_command_and_enter_runs_it() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = mpsc::unbounded_channel(); app.out = tx;
        let lead = KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL);
        app.on_key(lead);
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::NONE));
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::Search && km.query.is_empty()), "Space opens the search");
        let o = draw(&mut app, 160, 45); show(&o);
        assert!(o.contains("RECENT") && o.contains("Inbox"), "nothing typed shows recent commands\n{o}");
        // Every command the old palette listed can be found by name.
        for a in app.palette_commands() {
            let label = a.describe();
            let found = screen::keymap_actions(&app, &screen::KeyMap { query: label.to_lowercase(), ..screen::KeyMap::new(screen::KeyView::Search, None) });
            let direct = app.keymap.global.values().any(|g| *g == a);
            assert!(found.contains(&Some(a.clone())) || label.contains('`') || direct, "{label} is found by search");
        }
        // A command with no key is found and runs.
        for c in "clean up".chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        let o = draw(&mut app, 160, 45); show(&o);
        assert!(o.contains("Clean up worktrees") && o.contains("1 match"), "{o}");
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!matches!(app.mode, Mode::KeyMap(_)), "Enter runs it and closes the overlay");
        assert_eq!(app.screen.saved.recent.first().map(String::as_str), Some("tidy-worktrees"), "and it is recent now");
        let _ = rx.try_recv();
    }

    #[test]
    fn keys_are_keycaps_with_dim_plus_between_the_parts_of_a_combo() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.open_keymap(None);
        let mut terminal = Terminal::new(TestBackend::new(160, 45)).unwrap();
        terminal.draw(|frame| render::draw(&mut app, frame)).unwrap();
        let buffer = terminal.backend().buffer();
        let find = |sym: &str| (0..45).flat_map(|y| (0..160).map(move |x| (x, y))).filter(|&(x, y)| buffer[(x, y)].symbol() == sym).collect::<Vec<_>>();
        let cap = find("i").into_iter().any(|(x, y)| buffer[(x, y)].bg == app.theme.btn && buffer[(x, y)].fg == app.theme.strong);
        assert!(cap, "the Inbox binding is on a raised keycap, strong text");
        // Ctrl+Space in the banner: two caps, and a + in the dim colour between them.
        let plus = find("+").into_iter().any(|(x, y)| buffer[(x, y)].fg == app.theme.muted);
        assert!(plus, "combo parts are joined with a dim +");
        // Its hints sit in a bar of their own, on a lifted ground, as the last row of the card.
        let last_bar = (0..45).rev().find(|&y| (0..160).any(|x| buffer[(x, y)].bg == app.theme.card2 && buffer[(x, y)].symbol() != " ")).expect("an action bar");
        let row: String = (0..160).map(|x| buffer[(x, last_bar)].symbol()).collect();
        assert!(row.contains("run it") && row.contains("Esc"), "{row}");
    }

    #[test]
    fn the_collapsed_strip_puts_the_sessions_on_row_zero_and_keeps_mouse_access() {
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
                let screen = draw(&mut app, w, h);
                show(&screen);
                let g = screen::grid(&app, Rect::new(0, 0, w, h));
                assert_eq!((g.side.width, g.panes.width, g.tabs.y), (0, w, 1), "{w}x{h} {position}: the tabs and panes take the full width and start under the strip");
                let on_row0 = |hit: screen::ScreenHit| app.hits.iter().find(|(r, h)| *h == Hit::Screen(hit) && r.y == 0).map(|(r, _)| *r);
                assert!(on_row0(screen::ScreenHit::Session(1)).is_some() && on_row0(screen::ScreenHit::Session(2)).is_some(), "{w}x{h} {position}: a chip per session");
                assert!(on_row0(screen::ScreenHit::ExpandSidebar).is_some() && on_row0(screen::ScreenHit::Actions).is_some(), "{w}x{h} {position}: expand and Menu stay clickable");
                assert!(screen.lines().nth(1).unwrap().trim_end().ends_with(" +"), "the tabs moved down to row 1: {}", screen.lines().nth(1).unwrap());
                let click = |app: &mut App, r: Rect| {
                    for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
                        app.on_mouse(MouseEvent { kind, column: r.x + 1, row: r.y, modifiers: KeyModifiers::NONE });
                    }
                };
                let chip = on_row0(screen::ScreenHit::Session(2)).unwrap();
                click(&mut app, chip);
                assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, ClientMsg::Command(Command::FocusPane { term: 2 }))));
                // More sessions than fit: a `+n more` chip opens the sidebar.
                for id in 4..60 {
                    let mut t = app.snap.terms[&2].clone();
                    t.id = id;
                    app.snap.terms.insert(id, t);
                    app.snap.workspaces[0].tabs.push(TabInfo { id: 100 + id, name: String::new(), layout: crate::layout::Node::Leaf(id), focus: id });
                }
                let screen = draw(&mut app, w, h);
                assert!(screen.lines().next().unwrap().contains(" more"), "{w}x{h} {position}: chips that don't fit give way to +n more: {}", screen.lines().next().unwrap());
                let expand = app.hits.iter().rev().find(|(r, h)| *h == Hit::Screen(screen::ScreenHit::ExpandSidebar) && r.y == 0 && r.x > 6).map(|(r, _)| *r).expect("the +n more chip");
                click(&mut app, expand);
                assert!(app.sidebar);
                draw(&mut app, w, h);
                assert!(app.screen.side_rect.width > 7);
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
            if row != active_y {
                assert_eq!(terminal.backend().buffer()[(x + 8, active_y)].bg, active_bg, "pointing elsewhere leaves the open row as it is");
            }
            assert_eq!(terminal.backend().buffer()[(x + 2, active_y)].symbol(), "▎");
        }
        app.on_mouse(MouseEvent { kind: MouseEventKind::Moved, column: x + 10, row: hover_y, modifiers: KeyModifiers::NONE });
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        assert_ne!(terminal.backend().buffer()[(x + 8, hover_y)].bg, active_bg, "hover has its own quieter style");
        app.screen_side_set(screen::SideItem::Sess(2));
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = terminal.backend().buffer();
        assert_ne!(buf[(x + 2, active_y)].symbol(), "▎", "while the keyboard browses, only its row has a bar");
        assert_eq!((buf[(x + 2, hover_y)].symbol(), buf[(x + 2, hover_y)].fg), ("▎", app.theme.accent), "the one under the keyboard, in the accent");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: x + 10, row: hover_y, modifiers: KeyModifiers::NONE });
        app.on_mouse(MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column: x + 10, row: hover_y, modifiers: KeyModifiers::NONE });
        assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, ClientMsg::Command(Command::FocusPane { term: 2 }))));
        app.snap.workspaces[0].tabs[0].focus = 2;
        app.screen_sync();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        assert_eq!(app.focused(), Some(2));
        assert_eq!(terminal.backend().buffer()[(x + 2, hover_y)].symbol(), "▎");
        app.snap.terms.get_mut(&2).unwrap().asleep = true;
        app.screen_fresh();
        terminal.draw(|f| render::draw(&mut app, f)).unwrap();
        assert_eq!(terminal.backend().buffer()[(x + 2, hover_y)].symbol(), "▎", "sleep glyphs leave the selection bar visible");
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
    fn the_why_popover_says_a_quiet_agent_may_be_stuck() {
        let (_, mut app) = super::design_tests::render_with(120, 30);
        let t = app.snap.terms.get_mut(&1).unwrap();
        (t.status, t.quiet_since) = (Status::Working, Some(screen::now_secs() - 12 * 60));
        app.mode = Mode::Side;
        draw(&mut app, 120, 30);
        let at = app.hits.iter().filter_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::Why(1))).then_some(*r)).next_back().expect("the status is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.x, row: at.y, modifiers: KeyModifiers::NONE });
        let output = draw(&mut app, 120, 30); show(&output);
        assert!(output.contains("No output or report for 12m — it may be stuck."), "{output}");
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
    fn main_screen_follows_the_reference() {
        let (text, mut app) = super::design_tests::render_with(160, 45);
        show(&text);
        let lines: Vec<&str> = text.lines().collect();
        assert!(!text.contains(concat!(">_ ", env!("APP_COMMAND"))), "no logo");
        // The grid: no margin, the sidebar card the full height from the corner, the tabs on
        // row 0 beside it (ADR 0028), the pane card right under them.
        assert!(lines[0].starts_with("┌─"), "the sidebar card from the very corner: {}", lines[0]);
        assert!(lines[0].contains("● claude"), "the tab keeps its session and state: {}", lines[0]);
        assert!(text.contains("+ New") && text.contains("Menu"), "New and Actions stay in the sidebar foot");
        assert!(lines[0].contains(" claude ✕") && lines[0].trim_end().ends_with("  +") && !lines[0].contains('▯'), "the tab's name on the tab row (a lone tab has no number), then the + chip; no layout switch: {}", lines[0]);
        assert!(!lines[1].contains("claude") && !lines[1].contains("needs you") && !lines[1].contains('✕'), "a lone pane has no title: its tab names it: {}", lines[1]);
        // Projects and their sessions, nothing in between.
        assert!(text.contains("−  shop-api") && text.contains("SESSIONS") && text.contains("●1"), "a project: its fold arrow and name; what needs you in the sidebar's title");
        assert!(!text.contains("AGENTS") && !text.contains("TERMINALS") && !text.contains("BRANCHES") && !text.contains("WORKTREES"), "no heading per kind of session");
        assert!(!text.contains("main folder"), "no 'main folder' wording");
        assert!(!text.contains("+ open a project"), "opening a project is in the header now");
        // Rows: state and name, branch · age on the right, its question underneath.
        assert!(text.contains("● claude") && text.contains(" 3m") && !text.contains("main · 3m"), "state, then the name; age on the right (the agent too when the name isn't it), no branch");
        assert!(text.contains("Run npm test -- checkout?"), "the question under the agent");
        assert!(!text.contains("subagent") && !text.contains("Planning") && !text.contains("Building"), "the sidebar has no subagent tree: {text}");
        assert!(text.contains("⣻ rate") && text.contains(" 2m"), "a worktree's session is named after it");
        assert!(!text.contains("Rate limit /login"), "under a session only its question, as in the redesign");
        assert!(text.contains("› shell") && !text.contains("shell 2"), "a shell in the project's folder is just 'shell', no age");
        assert!(!text.contains("session"), "no 'session' wording on screen");
        assert!(!text.contains("● answer") && !text.contains(" Yes 1 "), "an agent's own question is answered in its own prompt: no answer bar");
        // Overlays are centred over a dimmed screen.
        for (mode, needle) in [
            (Mode::GoTo { query: String::new(), sel: 1 }, "NEEDS YOU"),
            (Mode::ScreenPane(screen::NewPaneHy::new(0, false)), "claude gets its own new worktree in shop-api"),
            (Mode::ScreenPane(screen::NewPaneHy { place: Some(1), ..screen::NewPaneHy::new(0, false) }), "Switches shop-api to a new branch"),
            (Mode::KeyMap(Box::new(screen::KeyMap::new(screen::KeyView::Map, None))), "goes after the leader"),
        ] {
            app.mode = mode;
            let o = draw(&mut app, 160, 45);
            show(&o);
            assert!(o.contains(needle), "overlay shows {needle}");
        }
        let tab = |c: modal::Cat| modal::Cat::ALL.iter().position(|x| *x == c).unwrap();
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: tab(modal::Cat::Sessions), sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(" General ") && o.contains(" Sessions ") && o.contains("NEW SESSIONS") && o.contains("Tabs in the sidebar"), "the Sessions page opens on new sessions and the sidebar");
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: tab(modal::Cat::Appearance), sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Gap") && !o.contains("Corners") && o.contains("PREVIEW") && o.contains("focused") && o.contains("dimmed"), "the pane settings, with a preview");
        // The splash: the <APP> wordmark, its tagline, what happened, buttons.
        app.mode = Mode::Normal;
        app.splash = true;
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(env!("APP_TAGLINE")) && !o.contains("⣿"), "the tagline, no art");
        assert!(o.contains("while you were away") && o.contains("1 need you") && o.contains("Resume where you left off") && o.contains("New session") && o.contains("Open a project…"));
        // Small windows keep it all.
        let o = draw(&mut app, 100, 30);
        assert!(!o.contains("⣿") && o.contains("New session"));
    }

    #[test]
    fn a_release_waiting_is_a_banner_over_the_splash_s_cards_in_every_look() {
        for look in ["floating", "hairline"] {
            for (w, h) in [(160u16, 45u16), (80, 24)] {
                let (_, mut app) = super::design_tests::render_with(w, h);
                app.cfg.ui.layout = look.into();
                app.splash = true;
                let plain = draw(&mut app, w, h);
                assert!(!plain.contains("is out"), "{look} {w}x{h}: no banner without a release");
                app.update_available = Some("9.9.9".into());
                let o = draw(&mut app, w, h);
                show(&o);
                let lines: Vec<&str> = o.lines().collect();
                let at = lines.iter().position(|l| l.contains(concat!("↑ ", env!("APP_COMMAND"), " 9.9.9 is out")) && l.contains("u  update")).unwrap_or_else(|| panic!("{look} {w}x{h}: the banner
{o}"));
                let cards = lines.iter().position(|l| l.contains("New session")).expect("the cards");
                assert!(at < cards, "{look} {w}x{h}: over the cards");
                assert!(lines[at - 1].contains('─') && lines[at + 1].contains('─'), "{look} {w}x{h}: boxed");
                assert!(app.hits.iter().any(|(_, h)| *h == Hit::Screen(screen::ScreenHit::Update)), "a click on it updates");
            }
        }
    }

    /// The splash at the design's three sizes: the tagline, the cards with Resume chosen and its
    /// last place under it until the window is short, the hints, the version bottom right.
    #[test]
    fn the_splash_holds_at_the_designs_sizes() {
        for (w, h) in [(160u16, 45u16), (120, 34), (80, 24)] {
            let (_, mut app) = super::design_tests::render_with(w, h);
            app.splash = true;
            let o = draw(&mut app, w, h);
            show(&o);
            let lines: Vec<&str> = o.lines().collect();
            let row = |needle: &str| lines.iter().position(|l| l.contains(needle)).unwrap_or_else(|| panic!("{w}x{h}: no {needle:?}"));
            let tag = row(concat!(env!("APP_TAGLINE"), "."));
            let (resume, new, open, hints) = (row("Resume where you left off"), row("New session"), row("Open a project…"), row("choose"));
            assert!(tag < resume && resume < new && new < open && open < hints, "{w}x{h}: top to bottom");
            let short = h < 30;
            assert_eq!(new - resume, if short { 1 } else { 3 }, "{w}x{h}: Resume's second line and the gaps go when short");
            assert!(lines[resume + 1].contains(" › ") != short, "{w}x{h}: where you were, under Resume");
            let version = concat!(env!("APP_COMMAND"), " ", env!("CARGO_PKG_VERSION"));
            assert!(lines.last().unwrap().trim_end().ends_with(version), "{w}x{h}: the version, bottom right");
        }
    }

    #[test]
    fn reported_subagents_make_no_tree_rows_and_no_inspector() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let term = 1;
        let pane = app.snap.terms.get_mut(&term).unwrap();
        pane.subagents = vec!["Explore".into()];
        pane.subagent_activity = vec![SubagentActivity {
            id: "child-live".into(), name: "general-purpose".into(), parent: None,
            state: SubagentState::Working, task: Some("review the bing flow".into()),
            model: Some("claude-opus".into()), effort: None, started_at: Some(100), updated_at: 130,
            latest_activity: Some("review returned".into()), result: Some("one blocking finding".into()),
        }];
        app.screen_fresh();
        let sidebar = draw(&mut app, 160, 45);
        assert!(!sidebar.contains("subagent") && !sidebar.contains("review the bing flow") && !sidebar.contains("Researching"), "no tree under the row: {sidebar}");
        let model = app.screen_model();
        assert!(!screen::side_lines(&app, &model, &app.theme).iter().any(|l| matches!(l, screen::Line::Note(text, ..) if text.contains("subagent"))));
        assert!(app.session_items(term).unwrap().1.iter().all(|(l, _)| !l.to_lowercase().contains("subagent")), "nothing to fold in the row menu");
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
        assert!(!o.contains("● answer") && !o.contains(" Yes 1 "), "no buttons that would only type a digit");
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
        assert!(o.contains("INBOX") && o.contains("NEEDS YOU  1") && o.contains("FINISHED  1") && !o.contains("EVERYTHING"), "the Inbox sheet: what needs you, what finished");
        assert!(o.contains("Run npm test -- checkout?") && o.contains("Reply m") && o.contains("Open o"), "the question and what to do about it");
        assert!(o.contains("↑↓") && o.contains("select") && o.contains("Esc") && o.contains("close"), "the sheet's keys");
        // It's docked beside the panes, not over them: the session's card is still there.
        assert!(o.contains("fix the flaky checkout test"), "the panes reflow beside it");
        // 2 answers the selected question without going there.
        app.on_key(KeyEvent::new(KeyCode::Char('2'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::GoTo { .. }), "still in the Inbox");
        assert!(app.notice.as_ref().is_some_and(|(m, ..)| m.contains("Always")), "{:?}", app.notice);
        // Typing finds any session instead (numbers too, once you're typing).
        for ch in "rate".chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(ch), KeyModifiers::NONE));
        }
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("FOUND  1") && !o.contains("NEEDS YOU"), "searching: just what matches");
        // j closes it again, and opens it afresh.
        app.act(Action::Jump);
        assert!(app.mode == Mode::Normal, "j closes the Inbox");
        app.act(Action::Jump);
        assert!(matches!(&app.mode, Mode::GoTo { query, .. } if query.is_empty()));
    }



    #[test]
    fn a_context_menu_pads_its_bottom_like_its_top() {
        let layout = "floating";
        let (_, mut app) = super::design_tests::render_with(230, 50);
        app.splash = false;
        app.mode = Mode::Normal;
        app.cfg.ui.layout = layout.into();
        app.cfg.ui.fold_retired_layout();
        app.menu_for_pane(1, (60, 1));
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(230, 50)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let card = app.theme.card;
        let x = 62;
        let last_item = (0..50u16).rev().find(|&y| (62..90).any(|x| buf[(x, y)].symbol() == "C") && buf[(x, y)].bg == card).expect("the Close pane row");
        let below = |n: u16| buf[(x, last_item + n)].bg;
        assert_eq!(below(1), card, "{layout}: one padding row under the last item");
        assert!(matches!(buf[(x, last_item + 2)].symbol(), "─" | "╰" | "└"), "{layout}: then its box, as it began one row above the first item");
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
        assert!(o.contains("↑↓  choose") && o.contains("Enter  open"));
        app.on_key(key(KeyCode::Enter));
        assert!(!app.splash && matches!(app.mode, Mode::Normal), "Enter picks the selected one (New session: an agent, straight away)");
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
        for item in ["Rename pane", "Split side by side", "Split stacked", "Send right-clicks to pane", "Close pane"] {
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
        assert!(o.contains("rate") && o.contains(" 2m") && !o.contains("working"), "a worktree session is named after it, with its age; its spinner says it is working");

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
        assert_eq!(buf[(inner.x - 1, inner.y)].bg, app.theme.bg, "a small contrasting panel does not recolor the card padding");
        assert_eq!(buf[(inner.x, inner.y)].bg, app.theme.card2, "not a light grey slab");
        assert_eq!(buf[(inner.x + 13, inner.y)].bg, app.theme.bg, "the rest on the card's ground");
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
    fn wheel_scrolls_history() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The terminal is as big as the card's inside (the app sizes it so).
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
        assert!(o.contains("↓ Back to bottom · 30 lines up"), "and it says so, with a way back");
        app.on_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::SHIFT));
        app.on_key(KeyEvent::new(KeyCode::PageDown, KeyModifiers::SHIFT));
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("line 100") && !o.contains("lines up"), "Shift+PageDown back to the bottom");
    }

    /// Where the "Back to bottom" button was drawn for `term`, if it was.
    fn to_bottom_at(app: &App, term: TermId) -> Option<Rect> {
        app.hits.iter().find(|(_, h)| *h == Hit::Screen(screen::ScreenHit::ToBottom(term))).map(|(r, _)| *r)
    }

    #[test]
    fn back_to_bottom_is_a_click_away_in_apps_history() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (_, inside) = app.panes[0];
        let mut p = vt100::Parser::new(inside.height, inside.width, 1000);
        for i in 1..=100 {
            p.process(format!("line {i}\r\n").as_bytes());
        }
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        assert!(to_bottom_at(&app, 1).is_none(), "at the bottom: no button");
        app.scroll_to(1, 30);
        draw(&mut app, 160, 45);
        let at = to_bottom_at(&app, 1).expect("scrolled up: a button");
        let (_, inner) = app.panes[0];
        assert_eq!(at.y, inner.bottom() - 1, "at the bottom of the pane");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.x + 1, row: at.y, modifiers: KeyModifiers::NONE });
        assert_eq!(app.history(1).0, 0, "one click: back at the newest line");
    }

    #[test]
    fn back_to_bottom_pages_an_agent_that_scrolls_itself_back_down() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let mut p = vt100::Parser::new(40, 120, 1000);
        p.process(b"\x1b[?1049h");
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        let (_, inner) = app.panes[0];
        for _ in 0..3 {
            app.wheel_page = None;
            app.on_mouse(MouseEvent { kind: MouseEventKind::ScrollUp, column: inner.x + 5, row: inner.y + 5, modifiers: KeyModifiers::NONE });
        }
        while rx.try_recv().is_ok() {}
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("↓ Back to bottom"), "{o}");
        let at = to_bottom_at(&app, 1).expect("a button");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.x + 1, row: at.y, modifiers: KeyModifiers::NONE });
        let sent: Vec<u8> = std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| if let ClientMsg::Input { term: 1, data } = m { Some(data) } else { None })
            .flatten()
            .collect();
        assert_eq!(sent, b"\x1b[6~".repeat(3), "three pages up, three back down");
        assert!(!draw(&mut app, 160, 45).contains("Back to bottom"), "and the button goes");
    }

    #[test]
    fn typing_to_an_agent_clears_its_back_to_bottom() {
        use crossterm::event::{MouseEvent, MouseEventKind};
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut p = vt100::Parser::new(40, 120, 1000);
        p.process(b"\x1b[?1049h");
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        let (_, inner) = app.panes[0];
        app.on_mouse(MouseEvent { kind: MouseEventKind::ScrollUp, column: inner.x + 5, row: inner.y + 5, modifiers: KeyModifiers::NONE });
        assert!(draw(&mut app, 160, 45).contains("Back to bottom"));
        app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        assert!(!draw(&mut app, 160, 45).contains("Back to bottom"), "the agent scrolls to its prompt itself");
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
        assert_eq!(app.history(1).0, 15, concat!(env!("APP_COMMAND"), " scrolled its history"));
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
        let p = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-paste-{}.png"), std::process::id()));
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
        assert!(!o.lines().next().unwrap_or_default().contains("Fix the login flow"), "the pane's title carries its tabs, not what the agent is on");
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

    /// Three panes in one tab, `a` focused: a beside b, c under b.
    fn three_panes(app: &mut App) -> [TermId; 3] {
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
        app.screen.tabs[0].focus = a;
        [a, b, c]
    }

    #[test]
    fn pane_colours_draw_each_pane_in_its_own_colour_by_place() {
        for corners in ["square", "rounded"] {
            let (_, mut app) = super::design_tests::render_with(200, 50);
            app.cfg.ui.pane_colors = true;
            (app.cfg.ui.layout, app.cfg.ui.rounded) = ("floating".into(), if corners == "rounded" { crate::config::Rounding::Corners } else { crate::config::Rounding::Off });
            let [a, b, c] = three_panes(&mut app);
            app.screen_fresh();
            let mut term = Terminal::new(TestBackend::new(200, 50)).unwrap();
            term.draw(|f| render::draw(&mut app, f)).unwrap();
            let buf = term.backend().buffer().clone();
            show(&(0..50).map(|y| (0..200).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>()).collect::<Vec<_>>().join("\n"));
            let card = |id| app.pane_frames.iter().find(|(t, _)| *t == id).unwrap().1;
            let colours = app.theme.grid_colors();
            let edge = |id| &buf[(card(id).x, card(id).y + 1)];
            assert_eq!(edge(a).fg, colours[0], "{corners}: the top-left pane is the first colour, full while focused");
            assert!(edge(a).modifier.contains(ratatui::style::Modifier::BOLD), "{corners}: focused is bold");
            assert_eq!(edge(b).fg, crate::theme::mix(colours[1], app.theme.bg, 0.42), "{corners}: the next, faded");
            assert!(!edge(b).modifier.contains(ratatui::style::Modifier::BOLD));
            assert_eq!(edge(c).fg, crate::theme::mix(colours[2], app.theme.bg, 0.42), "{corners}: the third");
            let corner = buf[(card(a).x, card(a).y)].symbol().to_string();
            assert_eq!(corner, if corners == "rounded" { "╭" } else { "┌" });
            assert!(card(b).x > card(a).right(), "{corners}: a gap, as in Floating");
        }
    }

    /// Where `term`'s box is drawn.
    fn card_of(app: &App, term: TermId) -> Rect {
        app.pane_frames.iter().find(|(t, _)| *t == term).unwrap().1
    }

    #[test]
    fn a_narrow_title_keeps_its_status_glyph_last_in_every_look() {
        let mut failures = Vec::new();
        let layout = "floating";
        for split in [false, true] {
            for (mode, update) in [("off", false), ("lit", false), ("held", false), ("off", true), ("lit", true), ("held", true)] {
                for w in 56..=110u16 {
                    let (_, mut app) = super::design_tests::render_with(w, 24);
                    (app.cfg.ui.layout, app.cfg.ui.rounded, app.cfg.ui.which_key) = (layout.into(), crate::config::Rounding::Full, false);
                    app.cfg.ui.fold_retired_layout();
                    app.update_available = update.then(|| "0.44.0".into());
                    if split {
                        let [_, b, _] = three_panes(&mut app);
                        app.snap.terms.get_mut(&b).unwrap().status = Status::Blocked;
                    }
                    if mode != "off" {
                        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
                    }
                    if mode == "held" {
                        tap(&mut app, ',');
                    }
                    let buf = drawn(&mut app, w, 24);
                    // The panes along the top: the modal covers any below.
                    let top = app.pane_frames.iter().map(|(_, r)| r.y).min().unwrap_or(0);
                    let blocked: Vec<Rect> = app.pane_frames.iter().filter(|(id, r)| r.y == top && app.snap.terms[id].status == Status::Blocked).map(|(_, r)| *r).collect();
                    assert!(!blocked.is_empty(), "{layout} {w}: a blocked pane is drawn");
                    for r in blocked {
                        let row: String = (r.x..r.right()).map(|x| buf[(x, r.y)].symbol()).collect();
                        // A modal (Settings, held open) can lie over the title: nothing to read there.
                        // A lone pane has no title: its tab shows the state.
                        if !split || !row.starts_with(['┌', '╭']) {
                            continue;
                        }
                        if !row.contains('▲') && !row.contains('●') {
                            failures.push(format!("{layout} split {split} {mode} update {update} {w}: {row}"));
                        }
                    }
                    // Typing says so on the mode bar at every width.
                    let (_, bar) = bar_text(&app, &buf);
                    if mode == "off" && !bar.contains(" TYPING ") {
                        failures.push(format!("{layout} split {split} {mode} update {update} {w}: no TYPING on the bar: {bar}"));
                    }
                }
            }
        }
        assert!(failures.is_empty(), "the status glyph is on the title:\n{}", failures.join("\n"));
    }

    fn drawn(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        term.backend().buffer().clone()
    }

    /// Where `word` last starts on row `y` between `from` and `to`.
    fn last_at(buf: &ratatui::buffer::Buffer, word: &str, y: u16, from: u16, to: u16) -> Option<u16> {
        let n = word.chars().count() as u16;
        (from..to.saturating_sub(n)).rev().find(|&x| (0..n).map(|i| buf[(x + i, y)].symbol()).collect::<String>() == word)
    }

    #[test]
    fn the_new_tab_chip_is_raised_and_a_cell_clear_of_the_tabs_in_every_look() {
        use crate::config::Rounding;
        for layout in ["floating", "tiled", "grid"] {
            for rounded in [Rounding::Off, Rounding::Full] {
                let (_, mut app) = super::design_tests::render_with(160, 45);
                (app.cfg.ui.layout, app.cfg.ui.rounded) = (layout.into(), rounded);
                app.cfg.ui.fold_retired_layout();
                let buf = drawn(&mut app, 160, 45);
                let at = format!("{layout} {rounded:?}");
                let t = app.theme.clone();
                let r = app.hits.iter().find(|(_, h)| *h == Hit::Screen(screen::ScreenHit::NewTab)).expect("a + chip").0;
                let plus = (r.x..r.right()).find(|&x| buf[(x, r.y)].symbol() == "+").expect("its +");
                assert_eq!((buf[(plus, r.y)].fg, buf[(plus, r.y)].bg), (t.strong, t.btn), "strong on btn: {at}");
                let (l, e) = (&buf[(r.x, r.y)], &buf[(r.right() - 1, r.y)]);
                if rounded == Rounding::Full {
                    assert_eq!((l.symbol(), l.fg, e.symbol(), e.fg), (screen::CAP_L, t.btn, screen::CAP_R, t.btn), "capped: {at}");
                } else {
                    assert_eq!((l.symbol(), l.bg, e.symbol(), e.bg), (" ", t.btn, " ", t.btn), "square: {at}");
                }
                let gap = &buf[(r.x - 1, r.y)];
                assert!(gap.symbol() == " " && gap.bg != t.btn, "a clear cell before it: {at}");
            }
        }
    }

    /// The three panes, the current tab at `place` among three tabs of the session; the other
    /// two show pane `c` and are named `other-tab-1` and `other-tab-2`.
    fn three_tabs(app: &mut App, place: usize) -> [TermId; 3] {
        let [a, b, c] = three_panes(app);
        let mut tabs: Vec<_> = (1..=2)
            .map(|k| {
                let mut tab = app.screen.tabs[0].clone();
                (tab.layout, tab.focus, tab.name) = (crate::layout::Node::Leaf(c), c, format!("other-tab-{k}"));
                tab
            })
            .collect();
        tabs.insert(place, app.screen.tabs[0].clone());
        (app.screen.tabs, app.screen.tab) = (tabs, place);
        [a, b, c]
    }

    /// One session in three tabs, each tab one pane: an idle agent, one working and one that
    /// needs you, the first tab on screen.
    fn tabbed_session(layout: &str) -> (App, [TermId; 3]) {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.layout = layout.into();
        let [a, b, c] = three_panes(&mut app);
        let owner = app.screen.tabs[0].owner;
        let tab = |id| screen::ScreenTab { layout: crate::layout::Node::Leaf(id), focus: id, owner, used: 0, name: String::new() };
        (app.screen.tabs, app.screen.tab) = (vec![tab(a), tab(b), tab(c)], 0);
        for (id, agent, status) in [(a, "claude", Status::Idle), (b, "codex", Status::Working), (c, "gemini", Status::Blocked)] {
            let info = app.snap.terms.get_mut(&id).unwrap();
            (info.agent, info.status) = (Some(agent.into()), status);
        }
        (app, [a, b, c])
    }

    /// The sidebar's rows as text, top to bottom.
    fn side_text(app: &App, buf: &ratatui::buffer::Buffer) -> Vec<String> {
        let r = app.screen.side_rect;
        (r.y..r.bottom()).map(|y| (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect::<String>()).collect()
    }

    #[test]
    fn a_session_s_tabs_each_show_their_own_state_under_its_row() {
        for layout in ["floating", "hairline"] {
            let (mut app, _) = tabbed_session(layout);
            let buf = drawn(&mut app, 160, 45);
            let rows = side_text(&app, &buf);
            show(&rows.join("
"));
            let line = |n: &str| rows.iter().position(|r| r.contains(&format!("{n} "))).unwrap_or_else(|| panic!("{layout}: tab line {n}
{}", rows.join("
")));
            let (two, three) = (line("├ 2"), line("└ 3"));
            assert_eq!(three, two + 1, "{layout}: one line per tab, in order");
            assert!(!rows.iter().any(|r| r.contains("├ 1 ") || r.contains("└ 1 ")), "{layout}: no line for the tab the session's own pane is in: its row is that one");
            let spinner = crate::client::design::glyph(&app, Status::Working);
            assert!(rows[two].contains(&spinner), "{layout}: the working tab says so though it is not on screen: {}", rows[two]);
            assert!(rows[three].contains("needs you"), "{layout}: so does the one that needs you: {}", rows[three]);
        }
    }

    #[test]
    fn a_tab_out_of_view_shows_its_state_on_its_tab() {
        for layout in ["floating", "hairline"] {
            let (mut app, _) = tabbed_session(layout);
            let buf = drawn(&mut app, 160, 45);
            let row = screen::grid(&app, buf.area).tabs;
            let tabs: String = (row.x..row.right()).map(|x| buf[(x, row.y)].symbol()).collect();
            show(&tabs);
            // Each tab's glyph: the first mark left of its number.
            let glyph_before = |n: &str| {
                let at = tabs.find(&format!("{n} ")).unwrap_or_else(|| panic!("{layout}: tab {n}: {tabs}"));
                let x = row.x + tabs[..at].chars().count() as u16;
                let gx = (row.x..x).rev().find(|&x| buf[(x, row.y)].symbol() != " ").expect("a glyph");
                (buf[(gx, row.y)].symbol().to_string(), buf[(gx, row.y)].fg)
            };
            let (working, _) = glyph_before("2");
            assert!(working != "○" && working != "✕", "{layout}: the working tab, not on screen, says so: {tabs}");
            let (blocked, colour) = glyph_before("3");
            assert!(["▲", "●"].contains(&blocked.as_str()) && colour == app.theme.blocked, "{layout}: so does the one that needs you, in its colour: {tabs}");
        }
    }

    #[test]
    fn every_close_asks_first_an_idle_shell_too() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // Closing from the sidebar acts on the row the cursor is on.
        let shell = *app.snap.terms.iter().find(|(_, t)| t.agent.is_none()).map(|(id, _)| id).expect("a shell");
        app.snap.terms.get_mut(&shell).unwrap().process = "pwsh.exe".into();
        app.screen.cursor = Some(shell);
        app.mode = Mode::Normal;
        app.act(Action::ClosePane);
        assert!(matches!(app.mode, Mode::Confirm(_)), "an idle shell asks first too");
        let agent = *app.snap.terms.iter().find(|(_, t)| t.agent.is_some()).map(|(id, _)| id).expect("an agent");
        app.screen.cursor = Some(agent);
        app.mode = Mode::Normal;
        app.act(Action::ClosePane);
        assert!(matches!(app.mode, Mode::Confirm(_)), "an agent asks first");
        let (_, mut busy) = super::design_tests::render_with(160, 45);
        busy.snap.terms.get_mut(&shell).unwrap().process = "npm".into();
        busy.screen.cursor = Some(shell);
        busy.mode = Mode::Normal;
        busy.act(Action::ClosePane);
        assert!(matches!(busy.mode, Mode::Confirm(_)), "a shell running something asks first");
    }

    #[test]
    fn alt_letters_run_the_everyday_actions_without_the_leader_unless_turned_off() {
        let alt = |c: char| crate::keys::KeySpec::from_event(&KeyEvent::new(KeyCode::Char(c), KeyModifiers::ALT));
        let on = Config::default().keymap();
        for (c, want) in [('n', Action::ShellHere), ('a', Action::StartSession), ('x', Action::ClosePane), ('s', Action::SplitRight), ('d', Action::SplitDown)] {
            assert_eq!(on.global.get(&alt(c)), Some(&want), "Alt+{c}");
        }
        for c in ['v', 't'] {
            assert!(!on.global.contains_key(&alt(c)), "Alt+{c} is Claude Code's");
        }
        let mut cfg = Config::default();
        cfg.ui.alt_keys = false;
        assert!(!cfg.keymap().global.contains_key(&alt('x')), "off: Alt+x goes to the program");
    }

    #[test]
    fn alt_x_in_the_sidebar_closes_the_selected_session_and_asks_for_an_agent() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let focused = app.focused();
        let agent = *app.snap.terms.iter().find(|(id, t)| t.agent.is_some() && Some(**id) != focused).map(|(id, _)| id).expect("an agent not in focus");
        app.screen_side_set(screen::SideItem::Sess(agent));
        app.mode = Mode::Side;
        app.on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::ALT));
        assert!(matches!(&app.mode, Mode::Confirm(c) if matches!(&c.act, crate::client::menu::Act::End(ts) if *ts == vec![agent])), "asks to close the row's session: {:?}", app.mode);
        assert_eq!(app.focused(), focused, "it doesn't go into the pane");
    }

    #[test]
    fn settings_keys_lists_the_keys_that_need_no_leader() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let keys = modal::Cat::ALL.iter().position(|c| *c == modal::Cat::Keys).unwrap();
        let rows = design::settings_rows(modal::Cat::Keys);
        let last = rows.len() - 1;
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: keys, sel: last, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(design::DIRECT_GROUP), "the heading
{o}");
        for (label, key) in [("Close pane", "Alt + x"), ("Split stacked", "Alt + d"), ("Go to tab 1–9", "Alt + 1–9")] {
            assert!(o.lines().any(|l| l.contains(label) && l.contains(key)), "{label} {key}
{o}");
        }
    }

    #[test]
    fn the_sidebar_shows_all_busy_or_no_tab_lines_as_set() {
        for layout in ["floating", "hairline"] {
            for (setting, want) in [("all", vec!["├ 2", "└ 3"]), ("busy", vec!["├ 2", "└ 3"]), ("off", vec![])] {
                let (mut app, _) = tabbed_session(layout);
                // The working tab on screen; the idle one is busy only while it is.
                app.screen.tab = 1;
                app.cfg.ui.sidebar_tabs = setting.into();
                let buf = drawn(&mut app, 160, 45);
                let rows = side_text(&app, &buf);
                let got: Vec<&str> = ["├ 1", "└ 1", "├ 2", "└ 2", "├ 3", "└ 3"].into_iter().filter(|n| rows.iter().any(|r| r.contains(&format!("{n} ")))).collect();
                assert_eq!(got, want, "{layout} {setting}
{}", rows.join("
"));
            }
        }
    }

    #[test]
    fn clicking_a_tab_line_shows_that_tab() {
        for layout in ["floating", "hairline"] {
            let (mut app, [_, _, c]) = tabbed_session(layout);
            let buf = drawn(&mut app, 160, 45);
            let rows = side_text(&app, &buf);
            let y = app.screen.side_rect.y + rows.iter().position(|r| r.contains("└ 3")).expect("tab 3's line") as u16;
            let x = app.screen.side_rect.x + 6;
            app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: x, row: y, modifiers: KeyModifiers::NONE });
            assert_eq!(app.screen.tab, 2, "{layout}: the third tab is on screen");
            assert_eq!(app.screen.tabs[2].focus, c, "{layout}");
        }
    }

    #[test]
    fn a_session_with_one_tab_has_no_tab_lines() {
        for layout in ["floating", "hairline"] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            app.cfg.ui.layout = layout.into();
            let buf = drawn(&mut app, 160, 45);
            let rows = side_text(&app, &buf);
            assert!(!rows.iter().any(|r| r.contains("├ 1") || r.contains("└ 1")), "{layout}
{}", rows.join("
"));
        }
    }

    #[test]
    fn a_narrow_title_keeps_the_current_tab_before_any_other() {
        use crate::config::Rounding;
        let mut failures = Vec::new();
        let mut others_dropped = false;
        let layout = "floating";
        for rounded in [Rounding::Off, Rounding::Full] {
            for place in 0..3 {
                for w in 44..=110u16 {
                    let (_, mut app) = super::design_tests::render_with(w, 24);
                    (app.cfg.ui.layout, app.cfg.ui.rounded) = (layout.into(), rounded);
                    app.cfg.ui.fold_retired_layout();
                    let [a, ..] = three_tabs(&mut app, place);
                    app.snap.terms.get_mut(&a).unwrap().status = Status::Working;
                    let buf = drawn(&mut app, w, 24);
                    let card = card_of(&app, a);
                    let title: String = (card.x..card.right()).map(|x| buf[(x, card.y)].symbol()).collect();
                    // The tabs have their own row over the panes (ADR 0028).
                    let row = screen::grid(&app, buf.area).tabs;
                    let top: String = (row.x..row.right()).map(|x| buf[(x, row.y)].symbol()).collect();
                    let at = format!("{layout} {rounded:?} current {place} {w}: {top} / {title}");
                    show(&at);
                    let on_row = |want: Hit| app.hits.iter().filter(|(r, h)| *h == want && r.y == row.y && row.intersects(*r)).count();
                    let shown = |i| on_row(Hit::Screen(screen::ScreenHit::TabPick(i))) == 1;
                    let others = (0..3).filter(|&i| i != place).filter(|&i| shown(i)).count();
                    if others > 0 && !(shown(place) && top.contains("claude")) {
                        failures.push(format!("another tab kept over the current one, or while it shortens: {at}"));
                    }
                    let plus = on_row(Hit::Screen(screen::ScreenHit::NewTab)) > 0;
                    if plus && !(0..3).all(shown) {
                        failures.push(format!("+ without every tab: {at}"));
                    }
                    let spinner = crate::client::design::glyph(&app, Status::Working);
                    if !title.contains(&spinner) {
                        failures.push(format!("no state on the title: {at}"));
                    }
                    others_dropped |= shown(place) && others < 2;
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("
"));
        assert!(others_dropped, "somewhere the other tabs give way and the current one stays");
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
        let o = draw(&mut app, 260, 50);
        show(&o);
        assert!(o.contains(" 1 ") && o.contains(" 2 "), "a tab bar");
        let side = app.screen.side_rect;
        assert_eq!(app.hits.iter().filter(|(r, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_))) && !side.intersects(*r)).count(), 2, "both tabs in the top-left pane's title");
        assert_eq!(app.hits.iter().filter(|(_, h)| *h == Hit::Screen(screen::ScreenHit::NewTab)).count(), 1, "one + chip after the tabs");
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
        assert_eq!(app.hits.iter().filter(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_)))).count(), 1, "the tab in the pane's title, even with one tab");
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
        let side = app.screen.side_rect;
        let title = o.lines().next().unwrap_or_default().chars().skip(side.right() as usize).collect::<String>();
        let in_title = app.hits.iter().filter(|(r, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_))) && !side.intersects(*r)).count();
        assert!(in_title == 2 && !title.contains("2 shell"), "claude's two tabs, unnamed ones shown by number: {title}");
        assert!(o.contains("└ 2 shell"), "the sidebar names the shell's tab under claude's row");
        // Rename uses the same centered text editor as pane and project names.
        app.act(Action::RenameTab);
        for c in "tests".chars() {
            key(&mut app, KeyCode::Char(c));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Rename session") && o.contains("tests█"), "the dialog shows the draft: {o}");
        key(&mut app, KeyCode::Enter);
        // The daemon owns the name: play its part by applying the command to the snapshot.
        let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        let focus = app.screen.tabs[app.screen.tab].focus;
        let label = sent
            .iter()
            .find_map(|m| match m {
                crate::protocol::ClientMsg::Command(crate::protocol::Command::RenamePane { term, name }) if *term == focus => Some(name.clone()),
                _ => None,
            })
            .expect("the session is renamed, so the sidebar and every notice use the name");
        assert_eq!(label, "tests");
        app.snap.terms.get_mut(&focus).unwrap().label = label;
        assert_eq!(app.session_name(focus), "tests", "notices say the name you gave");
        app.screen_sync();
        let o = draw(&mut app, 160, 45);
        let title = o.lines().next().unwrap_or_default().chars().skip(app.screen.side_rect.right() as usize).collect::<String>();
        assert!(title.contains(" tests ") && !title.contains("2 tests"), "renamed: the pill is its name: {title}");
        // Views get rebuilt (another session reuses the slot, the client restarts): the name stays.
        app.screen.tabs.iter_mut().for_each(|t| t.name.clear());
        app.screen_sync();
        let o = draw(&mut app, 160, 45);
        assert!(o.contains(" tests "), "the name comes back from the daemon: {o}");
        // Another session: its own view, with its own rail.
        app.screen_place(2, Some(3));
        draw(&mut app, 160, 45);
        assert_eq!(app.session_tabs().len(), 1);
        let side = app.screen.side_rect;
        assert!(app.hits.iter().filter(|(r, h)| matches!(h, Hit::Screen(screen::ScreenHit::TabPick(_))) && !side.intersects(*r)).count() == 1, "codex has one tab");
        // Back to claude: the tab you were last on there (the shell).
        app.screen_place(1, Some(2));
        assert_eq!(app.screen.tabs[app.screen.tab].focus, 3, "back on the shell tab");
        // Closing a tab asks first.
        let mine = app.screen.tabs.iter().position(|t| t.layout.contains(1)).unwrap();
        app.close_tab(mine);
        assert!(matches!(&app.mode, Mode::Confirm(c) if matches!(&c.act, crate::client::menu::Act::End(ts) if ts.contains(&1))), "{:?}", app.mode);
    }

    #[test]
    fn the_wheel_scrolls_an_agents_full_screen_view() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        let (term, r) = app.pane_frames[0];
        let at = (r.x + 10, r.y + 10);
        let mouse = |app: &mut App, kind: MouseEventKind| app.on_mouse(MouseEvent { kind, column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
        // A drag whose release never came is over once the mouse moves with no button held.
        app.screen.drag = Some(screen::Drag::Session(2, false, screen::ScreenHit::Session(2)));
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
        assert!(info.contains("normal screen") && info.contains(concat!("the wheel goes to ", env!("APP_COMMAND"))) && info.contains("61 lines of history"), "{info}");
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
        // No bare letter acts from the sidebar (ADR 0014): x is typing, it doesn't ask to close.
        app.on_key(key(KeyCode::Char('x')));
        assert!(!matches!(&app.mode, Mode::Confirm(_)), "x closes nothing");
        app.screen_side_set(screen::SideItem::Sess(first));
        // The leader's x asks to close it.
        app.mode = Mode::Prefix { since: std::time::Instant::now(), acted: false };
        app.act(Action::ClosePane);
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.title == "Close pane"), "leader x closes (after asking)");
        app.on_key(key(KeyCode::Esc));
        assert!(matches!(app.mode, Mode::Side) && app.screen.cursor == Some(first), "saying no goes back to the sidebar");
        let next = app.side_after_close();
        app.mode = Mode::Prefix { since: std::time::Instant::now(), acted: false };
        app.act(Action::ClosePane);
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Side), "after closing, the keys stay in the sidebar");
        assert_eq!(next.map(|n| n != screen::SideItem::Sess(first)), Some(true));
        // The leader key's close, from the sidebar: the cursor's row, and the keys stay too.
        let on = app.screen.cursor.or_else(|| app.screen.side_items.iter().find_map(|i| match i { screen::SideItem::Sess(t) => Some(*t), _ => None })).unwrap();
        app.screen_side_set(screen::SideItem::Sess(on));
        // Something running in it, so there is something to lose and it asks.
        app.snap.terms.get_mut(&on).unwrap().process = "npm".into();
        app.mode = Mode::Prefix { since: std::time::Instant::now(), acted: false };
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
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        // Typing in the sidebar goes to the pane instead.
        app.act(Action::BrowseTree);
        assert!(matches!(app.mode, Mode::Side));
        app.on_key(key(KeyCode::Char('q')));
        assert!(matches!(app.mode, Mode::Normal), "a letter leaves the sidebar (and is typed into the pane)");
    }

    #[test]
    fn changes_outside_git_offers_git_init() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let dir = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-nogit-{}"), std::process::id()));
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
        for g in ["UPDATES", "STARTUP", "INPUT"] {
            assert!(o.contains(g), "General is grouped: {g}");
        }
        assert!(design::settings_rows(modal::Cat::General).iter().any(|r| design::settings_group(r) == "SHELL"), "and SHELL, further down");
        let (updates, startup, input) = (o.find("UPDATES").unwrap(), o.find("STARTUP").unwrap(), o.find("INPUT").unwrap());
        assert!(updates < startup && startup < input, "UPDATES, then STARTUP, then INPUT");
        assert!(o.contains("› Version"), "the first row, the version, is selected");
        // Appearance, the next tab, has one theme control that opens the searchable browser.
        app.on_key(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("PANES") && o.contains("Gap") && !o.contains("Corners") && o.contains("PREVIEW"), "the pane settings first, with their preview");
        let theme_row = design::settings_rows(modal::Cat::Appearance).iter().position(|r| matches!(r, design::SRow::Setting(s) if s.path == "theme")).unwrap();
        for _ in 0..theme_row {
            app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("THEME") && o.contains("Enter browse"), "then the theme browser control");
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Themes") && o.contains("Day"));
        app.on_paste("Monokai".into());
        assert!(draw(&mut app, 160, 45).contains("Monokai"));
        app.on_key(KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        app.on_paste("Tokyo Night".into());
        assert!(draw(&mut app, 160, 45).contains("Tokyo Night"));
    }

    #[test]
    fn a_program_app_doesnt_know_goes_by_its_name_and_can_be_taught() {
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
        let top = app.pane_frames[0].1.y;
        assert_eq!((buf[(159, top)].symbol(), buf[(159, top)].fg), ("┐", app.theme.sky()), "the focused border turns sky");
        let (row, text) = bar_text(&app, &buf);
        assert!(text.starts_with("  SHORTCUT  ") && buf[(row.right() - 1, row.y)].bg == app.theme.sky(), "the mode bar lit, its tag a column in: {text}");
        // The delayed overview groups common commands; ? opens the full reference instead.
        app.open_keymap(None);
        assert!(matches!(app.mode, Mode::KeyMap(_)));
        let o = draw(&mut app, 160, 45);
        show(&o);
        for g in ["SESSIONS", "PANES", "TABS", "PROJECT", env!("APP_ENV")] {
            assert!(o.contains(g), "the key map has {g}");
        }
        assert!(o.contains("Inbox") && o.contains("Worktrees ›"), "steps marked");
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
        key(&mut app, KeyCode::Char('g'));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.step == Some(screen::Step::Worktrees)), "g opens the worktrees step");
        assert!(o.contains("WORKTREES IN SHOP-API") && o.contains("New worktree") && o.contains("Merge into main") && o.contains("the path so far"));
        key(&mut app, KeyCode::Backspace);
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.step.is_none()), "back to the key map");
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn actions_list_every_command_with_its_key() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Menu"));
        let at = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::Actions)).then_some((r.x, r.y))).expect("a actions is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
        assert!(matches!(app.mode, Mode::Actions { sel: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        for header in ["NEW", "GO TO", "VIEW", "SET UP"] {
            assert!(!o.contains(header), "the Menu has no {header} header");
        }
        assert!(o.contains("─ Menu ") && o.contains("Session with options…") && o.contains("Pane beside this") && o.contains("Inbox ● 1") && o.contains("Search commands"), "{o}");
        assert!(!o.contains("then a key works anywhere"), "no leader hint");
        let shell = o.lines().find(|l| l.contains("Shell here")).expect("Shell here");
        assert!(!shell.trim_end_matches(['│', ' ']).ends_with(" n"), "no bare letter beside a row: {shell}");
        assert!(!o.contains("Start a fleet") && !o.contains("All commands"), "the fleet row and the old palette row are gone");
        // A row's leader key runs it here, unshown: z collapses the sidebar.
        assert!(app.sidebar);
        app.on_key(KeyEvent::new(KeyCode::Char('z'), KeyModifiers::NONE));
        assert!(!app.sidebar && app.mode == Mode::Normal, "z: the sidebar collapses");
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
            assert!(screen.contains("Expand sidebar"));
            let index = screen::action_items().iter().position(|(_, a)| *a == Action::Palette).expect("all commands are discoverable");
            assert!(app.hits.iter().any(|(_, target)| *target == Hit::Screen(screen::ScreenHit::ActionRow(index))));
            click(&mut app, screen::ScreenHit::ActionRow(index));
            assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::Search), "Search commands opens the search");
            // Every command is a row the mouse can reach: scroll the list so the last one is selected.
            let all = screen::keymap_actions(&app, &screen::KeyMap { query: "e".into(), ..screen::KeyMap::new(screen::KeyView::Search, None) }).len();
            for sel in [0, all / 2, all.saturating_sub(1)] {
                app.mode = Mode::KeyMap(Box::new(screen::KeyMap { query: "e".into(), sel, ..screen::KeyMap::new(screen::KeyView::Search, None) }));
                draw(&mut app, w, h);
                assert!(app.hits.iter().any(|(_, target)| *target == Hit::Screen(screen::ScreenHit::KeyRow(sel))), "{w}x{h}: row {sel} of {all} can be scrolled into view");
            }
            app.mode = Mode::KeyMap(Box::new(screen::KeyMap::new(screen::KeyView::Map, None)));
            draw(&mut app, w, h);
            let help = screen::keymap_actions(&app, &screen::KeyMap::new(screen::KeyView::Map, None)).iter().position(|a| *a == Some(Action::Help)).expect("All keys is on the map");
            click(&mut app, screen::ScreenHit::KeyRow(help));
            assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::All));
            app.act(Action::AgentStatusSetup);
            let screen = draw(&mut app, w, h); show(&screen);
            assert!(screen.contains("Connect Codex") && screen.contains("Connect Pi") && screen.contains("Connect OpenCode"));
            for index in 0..2 {
                assert!(app.hits.iter().any(|(_, hit)| *hit == Hit::Screen(screen::ScreenHit::MenuPick(index))));
            }
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert_eq!(app.mode, Mode::Normal);
        }
    }

    fn sent(rx: &mut mpsc::UnboundedReceiver<ClientMsg>) -> Vec<Command> {
        std::iter::from_fn(|| rx.try_recv().ok()).filter_map(|m| match m { ClientMsg::Command(c) => Some(c), _ => None }).collect()
    }

    fn plain(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::NONE)
    }

    fn leader(app: &mut App, c: char) {
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    fn click_new_chip(app: &mut App) {
        let at = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::NewShell)).then_some((r.x + 1, r.y))).expect("+ new is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
    }

    #[test]
    fn the_new_chip_and_leader_n_open_a_plain_shell_where_you_are() {
        for cursor in [None, Some(2)] {
            for by_mouse in [false, true] {
                let (_, mut app) = super::design_tests::render_with(100, 30);
                let (tx, mut rx) = mpsc::unbounded_channel();
                app.out = tx;
                app.screen.cursor = cursor;
                app.mode = if cursor.is_some() { Mode::Side } else { Mode::Normal };
                let screen = draw(&mut app, 100, 30);
                show(&screen);
                if by_mouse {
                    assert!(screen.contains("+ New"), "a visible new chip");
                    click_new_chip(&mut app);
                } else {
                    leader(&mut app, 'n');
                }
                assert!(matches!(app.mode, Mode::Normal | Mode::Prefix { .. }), "new opens no form: {:?}", app.mode);
                let commands = sent(&mut rx);
                assert!(matches!(commands.as_slice(), [Command::NewWorkspace { cwd: Some(_), name: None, cmd: None }]), "one plain shell, no agent and no worktree: {commands:?}");
            }
        }
    }

    #[test]
    fn instant_sessions_follow_the_sessions_settings() {
        let start = |tweak: &dyn Fn(&mut App)| {
            let (_, mut app) = super::design_tests::render_with(100, 30);
            let (tx, mut rx) = mpsc::unbounded_channel();
            app.out = tx;
            tweak(&mut app);
            leader(&mut app, 's');
            (sent(&mut rx), app)
        };
        let (cmds, _) = start(&|a| a.cfg.sessions.place = "this folder".into());
        assert!(matches!(cmds.as_slice(), [Command::NewWorkspace { cwd: Some(cwd), cmd: Some(cmd), .. }] if cmd == "claude" && cwd.ends_with("shop-api")), "{cmds:?}");
        for agent in ["codex", "gemini"] {
            let (cmds, app) = start(&|a| a.cfg.sessions.agent = agent.into());
            assert!(matches!(cmds.as_slice(), [Command::NewWorktree { cmd: Some(cmd), .. }] if cmd == agent), "{cmds:?}");
            assert_eq!(app.screen.saved.last_agent, agent, "the agent is remembered");
        }
        let (cmds, _) = start(&|a| a.screen.saved.last_agent = "gemini".into());
        assert!(matches!(cmds.as_slice(), [Command::NewWorktree { cmd: Some(cmd), .. }] if cmd == "gemini"), "last used: {cmds:?}");
        let (cmds, _) = start(&|a| a.screen.saved.last_agent = "gone".into());
        assert!(matches!(cmds.as_slice(), [Command::NewWorktree { cmd: Some(cmd), .. }] if cmd == "claude"), "an unknown last agent falls back to claude: {cmds:?}");
        let (cmds, _) = start(&|a| {
            a.cfg.sessions.agent = "codex".into();
            a.screen.saved.last_agent = "gemini".into();
        });
        assert!(matches!(cmds.as_slice(), [Command::NewWorktree { cmd: Some(cmd), .. }] if cmd == "codex"), "a named agent wins over last used: {cmds:?}");
    }

    #[test]
    fn o_opens_the_new_session_dialog_with_its_rows_keys_and_buttons() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        leader(&mut app, 'o');
        assert!(matches!(&app.mode, Mode::NewSession(ns) if ns.row == 0), "opens on the Agent row");
        let o = draw(&mut app, 100, 30);
        show(&o);
        for want in ["Agent", "Where", "Project", "claude", "codex", "gemini", "new worktree", "this folder", "Start", "Cancel", "shop-api", "1 of 1", "Opens claude in a new worktree of shop-api. You type the task there."] {
            assert!(o.contains(want), "{want:?} in the dialog");
        }
        assert!(!o.contains("TASK") && !o.contains("GOAL"), "no goal field");
        // Down to Where, Right to "this folder"; the sentence follows.
        app.on_key(plain(KeyCode::Down));
        app.on_key(plain(KeyCode::Right));
        assert!(draw(&mut app, 100, 30).contains("Opens claude in the folder of shop-api. You type the task there."));
        // Up to Agent, Right to codex.
        app.on_key(plain(KeyCode::Up));
        app.on_key(plain(KeyCode::Right));
        assert!(draw(&mut app, 100, 30).contains("Opens codex in the folder of shop-api"));
        // Esc cancels, nothing starts.
        app.on_key(plain(KeyCode::Esc));
        assert!(matches!(app.mode, Mode::Prefix { .. }), "back to shortcut mode, where the dialog was opened from");
        app.on_key(plain(KeyCode::Esc));
        assert!(sent(&mut rx).is_empty());
        // Enter starts what it shows.
        leader(&mut app, 'o');
        app.on_key(plain(KeyCode::Right));
        app.on_key(plain(KeyCode::Down));
        app.on_key(plain(KeyCode::Right));
        app.on_key(plain(KeyCode::Enter));
        assert!(app.mode == Mode::Normal && !app.shortcut_held, "started: back to typing, into the new session");
        let cmds = sent(&mut rx);
        assert!(matches!(cmds.as_slice(), [Command::NewWorkspace { cwd: Some(cwd), cmd: Some(cmd), .. }] if cmd == "codex" && cwd.ends_with("shop-api")), "{cmds:?}");
        // The Start button does the same.
        leader(&mut app, 'o');
        let _ = draw(&mut app, 100, 30);
        let at = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::NsStart)).then_some((r.x + 1, r.y))).expect("Start is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
        assert!(matches!(sent(&mut rx).as_slice(), [Command::NewWorktree { cmd: Some(_), .. }]));
        // Its options take clicks too.
        leader(&mut app, 'o');
        let _ = draw(&mut app, 100, 30);
        let at = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::NsOpt(1, 1))).then_some((r.x + 1, r.y))).expect("an option is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: at.0, row: at.1, modifiers: KeyModifiers::NONE });
        assert!(matches!(&app.mode, Mode::NewSession(ns) if ns.place == 1 && ns.row == 1));
    }

    #[test]
    fn the_menu_starts_a_shell_or_a_session_and_n_is_the_shell_key() {
        let rows = screen::action_items();
        let labels: Vec<&str> = rows.iter().take(5).map(|(l, _)| *l).collect();
        assert_eq!(labels, ["Shell here", "Session", "Session with options…", "Pane beside this", "Tab"], "Shell first");
        assert!(rows.iter().any(|(l, a)| *l == "Session" && *a == Action::StartSession));
        assert!(rows.iter().any(|(l, a)| *l == "Shell here" && *a == Action::ShellHere));
        let key = |c: &str| c.parse::<crate::keys::KeySpec>().unwrap();
        let (_, app) = super::design_tests::render_with(100, 30);
        assert_eq!(app.keymap.prefixed.get(&key("n")), Some(&Action::ShellHere));
        assert_eq!(app.keymap.prefixed.get(&key("s")), Some(&Action::StartSession));
        assert_eq!(app.keymap.prefixed.get(&key("o")), Some(&Action::NewPane));
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        leader(&mut app, 'n');
        let cmds = sent(&mut rx);
        assert!(matches!(cmds.as_slice(), [Command::NewWorkspace { cwd: Some(_), name: None, cmd: None }]), "a plain shell: {cmds:?}");
        // In the Menu, the leader key shown beside a row runs it.
        for (key, plain) in [('n', true), ('s', false)] {
            app.mode = Mode::Actions { sel: 0 };
            app.on_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
            let cmds = sent(&mut rx);
            let shell = matches!(cmds.as_slice(), [Command::NewWorkspace { cmd: None, .. }]);
            assert_eq!(shell, plain, "Menu key {key}: {cmds:?}");
        }
    }

    #[test]
    fn a_just_started_agent_reads_new_type_to_begin_with_the_accent_ring() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let t = app.snap.terms.get_mut(&2).unwrap();
        (t.asleep, t.status, t.label, t.summary) = (false, Status::Idle, String::new(), String::new());
        let o = draw(&mut app, 160, 45);
        show(&o);
        let at = o.lines().position(|l| l.contains("codex · new · type to begin")).expect("the new row");
        assert!(o.lines().nth(at - 1).is_some_and(|l| l.contains('○')), "the accent ring marks it");
    }

    #[test]
    fn caps_lock_n_after_the_leader_starts_a_session_not_history() {
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

        assert_eq!(app.mode, Mode::Normal, "Caps Lock must not open What happened; the new shell has the keys");
        assert!(std::iter::from_fn(|| rx.try_recv().ok()).any(|message| matches!(
            message,
            ClientMsg::Command(Command::NewWorkspace { cmd: None, .. })
        )), "leader + Caps Lock + n starts a shell");
    }

    #[test]
    fn a_bare_x_in_the_sidebar_does_not_close_the_session() {
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

        assert!(!matches!(&app.mode, Mode::Confirm(_)), "x does not even ask to close session {term} (ADR 0014)");
    }

    #[test]
    fn settings_open_from_the_actions_menu_by_mouse_and_keyboard() {
        for by_mouse in [false, true] {
            let (_, mut app) = super::design_tests::render_with(100, 30);
            app.act(Action::Menu);
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
    fn padding_leaves_air_round_the_whole_screen_in_every_layout() {
        for layout in ["floating", "tiled", "grid"] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            app.cfg.ui.layout = layout.into();
            app.cfg.ui.fold_retired_layout();
            app.cfg.ui.padding = "1".into();
            let o = draw(&mut app, 160, 45);
            show(&o);
            let lines: Vec<&str> = o.lines().collect();
            assert!(lines[0].trim().is_empty(), "{layout}: the top row is air: {:?}", lines[0]);
            assert!(lines.iter().all(|l| l.chars().next().is_none_or(|c| c == ' ')), "{layout}: the left column is air");
            assert_eq!(app.screen.side_rect.x, 1, "{layout}: the sidebar starts one in");
            assert_eq!(app.screen.side_rect.bottom(), 43, "{layout}: and ends above the mode bar, which is one up");
            assert_eq!(screen::mode_bar_row(&app, Rect::new(0, 0, 160, 45)), Rect::new(1, 43, 158, 1), "{layout}: the bar is inside the padding too");
        }
    }

    #[test]
    fn the_hover_is_rounded_with_rounded_shapes_and_square_without() {
        use crate::client::screen::{CAP_L, CAP_R};
        for rounded in [true, false] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            app.cfg.ui.rounded = if rounded { crate::config::Rounding::Full } else { crate::config::Rounding::Off };
            app.mode = Mode::Actions { sel: 0 };
            draw(&mut app, 160, 45);
            let (row, _) = *app.hits.iter().find(|(_, h)| *h == Hit::Screen(screen::ScreenHit::ActionRow(2))).expect("the third Menu row");
            app.hover = Some(Position { x: row.x + 4, y: row.y });
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| render::draw(&mut app, f)).unwrap();
            let buf = term.backend().buffer().clone();
            let ends = (buf[(row.x, row.y)].symbol().to_string(), buf[(row.right() - 1, row.y)].symbol().to_string());
            let want = if rounded { (CAP_L.to_string(), CAP_R.to_string()) } else { (" ".to_string(), " ".to_string()) };
            assert_eq!(ends, want, "rounded {rounded}: the hovered Menu row's ends");
            assert_ne!(buf[(row.x + 2, row.y)].bg, buf[(row.x, row.y + 1)].bg, "rounded {rounded}: the row is tinted");
        }
    }

    /// The frame with the pointer at `at`, and without it.
    fn with_and_without_pointer(app: &mut App, at: Position) -> (ratatui::buffer::Buffer, ratatui::buffer::Buffer) {
        let frame = |app: &mut App| {
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            term.backend().buffer().clone()
        };
        app.hover = None;
        let plain = frame(app);
        app.hover = Some(at);
        (frame(app), plain)
    }

    #[test]
    fn behind_a_modal_nothing_lights_under_the_pointer_and_inside_it_does() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        draw(&mut app, 160, 45);
        let (row, _) = *app.hits.iter().find(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::Session(_)))).expect("a sidebar row");
        let on_row = Position { x: row.x + 2, y: row.y };
        let focused = app.focused().unwrap();
        for modal in ["Close pane", "Settings", "Message"] {
            match modal {
                "Settings" => app.act(Action::Settings),
                "Message" => app.act(Action::Message),
                _ => app.menu_act(menu::Act::End(vec![focused])),
            }
            assert_ne!(app.mode, Mode::Normal, "{modal} is open");
            let (lit, plain) = with_and_without_pointer(&mut app, on_row);
            assert_eq!(lit[(on_row.x, on_row.y)].bg, plain[(on_row.x, on_row.y)].bg, "{modal}: the row behind takes no hover tint");
            assert_eq!(lit, plain, "{modal}: nothing behind lights up");
            app.mode = Mode::Normal;
        }
        app.menu_act(menu::Act::End(vec![focused]));
        draw(&mut app, 160, 45);
        let (yes, _) = *app.hits.iter().find(|(_, h)| *h == Hit::Screen(screen::ScreenHit::ConfirmYes)).expect("the Close button");
        let on_yes = Position { x: yes.x + yes.width / 2, y: yes.y };
        let (lit, plain) = with_and_without_pointer(&mut app, on_yes);
        assert_ne!(lit[(on_yes.x, on_yes.y)].bg, plain[(on_yes.x, on_yes.y)].bg, "the button under the pointer lights up");
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
        app.cfg.ui.fold_retired_layout();
        app.cfg.ui.rounded = crate::config::Rounding::Off;
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.lines().next().unwrap().starts_with("┌─"), "tiled starts at the very corner, square: {}", o.lines().next().unwrap());
        assert!(!o.contains('╭'), "no rounded corners");
    }

    #[test]
    fn a_small_window_tightens() {
        let (o, _) = super::design_tests::render_with(100, 30);
        show(&o);
        let lines: Vec<&str> = o.lines().collect();
        // A 26-column sidebar card, no right-hand meta or question lines.
        assert!(lines[0].starts_with('┌') && lines[0].chars().nth(25) == Some('┐'), "a slim sidebar: {}", lines[0]);
        let side: String = lines.iter().map(|l| l.chars().take(30).collect::<String>() + "\n").collect();
        assert!(!side.contains("working"), "a working row is one line: {side}");
        assert!(side.contains("asks: Run npm"), "a question keeps its line: {side}");
        assert!(o.contains("+ New") && o.contains("Menu"), "both controls fit at the sidebar foot");
        assert!(!o.contains("? keys") && !o.contains("inbox"), "there is no full-width status footer");
    }

    #[test]
    fn the_fleet_is_gone_from_every_place_it_was_offered() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        assert!(!screen::action_items().iter().any(|(label, ..)| label.to_lowercase().contains("fleet")), "not in the Menu");
        assert!(!app.palette_commands().iter().any(|a| a.describe().to_lowercase().contains("fleet")), "not in search");
        assert!(app.session_items(1).unwrap().1.iter().all(|(l, _)| !l.to_lowercase().contains("commander")), "not on an agent's row menu");
        // A binding written for it long ago is harmless, not an error.
        assert_eq!("start-fleet".parse::<Action>().unwrap(), Action::None);
        for (w, h) in [(160, 45), (100, 30)] {
            app.act(Action::NewPane);
            let o = draw(&mut app, w, h);
            assert!(!o.to_lowercase().contains("fleet") && !o.contains("GOAL"), "at {w}x{h}: {o}");
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        }
    }

    #[test]
    fn a_goal_survives_each_shells_quoting() {
        let said = "fix `login` & \"signup\" for $5, it's urgent\nthen the tests".to_string();
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
        assert!(!rows[2].contains("Menu") && o.contains("+ New") && o.contains("Menu"), "new and Actions are at the foot");
        let actions_y = app.hits.iter().find_map(|(rect, hit)| (*hit == Hit::Screen(screen::ScreenHit::Actions)).then_some(rect.y)).unwrap();
        assert!(actions_y > 35, "Actions stays at the bottom of the sidebar");
        assert!(!o.lines().last().unwrap_or_default().contains("keys"), "no full-width status footer");
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
        // Each tab carries a ✕ (ADR 0028); the lone pane's title has none.
        assert_eq!(app.hits.iter().filter(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::CloseTab(_)))).count(), 3, "a ✕ on each tab");
        assert!(!app.hits.iter().any(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::CloseSplit(_)))), "none on the lone pane");
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
        // A middle click closes a tab, once you say so (a shell too).
        let shell_tab = app.session_tabs().into_iter().find(|i| app.screen.tabs[*i].focus == 3).unwrap();
        let at = pill(&app, shell_tab);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Middle), at);
        assert!(matches!(app.mode, Mode::Confirm(_)), "it asks first: {:?}", app.mode);
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
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
            screen::side_lines(app, &model, &app.theme).iter().filter_map(|l| screen::line_term(&model, l)).filter(|t| *t != 3).take(2).collect::<Vec<_>>()
        };
        // claude (1) and codex (2) share a group with the shell (3), which isn't counted.
        assert_eq!(rows(&mut app, Status::Idle), [1, 2]);
        assert_eq!(rows(&mut app, Status::Working), [1, 2], "starting work doesn't move it");
        assert_eq!(rows(&mut app, Status::Done), [1, 2], "nor does finishing");
        assert_eq!(rows(&mut app, Status::Blocked), [2, 1], "needing you does");
        app.cfg.ui.attention_sort = false;
        assert_eq!(rows(&mut app, Status::Blocked), [1, 2], "sorting off: nothing moves");
    }

    /// claude (1) started helpers: codex (2) is its helper, 4 is codex's, 5 is 4's.
    fn helper_tree() -> App {
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
    fn helper_rows(app: &mut App) -> Vec<(TermId, u8)> {
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
        let mut app = helper_tree();
        assert_eq!(helper_rows(&mut app), [(1, 0), (2, 1), (4, 2), (5, 2)], "each under the session that started it, no deeper than two steps");
        // One that needs you goes to the top, and takes its own helpers with it.
        app.snap.terms.get_mut(&2).unwrap().status = Status::Blocked;
        assert_eq!(helper_rows(&mut app), [(2, 0), (4, 1), (5, 2), (1, 0)]);
        // Sorting off: nothing moves, the helpers stay together.
        app.cfg.ui.attention_sort = false;
        assert_eq!(helper_rows(&mut app), [(1, 0), (2, 1), (4, 2), (5, 2)]);
        // The session that started them closed: an ordinary row, its own helpers still under it.
        app.cfg.ui.attention_sort = true;
        app.snap.terms.get_mut(&2).unwrap().status = Status::Idle;
        app.snap.terms.get_mut(&2).unwrap().started_by = Some(999);
        assert_eq!(helper_rows(&mut app), [(1, 0), (2, 0), (4, 1), (5, 2)]);
    }

    #[test]
    fn helpers_step_in_under_the_session_that_started_them() {
        let mut app = helper_tree();
        // The shell shares claude's group, so one claude started sits under it.
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
            assert_eq!(col(&name(2)), col(&name(1)) + 2, "a helper steps in under the session that started it");
            assert_eq!(col(&name(3)), col(&name(1)) + 2, "a shell started by it steps in too");
        }
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
        // claude, codex and the shell: one group.
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
            let pi = app.screen_model().iter().position(|p| p.name == name && p.kind == screen::Kind::Local).unwrap();
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
        assert!(!o.contains("ctx 76%") && !o.contains("$1.20"), "cost is the agent's own to show");
        assert!(o.contains(" 76%"), "context past half full is on its row");
        app.snap.terms.get_mut(&agent).unwrap().resume_at = Some(now + 600);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("resumes in 10m") || o.contains("resumes in 9m"), "waiting on a limit, on its row");
    }



    #[test]
    fn an_update_appears_in_the_tab_bar_only_when_available() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let initial = draw(&mut app, 160, 45);
        assert!(!initial.contains("Update") && initial.contains("Menu"), "actions stay in the sidebar, no update when current");
        assert!(app.hits.iter().any(|(_, hit)| *hit == Hit::Screen(screen::ScreenHit::Actions)));
        app.update_available = Some("9.9.9".into());
        app.update_notes = vec!["Fixed: copying".into(), "New: update button".into()];
        let updated = draw(&mut app, 160, 45);
        assert!(updated.contains("↑ Update 9.9.9"), "the mode bar announces an available update");
        // One notice in the working area: the Menu's Settings row carries no dot for it.
        app.act(Action::Menu);
        let menu = draw(&mut app, 160, 45);
        let settings_row = menu.lines().find(|l| l.contains("Settings")).unwrap_or_default();
        assert!(!settings_row.contains('●'), "the Menu does not repeat the update: {settings_row}");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        draw(&mut app, 160, 45);
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
        // Something running in it, so closing it asks.
        app.snap.terms.get_mut(&3).unwrap().process = "npm".into();
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
        let names = |app: &mut App| app.screen_model().iter().filter(|p| p.kind == screen::Kind::Local).map(|p| p.name.clone()).collect::<Vec<_>>();
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
            vec![(screen::Kind::Local, "shop-api".into()), (screen::Kind::Local, "notes".into()), (screen::Kind::Ssh, "build-box".into())],
            "folders here, then other machines, each grouped by where"
        );
        let o = draw(&mut app, 160, 45);
        show(&o);
        let at = |s: &str| o.find(s).unwrap_or_else(|| panic!("{s} in the sidebar"));
        assert!(at("−  shop-api") < at("−  notes") && at("−  notes") < at("−  build-box"), "folders here, then other machines");
        assert!(!o.contains("AGENTS") && !o.contains("TERMINALS") && at("SSH 1") > at("−  notes"), "only the other machines get a heading");
    }

    #[test]
    fn a_long_name_gives_way_to_its_age() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.snap.terms.get_mut(&2).unwrap().label = "Rate limiter rework for login and signup".into();
        app.screen_fresh();
        let o = draw(&mut app, 140, 30);
        show(&o);
        assert!(o.lines().any(|l| l.contains("Rate limiter") && l.contains("…") && l.contains(" 2m")), "the name is cut, the age stays: {o}");
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
        show(&o);
        let bar = o.lines().find(|l| l.contains("┌─ 1 ") && l.contains("┌─ 2 ")).unwrap_or_default();
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

    /// The frame as a buffer, for looking at single cells.
    fn frame(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        term.backend().buffer().clone()
    }

    /// The mode bar's row, which the toast sits above.
    const BAR: u16 = 1;

    fn row_text(buf: &ratatui::buffer::Buffer, y: u16) -> String {
        (0..buf.area.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string()
    }

    #[test]
    fn a_two_row_toast_sits_above_the_input_area_boxed_with_its_side_and_close() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.notify_card("Copied to clipboard".into(), "214 characters · 5 lines from swift-lynx".into(), None, false);
        let buf = frame(&mut app, 160, 45);
        let t = app.theme.clone();
        let (h, top) = (4u16, 45 - 4 - 5 - BAR);
        assert!(row_text(&buf, top + 1).contains("✓ Copied to clipboard") && row_text(&buf, top + 1).trim_end().ends_with('│'), "{}", row_text(&buf, top + 1));
        assert!(row_text(&buf, top + 1).contains("✕"), "the close sits on the title row");
        assert!(row_text(&buf, top + 2).contains("214 characters · 5 lines from swift-lynx"));
        let x = (0..160u16).find(|&x| buf[(x, top + 1)].symbol() == "┃").expect("the side");
        for y in top + 1..top + h - 1 {
            assert_eq!((buf[(x, y)].symbol(), buf[(x, y)].fg, buf[(x, y)].bg), ("┃", t.accent, t.bg), "the side runs between the box's top and bottom");
        }
        assert_eq!(buf[(x + 1, top + 2)].bg, t.bg, "Floating: an outline on the ground, no filled block");
        let title = (x..160).find(|&c| buf[(c, top + 1)].symbol() == "C").unwrap();
        assert_eq!((buf[(title, top + 1)].fg, buf[(title, top + 1)].modifier.contains(ratatui::style::Modifier::BOLD)), (t.strong, true));
        let detail = (x..160).find(|&c| buf[(c, top + 2)].symbol() == "2").unwrap();
        assert_eq!(buf[(detail, top + 2)].fg, t.text, "the detail is in the text colour");
    }

    #[test]
    fn a_floating_toast_is_round_whatever_the_rounding_setting() {
        use crate::config::Rounding;
        for rounded in [Rounding::Off, Rounding::Corners, Rounding::Full] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            app.cfg.ui.rounded = rounded;
            app.notify_card("Copied to clipboard".into(), "9 characters".into(), None, false);
            let buf = frame(&mut app, 160, 45);
            let t = app.theme.clone();
            let (top, bottom) = (45 - 4 - 5 - BAR, 45 - 5 - 1 - BAR);
            let side = (0..160u16).find(|&x| buf[(x, top + 1)].symbol() == "┃").expect("the side");
            let right = (side + 1..160u16).find(|&x| buf[(x, top + 1)].symbol() == "│").expect("the right side");
            let want = ["╭", "╮", "╰", "╯"];
            let got = [(side, top), (right, top), (side, bottom), (right, bottom)].map(|p| buf[p].symbol().to_string());
            assert_eq!(got, want.map(String::from), "{rounded:?}");
            assert_eq!(buf[(side, top)].fg, t.line, "{rounded:?}: the box in `line`");
        }
    }
    #[test]
    fn a_three_row_toast_previews_in_dim_italics() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.notify_card("Copied to clipboard".into(), "9 characters · 1 line".into(), Some("\"cargo test\"".into()), false);
        let buf = frame(&mut app, 160, 45);
        let t = app.theme.clone();
        let top = 45 - 5 - 5 - BAR;
        let o: String = (0..45).map(|y| row_text(&buf, y) + "\n").collect();
        show(&o);
        assert!(row_text(&buf, top + 3).contains("\"cargo test\""), "{}", row_text(&buf, top + 3));
        let q = (0..160u16).find(|&x| buf[(x, top + 3)].symbol() == "\"").unwrap();
        assert_eq!((buf[(q, top + 3)].fg, buf[(q, top + 3)].modifier.contains(ratatui::style::Modifier::ITALIC)), (t.muted, true));
        let side = (0..160u16).find(|&x| buf[(x, top + 1)].symbol() == "┃").expect("the side");
        assert_eq!(buf[(side, top + 4)].symbol(), "╰", "the box ends right under the preview");
        assert_ne!(buf[(side, top + 5)].symbol(), "╰", "and no further");
    }

    #[test]
    fn a_toast_clears_after_its_time_or_with_the_close_or_esc() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.notify("Copied".into(), false);
        let past = |app: &mut App, ms: u64| app.notice.as_mut().unwrap().1 = Instant::now() - Duration::from_millis(ms);
        past(&mut app, 2300);
        assert!(draw(&mut app, 160, 45).contains("Copied"), "still there just before 2.4 s");
        past(&mut app, 2500);
        assert!(!draw(&mut app, 160, 45).contains("Copied"), "gone after 2.4 s");
        // The close.
        app.notify("Copied".into(), false);
        draw(&mut app, 160, 45);
        let r = app.hits.iter().rev().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::ToastClose)).then_some(*r)).expect("a close");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: r.x, row: r.y, modifiers: KeyModifiers::NONE });
        assert!(app.notice.is_none(), "✕ clears it");
        // Esc clears it and still goes to the pane.
        app.notify("Copied".into(), false);
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.notice.is_none(), "Esc clears it");
    }

    #[test]
    fn a_mouse_selection_shows_no_copy_keys_since_release_copies_it() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut p = vt100::Parser::new(20, 80, 100);
        p.process(b"alpha beta gamma
second line
");
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        assert!(app.enter_copy(1));
        let r = app.panes.iter().find(|(t, _)| *t == 1).map(|(_, r)| *r).unwrap();
        if let Mode::Copy(c) = &mut app.mode {
            (c.mouse, c.cur, c.anchor) = (true, (0, 6), Some((0, 0)));
        }
        let line = row_text(&frame(&mut app, 160, 45), r.y);
        assert!(line.contains("alpha b") && !line.contains(" copy · ") && !line.contains(" cancel"), "{line}");
    }

    #[test]
    fn selecting_tints_sky_with_hints_and_copying_glows_accent_with_a_toast() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut p = vt100::Parser::new(20, 80, 100);
        p.process(b"alpha beta gamma\r\nsecond line\r\n");
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        assert!(app.enter_copy(1));
        let r = app.panes.iter().find(|(t, _)| *t == 1).map(|(_, r)| *r).unwrap();
        if let Mode::Copy(c) = &mut app.mode {
            c.cur = (0, 6);
            c.anchor = Some((0, 0));
        }
        let buf = frame(&mut app, 160, 45);
        let t = app.theme.clone();
        let sky = render::blend(t.bg, t.sky(), 0.38);
        assert_eq!((buf[(r.x, r.y)].symbol(), buf[(r.x, r.y)].bg, buf[(r.x, r.y)].fg), ("a", sky, t.strong), "the selection is sky with strong text");
        assert_eq!(buf[(r.x + 6, r.y)].bg, sky, "through the cursor's cell");
        assert_ne!(buf[(r.x + 8, r.y)].bg, sky, "and no further");
        let line = row_text(&buf, r.y);
        assert!(line.contains(" copy · ") && line.contains(" cancel") && line.contains('y') && line.contains("Esc"), "{line}");
        // y copies: the same cells glow accent, and the toast says what was copied.
        app.on_key(KeyEvent::new(KeyCode::Char('y'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Normal) && app.copy_flash.is_some());
        let buf = frame(&mut app, 160, 45);
        let glow = render::blend(t.bg, t.accent, 0.34);
        assert_eq!((buf[(r.x, r.y)].bg, buf[(r.x, r.y)].fg), (glow, t.strong), "copied: accent-tinted");
        let o: String = (0..45).map(|y| row_text(&buf, y) + "\n").collect();
        show(&o);
        assert!(o.contains("Copied to clipboard") && o.contains("7 characters · 1 line"), "{o}");
        assert!(o.contains("\"alpha b\""), "the preview quotes the text");
        assert_eq!(copy::LAST_COPY.with(|c| c.borrow().clone()).as_deref(), Some("alpha b"), "what reached the clipboard (recorded, never the real one)");
        app.copy_flash.as_mut().unwrap().at = Instant::now() - copy::FLASH_FOR;
        let buf = frame(&mut app, 160, 45);
        assert_ne!(buf[(r.x, r.y)].bg, glow, "the glow ends");
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

    fn one_snapshot(app: &mut App) {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let kept = vec![crate::snapshot::Snapshot { commit: "c0".into(), at: now - 120, why: "before claude's turn".into() }];
        app.snapshots_menu(std::path::PathBuf::from("/code/dsa-tool"), 1, kept, (30, 6));
    }

    #[test]
    fn a_snapshot_is_not_restored_while_its_agent_works() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.snap.terms.get_mut(&1).unwrap().status = Status::Working;
        one_snapshot(&mut app);
        app.on_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert!(!matches!(app.mode, Mode::Confirm(_)), "nothing to confirm: {:?}", app.mode);
        assert_eq!(app.notice.as_ref().map(|n| n.0.as_str()), Some(crate::client::menu::RESTORE_WHILE_WORKING));
    }

    #[test]
    fn a_restore_confirmed_before_the_agent_started_working_is_still_refused() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.snap.terms.get_mut(&1).unwrap().status = Status::Idle;
        one_snapshot(&mut app);
        app.on_key(KeyEvent::new(KeyCode::Char('1'), KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Confirm(_)), "idle: it asks first");
        app.snap.terms.get_mut(&1).unwrap().status = Status::Working;
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.notice.as_ref().map(|n| n.0.as_str()), Some(crate::client::menu::RESTORE_WHILE_WORKING));
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
        assert!(o.contains("queues this until the turn ends") && o.contains("Ctrl"), "the box says what Enter does now");
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
        let legacy_ui = o.lines().find(|line| line.contains("waking up"));
        assert!(legacy_ui.is_none(), "legacy status UI is absent: {legacy_ui:?}");
        assert!(o.contains("asleep zzZ"), "the row says it is asleep in words");
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
        let title = o.lines().find(|l| l.contains("▲ 62% CPU · 5.0 GB")).expect("said on the pane's top border");
        assert!(title.contains('┐'), "on the top border, at its right: {title}");
        let model = app.screen_model();
        assert!(!screen::side_lines(&app, &model, &app.theme).iter().any(|l| matches!(l, screen::Line::Note(text, ..) if text.starts_with("▲"))), "and no longer under its sidebar row");
        assert!(!o.contains("subagents"), "no subagent heading to fold");
    }

    /// What the client sent as helper answers, in order.
    fn helper_answers(rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::protocol::ClientMsg>) -> Vec<(TermId, crate::protocol::HelperChoice)> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| match m {
                crate::protocol::ClientMsg::Command(crate::protocol::Command::AnswerHelpers { term, choice }) => Some((term, choice)),
                _ => None,
            })
            .collect()
    }

    /// Session 2 asks for helpers: needs you, with three tasks.
    fn asking_for_helpers() -> (App, tokio::sync::mpsc::UnboundedReceiver<crate::protocol::ClientMsg>) {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let t = app.snap.terms.get_mut(&2).unwrap();
        t.status = Status::Blocked;
        t.helper_ask = vec!["fix the login test".into(), "port the footer".into(), "update the docs".into()];
        app.screen_fresh();
        (app, rx)
    }

    #[test]
    fn a_session_asking_for_helpers_reads_wants_to_start_n_helpers_and_enter_opens_the_question() {
        let (mut app, mut rx) = asking_for_helpers();
        for (w, h) in [(160, 45), (100, 30)] {
            let o = draw(&mut app, w, h);
            show(&o);
            let line = if w >= 160 { "wants to start 3 helpers" } else { "wants 3 helpers" };
            assert!(o.contains(line), "the row's line at {w}x{h}");
            assert!(o.contains("●2"), "it needs you: counted in the sidebar's title");
        }
        // Bare letters and digits do not answer from the sidebar (ADR 0014).
        app.screen_side_set(screen::SideItem::Sess(2));
        for c in ['1', '2', '3', 'y'] {
            app.mode = Mode::Side;
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        assert!(helper_answers(&mut rx).is_empty(), "nothing is answered from the sidebar");
        app.screen_side_set(screen::SideItem::Sess(2));
        app.mode = Mode::Side;
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Helpers { term: 2, side: true }, "Enter on the row opens the question");
        for (w, h) in [(160, 45), (100, 30), (80, 24)] {
            let o = draw(&mut app, w, h);
            show(&o);
            for must in ["START HELPERS?", "fix the login test", "port the footer", "update the docs", "Their permission prompts come to your Inbox.", "Nothing merges.", "Allow", "Esc later"] {
                assert!(o.contains(must), "{must:?} at {w}x{h}");
            }
        }
        let wide = draw(&mut app, 160, 45);
        assert!(wide.contains("Always in shop-api") && wide.contains("No, do it alone"), "the full words when they fit:\n{wide}");
        // Enter, y and Space are not an answer.
        for c in [KeyCode::Enter, KeyCode::Char('y'), KeyCode::Char(' ')] {
            app.on_key(KeyEvent::new(c, KeyModifiers::NONE));
            assert_eq!(app.mode, Mode::Helpers { term: 2, side: true });
        }
        assert!(helper_answers(&mut rx).is_empty());
        // Esc is "later": nothing is sent and the row still asks.
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(app.mode, Mode::Side);
        assert!(helper_answers(&mut rx).is_empty() && app.helpers_asked(2));
        assert!(draw(&mut app, 160, 45).contains("wants to start 3 helpers"), "the row stays");
        // 1, 2 and 3 answer.
        use crate::protocol::HelperChoice::{Allow, Always, No};
        for (key, want) in [('1', Allow), ('2', Always), ('3', No)] {
            app.mode = Mode::Helpers { term: 2, side: true };
            app.on_key(KeyEvent::new(KeyCode::Char(key), KeyModifiers::NONE));
            assert_eq!(helper_answers(&mut rx), [(2, want)], "{key}");
            assert_eq!(app.mode, Mode::Side);
        }
    }

    #[test]
    fn the_helpers_question_is_reachable_and_answerable_by_mouse() {
        let (mut app, mut rx) = asking_for_helpers();
        draw(&mut app, 160, 45);
        let line = app.hits.iter().rev().find_map(|(_, h)| match h {
            Hit::Screen(screen::ScreenHit::HelperAsk(2)) => Some(*h),
            _ => None,
        });
        let Some(Hit::Screen(open)) = line else { panic!("the row's line is a click target") };
        app.on_hy_hit(open, false);
        assert!(matches!(app.mode, Mode::Helpers { term: 2, .. }), "a click on the line opens the question");
        draw(&mut app, 160, 45);
        let choose = |app: &App, c| app.hits.iter().find_map(|(_, h)| match h {
            Hit::Screen(screen::ScreenHit::HelperAnswer(2, got)) if *got == c => Some(screen::ScreenHit::HelperAnswer(2, c)),
            _ => None,
        });
        let always = choose(&app, crate::protocol::HelperChoice::Always).expect("Always is clickable");
        choose(&app, crate::protocol::HelperChoice::Allow).expect("Allow is clickable");
        let no = choose(&app, crate::protocol::HelperChoice::No).expect("No is clickable");
        app.on_hy_hit(no, false);
        assert_eq!(helper_answers(&mut rx), [(2, crate::protocol::HelperChoice::No)]);
        assert!(!matches!(app.mode, Mode::Helpers { .. }));
        // With the question gone from the row, a stale click answers nothing.
        app.snap.terms.get_mut(&2).unwrap().helper_ask.clear();
        app.mode = Mode::Helpers { term: 2, side: false };
        app.on_hy_hit(always, false);
        assert!(helper_answers(&mut rx).is_empty(), "nothing is pending");
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn the_settings_tabs_carry_no_update_dot() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.update_available = Some("9.9.9".into());
        app.act(Action::Settings);
        let o = draw(&mut app, 160, 45);
        let tabs = o.lines().find(|l| l.contains("General") && l.contains("Sessions")).expect("the tab strip");
        assert!(!tabs.contains('●'), "the pill and the splash say so; the tabs do not: {tabs}");
        let version_row = design::settings_rows(modal::Cat::General).iter().position(|r| matches!(r, design::SRow::Version)).unwrap();
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: version_row, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        assert!(o.lines().any(|l| l.contains("Version") && l.contains("9.9.9 is out") && l.contains("Enter to update")), "General's Version row offers the update: {o}");
    }

    #[test]
    fn the_inbox_opens_the_helper_question_and_goes_back_to_it() {
        let (mut app, mut rx) = asking_for_helpers();
        app.act(Action::Jump);
        let at = |app: &App| screen::goto_rows(&app.screen_model(), "", screen::heads_up(app).len()).iter().position(|r| matches!(r, screen::GoRow::Ask(2))).expect("the asking session is in the Inbox");
        for key in [KeyCode::Enter, KeyCode::Char('1')] {
            let sel = at(&app);
            app.mode = Mode::GoTo { query: String::new(), sel };
            app.on_key(KeyEvent::new(key, KeyModifiers::NONE));
            assert_eq!(app.mode, Mode::Helpers { term: 2, side: false }, "{key:?} on the row opens the question, not a message box or a typed number");
            let o = draw(&mut app, 160, 45);
            assert!(o.contains("START HELPERS?") && o.contains("INBOX"), "the question over the Inbox:\n{o}");
            app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
            assert_eq!(app.mode, Mode::GoTo { query: String::new(), sel }, "Esc: back in the Inbox, on the same row");
        }
        assert!(helper_answers(&mut rx).is_empty(), "nothing was answered, and no number reached the agent");
        // Answering from there goes back to the Inbox too.
        let sel = at(&app);
        app.mode = Mode::GoTo { query: String::new(), sel };
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        app.on_key(KeyEvent::new(KeyCode::Char('3'), KeyModifiers::NONE));
        assert_eq!(helper_answers(&mut rx), [(2, crate::protocol::HelperChoice::No)]);
        assert!(matches!(app.mode, Mode::GoTo { .. }), "answered: back in the Inbox");
    }

    #[test]
    fn the_menu_opens_from_the_leader_not_from_a_bare_letter_in_the_sidebar() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.screen_side_set(screen::SideItem::Sess(2));
        assert_eq!(app.mode, Mode::Side);
        app.on_key(KeyEvent::new(KeyCode::Char('m'), KeyModifiers::NONE));
        assert!(!matches!(app.mode, Mode::Actions { .. }), "a bare m does not open the menu from the sidebar (ADR 0014)");
        app.act(Action::Menu);
        assert!(matches!(app.mode, Mode::Actions { .. }), "the leader's m does");
        // And no row's menu item is ever handed a key the sidebar already uses.
        let items: Vec<(String, crate::client::menu::Act)> = ["Archive", "Ask", "Mark", "Inspect", "Jump", "Keep"].iter().map(|l| (l.to_string(), crate::client::menu::Act::RenamePane(1))).collect();
        let keys = crate::client::menu::menu_chars(&app, &items);
        assert!(keys.iter().flatten().all(|k| !['a', 'm', 'i', 'j', 'k'].contains(k)), "{keys:?}");
    }

    #[test]
    fn the_selected_row_still_says_its_state_in_every_theme() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The colour of a row's mark in the sidebar, left of its name.
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
            (buf[(x - 2, y)].fg, buf[(x - 2, y)].bg)
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
            // Working is the accent, as everywhere.
            app.snap.terms.get_mut(&1).unwrap().status = Status::Working;
            let (fg, bg) = name_fg(&mut app, "claude");
            assert_eq!(fg, t.on_lit(t.accent), "{theme}: working is the accent on the lit row");
            assert!(crate::theme::contrast(fg, bg) >= 3.0, "{theme}: and reads there");
            // No state to show: a quiet mark.
            app.snap.terms.get_mut(&1).unwrap().status = Status::Idle;
            assert_eq!(name_fg(&mut app, "claude").0, t.on_lit(t.muted), "{theme}: idle is a quiet mark");
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
    fn a_closed_worktree_with_work_only_there_asks_and_enter_keeps_it() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        let ask = |app: &mut App| {
            app.mode = Mode::Normal;
            app.on_server(ServerMsg::AskRemoveWorktree { path: PathBuf::from("/wt/task"), name: "task".into(), why: "not merged anywhere".into() });
        };
        let removals = |rx: &mut tokio::sync::mpsc::UnboundedReceiver<crate::protocol::ClientMsg>| {
            std::iter::from_fn(|| rx.try_recv().ok())
                .filter_map(|m| match m {
                    crate::protocol::ClientMsg::Command(crate::protocol::Command::TidyWorktrees { items }) => Some(items),
                    _ => None,
                })
                .collect::<Vec<_>>()
        };
        ask(&mut app);
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.what == "task" && c.note.contains("not merged anywhere") && c.danger), "it asks, and says why");
        let o = draw(&mut app, 100, 30);
        assert!(o.contains("Keep worktree?") && o.contains("Remove folder"), "{o}");
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Normal) && removals(&mut rx).is_empty(), "Enter keeps the folder and sends nothing");
        ask(&mut app);
        app.on_key(key(KeyCode::Esc));
        assert!(removals(&mut rx).is_empty(), "Esc keeps it too");
        ask(&mut app);
        app.on_key(key(KeyCode::Char('r')));
        assert_eq!(removals(&mut rx), [vec![(PathBuf::from("/wt/task"), false)]], "only the button's key removes it, and never forces");
        // Another dialog is open: the question waits as a notice, nothing is replaced.
        app.mode = Mode::Prompt { kind: PromptKind::NewWorkspace, input: String::new() };
        app.on_server(ServerMsg::AskRemoveWorktree { path: PathBuf::from("/wt/task"), name: "task".into(), why: "not merged anywhere".into() });
        assert!(matches!(app.mode, Mode::Prompt { .. }), "the open dialog stays");
    }

    #[test]
    fn you_can_ask_for_an_update_check_and_are_told_what_it_found() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let said = |app: &App| app.notice.as_ref().map(|(m, _, err)| (m.clone(), *err)).unwrap();
        assert!(app.palette_commands().contains(&Action::CheckUpdate), "in the palette");
        // Settings: General's Version row, when no newer one is known; no other tab shows it.
        let version_row = design::settings_rows(modal::Cat::General).iter().position(|r| matches!(r, design::SRow::Version)).expect("General has a Version row");
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: version_row, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        let line = o.lines().find(|l| l.contains("Version")).unwrap_or_else(|| panic!("the Version row
{o}"));
        assert!(line.contains(&format!(concat!(env!("APP_COMMAND"), " {}"), env!("CARGO_PKG_VERSION"))) && line.contains("up to date") && line.contains("u check now"), "{line}");
        assert!(o.contains("UPDATES"), "under UPDATES");
        let keys = modal::Cat::ALL.iter().position(|c| *c == modal::Cat::Keys).unwrap();
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: keys, sel: 0, editing: None, capturing: false, scroll: 0 }));
        assert!(!draw(&mut app, 160, 45).contains("up to date"), "not at the top of every tab");
        // While it's asking, the row says so.
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: version_row, editing: None, capturing: false, scroll: 0 }));
        app.update_checking = true;
        let o = draw(&mut app, 160, 45);
        assert!(o.lines().any(|l| l.contains("Version") && l.contains("checking…")), "{o}");
        // Nothing newer: you asked, so it says so. A check the app made itself stays quiet.
        app.notice = None;
        app.update_checked_now(false, Ok(None), Vec::new());
        assert!(app.notice.is_none(), "its own checks say nothing when there's nothing new");
        app.update_checked_now(true, Ok(None), Vec::new());
        assert!(!app.update_checking && said(&app) == (format!(concat!("Installed ", env!("APP_COMMAND"), " {} is the latest"), env!("CARGO_PKG_VERSION")), false));
        app.update_checked_now(true, Err("couldn't reach GitHub: timed out".into()), Vec::new());
        assert!(said(&app).1 && said(&app).0.contains("Couldn't check for updates"), "{:?}", said(&app));
        // A newer one: the Update button takes the place of the check.
        app.update_checked_now(true, Ok(Some("9.9.9".into())), vec!["New: something".into()]);
        assert_eq!(app.update_available.as_deref(), Some("9.9.9"));
        let o = draw(&mut app, 160, 45);
        let line = o.lines().find(|l| l.contains("Version")).unwrap_or_else(|| panic!("{o}"));
        assert!(line.contains("9.9.9 is out") && line.contains("Enter to update") && !line.contains("check now"), "{line}");
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
    fn a_toast_s_mark_wears_the_colour_of_its_news() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // The fill and ink under the first letter of `word`, and the mark in front of it with its colour.
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
            (buf[(x, y)].bg, buf[(x, y)].fg, buf[(x - 2, y)].symbol().to_string(), buf[(x - 2, y)].fg)
        };
        let t = app.theme.clone();
        app.notify("Copied".into(), false);
        assert_eq!(toast(&mut app, "Copied"), (t.bg, t.strong, "✓".to_string(), t.accent), "a plain note: the accent");
        app.notify("Couldn't save".into(), true);
        assert_eq!(toast(&mut app, "Couldn't"), (t.bg, t.strong, "✕".to_string(), t.err), "an error");
        // claude (1) is on a prompt; codex (2) isn't.
        app.notify("pane-one needs you".into(), false);
        app.notice_term = Some(1);
        assert_eq!(toast(&mut app, "pane-one"), (t.bg, t.strong, "●".to_string(), t.blocked), "needs you: its state's colour");
        app.notify("pane-two finished".into(), false);
        app.notice_term = Some(2);
        assert_eq!(toast(&mut app, "pane-two"), (t.bg, t.strong, "✓".to_string(), t.done), "finished");
    }

    #[test]
    fn go_to_a_project_or_session() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |c: KeyCode| KeyEvent::new(c, KeyModifiers::NONE);
        app.act(Action::Jump);
        let o = draw(&mut app, 160, 45);
        show(&o);
        // claude needs you in the fixture: the Inbox; the search shows once you type.
        assert!(o.contains("INBOX") && !o.contains("type to find any session"));
        for c in "rate".chars() {
            app.on_key(key(KeyCode::Char(c)));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("FOUND  1") && o.contains("› rate█") && o.contains("rate · codex · shop-api"), "typing finds what matches");
        let Mode::GoTo { query, sel } = app.mode.clone() else { panic!("still open") };
        let rows = screen::goto_rows(&app.screen_model(), &query, 0);
        assert!(matches!(rows[sel], screen::GoRow::Sess(..)), "it lands on the matching session");
        app.on_key(key(KeyCode::Enter));
        assert!(matches!(app.mode, Mode::Normal), "Enter goes there");
    }

    #[test]
    fn the_search_and_keys_look_like_the_app() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.act(Action::Palette);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Search commands") && o.contains("RECENT") && o.contains("Inbox") && !o.contains("workspace ·"), "commands only, with the latest first");
        for c in "split".chars() {
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
        }
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(" matches") && o.contains("Split side by side") && o.contains("Split stacked") && o.contains("Panes") && !o.contains("Settings"), "typing narrows it, each with its group");
        app.mode = Mode::KeyMap(Box::new(screen::KeyMap::new(screen::KeyView::Map, None)));
        let o = draw(&mut app, 160, 45);
        show(&o);
        let a = o.lines().find(|l| l.contains("Inbox")).unwrap();
        let b = o.lines().find(|l| l.contains("New session  ") || l.contains("New session ")).unwrap();
        let column = |line: &str, label: &str| line.find(label).map(|byte| line[..byte].chars().count());
        assert_eq!(column(a, "Inbox"), column(b, "New session"), "labels of a group line up by terminal column");
    }

    #[test]
    fn settings_keys_tab_is_current() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let cat = modal::Cat::ALL.iter().position(|c| *c == modal::Cat::Keys).unwrap();
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat, sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("SESSIONS") && o.contains("Inbox") && o.contains("New session with options"), "{o}");
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
        assert!(o.contains("INBOX") && !o.contains("─ 1  claude "), "narrow: the sheet covers the pane column");
        app.mode = Mode::Normal;
        // Changes of a real repo.
        let repo = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-sheet-{}"), std::process::id()));
        let _ = std::fs::remove_dir_all(&repo);
        std::fs::create_dir_all(&repo).unwrap();
        crate::proc::git(&repo, &["init", "-q"]).unwrap();
        app.open_changes(repo.clone());
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains(" Changes c ") && o.contains("fix the flaky checkout test"), "Changes docks beside the panes");
        assert!(!o.contains("─ Changes "), "its lit tab names it; the title doesn't repeat it");
        let _ = std::fs::remove_dir_all(&repo);
    }

    #[test]
    fn a_follow_up_and_why_open_beside_the_row() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let key = |app: &mut App, c: KeyCode, m: KeyModifiers| app.on_key(KeyEvent::new(c, m));
        // claude's row (it needs you): m writes to it from the sidebar.
        app.mode = Mode::Side;
        app.screen.cursor = Some(1);
        app.start_compose(1, true, None);
        assert!(matches!(&app.mode, Mode::Compose(c) if c.term == 1 && c.side), "a follow-up to the row's agent");
        for ch in "use the fake clock".chars() {
            key(&mut app, KeyCode::Char(ch), KeyModifiers::NONE);
        }
        key(&mut app, KeyCode::Enter, KeyModifiers::SHIFT);
        key(&mut app, KeyCode::Char('x'), KeyModifiers::NONE);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("MESSAGE claude") && o.contains("claude asked") && o.contains("Run npm test -- checkout?") && o.contains("use the fake clock") && o.contains("5 words"), "the question, then the text, its word count");
        assert!(o.contains("◂"), "it points at the row");
        key(&mut app, KeyCode::Enter, KeyModifiers::NONE);
        assert!(app.mode == Mode::Side, "sent: back on the sidebar");
        assert!(app.screen.receipts.contains_key(&1), "the row carries the receipt, not a notice");
        // i: why it has its status.
        let info = app.snap.terms.get_mut(&1).unwrap();
        (info.status_why, info.why_hook, info.why_hook_at) = ("hook: Notification".into(), "Notification".into(), 1);
        app.act(Action::Why);
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
        assert!(matches!(&app.mode, Mode::Compose(c) if c.inbox.is_some()) && o.contains("INBOX") && o.contains("Next prompt for claude"), "the box in the Inbox row");
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
        let heads: Vec<&str> = rows.iter().filter_map(|r| if let screen::GoRow::Head(h) = r { Some(*h) } else { None }).collect();
        assert!(heads.first() == Some(&"NEEDS YOU") && heads.last() == Some(&"HEADS UP"), "{heads:?}");
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
        assert!(matches!(app.view, Some(View::Both(_))) && o.contains("─ Changes ") && o.contains("reading both diffs"), "both diffs, docked");
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
        assert!(o.contains("FINISHED  1") && o.contains("finished · not seen"), "what it finished");
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
            let (sheet, panes) = screen::split_for_sheet(160, col, screen::Look::of(&app.cfg.ui), frac, 72);
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
        assert_eq!(gap(&mut app), app.theme.bg, "solid, the default: the theme's own ground");
        app.cfg.ui.background = "terminal".into();
        assert_eq!(gap(&mut app), ratatui::style::Color::Reset, "terminal: SGR 49 under the chrome");
    }

    #[test]
    fn the_full_width_status_footer_is_removed_in_every_layout() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let cards = draw(&mut app, 160, 45);
        assert!(!cards.lines().last().unwrap_or_default().contains("inbox"));
        assert!(!cards.lines().last().unwrap_or_default().contains("settings"));
        assert!(cards.contains("+ New") && cards.contains("Menu"), "sidebar controls remain in the sidebar");
    }

    #[test]
    fn general_settings_are_in_sections_with_the_version_under_updates() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: 0, editing: None, capturing: false, scroll: 0 }));
        let o = draw(&mut app, 160, 45);
        show(&o);
        let top: String = o.lines().take(6).collect::<Vec<_>>().join("
");
        assert!(!top.contains("up to date"), "no version line over the tabs' pages: {top}");
        let rows_here = design::settings_rows(modal::Cat::General);
        let version = rows_here.iter().position(|r| matches!(r, design::SRow::Version)).expect("a Version row");
        assert_eq!(design::settings_group(&rows_here[version]), "UPDATES");
        let check = rows_here.iter().position(|r| matches!(r, design::SRow::Setting(s) if s.path == "ui.update_check")).expect("Check for updates");
        assert_eq!((version, check), (0, 1), "Updates head General: the version, then Check for updates");
        assert!(!o.contains("Installed here") && !o.contains("This window"), "the detail lines only show when something differs");
        let rows = design::settings_rows(modal::Cat::General);
        let sections: Vec<(&str, &str)> = rows.iter().filter_map(|r| match r { design::SRow::Setting(s) => Some((design::settings_group(r), s.label)), _ => None }).collect();
        let pos = |label: &str| sections.iter().position(|(_, l)| *l == label).unwrap_or_else(|| panic!("{label} is on the General page"));
        assert_eq!(sections[pos("Leader key")].0, "INPUT");
        assert_eq!(sections[pos("Editor")].0, "SHELL");
        assert!(pos("Check for updates") < pos("Splash screen") && pos("Splash screen") < pos("Leader key"), "updates, then startup, then input");
        assert!(pos("Leader key") < pos("Mouse") && pos("Mouse") < pos("Leader waits for next key") && pos("Leader waits for next key") < pos("Shell"), "input, then the shell");
        assert!(pos("Shell") < pos("Editor"), "the shell first in its group");
        // How it looks lives on the Appearance page, not here.
        for label in ["Layout", "Background", "Sidebar side", "Motion"] {
            assert!(!sections.iter().any(|(_, l)| *l == label), "{label} is not a General setting");
        }
        let looks = design::settings_rows(modal::Cat::Appearance);
        let looks: Vec<(&str, &str)> = looks.iter().filter_map(|r| match r { design::SRow::Setting(s) => Some((design::settings_group(r), s.label)), _ => None }).collect();
        for label in ["Layout", "Background", "Sidebar side", "Motion"] {
            assert!(looks.iter().any(|(g, l)| *l == label && *g == "LAYOUT"), "{label} is on the Appearance page");
        }
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
    fn a_finished_note_names_the_repository_and_branch() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        {
            let t = app.snap.terms.get_mut(&2).unwrap();
            t.label = "Checkout rewrite".into();
            t.root = Some(std::path::PathBuf::from("/code/shop-api"));
            t.top = Some(std::path::PathBuf::from("/code/shop-api-worktrees/grey-wolf"));
            t.branch = Some("fix/early-alerts".into());
            t.status = Status::Done;
        }
        app.screen_fresh();
        app.on_server(crate::protocol::ServerMsg::Attention { term: 2, status: Status::Done });
        app.settling.get_mut(&2).unwrap().1 = Instant::now() - Duration::from_millis(1);
        app.settle_alerts();
        let said = app.notice.as_ref().map(|(m, ..)| m.clone()).unwrap_or_default();
        assert_eq!(said, "Checkout rewrite (shop-api · fix/early-alerts) finished", "the worktree folder alone wouldn't say which");
    }

    #[test]
    fn a_finished_note_outside_a_repository_names_the_folder() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        {
            let t = app.snap.terms.get_mut(&2).unwrap();
            t.label = "Notes".into();
            (t.root, t.top, t.branch) = (None, None, None);
            t.cwd = std::path::PathBuf::from("/home/me/notes");
            t.status = Status::Done;
        }
        app.screen_fresh();
        app.on_server(crate::protocol::ServerMsg::Attention { term: 2, status: Status::Done });
        app.settling.get_mut(&2).unwrap().1 = Instant::now() - Duration::from_millis(1);
        app.settle_alerts();
        let said = app.notice.as_ref().map(|(m, ..)| m.clone()).unwrap_or_default();
        assert_eq!(said, "Notes (notes) finished", "like the desktop and phone alerts");
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
            let pane_lit = (pane.y + 1..pane.bottom() - 1).any(|y| buf[(pane.x, y)].fg == accent && buf[(pane.x, y)].symbol() == "│");
            let sheet_lit = (0..45u16).any(|y| (pane.right()..160u16).any(|x| buf[(x, y)].fg == accent && matches!(buf[(x, y)].symbol(), "▕" | "▏" | "│")));
            (pane_lit, sheet_lit)
        };
        assert_eq!(lit_edges(&mut app), (true, false), "the pane has the keys: its edge is lit");
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: 1, editing: None, capturing: false, scroll: 0 }));
        assert_eq!(lit_edges(&mut app), (false, false), "the modal dims the workspace and overlays it without creating a sheet edge");
    }

    #[test]
    fn a_card_is_square_or_rounded_on_one_ground() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        // Square corners (the default): a line round the card on the desk's ground, the
        // card's inside on the same ground.
        let side = app.screen.side_rect;
        let y = side.y + side.height / 2;
        assert_eq!(buf[(side.x, y)].symbol(), "│", "the card's line");
        assert_eq!(buf[(side.x, y)].bg, buf[(side.x + 1, y)].bg, "one ground: the line, not a second grey, marks the card");
        assert_eq!(buf[(side.x, side.y)].symbol(), "┌", "a square corner");
        // Rounded keeps the centred line.
        app.cfg.ui.rounded = crate::config::Rounding::Full;
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        assert_eq!(buf[(side.x, y)].symbol(), "│");
        assert_eq!(buf[(side.x, side.y)].symbol(), "╭");
    }

    #[test]
    fn agent_marks_wear_their_state() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        // Nothing lit by the keyboard, and the window on neither of them.
        app.screen.cursor = None;
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        // The mark left of a sidebar row's name (the sidebar only, not the card titles).
        let side = app.screen.side_rect.right();
        let mark_at = |name: &str| {
            (0..45u16).find_map(|y| {
                let row: String = (0..side).map(|x| buf[(x, y)].symbol().to_string()).collect();
                let col = row.find(&format!(" {name} "))?;
                Some(buf[(row[..col].chars().count() as u16 - 1, y)].fg)
            })
        };
        let t = &app.theme;
        assert_eq!(mark_at("claude"), Some(t.blocked), "needs you: its colour");
        let codex = mark_at("rate-limit").or_else(|| mark_at("rate")).expect("the working codex in the sidebar (named for its worktree)");
        assert!(codex != t.blocked && codex != t.done, "working is neither amber nor green: {codex:?}");
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
        assert!(row(y).contains('✓') && !row(y).contains('♪'), "the done mark, not the bell: {:?}", row(y));
        let dot = (0..40u16).find(|&x| buf[(x, y)].symbol() == "✓").unwrap();
        assert_eq!(buf[(dot, y)].fg, app.theme.done, "a green mark");
    }

    #[test]
    fn only_the_part_with_the_keys_looks_focused() {
        let (_, mut app) = super::design_tests::render_with(120, 30);
        // The colour of the focused card's top-right corner, and the sidebar card's left edge.
        let borders = |app: &mut App| {
            let mut term = Terminal::new(TestBackend::new(120, 30)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            let buf = term.backend().buffer().clone();
            let y = app.pane_frames[0].1.y;
            let x = 119;
            let side = (1..29).filter(|&y| buf[(0u16, y)].fg == app.theme.accent).count();
            (buf[(x, y)].fg, side)
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
        for (c, what) in [('i', "inbox"), ('g', "worktrees"), (',', "settings")] {
            app.mode = Mode::Normal;
            app.on_key(lead);
            assert!(matches!(app.mode, Mode::Prefix { .. }), "leader waits");
            assert_eq!(shortcut_tag(&mut app).map(|(tag, _)| tag).as_deref(), Some("SHORTCUT"), "the mode bar says so");
            app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
            assert!(!matches!(app.mode, Mode::Normal | Mode::Prefix { .. }), "leader + {c} opens {what}: {:?}", std::mem::discriminant(&app.mode));
        }
    }

    #[test]
    fn pane_colours_show_shortcut_mode_on_the_focused_border_too() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.pane_colors = true;
        let border = |app: &mut App| {
            let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
            term.draw(|f| render::draw(app, f)).unwrap();
            let r = app.pane_frames.first().expect("a pane").1;
            term.backend().buffer()[(r.x, r.y + 1)].fg
        };
        let own = border(&mut app);
        assert_ne!(own, app.theme.sky(), "typing: the pane's own colour");
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        assert_eq!(border(&mut app), app.theme.sky(), "shortcut mode: sky");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(border(&mut app), own, "back to its own colour");
    }

    fn shortcut_mode() -> App {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.which_key = false;
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app
    }

    fn tap(app: &mut App, c: char) {
        app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    #[test]
    fn a_view_opened_from_shortcut_mode_takes_the_keys_and_closing_it_brings_the_mode_back() {
        let (mut app, mut rx) = super::check_settings_tests::app_with_channel();
        app.cfg.ui.which_key = false;
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        tap(&mut app, 'f');
        assert!(matches!(app.view, Some(View::Files(_))), "f opened Files");
        assert!(matches!(app.mode, Mode::Normal), "Files has the keys, not shortcut mode: {:?}", std::mem::discriminant(&app.mode));
        while rx.try_recv().is_ok() {}
        // k moves in Files; in shortcut mode it would stop the agent (Ctrl+C into its pane).
        tap(&mut app, 'k');
        let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(!sent.iter().any(|m| matches!(m, ClientMsg::Input { .. })), "nothing typed into a pane: {sent:?}");
        assert!(matches!(app.view, Some(View::Files(_))), "still in Files");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(app.view.is_none() && matches!(app.mode, Mode::Prefix { .. }), "Esc closed Files and shortcut mode is back");
    }

    #[test]
    fn shortcut_mode_stays_for_the_next_key() {
        let mut app = shortcut_mode();
        tap(&mut app, 'z');
        assert!(matches!(app.mode, Mode::Prefix { acted: true, .. }), "still in shortcut mode after one");
        tap(&mut app, 'z');
        assert!(matches!(app.mode, Mode::Prefix { .. }), "and after two, with one leader press");
        tap(&mut app, 'u');
        assert!(matches!(app.mode, Mode::Prefix { .. }), "a key with no action does nothing and the mode stays");
    }

    #[test]
    fn esc_or_the_leader_again_goes_back_to_typing() {
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Normal), "Esc");
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        assert!(matches!(app.mode, Mode::Normal), "the leader again");
    }

    #[test]
    fn a_new_pane_ends_shortcut_mode_and_the_next_key_types_into_it() {
        for c in ['n', 's', 'v', 'h', 't'] {
            let mut app = shortcut_mode();
            let (tx, mut rx) = mpsc::unbounded_channel();
            app.out = tx;
            let before = app.focused().unwrap();
            tap(&mut app, c);
            assert_eq!(app.mode, Mode::Normal, "{c} ends shortcut mode");
            assert!(!app.shortcut_held, "{c}: nothing holds the mode to come back");
            assert!(sent(&mut rx).iter().any(|m| matches!(m, Command::NewWorkspace { .. } | Command::NewWorktree { .. })), "{c} started a pane");
            // The daemon focuses the pane it made; another of the fixture's panes stands in.
            let new = app.snap.terms.keys().copied().find(|t| *t != before).unwrap();
            let mut next = app.snap.clone();
            let (ws, tab) = next.locate(new).map(|(w, t)| (w.id, t.id)).unwrap();
            next.active_ws = Some(ws);
            for w in next.workspaces.iter_mut().filter(|w| w.id == ws) {
                w.active_tab = tab;
                for t in w.tabs.iter_mut().filter(|t| t.id == tab) {
                    t.focus = new;
                }
            }
            app.on_server(ServerMsg::State(next));
            assert_eq!(app.focused(), Some(new), "{c}: the new pane has the focus");
            assert_eq!(app.mode, Mode::Normal, "{c}: and the mode stays ended");
            tap(&mut app, 'a');
            let typed: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
            assert!(typed.iter().any(|m| matches!(m, ClientMsg::Input { term, data } if *term == new && data == b"a")), "{c}: the next key went to the new pane: {typed:?}");
        }
    }

    /// Shortcut mode on, as a popup it opened had closed: the loop's `keep_shortcut_mode` runs.
    fn ended_after(app: &mut App, what: &str) {
        app.keep_shortcut_mode();
        assert!(app.mode == Mode::Normal && !app.shortcut_held, "{what} ends shortcut mode: {:?}", std::mem::discriminant(&app.mode));
    }

    #[test]
    fn every_way_into_a_new_pane_ends_shortcut_mode() {
        let mut app = shortcut_mode();
        app.keymap.prefixed.insert("w".parse().unwrap(), Action::SpawnTab("lazygit".into()));
        tap(&mut app, 'w');
        ended_after(&mut app, "spawn-tab");

        let mut app = shortcut_mode();
        let ws = app.active_ws().unwrap().id;
        let entry = WorktreeEntry { path: PathBuf::from("/elsewhere/wt"), branch: "feature".into(), main: false, repo: "shop-api".into() };
        app.mode = Mode::Worktrees { ws, cmd: None, items: Some(vec![entry]), query: String::new(), sel: 0 };
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        ended_after(&mut app, "a row of the worktree picker");

        let mut app = shortcut_mode();
        app.mode = Mode::Worktrees { ws, cmd: None, items: Some(Vec::new()), query: "fresh".into(), sel: 0 };
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        ended_after(&mut app, "the picker's create row");

        let mut app = shortcut_mode();
        app.mode = Mode::Prompt { kind: PromptKind::NewWorkspace, input: String::new() };
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        ended_after(&mut app, "the new pane prompt");

        let mut app = shortcut_mode();
        let t = app.focused().unwrap();
        app.menu_do(menu::Act::Split(t, crate::layout::Dir::Right));
        ended_after(&mut app, "the pane menu's split");
    }

    #[test]
    fn opening_a_project_ends_shortcut_mode_whether_it_starts_a_session_or_not() {
        let mut app = shortcut_mode();
        let running = app.screen_model().iter().find(|p| p.sessions().next().is_some()).map(|p| p.path.clone()).unwrap();
        app.screen_open_project(running);
        ended_after(&mut app, "a project already running");
        let mut app = shortcut_mode();
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        let fresh = std::env::temp_dir().join(format!("{APP_COMMAND}-fresh-project-{}", std::process::id()));
        app.screen_open_project(fresh);
        assert!(sent(&mut rx).iter().any(|c| matches!(c, Command::NewWorkspace { .. })), "a new project starts a shell");
        ended_after(&mut app, "a new project");
    }

    #[test]
    fn the_shell_that_replaces_the_last_closed_session_keeps_shortcut_mode() {
        let mut app = shortcut_mode();
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.screen.last_focus = app.focused();
        let mut gone = app.snap.clone();
        (gone.terms, gone.workspaces, gone.active_ws) = (Default::default(), Vec::new(), None);
        app.on_server(ServerMsg::State(gone));
        assert!(sent(&mut rx).iter().any(|c| matches!(c, Command::NewWorkspace { cmd: None, .. })), "a home shell takes the last session's place");
        assert!(matches!(app.mode, Mode::Prefix { .. }) && app.shortcut_held, "nobody asked for it: shortcut mode stays");
    }

    #[test]
    fn an_action_that_starts_no_pane_keeps_shortcut_mode() {
        let mut app = shortcut_mode();
        tap(&mut app, 'z');
        assert!(matches!(app.mode, Mode::Prefix { .. }) && app.shortcut_held, "z keeps shortcut mode");
    }

    #[test]
    fn a_popup_takes_the_keys_and_closing_it_goes_back_to_shortcut_mode() {
        let mut app = shortcut_mode();
        tap(&mut app, 'i');
        assert!(matches!(app.mode, Mode::GoTo { .. }), "the Inbox has the keys");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Prefix { acted: true, .. }), "closed: shortcut mode is back");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Normal), "a second Esc goes back to typing");
    }

    #[test]
    fn a_click_leaves_shortcut_mode_even_under_a_popup() {
        let mut app = shortcut_mode();
        tap(&mut app, 'i');
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 0, row: 0, modifiers: KeyModifiers::NONE });
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Normal), "after a click, closing the popup goes back to typing");
    }

    /// The mode bar's row on `buf`, and its words.
    fn bar_text(app: &App, buf: &ratatui::buffer::Buffer) -> (Rect, String) {
        let row = screen::mode_bar_row(app, buf.area);
        (row, (row.x..row.right()).map(|x| buf[(x, row.y)].symbol()).collect())
    }

    /// The mode bar's tag unless typing: `SHORTCUT` while lit, else the name of what has the
    /// keys (`SETTINGS`, `FILES`, …), and the bar's ground at its right end, drawn at 160×45.
    fn shortcut_tag(app: &mut App) -> Option<(String, ratatui::style::Color)> {
        let buf = drawn(app, 160, 45);
        let (row, text) = bar_text(app, &buf);
        let tag = text.trim().split("  ").next().unwrap_or_default();
        (tag != "TYPING").then(|| (tag.into(), buf[(row.right() - 1, row.y)].bg))
    }

    #[test]
    fn the_shortcut_pill_shows_after_the_leader_and_stays_lit_after_an_action() {
        let mut app = shortcut_mode();
        let sky = app.theme.sky();
        assert_eq!(shortcut_tag(&mut app), Some(("SHORTCUT".into(), sky)), "after the leader");
        tap(&mut app, 'z');
        assert_eq!(shortcut_tag(&mut app), Some(("SHORTCUT".into(), sky)), "still lit after an action that opens nothing");
    }

    #[test]
    fn the_bar_names_the_popup_that_has_the_keys_and_lights_again_when_it_closes() {
        let mut app = shortcut_mode();
        let sky = app.theme.sky();
        tap(&mut app, 'y');
        assert!(matches!(app.mode, Mode::Why { .. }), "y opened why: {:?}", std::mem::discriminant(&app.mode));
        let (words, ground) = shortcut_tag(&mut app).expect("the bar names the popup");
        assert_eq!(words, "WHY");
        assert_ne!(ground, sky, "dimmed, not sky");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(shortcut_tag(&mut app), Some(("SHORTCUT".into(), sky)), "lit again once it closes");
    }

    #[test]
    fn the_focused_border_is_sky_only_while_shortcut_mode_is_active() {
        let mut app = shortcut_mode();
        tap(&mut app, 'i');
        let mut term = Terminal::new(TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let r = app.pane_frames.first().expect("a pane").1;
        assert_ne!(term.backend().buffer()[(r.x, r.y + 1)].fg, app.theme.sky(), "held under the Inbox: no sky border");
    }

    #[test]
    fn the_bar_names_a_view_shortcut_mode_opened_while_it_has_the_keys() {
        let (mut app, _rx) = super::check_settings_tests::app_with_channel();
        app.cfg.ui.which_key = false;
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        tap(&mut app, 'f');
        assert!(matches!(app.view, Some(View::Files(_))), "f opened Files");
        assert_eq!(shortcut_tag(&mut app).map(|(w, _)| w), Some("FILES".into()));
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(shortcut_tag(&mut app), Some(("SHORTCUT".into(), app.theme.sky())), "Files closed: lit again");
    }

    /// Something other than the panes with the keys, reached as a user would where a key or
    /// action gets there; `held` starts from shortcut mode, else from typing.
    const WHO: [(&str, bool, &str); 34] = [
        ("check page first time", false, "SETTINGS  · type a command · Enter edit · Ctrl+S save · Tab section · Esc close"),
        ("check page no project", false, "SETTINGS  · Tab section · Esc close"),
        ("project check", false, "CHECK SETTINGS  · Ctrl+S save · ↑↓ move · r run now · Esc cancel"),
        ("project check first time", false, "CHECK SETTINGS  · type a command · Enter edit · Ctrl+S save · Esc cancel"),
        ("check page", false, "SETTINGS  · ↑↓ move · Enter edit · Ctrl+S save · r run now · Del remove · Tab section · Esc close"),
        ("check page timeout", false, "SETTINGS  · ↑↓ move · ←→ change · Ctrl+S save · r run now · Del remove · Tab section · Esc close"),
        ("check page typing", false, "SETTINGS  · type a command · Enter done · Ctrl+S save · Tab section · Esc close"),
        ("project check typing", false, "CHECK SETTINGS  · type a command · Enter done · Ctrl+S save · Esc cancel"),
        ("inbox merge", true, "INBOX  · Enter merge · shortcut mode waits · Esc don't"),
        ("inbox throw away", true, "INBOX  · Enter throw away · shortcut mode waits · Esc don't"),
        ("settings capture", false, "SETTINGS  · press the new key · Esc cancel"),
        ("settings edit", false, "SETTINGS  · type · Enter save · Esc cancel"),
        ("copy", false, "COPY  · v select · y copy · / search · Esc leave"),
        ("copy search", false, "COPY  · type to search · Enter find · Esc cancel"),
        ("copy selection", false, "COPY  · y copy · Esc clear"),
        ("key map step", true, "KEY MAP  · a letter picks · ↑↓ choose · Enter run · Backspace back · Esc close"),
        ("folder for a setting", false, "OPEN FOLDER  · type a path · Tab complete · Enter open · Esc back"),
        ("sidebar", true, "SIDEBAR  · ↑↓ move · Enter open · shortcut mode waits · Esc back"),
        ("sidebar", false, "SIDEBAR  · ↑↓ move · Enter open · Esc back"),
        ("settings", true, "SETTINGS  · ↑↓ move · ←→ change · Tab section · shortcut mode waits · Esc close"),
        ("settings", false, "SETTINGS  · ↑↓ move · ←→ change · Tab section · Esc close"),
        ("files", true, "FILES  · / filter · i edit · Enter insert path · shortcut mode waits · Esc close"),
        ("files", false, "FILES  · / filter · i edit · Enter insert path · Esc close"),
        ("inbox", true, "INBOX  · ↑↓ select · Enter open · shortcut mode waits · Esc close"),
        ("changes", true, "CHANGES  · ↑↓ file · c commit · x reviewed · e editor · shortcut mode waits · Esc close"),
        ("changes", false, "CHANGES  · ↑↓ file · c commit · x reviewed · e editor · Esc close"),
        ("branch", true, "BRANCH  · ↑↓ choose · Enter open · shortcut mode waits · Esc close"),
        ("key map search", true, "KEY MAP  · type to search · ↑↓ pick · Enter run · Esc close"),
        ("confirm", true, "CONFIRM  · y yes · shortcut mode waits · n no"),
        ("confirm", false, "CONFIRM  · y yes · n no"),
        ("prompt", true, "NAME  · type · Enter save · shortcut mode waits · Esc cancel"),
        ("prompt", false, "NAME  · type · Enter save · Esc cancel"),
        ("compose", true, "MESSAGE  · Enter send · Shift+Enter new line · shortcut mode waits · Esc keep draft"),
        ("compose", false, "MESSAGE  · Enter send · Shift+Enter new line · Esc keep draft"),
    ];

    /// The project Settings' Check page edits: the active workspace's repository.
    fn settings_project(app: &App) -> PathBuf {
        let ws = app.snap.active().expect("a workspace");
        ws.git.as_ref().map_or_else(|| ws.cwd.clone(), |g| g.root.clone())
    }

    /// `app` with `what` holding the keys, from shortcut mode when `held`.
    fn give_keys(app: &mut App, what: &str, held: bool) {
        app.cfg.ui.which_key = false;
        let leader = |app: &mut App, c: char| {
            app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
            tap(app, c);
        };
        match (what, held) {
            ("sidebar", true) => leader(app, 'e'),
            ("sidebar", false) => app.mode = Mode::Side,
            ("settings", true) => leader(app, ','),
            ("settings", false) => app.act(Action::Settings),
            ("files", true) => leader(app, 'f'),
            ("files", false) => app.act(Action::Files),
            ("inbox", _) => leader(app, 'i'),
            ("changes", _) => {
                if held {
                    leader(app, 'z');
                    app.mode = Mode::Normal;
                }
                app.show_view(View::Changes(Box::new(views::ChangesView {
                    dir: PathBuf::from("/repo"),
                    review: None,
                    error: None,
                    agent: String::new(),
                    said: String::new(),
                    term: None,
                    ws: None,
                    reviewed: Default::default(),
                    linked: false,
                    confirm: None,
                })));
            }
            ("branch", _) => leader(app, 'b'),
            ("key map search", _) => {
                leader(app, 'z');
                app.mode = Mode::KeyMap(Box::new(screen::KeyMap::new(screen::KeyView::Search, None)));
            }
            ("confirm", true) => leader(app, 'x'),
            ("confirm", false) => app.act(Action::ClosePane),
            ("prompt", true) => leader(app, 'r'),
            ("prompt", false) => app.act(Action::RenameTab),
            ("compose", true) => {
                app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
                app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
            }
            ("compose", false) => app.act(Action::Message),
            ("inbox merge" | "inbox throw away", _) => {
                give_keys(app, "inbox", true);
                let term = *app.snap.terms.keys().next().expect("a session");
                app.screen.inbox_confirm = Some((term, what == "inbox merge", false));
            }
            ("settings capture" | "settings edit", _) => {
                app.act(Action::Settings);
                let Mode::ScreenSettings(v) = &mut app.mode else { panic!("Settings is open") };
                if what == "settings capture" {
                    v.capturing = true;
                } else {
                    v.editing = Some(String::new());
                }
            }
            ("copy" | "copy search" | "copy selection", _) => {
                let term = app.focused().expect("a pane");
                assert!(app.enter_copy(term), "copy mode");
                let Mode::Copy(c) = &mut app.mode else { panic!("copy mode") };
                match what {
                    "copy search" => c.input = Some(String::new()),
                    "copy selection" => c.anchor = Some(c.cur),
                    _ => {}
                }
            }
            ("key map step", _) => {
                leader(app, 'z');
                app.mode = Mode::KeyMap(Box::new(screen::KeyMap::new(screen::KeyView::Map, Some(screen::Step::Worktrees))));
            }
            ("folder for a setting", _) => {
                let mut fd = screen::Finder::new(&PathBuf::from("."));
                fd.for_setting = Some("start.folder");
                app.mode = Mode::Finder(Box::new(fd));
            }
            ("check page" | "check page timeout" | "check page typing" | "check page first time" | "check page no project", _) => {
                use crate::client::modal::Cat;
                use screen::check_settings::{CheckDraft, DraftRow};
                let project = settings_project(app);
                if what != "check page first time" {
                    app.cfg.check = vec![crate::config::CheckDef { project: project.clone(), name: None, command: "cargo test".into(), timeout_s: 600 }];
                }
                app.act(Action::Settings);
                let Mode::ScreenSettings(v) = &mut app.mode else { panic!("Settings is open") };
                v.cat = Cat::ALL.iter().position(|c| *c == Cat::Check).expect("a Check page");
                let draft = CheckDraft::new(project);
                app.screen.check_page = match what {
                    "check page timeout" => Some(CheckDraft { row: DraftRow::Timeout, ..draft }),
                    "check page typing" => Some(CheckDraft { editing: true, ..draft }),
                    _ => None,
                };
                if what == "check page no project" {
                    app.snap.active_ws = None;
                }
            }
            ("project check" | "project check first time" | "project check typing", _) => {
                let project = settings_project(app);
                if what == "project check" {
                    app.cfg.check = vec![crate::config::CheckDef { project: project.clone(), name: None, command: "cargo test".into(), timeout_s: 600 }];
                }
                let draft = screen::check_settings::CheckDraft::new(project);
                app.mode = Mode::ProjectCheck(Box::new(screen::check_settings::CheckDraft { editing: what == "project check typing", ..draft }));
            }
            _ => unreachable!("{what}"),
        }
    }

    #[test]
    fn the_bar_says_who_has_the_keys_and_how_to_get_out() {
        for (what, held, want) in WHO {
            let (mut app, _rx) = super::check_settings_tests::app_with_channel();
            give_keys(&mut app, what, held);
            let at = format!("{what} held {held}");
            assert_eq!(app.shortcut_held, held, "{at}: shortcut mode held");
            let buf = drawn(&mut app, 160, 45);
            let (row, bar) = bar_text(&app, &buf);
            show(&format!("{at}: |{bar}|"));
            assert_eq!(bar.trim(), want, "{at}");
            assert!(!bar.contains("paused"), "{at}: {bar}");
            assert_ne!(buf[(row.right() - 1, row.y)].bg, app.theme.sky(), "{at}: sky is for shortcut mode lit");
            let label = on_bar(&app, &buf, want.split("  ").next().unwrap()).expect("the label");
            assert_eq!(buf[(label, row.y)].bg, app.theme.btn, "{at}: the label in a calm chip");
        }
    }

    #[test]
    fn the_key_map_before_a_search_keeps_the_bar_lit() {
        let mut app = shortcut_mode();
        app.mode = Mode::KeyMap(Box::new(screen::KeyMap::new(screen::KeyView::Map, None)));
        let buf = drawn(&mut app, 160, 45);
        let (_, bar) = bar_text(&app, &buf);
        assert!(bar.trim().starts_with("SHORTCUT") && bar.contains("f files"), "its keys still run shortcuts: {bar}");
    }

    /// The bar's words, split: the label, then each hint and the way out.
    fn bar_parts(bar: &str) -> (String, Vec<String>) {
        let mut halves = bar.trim().splitn(2, "  ");
        let label = halves.next().unwrap_or_default().to_string();
        let rest = halves.next().unwrap_or_default().trim().trim_start_matches("· ");
        (label, rest.split(" · ").filter(|s| !s.is_empty()).map(str::to_string).collect())
    }

    #[test]
    fn a_narrow_bar_drops_who_has_the_keys_hints_from_the_right_and_the_way_out_last() {
        for (what, held, want) in WHO {
            let (label, parts) = bar_parts(want);
            let (hints, out) = parts.split_at(parts.len() - 1);
            for w in 40..=160u16 {
                let (mut app, _rx) = super::check_settings_tests::app_with_channel();
                app.update_available = (w % 2 == 0).then(|| "0.44.0".into());
                give_keys(&mut app, what, held);
                let buf = drawn(&mut app, w, 30);
                let (_, bar) = bar_text(&app, &buf);
                let at = format!("{what} held {held} {w}: |{bar}|");
                if [40, 60, 80].contains(&w) {
                    show(&at);
                }
                let bar = bar.replace("↑ Update 0.44.0", "").replace(" ↑ 0.44.0", "").replace(" ↑ ", "");
                let (shown_label, shown) = bar_parts(&bar);
                assert_eq!(shown_label, label, "the label stays: {at}");
                let kept = shown.iter().filter(|s| *s != &out[0]).count();
                assert!(kept <= hints.len() && shown[..kept] == hints[..kept], "hints go from the right: {at}");
                assert!(kept == 0 || shown.last() == Some(&out[0]), "the way out goes after every hint: {at}");
                if w >= 80 {
                    assert!(shown.contains(&out[0]), "from 80 columns the way out always shows: {at}");
                }
            }
        }
    }

    #[test]
    fn a_click_on_the_bar_under_whatever_has_the_keys_does_nothing() {
        for (what, held, _) in WHO {
            let (mut app, _rx) = super::check_settings_tests::app_with_channel();
            give_keys(&mut app, what, held);
            let buf = drawn(&mut app, 160, 45);
            let row = screen::mode_bar_row(&app, buf.area);
            // A click anywhere leaves shortcut mode and the sidebar (as before this bar); what
            // must hold is the popup or view and what was typed in it.
            let keeps = app.mode != Mode::Side;
            let before = (std::mem::discriminant(&app.mode), app.view.as_ref().map(std::mem::discriminant));
            for x in [row.x + 3, row.x + row.width / 2, row.right() - 2] {
                let at = format!("{what} held {held} at {x}");
                let words_only = app.shortcut_held || !matches!(app.mode, Mode::Normal | Mode::Side);
                let hit = app.hits.iter().rev().find(|(r, _)| r.contains(Position::new(x, row.y))).map(|(_, h)| *h);
                if words_only {
                    assert_eq!(hit, Some(Hit::Screen(screen::ScreenHit::Noop)), "{at}: an indicator only");
                } else {
                    assert_eq!(hit, None, "{at}: no click on its words");
                }
                if !keeps {
                    break;
                }
                click_at(&mut app, x, row.y);
                let after = (std::mem::discriminant(&app.mode), app.view.as_ref().map(std::mem::discriminant));
                assert_eq!(after, before, "{at}: the keys stay where they were");
                drawn(&mut app, 160, 45);
            }
        }
    }

    #[test]
    fn the_shortcut_pill_goes_after_esc_the_leader_or_a_click() {
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert_eq!(shortcut_tag(&mut app), None, "Esc");
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        assert_eq!(shortcut_tag(&mut app), None, "the leader again");
        let mut app = shortcut_mode();
        tap(&mut app, 'y');
        shortcut_tag(&mut app);
        let (_, inner) = *app.panes.first().expect("a pane on screen");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: inner.x + 2, row: inner.bottom() - 2, modifiers: KeyModifiers::NONE });
        assert_eq!(shortcut_tag(&mut app), None, "a click, even under a popup");
    }

    /// Background work closing whatever popup is open, with no key pressed.
    fn close_by_itself(app: &mut App) {
        app.on_bg(Bg::Then(Box::new(|app: &mut App| app.mode = Mode::Normal)));
    }

    #[test]
    fn shortcut_mode_returns_when_a_popup_closes_by_itself() {
        let mut app = shortcut_mode();
        tap(&mut app, 'i');
        assert!(matches!(app.mode, Mode::GoTo { .. }), "the Inbox has the keys");
        close_by_itself(&mut app);
        assert!(matches!(app.mode, Mode::Prefix { acted: true, .. }), "back at once: {:?}", std::mem::discriminant(&app.mode));
        assert_eq!(shortcut_tag(&mut app), Some(("SHORTCUT".into(), app.theme.sky())), "and the pill lit");
    }

    /// A letter shortcut mode has no action for.
    fn dead_letter(app: &App) -> char {
        ('a'..='z').chain('A'..='Z').find(|c| !app.keymap.prefixed.contains_key(&KeySpec::from_event(&KeyEvent::new(KeyCode::Char(*c), KeyModifiers::NONE)))).expect("an unbound letter")
    }

    fn notice(app: &App) -> Option<String> {
        app.notice.as_ref().map(|(title, ..)| title.clone())
    }

    #[test]
    fn a_dead_key_in_shortcut_mode_says_how_to_type() {
        let mut app = shortcut_mode();
        let dead = dead_letter(&app);
        tap(&mut app, dead);
        assert!(matches!(app.mode, Mode::Prefix { .. }), "the mode stays");
        assert_eq!(notice(&app).as_deref(), Some("Shortcut mode — Esc to type"));
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("i Shortcut mode — Esc to type"), "the toast is on screen, marked as information: {o}");
        let buf = drawn(&mut app, 160, 45);
        let y = (0..45).find(|&y| (0..160).map(|x| buf[(x, y)].symbol()).collect::<String>().contains("Shortcut mode")).expect("the toast row");
        let x = (0..160).find(|&x| buf[(x, y)].symbol() == "i" && buf[(x + 2, y)].symbol() == "S").expect("its mark");
        assert_eq!(buf[(x, y)].fg, app.theme.sky(), "the info mark is sky, not the ✓ of a success");
        app.notify("worktree created".into(), false);
        assert!(draw(&mut app, 160, 45).contains("✓ worktree created"), "news keeps its ✓");
    }

    #[test]
    fn the_dead_key_hint_shows_once_while_keys_are_mashed() {
        let mut app = shortcut_mode();
        let dead = dead_letter(&app);
        tap(&mut app, dead);
        let first = app.notice.as_ref().map(|(_, at, _)| *at);
        tap(&mut app, dead);
        assert_eq!(app.notice.as_ref().map(|(_, at, _)| *at), first, "a second key doesn't restart it");
        app.notice = None;
        tap(&mut app, dead);
        assert_eq!(notice(&app), None, "nor bring it back soon after");
        app.dead_key_hint = Some(crate::clock::ago(std::time::Duration::from_secs(7)));
        tap(&mut app, dead);
        assert_eq!(notice(&app).as_deref(), Some("Shortcut mode — Esc to type"), "a while later, it says so again");
    }

    #[test]
    fn a_dead_key_does_not_cover_another_toast() {
        let mut app = shortcut_mode();
        app.notify("worktree created".into(), false);
        let dead = dead_letter(&app);
        tap(&mut app, dead);
        assert_eq!(notice(&app).as_deref(), Some("worktree created"));
    }

    #[test]
    fn a_bound_key_in_shortcut_mode_shows_no_hint() {
        let mut app = shortcut_mode();
        tap(&mut app, 'z');
        assert_eq!(notice(&app), None, "z toggled the sidebar quietly");
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let dead = dead_letter(&app);
        tap(&mut app, dead);
        assert_eq!(notice(&app), None, "typing: the key went to the pane");
    }

    #[test]
    fn a_narrow_title_in_shortcut_mode_keeps_the_current_tab_and_drops_the_plus_first() {
        use screen::ScreenHit as S;
        for layout in ["floating", "tiled", "grid"] {
            for h in [24, 30] {
                for (mode, update) in [("lit", false), ("held", false), ("typing", true), ("lit", true), ("held", true)] {
                    let (_, mut app) = super::design_tests::render_with(80, h);
                    (app.cfg.ui.layout, app.cfg.ui.rounded, app.cfg.ui.which_key) = (layout.into(), crate::config::Rounding::Full, false);
                    app.cfg.ui.fold_retired_layout();
                    app.update_available = update.then(|| "0.44.0".into());
                    if mode != "typing" {
                        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
                    }
                    if mode == "held" {
                        tap(&mut app, ',');
                    }
                    let at = format!("{layout} 80x{h} {mode} update {update}");
                    let o = draw(&mut app, 80, h);
                    let row0 = o.lines().next().unwrap_or_default();
                    let current = app.screen.tab;
                    let tabs: Vec<usize> = app.hits.iter().filter_map(|(_, k)| match k { Hit::Screen(S::TabPick(i)) => Some(*i), _ => None }).collect();
                    assert!(tabs.contains(&current), "the current tab is drawn: {at}\n{row0}");
                    let plus = app.hits.iter().any(|(_, k)| *k == Hit::Screen(S::NewTab));
                    assert!(!plus || tabs.len() == app.session_tabs().len(), "no + without every tab before it: {at}\n{row0}");
                }
            }
        }
    }

    #[test]
    fn shortcut_mode_returns_when_a_server_message_closes_a_popup() {
        let mut app = shortcut_mode();
        tap(&mut app, 'i');
        app.mode = Mode::Normal;
        let snap = app.snap.clone();
        app.on_server(ServerMsg::State(snap));
        assert!(matches!(app.mode, Mode::Prefix { acted: true, .. }), "back with the message: {:?}", std::mem::discriminant(&app.mode));
        assert_eq!(shortcut_tag(&mut app), Some(("SHORTCUT".into(), app.theme.sky())), "and the pill lit");
    }

    #[test]
    fn a_popup_closing_by_itself_does_not_bring_back_a_mode_you_left() {
        let mut app = shortcut_mode();
        tap(&mut app, 'i');
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: 0, row: 0, modifiers: KeyModifiers::NONE });
        close_by_itself(&mut app);
        assert_eq!(app.mode, Mode::Normal, "a click left it");
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        close_by_itself(&mut app);
        assert_eq!(app.mode, Mode::Normal, "Esc left it");
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        close_by_itself(&mut app);
        assert_eq!(app.mode, Mode::Normal, "the leader left it");
    }

    /// The cells on the window's outermost ring.
    fn ring(buf: &ratatui::buffer::Buffer) -> Vec<&ratatui::buffer::Cell> {
        let a = buf.area;
        let on_ring = |x: u16, y: u16| x == 0 || y == 0 || x == a.right() - 1 || y == a.bottom() - 1;
        (0..a.height).flat_map(|y| (0..a.width).map(move |x| (x, y))).filter(|&(x, y)| on_ring(x, y)).map(|(x, y)| &buf[(x, y)]).collect()
    }

    /// The window in one look, padding and sidebar, in shortcut mode, at 160×45.
    fn framed_app(layout: &str, padding: &str, side: &str) -> App {
        let mut app = shortcut_mode();
        (app.cfg.ui.layout, app.cfg.ui.padding, app.cfg.ui.motion) = (layout.into(), padding.into(), false);
        app.cfg.ui.fold_retired_layout();
        match side {
            "right" => app.cfg.ui.sidebar_position = "right".into(),
            "collapsed" => app.sidebar = false,
            "hidden" => (app.sidebar, app.side_hidden) = (false, true),
            _ => {}
        }
        app
    }

    const LOOKS: [&str; 3] = ["floating", "tiled", "grid"];
    const SIDES: [&str; 4] = ["left", "right", "collapsed", "hidden"];

    #[test]
    fn no_mode_draws_a_frame_round_the_window() {
        for layout in LOOKS {
            for padding in ["0", "1", "2"] {
                for side in SIDES {
                    for mode in ["lit", "held", "typing"] {
                        let mut app = framed_app(layout, padding, side);
                        match mode {
                            "held" => tap(&mut app, 'y'),
                            "typing" => app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)),
                            _ => {}
                        }
                        let buf = drawn(&mut app, 160, 45);
                        let at = format!("{layout} padding {padding} {side} {mode}");
                        assert!(!ring(&buf).iter().any(|c| ["▀", "▄", "█"].contains(&c.symbol())), "{at}: no solid frame");
                        if padding != "0" {
                            assert!(ring(&buf).iter().all(|c| c.symbol() == " "), "{at}: the padding stays plain");
                        } else if side == "left" {
                            assert_ne!(buf[(0, 22)].fg, app.theme.sky(), "{at}: the sidebar's edge on the window keeps its colour");
                        }
                    }
                }
            }
        }
    }

    fn buf_text(buf: &ratatui::buffer::Buffer) -> String {
        (0..buf.area.height).map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>() + "\n").collect()
    }

    /// `app` put in `mode`: typing, lit (after the leader) or held (why open from it).
    fn into_mode(app: &mut App, mode: &str) {
        app.cfg.ui.which_key = false;
        if mode != "typing" {
            app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        }
        if mode == "held" {
            tap(app, 'y');
        }
    }

    /// What the bar starts with in each mode.
    fn bar_words(mode: &str) -> &'static str {
        match mode {
            "typing" => "TYPING  · Ctrl+Space for shortcuts",
            "lit" => "SHORTCUT",
            _ => "WHY  · shortcut mode waits · Esc close",
        }
    }

    /// Where `words` sits on the mode bar.
    fn on_bar(app: &App, buf: &ratatui::buffer::Buffer, words: &str) -> Option<u16> {
        let row = screen::mode_bar_row(app, buf.area);
        last_at(buf, words, row.y, row.x, row.right() + 1)
    }

    fn click_at(app: &mut App, x: u16, y: u16) {
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: x, row: y, modifiers: KeyModifiers::NONE });
    }

    #[test]
    fn the_mode_bar_reads_at_a_glance_in_every_look() {
        for layout in LOOKS {
            for (w, h) in [(80, 24), (160, 45)] {
                for mode in ["typing", "lit", "held"] {
                    let (_, mut app) = super::design_tests::render_with(w, h);
                    app.cfg.ui.layout = layout.into();
                    app.cfg.ui.fold_retired_layout();
                    into_mode(&mut app, mode);
                    let buf = drawn(&mut app, w, h);
                    let text = buf_text(&buf);
                    show(&format!("===== {layout} {w}x{h} {mode}\n{text}"));
                    let at = format!("{layout} {w}x{h} {mode}");
                    let (row, bar) = bar_text(&app, &buf);
                    assert_eq!(row.y, h - 1, "{at}: the bar is the bottom row (no padding here)");
                    assert!(bar.trim().starts_with(bar_words(mode)), "{at}: {bar}");
                    let sky = buf[(row.right() - 1, row.y)].bg == app.theme.sky();
                    assert_eq!(sky, mode == "lit", "{at}: the whole bar is sky only while lit");
                    let row0: String = (0..w).map(|x| buf[(x, 0)].symbol()).collect();
                    assert!(!row0.contains("TYPING") && !row0.contains("SHORTCUT"), "{at}: no tag in the corner: {row0}");
                    assert!(app.pane_frames.iter().all(|(_, r)| r.bottom() <= row.y), "{at}: every pane ends above the bar");
                }
            }
        }
    }

    #[test]
    fn the_mode_bar_holds_in_every_look_sidebar_padding_rounding_and_size() {
        use crate::config::Rounding;
        let mut failures = Vec::new();
        for layout in LOOKS {
            for side in SIDES {
                for padding in ["0", "1", "2"] {
                    for rounded in [Rounding::Off, Rounding::Corners, Rounding::Full] {
                        for (w, h) in [(80, 24), (120, 36), (200, 50)] {
                            for mode in ["typing", "lit", "held"] {
                                let mut app = framed_app(layout, padding, side);
                                app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                                app.cfg.ui.rounded = rounded;
                                into_mode(&mut app, mode);
                                let buf = drawn(&mut app, w, h);
                                let at = format!("{layout} {side} padding {padding} {rounded:?} {w}x{h} {mode}");
                                let (row, bar) = bar_text(&app, &buf);
                                let pad: u16 = padding.parse().unwrap();
                                if (row.x, row.y, row.width) != (pad, h - 1 - pad, w - 2 * pad) {
                                    failures.push(format!("{at}: the bar at {row:?}"));
                                }
                                let plain = |s: &str| s.replace(screen::CAP_L, " ").replace(screen::CAP_R, " ").split_whitespace().collect::<Vec<_>>().join(" ");
                                if !plain(&bar).starts_with(&plain(bar_words(mode))) {
                                    failures.push(format!("{at}: {bar}"));
                                }
                                let cards = app.pane_frames.iter().map(|(_, r)| *r).chain([app.screen.side_rect, app.screen.sheet_rect]);
                                if cards.into_iter().any(|r| r.intersects(row)) {
                                    failures.push(format!("{at}: a card reaches the bar"));
                                }
                            }
                        }
                    }
                }
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }

    /// The suggestions in the order they are dropped last to first.
    const SUGGESTED: [&str; 9] = ["n shell", "s session", "c changes", "f files", "i inbox", ", settings", "a references", "? all keys", "Esc type"];

    #[test]
    fn the_lit_bar_suggests_keys_from_the_keymap() {
        let mut app = shortcut_mode();
        let buf = drawn(&mut app, 160, 45);
        let (_, bar) = bar_text(&app, &buf);
        let at: Vec<usize> = SUGGESTED.iter().map(|s| bar.find(s).unwrap_or_else(|| panic!("{s}: {bar}"))).collect();
        assert!(at.is_sorted(), "in order: {bar}");
        app.cfg.keys.prefix.insert("f".into(), "none".into());
        app.cfg.keys.prefix.insert("w".into(), "files".into());
        app.keymap = app.cfg.keymap();
        let buf = drawn(&mut app, 160, 45);
        let (_, bar) = bar_text(&app, &buf);
        assert!(bar.contains("w files") && !bar.contains("f files"), "the rebound key: {bar}");
        let key = on_bar(&app, &buf, "w files").expect("w files");
        let y = screen::mode_bar_row(&app, buf.area).y;
        let bold = |x: u16| buf[(x, y)].modifier.contains(ratatui::style::Modifier::BOLD);
        assert!(bold(key) && !bold(key + 2), "the key in bold, its label plain");
        app.cfg.keys.prefix.insert("w".into(), "none".into());
        app.keymap = app.cfg.keymap();
        let buf = drawn(&mut app, 160, 45);
        let (_, bar) = bar_text(&app, &buf);
        assert!(!bar.contains("files"), "an unbound action is not suggested: {bar}");
    }

    #[test]
    fn a_narrow_bar_drops_suggestions_from_the_right_and_all_keys_and_esc_last() {
        for update in [false, true] {
            for w in 40..=200u16 {
                let (_, mut app) = super::design_tests::render_with(w, 24);
                app.update_available = update.then(|| "0.44.0".into());
                into_mode(&mut app, "lit");
                let buf = drawn(&mut app, w, 24);
                let (_, bar) = bar_text(&app, &buf);
                let at = format!("{w} update {update}: {bar}");
                let shown: Vec<bool> = SUGGESTED.iter().map(|s| bar.contains(s)).collect();
                let (suggestions, tail) = shown.split_at(7);
                assert!(bar.contains("SHORTCUT"), "the tag stays: {at}");
                assert!(suggestions.windows(2).all(|p| p[0] || !p[1]), "dropped from the right: {at}");
                assert!(!suggestions[0] || tail.iter().all(|s| *s), "all keys and Esc go after every suggestion: {at}");
                assert!(!tail[0] || tail[1], "Esc goes last: {at}");
                if w >= 80 {
                    assert!(tail.iter().all(|s| *s), "from 80 columns, all keys and Esc always show: {at}");
                }
                if w >= 120 && update {
                    assert!(bar.trim_end().ends_with("↑ Update 0.44.0"), "Update at the right end: {at}");
                }
            }
            let (_, mut app) = super::design_tests::render_with(200, 24);
            into_mode(&mut app, "lit");
            let buf = drawn(&mut app, 200, 24);
            let (_, bar) = bar_text(&app, &buf);
            assert!(SUGGESTED.iter().all(|s| bar.contains(s)), "wide, every suggestion: {bar}");
        }
    }

    #[test]
    fn a_click_on_a_suggestion_presses_its_key_in_shortcut_mode() {
        let mut app = shortcut_mode();
        let buf = drawn(&mut app, 160, 45);
        let y = screen::mode_bar_row(&app, buf.area).y;
        let files = on_bar(&app, &buf, "f files").expect("f files");
        click_at(&mut app, files + 2, y);
        assert!(matches!(app.view, Some(View::Files(_))) && screen::shortcut_waits(&app), "f opened Files and the mode is held");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Prefix { .. }), "Files closed: lit again");
        let buf = drawn(&mut app, 160, 45);
        let shell = on_bar(&app, &buf, "n shell").expect("n shell");
        click_at(&mut app, shell, y);
        assert_eq!(app.mode, Mode::Normal, "a new shell is typed into: the mode ends");
        let mut app = shortcut_mode();
        let buf = drawn(&mut app, 160, 45);
        let all = on_bar(&app, &buf, "? all keys").expect("? all keys");
        click_at(&mut app, all + 3, y);
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::All), "? opens All keys: {:?}", std::mem::discriminant(&app.mode));
        let mut app = shortcut_mode();
        let buf = drawn(&mut app, 160, 45);
        let esc = on_bar(&app, &buf, "Esc type").expect("Esc type");
        click_at(&mut app, esc + 4, y);
        assert_eq!(app.mode, Mode::Normal, "Esc goes back to typing");
        assert_eq!(shortcut_tag(&mut app), None, "and the bar says typing");
    }

    #[test]
    fn a_click_on_typing_starts_shortcut_mode_but_not_over_a_popup() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.which_key = false;
        let buf = drawn(&mut app, 160, 45);
        let y = screen::mode_bar_row(&app, buf.area).y;
        let x = on_bar(&app, &buf, "TYPING").expect("TYPING");
        click_at(&mut app, x, y);
        assert!(matches!(app.mode, Mode::Prefix { .. }), "it starts shortcut mode: {:?}", std::mem::discriminant(&app.mode));
        assert_eq!(shortcut_tag(&mut app).map(|(tag, _)| tag).as_deref(), Some("SHORTCUT"));
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        let buf = drawn(&mut app, 160, 45);
        let x = on_bar(&app, &buf, "for shortcuts").expect("the hint");
        click_at(&mut app, x, y);
        assert!(matches!(app.mode, Mode::Prefix { .. }), "so does its hint");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        app.act(Action::Settings);
        drawn(&mut app, 160, 45);
        assert!(!app.hits.iter().any(|(_, h)| *h == Hit::Screen(screen::ScreenHit::ShortcutMode)), "under Settings, typing is only words");
    }

    #[test]
    fn a_click_on_the_bar_under_a_popup_neither_acts_nor_closes_it() {
        for from in ["typing", "shortcut mode"] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            app.cfg.ui.which_key = false;
            if from == "typing" {
                app.act(Action::Settings);
            } else {
                app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
                tap(&mut app, ',');
            }
            assert!(matches!(app.mode, Mode::ScreenSettings(_)), "{from}: Settings is open");
            let buf = drawn(&mut app, 160, 45);
            let row = screen::mode_bar_row(&app, buf.area);
            for x in [row.x + 3, row.x + row.width / 2, row.right() - 2] {
                click_at(&mut app, x, row.y);
                assert!(matches!(app.mode, Mode::ScreenSettings(_)), "{from}: a click on the bar at {x} keeps Settings open: {:?}", std::mem::discriminant(&app.mode));
                drawn(&mut app, 160, 45);
            }
        }
    }

    #[test]
    fn the_key_map_s_search_keeps_its_query_when_the_bar_is_clicked() {
        let mut app = shortcut_mode();
        app.mode = Mode::KeyMap(Box::new(screen::KeyMap { query: "fil".into(), ..screen::KeyMap::new(screen::KeyView::Search, None) }));
        let buf = drawn(&mut app, 160, 45);
        let (row, bar) = bar_text(&app, &buf);
        assert!(bar.trim().starts_with("KEY MAP  · type to search"), "the search has the keys: {bar}");
        assert!(!app.hits.iter().any(|(r, h)| r.y == row.y && matches!(h, Hit::Screen(screen::ScreenHit::BarKey(_)))), "nothing on the bar to press there");
        let x = on_bar(&app, &buf, "type to search").expect("its hint");
        click_at(&mut app, x, row.y);
        assert!(matches!(&app.mode, Mode::KeyMap(km) if km.query == "fil"), "the search and its text survive: {:?}", std::mem::discriminant(&app.mode));
        assert!(app.view.is_none(), "and Files did not open");
    }

    #[test]
    fn the_bar_takes_no_click_from_a_pane_in_any_look() {
        for layout in LOOKS {
            for mode in ["typing", "lit"] {
                let (_, mut app) = super::design_tests::render_with(160, 45);
                app.cfg.ui.layout = layout.into();
                app.cfg.ui.fold_retired_layout();
                three_panes(&mut app);
                into_mode(&mut app, mode);
                let buf = drawn(&mut app, 160, 45);
                let row = screen::mode_bar_row(&app, buf.area);
                let at = format!("{layout} {mode}");
                for (r, h) in &app.hits {
                    if r.intersects(row) {
                        assert_eq!((r.y, r.height), (row.y, 1), "{at}: only the bar's own hits on its row: {h:?}");
                        assert!(matches!(h, Hit::Screen(screen::ScreenHit::BarKey(_) | screen::ScreenHit::Leader | screen::ScreenHit::ShortcutMode)), "{at}: {h:?}");
                    }
                }
                let lowest = app.panes.iter().map(|(_, r)| r.bottom()).max().unwrap_or(0);
                for (term, inner) in app.panes.clone().into_iter().filter(|(_, r)| r.bottom() == lowest) {
                    let hit = app.hits.iter().rev().find(|(r, _)| r.contains(Position::new(inner.x, inner.bottom() - 1))).map(|(_, h)| *h);
                    assert_eq!(hit, Some(Hit::Pane(term)), "{at}: the last row of a pane above the bar is still the pane's");
                }
            }
        }
    }

    #[test]
    fn every_bar_chip_is_capped_when_rounded_and_its_click_covers_the_caps() {
        use crate::config::Rounding;
        use screen::ScreenHit as S;
        type SetUp = fn(&mut App);
        let states: [(&str, Option<S>, SetUp); 5] = [
            ("TYPING", Some(S::ShortcutMode), |app| app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE))),
            ("SHORTCUT", Some(S::Leader), |_| {}),
            ("INBOX", None, |app| tap(app, 'i')),
            ("↑ Update 9.9.9", Some(S::Update), |app| {
                app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                app.update_available = Some("9.9.9".into());
            }),
            ("↻ Restart", Some(S::Restart), |app| {
                app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                app.installed_version = Some(Ok("999.0.0".into()));
            }),
        ];
        for (words, kind, set_up) in states {
            for rounded in [Rounding::Off, Rounding::Full] {
                let mut app = shortcut_mode();
                app.cfg.ui.rounded = rounded;
                set_up(&mut app);
                let at = format!("{words} {rounded:?}");
                let buf = drawn(&mut app, 160, 45);
                let y = screen::mode_bar_row(&app, buf.area).y;
                let caps = u16::from(rounded == Rounding::Full);
                let n = words.chars().count() as u16;
                let start = on_bar(&app, &buf, words).unwrap_or_else(|| panic!("the chip: {at}"));
                let r = Rect { x: start - 1 - caps, y, width: n + 2 + 2 * caps, height: 1 };
                match kind {
                    Some(kind) => {
                        let click = app.hits.iter().rev().find(|(h, k)| h.contains(Position::new(start, y)) && *k == Hit::Screen(kind)).map(|(h, _)| *h);
                        assert_eq!(click, Some(r), "the click covers the whole chip: {at}");
                    }
                    None => assert!(app.hits.iter().rev().find(|(h, _)| h.contains(Position::new(start, y))).is_some_and(|(_, k)| *k == Hit::Screen(S::Noop)), "an indicator: {at}"),
                }
                let (l, e, inside) = (&buf[(r.x, y)], &buf[(r.right() - 1, y)], buf[(r.x + 1, y)].bg);
                if rounded == Rounding::Full {
                    assert_eq!((l.symbol(), l.fg, e.symbol(), e.fg), (screen::CAP_L, inside, screen::CAP_R, inside), "capped, the click on both caps: {at}");
                    assert_ne!(l.bg, inside, "the caps sit on the bar's ground: {at}");
                } else {
                    assert_eq!((l.symbol(), l.bg, e.symbol(), e.bg), (" ", inside, " ", inside), "square: {at}");
                }
            }
        }
    }

    #[test]
    fn update_sits_at_the_bar_s_right_end_and_clicks_except_while_held() {
        let mut app = shortcut_mode();
        app.update_available = Some("9.9.9".into());
        for mode in ["lit", "held", "typing"] {
            match mode {
                "held" => tap(&mut app, 'y'),
                "typing" => {
                    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                    app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                }
                _ => {}
            }
            let buf = drawn(&mut app, 160, 45);
            let (row, bar) = bar_text(&app, &buf);
            assert!(bar.trim_end().ends_with("↑ Update 9.9.9"), "{mode}: at the right end: {bar}");
            let row0: String = (0..160).map(|x| buf[(x, 0)].symbol()).collect();
            assert!(!row0.contains("Update"), "{mode}: not in the corner any more: {row0}");
            let clickable = app.hits.iter().any(|(r, h)| r.y == row.y && *h == Hit::Screen(screen::ScreenHit::Update));
            assert_eq!(clickable, mode != "held", "{mode}: clickable unless a popup has the keys");
        }
    }

    #[test]
    fn a_click_on_the_bar_under_a_message_keeps_it_and_what_was_typed() {
        let mut app = shortcut_mode();
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(app.mode, Mode::Compose(_)), "Enter opened the message box: {:?}", std::mem::discriminant(&app.mode));
        for c in "hold on".chars() {
            tap(&mut app, c);
        }
        assert_eq!(shortcut_tag(&mut app).map(|(w, _)| w), Some("MESSAGE".into()));
        let buf = drawn(&mut app, 160, 45);
        let x = on_bar(&app, &buf, "MESSAGE").expect("the tag");
        let y = screen::mode_bar_row(&app, buf.area).y;
        click_at(&mut app, x + 2, y);
        assert!(matches!(&app.mode, Mode::Compose(c) if c.text == "hold on"), "the draft survives: {:?}", std::mem::discriminant(&app.mode));
    }

    #[test]
    fn the_top_right_pane_top_holds_still_in_every_mode() {
        for layout in LOOKS {
            let mut closes: Vec<String> = Vec::new();
            for mode in ["lit", "held", "typing"] {
                let (_, mut app) = super::design_tests::render_with(160, 45);
                app.cfg.ui.layout = layout.into();
                app.cfg.ui.fold_retired_layout();
                into_mode(&mut app, mode);
                let buf = drawn(&mut app, 160, 45);
                let top = app.pane_frames.iter().map(|(_, r)| r.y).min().unwrap_or(0);
                let pane = app.pane_frames.iter().map(|(_, r)| *r).filter(|r| r.y == top).max_by_key(|r| r.right()).expect("the top-right pane");
                let row: String = (pane.x..pane.right()).map(|x| buf[(x, pane.y)].symbol()).collect();
                // A lone pane's name, state and ✕ are on its tab: its top says nothing.
                assert!(!row.contains('✕') && !row.contains("needs you") && !row.contains("claude"), "{layout} {mode}: {row}");
                closes.push(row);
            }
            assert!(closes.windows(2).all(|p| p[0] == p[1]), "{layout}: the pane's top holds still: {closes:?}");
        }
    }

    #[test]
    fn the_sheet_s_close_stays_clickable_while_held() {
        let (mut app, _rx) = super::check_settings_tests::app_with_channel();
        app.cfg.ui.which_key = false;
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        tap(&mut app, 'f');
        assert!(matches!(app.view, Some(View::Files(_))), "f opened Files");
        let buf = drawn(&mut app, 160, 45);
        let sheet = app.screen.sheet_rect;
        let close = (sheet.x..sheet.right()).rev().find(|&x| buf[(x, sheet.y)].symbol() == "✕").expect("the sheet's ✕");
        assert!(close + 5 >= sheet.right(), "at the corner");
        click_at(&mut app, close, sheet.y);
        assert!(app.view.is_none(), "a click on it closes Files");
    }

    #[test]
    fn a_right_hand_sidebar_keeps_its_title_and_counts_and_the_bar_its_mode() {
        for (w, h) in [(80, 24), (140, 40), (160, 45)] {
            for release in ["none", "update", "restart"] {
                for mode in ["typing", "lit", "held"] {
                    let (_, mut app) = super::design_tests::render_with(w, h);
                    (app.cfg.ui.sidebar_position, app.cfg.ui.motion) = ("right".into(), false);
                    match release {
                        "update" => app.update_available = Some("0.44.0".into()),
                        "restart" => app.installed_version = Some(Ok("999.0.0".into())),
                        _ => {}
                    }
                    into_mode(&mut app, mode);
                    let buf = drawn(&mut app, w, h);
                    let at = format!("{w}x{h} {release} {mode}");
                    let row: String = (0..w).map(|x| buf[(x, 0)].symbol()).collect();
                    let side = app.screen.side_rect;
                    let title: String = (side.x..side.right()).map(|x| buf[(x, 0)].symbol()).collect();
                    assert!(title.contains("SESSIONS") && title.contains("●1"), "{at}: the sidebar's title and count: {row}");
                    let top = app.pane_frames.iter().map(|(_, r)| r.y).min().unwrap_or(0);
                    let pane = app.pane_frames.iter().map(|(_, r)| *r).find(|r| r.y == top).expect("a pane along the top");
                    let tabs: String = (pane.x..pane.right()).map(|x| buf[(x, pane.y.saturating_sub(1))].symbol()).collect();
                    assert!(tabs.contains('●'), "{at}: the session's status on its tab: {row}");
                    let (_, bar) = bar_text(&app, &buf);
                    assert!(bar.trim().starts_with(bar_words(mode)), "{at}: {bar}");
                    let pill = match release { "update" => Some("↑"), "restart" => Some("↻"), _ => None };
                    assert!(pill.is_none_or(|p| bar.contains(p)), "{at}: the release pill on the bar: {bar}");
                }
            }
        }
    }

    #[test]
    fn the_key_map_opens_on_a_pause_only_before_the_first_shortcut() {
        let mut app = shortcut_mode();
        app.cfg.ui.which_key = true;
        app.mode = Mode::Prefix { since: crate::clock::ago(std::time::Duration::from_secs(10)), acted: false };
        tap(&mut app, 'u');
        assert!(matches!(app.mode, Mode::KeyMap(_)), "before any shortcut, a letter with the map up searches it");
        app.mode = Mode::Prefix { since: crate::clock::ago(std::time::Duration::from_secs(10)), acted: true };
        tap(&mut app, 'u');
        assert!(matches!(app.mode, Mode::Prefix { .. }), "after one, the mode just stays");
        assert!(!draw(&mut app, 160, 45).contains("SEARCH"), "and no key map covers the screen");
    }

    #[test]
    fn a_click_into_a_pane_goes_back_to_typing() {
        let mut app = shortcut_mode();
        draw(&mut app, 160, 45);
        let (_, inner) = *app.panes.first().expect("a pane on screen");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: inner.x + 2, row: inner.y + 2, modifiers: KeyModifiers::NONE });
        assert!(matches!(app.mode, Mode::Normal), "{:?}", std::mem::discriminant(&app.mode));
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
            assert!(o.contains(title) && o.contains("Esc close"), "{title}: a {APP_COMMAND} modal (title in its accent chip, Esc to close)");
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
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 1, sel: design::settings_rows(modal::Cat::Appearance).iter().position(|r| matches!(r, design::SRow::Setting(s) if s.path == "theme")).unwrap(), editing: None, capturing: false, scroll: 0 }));
        let keys = draw(&mut app);
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("{splash}\n{settings}\n{keys}");
        }
        assert!(splash.contains(env!("APP_TAGLINE")));
        assert!(splash.contains("Resume where you left off") && splash.contains("New session") && splash.contains("Open a project…"));
        for page in ["General", "Appearance", "Sessions", "Alerts", "Agents", "Keys"] {
            assert!(settings.contains(page), "page {page}");
        }
        assert!(settings.contains("Leader key") && settings.contains("Splash screen"));
        assert!(keys.contains("Theme") && keys.contains("Enter browse"), "the theme control opens the browser");
    }
}

#[cfg(test)]
mod settings_v8_tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use crate::client::modal::Cat;

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let frame: String = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        frame.replace(|c| ('\u{e000}'..='\u{f8ff}').contains(&c), "")
    }

    fn open(app: &mut App, cat: Cat, sel: usize) {
        let cat = Cat::ALL.iter().position(|c| *c == cat).unwrap();
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat, sel, editing: None, capturing: false, scroll: 0 }));
    }

    fn row_of(cat: Cat, label: &str) -> usize {
        design::settings_rows(cat)
            .iter()
            .position(|r| match r {
                design::SRow::Setting(s) => s.label == label,
                design::SRow::Agent(a) => a.label() == label,
                _ => false,
            })
            .unwrap_or_else(|| panic!("{label} is on the page"))
    }

    #[test]
    fn the_tabs_are_general_appearance_sessions_alerts_agents_check_keys() {
        let names: Vec<&str> = Cat::ALL.iter().map(|c| c.label()).collect();
        assert_eq!(names, ["General", "Appearance", "Sessions", "Alerts", "Agents", "Check", "Keys"]);
        let (_, mut app) = super::design_tests::render_with(160, 45);
        open(&mut app, Cat::General, 0);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("Keys") && !o.contains("Team"), "{o}");
    }

    #[test]
    fn an_old_config_with_team_keys_still_opens_settings() {
        let cfg: Config = toml::from_str("[team]\nfamily = \"codex\"\n").unwrap();
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg = cfg;
        open(&mut app, Cat::Agents, 0);
        assert!(!draw(&mut app, 160, 45).contains("TEAM"));
    }

    #[test]
    fn one_layout_setting_names_each_look() {
        let layout = modal::SETTINGS.iter().find(|s| s.path == "ui.layout").expect("a Layout row");
        assert!(matches!(layout.kind, modal::Kind::Choice(c) if c == ["floating", "hairline"]), "{:?}", layout.kind);
        assert!(!modal::SETTINGS.iter().any(|s| s.path == "ui.panes"), "no separate Cards spacing");
    }

    #[test]
    fn rounded_says_it_does_not_apply_in_hairline() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let rounded = |o: &str| o.lines().find(|l| l.contains("Rounded")).expect("the Rounded row").to_string();
        open(&mut app, Cat::Appearance, row_of(Cat::Appearance, "Rounded"));
        assert!(!rounded(&draw(&mut app, 160, 45)).contains("not in Hairline"), "other looks offer it");
        app.cfg.ui.layout = "hairline".into();
        let row = rounded(&draw(&mut app, 160, 45));
        assert!(row.contains("not in Hairline") && !row.contains("corners"), "{row}");
    }

    #[test]
    fn floating_only_settings_say_they_do_not_apply_in_hairline() {
        for label in ["Pane colours", "Gap"] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            let line = |o: &str| o.lines().find(|l| l.contains(label)).unwrap_or_else(|| panic!("the {label} row")).to_string();
            open(&mut app, Cat::Appearance, row_of(Cat::Appearance, label));
            assert!(!line(&draw(&mut app, 160, 45)).contains("not in Hairline"), "{label}: Floating offers it");
            app.cfg.ui.layout = "hairline".into();
            assert!(line(&draw(&mut app, 160, 45)).contains("not in Hairline"), "{label}");
        }
    }

    #[test]
    fn no_label_is_wider_than_the_label_column_and_every_help_fits_two_lines() {
        for s in modal::SETTINGS {
            assert!(s.label.chars().count() <= usize::from(screen::SETTINGS_LABEL_COLS), "{} is too long for the label column", s.label);
            let lines = screen::wrap_two(s.help, 64);
            assert!(lines.len() <= 2 && !lines.iter().any(|l| l.ends_with('…')), "{}: help needs more than two lines: {:?}", s.path, lines);
        }
    }

    #[test]
    fn leader_waits_for_next_key_is_the_renamed_pause_setting_and_off_by_default() {
        let s = modal::SETTINGS.iter().find(|s| s.path == "ui.which_key").unwrap();
        assert_eq!(s.label, "Leader waits for next key");
        assert!(!Config::default().ui.which_key);
        assert!(modal::SETTINGS.iter().all(|s| s.label != "Keys after a pause"));
    }

    #[test]
    fn sessions_page_has_new_session_and_helper_settings() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        open(&mut app, Cat::Sessions, 0);
        let o = draw(&mut app, 160, 45);
        for want in ["NEW SESSIONS", "Agent", "last used", "Where", "new worktree", "this folder", "SIDEBAR", "Tabs in the sidebar", "WORKTREES"] {
            assert!(o.contains(want), "Sessions shows {want}: {o}");
        }
        let rows = design::settings_rows(Cat::Sessions);
        let groups: Vec<&str> = rows.iter().map(design::settings_group).collect();
        assert_eq!(&groups[..4], ["NEW SESSIONS", "NEW SESSIONS", "SIDEBAR", "SIDEBAR"]);
        assert!(!o.contains("+ starts"), "the + chips start shells, so there is nothing to choose: {o}");
    }

    #[test]
    fn agents_page_shows_status_and_the_subagent_toggle_and_set_up_opens_the_list() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        open(&mut app, Cat::Agents, 0);
        let o = draw(&mut app, 160, 45);
        for want in ["AGENT STATUS", "Claude Code", "Codex", "✓ read from the process and screen", "Connect agents…", "HELPERS", "Sessions may start helpers", "Count on the session row", "LIMITS"] {
            assert!(o.contains(want), "Agents shows {want}: {o}");
        }
        // Further down, past the fold at this size.
        assert!(design::settings_rows(Cat::Agents).iter().any(|r| design::settings_group(r) == "AGENTS TALKING TO AGENTS"), "agents talking to agents is on the page");
        app.claude_hooks = Some(true);
        let o = draw(&mut app, 160, 45);
        super::check_settings_tests::show(&o);
        assert!(o.contains("✓ hooks installed"));
        open(&mut app, Cat::Agents, row_of(Cat::Agents, "Codex"));
        let o = draw(&mut app, 160, 45);
        super::check_settings_tests::show(&o);
        assert!(o.contains("Connect agents installs its hooks") && !o.contains("No hooks needed"), "Codex's help is true: {o}");
        open(&mut app, Cat::Agents, row_of(Cat::Agents, "Connect agents…"));
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(!matches!(app.mode, Mode::ScreenSettings(_)), "the Connect agents list opened");
    }

    #[test]
    fn connect_agents_lists_every_status_integration_and_each_one_installs() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.act(Action::AgentStatusSetup);
        let o = draw(&mut app, 160, 45);
        super::check_settings_tests::show(&o);
        let Mode::ScreenMenu(menu) = &app.mode else { panic!("Connect agents opens a menu") };
        let labels: Vec<&str> = menu.items.iter().map(|(label, _)| label.as_str()).collect();
        assert_eq!(labels, [
            "Connect Claude Code", "Connect Codex", "Connect Gemini CLI", "Connect Qwen Code", "Connect OpenCode",
            "Connect Pi", "Connect OMP", "Connect Kilo", "Connect Kimi", "Connect Mastra Code",
        ]);
        for (label, act) in &menu.items {
            assert!(o.contains(label.as_str()), "{label} is on screen: {o}");
            let menu::Act::InstallStatus(agent) = act else { panic!("{label} installs a status integration") };
            assert!(crate::cli::integrations::status_installer(agent).is_ok(), "{label}: the installer accepts {agent}");
        }
    }

    #[test]
    fn the_subagent_toggle_flips_the_setting() {
        let (_, app) = super::design_tests::render_with(160, 45);
        let s = modal::SETTINGS.iter().find(|s| s.path == "ui.subagent_count").unwrap();
        assert_eq!(modal::step(&app.cfg, s, 1).unwrap().as_bool(), Some(false));
        assert!(app.cfg.ui.subagent_count);
    }

    #[test]
    fn the_selected_row_explains_itself_at_the_bottom_wrapped_over_two_lines() {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        open(&mut app, Cat::Agents, row_of(Cat::Agents, "Count on the session row"));
        let o = draw(&mut app, 100, 30);
        let help = modal::SETTINGS.iter().find(|s| s.path == "ui.subagent_count").unwrap().help;
        let lines = screen::wrap_two(help, 88);
        assert!(lines.iter().all(|l| o.contains(l.as_str())), "{lines:?} in {o}");
    }

    #[test]
    fn labels_show_whole_at_80_by_24() {
        let (_, mut app) = super::design_tests::render_with(80, 24);
        for (cat, label) in [(Cat::General, "Alt+arrows move between panes"), (Cat::General, "Leader waits for next key"), (Cat::Agents, "Sessions may start helpers"), (Cat::Sessions, "Bring sessions back after restart"), (Cat::Alerts, "Phone: say what it's doing"), (Cat::Agents, "Count on the session row")] {
            open(&mut app, cat, row_of(cat, label));
            let o = draw(&mut app, 80, 24);
            assert!(o.contains(label), "{label} is whole at 80x24: {o}");
            let help = modal::SETTINGS.iter().find(|s| s.label == label).unwrap().help;
            assert!(screen::wrap_two(help, 68).iter().all(|l| o.contains(l.as_str())), "its help is whole at 80x24: {o}");
        }
    }
}

mod shots {
    //! Not a check: writes the screens the README shows, cell by cell, for the image step.
    //! `<APP>_SHOTS=<dir> cargo test write_the_readme_screens -- --ignored`
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
        let dir = std::path::PathBuf::from(std::env::var(concat!(env!("APP_ENV"), "_SHOTS")).expect(concat!("set ", env!("APP_ENV"), "_SHOTS")));
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

mod check_row_tests {
    use super::*;
    use crate::layout::Node;
    use crate::protocol::{CheckInfo, CheckState};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::layout::Position;
    use ratatui::style::Modifier;

    const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];
    /// (ui.layout, ui.panes): Cards floating and Cards tiled, as older files wrote them.
    const LAYOUTS: [(&str, &str); 2] = [("cards", "floating"), ("cards", "tiled")];
    const NAME: &str = "t-row";
    const COMMAND: &str = "just ci";
    const EDGE_BAR: &str = "▎";

    struct Case {
        state: CheckState,
        exit: Option<i32>,
        glyph: &'static str,
        plain: &'static str,
        words: &'static str,
    }

    /// Each state's glyph (Unicode, ASCII) and the start of its line, short enough for the
    /// 80-column sidebar.
    const CASES: [Case; 8] = [
        Case { state: CheckState::Checking, exit: None, glyph: "◜", plain: "-", words: "checking ci" },
        Case { state: CheckState::Passed, exit: Some(0), glyph: "✓", plain: "+", words: "passed: ci · 14.2s" },
        Case { state: CheckState::Failed, exit: Some(1), glyph: "✕", plain: "x", words: "failed: ci · exit 1" },
        Case { state: CheckState::Overridden, exit: Some(1), glyph: "⊘", plain: "/", words: "failed, overridden" },
        Case { state: CheckState::CouldNotRun, exit: Some(127), glyph: "?", plain: "?", words: "ci could not run" },
        Case { state: CheckState::Stale, exit: Some(0), glyph: "◷", plain: "~", words: "stale pass: ci" },
        Case { state: CheckState::Never, exit: None, glyph: "?", plain: "?", words: "not checked: never" },
        Case { state: CheckState::NeedsApproval, exit: None, glyph: "●", plain: "!", words: "approve check: just" },
    ];

    fn now() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    fn info(state: CheckState, exit: Option<i32>) -> CheckInfo {
        CheckInfo { name: "ci".into(), command: COMMAND.into(), state, exit_code: exit, started_at: now() - 180, duration_ms: 14_200, ..Default::default() }
    }

    /// The shop-api scene plus one finished agent per entry of `extra`, each with its check.
    fn scene(w: u16, h: u16, layout: (&str, &str), extra: &[(&str, Status, Option<CheckInfo>)]) -> App {
        let (_, mut app) = super::design_tests::render_with(w, h);
        (app.cfg.ui.layout, app.cfg.ui.panes) = (layout.0.into(), layout.1.into());
        app.cfg.ui.fold_retired_layout();
        for (i, (label, status, check)) in extra.iter().enumerate() {
            let id = 20 + i as TermId;
            let mut t = app.snap.terms[&2].clone();
            (t.id, t.label, t.status, t.checks) = (id, (*label).into(), *status, check.clone().into_iter().collect());
            app.snap.terms.insert(id, t);
            app.snap.workspaces[0].tabs.push(TabInfo { id: 30 + id, name: String::new(), layout: Node::Leaf(id), focus: id });
            app.parsers.insert(id, vt100::Parser::new(20, 80, 0));
        }
        app.screen.tabs.clear();
        app.screen.last_focus = None;
        app.screen_sync();
        app.screen.cursor = None;
        app
    }

    fn one(w: u16, h: u16, layout: (&str, &str), c: &Case) -> App {
        scene(w, h, layout, &[(NAME, Status::Done, Some(info(c.state, c.exit)))])
    }

    fn render(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        term.backend().buffer().clone()
    }

    fn show(buf: &Buffer, title: &str) {
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("--- {title}");
            for y in 0..buf.area.height {
                println!("{}", (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>().trim_end());
            }
        }
    }

    fn side_text(buf: &Buffer, app: &App, y: u16) -> String {
        let r = app.screen.side_rect;
        (r.x..r.right()).map(|x| buf[(x, y)].symbol()).collect()
    }

    fn row_of(buf: &Buffer, app: &App, name: &str) -> u16 {
        (0..buf.area.height).find(|&y| side_text(buf, app, y).contains(name)).unwrap_or_else(|| panic!("no sidebar row named {name}"))
    }

    fn cell_with<'a>(buf: &'a Buffer, app: &App, y: u16, symbol: &str) -> &'a ratatui::buffer::Cell {
        let r = app.screen.side_rect;
        (r.x..r.right()).map(|x| &buf[(x, y)]).find(|c| c.symbol() == symbol).unwrap_or_else(|| panic!("no {symbol} on row {y}"))
    }

    /// The column a row's edge bar is drawn in: the reference's X.
    fn bar_column(app: &App) -> u16 {
        app.screen.side_rect.x + 2
    }

    fn label(layout: (&str, &str), w: u16, h: u16) -> String {
        format!("{}/{} {w}x{h}", layout.0, layout.1)
    }

    /// The home screen's counters line.
    fn away_line(buf: &Buffer) -> String {
        let rows = (0..buf.area.height).map(|y| (0..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>());
        rows.map(|r| r.trim().to_string()).find(|r| r.contains("need you") && r.contains("still working")).expect("the counters line")
    }

    fn splash_line(w: u16, h: u16, layout: (&str, &str), extra: &[(&str, Status, Option<CheckInfo>)]) -> String {
        let mut app = scene(w, h, layout, extra);
        app.splash = true;
        let buf = render(&mut app, w, h);
        show(&buf, &format!("splash {}", label(layout, w, h)));
        away_line(&buf)
    }

    #[test]
    fn the_home_counters_do_not_call_a_checking_or_failed_session_finished() {
        let extra = [
            ("t-failed", Status::Done, Some(info(CheckState::Failed, Some(1)))),
            ("t-running", Status::Done, Some(info(CheckState::Checking, None))),
            ("t-passed", Status::Done, Some(info(CheckState::Passed, Some(0)))),
            ("t-plain", Status::Done, None),
        ];
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let line = splash_line(w, h, layout, &extra);
                let at = format!("{}: {line}", label(layout, w, h));
                assert!(line.contains("2 finished"), "only the passed and the unchecked finished: {at}");
                assert!(line.contains("1 check failed"), "{at}");
                assert!(line.contains("1 need you") && line.contains("1 still working"), "{at}");
                assert!(unicode_width::UnicodeWidthStr::width(line.as_str()) <= w as usize, "{at}");
            }
        }
    }

    #[test]
    fn the_home_counters_count_an_approval_as_needs_you_and_stale_or_never_as_finished() {
        let extra = [
            ("t-approve", Status::Done, Some(info(CheckState::NeedsApproval, None))),
            ("t-stale", Status::Done, Some(info(CheckState::Stale, Some(0)))),
            ("t-never", Status::Done, Some(info(CheckState::Never, None))),
        ];
        for layout in LAYOUTS {
            let line = splash_line(160, 45, layout, &extra);
            assert!(line.contains("2 need you") && line.contains("2 finished") && !line.contains("check"), "{}: {line}", label(layout, 160, 45));
        }
    }

    #[test]
    fn the_lead_in_gives_way_to_a_failed_check_count_when_the_line_is_too_wide() {
        let failed = [("t-failed", Status::Done, Some(info(CheckState::Failed, Some(1))))];
        for layout in LAYOUTS {
            let narrow = splash_line(80, 24, layout, &failed);
            let at = format!("{}: {narrow}", label(layout, 80, 24));
            assert!(!narrow.contains("while you were away"), "{at}");
            assert!(narrow.contains("1 check failed") && narrow.contains("0 finished") && narrow.contains("1 need you"), "{at}");
            assert!(unicode_width::UnicodeWidthStr::width(narrow.as_str()) <= 80, "{at}");
            let wide = splash_line(100, 30, layout, &failed);
            assert!(wide.starts_with("while you were away") && wide.contains("1 check failed"), "{}: {wide}", label(layout, 100, 30));
        }
    }

    #[test]
    fn a_failed_check_count_shows_only_above_zero_and_counts_each() {
        for layout in LAYOUTS {
            let at = label(layout, 80, 24);
            let ok = [("t-passed", Status::Done, Some(info(CheckState::Passed, Some(0)))), ("t-late", Status::Done, Some(info(CheckState::Overridden, Some(1))))];
            let line = splash_line(80, 24, layout, &ok);
            assert!(!line.contains("check") && line.contains("2 finished"), "{at}: {line}");
            let two = [("t-a", Status::Done, Some(info(CheckState::Failed, Some(1)))), ("t-b", Status::Done, Some(info(CheckState::Failed, Some(2))))];
            let line = splash_line(80, 24, layout, &two);
            assert!(line.contains("2 checks failed") && line.contains("0 finished"), "{at}: {line}");
        }
    }

    #[test]
    fn a_project_with_no_check_keeps_its_home_counters_as_they_were() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let line = splash_line(w, h, layout, &[("t-plain", Status::Done, None)]);
                let lead = if w >= 100 { "while you were away   " } else { "" };
                assert_eq!(line, format!("{lead}● 1 need you   ⣻ 1 still working   ● 1 finished"), "{}", label(layout, w, h));
            }
        }
    }

    #[test]
    fn each_state_shows_its_glyph_and_words_in_every_layout_and_size() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                for c in &CASES {
                    let mut app = one(w, h, layout, c);
                    let buf = render(&mut app, w, h);
                    show(&buf, &format!("{} {:?}", label(layout, w, h), c.state));
                    let y = row_of(&buf, &app, NAME);
                    let at = label(layout, w, h);
                    assert!(side_text(&buf, &app, y).contains(c.glyph), "{at} {:?}: glyph {} on {:?}", c.state, c.glyph, side_text(&buf, &app, y));
                    let words = if w < 100 { &c.words[..c.words.len().min(16)] } else { c.words };
                    assert!(side_text(&buf, &app, y + 1).contains(words), "{at} {:?}: {words:?} under the row at every width, got {:?}", c.state, side_text(&buf, &app, y + 1));
                }
            }
        }
    }

    #[test]
    fn glyphs_use_existing_theme_slots_only() {
        let (w, h) = (160, 45);
        for layout in LAYOUTS {
            for c in &CASES {
                let mut app = one(w, h, layout, c);
                let buf = render(&mut app, w, h);
                let t = app.theme.clone();
                let want = match c.state {
                    CheckState::Checking => t.sky(),
                    CheckState::Passed => t.done,
                    CheckState::Failed => t.err,
                    CheckState::Overridden | CheckState::Stale => t.text,
                    CheckState::CouldNotRun | CheckState::Never => t.violet(),
                    CheckState::NeedsApproval => t.blocked,
                    other => unreachable!("not among the cases: {other:?}"),
                };
                let y = row_of(&buf, &app, NAME);
                let cell = cell_with(&buf, &app, y, c.glyph);
                assert_eq!(cell.fg, want, "{} {:?}", label(layout, w, h), c.state);
                if c.state == CheckState::Failed {
                    assert!(cell.modifier.contains(Modifier::BOLD), "failed is bold");
                }
                if matches!(c.state, CheckState::Overridden | CheckState::Stale) {
                    assert_ne!(cell.fg, t.done, "an override or a stale pass is never green");
                }
            }
        }
    }

    #[test]
    fn no_state_gives_an_unselected_row_a_bar() {
        let (w, h) = (160, 45);
        for layout in LAYOUTS {
            for c in &CASES {
                let mut app = one(w, h, layout, c);
                let buf = render(&mut app, w, h);
                let y = row_of(&buf, &app, NAME);
                let bar_x = bar_column(&app);
                let picked = app.focused().is_some_and(|f| app.screen.row_y.get(&f) == Some(&y));
                for line in [y, y + 1] {
                    let cell = &buf[(bar_x, line)];
                    assert_eq!(cell.symbol() == EDGE_BAR, picked, "{} {:?} line {line}: {:?}", label(layout, w, h), c.state, cell.symbol());
                }
            }
        }
    }

    #[test]
    fn failed_and_needs_you_differ_in_glyph_and_word_without_colour() {
        let (w, h) = (160, 45);
        let failed = info(CheckState::Failed, Some(1));
        let mut app = scene(w, h, ("cards", "floating"), &[("t-bad", Status::Done, Some(failed)), ("t-ask", Status::Blocked, None)]);
        let buf = render(&mut app, w, h);
        let (bad, ask) = (row_of(&buf, &app, "t-bad"), row_of(&buf, &app, "t-ask"));
        let (bad_line, ask_line) = (side_text(&buf, &app, bad), side_text(&buf, &app, ask));
        assert!(bad_line.contains('✕') && !bad_line.contains('●'), "{bad_line:?}");
        assert!(ask_line.contains('●') && !ask_line.contains('✕'), "{ask_line:?}");
        assert!(side_text(&buf, &app, bad + 1).contains("failed:"), "failed says so in words");
        assert!(!side_text(&buf, &app, bad + 1).contains("needs you"));
    }

    #[test]
    fn a_check_stays_under_its_own_row_when_sessions_sit_in_different_folders() {
        for layout in LAYOUTS {
            for (w, h) in [(100, 30), (160, 45)] {
                let failed = info(CheckState::Failed, Some(1));
                let mut app = scene(w, h, layout, &[("t-far", Status::Done, Some(failed)), ("t-bare", Status::Done, None)]);
                let far = app.snap.terms.get_mut(&20).unwrap();
                (far.root, far.top, far.cwd) = (None, None, std::path::PathBuf::from("/tmp/elsewhere"));
                app.screen_sync();
                let buf = render(&mut app, w, h);
                show(&buf, &format!("grouped {}", label(layout, w, h)));
                let at = label(layout, w, h);
                let (far_y, bare_y) = (row_of(&buf, &app, "t-far"), row_of(&buf, &app, "t-bare"));
                assert!(side_text(&buf, &app, far_y).contains('✕'), "{at}: glyph on its own row");
                assert!(side_text(&buf, &app, far_y + 1).contains("failed:"), "{at}: its line directly under it");
                let bar_x = bar_column(&app);
                let all: String = (bare_y..bare_y + 2).map(|y| side_text(&buf, &app, y)).collect();
                assert!(!all.contains("failed:") && !all.contains(COMMAND), "{at}: a session with no check has no sub-line: {all:?}");
                assert_ne!(buf[(bar_x, bare_y)].fg, app.theme.err, "{at}: no red bar on a session with no check");
            }
        }
    }

    #[test]
    fn plain_icons_use_the_ascii_check_glyphs() {
        let (w, h) = (160, 45);
        for c in &CASES {
            let mut app = one(w, h, ("cards", "floating"), c);
            (app.cfg.icons.blocked, app.cfg.icons.done, app.cfg.icons.idle) = ("!".into(), "+".into(), "o".into());
            app.cfg.ui.glyphs = "auto".into();
            let buf = render(&mut app, w, h);
            let y = row_of(&buf, &app, NAME);
            let (top, sub) = (side_text(&buf, &app, y), side_text(&buf, &app, y + 1));
            assert!(top.contains(c.plain), "{:?}: {} on {top:?}", c.state, c.plain);
            let unicode = ["◜", "✓", "✕", "⊘"];
            assert!(!unicode.iter().any(|g| top.contains(g) || sub.contains(g)), "{:?}: no Unicode marks left: {top:?} {sub:?}", c.state);
        }
    }

    #[test]
    fn the_collapsed_strip_shows_the_same_glyphs() {
        let (w, h) = (160, 45);
        for layout in LAYOUTS {
            for c in &CASES {
                let mut app = one(w, h, layout, c);
                app.sidebar = false;
                app.side_frac = 0.0;
                let buf = render(&mut app, w, h);
                show(&buf, &format!("rail {} {:?}", label(layout, w, h), c.state));
                let found = (0..w).any(|x| buf[(x, 0)].symbol() == c.glyph);
                assert!(found, "{} {:?}: {} in the strip", label(layout, w, h), c.state, c.glyph);
            }
        }
    }

    #[test]
    fn a_project_without_a_check_renders_as_before() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let mut none = scene(w, h, layout, &[(NAME, Status::Done, None)]);
                let plain = render(&mut none, w, h);
                show(&plain, &format!("no check {}", label(layout, w, h)));
                let all: String = (0..h).map(|y| side_text(&plain, &none, y)).collect();
                assert!(!all.contains(COMMAND) && !all.contains('⊘') && !all.contains('✕'), "{}", label(layout, w, h));
                // A result about a turn that is no longer the last one is not drawn on its row.
                for status in [Status::Working, Status::Blocked] {
                    let failed = Some(info(CheckState::Failed, Some(1)));
                    let mut app = scene(w, h, layout, &[(NAME, status, failed)]);
                    let mut bare = scene(w, h, layout, &[(NAME, status, None)]);
                    let rows = |app: &App, buf: &Buffer| {
                        let y = row_of(buf, app, NAME);
                        (y..y + 2).map(|y| side_text(buf, app, y)).collect::<Vec<_>>()
                    };
                    let (with, without) = (render(&mut app, w, h), render(&mut bare, w, h));
                    assert_eq!(rows(&app, &with), rows(&bare, &without), "{} {status:?}", label(layout, w, h));
                }
            }
        }
    }

    #[test]
    fn a_narrow_sidebar_keeps_the_check_line_and_the_folder_name() {
        let (w, h) = (80, 24);
        for layout in LAYOUTS {
            let mut app = one(w, h, layout, &CASES[2]);
            let buf = render(&mut app, w, h);
            let y = row_of(&buf, &app, NAME);
            assert!(side_text(&buf, &app, y).contains('✕'), "the glyph stays");
            assert!(side_text(&buf, &app, y + 1).contains("failed: ci"), "and so does the result's line");
            assert!(side_text(&buf, &app, row_of(&buf, &app, "shop-api")).contains("shop-api"), "{}: folder name whole", label(layout, w, h));
        }
    }

    #[test]
    fn a_running_check_keeps_the_redraw_tick_going() {
        let mut app = one(160, 45, LAYOUTS[0], &CASES[0]);
        assert!(app.any_checking(), "a check in progress animates its ring");
        for c in &CASES[1..] {
            app = one(160, 45, LAYOUTS[0], c);
            assert!(!app.any_checking(), "{:?} is finished and needs no tick", c.state);
        }
        assert!(!scene(160, 45, LAYOUTS[0], &[(NAME, Status::Done, None)]).any_checking());
    }

    #[test]
    fn a_narrow_check_adds_only_its_result_line() {
        const AFTER: &str = "after-agent";
        for layout in LAYOUTS {
            for (w, h) in [(80, 24), (100, 30)] {
                let at = label(layout, w, h);
                let rows = |check: Option<CheckInfo>| {
                    let mut app = scene(w, h, layout, &[(NAME, Status::Idle, check), (AFTER, Status::Idle, None)]);
                    let buf = render(&mut app, w, h);
                    (row_of(&buf, &app, AFTER), app.screen.row_y.len())
                };
                let (with, without) = (rows(Some(info(CheckState::Failed, Some(1)))), rows(None));
                assert_eq!(with, (without.0 + 1, without.1), "{at}: below 140 columns a finished, seen row is one line, a checked one two");
            }
        }
    }

    #[test]
    fn selected_and_hovered_check_rows_tint_both_lines() {
        let (w, h) = (160, 45);
        for layout in LAYOUTS {
            let mut app = one(w, h, layout, &CASES[2]);
            app.mode = Mode::Side;
            app.screen.cursor = Some(20);
            let buf = render(&mut app, w, h);
            show(&buf, &format!("selected {}", label(layout, w, h)));
            let y = row_of(&buf, &app, NAME);
            let x = app.screen.side_rect.x + 10;
            assert_eq!(buf[(x, y)].bg, buf[(x, y + 1)].bg, "{}: one tint over both lines", label(layout, w, h));
            assert_ne!(buf[(x, y)].bg, buf[(x, y + 3)].bg, "{}: and not the rows below", label(layout, w, h));

            let mut app = one(w, h, layout, &CASES[1]);
            app.hover = Some(Position { x: app.screen.side_rect.x + 10, y });
            let buf = render(&mut app, w, h);
            show(&buf, &format!("hovered {}", label(layout, w, h)));
            assert_eq!(buf[(x, y)].bg, buf[(x, y + 1)].bg, "{}: hover tints both lines", label(layout, w, h));
        }
    }

    #[test]
    fn a_result_names_its_checkpoint_and_an_approval_its_command() {
        let (w, h) = (160, 45);
        let mut long = info(CheckState::Failed, Some(1));
        long.command = "npm run test:unit -- --coverage".into();
        let mut app = scene(w, h, ("cards", "floating"), &[(NAME, Status::Done, Some(long.clone()))]);
        let buf = render(&mut app, w, h);
        let sub = side_text(&buf, &app, row_of(&buf, &app, NAME) + 1);
        assert!(sub.contains("failed: ci · exit 1") && !sub.contains("npm"), "{sub:?}");
        let approve = CheckInfo { state: CheckState::NeedsApproval, exit_code: None, ..long };
        let mut app = scene(w, h, ("cards", "floating"), &[(NAME, Status::Done, Some(approve))]);
        let buf = render(&mut app, w, h);
        let sub = side_text(&buf, &app, row_of(&buf, &app, NAME) + 1);
        assert!(sub.contains("approve check: npm run test") && !sub.contains("coverage"), "the command, cut at the edge: {sub:?}");
    }
}

mod check_why_tests {
    use super::*;
    use crate::protocol::{CheckInfo, CheckState};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];

    fn now() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    }

    fn turn_ended() -> u64 {
        now() - 3_000
    }

    fn check(state: CheckState, exit_code: Option<i32>) -> CheckInfo {
        CheckInfo {
            command: "npm test".into(),
            state,
            exit_code,
            started_at: turn_ended() + 14,
            duration_ms: 14_200,
            after_turn_ended_at: turn_ended(),
            first_failing_line: "1 of 59 tests failed".into(),
            ..Default::default()
        }
    }

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let frame: String = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        // Keycap edges are private-use glyphs a reader doesn't see.
        frame.replace(|c| ('\u{e000}'..='\u{f8ff}').contains(&c), "")
    }

    fn show(s: &str) {
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("{s}");
        }
    }

    type Layout = fn(&mut App);

    fn layouts() -> [(&'static str, Layout); 2] {
        [
            ("cards floating", |a| a.cfg.ui.panes = "floating".into()),
            ("cards tiled", |a| a.cfg.ui.gap = "0".into()),
        ]
    }

    fn app_with(w: u16, h: u16, layout: Layout, check: Option<CheckInfo>) -> App {
        let (_, mut app) = super::design_tests::render_with(w, h);
        layout(&mut app);
        let info = app.snap.terms.get_mut(&1).unwrap();
        info.said = "done".into();
        info.checks = check.into_iter().collect();
        app.mode = Mode::Why { term: 1, side: false };
        app
    }

    #[test]
    fn a_failed_check_reads_as_a_timeline_in_every_layout_at_every_size() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let mut app = app_with(w, h, layout, Some(check(CheckState::Failed, Some(1))));
                let o = draw(&mut app, w, h);
                show(&o);
                let at = format!("{name} {w}x{h}");
                // Flat pads keycaps with spaces where cards use edge glyphs.
                let text = o.split_whitespace().collect::<Vec<_>>().join(" ");
                let ended = format!("{} agent's turn ended · it said \"done\"", crate::clock::local_hms(app.snap.terms[&1].checks[0].after_turn_ended_at));
                for want in [
                    "WHY · claude reads",
                    &ended,
                    concat!(env!("APP_NAME"), " ran npm test"),
                    "✕ exit 1 after 14.2s",
                    "Esc close",
                ] {
                    assert!(text.contains(want), "{at}: missing {want:?} in\n{o}");
                }
                // A narrow popover drops the trailing keys whole rather than clip one.
                let all = ["⏎ evidence", "r re-run", "O override"].iter().filter(|k| text.contains(*k)).count();
                assert!(w < 100 || all == 3, "{at}: every key shows when there is room");
                assert!(!text.contains("O override") || all == 3, "{at}: no half-drawn key");
                assert!(text.contains("⏎ evidence") || !text.contains("r re-run"), "{at}: keys drop from the end");
                assert!(!o.contains("most specific source wins"), "{at}: the check replaces the source rows");
                assert!(o.lines().count() <= h as usize && o.lines().all(|l| l.chars().count() <= w as usize), "{at}: fits the screen");
                app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                assert!(app.mode == Mode::Normal, "{at}: Esc closes it");
            }
        }
    }

    #[test]
    fn the_longest_timeline_line_is_cut_at_the_card_edge_at_80x24() {
        for (name, layout) in layouts() {
            let mut app = app_with(80, 24, layout, Some(check(CheckState::Failed, Some(1))));
            app.snap.terms.get_mut(&1).unwrap().said = "x".repeat(200);
            let o = draw(&mut app, 80, 24);
            show(&o);
            let line = o.lines().find(|l| l.contains("agent's turn ended")).unwrap_or_else(|| panic!("{name}: no turn line in\n{o}"));
            assert!(line.chars().count() <= 80, "{name}: the line stays on screen");
            assert!(!line.contains(&"x".repeat(200)), "{name}: the long quote is cut");
            let (_, after) = line.rsplit_once('x').unwrap();
            assert!(after.chars().all(|c| c == ' ' || "│▏▕▎".contains(c)), "{name}: cut at the popover's edge, nothing after it: {line:?}");
            let hints = o.lines().find(|l| l.contains("Esc")).unwrap_or_else(|| panic!("{name}: Esc hint clipped in\n{o}"));
            let words = hints.split_whitespace().collect::<Vec<_>>().join(" ");
            assert!(words.contains("Esc close"), "{name}: the Esc hint is cut: {hints:?}");
        }
    }

    #[test]
    fn a_check_with_no_start_time_shows_no_finish_time() {
        let mut c = check(CheckState::Failed, Some(1));
        c.started_at = 0;
        let mut app = app_with(100, 30, |_| {}, Some(c));
        let o = draw(&mut app, 100, 30);
        let line = o.lines().find(|l| l.contains("✕ exit 1")).unwrap();
        assert!(!line.contains("ago"), "no time for a run that never recorded its start: {line:?}");
    }

    #[test]
    fn hints_show_only_the_keys_that_apply_to_the_result() {
        let hints = |state, code| {
            let mut app = app_with(100, 30, |_| {}, Some(check(state, code)));
            draw(&mut app, 100, 30)
        };
        let passed = hints(CheckState::Passed, Some(0));
        assert!(passed.contains("✓ exit 0 after 14.2s") && passed.contains("⏎  evidence") && passed.contains("r  re-run"));
        assert!(!passed.contains("O  override"), "only a failure can be overridden");
        let running = hints(CheckState::Checking, None);
        assert!(running.contains(concat!(env!("APP_NAME"), " ran npm test")) && running.contains("checking"));
        assert!(running.contains("⏎  evidence") && !running.contains("r  re-run") && !running.contains("O  override"), "a run can be read, not re-run");
        let missing = hints(CheckState::CouldNotRun, Some(127));
        assert!(missing.contains("? npm test could not run · not found") && missing.contains("r  re-run") && !missing.contains("O  override"));
    }

    #[test]
    fn an_override_reads_as_its_row_says() {
        let mut c = check(CheckState::Overridden, Some(1));
        c.overridden_at = Some(turn_ended() + 60);
        c.override_reason = "flaky".into();
        let mut app = app_with(100, 30, |_| {}, Some(c));
        let o = draw(&mut app, 100, 30);
        show(&o);
        assert!(o.contains("⊘ failed, overridden: npm test"), "{o}");
        assert!(!o.contains("O  override"), "an overridden result is not overridden twice");
    }

    #[test]
    fn a_session_with_no_check_shows_the_popover_as_before() {
        for (name, layout) in layouts() {
            let mut app = app_with(100, 30, layout, None);
            let plain = draw(&mut app, 100, 30);
            assert!(plain.contains("Status uncertain") && plain.contains("most specific source wins"), "{name}");
            assert!(!plain.contains("evidence") && !plain.contains(concat!(env!("APP_NAME"), " ran")), "{name}: no check text anywhere");
            app.snap.terms.get_mut(&1).unwrap().checks = vec![check(CheckState::Failed, Some(1))];
            assert_ne!(draw(&mut app, 100, 30), plain, "{name}: a check changes it");
            app.snap.terms.get_mut(&1).unwrap().checks.clear();
            assert_eq!(draw(&mut app, 100, 30), plain, "{name}: with no check it is the same frame");
        }
    }
}

mod check_sheet_tests {
    use super::*;
    use crate::protocol::{CheckInfo, CheckState, Query, Reply};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;
    use ratatui::style::Color;

    const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];
    const TERM: TermId = 1;
    const SHEET_W: u16 = 72;
    const SHORT_OUTPUT: &str = "✓ renders the footer\n✓ links resolve\n✗ keeps the sticky footer\nError: expected 3 got 2\n";

    fn now() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    }

    fn check(state: CheckState, exit_code: Option<i32>) -> CheckInfo {
        CheckInfo {
            command: "npm test".into(),
            state,
            exit_code,
            started_at: now() - 600,
            duration_ms: 14_200,
            after_turn_ended_at: now() - 620,
            first_failing_line: "Error: expected 3 got 2".into(),
            ..Default::default()
        }
    }

    type Layout = fn(&mut App);

    fn layouts() -> [(&'static str, Layout); 2] {
        [
            ("cards floating", |a| a.cfg.ui.panes = "floating".into()),
            ("cards tiled", |a| a.cfg.ui.gap = "0".into()),
        ]
    }

    /// Session 1 finished, with `check`; its sheet open and its output arrived.
    fn open(w: u16, h: u16, layout: Layout, info: CheckInfo, output: Option<&str>) -> App {
        let (_, mut app) = super::design_tests::render_with(w, h);
        layout(&mut app);
        let term = app.snap.terms.get_mut(&TERM).unwrap();
        (term.status, term.said, term.checks) = (Status::Done, "tests pass".into(), vec![info]);
        app.screen_sync();
        app.open_check_sheet(TERM);
        if let Some(text) = output {
            app.on_server(ServerMsg::Reply(Reply::Text(text.into())));
        }
        app
    }

    fn render(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        term.backend().buffer().clone()
    }

    fn frame(buf: &Buffer) -> String {
        let a = buf.area;
        let text: String = (0..a.height).map(|y| (0..a.width).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        // Keycap edges are private-use glyphs a reader doesn't see.
        text.replace(|c| ('\u{e000}'..='\u{f8ff}').contains(&c), "")
    }

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        let o = frame(&render(app, w, h));
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("{o}");
        }
        o
    }

    /// Flat pads keycaps with spaces where cards use edge glyphs.
    fn words(o: &str) -> String {
        o.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    /// Only the sheet's columns, read top to bottom, so a line it wrapped reads whole and the
    /// sidebar beside it does not interleave.
    fn sheet_words(app: &mut App, w: u16, h: u16) -> String {
        let o = draw(app, w, h);
        let g = screen::grid(app, Rect::new(0, 0, w, h));
        let cols: String = o.lines().map(|l| l.chars().skip(g.sheet.x as usize).take(g.sheet.width as usize).collect::<String>() + "\n").collect();
        words(&cols.replace(['▕', '▏', '▁', '▔', '│', '┃', '▎'], " "))
    }

    /// The colour of `needle` in the sheet: its right-most place on screen, since the sidebar
    /// can show a failing line too.
    fn fg_at(buf: &Buffer, needle: &str) -> Color {
        let a = buf.area;
        let found = (0..a.height).filter_map(|y| {
            let line: String = (0..a.width).map(|x| buf[(x, y)].symbol().to_string()).collect();
            line.rfind(needle).map(|at| (line[..at].chars().count() as u16, y))
        });
        let (x, y) = found.max_by_key(|(x, _)| *x).unwrap_or_else(|| panic!("no {needle:?} on screen in\n{}", frame(buf)));
        buf[(x, y)].fg
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn sent(rx: &mut mpsc::UnboundedReceiver<ClientMsg>) -> Vec<ClientMsg> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    fn long_output(n: usize) -> String {
        (1..=n).map(|i| format!("line {i}\n")).collect()
    }

    #[test]
    fn a_failed_result_shows_its_chip_rows_and_numbered_output_in_every_layout_at_every_size() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let at = format!("{name} {w}x{h}");
                let mut app = open(w, h, layout, check(CheckState::Failed, Some(1)), Some(SHORT_OUTPUT));
                let buf = render(&mut app, w, h);
                let o = draw(&mut app, w, h);
                let text = sheet_words(&mut app, w, h);
                let ran = format!("Ran {} · took 14.2s", crate::clock::local_hms(app.snap.terms[&TERM].checks[0].started_at));
                for want in [
                    "CHECK",
                    "✕ failed",
                    "exit 1",
                    "Command npm test",
                    "Folder",
                    &ran,
                    concat!("Seen by ", env!("APP_NAME"), " ran this itself."),
                    "OUTPUT all 4 lines",
                    "1 ✓ renders the footer",
                    "4 Error: expected 3 got 2",
                    "Re-run r",
                    "Override… O",
                ] {
                    assert!(text.contains(want), "{at}: missing {want:?} in\n{o}");
                }
                assert!(o.lines().count() <= h as usize && o.lines().all(|l| l.chars().count() <= w as usize), "{at}: fits the screen");
                assert_eq!(fg_at(&buf, "✓ renders the footer"), app.theme.done, "{at}: passing lines are done-green");
                assert_eq!(fg_at(&buf, "Error: expected 3 got 2"), app.theme.err, "{at}: error lines are err");
                assert_eq!(fg_at(&buf, "✗ keeps the sticky footer"), app.theme.err, "{at}: ✗ lines are err");
                assert_eq!(fg_at(&buf, "links resolve"), app.theme.done, "{at}: the whole line");
            }
        }
    }

    #[test]
    fn a_long_checkpoint_name_truncates_the_sheet_title_once_in_every_layout_at_every_size() {
        let checkpoint = "echo start; then echo ok; echo more output here; and keep going until the line is far wider than the sheet";
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let at = format!("{name} {w}x{h}");
                let mut info = check(CheckState::Failed, Some(1));
                info.name = checkpoint.into();
                let mut app = open(w, h, layout, info, Some(SHORT_OUTPUT));
                let o = draw(&mut app, w, h);
                let g = screen::grid(&app, Rect::new(0, 0, w, h));
                let header = o
                    .lines()
                    .map(|l| l.chars().skip(g.sheet.x as usize).take(g.sheet.width as usize).collect::<String>())
                    .find(|l| l.contains("CHECK"))
                    .unwrap_or_else(|| panic!("{at}: no title row in\n{o}"));
                let (head, tail) = header.split_once('…').unwrap_or_else(|| panic!("{at}: the title is cut, so it ends in …: {header:?}"));
                assert!(head.contains("· claude › echo start;"), "{at}: the checkpoint's start follows the session: {header:?}");
                assert!(!tail.contains('…'), "{at}: one ellipsis: {header:?}");
                let edge = |c: char| c == ' ' || "─╮┐".contains(c);
                assert!(tail.trim_matches(edge) == "✕", "{at}: only the close follows the ellipsis, got {tail:?} in {header:?}");
            }
        }
    }

    #[test]
    fn the_sheet_keeps_the_sidebars_foot_and_waits_for_output_it_has_not_got() {
        let mut app = open(160, 45, |_| {}, check(CheckState::Failed, Some(1)), None);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("+ New") && o.contains("Menu"), "the foot stays");
        assert!(o.contains("OUTPUT  reading…"), "no output yet:\n{o}");
    }

    #[test]
    fn long_output_shows_the_tail_that_fits_and_f_opens_all_of_it() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let at = format!("{name} {w}x{h}");
                let mut app = open(w, h, layout, check(CheckState::Failed, Some(1)), Some(&long_output(200)));
                let o = draw(&mut app, w, h);
                let text = sheet_words(&mut app, w, h);
                assert!(text.contains("of 200 lines") && text.contains("last ") && text.contains("f full output"), "{at}: the tail header in\n{o}");
                assert!(text.contains("200 line 200"), "{at}: the end is shown, numbered");
                assert!(!text.contains(" 1 line 1 "), "{at}: the start is not");
                press(&mut app, KeyCode::Char('f'));
                let full = draw(&mut app, w, h);
                assert!(words(&full).contains("1 line 1 "), "{at}: f shows from the top:\n{full}");
                assert!(!words(&full).contains("200 line 200"), "{at}: and scrolls");
                assert!(sheet_words(&mut app, w, h).contains("lines 1–"), "{at}: it says which lines");
                press(&mut app, KeyCode::PageDown);
                press(&mut app, KeyCode::Down);
                let moved = draw(&mut app, w, h);
                assert!(!words(&moved).contains(" 1 line 1 "), "{at}: ↓ and PageDown scroll");
                assert!(moved.contains('▲'), "{at}: more above once scrolled");
                press(&mut app, KeyCode::Up);
                press(&mut app, KeyCode::PageUp);
                assert!(sheet_words(&mut app, w, h).contains("1 line 1 "), "{at}: ↑ and PageUp come back");
                for _ in 0..40 {
                    press(&mut app, KeyCode::PageDown);
                }
                assert!(sheet_words(&mut app, w, h).contains("200 line 200"), "{at}: scrolling stops at the end");
                press(&mut app, KeyCode::Char('f'));
                assert!(sheet_words(&mut app, w, h).contains("last "), "{at}: f again is the tail");
            }
        }
    }

    #[test]
    fn an_arrow_on_the_tail_scrolls_from_where_the_tail_starts() {
        let mut app = open(100, 30, |_| {}, check(CheckState::Failed, Some(1)), Some(&long_output(200)));
        draw(&mut app, 100, 30);
        press(&mut app, KeyCode::Up);
        let o = sheet_words(&mut app, 100, 30);
        assert!(o.contains("200 line 200") || o.contains("199 line 199"), "one line up from the tail still shows nearly the end: {o}");
        assert!(!o.contains(" 1 line 1 "));
    }

    #[test]
    fn empty_output_says_so_and_what_passed_it() {
        for (name, layout) in layouts() {
            let mut app = open(100, 30, layout, check(CheckState::Passed, Some(0)), Some(""));
            let text = sheet_words(&mut app, 100, 30);
            assert!(text.contains("✓ passed") && text.contains("exit 0"), "{name}");
            assert!(text.contains("(the command printed nothing)") && text.contains("exit 0 is what passed it."), "{name}: {text}");
            assert!(!text.contains("Override") && !text.contains("lines"), "{name}: a pass has nothing to override");
            let mut failed = open(100, 30, layout, check(CheckState::Failed, Some(2)), Some("\n"));
            let text = sheet_words(&mut failed, 100, 30);
            assert!(text.contains("(no output)") && text.contains("It exited 2 without printing.") && !text.contains("what passed it"), "{name}: {text}");
        }
    }

    #[test]
    fn an_override_shows_its_banner_with_the_failing_output_still_below() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let mut info = check(CheckState::Overridden, Some(1));
                info.overridden_at = Some(now() - 120);
                info.override_reason = "flaky on CI".into();
                let mut app = open(w, h, layout, info, Some(SHORT_OUTPUT));
                let text = sheet_words(&mut app, w, h);
                let by = format!("⊘ Overridden by you {}", crate::clock::local_hm(app.snap.terms[&TERM].checks[0].overridden_at.unwrap()));
                for want in ["⊘ failed · overridden", &by, "reason: flaky on CI", "u undo", "next run replaces this", "Error: expected 3 got 2", "exit 1"] {
                    assert!(text.contains(want), "{name} {w}x{h}: missing {want:?} in {text}");
                }
                assert!(!text.contains("Override… O"), "{name} {w}x{h}: it is not overridden twice");
            }
        }
    }

    #[test]
    fn could_not_run_says_it_is_not_a_test_failure() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let mut info = check(CheckState::CouldNotRun, Some(127));
                info.command = "pnpm test".into();
                let mut app = open(w, h, layout, info, Some("sh: pnpm: not found\n"));
                let text = sheet_words(&mut app, w, h);
                for want in ["? could not run", "exit 127", "command not found", "Not a test failure.", "Fix the command in settings", "sh: pnpm: not found"] {
                    assert!(text.contains(want), "{name} {w}x{h}: missing {want:?} in {text}");
                }
                if w >= 100 {
                    for want in ["Not a test failure. The work hasn't been checked.", "install pnpm, then r."] {
                        assert!(text.contains(want), "{name} {w}x{h}: missing {want:?} in {text}");
                    }
                }
                assert!(!text.contains("✕ failed") && !text.contains("Override… O"), "{name} {w}x{h}");
            }
        }
    }

    #[test]
    fn the_sheet_takes_the_pane_column_below_140_columns_and_is_72_wide_beside() {
        for (name, layout) in layouts() {
            let sheet = |w: u16, h: u16| {
                let mut app = open(w, h, layout, check(CheckState::Failed, Some(1)), Some(SHORT_OUTPUT));
                draw(&mut app, w, h);
                let g = screen::grid(&app, Rect::new(0, 0, w, h));
                (g.sheet.width, g.panes.width)
            };
            let (narrow, left) = sheet(100, 30);
            assert!(narrow > 0 && left == 0, "{name}: at 100 columns the sheet is the pane column");
            assert_eq!(sheet(160, 45).0, SHEET_W, "{name}");
            assert_eq!(sheet(200, 50).0, SHEET_W, "{name}");
            let mut app = open(100, 30, layout, check(CheckState::Failed, Some(1)), Some(SHORT_OUTPUT));
            draw(&mut app, 100, 30);
            press(&mut app, KeyCode::Esc);
            assert!(app.view.is_none(), "{name}: Esc returns");
            let g = screen::grid(&app, Rect::new(0, 0, 100, 30));
            assert!(g.sheet.width == 0 && g.panes.width > 0, "{name}: the panes are back");
        }
    }

    #[test]
    fn a_key_the_sheet_does_not_offer_leaves_it_as_it_is() {
        let mut app = open(160, 45, |_| {}, check(CheckState::Failed, Some(1)), Some(&long_output(100)));
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        draw(&mut app, 160, 45);
        for c in ['u', 'x', 'z'] {
            press(&mut app, KeyCode::Char(c));
            assert!(matches!(app.view, Some(View::Check(_))) && app.mode == Mode::Normal, "{c} leaves the sheet as it is");
        }
        assert!(sent(&mut rx).is_empty(), "and sends nothing");
        press(&mut app, KeyCode::Esc);
        assert!(app.view.is_none());
    }

    #[test]
    fn every_button_and_the_close_are_clickable() {
        let mut info = check(CheckState::Failed, Some(1));
        info.line_count = 1284;
        let mut app = open(160, 45, |_| {}, info, Some(&long_output(100)));
        draw(&mut app, 160, 45);
        let has = |app: &App, h: Hit| app.hits.iter().any(|(_, x)| *x == h);
        for key in ['r', 'O', 'o', 'f'] {
            assert!(has(&app, Hit::Button(Btn::ViewKey(key))), "{key} is a button");
        }
        assert!(has(&app, Hit::Screen(screen::ScreenHit::ViewClose)), "the ✕");
        app.on_button(Hit::Button(Btn::ViewKey('f')), false);
        assert!(sheet_words(&mut app, 160, 45).contains("1185 line 1 "), "clicking full output opens it, numbered as the whole");
        app.on_hy_hit(screen::ScreenHit::ViewClose, false);
        assert!(app.view.is_none());
    }

    #[test]
    fn the_buttons_follow_the_result() {
        let buttons = |state, code| {
            let mut app = open(160, 45, |_| {}, check(state, code), Some(SHORT_OUTPUT));
            draw(&mut app, 160, 45);
            [('r', Btn::ViewKey('r')), ('O', Btn::ViewKey('O')), ('u', Btn::ViewKey('u'))].map(|(_, b)| app.hits.iter().any(|(_, x)| *x == Hit::Button(b)))
        };
        assert_eq!(buttons(CheckState::Failed, Some(1)), [true, true, false]);
        assert_eq!(buttons(CheckState::Overridden, Some(1)), [true, false, true]);
        assert_eq!(buttons(CheckState::CouldNotRun, Some(127)), [true, false, false]);
        assert_eq!(buttons(CheckState::Passed, Some(0)), [true, false, false]);
    }

    #[test]
    fn opening_asks_the_daemon_for_the_output_and_a_new_run_asks_again() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        let term = app.snap.terms.get_mut(&TERM).unwrap();
        (term.status, term.checks) = (Status::Done, vec![check(CheckState::Failed, Some(1))]);
        app.open_check_sheet(TERM);
        let asks = |rx: &mut mpsc::UnboundedReceiver<ClientMsg>| sent(rx).into_iter().filter(|m| matches!(m, ClientMsg::Query(Query::CheckOutput { term: TERM, name: None }))).count();
        assert_eq!(asks(&mut rx), 1, "one query on open");
        app.on_server(ServerMsg::Reply(Reply::Text("first run\n".into())));
        assert!(matches!(&app.view, Some(View::Check(v)) if v.output.as_deref() == Some("first run\n")));
        let mut next = app.snap.clone();
        let again = next.terms.get_mut(&TERM).unwrap().checks.first_mut().unwrap();
        (again.started_at, again.exit_code) = (now(), Some(2));
        app.on_server(ServerMsg::State(next.clone()));
        assert!(matches!(&app.view, Some(View::Check(v)) if v.output.is_none()), "the old output is about an older run");
        assert_eq!(asks(&mut rx), 1, "so it asks again");
        app.on_server(ServerMsg::State(next));
        assert_eq!(asks(&mut rx), 0, "an unchanged snapshot asks nothing");
    }

    #[test]
    fn the_sheet_closes_when_its_check_goes_away() {
        let mut app = open(160, 45, |_| {}, check(CheckState::Failed, Some(1)), Some(SHORT_OUTPUT));
        let mut next = app.snap.clone();
        next.terms.get_mut(&TERM).unwrap().checks.clear();
        app.on_server(ServerMsg::State(next));
        assert!(app.view.is_none());
    }

    fn enter_on_row(state: Option<CheckState>, status: Status) -> App {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let term = app.snap.terms.get_mut(&TERM).unwrap();
        (term.status, term.checks) = (status, state.map(|s| check(s, Some(1))).into_iter().collect());
        app.screen_sync();
        draw(&mut app, 160, 45);
        app.mode = Mode::Side;
        app.screen.cursor = Some(TERM);
        app.screen.cursor_proj = None;
        press(&mut app, KeyCode::Enter);
        app
    }

    #[test]
    fn enter_opens_the_sheet_only_for_a_result_that_needs_a_look() {
        for state in [CheckState::Failed, CheckState::CouldNotRun, CheckState::Overridden] {
            let app = enter_on_row(Some(state), Status::Done);
            assert!(matches!(&app.view, Some(View::Check(v)) if v.term == TERM), "{state:?} opens the sheet");
        }
        for state in [None, Some(CheckState::Passed), Some(CheckState::Checking)] {
            let app = enter_on_row(state, Status::Done);
            assert!(app.view.is_none(), "{state:?} focuses the session as today");
        }
        let working = enter_on_row(Some(CheckState::Failed), Status::Working);
        assert!(working.view.is_none(), "an old result under a working agent is not shown, so not opened");
    }

    #[test]
    fn what_a_command_printed_cannot_restyle_or_move_the_screen() {
        let mut app = open(160, 45, |_| {}, check(CheckState::Failed, Some(1)), Some("\x1b[31mred\x1b[0m text\x1b]0;title\x07\nprogress 10%\rprogress 100%\n\x1b[2J\x07bell\tTab\n"));
        let buf = render(&mut app, 160, 45);
        let o = frame(&buf);
        assert!(!o.contains('\u{1b}') && !o.contains('\u{7}') && !o.contains('\r'), "no control character reaches the screen");
        let text = sheet_words(&mut app, 160, 45);
        assert!(text.contains("1 red text 2") && text.contains("2 progress 100%") && text.contains("3 bell Tab"), "{text}");
        assert!(!text.contains("title") && !text.contains("10%") && !text.contains("[31m") && !text.contains("[2J"), "{text}");
    }
}

mod check_inbox_tests {
    use super::*;
    use crate::layout::Node;
    use crate::protocol::{CheckInfo, CheckState, ClientMsg, Command};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];
    /// (ui.layout, ui.panes): Cards floating and Cards tiled, as older files wrote them.
    const LAYOUTS: [(&str, &str); 2] = [("cards", "floating"), ("cards", "tiled")];
    const COMMAND: &str = "just ci";
    const FAILING_LINE: &str = "FAIL footer.test.ts";
    const QUESTION: &str = "Run npm test -- checkout?";
    const FAILED: &str = "FAILED CHECKS";
    const NOT_RUN: &str = "COULD NOT RUN";
    const FINISHED: &str = "FINISHED";

    type Extra = (&'static str, Status, Option<CheckInfo>);

    fn now() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
    }

    fn info(state: CheckState, exit: Option<i32>) -> CheckInfo {
        let first_failing_line = if state == CheckState::Failed { FAILING_LINE.into() } else { String::new() };
        CheckInfo { command: COMMAND.into(), state, exit_code: exit, started_at: now() - 180, duration_ms: 14_200, first_failing_line, ..Default::default() }
    }

    fn failed() -> Option<CheckInfo> {
        Some(info(CheckState::Failed, Some(1)))
    }

    fn not_run() -> Option<CheckInfo> {
        Some(info(CheckState::CouldNotRun, Some(127)))
    }

    /// The shop-api scene (claude asks) plus one agent per entry of `extra`, from term 20 up.
    fn scene(w: u16, h: u16, layout: (&str, &str), extra: &[Extra]) -> App {
        let (_, mut app) = super::design_tests::render_with(w, h);
        (app.cfg.ui.layout, app.cfg.ui.panes) = (layout.0.into(), layout.1.into());
        app.cfg.ui.fold_retired_layout();
        for (i, (label, status, check)) in extra.iter().enumerate() {
            let id = 20 + i as TermId;
            let mut t = app.snap.terms[&2].clone();
            (t.id, t.label, t.status, t.checks) = (id, (*label).into(), *status, check.clone().into_iter().collect());
            app.snap.terms.insert(id, t);
            app.snap.workspaces[0].tabs.push(TabInfo { id: 30 + id, name: String::new(), layout: Node::Leaf(id), focus: id });
            app.parsers.insert(id, vt100::Parser::new(20, 80, 0));
        }
        app.screen.tabs.clear();
        app.screen.last_focus = None;
        app.screen_sync();
        app.screen.cursor = None;
        app
    }

    fn buffer(app: &mut App, w: u16, h: u16) -> ratatui::buffer::Buffer {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        term.backend().buffer().clone()
    }

    /// One string per screen line, keycap edge glyphs removed, columns from `from` on.
    fn lines(buf: &Buffer, from: u16) -> Vec<String> {
        (0..buf.area.height)
            .map(|y| (from..buf.area.width).map(|x| buf[(x, y)].symbol()).collect::<String>().replace(|c| ('\u{e000}'..='\u{f8ff}').contains(&c), "").trim_end().to_string())
            .collect()
    }

    fn inbox(app: &mut App, w: u16, h: u16) -> Vec<String> {
        app.act(Action::Jump);
        let buf = buffer(app, w, h);
        // The Inbox only: the sidebar beside it lists the same sessions.
        let out = lines(&buf, app.screen.side_rect.right());
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("--- {w}x{h} {}/{}\n{}", app.cfg.ui.layout, app.cfg.ui.panes, out.join("\n"));
        }
        out
    }

    fn at(frame: &[String], needle: &str) -> Option<usize> {
        frame.iter().position(|l| l.contains(needle))
    }

    fn rows(app: &App) -> Vec<screen::GoRow> {
        screen::goto_rows(&app.screen_model(), "", 0)
    }

    fn heads(rows: &[screen::GoRow]) -> Vec<&'static str> {
        rows.iter().filter_map(|r| if let screen::GoRow::Head(h) = r { Some(*h) } else { None }).collect()
    }

    fn label(layout: (&str, &str), w: u16, h: u16) -> String {
        format!("{}/{} {w}x{h}", layout.0, layout.1)
    }

    #[test]
    fn groups_come_in_order_and_a_failed_check_is_never_finished() {
        let extra: [Extra; 7] = [
            ("f-done", Status::Done, failed()),
            ("n-done", Status::Done, not_run()),
            ("passed", Status::Done, Some(info(CheckState::Passed, Some(0)))),
            ("plain", Status::Done, None),
            ("overridden", Status::Done, Some(info(CheckState::Overridden, Some(1)))),
            ("f-seen", Status::Idle, failed()),
            ("n-seen", Status::Idle, not_run()),
        ];
        let app = scene(160, 45, LAYOUTS[0], &extra);
        let rows = rows(&app);
        assert_eq!(heads(&rows), ["NEEDS YOU", FAILED, NOT_RUN, FINISHED]);
        use screen::GoRow::*;
        let section = |head: &str| {
            let from = rows.iter().position(|r| matches!(r, Head(h) if *h == head)).unwrap() + 1;
            rows[from..].iter().take_while(|r| !matches!(r, Head(_))).cloned().collect::<Vec<_>>()
        };
        assert_eq!(section("NEEDS YOU"), [Ask(1)]);
        assert_eq!(section(FAILED), [Failed(20), Failed(25)], "a failure stays listed once it has been looked at");
        assert_eq!(section(NOT_RUN), [NotRun(21)], "unchecked work is listed until it is looked at");
        assert_eq!(section(FINISHED), [Done(22), Done(24), Done(23)], "passed and overridden (checked first), then no check, still just finished");
    }

    /// One row per check state of a Done session: whether it counts as finished, in the Inbox
    /// and on the home screen alike.
    fn finished_table() -> [(&'static str, Option<CheckInfo>, bool); 6] {
        [
            ("no check", None, true),
            ("checking", Some(info(CheckState::Checking, None)), false),
            ("passed", Some(info(CheckState::Passed, Some(0))), true),
            ("failed", failed(), false),
            ("could not run", not_run(), false),
            ("overridden", Some(info(CheckState::Overridden, Some(1))), true),
        ]
    }

    fn home_finished(app: &mut App, w: u16, h: u16) -> usize {
        app.splash = true;
        let buf = buffer(app, w, h);
        let line = lines(&buf, 0).into_iter().find(|l| l.contains("need you") && l.contains("still working")).expect("the counters line");
        let words: Vec<&str> = line.split_whitespace().collect();
        let at = words.iter().position(|w| *w == "finished").expect("a finished count");
        words[at - 1].parse().expect("a number")
    }

    #[test]
    fn finished_means_the_same_in_the_inbox_and_on_the_home_screen() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                for (what, check, finished) in finished_table() {
                    let at_ = format!("{} {what}", label(layout, w, h));
                    let extra: [Extra; 1] = [("sub", Status::Done, check)];
                    let mut app = scene(w, h, layout, &extra);
                    let listed = rows(&app).contains(&screen::GoRow::Done(20));
                    assert_eq!(listed, finished, "{at_}: listed under JUST FINISHED");
                    inbox(&mut app, w, h);
                    let mut home = scene(w, h, layout, &extra);
                    assert_eq!(home_finished(&mut home, w, h), usize::from(finished), "{at_}: counted on the home screen");
                }
            }
        }
    }

    #[test]
    fn a_session_the_agent_is_working_on_or_asking_about_is_not_listed_for_its_old_check() {
        let extra: [Extra; 2] = [("busy", Status::Working, failed()), ("asks", Status::Blocked, failed())];
        let app = scene(160, 45, LAYOUTS[0], &extra);
        let rows = rows(&app);
        assert_eq!(heads(&rows), ["NEEDS YOU"], "the old result is about an older turn, and an empty group is left out");
        assert!(rows.contains(&screen::GoRow::Ask(21)) && !rows.iter().any(|r| matches!(r, screen::GoRow::Failed(_))));
    }

    #[test]
    fn a_project_with_no_check_has_the_inbox_it_always_had() {
        let none: [Extra; 1] = [("plain", Status::Done, None)];
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let at_ = label(layout, w, h);
                let mut app = scene(w, h, layout, &none);
                let base = rows(&app);
                assert_eq!(heads(&base), ["NEEDS YOU", FINISHED], "{at_}");
                let text = inbox(&mut app, w, h).join("\n");
                assert!(!text.contains(FAILED) && !text.contains(NOT_RUN) && !text.contains(COMMAND), "{at_}: no check text\n{text}");
                assert!(text.contains("✓ plain") && text.contains("finished · not seen"), "{at_}: the finished row\n{text}");
                // A result that counts as finished keeps the rows where they were.
                for state in [CheckState::Passed, CheckState::Overridden] {
                    let app = scene(w, h, layout, &[("plain", Status::Done, Some(info(state, Some(0))))]);
                    assert_eq!(rows(&app), base, "{at_} {state:?}: the same rows in the same groups");
                }
            }
        }
    }

    #[test]
    fn a_failed_row_shows_the_line_the_daemon_picked_from_real_output() {
        let line = crate::daemon::first_failing_line("running\nFAIL tests/foo: boom\n");
        let result = CheckInfo { first_failing_line: line.clone(), ..info(CheckState::Failed, Some(1)) };
        let (w, h) = SIZES[1];
        let mut app = scene(w, h, LAYOUTS[0], &[("fix-footer", Status::Done, Some(result))]);
        let frame = inbox(&mut app, w, h);
        let row = at(&frame, "✕ fix-footer").expect("the failed row");
        assert_eq!(line, "FAIL tests/foo: boom");
        assert!(frame[row + 2].contains(&line), "the failing line under the result: {:?}", frame[row + 2]);
    }

    #[test]
    fn every_layout_and_size_shows_the_groups_the_rows_and_the_failing_line() {
        let extra: [Extra; 3] = [("fix-footer", Status::Done, failed()), ("add-pay", Status::Done, not_run()), ("docs", Status::Done, Some(info(CheckState::Passed, Some(0))))];
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let at_ = label(layout, w, h);
                let mut app = scene(w, h, layout, &extra);
                let frame = inbox(&mut app, w, h);
                let text = frame.join("\n");
                assert!(frame.iter().all(|l| l.chars().count() <= w as usize) && frame.len() <= h as usize, "{at_}: fits");
                let (ask, fail) = (at(&frame, "NEEDS YOU"), at(&frame, "FAILED CHECKS  1"));
                assert!(ask.is_some() && fail.is_some() && ask < fail, "{at_}: needs you, then failed checks\n{text}");
                let row = at(&frame, "✕ fix-footer").unwrap_or_else(|| panic!("{at_}: the failed row\n{text}"));
                assert!(frame[row].contains("shop-api") && frame[row].contains("2m"), "{at_}: folder and age on line 1: {:?}", frame[row]);
                assert!(frame[row + 1].contains("failed: just ci · exit 1"), "{at_}: the result on line 2: {:?}", frame[row + 1]);
                assert!(frame[row + 2].contains(FAILING_LINE), "{at_}: the first failing line on line 3: {:?}", frame[row + 2]);
                if h >= 45 {
                    for want in ["COULD NOT RUN  1", "? add-pay", "just ci could not run", "FINISHED  1", "✓ docs"] {
                        assert!(text.contains(want), "{at_}: missing {want:?}\n{text}");
                    }
                    assert!(at(&frame, "COULD NOT RUN").unwrap() > fail.unwrap() && at(&frame, FINISHED).unwrap() > at(&frame, "COULD NOT RUN").unwrap(), "{at_}: group order");
                }
            }
        }
    }

    #[test]
    fn a_could_not_run_row_says_so_without_calling_it_a_failure() {
        let extra: [Extra; 1] = [("add-pay", Status::Done, not_run())];
        for layout in LAYOUTS {
            let mut app = scene(100, 30, layout, &extra);
            let frame = inbox(&mut app, 100, 30);
            let row = at(&frame, "? add-pay").unwrap_or_else(|| panic!("{}: the row\n{}", label(layout, 100, 30), frame.join("\n")));
            assert!(frame[row + 1].contains("just ci could not run: just not found"), "{:?}", frame[row + 1]);
            assert!(!frame.join("\n").contains(FAILED), "unchecked work is not a failed check");
        }
    }

    #[test]
    fn a_question_is_never_pushed_below_a_failed_check() {
        let many: Vec<Extra> = ["a", "b", "c", "d", "e", "f"].iter().map(|n| (*n, Status::Done, failed())).collect();
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let at_ = label(layout, w, h);
                let mut app = scene(w, h, layout, &many);
                let rows = rows(&app);
                let first = |p: fn(&screen::GoRow) -> bool| rows.iter().position(p).unwrap();
                assert!(first(|r| matches!(r, screen::GoRow::Ask(_))) < first(|r| matches!(r, screen::GoRow::Failed(_))), "{at_}: rows");
                let frame = inbox(&mut app, w, h);
                let (q, f) = (at(&frame, QUESTION), at(&frame, FAILED));
                assert!(q.is_some() && f.is_some() && q < f, "{at_}: the question is above the failures\n{}", frame.join("\n"));
            }
        }
    }

    #[test]
    fn the_selected_row_scrolls_into_whole_view_even_when_it_is_the_last_failure() {
        let many: Vec<Extra> = ["a1", "b2", "c3", "d4", "e5", "f6"].iter().map(|n| (*n, Status::Done, failed())).collect();
        for layout in LAYOUTS {
            let mut app = scene(80, 24, layout, &many);
            let last = rows(&app).iter().rposition(|r| matches!(r, screen::GoRow::Failed(_))).unwrap();
            app.act(Action::Jump);
            app.mode = Mode::GoTo { query: String::new(), sel: last };
            let buf = buffer(&mut app, 80, 24);
            let frame = lines(&buf, app.screen.side_rect.right());
            let row = at(&frame, "✕ f6").unwrap_or_else(|| panic!("{}: the last failure\n{}", label(layout, 80, 24), frame.join("\n")));
            assert!(frame[row + 1].contains("failed: just ci") && frame[row + 2].contains(FAILING_LINE), "{}: whole, not cut\n{}", label(layout, 80, 24), frame.join("\n"));
        }
    }

    #[test]
    fn empty_groups_stay_out_and_typing_still_finds_a_failure() {
        for layout in LAYOUTS {
            let at_ = label(layout, 100, 30);
            // Only a failure: nothing waits, nothing else finished.
            let mut app = scene(100, 30, layout, &[("fix-footer", Status::Done, failed())]);
            app.snap.terms.get_mut(&1).unwrap().status = Status::Idle;
            let text = inbox(&mut app, 100, 30).join("\n");
            assert!(text.contains("FAILED CHECKS  1"), "{at_}: the failure's group\n{text}");
            for absent in ["NEEDS YOU", NOT_RUN, FINISHED] {
                assert!(!text.contains(absent), "{at_}: no group without rows: {absent}\n{text}");
            }
            // A failure reached by typing is still found as a session.
            let rows = screen::goto_rows(&app.screen_model(), "fix-footer", 0);
            assert!(matches!(rows.as_slice(), [screen::GoRow::Head("FOUND"), screen::GoRow::Sess(_, 20)]), "{at_}: {rows:?}");
        }
    }

    #[test]
    fn enter_on_a_check_row_opens_its_output_and_o_its_session() {
        for layout in LAYOUTS {
            for (term, state) in [(20, failed()), (20, not_run())] {
                for (key, output) in [(KeyCode::Enter, true), (KeyCode::Char('o'), false)] {
                    let mut app = scene(100, 30, layout, &[("fix-footer", Status::Done, state.clone())]);
                    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
                    app.out = tx;
                    let sel = rows(&app).iter().position(|r| matches!(r, screen::GoRow::Failed(_) | screen::GoRow::NotRun(_))).unwrap();
                    app.act(Action::Jump);
                    app.mode = Mode::GoTo { query: String::new(), sel };
                    app.on_key(KeyEvent::new(key, KeyModifiers::NONE));
                    assert!(app.mode == Mode::Normal, "{}: the Inbox closes", label(layout, 100, 30));
                    let sent: Vec<_> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
                    if output {
                        assert!(matches!(&app.view, Some(View::Check(v)) if v.term == term), "{key:?}: the output");
                    } else {
                        assert!(sent.iter().any(|m| matches!(m, ClientMsg::Command(Command::FocusPane { term: t }) if *t == term)), "{key:?}: {sent:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn up_and_down_land_on_check_rows_and_skip_their_headings() {
        let extra: [Extra; 2] = [("f-one", Status::Done, failed()), ("n-one", Status::Done, not_run())];
        let mut app = scene(100, 30, LAYOUTS[0], &extra);
        app.act(Action::Jump);
        let Mode::GoTo { sel: start, .. } = app.mode.clone() else { panic!("the Inbox") };
        let mut seen = vec![rows(&app)[start].clone()];
        for _ in 0..2 {
            app.on_key(KeyEvent::new(KeyCode::Down, KeyModifiers::NONE));
            let Mode::GoTo { sel, .. } = app.mode.clone() else { panic!("still in the Inbox") };
            seen.push(rows(&app)[sel].clone());
        }
        assert_eq!(seen, [screen::GoRow::Ask(1), screen::GoRow::Failed(20), screen::GoRow::NotRun(21)]);
    }
}

mod check_settings_tests {
    use super::*;
    use crate::config::{CHECK_TIMEOUT_S_DEFAULT, CheckDef};
    use crate::client::screen::check_settings::{CheckEdit, step_timeout};
    use crate::protocol::{CheckInfo, CheckState};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    pub(super) const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];

    pub(super) fn project() -> PathBuf {
        PathBuf::from(if cfg!(windows) { r"C:\code\shop-api" } else { "/code/shop-api" })
    }

    pub(super) fn draw(app: &mut App, w: u16, h: u16) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let frame: String = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        frame.replace(|c| ('\u{e000}'..='\u{f8ff}').contains(&c), "")
    }

    pub(super) fn show(s: &str) {
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("{s}");
        }
    }

    pub(super) type Layout = fn(&mut App);

    pub(super) fn layouts() -> [(&'static str, Layout); 2] {
        [
            ("cards floating", |a| a.cfg.ui.panes = "floating".into()),
            ("cards tiled", |a| a.cfg.ui.gap = "0".into()),
        ]
    }

    fn check_page() -> usize {
        modal::Cat::ALL.iter().position(|c| *c == modal::Cat::Check).unwrap()
    }

    fn open_check_page(app: &mut App, sel: usize) {
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: check_page(), sel, editing: None, capturing: false, scroll: 0 }));
    }

    fn with_check(app: &mut App, command: &str, timeout_s: u64) {
        app.cfg.check = vec![CheckDef { project: project(), name: None, command: command.into(), timeout_s }];
    }

    fn result(state: CheckState, exit_code: Option<i32>) -> CheckInfo {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        CheckInfo { command: "npm test".into(), state, exit_code, started_at: now - 120, duration_ms: 14_200, ..Default::default() }
    }

    pub(super) fn words(o: &str) -> String {
        o.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    #[test]
    fn a_project_with_no_check_says_so_in_every_layout_at_every_size() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let (_, mut app) = super::design_tests::render_with(w, h);
                layout(&mut app);
                open_check_page(&mut app, 0);
                let o = draw(&mut app, w, h);
                show(&o);
                let at = format!("{name} {w}x{h}");
                let text = words(&o);
                for want in ["Check", "Command", "e.g. npm test", "No check set.", "passes.", "working."] {
                    assert!(text.contains(want), "{at}: missing {want:?} in\n{o}");
                }
                assert!(o.lines().count() <= h as usize && o.lines().all(|l| l.chars().count() <= w as usize), "{at}: fits the screen");
            }
        }
    }

    #[test]
    fn a_set_check_shows_command_timeout_where_it_runs_and_the_latest_result() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let (_, mut app) = super::design_tests::render_with(w, h);
                layout(&mut app);
                with_check(&mut app, "npm test", CHECK_TIMEOUT_S_DEFAULT);
                app.snap.terms.get_mut(&1).unwrap().checks = vec![result(CheckState::Failed, Some(1))];
                open_check_page(&mut app, 0);
                let o = draw(&mut app, w, h);
                show(&o);
                let at = format!("{name} {w}x{h}");
                let text = words(&o);
                let ran = crate::clock::local_hm(app.snap.terms[&1].checks[0].started_at);
                for want in ["npm test", "10 min", "after each turn", "failed on", "exit 1", ran.as_str()] {
                    assert!(text.contains(want), "{at}: missing {want:?} in\n{o}");
                }
                assert!(!text.contains("No check set."), "{at}: a set check is not the empty state");
            }
        }
    }

    #[test]
    fn the_status_row_names_each_result_by_shape_and_word() {
        let cases = [
            (None, "never run"),
            (Some(result(CheckState::Passed, Some(0))), "passed"),
            (Some(result(CheckState::Failed, Some(1))), "failed"),
            (Some(result(CheckState::Checking, None)), "checking"),
            (Some(result(CheckState::Overridden, Some(1))), "overridden"),
            (Some(result(CheckState::CouldNotRun, Some(127))), "could not run"),
        ];
        for (info, want) in cases {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            with_check(&mut app, "npm test", CHECK_TIMEOUT_S_DEFAULT);
            app.snap.terms.get_mut(&1).unwrap().checks = info.into_iter().collect();
            open_check_page(&mut app, 0);
            let o = draw(&mut app, 160, 45);
            assert!(words(&o).contains(want), "{want}: in\n{o}");
        }
    }

    #[test]
    fn everything_outside_the_check_page_is_the_same_with_or_without_a_check_set() {
        for (name, layout) in layouts() {
            let frames = |set: bool| {
                let (_, mut app) = super::design_tests::render_with(100, 30);
                layout(&mut app);
                // A one-frame spinner, so two draws a moment apart cannot differ by the clock.
                app.cfg.ui.spinner = vec!["*".into()];
                app.cfg.ui.motion = false;
                if set {
                    with_check(&mut app, "npm test", CHECK_TIMEOUT_S_DEFAULT);
                }
                let normal = draw(&mut app, 100, 30);
                app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat: 0, sel: 0, editing: None, capturing: false, scroll: 0 }));
                (normal, draw(&mut app, 100, 30))
            };
            let (plain, with) = (frames(false), frames(true));
            // The folder's meta names its checkpoint; nothing else changes.
            let rest = |s: &str| s.lines().filter(|l| !l.contains("−  shop-api")).collect::<Vec<_>>().join("\n");
            assert_eq!(rest(&plain.0), rest(&with.0), "{name}: the sidebar ignores a project's check but in its folder's meta");
            // Flat draws keycaps with or without edge glyphs depending on what other tests set, so
            // compare the page's words and not its padding.
            let squash = |s: &str| s.split_whitespace().collect::<String>();
            assert_eq!(squash(&plain.1), squash(&with.1), "{name}: the other pages ignore a project's check");
        }
    }

    #[test]
    fn the_timeout_steps_through_round_values_and_clamps() {
        assert_eq!(step_timeout(600, 1), 900);
        assert_eq!(step_timeout(600, -1), 300);
        assert_eq!(step_timeout(30, -1), 30, "the shortest stays");
        assert_eq!(step_timeout(3600, 1), 3600, "the longest stays");
        assert_eq!(step_timeout(90, 1), 120, "a hand-edited value steps to the next round one");
        assert_eq!(step_timeout(90, -1), 60);
    }

    fn edited(before: &str, edit: CheckEdit) -> (String, crate::config::Config) {
        let mut doc: toml_edit::DocumentMut = before.parse().unwrap();
        edit.apply(&mut doc).unwrap();
        let out = doc.to_string();
        let cfg: crate::config::Config = toml::from_str(&out).unwrap();
        (out, cfg)
    }

    fn set(command: &str, timeout_s: u64) -> CheckEdit {
        CheckEdit::Set { project: project(), name: Some("test".into()), command: command.into(), timeout_s }
    }

    #[test]
    fn saving_a_check_writes_its_table_in_place_and_keeps_the_comments() {
        let before = "# my notes\ntheme = \"nord\"\n\n[ui]\n# my ui notes\nsidebar = true\n";
        let (out, cfg) = edited(before, set("npm test", 600));
        assert!(out.contains("# my notes") && out.contains("# my ui notes"), "{out}");
        assert!(out.contains("[[check]]"), "{out}");
        assert_eq!(cfg.check, vec![CheckDef { project: project(), name: None, command: "npm test".into(), timeout_s: 600 }]);

        let with_note = format!("{out}\n# why this one\n[[check]]\nproject = \"/code/lib\"\ncommand = \"cargo test\"\n");
        let (out, cfg) = edited(&with_note, set("pnpm test", 900));
        assert!(out.contains("# why this one") && out.contains("# my notes"), "{out}");
        assert_eq!(cfg.check.len(), 2, "the same project is updated, not added twice: {out}");
        let mine = cfg.check.iter().find(|c| crate::config::same_path(&c.project, &project())).unwrap();
        assert_eq!((mine.command.as_str(), mine.timeout_s), ("pnpm test", 900));
        assert!(cfg.check.iter().any(|c| c.command == "cargo test"), "the other project is untouched");
    }

    #[test]
    fn removing_a_check_deletes_only_its_table() {
        let two = "# top\n[[check]]\nproject = \"/code/lib\"\ncommand = \"cargo test\"\n\n[[check]]\nproject = \"/code/shop-api\"\ncommand = \"npm test\"\n";
        let two = two.replace("/code/shop-api", &project().to_string_lossy().replace('\\', "\\\\"));
        let remove = |project: PathBuf| CheckEdit::Remove { project, name: "test".into() };
        let (out, cfg) = edited(&two, remove(project()));
        assert!(out.contains("# top"), "{out}");
        assert_eq!(cfg.check.len(), 1);
        assert_eq!(cfg.check[0].command, "cargo test");
        let (out, cfg) = edited(&out, remove(PathBuf::from("/code/lib")));
        assert!(cfg.check.is_empty() && !out.contains("check"), "the last one takes the key with it: {out}");
        let (_, cfg) = edited("theme = \"nord\"\n", remove(project()));
        assert!(cfg.check.is_empty(), "removing what is not there is fine");
    }

    #[test]
    fn a_check_list_written_another_way_is_refused_not_overwritten() {
        let mut doc: toml_edit::DocumentMut = "check = 3\n".parse().unwrap();
        assert!(set("npm test", 600).apply(&mut doc).is_err());
    }

    pub(super) fn app_with_channel() -> (App, mpsc::UnboundedReceiver<ClientMsg>) {
        let (_, mut app) = super::design_tests::render_with(100, 30);
        let (tx, rx) = mpsc::unbounded_channel();
        app.out = tx;
        (app, rx)
    }

    fn draft(app: &App) -> Option<&crate::client::screen::check_settings::CheckDraft> {
        app.screen.check_page.as_ref()
    }

    #[test]
    fn a_failed_save_keeps_the_draft_and_says_why() {
        let (mut app, _rx) = app_with_channel();
        open_check_page(&mut app, 0);
        app.on_key(KeyEvent::new(KeyCode::Char('p'), KeyModifiers::NONE));
        app.check_saved(Err(anyhow::anyhow!("disk full")), "Saved.", true);
        assert!(matches!(&app.mode, Mode::ScreenSettings(_)));
        assert!(draft(&app).is_some_and(|d| d.command.as_deref() == Some("p") && !d.saving), "the draft is still there");
        let (msg, _, error) = app.notice.clone().expect("a toast");
        assert!(error && msg.contains("couldn't save") && msg.contains("disk full"), "{msg}");
        assert!(app.cfg.check.is_empty(), "nothing was applied");
    }

    #[test]
    fn a_saved_check_reaches_the_daemon() {
        let (mut app, mut rx) = app_with_channel();
        open_check_page(&mut app, 0);
        app.on_key(KeyEvent::new(KeyCode::Char('n'), KeyModifiers::NONE));
        let mut cfg = app.cfg.clone();
        cfg.check = vec![CheckDef { project: project(), name: None, command: "npm test".into(), timeout_s: 600 }];
        app.check_saved(Ok(cfg), "Saved. Runs after the next turn ends.", true);
        assert_eq!(app.cfg.check.len(), 1);
        let mut sent = Vec::new();
        while let Ok(m) = rx.try_recv() {
            sent.push(m);
        }
        assert!(sent.iter().any(|m| matches!(m, ClientMsg::Command(Command::ReloadConfig))), "the daemon is told to reload");
        assert!(draft(&app).is_none(), "a saved draft is no longer a draft");
        assert_eq!(app.notice.clone().expect("a toast").0, "Saved. Runs after the next turn ends.");
    }

    #[test]
    fn keys_on_the_check_page_edit_the_draft_and_esc_drops_it() {
        let (mut app, _rx) = app_with_channel();
        with_check(&mut app, "npm test", 600);
        open_check_page(&mut app, 0);
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(draft(&app).is_some_and(|d| d.editing && d.command.as_deref() == Some("npm test")), "Enter edits the command");
        assert!(draw(&mut app, 100, 30).contains("npm test█"), "the draft shows with its cursor");
        app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
        assert!(draft(&app).is_none(), "Esc drops the draft");
        assert_eq!(app.cfg.check[0].command, "npm test");
    }

    #[test]
    fn with_no_check_the_arrows_and_delete_write_nothing() {
        let (mut app, _rx) = app_with_channel();
        open_check_page(&mut app, 0);
        for code in [KeyCode::Down, KeyCode::Right, KeyCode::Delete] {
            app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
        }
        assert!(app.notice.is_none(), "nothing to change, nothing said");
        assert!(draft(&app).is_some_and(|d| !d.saving));
    }

    #[test]
    fn without_a_project_open_the_command_cannot_be_edited() {
        let (mut app, _rx) = app_with_channel();
        app.snap.workspaces.clear();
        app.snap.active_ws = None;
        open_check_page(&mut app, 0);
        app.on_key(KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE));
        assert!(matches!(&app.mode, Mode::ScreenSettings(_)) && draft(&app).is_none());
        let (msg, _, _) = app.notice.clone().expect("a toast");
        assert!(msg.contains("project"), "{msg}");
    }
}

mod check_wiring_tests {
    use super::*;
    use crate::client::menu::{Act, menu_chars};
    use crate::protocol::{CheckInfo, CheckState, ClientMsg, Command, ServerMsg};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];
    const TERM: TermId = 1;
    const COMMAND: &str = "npm test";
    /// Keys a sidebar row already uses; a menu item must not take one (x is Close's).
    const TAKEN: [char; 5] = ['j', 'k', 'a', 'm', 'i'];
    const ALL_STATES: [CheckState; 5] = [CheckState::Checking, CheckState::Passed, CheckState::Failed, CheckState::Overridden, CheckState::CouldNotRun];
    type Rx = mpsc::UnboundedReceiver<ClientMsg>;
    type Layout = fn(&mut App);

    fn layouts() -> [(&'static str, Layout); 2] {
        [
            ("cards floating", |a| a.cfg.ui.panes = "floating".into()),
            ("cards tiled", |a| a.cfg.ui.gap = "0".into()),
        ]
    }

    fn now() -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs()
    }

    fn info(state: CheckState) -> CheckInfo {
        let exit_code = match state {
            CheckState::Checking | CheckState::Pending | CheckState::NeedsApproval | CheckState::Never => None,
            CheckState::Passed | CheckState::Stale => Some(0),
            CheckState::CouldNotRun => Some(127),
            CheckState::Failed | CheckState::Overridden => Some(1),
        };
        let overridden = state == CheckState::Overridden;
        CheckInfo {
            command: COMMAND.into(),
            state,
            exit_code,
            started_at: now() - 180,
            duration_ms: 14_200,
            after_turn_ended_at: now() - 200,
            first_failing_line: "Error: expected 3 got 2".into(),
            overridden_at: overridden.then(|| now() - 30),
            ..Default::default()
        }
    }

    /// The shop-api scene with session 1 finished (or as `status`) and carrying `check`.
    fn rig(w: u16, h: u16, layout: Layout, check: Option<CheckInfo>, status: Status) -> (App, Rx) {
        let (_, mut app) = super::design_tests::render_with(w, h);
        layout(&mut app);
        let (tx, rx) = mpsc::unbounded_channel();
        app.out = tx;
        let t = app.snap.terms.get_mut(&TERM).unwrap();
        (t.status, t.said, t.checks) = (status, "tests pass".into(), check.into_iter().collect());
        app.screen_sync();
        (app, rx)
    }

    fn done(state: CheckState) -> (App, Rx) {
        rig(100, 30, |_| {}, Some(info(state)), Status::Done)
    }

    fn sent(rx: &mut Rx) -> Vec<ClientMsg> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    fn commands(rx: &mut Rx) -> Vec<Command> {
        sent(rx).into_iter().filter_map(|m| if let ClientMsg::Command(c) = m { Some(c) } else { None }).collect()
    }

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let frame: String = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        let frame = frame.replace(|c| ('\u{e000}'..='\u{f8ff}').contains(&c), "");
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("--- {w}x{h} {}/{}\n{frame}", app.cfg.ui.layout, app.cfg.ui.panes);
        }
        frame
    }

    fn words(o: &str) -> String {
        o.split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn fits(o: &str, w: u16, h: u16) -> bool {
        o.lines().count() <= h as usize && o.lines().all(|l| l.chars().count() <= w as usize)
    }

    fn press(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn typed(app: &mut App, text: &str) {
        for c in text.chars() {
            press(app, KeyCode::Char(c));
        }
    }

    /// Click the topmost control whose hit is `want`, drawing first so it exists.
    pub(super) fn click(app: &mut App, w: u16, h: u16, want: Hit) {
        draw(app, w, h);
        let (r, _) = app.hits.iter().rev().find(|(_, hit)| *hit == want).unwrap_or_else(|| panic!("no control for {want:?}"));
        let (column, row) = (r.x, r.y);
        for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
            app.on_mouse(MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE });
        }
    }

    fn viewkey(c: char) -> Hit {
        Hit::Button(Btn::ViewKey(c))
    }

    fn toast(app: &App) -> (String, bool) {
        let (msg, _, error) = app.notice.clone().expect("a toast");
        (msg, error)
    }

    fn rerun_of(cmds: &[Command]) -> usize {
        cmds.iter().filter(|c| matches!(c, Command::RerunCheck { term: TERM, .. })).count()
    }

    fn overrides(cmds: &[Command]) -> Vec<String> {
        cmds.iter().filter_map(|c| if let Command::OverrideCheck { term: TERM, name: None, reason } = c { Some(reason.clone()) } else { None }).collect()
    }

    fn labels(app: &App) -> Vec<String> {
        app.session_items(TERM).unwrap().1.into_iter().map(|(l, _)| l).collect()
    }

    // ---- the row menu ------------------------------------------------------------------

    #[test]
    fn the_row_menu_offers_a_check_item_only_where_it_applies() {
        let offered = |state: Option<CheckState>| {
            let (app, _) = rig(100, 30, |_| {}, state.map(info), Status::Done);
            let l = labels(&app);
            ["Check output", "Re-run check", "Override check…", "Undo override"].map(|want| l.iter().any(|x| x == want))
        };
        assert_eq!(offered(None), [false, false, false, false], "no check, no items");
        assert_eq!(offered(Some(CheckState::Checking)), [true, false, false, false], "running: its output, not a second run");
        assert_eq!(offered(Some(CheckState::Passed)), [true, true, false, false]);
        assert_eq!(offered(Some(CheckState::CouldNotRun)), [true, true, false, false]);
        assert_eq!(offered(Some(CheckState::Overridden)), [true, true, false, true], "an override is not overridden twice, but can be taken back");
        assert_eq!(offered(Some(CheckState::Failed)), [true, true, true, false]);
    }

    #[test]
    fn a_checks_menu_items_keep_their_row_keys_and_rename_moves_over() {
        let keyed = |state: Option<CheckState>| {
            let (app, _) = rig(100, 30, |_| {}, state.map(info), Status::Done);
            let items = app.session_items(TERM).unwrap().1;
            items.iter().map(|(l, _)| l.clone()).zip(menu_chars(&app, &items)).collect::<Vec<_>>()
        };
        let key = |keys: &[(String, Option<char>)], label: &str| keys.iter().find(|(l, _)| l == label).and_then(|(_, k)| *k);
        assert_eq!(key(&keyed(None), "Rename"), Some('r'), "no check: Rename keeps r");
        let failed = keyed(Some(CheckState::Failed));
        assert_eq!(["Check output", "Re-run check", "Override check…"].map(|l| key(&failed, l)), [Some('o'), Some('r'), Some('O')]);
        assert_eq!(key(&failed, "Rename"), Some('e'), "Rename takes its next free letter");
        assert_eq!(key(&keyed(Some(CheckState::Overridden)), "Undo override"), Some('u'));
        let running = keyed(Some(CheckState::Checking));
        assert!(running.iter().all(|(l, k)| l.starts_with("Check") || !matches!(k, Some('r' | 'o' | 'u'))), "reserved even while it runs: {running:?}");
    }

    /// `state`'s session under the sidebar's keyboard cursor.
    fn on_its_row(state: CheckState) -> (App, Rx) {
        let (mut app, rx) = done(state);
        app.mode = Mode::Side;
        app.screen.cursor = Some(TERM);
        (app, rx)
    }

    #[test]
    fn the_sidebar_keys_act_on_the_rows_check_and_say_why_when_they_cant() {
        // No bare letter acts from the sidebar (ADR 0014): r, O and u are typing, so they
        // re-run, override and undo nothing.
        for (state, key) in [(CheckState::Failed, KeyCode::Char('r')), (CheckState::Failed, KeyCode::Char('O')), (CheckState::Overridden, KeyCode::Char('u'))] {
            let (mut app, mut rx) = on_its_row(state);
            app.on_key(KeyEvent::new(key, KeyModifiers::NONE));
            assert!(!matches!(&app.mode, Mode::Confirm(_)), "{key:?} asks nothing: {:?}", app.mode);
            assert!(commands(&mut rx).is_empty(), "{key:?} sends no command");
        }
        for state in [CheckState::Failed, CheckState::CouldNotRun, CheckState::Overridden, CheckState::Stale, CheckState::Never] {
            let (mut app, _) = on_its_row(state);
            press(&mut app, KeyCode::Enter);
            assert!(matches!(&app.view, Some(View::Check(v)) if v.term == TERM), "{state:?}: Enter opens its evidence");
        }
        let (mut app, _) = on_its_row(CheckState::Passed);
        press(&mut app, KeyCode::Enter);
        assert!(app.view.is_none(), "a pass is not worth a look: Enter goes to the session");
    }

    #[test]
    fn the_menu_tells_override_from_output_by_case() {
        let (mut app, mut rx) = done(CheckState::Failed);
        app.menu_for_session(TERM, (20, 10));
        app.on_key(KeyEvent::new(KeyCode::Char('O'), KeyModifiers::SHIFT));
        assert!(matches!(&app.mode, Mode::Confirm(c) if matches!(c.act, Act::OverrideCheck(TERM, ..))), "O overrides: {:?}", app.mode);
        assert!(commands(&mut rx).is_empty());
    }

    #[test]
    fn the_check_items_never_take_a_letter_the_row_already_uses() {
        for state in ALL_STATES {
            for status in [Status::Done, Status::Working, Status::Idle] {
                let (app, _) = rig(100, 30, |_| {}, Some(info(state)), status);
                let items = app.session_items(TERM).unwrap().1;
                let keys = menu_chars(&app, &items);
                let mut seen = Vec::new();
                for ((label, act), key) in items.iter().zip(&keys) {
                    let key = key.unwrap_or_else(|| panic!("{state:?}: {label} has a key"));
                    if matches!(act, Act::End(_)) {
                        assert_eq!(key, 'x');
                    } else {
                        assert!(!TAKEN.contains(&key) && key != 'x', "{state:?} {status:?}: {label} took {key}");
                    }
                    assert!(!seen.contains(&key), "{state:?} {status:?}: {label} repeats {key}");
                    seen.push(key);
                }
            }
        }
    }

    fn pick(app: &mut App, label: &str) {
        app.menu_for_session(TERM, (20, 10));
        let (items, keys) = match &app.mode {
            Mode::ScreenMenu(m) => (m.items.clone(), menu_chars(app, &m.items)),
            other => panic!("no menu: {other:?}"),
        };
        let i = items.iter().position(|(l, _)| l == label).unwrap_or_else(|| panic!("no {label} in {items:?}"));
        press(app, KeyCode::Char(keys[i].unwrap()));
    }

    #[test]
    fn the_menus_rerun_item_asks_the_daemon_and_says_so() {
        let (mut app, mut rx) = done(CheckState::Failed);
        pick(&mut app, "Re-run check");
        assert_eq!(rerun_of(&commands(&mut rx)), 1);
        let (msg, error) = toast(&app);
        assert!(msg.contains("Re-running") && msg.contains(COMMAND) && msg.contains("'s worktree…") && !error, "{msg}");
        assert_eq!(app.mode, Mode::Normal);
    }

    #[test]
    fn the_menus_output_item_opens_the_evidence_sheet() {
        for state in ALL_STATES {
            let (mut app, mut rx) = done(state);
            pick(&mut app, "Check output");
            assert!(matches!(&app.view, Some(View::Check(v)) if v.term == TERM), "{state:?}");
            assert!(sent(&mut rx).iter().any(|m| matches!(m, ClientMsg::Query(crate::protocol::Query::CheckOutput { term: TERM, name: None }))), "{state:?}: the output is asked for");
        }
    }

    #[test]
    fn the_menus_override_item_asks_first_and_sends_the_reason_on_yes() {
        let (mut app, mut rx) = done(CheckState::Failed);
        pick(&mut app, "Override check…");
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.danger && matches!(c.act, Act::OverrideCheck(TERM, ..))), "{:?}", app.mode);
        assert!(commands(&mut rx).is_empty(), "nothing is sent until it is confirmed");
        typed(&mut app, "flaky on CI");
        press(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::Normal);
        assert_eq!(overrides(&commands(&mut rx)), ["flaky on CI"]);
    }

    #[test]
    fn a_result_that_is_no_longer_failed_is_not_overridden_from_a_stale_menu() {
        let (mut app, mut rx) = done(CheckState::Failed);
        app.menu_for_session(TERM, (20, 10));
        app.snap.terms.get_mut(&TERM).unwrap().checks = vec![info(CheckState::Passed)];
        app.screen_sync();
        app.menu_act(Act::OverrideCheck(TERM, String::new(), None));
        assert!(commands(&mut rx).is_empty() && !matches!(app.mode, Mode::Confirm(_)), "{:?}", app.mode);
        assert!(toast(&app).0.contains("failed"), "it says why");
    }

    // ---- the override confirm ----------------------------------------------------------

    fn ask_override(app: &mut App) {
        press(app, KeyCode::Char('O'));
    }

    #[test]
    fn the_override_confirm_says_what_it_hides_and_what_stays_visible_in_every_layout() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let at = format!("{name} {w}x{h}");
                let (mut app, mut rx) = rig(w, h, layout, Some(info(CheckState::Failed)), Status::Done);
                app.open_check_sheet(TERM);
                ask_override(&mut app);
                let o = draw(&mut app, w, h);
                let text = words(&o);
                for want in [
                    "OVERRIDE A FAILED CHECK",
                    "will read done (overridden)",
                    "npm test failed",
                    "(exit 1 at",
                    "This hides a failure. It stays visible:",
                    "⊘ on the row, in the Inbox history and in the evidence",
                    "for this result only",
                    "Reason (optional)",
                    "why is this OK?",
                    "Override O",
                    "Cancel Esc",
                ] {
                    assert!(text.contains(want), "{at}: missing {want:?} in\n{o}");
                }
                assert!(fits(&o, w, h), "{at}: fits");
                typed(&mut app, "ok");
                let o = draw(&mut app, w, h);
                assert!(words(&o).contains("ok") && !words(&o).contains("why is this OK?"), "{at}: the reason shows as it is typed");
                press(&mut app, KeyCode::Esc);
                assert!(commands(&mut rx).is_empty(), "{at}: Esc sends nothing");
            }
        }
    }

    #[test]
    fn plain_icons_leave_no_unicode_check_mark_on_the_sheet_popover_or_override() {
        let plain = |app: &mut App| {
            (app.cfg.icons.blocked, app.cfg.icons.done, app.cfg.icons.idle) = ("!".into(), "+".into(), "o".into());
            app.cfg.ui.glyphs = "auto".into();
        };
        let (w, h) = (80, 24);
        for (name, layout) in layouts() {
            let at = format!("{name} {w}x{h}");
            let (mut app, _rx) = rig(w, h, layout, Some(info(CheckState::Overridden)), Status::Done);
            plain(&mut app);
            app.open_check_sheet(TERM);
            let sheet = words(&draw(&mut app, w, h));
            assert!(sheet.contains("/ Overridden by you") && !sheet.contains('⊘'), "{at}: the sheet's override banner\n{sheet}");

            for (state, want) in [(CheckState::Passed, "+ exit 0 after"), (CheckState::Failed, "x exit 1 after"), (CheckState::Overridden, "/ failed, overridden:")] {
                let (mut app, _rx) = rig(w, h, layout, Some(info(state)), Status::Done);
                plain(&mut app);
                app.mode = Mode::Why { term: TERM, side: false };
                let why = words(&draw(&mut app, w, h));
                assert!(why.contains(want), "{at} {state:?}: the popover's timeline says {want:?}\n{why}");
                assert!(!["✓ exit", "✕ exit", "⊘"].iter().any(|g| why.contains(g)), "{at} {state:?}: no Unicode mark\n{why}");
            }

            let (mut app, _rx) = rig(w, h, layout, Some(info(CheckState::Failed)), Status::Done);
            plain(&mut app);
            app.open_check_sheet(TERM);
            ask_override(&mut app);
            let confirm = words(&draw(&mut app, w, h));
            assert!(confirm.contains("/ on the row, in the Inbox history") && !confirm.contains('⊘'), "{at}: the override confirm\n{confirm}");
            press(&mut app, KeyCode::Enter);
            let (said, _) = toast(&app);
            assert!(said.contains("Overridden.") && !said.contains('⊘'), "{at}: the toast: {said:?}");
        }
    }

    #[test]
    fn the_reason_is_optional_editable_and_bounded() {
        let (mut app, mut rx) = done(CheckState::Failed);
        app.open_check_sheet(TERM);
        ask_override(&mut app);
        press(&mut app, KeyCode::Enter);
        assert_eq!(overrides(&commands(&mut rx)), [""], "an empty reason is fine");
        app.open_check_sheet(TERM);
        ask_override(&mut app);
        typed(&mut app, "abc");
        press(&mut app, KeyCode::Backspace);
        app.on_paste("\u{e9}d\nsecond line".into());
        press(&mut app, KeyCode::Enter);
        assert_eq!(overrides(&commands(&mut rx)), ["ab\u{e9}d"], "backspace edits, a paste takes its first line");
        app.open_check_sheet(TERM);
        ask_override(&mut app);
        typed(&mut app, &"x".repeat(400));
        press(&mut app, KeyCode::Enter);
        let reason = overrides(&commands(&mut rx)).remove(0);
        assert_eq!(reason.chars().count(), crate::protocol::OVERRIDE_REASON_MAX_CHARS, "the daemon would refuse a longer one");
    }

    #[test]
    fn the_override_button_is_clickable_and_so_is_cancel() {
        let (mut app, mut rx) = done(CheckState::Failed);
        app.open_check_sheet(TERM);
        ask_override(&mut app);
        click(&mut app, 100, 30, Hit::Screen(screen::ScreenHit::ConfirmNo));
        assert_eq!((app.mode.clone(), commands(&mut rx).len()), (Mode::Normal, 0), "Cancel");
        assert!(matches!(app.view, Some(View::Check(_))), "the sheet is still open behind it");
        ask_override(&mut app);
        typed(&mut app, "by mouse");
        click(&mut app, 100, 30, Hit::Screen(screen::ScreenHit::ConfirmYes));
        assert_eq!(overrides(&commands(&mut rx)), ["by mouse"]);
        assert!(toast(&app).0.contains("overridden"));
    }

    // ---- the evidence sheet ------------------------------------------------------------

    #[test]
    fn the_sheets_keys_do_what_their_buttons_say_and_only_where_they_apply() {
        // (state, r re-runs, O asks, u undoes)
        let table = [
            (CheckState::Failed, true, true, false),
            (CheckState::Passed, true, false, false),
            (CheckState::CouldNotRun, true, false, false),
            (CheckState::Overridden, true, false, true),
            (CheckState::Checking, false, false, false),
        ];
        for (state, rerun, over, undo) in table {
            let (mut app, mut rx) = done(state);
            app.open_check_sheet(TERM);
            sent(&mut rx);
            press(&mut app, KeyCode::Char('r'));
            let cmds = commands(&mut rx);
            assert_eq!(rerun_of(&cmds) == 1, rerun, "{state:?}: r");
            press(&mut app, KeyCode::Char('O'));
            assert_eq!(matches!(app.mode, Mode::Confirm(_)), over, "{state:?}: O");
            app.mode = Mode::Normal;
            press(&mut app, KeyCode::Char('u'));
            assert_eq!(commands(&mut rx).iter().filter(|c| matches!(c, Command::UndoOverride { term: TERM, name: None })).count() == 1, undo, "{state:?}: u");
            assert!(matches!(app.view, Some(View::Check(_))), "{state:?}: the sheet stays open");
        }
    }

    #[test]
    fn the_sheets_buttons_click_through_to_the_same_actions() {
        let (mut app, mut rx) = done(CheckState::Failed);
        app.open_check_sheet(TERM);
        click(&mut app, 100, 30, viewkey('r'));
        assert_eq!(rerun_of(&commands(&mut rx)), 1, "Re-run");
        click(&mut app, 100, 30, viewkey('O'));
        assert!(matches!(&app.mode, Mode::Confirm(c) if matches!(c.act, Act::OverrideCheck(TERM, ..))), "Override…");
        app.mode = Mode::Normal;
        let (mut app, mut rx) = done(CheckState::Overridden);
        app.open_check_sheet(TERM);
        click(&mut app, 100, 30, viewkey('u'));
        assert!(commands(&mut rx).iter().any(|c| matches!(c, Command::UndoOverride { term: TERM, name: None })), "Undo override");
        assert!(toast(&app).0.contains("failed again"));
    }

    #[test]
    fn the_sheets_key_hints_are_the_same_for_every_result_and_its_buttons_follow_it() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                for state in ALL_STATES {
                    let at = format!("{name} {w}x{h} {state:?}");
                    let (mut app, _rx) = rig(w, h, layout, Some(info(state)), Status::Done);
                    app.open_check_sheet(TERM);
                    let o = draw(&mut app, w, h);
                    // Above the mode bar, which names the sheet and its way out too.
                    let bar = screen::mode_bar_row(&app, Rect::new(0, 0, w, h)).y as usize;
                    let last = o.lines().take(bar).filter(|l| words(l).contains("Esc back")).last().unwrap_or_else(|| panic!("{at}: a hint row in\n{o}"));
                    assert!(words(last).contains("Esc back r re-run f full output i why"), "{at}: {last}");
                    let buttons = words(o.lines().rfind(|l| words(l).contains("Open session o") || words(l).contains("Re-run r")).unwrap_or(""));
                    assert_eq!(buttons.contains("Re-run r"), state != CheckState::Checking, "{at}: {buttons}");
                    assert_eq!(buttons.contains("Override… O"), state == CheckState::Failed, "{at}: {buttons}");
                    assert_eq!(buttons.contains("Undo override u"), state == CheckState::Overridden, "{at}: {buttons}");
                }
            }
        }
    }

    #[test]
    fn a_queued_re_run_says_queued_in_the_toast_and_the_sheet_and_is_not_sent_twice() {
        let (mut app, mut rx) = rig(100, 30, |_| {}, Some(info(CheckState::Failed)), Status::Working);
        app.open_check_sheet(TERM);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(rerun_of(&commands(&mut rx)), 1);
        assert!(toast(&app).0.contains("queued"), "{}", toast(&app).0);
        app.snap.terms.get_mut(&TERM).unwrap().checks.first_mut().unwrap().rerun_queued = true;
        app.screen_sync();
        assert!(words(&draw(&mut app, 100, 30)).contains("re-run queued"), "the sheet says it too");
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(rerun_of(&commands(&mut rx)), 0, "already queued: nothing more is sent");
        assert!(toast(&app).0.contains("queued"));
    }

    #[test]
    fn a_re_run_toasts_then_the_row_shows_the_checking_ring() {
        let (mut app, mut rx) = rig(160, 45, |_| {}, Some(info(CheckState::Failed)), Status::Done);
        app.open_check_sheet(TERM);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(rerun_of(&commands(&mut rx)), 1);
        assert!(words(&draw(&mut app, 160, 45)).contains("Re-running npm test in"), "the toast shows");
        let mut running = info(CheckState::Checking);
        running.exit_code = None;
        app.snap.terms.get_mut(&TERM).unwrap().checks = vec![running];
        app.screen_sync();
        app.open_check_sheet(TERM);
        press(&mut app, KeyCode::Esc);
        let o = draw(&mut app, 160, 45);
        assert!(o.contains("checking test ·") && ['◜', '◝', '◞', '◟'].iter().any(|g| o.contains(*g)), "the ring and its checkpoint's name:\n{o}");
        app.open_check_sheet(TERM);
        press(&mut app, KeyCode::Char('r'));
        assert!(commands(&mut rx).is_empty(), "the sheet offers no r while it runs");
        with_command(&mut app);
        check_page(&mut app, 0);
        press(&mut app, KeyCode::Char('r'));
        assert!(commands(&mut rx).is_empty() && toast(&app).0.contains("already running"), "Run now does not start it twice");
    }

    #[test]
    fn an_error_from_the_daemon_reaches_the_user_as_a_toast() {
        let (mut app, _rx) = done(CheckState::Failed);
        app.on_server(ServerMsg::Error("only a failed check can be overridden".into()));
        assert_eq!(toast(&app), ("only a failed check can be overridden".to_string(), true));
    }

    // ---- the why popover ---------------------------------------------------------------

    #[test]
    fn every_key_the_why_popover_shows_acts_and_the_others_do_not_reach_the_pane() {
        for (name, layout) in layouts() {
            for state in ALL_STATES {
                let at = format!("{name} {state:?}");
                let shown = |app: &mut App| words(&draw(app, 100, 30));
                let open = || {
                    let (mut app, rx) = rig(100, 30, layout, Some(info(state)), Status::Done);
                    app.mode = Mode::Why { term: TERM, side: false };
                    (app, rx)
                };
                let (mut app, _) = open();
                let text = shown(&mut app);
                let rerun = text.contains("r re-run");
                assert_eq!((text.contains("⏎ evidence"), rerun, text.contains("O override")), (true, state != CheckState::Checking, state == CheckState::Failed), "{at}: {text}");

                let (mut app, mut rx) = open();
                press(&mut app, KeyCode::Enter);
                assert!(matches!(app.view, Some(View::Check(_))), "{at}: Enter");
                let (mut app, mut rx2) = open();
                press(&mut app, KeyCode::Char('r'));
                let (cmds, other) = (commands(&mut rx2), sent(&mut rx2));
                assert_eq!(rerun_of(&cmds) == 1, rerun, "{at}: r");
                assert!(!other.iter().any(|m| matches!(m, ClientMsg::Input { .. })), "{at}: r never types into the pane");
                let (mut app, mut rx3) = open();
                press(&mut app, KeyCode::Char('O'));
                assert_eq!(matches!(&app.mode, Mode::Confirm(c) if matches!(c.act, Act::OverrideCheck(TERM, ..))), state == CheckState::Failed, "{at}: O");
                assert!(!sent(&mut rx3).iter().any(|m| matches!(m, ClientMsg::Input { .. })), "{at}: O never types into the pane");
                sent(&mut rx);
            }
        }
    }

    // ---- the Inbox ---------------------------------------------------------------------

    fn goto_failed(app: &mut App) {
        if !matches!(app.mode, Mode::GoTo { .. }) {
            app.act(Action::Jump);
        }
        for _ in 0..8 {
            let Mode::GoTo { sel, .. } = &app.mode else { panic!("no inbox: {:?}", app.mode) };
            let rows = screen::goto_rows(&app.screen_model(), "", 0);
            if matches!(rows.get(*sel), Some(screen::GoRow::Failed(TERM))) {
                return;
            }
            press(app, KeyCode::Down);
        }
        panic!("the failed row never got the selection");
    }

    #[test]
    fn a_selected_failed_inbox_row_shows_the_design_buttons_in_every_layout_at_every_size() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let at = format!("{name} {w}x{h}");
                let (mut app, _rx) = rig(w, h, layout, Some(info(CheckState::Failed)), Status::Done);
                app.act(Action::Jump);
                goto_failed(&mut app);
                let o = draw(&mut app, w, h);
                let text = words(&o);
                for want in ["Output ⏎", "Re-run r", "Override… O"] {
                    assert!(text.contains(want), "{at}: missing {want:?} in\n{o}");
                }
                assert!(fits(&o, w, h), "{at}: fits");
            }
        }
    }

    #[test]
    fn the_inbox_buttons_and_their_keys_act() {
        let (mut app, mut rx) = done(CheckState::Failed);
        goto_failed(&mut app);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(rerun_of(&commands(&mut rx)), 1, "r");
        assert!(matches!(app.mode, Mode::GoTo { .. }), "the Inbox stays open");
        draw(&mut app, 100, 30);
        click(&mut app, 100, 30, Hit::Screen(screen::ScreenHit::InboxKey('r')));
        assert_eq!(rerun_of(&commands(&mut rx)), 1, "the Re-run button");
        press(&mut app, KeyCode::Char('O'));
        assert!(matches!(&app.mode, Mode::Confirm(c) if matches!(c.act, Act::OverrideCheck(TERM, ..))), "O");
        press(&mut app, KeyCode::Esc);
        goto_failed(&mut app);
        click(&mut app, 100, 30, Hit::Screen(screen::ScreenHit::InboxKey('O')));
        assert!(matches!(app.mode, Mode::Confirm(_)), "the Override… button");
        press(&mut app, KeyCode::Esc);
        goto_failed(&mut app);
        click(&mut app, 100, 30, Hit::Screen(screen::ScreenHit::InboxKey('\n')));
        assert!(matches!(&app.view, Some(View::Check(v)) if v.term == TERM) && app.mode == Mode::Normal, "the Output button");
        app.view = None;
        goto_failed(&mut app);
        press(&mut app, KeyCode::Char('o'));
        assert!(app.view.is_none() && app.mode == Mode::Normal, "o opens the session");
        assert!(commands(&mut rx).iter().any(|c| matches!(c, Command::FocusPane { term } if *term == TERM)), "o focuses it");
    }

    #[test]
    fn enter_in_the_inbox_opens_a_failure_s_output_and_typing_a_search_still_searches() {
        let (mut app, mut rx) = done(CheckState::Failed);
        goto_failed(&mut app);
        press(&mut app, KeyCode::Enter);
        assert!(app.mode == Mode::Normal && matches!(&app.view, Some(View::Check(v)) if v.term == TERM), "Enter is the first button: Output");
        app.view = None;
        assert!(commands(&mut rx).iter().all(|c| !matches!(c, Command::RerunCheck { .. })));
        app.act(Action::Jump);
        assert!(matches!(app.mode, Mode::GoTo { .. }), "{:?}", app.mode);
        typed(&mut app, "sh");
        assert!(matches!(&app.mode, Mode::GoTo { query, .. } if query == "sh"), "other letters still search");
        assert_eq!(rerun_of(&commands(&mut rx)), 0);
        typed(&mut app, "r");
        assert!(matches!(&app.mode, Mode::GoTo { query, .. } if query == "shr"), "and once a search has begun, r is part of it");
    }

    #[test]
    fn a_failed_row_that_is_not_selected_and_other_rows_have_no_check_buttons() {
        let (mut app, _rx) = done(CheckState::Passed);
        app.act(Action::Jump);
        let text = words(&draw(&mut app, 100, 30));
        assert!(!text.contains("Re-run r") && !text.contains("Override…"), "{text}");
    }

    // ---- Settings > Check --------------------------------------------------------------

    fn check_page(app: &mut App, sel: usize) {
        let cat = modal::Cat::ALL.iter().position(|c| *c == modal::Cat::Check).unwrap();
        app.mode = Mode::ScreenSettings(Box::new(design::SettingsView { cat, sel, editing: None, capturing: false, scroll: 0 }));
    }

    fn with_command(app: &mut App) {
        let project = PathBuf::from(if cfg!(windows) { r"C:\code\shop-api" } else { "/code/shop-api" });
        app.cfg.check = vec![crate::config::CheckDef { project, name: None, command: COMMAND.into(), timeout_s: 600 }];
    }

    #[test]
    fn the_check_page_has_a_run_now_button_in_every_layout_at_every_size() {
        for (name, layout) in layouts() {
            for (w, h) in SIZES {
                let at = format!("{name} {w}x{h}");
                let (mut app, _rx) = rig(w, h, layout, Some(info(CheckState::Passed)), Status::Done);
                with_command(&mut app);
                check_page(&mut app, 0);
                let o = draw(&mut app, w, h);
                assert!(words(&o).contains("Run now r"), "{at}: the button in\n{o}");
                assert!(fits(&o, w, h), "{at}: fits");
            }
        }
    }

    #[test]
    fn run_now_re_runs_the_latest_session_by_key_and_click() {
        let (mut app, mut rx) = done(CheckState::Passed);
        with_command(&mut app);
        check_page(&mut app, 0);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(rerun_of(&commands(&mut rx)), 1, "r on the page");
        click(&mut app, 100, 30, Hit::Screen(screen::ScreenHit::CheckDraft(screen::check_settings::DraftHit::RunNow)));
        assert_eq!(rerun_of(&commands(&mut rx)), 1, "a click on the button");
        assert!(matches!(app.mode, Mode::ScreenSettings(_)), "Settings stays open");
    }

    #[test]
    fn with_no_check_set_r_is_typed_into_the_command() {
        let (mut app, mut rx) = done(CheckState::Passed);
        check_page(&mut app, 0);
        press(&mut app, KeyCode::Char('r'));
        assert!(commands(&mut rx).is_empty());
        assert_eq!(app.screen.check_page.as_ref().and_then(|d| d.command.as_deref()), Some("r"));
    }

    #[test]
    fn run_now_while_the_agent_works_queues_and_says_so() {
        let (mut app, mut rx) = rig(100, 30, |_| {}, Some(info(CheckState::Failed)), Status::Working);
        with_command(&mut app);
        check_page(&mut app, 0);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(rerun_of(&commands(&mut rx)), 1);
        assert!(toast(&app).0.contains("queued"));
    }

    #[test]
    fn typing_r_in_the_command_field_still_types_an_r() {
        let (mut app, mut rx) = done(CheckState::Passed);
        with_command(&mut app);
        check_page(&mut app, 0);
        press(&mut app, KeyCode::Enter);
        press(&mut app, KeyCode::Char('r'));
        assert_eq!(app.screen.check_page.as_ref().and_then(|d| d.command.as_deref()), Some("npm testr"));
        assert!(commands(&mut rx).is_empty());
    }
}

mod restart_tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    const SIZES: [(u16, u16); 3] = [(80, 24), (100, 30), (160, 45)];
    /// (ui.layout, ui.panes): Cards floating and Cards tiled, as older files wrote them.
    const LAYOUTS: [(&str, &str); 2] = [("cards", "floating"), ("cards", "tiled")];
    const OLD_DAEMON: &str = "0.36.11";
    const INSTALLED: &str = "9.9.9";

    /// The shop-api scene (3 sessions, 1 working) whose daemon is older than the installed the app.
    fn scene(w: u16, h: u16, layout: (&str, &str)) -> (App, mpsc::UnboundedReceiver<ClientMsg>) {
        let (_, mut app) = super::design_tests::render_with(w, h);
        (app.cfg.ui.layout, app.cfg.ui.panes) = (layout.0.into(), layout.1.into());
        app.cfg.ui.fold_retired_layout();
        let (tx, rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.got_state = true;
        app.installed_version = Some(Ok(INSTALLED.into()));
        app.snap.daemon_version = OLD_DAEMON.into();
        (app, rx)
    }

    fn draw(app: &mut App, w: u16, h: u16) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let out: String = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
            println!("{out}");
        }
        out
    }

    /// The mode bar's row on a screen `h` rows tall.
    fn bar_y(app: &App, h: u16) -> u16 {
        screen::mode_bar_row(app, ratatui::layout::Rect::new(0, 0, 1, h)).y
    }

    /// The Restart buttons of the toast and Settings, on a screen `h` rows tall: not the pill on
    /// the mode bar.
    fn restart_hit(app: &App, h: u16) -> Option<ratatui::layout::Rect> {
        app.hits.iter().find_map(|(r, k)| (*k == Hit::Screen(screen::ScreenHit::Restart) && r.y != bar_y(app, h)).then_some(*r))
    }

    fn pill_hit(app: &App, h: u16) -> Option<ratatui::layout::Rect> {
        app.hits.iter().find_map(|(r, k)| (*k == Hit::Screen(screen::ScreenHit::Restart) && r.y == bar_y(app, h)).then_some(*r))
    }

    fn click(app: &mut App, r: ratatui::layout::Rect) {
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: r.x + r.width / 2, row: r.y, modifiers: KeyModifiers::NONE });
    }

    fn key(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn sent(rx: &mut mpsc::UnboundedReceiver<ClientMsg>) -> Vec<ClientMsg> {
        std::iter::from_fn(|| rx.try_recv().ok()).collect()
    }

    fn is_restart(act: &menu::Act) -> bool {
        *act == menu::Act::RestartDaemon
    }

    #[test]
    fn settings_offers_restart_now_in_every_layout_and_size() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let (mut app, mut rx) = scene(w, h, layout);
                app.act(Action::Settings);
                let out = draw(&mut app, w, h);
                assert!(out.contains("Restart now"), "{layout:?} {w}x{h}\n{out}");
                let r = restart_hit(&app, h).unwrap_or_else(|| panic!("{layout:?} {w}x{h}: the button is clickable"));
                assert!(r.right() <= w && r.bottom() <= h, "{layout:?} {w}x{h}: the button fits {r:?}");
                click(&mut app, r);
                assert!(matches!(&app.mode, Mode::Confirm(c) if is_restart(&c.act)), "{layout:?} {w}x{h}");
                assert!(sent(&mut rx).is_empty(), "choosing it only asks");
            }
        }
    }

    #[test]
    fn settings_has_no_restart_button_when_the_daemon_is_current() {
        let (mut app, _rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.daemon_version = INSTALLED.into();
        app.act(Action::Settings);
        let out = draw(&mut app, 100, 30);
        assert!(!out.contains("Restart now") && restart_hit(&app, 30).is_none());
        app.snap.daemon_version = OLD_DAEMON.into();
        app.daemon_restart_requested = true;
        let out = draw(&mut app, 100, 30);
        assert!(!out.contains("Restart now"), "not while a restart is already under way");
    }

    #[test]
    fn the_update_toast_offers_restart_now_in_every_layout_and_size() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let (mut app, _rx) = scene(w, h, layout);
                app.update_installed(INSTALLED.into());
                let out = draw(&mut app, w, h);
                assert!(out.contains("Restart now"), "{layout:?} {w}x{h}\n{out}");
                let r = restart_hit(&app, h).unwrap_or_else(|| panic!("{layout:?} {w}x{h}: the toast button is clickable"));
                click(&mut app, r);
                assert!(matches!(&app.mode, Mode::Confirm(c) if is_restart(&c.act)), "{layout:?} {w}x{h}");
            }
        }
    }

    #[test]
    fn a_later_toast_does_not_keep_the_restart_button() {
        let (mut app, _rx) = scene(160, 45, LAYOUTS[0]);
        app.update_installed(INSTALLED.into());
        app.notify("Copied".into(), false);
        let out = draw(&mut app, 160, 45);
        assert!(out.contains("Copied") && !out.contains("Restart now") && restart_hit(&app, 45).is_none());
    }

    #[test]
    fn the_confirm_says_how_many_sessions_stop_and_how_many_are_working() {
        for layout in LAYOUTS {
            for (w, h) in SIZES {
                let (mut app, _rx) = scene(w, h, layout);
                app.ask_restart();
                assert!(matches!(&app.mode, Mode::Confirm(c) if is_restart(&c.act)), "{layout:?} {w}x{h}");
                let out = draw(&mut app, w, h);
                for want in ["3 sessions stop and resume, 1 is working", "Restart", "Cancel"] {
                    assert!(out.contains(want), "{layout:?} {w}x{h} lacks {want:?}\n{out}");
                }
            }
        }
    }

    #[test]
    fn the_summary_words_sessions_working_and_waiting() {
        assert_eq!(restart_summary(1, 0, 0), "1 session stops and resumes");
        assert_eq!(restart_summary(3, 1, 0), "3 sessions stop and resume, 1 is working");
        assert_eq!(restart_summary(4, 2, 1), "4 sessions stop and resume, 2 are working, 1 waiting for you");
        assert_eq!(restart_summary(2, 0, 2), "2 sessions stop and resume, 2 waiting for you");
    }

    #[test]
    fn idle_sessions_restart_without_asking() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.terms.retain(|id, _| *id == 3);
        assert_ne!(app.snap.terms[&3].status, Status::Working);
        app.ask_restart();
        assert_eq!(app.mode, Mode::Normal, "nothing is mid-turn: no question");
        assert!(matches!(sent(&mut rx).as_slice(), [ClientMsg::Command(Command::KillServer { forget: false })]));
    }

    #[test]
    fn a_session_waiting_for_you_asks_once_and_is_counted() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.terms.retain(|id, _| *id == 3);
        app.snap.terms.get_mut(&3).unwrap().status = Status::Blocked;
        app.ask_restart();
        assert!(matches!(&app.mode, Mode::Confirm(c) if is_restart(&c.act) && c.what == "1 session stops and resumes, 1 waiting for you"), "{:?}", app.mode);
        assert!(sent(&mut rx).is_empty());
        key(&mut app, KeyCode::Enter);
        assert!(matches!(sent(&mut rx).as_slice(), [ClientMsg::Command(Command::KillServer { forget: false })]), "Enter is the one keypress");
    }

    #[test]
    fn with_restoring_off_the_restart_asks_and_says_sessions_close() {
        let (mut app, _rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.terms.retain(|id, _| *id == 3);
        app.cfg.restore.enabled = false;
        app.ask_restart();
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.note.contains("close for good") && c.danger), "{:?}", app.mode);
    }

    #[test]
    fn an_old_window_gives_way_to_the_installed_app_when_the_server_stops() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        assert!(app.window_stale(), "the installed version is newer than this window");
        app.ask_restart();
        key(&mut app, KeyCode::Enter);
        assert!(matches!(sent(&mut rx).as_slice(), [ClientMsg::Command(Command::KillServer { forget: false })]));
        app.on_server(ServerMsg::Bye);
        assert!(app.relaunch && app.quit.is_some(), concat!("the window closes so the installed ", env!("APP_COMMAND"), " opens in its place"));
        assert!(!app.daemon_restart_reconnect, "an old window never reconnects to the new server");
    }

    #[test]
    fn a_current_window_reconnects_to_the_restarted_server() {
        let (mut app, _rx) = scene(100, 30, LAYOUTS[0]);
        app.installed_version = Some(Ok(env!("CARGO_PKG_VERSION").into()));
        assert!(!app.window_stale() && app.daemon_restartable());
        app.daemon_restart_requested = true;
        app.on_server(ServerMsg::Bye);
        assert!(!app.relaunch && app.quit.is_none() && app.daemon_restart_reconnect);
    }

    #[test]
    fn when_only_the_window_is_old_restart_swaps_the_window_and_leaves_the_server() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.daemon_version = INSTALLED.into();
        assert!(!app.daemon_restartable() && app.restart_needed());
        app.ask_restart();
        assert!(app.relaunch && app.quit.is_some());
        assert!(sent(&mut rx).is_empty(), "the server and every session keep running");
    }

    #[test]
    fn the_bar_pill_says_update_then_restart_then_nothing() {
        let row0 = |app: &mut App| draw(app, 100, 30).lines().nth(bar_y(app, 30) as usize).unwrap().to_string();
        let (mut app, _rx) = scene(100, 30, LAYOUTS[0]);
        app.update_available = Some("9.9.9".into());
        let top = row0(&mut app);
        assert!(top.contains("↑ Update 9.9.9") && !top.contains("↻ Restart"), "a release is out: {top}");
        app.update_available = None;
        let top = row0(&mut app);
        assert!(top.contains("↻ Restart") && !top.contains("Update"), "installed but not running: {top}");
        app.daemon_restart_requested = true;
        assert!(!row0(&mut app).contains("↻ Restart"), "not while a restart is under way");
        app.daemon_restart_requested = false;
        app.snap.daemon_version = env!("CARGO_PKG_VERSION").into();
        app.installed_version = Some(Ok(env!("CARGO_PKG_VERSION").into()));
        let top = row0(&mut app);
        assert!(!top.contains("Restart") && !top.contains("Update"), "everything runs the installed version: {top}");
    }

    #[test]
    fn the_pill_is_drawn_and_clicking_it_restarts() {
        for layout in LAYOUTS {
            let (mut app, mut rx) = scene(100, 30, layout);
            app.snap.terms.retain(|id, _| *id == 3);
            let out = draw(&mut app, 100, 30);
            assert!(out.lines().nth(bar_y(&app, 30) as usize).unwrap().contains("↻ Restart"), "{layout:?}\n{out}");
            let r = pill_hit(&app, 30).expect("the pill is clickable, on the mode bar");
            click(&mut app, r);
            assert!(matches!(sent(&mut rx).as_slice(), [ClientMsg::Command(Command::KillServer { forget: false })]), "{layout:?}");
        }
    }

    #[test]
    fn restart_is_in_the_palette_and_says_so_when_there_is_nothing_to_do() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.terms.retain(|id, _| *id == 3);
        app.act(Action::Restart);
        assert!(matches!(sent(&mut rx).as_slice(), [ClientMsg::Command(Command::KillServer { .. })]));
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.daemon_version = env!("CARGO_PKG_VERSION").into();
        app.installed_version = Some(Ok(env!("CARGO_PKG_VERSION").into()));
        app.act(Action::Restart);
        assert!(sent(&mut rx).is_empty());
        assert!(app.notice.as_ref().is_some_and(|(t, ..)| t.contains("already running the installed version")));
        assert_eq!(Action::Restart.to_config().as_deref(), Some("restart"));
    }

    #[test]
    fn cancel_changes_nothing() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.ask_restart();
        key(&mut app, KeyCode::Esc);
        assert_eq!(app.mode, Mode::Normal);
        assert!(sent(&mut rx).is_empty() && !app.daemon_restart_requested && !app.daemon_restart_reconnect);
        app.ask_restart();
        draw(&mut app, 100, 30);
        let cancel = app.hits.iter().rev().filter(|(_, h)| *h == Hit::Screen(screen::ScreenHit::ConfirmNo)).map(|(r, _)| *r).min_by_key(|r| r.width).unwrap();
        click(&mut app, cancel);
        assert_eq!(app.mode, Mode::Normal);
        assert!(sent(&mut rx).is_empty() && !app.daemon_restart_requested);
    }

    #[test]
    fn restarting_stops_the_daemon_and_the_window_reconnects_to_the_new_one() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        // A window already on the installed version has nothing to swap but the server.
        app.installed_version = Some(Ok(env!("CARGO_PKG_VERSION").into()));
        app.snap.daemon_version = "0.1.0".into();
        app.ask_restart();
        key(&mut app, KeyCode::Enter);
        assert!(matches!(sent(&mut rx).as_slice(), [ClientMsg::Command(Command::KillServer { forget: false })]));
        assert!(app.daemon_restart_requested);

        app.on_server(ServerMsg::Bye);
        assert!(app.quit.is_none() && app.daemon_restart_reconnect, "the window stays open for the reconnect");

        let mut snapshot = app.snap.clone();
        snapshot.daemon_version = env!("CARGO_PKG_VERSION").into();
        app.on_server(ServerMsg::State(snapshot));
        assert!(!app.daemon_restart_requested && !app.daemon_restart_reconnect);
        assert_eq!(app.snap.terms.len(), 3, "the restored sessions are back");
        assert!(!app.daemon_restartable(), "the daemon now matches the install");
        assert!(app.notice.as_ref().is_some_and(|(text, ..)| text.contains("daemon restarted with the update")));
    }

    #[test]
    fn with_no_sessions_open_it_restarts_without_asking() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.terms.clear();
        app.ask_restart();
        assert_eq!(app.mode, Mode::Normal);
        assert!(matches!(sent(&mut rx).as_slice(), [ClientMsg::Command(Command::KillServer { forget: false })]));
        assert!(app.daemon_restart_requested);
    }

    #[test]
    fn nothing_restarts_when_the_daemon_is_not_older() {
        let (mut app, mut rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.daemon_version = INSTALLED.into();
        app.ask_restart();
        assert_eq!(app.mode, Mode::Normal);
        assert!(sent(&mut rx).is_empty() && !app.daemon_restart_requested);
    }

    #[test]
    fn capital_r_in_settings_asks_to_restart() {
        let (mut app, _rx) = scene(100, 30, LAYOUTS[0]);
        app.act(Action::Settings);
        key(&mut app, KeyCode::Char('R'));
        assert!(matches!(&app.mode, Mode::Confirm(c) if is_restart(&c.act)));
        let (mut app, _rx) = scene(100, 30, LAYOUTS[0]);
        app.snap.daemon_version = INSTALLED.into();
        app.act(Action::Settings);
        key(&mut app, KeyCode::Char('R'));
        assert!(matches!(app.mode, Mode::ScreenSettings(_)), "nothing to restart: Settings stays");
    }

    #[test]
    fn the_settings_button_shows_its_key() {
        let (mut app, _rx) = scene(100, 30, LAYOUTS[0]);
        app.act(Action::Settings);
        assert!(draw(&mut app, 100, 30).contains("Restart now R"));
    }
}


mod inbox_search_tests {
    use super::*;

    /// An empty Inbox: type a letter (sessions match), Backspace it. The selection from the
    /// matches must not outlive them, or drawing the empty list reads past its end.
    #[test]
    fn backspacing_the_last_letter_in_an_empty_inbox_keeps_it_open() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        for t in app.snap.terms.values_mut() {
            t.status = crate::protocol::Status::Idle;
        }
        app.snap.questions.clear();
        app.screen_fresh();
        app.open_goto();
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 45)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        app.on_key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE));
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        app.on_key(KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE));
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        assert!(matches!(&app.mode, Mode::GoTo { query, .. } if query.is_empty()), "the Inbox stays open, its search empty");
    }
}

mod side_panel_width_tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;

    /// The Inbox open in the side sheet on a screen `w` columns wide, drawn once.
    fn inbox_open(w: u16, h: u16) -> App {
        let (_, mut app) = super::design_tests::render_with(w, h);
        app.act(Action::Jump);
        draw(&mut app, w, h);
        app
    }

    fn draw(app: &mut App, w: u16, h: u16) {
        let mut term = Terminal::new(TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
    }

    fn key(app: &mut App, c: char) {
        app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    fn edge(app: &App) -> Option<Rect> {
        app.hits.iter().find(|(_, h)| *h == Hit::Screen(screen::ScreenHit::SheetEdge)).map(|(r, _)| *r)
    }

    #[test]
    fn brackets_narrow_and_widen_the_sheet_and_equals_puts_its_default_back() {
        let mut app = inbox_open(200, 50);
        assert_eq!(app.screen.sheet_rect.width, 72, "the Inbox starts 72 wide");
        key(&mut app, '[');
        draw(&mut app, 200, 50);
        assert_eq!(app.screen.sheet_rect.width, 68, "[ narrows it 4");
        key(&mut app, ']');
        key(&mut app, ']');
        draw(&mut app, 200, 50);
        assert_eq!(app.screen.sheet_rect.width, 76, "] widens it 4 each");
        assert_eq!(app.screen.saved.sheet_w.get("inbox"), Some(&76), "kept in the saved UI state");
        key(&mut app, '=');
        draw(&mut app, 200, 50);
        assert_eq!(app.screen.sheet_rect.width, 72, "= puts the default back");
        assert!(matches!(&app.mode, Mode::GoTo { query, .. } if query.is_empty()), "the keys sized the sheet, not the search");
    }

    #[test]
    fn dragging_the_sheets_edge_resizes_it() {
        let mut app = inbox_open(200, 50);
        let at = edge(&app).expect("an edge to drag beside the panes");
        let mouse = |kind, column| MouseEvent { kind, column, row: at.y + 5, modifiers: KeyModifiers::NONE };
        app.on_mouse(mouse(MouseEventKind::Down(MouseButton::Left), at.x));
        app.on_mouse(mouse(MouseEventKind::Drag(MouseButton::Left), at.x - 10));
        app.on_mouse(mouse(MouseEventKind::Up(MouseButton::Left), at.x - 10));
        draw(&mut app, 200, 50);
        assert_eq!(app.screen.sheet_rect.width, app.screen.sheet_rect.right() - (at.x - 10), "the sheet's edge follows the pointer");
        assert!(app.screen.sheet_rect.width > 72, "wider than before");
    }

    #[test]
    fn the_sheet_stays_between_its_least_and_what_leaves_the_panes_their_room() {
        let mut app = inbox_open(200, 50);
        app.screen.saved.sheet_w.insert("inbox".into(), 10);
        draw(&mut app, 200, 50);
        assert_eq!(app.screen.sheet_rect.width, screen::SHEET_MIN, "never under its least");
        app.screen.saved.sheet_w.insert("inbox".into(), 500);
        draw(&mut app, 200, 50);
        let panes_w = edge(&app).map(|e| e.x).unwrap() - app.screen.side_rect.right();
        assert!(panes_w >= screen::PANES_KEEP, "the panes keep {} columns, have {panes_w}", screen::PANES_KEEP);
        // Too narrow for both: the sheet takes the whole pane column, with no edge to drag.
        let app = inbox_open(80, 24);
        assert!(edge(&app).is_none() && app.screen.sheet_rect.x <= app.screen.side_rect.right() + 1, "{:?}", app.screen.sheet_rect);
    }

    #[test]
    fn changes_starts_half_the_screen_wide_and_an_older_saved_file_still_loads() {
        let (_, app) = super::design_tests::render_with(200, 50);
        assert_eq!(screen::wanted_w(&app, screen::SheetKind::Changes, 200), 100);
        assert_eq!(screen::wanted_w(&app, screen::SheetKind::Check, 200), 72);
        let old: screen::Saved = serde_json::from_str(r#"{"order":[],"draft":""}"#).expect("a file from before widths were kept");
        assert!(old.sheet_w.is_empty());
    }

    #[test]
    fn on_a_narrow_screen_the_keys_leave_your_width_alone() {
        let mut app = inbox_open(80, 24);
        key(&mut app, ']');
        key(&mut app, '[');
        assert!(app.screen.saved.sheet_w.is_empty(), "taking the whole column, there is no width to change: {:?}", app.screen.saved.sheet_w);
    }

    #[test]
    fn keys_step_from_the_chosen_width_not_the_drawn_one() {
        let mut app = inbox_open(200, 50);
        // Mid-slide, or left from another view: the drawn rectangle says nothing about the width.
        app.screen.sheet_rect.width = 22;
        key(&mut app, '[');
        assert_eq!(app.screen.saved.sheet_w.get("inbox"), Some(&68));
    }

    #[test]
    fn at_160_columns_the_inbox_leaves_the_panes_their_room() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        app.cfg.ui.padding = "1".into();
        app.act(Action::Jump);
        draw(&mut app, 160, 45);
        let panes_w = edge(&app).map(|e| e.x).unwrap() - app.screen.side_rect.right();
        assert!(panes_w >= screen::PANES_KEEP, "{panes_w}");
        assert!(app.screen.sheet_rect.width < 72 && app.screen.sheet_rect.width >= screen::SHEET_MIN, "{}", app.screen.sheet_rect.width);
    }

    #[test]
    fn the_edge_lights_up_while_pointed_at() {
        let mut app = inbox_open(200, 50);
        let at = edge(&app).unwrap();
        app.hover = Some(Position { x: at.x, y: at.y + 5 });
        let mut term = Terminal::new(TestBackend::new(200, 50)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let cell = &term.backend().buffer()[(at.x + at.width / 2, at.y + 5)];
        assert_eq!((cell.symbol(), cell.fg), ("│", app.theme.accent));
    }

    #[test]
    fn widths_survive_a_save_and_load() {
        let mut saved = screen::Saved::default();
        saved.sheet_w.insert("changes".into(), 90);
        let back: screen::Saved = serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        assert_eq!(back.sheet_w.get("changes"), Some(&90));
    }

    #[test]
    fn the_keys_size_changes_too() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        app.view = Some(View::Changes(Box::new(views::ChangesView {
            dir: PathBuf::from("/repo"),
            review: None,
            error: None,
            agent: String::new(),
            said: String::new(),
            term: None,
            ws: None,
            reviewed: Default::default(),
            linked: false,
            confirm: None,
        })));
        draw(&mut app, 200, 50);
        let widest = app.screen.sheet_widest;
        assert_eq!(app.screen.sheet_rect.width, 100.min(widest), "Changes starts half the screen, held to the room");
        key(&mut app, '[');
        assert_eq!(app.screen.saved.sheet_w.get("changes"), Some(&(100.min(widest) - 4)));
    }

    fn changes_view() -> View {
        View::Changes(Box::new(views::ChangesView {
            dir: PathBuf::from("/repo"),
            review: None,
            error: None,
            agent: String::new(),
            said: String::new(),
            term: None,
            ws: None,
            reviewed: Default::default(),
            linked: false,
            confirm: None,
        }))
    }

    fn click(app: &mut App, x: u16, y: u16) {
        for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
            app.on_mouse(MouseEvent { kind, column: x, row: y, modifiers: KeyModifiers::NONE });
        }
    }

    /// Whether `c` typed now reaches a pane (an `Input` goes out), drained afterwards.
    fn typed_into_a_pane(app: &mut App, rx: &mut mpsc::UnboundedReceiver<ClientMsg>, c: char) -> bool {
        while rx.try_recv().is_ok() {}
        key(app, c);
        std::iter::from_fn(|| rx.try_recv().ok()).any(|m| matches!(m, ClientMsg::Input { .. }))
    }

    #[test]
    fn a_click_into_a_pane_gives_it_the_keys_and_the_view_stays_open() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.view = Some(changes_view());
        draw(&mut app, 200, 50);
        assert!(!typed_into_a_pane(&mut app, &mut rx, 'm'), "Changes has the keys first");
        let (_, pane) = *app.pane_frames.first().expect("a pane beside the sheet");
        click(&mut app, pane.x + pane.width / 2, pane.y + pane.height / 2);
        draw(&mut app, 200, 50);
        assert!(typed_into_a_pane(&mut app, &mut rx, 'm'), "after a click into the pane, typing goes there");
        assert!(matches!(app.view, Some(View::Changes(_))), "and Changes is still open");
        let sheet = app.screen.sheet_rect;
        click(&mut app, sheet.x + sheet.width / 2, sheet.y + sheet.height / 2);
        assert!(!typed_into_a_pane(&mut app, &mut rx, 'm'), "a click in the sheet gives Changes the keys back");
    }

    #[test]
    fn after_leader_n_beside_a_panel_the_next_key_goes_to_the_new_pane() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.cfg.ui.which_key = false;
        app.show_view(changes_view());
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        key(&mut app, 'n');
        assert!(matches!(app.view, Some(View::Changes(_))), "Changes stays open beside it");
        let mut term = Terminal::new(TestBackend::new(200, 50)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let buf = term.backend().buffer();
        let bar = screen::mode_bar_row(&app, buf.area);
        let text: String = (bar.x..bar.right()).map(|x| buf[(x, bar.y)].symbol().to_string()).collect();
        assert!(text.trim_start().starts_with("TYPING"), "the bar says typing, not CHANGES: {text}");
        assert!(typed_into_a_pane(&mut app, &mut rx, 'm'), "the next key types into the pane, not Changes");
    }

    fn leader_x(app: &mut App) {
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        key(app, 'x');
    }

    /// A Changes sheet open beside a pane that is still working.
    fn changes_beside_a_working_pane() -> App {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let focused = app.focused().unwrap();
        app.snap.terms.get_mut(&focused).unwrap().status = Status::Working;
        app.show_view(changes_view());
        draw(&mut app, 200, 50);
        app
    }

    #[test]
    fn leader_x_closes_the_side_panel_that_has_the_keys_not_the_pane_behind_it() {
        let mut app = changes_beside_a_working_pane();
        leader_x(&mut app);
        assert!(app.view.is_none(), "x closed Changes");
        assert!(!matches!(app.mode, Mode::Confirm(_)), "and asked nothing about the pane behind it: {:?}", std::mem::discriminant(&app.mode));
    }

    #[test]
    fn leader_x_closes_the_pane_when_a_click_set_the_panel_aside() {
        let mut app = changes_beside_a_working_pane();
        let focused = app.focused().unwrap();
        let (_, pane) = *app.pane_frames.iter().find(|(t, _)| *t == focused).expect("the focused pane beside the sheet");
        click(&mut app, pane.x + pane.width / 2, pane.y + pane.height / 2);
        leader_x(&mut app);
        let Mode::Confirm(c) = &app.mode else { panic!("the pane has the keys: x asks before closing it") };
        assert_eq!(c.act, crate::client::menu::Act::End(vec![focused]), "it closes the focused pane");
        assert!(c.note.contains("still working"), "and warns that it is working: {}", c.note);
        assert!(matches!(app.view, Some(View::Changes(_))), "Changes stays open beside it");
    }

    #[test]
    fn leader_x_in_the_sidebar_closes_the_cursor_s_row_not_the_panel() {
        let mut app = changes_beside_a_working_pane();
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app.on_key(KeyEvent::new(KeyCode::Left, KeyModifiers::NONE));
        let row = app.screen.cursor.expect("leader ← put the cursor on a sidebar row");
        app.snap.terms.get_mut(&row).unwrap().status = Status::Working;
        leader_x(&mut app);
        let Mode::Confirm(c) = &app.mode else { panic!("x asks before closing the row: {:?}", std::mem::discriminant(&app.mode)) };
        assert_eq!(c.act, crate::client::menu::Act::End(vec![row]), "it closes the cursor's session");
        assert!(c.note.contains("still working"), "and warns that it is working: {}", c.note);
        assert!(matches!(app.view, Some(View::Changes(_))), "Changes stays open");
    }

    #[test]
    fn leader_x_with_no_panel_closes_the_focused_split() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let a = app.focused().unwrap();
        let b = app.snap.terms.keys().copied().find(|t| *t != a).unwrap();
        app.screen.tabs.clear();
        app.screen_place(a, None);
        app.screen.pending_split = Some((a, Instant::now()));
        app.screen_place(b, Some(a));
        draw(&mut app, 200, 50);
        leader_x(&mut app);
        assert!(matches!(&app.mode, Mode::Confirm(c) if c.act == crate::client::menu::Act::End(vec![a])), "x asks about the focused split only");
    }

    /// Files open in the sheet, set aside by a click into the pane beside it.
    fn files_set_aside() -> App {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        app.show_view(View::Files(Box::new(views::FilesTree::new(std::env::temp_dir(), None))));
        draw(&mut app, 200, 50);
        let (_, pane) = *app.pane_frames.first().unwrap();
        click(&mut app, pane.x + pane.width / 2, pane.y + pane.height / 2);
        assert!(!app.view_has_keys(), "set aside");
        app
    }

    #[test]
    fn the_pane_with_the_keys_shows_its_cursor() {
        let mut app = files_set_aside();
        let mut term = Terminal::new(TestBackend::new(200, 50)).unwrap();
        term.draw(|f| render::draw(&mut app, f)).unwrap();
        let at = term.get_cursor_position().unwrap();
        let (_, pane) = *app.pane_frames.first().unwrap();
        assert!(pane.contains(at), "the caret is in the pane that has the keys: {at:?} not in {pane:?}");
    }

    #[test]
    fn f_hands_a_set_aside_files_view_the_keys_and_the_leader_leaves_you_in_it() {
        let mut app = files_set_aside();
        app.act(Action::Files);
        assert!(matches!(app.view, Some(View::Files(_))) && app.view_has_keys(), "f gave Files the keys back, not closed it");
        // Through the leader in shortcut mode, too: the keys go to Files, not back to the leader.
        let mut app = files_set_aside();
        app.cfg.ui.which_key = false;
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        key(&mut app, 'f');
        assert!(app.view_has_keys() && matches!(app.mode, Mode::Normal), "in Files: {:?}", std::mem::discriminant(&app.mode));
    }

    #[test]
    fn a_new_view_opened_any_way_has_the_keys() {
        let mut app = files_set_aside();
        app.open_files(std::env::temp_dir());
        assert!(app.view_has_keys(), "open_files (as Ctrl+click on a path does) gives the new view the keys");
    }

    #[test]
    fn a_check_sheet_set_aside_stays_aside_when_its_run_updates() {
        use crate::protocol::{CheckInfo, CheckState};
        let run = |started_at| CheckInfo { command: "npm test".into(), state: CheckState::Failed, exit_code: Some(1), started_at, ..Default::default() };
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.snap.terms.get_mut(&1).unwrap().checks = vec![run(100)];
        app.open_check_sheet(1);
        draw(&mut app, 200, 50);
        let (_, pane) = *app.pane_frames.first().unwrap();
        click(&mut app, pane.x + pane.width / 2, pane.y + pane.height / 2);
        // The check runs again: the sheet is brought up to date in place.
        app.snap.terms.get_mut(&1).unwrap().checks = vec![run(200)];
        app.check_view_sync();
        assert!(matches!(app.view, Some(View::Check(_))), "the sheet is still open");
        assert!(typed_into_a_pane(&mut app, &mut rx, 'o'), "typing still reaches the pane, not the sheet");
    }

    #[test]
    fn a_view_opened_again_has_the_keys() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.view = Some(changes_view());
        draw(&mut app, 200, 50);
        let (_, pane) = *app.pane_frames.first().unwrap();
        click(&mut app, pane.x + pane.width / 2, pane.y + pane.height / 2);
        app.view = None;
        draw(&mut app, 200, 50);
        app.view = Some(changes_view());
        draw(&mut app, 200, 50);
        assert!(!typed_into_a_pane(&mut app, &mut rx, 'm'), "the reopened view has the keys");
    }
}

mod files_panel_tests {
    use super::*;
    use super::check_settings_tests::{draw, show};
    use std::path::Path;

    /// A folder with `src/main.rs` (60 numbered lines) and `notes.txt`, unique to `name`.
    fn project(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(concat!(env!("APP_COMMAND"), "-files-{}-{}"), name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("src")).unwrap();
        let body: String = (1..=60).map(|i| format!("let line_{i} = {i};\n")).collect();
        std::fs::write(dir.join("src").join("main.rs"), body).unwrap();
        std::fs::write(dir.join("notes.txt"), "remember the milk\n").unwrap();
        dir
    }

    /// The main screen `w`×`h` with the focused pane running in `dir`.
    fn screen_in(dir: &Path, w: u16, h: u16) -> App {
        let (_, mut app) = super::design_tests::render_with(w, h);
        let pane = app.snap.terms.get_mut(&1).expect("the focused pane");
        pane.cwd = dir.to_path_buf();
        pane.top = Some(dir.to_path_buf());
        pane.root = Some(dir.to_path_buf());
        app
    }

    /// The tree read, as the background scan would hand it back.
    fn tree_read(app: &mut App, root: &Path) {
        app.on_bg(Bg::Tree(root.to_path_buf(), views::scan_tree(root), views::git_marks(root)));
    }

    fn files(app: &App) -> &views::FilesTree {
        match &app.view {
            Some(View::Files(v)) => v,
            _ => panic!("Files is open"),
        }
    }

    fn key(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn select(app: &mut App, name: &str) {
        let Some(View::Files(v)) = &mut app.view else { panic!("Files is open") };
        v.expanded.insert(v.root.join("src"));
        v.sel = v.visible().iter().position(|(_, n)| n.label == name).expect("the file is listed");
        v.refresh_preview();
    }

    #[test]
    fn f_opens_files_in_the_side_panel_beside_the_panes_and_f_or_esc_closes_it() {
        let dir = project("open");
        let mut app = screen_in(&dir, 160, 45);
        let before = draw(&mut app, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert_eq!(screen::open_sheet(&app), Some(screen::SheetKind::Files));
        let sheet = app.screen.sheet_rect;
        assert!(sheet.width > 0 && o.contains("notes.txt"), "the tree is in the panel:\n{o}");
        let (_, pane) = *app.pane_frames.first().expect("a pane still on screen");
        assert!(pane.right() <= sheet.x, "the panes reflow beside it: {pane:?} vs {sheet:?}");
        // Above the mode bar, which now says Files has the keys.
        let bar = screen::mode_bar_row(&app, Rect::new(0, 0, 160, 45)).y as usize;
        let sidebar = |frame: &str| frame.lines().take(bar).map(|l| l.chars().take(20).collect::<String>()).collect::<Vec<_>>();
        assert_eq!(sidebar(&o), sidebar(&before), "the sidebar is not covered");
        key(&mut app, KeyCode::Char('f'));
        assert!(app.view.is_none(), "f again closes it");
        app.act(Action::Files);
        key(&mut app, KeyCode::Esc);
        assert!(app.view.is_none(), "Esc closes it");
        app.act(Action::Files);
        app.act(Action::Files);
        assert!(app.view.is_none(), "the leader's f again closes it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_tree_and_the_selected_file_sit_side_by_side_in_a_wide_panel() {
        let dir = project("wide");
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        let o = draw(&mut app, 160, 45);
        show(&o);
        let row = o.lines().find(|l| l.contains("notes.txt") && l.contains("let line_1 = 1;"));
        assert!(row.is_some(), "the tree's row and the file beside it:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn files_starts_at_55_percent_of_the_screen_and_keeps_the_width_you_give_it() {
        let dir = project("width");
        let mut app = screen_in(&dir, 200, 50);
        assert_eq!(screen::wanted_w(&app, screen::SheetKind::Files, 200), 110);
        app.act(Action::Files);
        draw(&mut app, 200, 50);
        let widest = app.screen.sheet_widest;
        assert_eq!(app.screen.sheet_rect.width, 110.min(widest), "55% of 200, held to the room");
        key(&mut app, KeyCode::Char('['));
        draw(&mut app, 200, 50);
        assert_eq!(app.screen.saved.sheet_w.get("files"), Some(&(110.min(widest) - 4)), "kept per view, like the others");
        assert_eq!(app.screen.sheet_rect.width, 110.min(widest) - 4);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn brackets_are_text_while_the_filter_or_the_editor_takes_it() {
        let dir = project("text");
        let mut app = screen_in(&dir, 200, 50);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        key(&mut app, KeyCode::Char('/'));
        key(&mut app, KeyCode::Char('['));
        assert_eq!(files(&app).filter, "[", "the filter took it");
        key(&mut app, KeyCode::Esc);
        select(&mut app, "notes.txt");
        key(&mut app, KeyCode::Char('i'));
        key(&mut app, KeyCode::Char(']'));
        let ed = files(&app).edit.as_ref().expect("editing in place");
        assert!(ed.lines[0].starts_with(']'), "the editor took it: {:?}", ed.lines[0]);
        assert!(!app.screen.saved.sheet_w.contains_key("files"), "the panel kept its width");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_narrow_panel_gives_the_opened_file_the_whole_panel_with_a_way_back() {
        let dir = project("narrow");
        let mut app = screen_in(&dir, 80, 24);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        let o = draw(&mut app, 80, 24);
        show(&o);
        assert!(app.screen.sheet_rect.width < 60, "{:?}", app.screen.sheet_rect);
        assert!(o.contains("notes.txt") && !o.contains("let line_1"), "the tree alone:\n{o}");
        key(&mut app, KeyCode::Right);
        let o = draw(&mut app, 80, 24);
        show(&o);
        assert!(o.contains("let line_1 = 1;") && o.contains('‹') && !o.contains("notes.txt"), "the file alone, with ‹ back:\n{o}");
        let back = app.hits.iter().rev().find(|(_, h)| *h == Hit::Button(Btn::ViewKey('h'))).map(|(r, _)| *r).expect("‹ is clickable");
        app.on_mouse(MouseEvent { kind: MouseEventKind::Down(MouseButton::Left), column: back.x, row: back.y, modifiers: KeyModifiers::NONE });
        app.on_mouse(MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), column: back.x, row: back.y, modifiers: KeyModifiers::NONE });
        let o = draw(&mut app, 80, 24);
        assert!(o.contains("notes.txt") && !o.contains("let line_1"), "‹ goes back to the tree:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_click_on_a_path_and_line_opens_files_in_the_panel_at_that_line() {
        let dir = project("reveal");
        let mut app = screen_in(&dir, 160, 45);
        app.open_path_from(1, "src/main.rs", Some(40));
        // Outside a repo, the tree is rooted at the file's folder.
        let root = files(&app).root.clone();
        tree_read(&mut app, &root);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert_eq!(screen::open_sheet(&app), Some(screen::SheetKind::Files), "in the panel, not a popup");
        let v = files(&app);
        assert_eq!(v.preview.as_ref().map(|(p, _)| p.clone()), Some(dir.join("src").join("main.rs")));
        assert!(o.contains("let line_40 = 40;"), "that line is shown:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn click(app: &mut App, at: Rect, modifiers: KeyModifiers) {
        for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
            app.on_mouse(MouseEvent { kind, column: at.x, row: at.y, modifiers });
        }
    }

    /// Where a row of the tree is drawn, by its text.
    fn row_at(app: &App, frame: &str, name: &str) -> Rect {
        let y = frame.lines().position(|l| l.contains(name)).expect("the row is drawn") as u16;
        app.hits.iter().rev().find(|(r, _)| r.y == y && r.x >= app.screen.sheet_rect.x).map(|(r, _)| *r).expect("the row is clickable")
    }

    #[test]
    fn ctrl_click_on_a_path_in_a_pane_opens_files_and_again_while_it_is_open() {
        let dir = project("mouse");
        let mut app = screen_in(&dir, 160, 45);
        let mut p = vt100::Parser::new(20, 80, 0);
        p.process(b"see src/main.rs:40\r\nand notes.txt:1\r\n");
        app.parsers.insert(1, p);
        draw(&mut app, 160, 45);
        let (_, inner) = *app.panes.iter().find(|(t, _)| *t == 1).expect("the pane is drawn");
        click(&mut app, Rect { x: inner.x + 6, y: inner.y, width: 1, height: 1 }, KeyModifiers::CONTROL);
        let root = files(&app).root.clone();
        tree_read(&mut app, &root);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("let line_40 = 40;"), "Files opened at that line:\n{o}");
        let (_, inner) = *app.panes.iter().find(|(t, _)| *t == 1).expect("the pane is still beside the panel");
        click(&mut app, Rect { x: inner.x + 6, y: inner.y + 1, width: 1, height: 1 }, KeyModifiers::CONTROL);
        tree_read(&mut app, &dir);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert_eq!(files(&app).preview.as_ref().map(|(p, _)| p.clone()), Some(dir.join("notes.txt")), "retargeted to the second path");
        assert!(o.contains("remember the milk"), "{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn in_a_narrow_panel_a_click_on_the_selected_file_or_a_double_click_opens_it() {
        let dir = project("narrowclick");
        let mut app = screen_in(&dir, 80, 24);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        let o = draw(&mut app, 80, 24);
        let at = row_at(&app, &o, "main.rs");
        click(&mut app, at, KeyModifiers::NONE);
        let o = draw(&mut app, 80, 24);
        assert!(o.contains("let line_1 = 1;") && app.view.is_some(), "the selected file opens in the panel:\n{o}");
        key(&mut app, KeyCode::Left);
        let o = draw(&mut app, 80, 24);
        let at = row_at(&app, &o, "notes.txt");
        click(&mut app, at, KeyModifiers::NONE);
        let o = draw(&mut app, 80, 24);
        let at = row_at(&app, &o, "notes.txt");
        click(&mut app, at, KeyModifiers::NONE);
        let o = draw(&mut app, 80, 24);
        show(&o);
        assert!(o.contains("remember the milk") && app.view.is_some(), "a double-click opens it, not inserts it:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_narrow_file_header_keeps_its_hint_clear_of_the_line_count() {
        let dir = project("header");
        let mut app = screen_in(&dir, 80, 24);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        key(&mut app, KeyCode::Right);
        let o = draw(&mut app, 80, 24);
        let header = o.lines().find(|l| l.contains('‹')).expect("the file's header");
        assert!(header.contains("i edit") && header.contains("1–15 of 60 "), "the tab strip and the mode bar take a row each: {header}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ending_an_edit_in_a_narrow_panel_returns_to_the_file() {
        let dir = project("editback");
        let mut app = screen_in(&dir, 80, 24);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        key(&mut app, KeyCode::Right);
        key(&mut app, KeyCode::Char('i'));
        assert!(files(&app).edit.is_some(), "editing");
        key(&mut app, KeyCode::Esc);
        let o = draw(&mut app, 80, 24);
        assert!(o.contains("let line_1 = 1;") && o.contains('‹') && !o.contains("notes.txt"), "back on the file:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn filtering_from_a_file_in_a_narrow_panel_shows_the_tree() {
        let dir = project("filterback");
        let mut app = screen_in(&dir, 80, 24);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        key(&mut app, KeyCode::Right);
        assert!(files(&app).in_preview, "reading the file");
        key(&mut app, KeyCode::Char('/'));
        assert!(files(&app).filtering && !files(&app).in_preview, "the filter shows the tree, not the file");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn at_100_by_30_the_panel_takes_the_pane_column_with_both_sides() {
        let dir = project("mid");
        let mut app = screen_in(&dir, 100, 30);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        let o = draw(&mut app, 100, 30);
        show(&o);
        assert!(app.panes.is_empty(), "the panel takes the pane column");
        assert!(app.screen.sheet_rect.width >= 60, "{:?}", app.screen.sheet_rect);
        assert!(o.lines().any(|l| l.contains("notes.txt") && l.contains("let line_1 = 1;")), "the tree beside the file:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Every block markdown has, then a list of 60 so it scrolls and has a line 42.
    const GUIDE: &str = "# Guide\n\nSome *odd* and **bold** words with `code`.\n\n## Lists\n\n- one\n  - inner\n\n1. first\n2. second\n\n> quoted words\n\n```\nfn main() {}\n```\n\n| name | n |\n|:--|--:|\n| alpha | 1 |\n| b | 200 |\n\nSee [the docs](https://x.dev).\n\n---\n";

    /// `project` plus `GUIDE.md`.
    fn project_with_guide(name: &str) -> PathBuf {
        let dir = project(name);
        let tail: String = (1..=60).map(|i| format!("- para {i}\n")).collect();
        std::fs::write(dir.join("GUIDE.md"), format!("{GUIDE}\n{tail}")).unwrap();
        dir
    }

    fn guide_open(name: &str) -> (PathBuf, App) {
        let dir = project_with_guide(name);
        let mut app = screen_in(&dir, 200, 50);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "GUIDE.md");
        (dir, app)
    }

    /// The frame as text and its cells, for styles.
    fn frame(app: &mut App, w: u16, h: u16) -> (String, ratatui::buffer::Buffer) {
        app.screen_fresh();
        let mut term = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        let text = (0..h).map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>().trim_end().to_string() + "\n").collect();
        (text, buf)
    }

    /// Where `word` starts inside the preview.
    fn at(app: &App, buf: &ratatui::buffer::Buffer, word: &str) -> (u16, u16) {
        let r = app.screen.preview_rect;
        for y in r.top()..r.bottom() {
            let row: String = (r.left()..r.right()).map(|x| buf[(x, y)].symbol().to_string()).collect();
            if let Some(i) = row.find(word) {
                return (r.left() + row[..i].chars().count() as u16, y);
            }
        }
        panic!("{word} is in the preview")
    }

    fn cell_of<'b>(app: &App, buf: &'b ratatui::buffer::Buffer, word: &str) -> &'b ratatui::buffer::Cell {
        &buf[at(app, buf, word)]
    }

    fn hints(frame: &str) -> String {
        frame.lines().find(|l| l.contains("Insert path into")).map(str::to_string).expect("the key hints")
    }

    #[test]
    fn a_markdown_file_previews_pretty_by_default() {
        let (dir, mut app) = guide_open("mdpretty");
        let (o, buf) = frame(&mut app, 200, 50);
        show(&o);
        use ratatui::style::Modifier;
        let t = app.theme.clone();
        let heading = cell_of(&app, &buf, "Guide");
        assert!(heading.fg == t.strong && heading.modifier.contains(Modifier::BOLD), "the heading is bold and strong");
        assert!(!o.contains("# Guide") && !o.contains("**bold**") && !o.contains("```"), "no markup:\n{o}");
        assert!(o.lines().skip_while(|l| !l.contains("Guide")).nth(1).is_some_and(|l| l.contains("────────")), "a rule under the title:\n{o}");
        assert!(cell_of(&app, &buf, "odd").modifier.contains(Modifier::ITALIC), "emphasis");
        assert_eq!(cell_of(&app, &buf, "code").bg, t.card2, "inline code");
        assert!(o.contains("• one") && o.contains("  • inner") && o.contains("1. first") && o.contains("2. second"), "lists:\n{o}");
        let bar = cell_of(&app, &buf, "│ quoted words");
        assert_eq!(bar.fg, t.muted, "the quote's bar is muted");
        assert_eq!(cell_of(&app, &buf, "fn main()").bg, t.card, "fenced code is on the raised ground");
        assert!(o.contains("name  │   n") && o.contains("alpha │   1") && o.contains("b     │ 200"), "the table's columns line up:\n{o}");
        assert!(o.contains("the docs (https://x.dev)"), "the link's text and its url:\n{o}");
        assert_eq!(cell_of(&app, &buf, "(https://x.dev)").fg, t.muted, "the url is muted");
        assert_eq!(at(&app, &buf, "Guide").0, app.screen.preview_rect.x, "no line numbers on drawn lines:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn m_toggles_a_markdown_file_between_pretty_and_raw_and_the_hints_say_which() {
        let (dir, mut app) = guide_open("mdtoggle");
        let (o, _) = frame(&mut app, 200, 50);
        assert!(hints(&o).contains(" m  Raw"), "the key hint offers raw:\n{}", hints(&o));
        key(&mut app, KeyCode::Char('m'));
        let (o, _) = frame(&mut app, 200, 50);
        show(&o);
        assert!(o.contains("   1  # Guide") && o.contains("**bold**"), "the text as it is, numbered:\n{o}");
        assert!(hints(&o).contains(" m  Pretty"), "the key hint offers pretty:\n{}", hints(&o));
        key(&mut app, KeyCode::Right);
        key(&mut app, KeyCode::Char('m'));
        let (o, _) = frame(&mut app, 200, 50);
        assert!(!o.contains("# Guide") && files(&app).in_preview, "m in the preview draws it again, still reading it:\n{o}");
        select(&mut app, "notes.txt");
        let (o, _) = frame(&mut app, 200, 50);
        assert!(!hints(&o).contains(" m "), "only a markdown file has the hint:\n{}", hints(&o));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_markdown_file_edits_as_its_text_and_scrolls_both_ways() {
        let (dir, mut app) = guide_open("mdedit");
        frame(&mut app, 200, 50);
        key(&mut app, KeyCode::PageDown);
        let (o, _) = frame(&mut app, 200, 50);
        assert!(files(&app).scroll > 0 && !o.contains("Guide") && o.contains("• para"), "the drawn file pages down:\n{o}");
        key(&mut app, KeyCode::Char('i'));
        let ed = files(&app).edit.as_ref().expect("editing in place");
        assert_eq!(ed.lines[0], "# Guide", "the editor has the text, not the drawing");
        let (o, _) = frame(&mut app, 200, 50);
        assert!(o.contains("   1  # Guide"), "from the top:\n{o}");
        key(&mut app, KeyCode::Esc);
        let (o, _) = frame(&mut app, 200, 50);
        assert!(files(&app).scroll == 0 && o.contains("Guide") && !o.contains("# Guide"), "the edit's row isn't taken for a drawn line:\n{o}");
        key(&mut app, KeyCode::Char('m'));
        key(&mut app, KeyCode::PageDown);
        let (o, _) = frame(&mut app, 200, 50);
        assert!(files(&app).scroll > 0 && !o.contains("# Guide"), "the raw text pages down too:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn m_keeps_about_the_same_place_in_the_file() {
        let (dir, mut app) = guide_open("mdplace");
        frame(&mut app, 200, 50);
        key(&mut app, KeyCode::PageDown);
        key(&mut app, KeyCode::Char('m'));
        let (o, _) = frame(&mut app, 200, 50);
        assert!(files(&app).scroll > 0 && !o.contains("# Guide") && o.contains("- para"), "still down in the list, now as text:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn drawn_markdown_is_laid_out_once_and_again_for_a_new_width_or_file() {
        let (dir, mut app) = guide_open("mdcache");
        frame(&mut app, 200, 50);
        // A line the renderer never makes: still there means the last layout was reused.
        let mark = |app: &mut App| {
            let Some(View::Files(v)) = &mut app.view else { panic!("Files is open") };
            v.drawn_cache.as_mut().expect("laid out").lines.insert(0, vec![design::seg("SENTINEL", ratatui::style::Style::default())]);
        };
        mark(&mut app);
        key(&mut app, KeyCode::PageDown);
        key(&mut app, KeyCode::PageUp);
        let (o, _) = frame(&mut app, 200, 50);
        assert!(o.contains("SENTINEL"), "the same file, width and theme reuse it:\n{o}");
        let (o, _) = frame(&mut app, 180, 50);
        assert!(!o.contains("SENTINEL"), "a new width lays it out again:\n{o}");
        mark(&mut app);
        select(&mut app, "notes.txt");
        select(&mut app, "GUIDE.md");
        let (o, _) = frame(&mut app, 180, 50);
        assert!(!o.contains("SENTINEL") && o.contains("Guide"), "a new preview lays it out again:\n{o}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn e_opens_a_markdown_file_in_your_editor() {
        let (dir, mut app) = guide_open("mdeditor");
        // A terminal editor opens in a new pane: a message to the daemon, not a process here.
        app.cfg.editor = "nvim".into();
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        app.out = tx;
        key(&mut app, KeyCode::Char('e'));
        let mut opened = None;
        while let Ok(msg) = rx.try_recv() {
            if let crate::protocol::ClientMsg::Command(crate::protocol::Command::NewWorkspace { cmd, .. }) = msg {
                opened = cmd;
            }
        }
        assert!(opened.as_ref().is_some_and(|c| c.starts_with("nvim") && c.contains("GUIDE.md")), "{opened:?}");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_click_on_a_markdown_path_and_line_opens_it_raw_at_that_line() {
        let dir = project_with_guide("mdreveal");
        let mut app = screen_in(&dir, 160, 45);
        app.open_path_from(1, "GUIDE.md", Some(42));
        let root = files(&app).root.clone();
        tree_read(&mut app, &root);
        let (o, _) = frame(&mut app, 160, 45);
        show(&o);
        let source = std::fs::read_to_string(dir.join("GUIDE.md")).unwrap();
        let want = format!("{:>4}  {}", 42, source.lines().nth(41).unwrap());
        assert!(want.contains("- para"), "line 42 is a line of the list: {want:?}");
        assert!(o.contains(&want), "line 42 is shown with its number:\n{o}");
        assert!(!files(&app).drawn(), "raw, since drawn lines have no numbers");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// `src/main.rs` open in the editor at 160×45, drawn once so the preview has its place.
    fn editing(name: &str) -> (PathBuf, App) {
        let dir = project(name);
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        key(&mut app, KeyCode::Char('i'));
        frame(&mut app, 160, 45);
        (dir, app)
    }

    fn edit(app: &App) -> &views::Edit {
        files(app).edit.as_ref().expect("editing")
    }

    fn press(app: &mut App, code: KeyCode, mods: KeyModifiers) {
        app.on_key(KeyEvent::new(code, mods));
    }

    fn mouse(app: &mut App, kind: MouseEventKind, (column, row): (u16, u16)) {
        app.on_mouse(MouseEvent { kind, column, row, modifiers: KeyModifiers::NONE });
    }

    #[test]
    fn shift_and_the_arrows_select_in_the_editor_and_typing_replaces_it() {
        let (dir, mut app) = editing("shiftsel");
        for _ in 0..3 {
            press(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
        }
        assert_eq!(edit(&app).selection(), Some(((0, 0), (0, 3))), "three characters");
        let (o, buf) = frame(&mut app, 160, 45);
        show(&o);
        let selected = render::select_style(&app.theme).bg.expect("a ground");
        assert_eq!(cell_of(&app, &buf, "let line_1").bg, selected, "drawn in the selection colour");
        assert_ne!(cell_of(&app, &buf, "line_1").bg, selected, "only what is selected");
        press(&mut app, KeyCode::Char('X'), KeyModifiers::SHIFT);
        assert_eq!(edit(&app).lines[0], "X line_1 = 1;", "typing replaces the selection");
        assert_eq!(edit(&app).selection(), None);
        press(&mut app, KeyCode::Down, KeyModifiers::SHIFT);
        press(&mut app, KeyCode::End, KeyModifiers::SHIFT);
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        assert_eq!(edit(&app).lines[..2], ["X", "let line_3 = 3;"], "Backspace removes a selection across lines");
        press(&mut app, KeyCode::Left, KeyModifiers::SHIFT);
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        assert_eq!(edit(&app).selection(), None, "a move without Shift drops it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_a_selects_the_whole_file() {
        let (dir, mut app) = editing("selall");
        press(&mut app, KeyCode::Char('a'), KeyModifiers::CONTROL);
        assert_eq!(edit(&app).selection(), Some(((0, 0), (59, "let line_60 = 60;".len()))));
        press(&mut app, KeyCode::Char('z'), KeyModifiers::NONE);
        assert_eq!(edit(&app).lines, ["z"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_mouse_selects_in_the_editor_and_a_click_moves_the_cursor() {
        let (dir, mut app) = editing("mousesel");
        let (_, buf) = frame(&mut app, 160, 45);
        let (from, to) = (at(&app, &buf, "line_1"), at(&app, &buf, "= 3;"));
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), to);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), to);
        assert_eq!(edit(&app).selection(), Some(((0, 4), (2, 11))), "dragged from line 1 to line 3");
        let click = at(&app, &buf, "line_2");
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), click);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), click);
        assert_eq!((edit(&app).row, edit(&app).col, edit(&app).selection()), (1, 4, None), "a click moves the cursor");
        let word = at(&app, &buf, "line_4");
        for _ in 0..2 {
            mouse(&mut app, MouseEventKind::Down(MouseButton::Left), word);
            mouse(&mut app, MouseEventKind::Up(MouseButton::Left), word);
        }
        assert_eq!(edit(&app).selection(), Some(((3, 4), (3, 10))), "a double-click takes the word");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn last_copy() -> Option<String> {
        copy::LAST_COPY.with(|c| c.borrow().clone())
    }

    /// Ctrl+V, and the clipboard read it starts in the background handed back.
    fn ctrl_v(app: &mut App, rx: &mut mpsc::UnboundedReceiver<Bg>) {
        press(app, KeyCode::Char('v'), KeyModifiers::CONTROL);
        let read = super::branch_view_tests::next_bg(rx);
        app.on_bg(read);
    }

    #[test]
    fn copy_cut_and_paste_round_trip_in_the_editor() {
        let (dir, mut app) = editing("clip");
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.bg = tx;
        for _ in 0..3 {
            press(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
        }
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(last_copy().as_deref(), Some("let"), "Ctrl+C copies the selection");
        press(&mut app, KeyCode::End, KeyModifiers::NONE);
        ctrl_v(&mut app, &mut rx);
        assert_eq!(edit(&app).lines[0], "let line_1 = 1;let", "Ctrl+V pastes at the cursor");
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert_eq!(last_copy().as_deref(), Some("let line_2 = 2;\n"), "Ctrl+X with nothing selected cuts the line");
        assert_eq!(edit(&app).lines[1], "let line_3 = 3;");
        press(&mut app, KeyCode::Home, KeyModifiers::NONE);
        ctrl_v(&mut app, &mut rx);
        assert_eq!(edit(&app).lines[1..3], ["let line_2 = 2;", "let line_3 = 3;"], "the line comes back whole");
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(last_copy().as_deref(), Some("let line_3 = 3;\n"), "Ctrl+C with nothing selected copies the line");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_terminals_paste_goes_into_the_editor_as_text() {
        let (dir, mut app) = editing("bracketed");
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        for _ in 0..3 {
            press(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
        }
        app.on_event(Event::Paste("one\r\ntwo".into()));
        assert_eq!(edit(&app).lines[..2], ["one", "two line_1 = 1;"], "the paste replaces the selection, its lines kept");
        assert_eq!((edit(&app).row, edit(&app).col), (1, 3), "the cursor after it");
        app.on_event(Event::Paste(String::new()));
        assert_eq!(app.notice.as_ref().map(|(m, _, err)| (m.as_str(), *err)), Some((copy::NO_TEXT, true)), "an empty paste is no image paste");
        assert!(out.try_recv().is_err(), "nothing reached the pane");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_c_in_the_editor_never_reaches_the_pane() {
        let (dir, mut app) = editing("noint");
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        for code in ['c', 'x', 'v', 'a'] {
            press(&mut app, KeyCode::Char(code), KeyModifiers::CONTROL);
        }
        while let Ok(msg) = out.try_recv() {
            assert!(!matches!(msg, ClientMsg::Input { .. }), "a key went to the agent: {msg:?}");
        }
        assert!(files(&app).edit.is_some(), "still editing");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn dragging_over_the_shown_file_selects_it_and_ctrl_c_copies_it() {
        let dir = project("readsel");
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "main.rs");
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        let (_, buf) = frame(&mut app, 160, 45);
        let (from, to) = (at(&app, &buf, "line_1"), at(&app, &buf, "= 2;"));
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), to);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), to);
        let (o, buf) = frame(&mut app, 160, 45);
        show(&o);
        let selected = render::select_style(&app.theme).bg.expect("a ground");
        assert_eq!(cell_of(&app, &buf, "line_1").bg, selected, "drawn in the selection colour");
        assert_ne!(cell_of(&app, &buf, "= 2;").bg, selected, "it ends where the drag let go");
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(last_copy().as_deref(), Some("line_1 = 1;\nlet line_2 "), "Ctrl+C copies the selection");
        assert!(out.try_recv().is_err(), "Ctrl+C reached no pane");
        key(&mut app, KeyCode::Char('y'));
        assert_eq!(last_copy(), Some(dir.join("src").join("main.rs").display().to_string()), "y still copies the path");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_z_undoes_a_run_of_typing_at_once_and_ctrl_y_redoes_it() {
        let (dir, mut app) = editing("undo");
        for c in "abc".chars() {
            press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        press(&mut app, KeyCode::End, KeyModifiers::NONE);
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        press(&mut app, KeyCode::Backspace, KeyModifiers::NONE);
        app.on_event(Event::Paste("!".into()));
        assert_eq!(edit(&app).lines[0], "abclet line_1 = !");
        let undo = |app: &mut App| press(app, KeyCode::Char('z'), KeyModifiers::CONTROL);
        undo(&mut app);
        assert_eq!(edit(&app).lines[0], "abclet line_1 = ", "the paste is a step of its own");
        undo(&mut app);
        assert_eq!(edit(&app).lines[0], "abclet line_1 = 1;", "both Backspaces at once");
        undo(&mut app);
        assert_eq!(edit(&app).lines[0], "let line_1 = 1;", "the whole word typed");
        undo(&mut app);
        assert_eq!(app.notice.as_ref().map(|(m, _, _)| m.as_str()), Some("nothing to undo"));
        press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(edit(&app).lines[0], "abclet line_1 = 1;", "Ctrl+Y puts it back");
        press(&mut app, KeyCode::Char('q'), KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('y'), KeyModifiers::CONTROL);
        assert_eq!(app.notice.as_ref().map(|(m, _, _)| m.as_str()), Some("nothing to redo"), "a new change forgets what was undone");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn click_hit(app: &mut App, want: Hit) {
        let r = app.hits.iter().find_map(|(r, h)| (*h == want).then_some(*r)).expect("the button is drawn");
        for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
            mouse(app, kind, (r.x + 1, r.y));
        }
    }

    #[test]
    fn the_editor_has_its_own_buttons_and_they_type_nothing() {
        let (dir, mut app) = editing("buttons");
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.bg = tx;
        let (o, _) = frame(&mut app, 160, 45);
        show(&o);
        let row = o.lines().find(|l| l.contains("Save")).expect("the action row");
        for label in ["^S", "Save", "^C", "Copy", "^V", "Paste", "^Z", "Undo", "Esc", "Done"] {
            assert!(row.contains(label), "{label} in {row}");
        }
        assert!(!o.contains("Edit here") && !o.contains("Insert path"), "no file-browser buttons while editing:\n{o}");
        let ctrl = |c: char| Hit::Button(Btn::ViewChord(KeyCode::Char(c), KeyModifiers::CONTROL));
        click_hit(&mut app, ctrl('c'));
        assert_eq!(last_copy().as_deref(), Some("let line_1 = 1;\n"), "Copy copies");
        assert_eq!(edit(&app).lines[0], "let line_1 = 1;", "and types nothing");
        click_hit(&mut app, ctrl('v'));
        let read = super::branch_view_tests::next_bg(&mut rx);
        app.on_bg(read);
        assert_eq!(edit(&app).lines[..2], ["let line_1 = 1;", "let line_1 = 1;"], "Paste pastes");
        click_hit(&mut app, ctrl('z'));
        assert_eq!(edit(&app).lines[..2], ["let line_1 = 1;", "let line_2 = 2;"], "Undo undoes");
        assert!(edit(&app).dirty, "an undo marks the file changed");
        click_hit(&mut app, ctrl('s'));
        assert!(!edit(&app).dirty, "Save saves");
        assert_eq!(std::fs::read_to_string(dir.join("src").join("main.rs")).unwrap().lines().next(), Some("let line_1 = 1;"));
        click_hit(&mut app, Hit::Button(Btn::ViewChord(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(files(&app).edit.is_none(), "Done leaves the editor");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// What the next message to the server types into a pane, if anything.
    fn typed(out: &mut mpsc::UnboundedReceiver<ClientMsg>) -> Option<String> {
        std::iter::from_fn(|| out.try_recv().ok()).find_map(|m| match m {
            ClientMsg::Input { data, .. } => Some(String::from_utf8_lossy(&data).into_owned()),
            _ => None,
        })
    }

    /// An empty paste started an image paste: its background read came back and said so.
    fn image_paste_tried(app: &mut App, rx: &mut mpsc::UnboundedReceiver<Bg>) -> bool {
        let read = super::branch_view_tests::next_bg(rx);
        app.on_bg(read);
        app.notice.as_ref().is_some_and(|(m, _, _)| m == "there's no image on the clipboard")
    }

    #[test]
    fn pastes_still_reach_the_pane_whenever_files_does_not_have_the_keys() {
        let (dir, mut app) = editing("pastepane");
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        let (btx, mut bg) = mpsc::unbounded_channel();
        app.bg = btx;
        let pane = app.pane_frames.iter().find(|(t, _)| *t == 1).map(|(_, r)| *r).expect("the pane beside the panel");
        let inside = (pane.x + pane.width / 2, pane.y + pane.height / 2);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), inside);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), inside);
        assert!(!app.view_has_keys(), "the click set the editor aside");
        app.on_event(Event::Paste("x".into()));
        assert_eq!(typed(&mut out).as_deref(), Some("x"), "set aside: the pane gets it");
        app.on_event(Event::Paste(String::new()));
        assert!(image_paste_tried(&mut app, &mut bg), "set aside: an empty paste is the pane's image paste");
        click_hit(&mut app, Hit::Button(Btn::ViewChord(KeyCode::Esc, KeyModifiers::NONE)));
        assert!(app.view_has_keys() && files(&app).edit.is_none(), "Esc left the editor; Files has the keys");
        app.on_event(Event::Paste("x".into()));
        assert_eq!(typed(&mut out), None, "Files with the keys keeps a paste from the pane");
        key(&mut app, KeyCode::Esc);
        assert!(app.view.is_none(), "Files closed");
        app.on_event(Event::Paste("x".into()));
        assert_eq!(typed(&mut out).as_deref(), Some("x"), "closed: the pane gets it");
        app.on_event(Event::Paste(String::new()));
        assert!(image_paste_tried(&mut app, &mut bg), "closed: an empty paste is the image paste");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_paste_while_files_filters_goes_into_the_filter() {
        let dir = project("pastefilter");
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        key(&mut app, KeyCode::Char('/'));
        app.on_event(Event::Paste("notes\nsecond line".into()));
        assert_eq!(files(&app).filter, "notes", "its first line");
        assert_eq!(files(&app).preview.as_ref().map(|(p, _)| p.clone()), Some(dir.join("notes.txt")), "the match is shown");
        app.on_event(Event::Paste(String::new()));
        assert_eq!(typed(&mut out), None, "nothing reached the pane");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Keys as the Windows console hands them over in one read: a press and a release each.
    pub(super) fn burst(s: &str) -> Vec<Event> {
        s.chars()
            .flat_map(|c| {
                let press = KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE);
                [Event::Key(press), Event::Key(KeyEvent { kind: ratatui::crossterm::event::KeyEventKind::Release, ..press })]
            })
            .collect()
    }

    #[test]
    fn a_short_paste_read_as_keys_runs_no_files_command() {
        let dir = project("shortpaste");
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "notes.txt");
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        // `r/` rather than the `eyo` of the report: run as keys, those would launch an editor
        // and an app and write the clipboard; these only change Files.
        app.on_burst(burst("r/"));
        assert!(app.notice.as_ref().is_some_and(|(m, _, _)| m.starts_with("Files has the keys")), "taken as a paste");
        assert!(!files(&app).recent && !files(&app).filtering, "not run as r and /");
        assert_eq!(typed(&mut out), None, "nothing reached the pane");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_held_key_read_in_one_burst_still_moves_in_files() {
        let dir = project("heldkey");
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "src");
        let from = files(&app).sel;
        app.on_burst(burst("jj"));
        assert_eq!(files(&app).sel, from + 2, "two moves down, not a paste");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_release_outside_the_window_ends_the_selection_drag() {
        let (dir, mut app) = editing("staledrag");
        let (_, buf) = frame(&mut app, 160, 45);
        let from = at(&app, &buf, "line_1");
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        assert!(files(&app).dragging);
        mouse(&mut app, MouseEventKind::Moved, (0, 0));
        assert!(!files(&app).dragging, "moving with no button held ends it");
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), (0, 44));
        assert!(!files(&app).dragging, "a press elsewhere ends it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn below_the_last_line_the_pointer_means_the_end_of_the_file() {
        let dir = project("pastend");
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "notes.txt");
        key(&mut app, KeyCode::Char('i'));
        let (_, buf) = frame(&mut app, 160, 45);
        let below = (app.screen.preview_rect.x + 2, app.screen.preview_rect.y + 5);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), below);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), below);
        assert_eq!((edit(&app).row, edit(&app).col), (0, "remember the milk".len()), "a click below a short file: its end");
        let word = at(&app, &buf, "the milk");
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), word);
        let past = (word.0, app.screen.preview_rect.bottom() + 1);
        mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), past);
        assert_eq!(edit(&app).selection(), Some(((0, 9), (0, 17))), "a drag past the end takes the rest of the last line");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn undo_treats_typing_over_a_selection_and_a_line_cut_as_one_step_each() {
        let (dir, mut app) = editing("undosteps");
        for _ in 0..3 {
            press(&mut app, KeyCode::Right, KeyModifiers::SHIFT);
        }
        for c in "var".chars() {
            press(&mut app, KeyCode::Char(c), KeyModifiers::NONE);
        }
        assert_eq!(edit(&app).lines[0], "var line_1 = 1;");
        press(&mut app, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(edit(&app).lines[0], "let line_1 = 1;", "one Ctrl+Z undoes the replacing and the typing");
        press(&mut app, KeyCode::Down, KeyModifiers::NONE);
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        press(&mut app, KeyCode::Right, KeyModifiers::NONE);
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        press(&mut app, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!((edit(&app).lines[1].as_str(), edit(&app).row, edit(&app).col), ("let line_2 = 2;", 1, 5), "the line back, the cursor where it was");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ctrl_x_in_an_empty_file_does_nothing() {
        let dir = project("emptycut");
        std::fs::write(dir.join("empty.txt"), "").unwrap();
        let mut app = screen_in(&dir, 160, 45);
        app.act(Action::Files);
        tree_read(&mut app, &dir);
        select(&mut app, "empty.txt");
        key(&mut app, KeyCode::Char('i'));
        app.notice = None;
        press(&mut app, KeyCode::Char('x'), KeyModifiers::CONTROL);
        assert!(app.notice.is_none() && !edit(&app).dirty, "no cut, no toast");
        press(&mut app, KeyCode::Char('z'), KeyModifiers::CONTROL);
        assert_eq!(app.notice.as_ref().map(|(m, _, _)| m.as_str()), Some("nothing to undo"), "and no undo step");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_double_click_on_drawn_markdown_takes_a_word() {
        let (dir, mut app) = guide_open("mdword");
        let (_, buf) = frame(&mut app, 200, 50);
        let word = at(&app, &buf, "para 7");
        for _ in 0..2 {
            mouse(&mut app, MouseEventKind::Down(MouseButton::Left), word);
            mouse(&mut app, MouseEventKind::Up(MouseButton::Left), word);
        }
        press(&mut app, KeyCode::Char('c'), KeyModifiers::CONTROL);
        assert_eq!(last_copy().as_deref(), Some("para"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}


mod branch_view_tests {
    use super::*;
    use super::check_settings_tests::{draw, show};
    use crate::client::menu::Act;
    use crate::protocol::{CheckInfo, CheckState};

    fn leader(app: &mut App, c: char) {
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    fn key(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn sheet(app: &App) -> Option<&screen::BranchSheet> {
        match &app.view {
            Some(View::Branch(v)) => Some(v),
            _ => None,
        }
    }

    /// A repo on `main` with a commit, on a feature branch one commit ahead, with a snapshot.
    fn repo(tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{APP_COMMAND}-branch-view-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let git = |args: &[&str]| crate::proc::git(&dir, args).unwrap();
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(dir.join("a.txt"), "1").unwrap();
        git(&["add", "."]);
        git(&["commit", "-qm", "start the shop"]);
        git(&["switch", "-qc", "fix-footer"]);
        std::fs::write(dir.join("a.txt"), "2").unwrap();
        git(&["commit", "-qam", "fix the footer links"]);
        std::fs::write(dir.join("a.txt"), "3").unwrap();
        crate::snapshot::take(&dir, "before claude's turn").unwrap();
        dir
    }

    /// Background work the app queues comes here instead of being dropped.
    fn catch_bg(app: &mut App) -> mpsc::UnboundedReceiver<Bg> {
        let (tx, rx) = mpsc::unbounded_channel();
        app.bg = tx;
        rx
    }

    /// The next background result, once its thread is done.
    pub(super) fn next_bg(rx: &mut mpsc::UnboundedReceiver<Bg>) -> Bg {
        let until = Instant::now() + std::time::Duration::from_secs(20);
        loop {
            if let Ok(b) = rx.try_recv() {
                return b;
            }
            assert!(Instant::now() < until, "no background work was queued");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// The app with session 1 in `dir` and its Branch view open, git's answer landed from the
    /// background task.
    fn opened(dir: &std::path::Path, w: u16, h: u16) -> (App, mpsc::UnboundedReceiver<Bg>) {
        let (_, mut app) = super::design_tests::render_with(w, h);
        let mut rx = catch_bg(&mut app);
        app.snap.terms.get_mut(&1).unwrap().top = Some(dir.to_path_buf());
        leader(&mut app, 'b');
        assert!(sheet(&app).is_some_and(|v| v.dir == dir && v.facts.is_none()), "b opens the Branch view, reading git");
        let b = next_bg(&mut rx);
        app.on_bg(b);
        assert!(matches!(sheet(&app).and_then(|v| v.facts.as_ref()), Some(screen::Facts::Repo(_))), "git's answer landed");
        (app, rx)
    }

    #[test]
    fn b_opens_branch_in_the_side_panel_and_b_or_esc_closes_it() {
        let (_, mut app) = super::design_tests::render_with(200, 50);
        leader(&mut app, 'b');
        assert!(sheet(&app).is_some() && app.mode == Mode::Normal, "the view has the keys: {:?}", app.mode);
        let o = draw(&mut app, 200, 50);
        show(&o);
        assert!(o.contains("BRANCH") && o.contains("Switch branch…"), "in the side panel");
        assert_eq!(app.screen.sheet_rect.width, 60, "30% of the screen");
        key(&mut app, KeyCode::Char(']'));
        assert_eq!(app.screen.saved.sheet_w.get("branch"), Some(&64), "its width is remembered as Branch's");
        key(&mut app, KeyCode::Char('b'));
        assert!(sheet(&app).is_none() && matches!(app.mode, Mode::Prefix { .. }), "b closes it, back to shortcut mode");
        key(&mut app, KeyCode::Char('b'));
        assert!(sheet(&app).is_some(), "the leader's b opens it again");
        key(&mut app, KeyCode::Esc);
        assert!(sheet(&app).is_none() && matches!(app.mode, Mode::Prefix { .. }), "Esc closes it, back to shortcut mode");
    }

    #[test]
    fn opened_from_shortcut_mode_its_keys_reach_it_and_k_types_nothing() {
        let (mut app, mut out) = super::check_settings_tests::app_with_channel();
        leader(&mut app, 'b');
        while out.try_recv().is_ok() {}
        // k is the leader's stop-the-agent: here it moves up, and nothing goes to a pane.
        key(&mut app, KeyCode::Char('k'));
        let sent: Vec<ClientMsg> = std::iter::from_fn(|| out.try_recv().ok()).collect();
        assert!(!sent.iter().any(|m| matches!(m, ClientMsg::Input { .. })), "nothing typed into a pane: {sent:?}");
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Enter);
        assert!(matches!(app.mode, Mode::Branch(_)), "Enter on Switch branch… opens the switcher: {:?}", std::mem::discriminant(&app.mode));
    }

    #[test]
    fn an_answer_for_another_folder_is_dropped_and_a_switch_or_restore_reads_again() {
        let dir = repo("reload");
        let (mut app, mut rx) = opened(&dir, 160, 45);
        let before = sheet(&app).unwrap().facts.clone();
        app.branch_facts_arrived(&dir.join("elsewhere"), screen::Facts::NotGit);
        assert_eq!(sheet(&app).unwrap().facts, before, "not this view's folder");
        app.on_bg(Bg::Switched(Ok("now on main".into())));
        let b = next_bg(&mut rx);
        app.on_bg(b);
        let Some(screen::Facts::Repo(r)) = &sheet(&app).unwrap().facts else { panic!("a repo") };
        assert_eq!(r.branch, "fix-footer", "read again after the switch");
        let s = r.snapshots[0].clone();
        std::fs::write(dir.join("a.txt"), "4").unwrap();
        app.menu_do(Act::Restore(dir.clone(), s.commit, "now".into(), 1));
        let restored = next_bg(&mut rx);
        app.on_bg(restored);
        let b = next_bg(&mut rx);
        app.on_bg(b);
        let Some(screen::Facts::Repo(r)) = &sheet(&app).unwrap().facts else { panic!("a repo") };
        assert_eq!(r.snapshots.len(), 2, "read again after the restore: it kept one before it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn it_shows_the_branch_its_gaps_commits_check_and_snapshots() {
        let dir = repo("shows");
        let (mut app, _rx) = opened(&dir, 160, 45);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        app.snap.terms.get_mut(&1).unwrap().checks =
            vec![CheckInfo { name: "test".into(), command: "npm test".into(), state: CheckState::Failed, exit_code: Some(1), started_at: now - 60, ..Default::default() }];
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("Branch    fix-footer"), "the branch");
        assert!(o.contains("Upstream  none"), "no upstream yet");
        assert!(o.contains("Base      main  ↑1 ↓0"), "one commit ahead of main");
        assert!(o.contains("fix the footer links") && o.contains("start the shop"), "the commits");
        let Some(screen::Facts::Repo(r)) = &sheet(&app).unwrap().facts else { panic!("a repo") };
        assert!(o.contains(&r.commits.as_ref().unwrap()[0].hash), "with their short hash");
        assert!(o.contains("Check     failed: test · exit 1"), "the last check");
        assert!(o.contains("SNAPSHOTS") && o.contains("ago · before claude's turn"), "the snapshots");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn enter_on_its_first_row_opens_the_switcher_and_on_a_snapshot_asks_to_restore() {
        let dir = repo("enter");
        let (mut app, _rx) = opened(&dir, 160, 45);
        draw(&mut app, 160, 45);
        key(&mut app, KeyCode::Enter);
        assert!(matches!(&app.mode, Mode::Branch(v) if v.dir == dir), "the switcher: {:?}", app.mode);
        key(&mut app, KeyCode::Esc);
        assert!(app.mode == Mode::Normal && sheet(&app).is_some(), "Esc leaves the switcher for the Branch view");
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Enter);
        let Mode::Confirm(c) = &app.mode else { panic!("a restore asks first: {:?}", app.mode) };
        let Act::Restore(at, _, when, term) = &c.act else { panic!("{:?}", c.act) };
        assert!(at == &dir && *term == 1 && when.ends_with("ago · before claude's turn"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_folder_outside_git_says_so() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        leader(&mut app, 'b');
        let dir = sheet(&app).unwrap().dir.clone();
        app.branch_facts_arrived(&dir, screen::Facts::NotGit);
        let o = draw(&mut app, 160, 45);
        show(&o);
        assert!(o.contains("isn't in a git repository."), "it says so");
        assert!(!o.contains("Switch branch…") && !o.contains("COMMITS") && !o.contains("SNAPSHOTS"), "instead of the lists");
        key(&mut app, KeyCode::Enter);
        assert_eq!(app.mode, Mode::Normal, "Enter has nothing to open");
    }
}

mod leader_key_tests {
    use super::*;

    fn leader(app: &mut App, code: KeyCode, mods: KeyModifiers) {
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app.on_key(KeyEvent::new(code, mods));
    }

    /// A fresh git repo the focused pane runs in.
    fn in_repo(app: &mut App, tag: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{APP_COMMAND}-leader-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::proc::git(&dir, &["init", "-q"]).unwrap();
        let pane = app.snap.terms.get_mut(&1).expect("the focused pane");
        pane.cwd = dir.clone();
        pane.top = Some(dir.clone());
        dir
    }

    #[test]
    fn leader_c_opens_changes() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        let dir = in_repo(&mut app, "c");
        leader(&mut app, KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(matches!(app.view, Some(View::Changes(_))), "c opens Changes: {:?}", app.mode);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn leader_question_mark_opens_all_keys_however_shift_is_reported() {
        for mods in [KeyModifiers::SHIFT, KeyModifiers::NONE] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            leader(&mut app, KeyCode::Char('?'), mods);
            assert!(matches!(&app.mode, Mode::KeyMap(km) if km.view == screen::KeyView::All), "{mods:?}: {:?}", app.mode);
        }
    }

    #[test]
    fn f1_and_d_do_nothing_after_the_leader_by_default() {
        for code in [KeyCode::F(1), KeyCode::Char('d')] {
            let (_, mut app) = super::design_tests::render_with(160, 45);
            leader(&mut app, code, KeyModifiers::NONE);
            assert!(app.view.is_none() && matches!(app.mode, Mode::Prefix { .. }), "{code:?} is unbound: {:?}", app.mode);
        }
    }

    #[test]
    fn copy_mode_has_no_key_and_search_commands_finds_it() {
        let (_, app) = super::design_tests::render_with(160, 45);
        assert!(!app.keymap.prefixed.values().any(|a| *a == Action::CopyMode), "no default key");
        let km = screen::KeyMap { query: "select text".into(), ..screen::KeyMap::new(screen::KeyView::Search, None) };
        assert!(screen::keymap_actions(&app, &km).contains(&Some(Action::CopyMode)), "Search commands lists it");
    }

    #[test]
    fn an_old_config_binding_keys_by_action_name_still_loads() {
        let (_, mut app) = super::design_tests::render_with(160, 45);
        for (key, action) in [("d", "changes"), ("f1", "help"), ("y", "copy-mode")] {
            app.cfg.keys.prefix.insert(key.into(), action.into());
        }
        app.keymap = app.cfg.keymap();
        let bound = |k: &str| app.keymap.prefixed.get(&k.parse().unwrap()).cloned();
        assert_eq!(bound("d"), Some(Action::Changes));
        assert_eq!(bound("f1"), Some(Action::Help));
        assert_eq!(bound("y"), Some(Action::CopyMode));
    }
}

mod drawer_tab_tests {
    use super::*;
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::buffer::Buffer;

    const W: u16 = 200;
    const H: u16 = 50;

    /// The main screen with the focused pane in a fresh git repo of its own.
    fn in_repo(tag: &str) -> (App, PathBuf) {
        let (_, mut app) = super::design_tests::render_with(W, H);
        let dir = std::env::temp_dir().join(format!("{APP_COMMAND}-drawer-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        crate::proc::git(&dir, &["init", "-q"]).unwrap();
        std::fs::write(dir.join("notes.txt"), "remember the milk\n").unwrap();
        let pane = app.snap.terms.get_mut(&1).expect("the focused pane");
        pane.cwd = dir.clone();
        pane.top = Some(dir.clone());
        pane.root = Some(dir.clone());
        (app, dir)
    }

    fn draw(app: &mut App) -> Buffer {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(W, H)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        term.backend().buffer().clone()
    }

    fn text(buf: &Buffer) -> String {
        (0..H).map(|y| (0..W).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>() + "\n").collect()
    }

    /// Where `word` first starts on screen.
    fn find(buf: &Buffer, word: &str) -> Option<(u16, u16)> {
        let lines = text(buf);
        lines.lines().enumerate().find_map(|(y, l)| l.find(word).map(|i| (l[..i].chars().count() as u16, y as u16)))
    }

    fn open(app: &App) -> Option<screen::SheetKind> {
        screen::open_sheet(app)
    }

    fn key(app: &mut App, code: KeyCode, mods: KeyModifiers) {
        app.on_key(KeyEvent::new(code, mods));
    }

    fn click_tab(app: &mut App, i: usize) {
        let r = app.hits.iter().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::DrawerTab(i))).then_some(*r)).expect("the tab is clickable");
        for kind in [MouseEventKind::Down(MouseButton::Left), MouseEventKind::Up(MouseButton::Left)] {
            app.on_mouse(MouseEvent { kind, column: r.x + 1, row: r.y, modifiers: KeyModifiers::NONE });
        }
    }

    #[test]
    fn the_strip_names_each_view_with_its_key_and_lights_the_open_one() {
        let (mut app, dir) = in_repo("strip");
        app.act(Action::Files);
        let buf = draw(&mut app);
        let o = text(&buf);
        super::check_settings_tests::show(&o);
        let row = o.lines().find(|l| l.contains("Files f")).unwrap_or_else(|| panic!("a tab strip:\n{o}"));
        let (files, changes, branch) = (row.find("Files f").unwrap(), row.find("Changes c").unwrap(), row.find("Branch b").unwrap());
        assert!(files < changes && changes < branch && row.contains('·'), "in order, a dot apart: {row}");
        let (fx, fy) = find(&buf, "Files f").unwrap();
        let (cx, cy) = find(&buf, "Changes c").unwrap();
        assert_eq!(buf[(fx, fy)].bg, app.theme.accent, "the open tab is lit");
        assert_ne!(buf[(cx, cy)].bg, app.theme.accent, "the others are not");
        app.cfg.keys.prefix.insert("c".into(), "none".into());
        app.cfg.keys.prefix.insert("d".into(), "changes".into());
        app.keymap = app.cfg.keymap();
        assert!(find(&draw(&mut app), "Changes d").is_some(), "the keys come from the keymap");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_inbox_has_no_strip() {
        let (_, mut app) = super::design_tests::render_with(W, H);
        app.act(Action::Jump);
        let buf = draw(&mut app);
        assert_eq!(open(&app), Some(screen::SheetKind::Inbox));
        assert!(find(&buf, "Branch b").is_none(), "{}", text(&buf));
    }

    #[test]
    fn tab_and_shift_tab_cycle_files_changes_branch_and_references() {
        let (mut app, dir) = in_repo("cycle");
        app.act(Action::Files);
        use screen::SheetKind::{Branch, Changes, Files, References};
        for want in [Changes, Branch, References, Files] {
            key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
            assert_eq!(open(&app), Some(want), "Tab");
            assert!(app.view_has_keys(), "the new view has the keys");
        }
        for want in [References, Branch, Changes, Files] {
            key(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
            assert_eq!(open(&app), Some(want), "Shift+Tab");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tab_is_text_while_files_filters_or_edits() {
        let (mut app, dir) = in_repo("typing");
        app.act(Action::Files);
        let Some(View::Files(v)) = &mut app.view else { panic!("Files is open") };
        v.filtering = true;
        key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(open(&app), Some(screen::SheetKind::Files), "the filter keeps it");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Nothing typed into a pane since the last look.
    fn no_input(out: &mut mpsc::UnboundedReceiver<ClientMsg>) -> bool {
        std::iter::from_fn(|| out.try_recv().ok()).all(|m| !matches!(m, ClientMsg::Input { .. }))
    }

    #[test]
    fn a_burst_in_changes_stays_its_keys_and_never_reaches_the_pane() {
        let (mut app, dir) = in_repo("burst");
        crate::proc::git(&dir, &["add", "."]).unwrap();
        crate::proc::git(&dir, &["-c", "user.name=t", "-c", "user.email=t@t", "commit", "-q", "-m", "first"]).unwrap();
        for f in ["a.txt", "b.txt", "c.txt"] {
            std::fs::write(dir.join(f), "new\n").unwrap();
        }
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        let (btx, mut bg) = mpsc::unbounded_channel();
        app.bg = btx;
        app.act(Action::Changes);
        let sel = |app: &App| match &app.view {
            Some(View::Changes(v)) => (v.order(), v.review.as_ref().map(|r| r.sel)),
            _ => panic!("Changes is open"),
        };
        // Other background work may come back first.
        while sel(&app).1.is_none() {
            let read = super::branch_view_tests::next_bg(&mut bg);
            app.on_bg(read);
        }
        let (order, first) = sel(&app);
        assert!(order.len() >= 3 && first == Some(order[0]), "the files are read: {order:?}");
        app.on_burst(super::files_panel_tests::burst("jjk"));
        assert_eq!(sel(&app).1, Some(order[1]), "down, down, up: run as keys");
        assert!(no_input(&mut out), "nothing typed into the pane");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_paste_with_branch_open_never_reaches_the_pane() {
        let (mut app, dir) = in_repo("bpaste");
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        app.act(Action::Branches);
        assert_eq!(open(&app), Some(screen::SheetKind::Branch));
        app.on_event(Event::Paste("jk\n".into()));
        assert!(no_input(&mut out), "nothing typed into the pane");
        assert!(app.notice.as_ref().is_some_and(|(m, _, _)| m.starts_with("the side panel has the keys")), "it says why");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn r_switches_files_to_recent_files_and_back() {
        let (mut app, dir) = in_repo("recent");
        app.act(Action::Files);
        key(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(matches!(&app.view, Some(View::Files(v)) if v.recent), "r shows recent files");
        assert!(text(&draw(&mut app)).contains("r project"), "the hint says r");
        key(&mut app, KeyCode::Char('r'), KeyModifiers::NONE);
        assert!(matches!(&app.view, Some(View::Files(v)) if !v.recent), "r again shows the project");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn d_in_files_does_not_open_changes() {
        let (mut app, dir) = in_repo("nod");
        app.act(Action::Files);
        key(&mut app, KeyCode::Char('d'), KeyModifiers::NONE);
        assert_eq!(open(&app), Some(screen::SheetKind::Files), "Changes is c, not d");
        assert!(!text(&draw(&mut app)).contains("Show diff"), "no button offers it either");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_click_on_a_tab_switches_to_it() {
        let (mut app, dir) = in_repo("click");
        app.act(Action::Files);
        draw(&mut app);
        click_tab(&mut app, 2);
        assert_eq!(open(&app), Some(screen::SheetKind::Branch));
        draw(&mut app);
        click_tab(&mut app, 1);
        assert_eq!(open(&app), Some(screen::SheetKind::Changes));
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// The next background result, once its thread is done.
    fn next_bg(rx: &mut mpsc::UnboundedReceiver<Bg>) -> Bg {
        let until = Instant::now() + std::time::Duration::from_secs(20);
        loop {
            if let Ok(b) = rx.try_recv() {
                return b;
            }
            assert!(Instant::now() < until, "no background work was queued");
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }

    /// Changes open on a repo with one commit and one new file, its review read.
    fn changes_with_a_new_file(tag: &str) -> (App, PathBuf, mpsc::UnboundedReceiver<Bg>) {
        let (mut app, dir) = in_repo(tag);
        let git = |args: &[&str]| crate::proc::git(&dir, args).unwrap();
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        git(&["add", "."]);
        git(&["commit", "-qm", "start"]);
        std::fs::write(dir.join("new.txt"), "fresh\n").unwrap();
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.bg = tx;
        app.act(Action::Changes);
        let b = next_bg(&mut rx);
        app.on_bg(b);
        assert!(matches!(&app.view, Some(View::Changes(v)) if v.review.is_some()), "Changes read the repo");
        (app, dir, rx)
    }

    fn commits(dir: &std::path::Path) -> usize {
        crate::proc::git(dir, &["rev-list", "--count", "HEAD"]).unwrap().trim().parse().unwrap()
    }

    #[test]
    fn c_in_changes_asks_before_committing_and_y_commits() {
        let (mut app, dir, mut rx) = changes_with_a_new_file("commit");
        key(&mut app, KeyCode::Char('c'), KeyModifiers::NONE);
        assert!(matches!(&app.view, Some(View::Changes(v)) if v.confirm.as_ref().is_some_and(|(q, _)| q.starts_with("Commit 1 file on "))), "c asks first");
        assert!(text(&draw(&mut app)).contains("Commit 1 file on "), "the question is on screen");
        assert!(rx.try_recv().is_err() && commits(&dir) == 1, "nothing is committed yet");
        key(&mut app, KeyCode::Char('y'), KeyModifiers::NONE);
        let b = next_bg(&mut rx);
        app.on_bg(b);
        assert_eq!(commits(&dir), 2, "y commits");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn tab_answers_no_to_a_changes_question_instead_of_switching() {
        let (mut app, dir, mut rx) = changes_with_a_new_file("ask-tab");
        key(&mut app, KeyCode::Char('c'), KeyModifiers::NONE);
        key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert!(matches!(&app.view, Some(View::Changes(v)) if v.confirm.is_none()), "still Changes, the question answered no");
        assert!(rx.try_recv().is_err() && commits(&dir) == 1, "nothing is committed");
        key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        assert_eq!(open(&app), Some(screen::SheetKind::Branch), "with no question, Tab switches");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn at_its_narrowest_the_strip_keeps_every_tab_whole_and_clickable() {
        let (mut app, dir) = in_repo("narrow");
        app.screen.saved.sheet_w.insert("files".into(), screen::SHEET_MIN);
        app.act(Action::Files);
        let buf = draw(&mut app);
        let sheet = app.screen.sheet_rect;
        assert_eq!(sheet.width, screen::SHEET_MIN);
        let o = text(&buf);
        super::check_settings_tests::show(&o);
        for i in 0..3 {
            let r = app.hits.iter().rev().find_map(|(r, h)| (*h == Hit::Screen(screen::ScreenHit::DrawerTab(i))).then_some(*r)).expect("each tab has a target");
            assert!(r.width >= 4 && r.right() < sheet.right(), "tab {i} is whole inside the panel: {r:?} in {sheet:?}");
        }
        let row = o.lines().find(|l| l.contains("Chg")).unwrap_or_else(|| panic!("short names:\n{o}"));
        assert!(row.contains("Files") && row.contains("Br"), "short names: {row}");
        click_tab(&mut app, 2);
        assert_eq!(open(&app), Some(screen::SheetKind::Branch), "the last tab still switches");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_view_keeps_its_own_width_across_switches() {
        let (mut app, dir) = in_repo("width");
        app.screen.saved.sheet_w.insert("files".into(), 70);
        app.screen.saved.sheet_w.insert("changes".into(), 90);
        app.act(Action::Files);
        draw(&mut app);
        assert_eq!(app.screen.sheet_rect.width, 70);
        key(&mut app, KeyCode::Tab, KeyModifiers::NONE);
        draw(&mut app);
        assert_eq!(app.screen.sheet_rect.width, 90, "Changes' own width");
        key(&mut app, KeyCode::BackTab, KeyModifiers::SHIFT);
        draw(&mut app);
        assert_eq!(app.screen.sheet_rect.width, 70, "back to Files' width");
        let _ = std::fs::remove_dir_all(&dir);
    }
}

#[cfg(test)]
mod references_view_tests {
    use super::*;
    use super::branch_view_tests::next_bg;
    use super::check_settings_tests::{draw, show};

    fn leader(app: &mut App, c: char) {
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app.on_key(KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE));
    }

    fn key(app: &mut App, code: KeyCode) {
        app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn sheet(app: &App) -> Option<&screen::ReferencesSheet> {
        match &app.view {
            Some(View::References(v)) => Some(v),
            _ => None,
        }
    }

    /// A folder outside git for session 1, with `files` in its references (name, seconds old).
    fn project(tag: &str, files: &[(&str, u64)]) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("{APP_COMMAND}-refs-view-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let refs = crate::references::folder(&dir);
        std::fs::create_dir_all(&refs).unwrap();
        for (name, ago) in files {
            let p = refs.join(name);
            std::fs::write(&p, vec![0u8; 2048]).unwrap();
            let at = std::time::SystemTime::now() - std::time::Duration::from_secs(*ago);
            std::fs::File::options().write(true).open(&p).unwrap().set_modified(at).unwrap();
        }
        dir
    }

    /// The app at w×h with session 1 in `dir` and References open, the folder read.
    fn opened(dir: &std::path::Path, w: u16, h: u16) -> (App, mpsc::UnboundedReceiver<Bg>) {
        let (_, mut app) = super::design_tests::render_with(w, h);
        let (tx, mut rx) = mpsc::unbounded_channel();
        app.bg = tx;
        let pane = app.snap.terms.get_mut(&1).unwrap();
        pane.cwd = dir.to_path_buf();
        pane.top = Some(dir.to_path_buf());
        // Unknown to the snapshot: the background read finds the project from the folder.
        pane.root = None;
        leader(&mut app, 'a');
        assert!(sheet(&app).is_some_and(|v| v.list.is_none()), "a opens References, reading the folder");
        let b = next_bg(&mut rx);
        app.on_bg(b);
        (app, rx)
    }

    fn names(app: &App) -> Vec<String> {
        match sheet(app).and_then(|v| v.list.clone()) {
            Some(Ok(l)) => l.items.into_iter().map(|r| r.name).collect(),
            other => panic!("a list: {other:?}"),
        }
    }

    #[test]
    fn an_empty_project_says_how_to_add_one_at_both_sizes() {
        let dir = project("empty", &[]);
        for (w, h) in [(80, 24), (160, 45)] {
            let (mut app, _rx) = opened(&dir, w, h);
            let o = draw(&mut app, w, h);
            show(&o);
            assert!(o.contains("REFERENCES"), "in the side panel at {w}×{h}:\n{o}");
            assert!(o.contains("No references yet · paste an image or add a file"), "the empty state at {w}×{h}:\n{o}");
            assert!(o.contains("Refs") || o.contains("References"), "its tab in the strip at {w}×{h}");
            if w >= 160 {
                assert!(o.contains("Files f") && o.contains("References a"), "the strip names its key:\n{o}");
            }
            key(&mut app, KeyCode::Char('a'));
            assert!(sheet(&app).is_none(), "a closes it again");
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn references_are_listed_newest_first_with_kind_size_and_age() {
        let dir = project("list", &[("old-home.png", 7200), ("checkout.fig", 120), ("brief.pdf", 5)]);
        let (mut app, _rx) = opened(&dir, 160, 45);
        assert_eq!(names(&app), ["brief.pdf", "checkout.fig", "old-home.png"]);
        let o = draw(&mut app, 160, 45);
        show(&o);
        let row = |name: &str| o.lines().find(|l| l.contains(name)).unwrap_or_else(|| panic!("{name}:\n{o}")).to_string();
        let old = row("old-home.png");
        assert!(old.contains("image") && old.contains("2 KB") && old.contains(" 2h"), "{old}");
        assert!(row("checkout.fig").contains("design"));
        assert!(o.find("brief.pdf").unwrap() < o.find("old-home.png").unwrap(), "newest on top");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn remove_asks_first_then_deletes_the_file() {
        let dir = project("remove", &[("a.png", 10), ("b.png", 20)]);
        let (mut app, mut rx) = opened(&dir, 160, 45);
        key(&mut app, KeyCode::Down);
        key(&mut app, KeyCode::Char('d'));
        let Mode::Confirm(c) = &app.mode else { panic!("asked first: {:?}", std::mem::discriminant(&app.mode)) };
        assert!(c.danger && c.what == "b.png", "{c:?}");
        assert!(crate::references::folder(&dir).join("b.png").exists(), "nothing gone before the answer");
        key(&mut app, KeyCode::Enter);
        let done = next_bg(&mut rx);
        app.on_bg(done);
        assert!(!crate::references::folder(&dir).join("b.png").exists(), "removed");
        let reread = next_bg(&mut rx);
        app.on_bg(reread);
        assert_eq!(names(&app), ["a.png"], "the list reads again");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn n_copies_a_typed_or_dropped_file_in() {
        let dir = project("add", &[]);
        let src = dir.join("hero shot.png");
        std::fs::write(&src, "png").unwrap();
        let (mut app, mut rx) = opened(&dir, 160, 45);
        key(&mut app, KeyCode::Char('n'));
        assert!(matches!(app.mode, Mode::Prompt { kind: PromptKind::AddReference(..), .. }), "n asks for a path");
        app.on_event(Event::Paste(format!("\"{}\"", src.display())));
        key(&mut app, KeyCode::Enter);
        let added = next_bg(&mut rx);
        app.on_bg(added);
        let reread = next_bg(&mut rx);
        app.on_bg(reread);
        assert_eq!(names(&app), ["hero shot.png"]);
        assert!(src.exists(), "copied, not moved");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_file_dropped_on_the_open_view_is_added() {
        let dir = project("drop", &[]);
        let src = dir.join("mock.png");
        std::fs::write(&src, "png").unwrap();
        let (mut app, mut rx) = opened(&dir, 160, 45);
        app.on_event(Event::Paste(format!("'{}'", src.display())));
        let added = next_bg(&mut rx);
        app.on_bg(added);
        let reread = next_bg(&mut rx);
        app.on_bg(reread);
        assert_eq!(names(&app), ["mock.png"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn references_takes_its_own_paste_and_a_burst_stays_keys_never_reaching_the_pane() {
        let dir = project("ownpaste", &[]);
        let src = dir.join("mock.png");
        std::fs::write(&src, "png").unwrap();
        let (mut app, mut rx) = opened(&dir, 160, 45);
        let (tx, mut out) = mpsc::unbounded_channel();
        app.out = tx;
        app.on_burst(super::files_panel_tests::burst("jk"));
        assert!(app.notice.is_none(), "a short burst is keys here, not a paste");
        app.on_event(Event::Paste(src.display().to_string()));
        let added = next_bg(&mut rx);
        app.on_bg(added);
        let reread = next_bg(&mut rx);
        app.on_bg(reread);
        assert_eq!(names(&app), ["mock.png"], "References added it, not the side-panel guard");
        let typed = std::iter::from_fn(|| out.try_recv().ok()).filter(|m| matches!(m, ClientMsg::Input { .. })).count();
        assert_eq!(typed, 0, "nothing typed into the pane");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn pasted_text_that_names_no_file_adds_nothing_and_is_never_shown() {
        let dir = project("text", &[]);
        let (mut app, mut rx) = opened(&dir, 160, 45);
        app.on_event(Event::Paste("sk-secret-token\nsecond line".into()));
        let shown = app.notice.as_ref().map(|(t, ..)| t.clone()).unwrap_or_default();
        assert!(shown.contains("References has the keys") && !shown.contains("sk-secret"), "{shown}");
        assert!(rx.try_recv().is_err(), "nothing to add for several lines");
        app.notice = None;
        app.on_event(Event::Paste("sk-secret-token".into()));
        let b = next_bg(&mut rx);
        app.on_bg(b);
        let shown = app.notice.as_ref().map(|(t, ..)| t.clone()).unwrap_or_default();
        assert!(shown.contains("References has the keys") && !shown.contains("sk-secret"), "{shown}");
        assert_eq!(names(&app), Vec::<String>::new());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_paste_with_no_image_says_so_and_makes_no_folder() {
        let dir = std::env::temp_dir().join(format!("{APP_COMMAND}-refs-view-noimg-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let (mut app, mut rx) = opened(&dir, 160, 45);
        key(&mut app, KeyCode::Char('p'));
        let b = next_bg(&mut rx);
        app.on_bg(b);
        assert!(app.notice.as_ref().is_some_and(|(t, ..)| t.contains("no image")), "{:?}", app.notice);
        assert!(!crate::references::folder(&dir).exists(), "no image, no folder");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_session_in_a_worktree_sees_the_main_checkouts_references() {
        let main = project("wt", &[("shared.png", 10)]);
        let git = |args: &[&str]| crate::proc::git(&main, args).unwrap();
        git(&["init", "-q", "-b", "main"]);
        git(&["config", "user.email", "t@t"]);
        git(&["config", "user.name", "t"]);
        std::fs::write(main.join("a.txt"), "a").unwrap();
        git(&["add", "a.txt"]);
        git(&["commit", "-qm", "first"]);
        let wt = main.with_extension("feat");
        let _ = std::fs::remove_dir_all(&wt);
        git(&["worktree", "add", "-q", "-b", "feat", wt.to_str().unwrap()]);
        let (app, _rx) = opened(&wt, 160, 45);
        assert_eq!(names(&app), ["shared.png"], "the worktree lists the main checkout's set");
        assert_eq!(sheet(&app).unwrap().root.canonicalize().unwrap(), main.canonicalize().unwrap());
        let _ = std::fs::remove_dir_all(&wt);
        let _ = std::fs::remove_dir_all(&main);
    }

    #[test]
    fn y_copies_the_chosen_files_absolute_path() {
        let dir = project("copy", &[("a.png", 10)]);
        let (mut app, _rx) = opened(&dir, 160, 45);
        key(&mut app, KeyCode::Char('y'));
        let copied = crate::client::copy::LAST_COPY.with(|c| c.borrow().clone()).unwrap();
        assert_eq!(PathBuf::from(&copied), crate::references::folder(&dir).join("a.png"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}

mod pane_swap_tests {
    use super::*;
    use crate::layout::{Dir, Node};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use ratatui::layout::{Position, Rect};

    const LAYOUTS: [&str; 4] = ["floating", "tiled", "grid", "hairline"];
    /// Wide enough that Compact splits too.
    const W: u16 = 200;
    const H: u16 = 45;

    /// Panes 1 and 2 side by side in one tab, as the server keeps them, drawn in `layout`;
    /// and what the app sends.
    fn side_by_side(layout: &str) -> (App, mpsc::UnboundedReceiver<ClientMsg>) {
        let (_, mut app) = super::design_tests::render_with(W, H);
        let (tx, rx) = mpsc::unbounded_channel();
        app.out = tx;
        app.cfg.ui.layout = layout.into();
        app.cfg.ui.fold_retired_layout();
        let mut split = Node::Leaf(1);
        split.split(1, Dir::Right, 2);
        app.snap.workspaces[0].tabs = vec![
            TabInfo { id: 11, name: String::new(), layout: split, focus: 1 },
            TabInfo { id: 13, name: String::new(), layout: Node::Leaf(3), focus: 3 },
        ];
        app.screen.tabs.clear();
        app.screen.last_focus = None;
        app.screen_sync();
        draw(&mut app);
        (app, rx)
    }

    fn draw(app: &mut App) -> String {
        app.screen_fresh();
        let mut term = Terminal::new(TestBackend::new(W, H)).unwrap();
        term.draw(|f| render::draw(app, f)).unwrap();
        let buf = term.backend().buffer().clone();
        (0..H).map(|y| (0..W).map(|x| buf[(x, y)].symbol().to_string()).collect::<String>() + "\n").collect()
    }

    fn mouse(app: &mut App, kind: MouseEventKind, at: Position) {
        app.on_mouse(MouseEvent { kind, column: at.x, row: at.y, modifiers: KeyModifiers::NONE });
    }

    fn frame_of(app: &App, term: TermId) -> Rect {
        app.pane_frames.iter().find(|(id, _)| *id == term).map(|(_, r)| *r).expect("the pane is drawn")
    }

    /// A cell of `term`'s title row that none of its title's controls take, nearest the middle.
    fn title_spot(app: &App, term: TermId) -> Position {
        let r = frame_of(app, term);
        let free = |x: u16| {
            let at = Position::new(x, r.y);
            matches!(app.hits.iter().rev().find(|(h, _)| h.contains(at)).map(|(_, h)| *h), None | Some(Hit::Pane(_)))
        };
        let mid = r.x + r.width / 2;
        let x = (r.x..r.right()).filter(|x| free(*x)).min_by_key(|x| x.abs_diff(mid)).expect("blank title space");
        Position::new(x, r.y)
    }

    fn middle(app: &App, term: TermId) -> Position {
        let r = frame_of(app, term);
        Position::new(r.x + r.width / 2, r.y + r.height / 2)
    }

    fn swaps_sent(rx: &mut mpsc::UnboundedReceiver<ClientMsg>) -> Vec<(TermId, TermId)> {
        std::iter::from_fn(|| rx.try_recv().ok())
            .filter_map(|m| match m {
                ClientMsg::Command(Command::SwapPanes { a, b }) => Some((a, b)),
                _ => None,
            })
            .collect()
    }

    fn shown(app: &App) -> Vec<TermId> {
        app.screen.tabs[app.screen.tab].layout.leaves()
    }

    #[test]
    fn dragging_a_title_onto_another_pane_swaps_them_in_every_layout() {
        for layout in LAYOUTS {
            let (mut app, mut rx) = side_by_side(layout);
            assert_eq!(shown(&app), [1, 2], "{layout}");
            let (from, onto) = (title_spot(&app, 1), middle(&app, 2));
            mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
            mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), onto);
            let screen = draw(&mut app);
            if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
                println!("{layout}\n{screen}");
            }
            assert!(screen.contains("swap here"), "{layout}: the drop target shows\n{screen}");
            mouse(&mut app, MouseEventKind::Up(MouseButton::Left), onto);
            assert_eq!(shown(&app), [2, 1], "{layout}: they traded places");
            assert_eq!(swaps_sent(&mut rx), [(1, 2)], "{layout}: the server's tab follows");
            assert!(app.screen.drag.is_none(), "{layout}");
        }
    }

    #[test]
    fn a_title_drag_let_go_on_itself_outside_the_panes_or_after_esc_moves_nothing() {
        let (mut app, mut rx) = side_by_side("floating");
        let (from, own, other, sidebar) = (title_spot(&app, 1), middle(&app, 1), middle(&app, 2), Position::new(1, 5));
        let ends: [&dyn Fn(&mut App); 4] = [
            &|app| mouse(app, MouseEventKind::Up(MouseButton::Left), own),
            &|app| mouse(app, MouseEventKind::Up(MouseButton::Left), sidebar),
            &|app| mouse(app, MouseEventKind::Moved, other),
            &|app| {
                app.on_key(KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE));
                mouse(app, MouseEventKind::Up(MouseButton::Left), other);
            },
        ];
        for end in ends {
            mouse(&mut app, MouseEventKind::Down(MouseButton::Left), from);
            mouse(&mut app, MouseEventKind::Drag(MouseButton::Left), other);
            end(&mut app);
            assert!(app.screen.drag.is_none());
            assert_eq!(shown(&app), [1, 2]);
        }
        assert!(swaps_sent(&mut rx).is_empty());
    }

    #[test]
    fn a_press_on_a_title_still_focuses_its_pane() {
        let (mut app, mut rx) = side_by_side("tiled");
        let at = title_spot(&app, 2);
        mouse(&mut app, MouseEventKind::Down(MouseButton::Left), at);
        mouse(&mut app, MouseEventKind::Up(MouseButton::Left), at);
        let sent: Vec<ClientMsg> = std::iter::from_fn(|| rx.try_recv().ok()).collect();
        assert!(sent.iter().any(|m| matches!(m, ClientMsg::Command(Command::FocusPane { term: 2 }))), "{sent:?}");
        assert!(!sent.iter().any(|m| matches!(m, ClientMsg::Command(Command::SwapPanes { .. }))));
    }

    #[test]
    fn a_split_panes_title_has_no_close_button() {
        for layout in LAYOUTS {
            let (app, _rx) = side_by_side(layout);
            assert!(!app.hits.iter().any(|(_, h)| matches!(h, Hit::Screen(screen::ScreenHit::CloseSplit(_)))), "{layout}: the tab's ✕, Alt+x or the pane's menu close it");
        }
    }

    #[test]
    fn leader_alt_arrow_swaps_with_the_neighbour_that_way() {
        let (mut app, mut rx) = side_by_side("tiled");
        app.on_key(KeyEvent::new(KeyCode::Char(' '), KeyModifiers::CONTROL));
        app.on_key(KeyEvent::new(KeyCode::Left, KeyModifiers::ALT));
        assert_eq!(shown(&app), [1, 2], "nothing left of pane 1");
        assert!(matches!(app.mode, Mode::Prefix { .. }), "shortcut mode stays on");
        app.on_key(KeyEvent::new(KeyCode::Right, KeyModifiers::ALT));
        assert_eq!(shown(&app), [2, 1]);
        assert_eq!(app.focused(), Some(1), "the keys stay with the pane that moved");
        assert_eq!(swaps_sent(&mut rx), [(1, 2)]);
    }

    #[test]
    fn the_pane_menu_offers_a_move_toward_each_neighbour() {
        let (mut app, mut rx) = side_by_side("tiled");
        app.menu_for_pane(1, (5, 5));
        let Mode::ScreenMenu(menu) = app.mode.clone() else { panic!("a menu") };
        let labels: Vec<&str> = menu.items.iter().map(|(l, _)| l.as_str()).collect();
        assert!(labels.contains(&"Move pane right"), "{labels:?}");
        assert!(!labels.iter().any(|l| ["Move pane left", "Move pane up", "Move pane down"].contains(l)), "{labels:?}");
        app.menu_do(menu::Act::Swap(1, 2));
        assert_eq!(shown(&app), [2, 1]);
        assert_eq!(swaps_sent(&mut rx), [(1, 2)]);
    }

    #[test]
    fn panes_split_here_only_swap_here() {
        let (mut app, mut rx) = side_by_side("tiled");
        app.snap.workspaces[0].tabs[0].layout = Node::Leaf(1);
        app.snap.workspaces[0].tabs.push(TabInfo { id: 12, name: String::new(), layout: Node::Leaf(2), focus: 2 });
        assert!(app.swap_panes(1, 2));
        assert_eq!(shown(&app), [2, 1]);
        assert!(swaps_sent(&mut rx).is_empty(), "the server keeps them apart: nothing to tell it");
        assert!(!app.swap_panes(1, 3), "3 isn't shown beside them");
    }
}
