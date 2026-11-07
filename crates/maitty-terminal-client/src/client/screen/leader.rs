//! The Menu (the common commands) and tabs (new,
//! renamed in their pill). The key map, search and All keys are in `keyoverlay`.

use super::*;

/// How long a tab's typed name may get.
const NAME_MAX: usize = 40;

/// A Menu row: its name and what it does. The leader key bound to it runs it here too, but the
/// Menu doesn't show it: a bare letter beside a row reads as a key that works anywhere.
pub(in crate::client) type MenuItem = (&'static str, Action);

/// A section of the Menu: what its rows are for (nothing for one that needs no saying) and
/// the rows.
pub(in crate::client) type MenuGroup = (&'static str, Vec<MenuItem>);

/// The Menu's sections in order, each under its title. "Shell here" starts a plain shell in the
/// folder you are in; "Session..." opens the New session dialog for any project and machine.
pub(in crate::client) fn menu_groups() -> Vec<MenuGroup> {
    vec![
        (
            "NEW",
            vec![
                ("Shell here", Action::ShellHere),
                ("Session...", Action::PickSession),
                ("Pane beside this", Action::SplitRight),
            ],
        ),
        (
            "TOOLS",
            vec![
                ("Files", Action::Files),
                ("Changes", Action::Changes),
                ("Branches", Action::Branches),
                ("History", Action::History),
                ("References", Action::References),
            ],
        ),
        (
            "FIND",
            vec![
                ("Inbox", Action::Jump),
                ("Search commands", Action::Palette),
                ("All keys", Action::Help),
            ],
        ),
        ("SIDEBAR", vec![("Collapse sidebar", Action::ToggleSidebar)]),
        ("", vec![("Settings", Action::Settings)]),
    ]
}

/// The Menu's rows in the order they are numbered.
pub(in crate::client) fn action_items() -> Vec<MenuItem> {
    menu_groups()
        .into_iter()
        .flat_map(|(_, items)| items)
        .collect()
}

/// How much of the Menu's layout a window has rows for. The blank rows between sections give
/// way first, then the titles; a row that does something is the last thing dropped.
#[derive(Clone, Copy)]
struct MenuRoom {
    titles: bool,
    gaps: bool,
}

impl MenuRoom {
    /// The rows the sections take laid out this way.
    fn rows(self, groups: &[MenuGroup]) -> u16 {
        let items: usize = groups.iter().map(|(_, items)| items.len()).sum();
        let titles = groups.iter().filter(|(title, _)| !title.is_empty()).count();
        (items
            + if self.titles { titles } else { 0 }
            + if self.gaps { groups.len() - 1 } else { 0 }) as u16
    }

    /// The fullest layout that fits a card of at most `most` rows.
    fn of(groups: &[MenuGroup], most: u16) -> MenuRoom {
        [(true, true), (true, false), (false, false)]
            .into_iter()
            .map(|(titles, gaps)| MenuRoom { titles, gaps })
            .find(|room| room.rows(groups) + MENU_FRAME_ROWS <= most)
            .unwrap_or(MenuRoom {
                titles: false,
                gaps: false,
            })
    }
}

/// The rows of the Menu's card that are not its sections: the two borders and a blank row
/// inside each.
const MENU_FRAME_ROWS: u16 = 4;

/// The label a Menu row shows now: the sidebar's rows say what the next press does.
fn menu_label(app: &App, a: &Action, label: &'static str) -> &'static str {
    match a {
        Action::ToggleSidebar if !app.sidebar => "Expand sidebar",
        _ => label,
    }
}

/// The Menu: a card at the bottom left, over the sidebar's foot. Groups a blank row apart.
pub(in crate::client) fn draw_actions(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    t: &Theme,
    sel: usize,
) {
    let look = Look::of(&app.cfg.ui);
    let buf = f.buffer_mut();
    dim_all(app, buf, area, t);
    hit(app, area, ScreenHit::Close);
    let groups = menu_groups();
    let most = area.height.saturating_sub(2);
    let room = MenuRoom::of(&groups, most);
    let w = 46.min(area.width.saturating_sub(4));
    let h = (room.rows(&groups) + MENU_FRAME_ROWS).min(most);
    let side = grid(app, framed(app, area)).side;
    let x = (if side.width > 0 {
        side.x + 1
    } else {
        area.x + 3
    })
    .min(area.right().saturating_sub(w));
    let y = area.bottom().saturating_sub(4 + h).max(area.y);
    let r = Rect {
        x,
        y,
        width: w,
        height: h,
    };
    let mut c = Card::new(t, "Menu").lit(t.accent);
    c.bg = t.bg;
    let inside = card(app, buf, r, &c, t);
    hit(app, r, ScreenHit::Noop);
    let needs = app
        .screen_model()
        .iter()
        .flat_map(|p| p.sessions())
        .filter(|s| s.status == Status::Blocked)
        .count();
    let mut yy = r.y + 2;
    let mut i = 0usize;
    for (g, (title, items)) in groups.iter().enumerate() {
        if yy >= inside.bottom().saturating_sub(1) {
            break;
        }
        if g > 0 && room.gaps {
            yy += 1;
        }
        if room.titles && !title.is_empty() {
            put(
                buf,
                r.x + 4,
                yy,
                &[seg(*title, Style::default().bg(t.bg).fg(t.muted))],
                r.right().saturating_sub(6),
            );
            yy += 1;
        }
        for (label, a) in items {
            if yy >= inside.bottom().saturating_sub(1) {
                i += 1;
                continue;
            }
            let rr = Rect {
                x: r.x + 2,
                y: yy,
                width: w.saturating_sub(4),
                height: 1,
            };
            let on = i == sel || hovered(app, rr);
            if on {
                row_pill(look, buf, rr.x, yy, rr.width, t.sel_on(t.bg), t.bg);
                put(
                    buf,
                    rr.x + 1,
                    yy,
                    &[seg("▌", Style::default().fg(t.accent).bg(t.sel_on(t.bg)))],
                    rr.right(),
                );
            }
            let bg = if on { t.sel_on(t.bg) } else { t.bg };
            let st = Style::default().bg(bg);
            let mut ls = st.fg(if on { t.strong } else { t.text });
            if on {
                ls = ls.add_modifier(Modifier::BOLD);
            }
            let label = menu_label(app, a, label);
            let end = put(
                buf,
                r.x + 4,
                yy,
                &[seg(label, ls)],
                r.right().saturating_sub(6),
            );
            if *a == Action::Jump && needs > 0 {
                put(
                    buf,
                    end + 1,
                    yy,
                    &[seg(
                        format!("● {needs}"),
                        st.fg(t.blocked).add_modifier(Modifier::BOLD),
                    )],
                    r.right().saturating_sub(6),
                );
            } else if *a == Action::Settings && app.restart_pending() {
                put(
                    buf,
                    end + 1,
                    yy,
                    &[seg("●", st.fg(t.accent).add_modifier(Modifier::BOLD))],
                    r.right().saturating_sub(6),
                );
            }
            hit(app, rr, ScreenHit::ActionRow(i));
            yy += 1;
            i += 1;
        }
    }
}

impl App {
    pub(in crate::client) fn on_actions_key(&mut self, sel: usize, k: &KeyEvent) {
        let items = action_items();
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Up => {
                self.mode = Mode::Actions {
                    sel: sel.saturating_sub(1),
                }
            }
            KeyCode::Down => {
                self.mode = Mode::Actions {
                    sel: (sel + 1).min(items.len() - 1),
                }
            }
            KeyCode::Enter => self.actions_run(sel),
            KeyCode::Char(_)
                if !k
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                let bound = self
                    .keymap
                    .prefixed
                    .get(&crate::keys::KeySpec::from_event(k));
                if let Some(i) = items.iter().position(|(_, a)| Some(a) == bound) {
                    self.actions_run(i);
                }
            }
            _ => {}
        }
    }

    pub(in crate::client) fn actions_run(&mut self, i: usize) {
        if let Some((_, a)) = action_items().into_iter().nth(i) {
            self.mode = Mode::Normal;
            self.act(a);
        }
    }

    /// A new tab of the session you're on: a shell where you are (cd and run what you like;
    /// the sidebar groups it by where it works).
    pub(in crate::client) fn new_tab_start(&mut self) {
        let Some(owner) = self
            .screen
            .tabs
            .get(self.screen.tab)
            .map(|t| t.owner)
            .or(self.focused())
        else {
            self.notify("open a session first".into(), true);
            return;
        };
        self.screen_act(&Action::ShellHere);
        // After the command goes, which clears it: this is the one session it is for.
        let known = self.all_terms().into_iter().map(|(t, _)| t).collect();
        *self.screen.new_tab.borrow_mut() = Some(PendingTab {
            at: Instant::now(),
            owner,
            known,
        });
    }

    /// Rename the session you're on, in its pill: its name is the one the sidebar, the pill and
    /// every notice use, so a name kept on the tab alone would leave them all saying the old one.
    pub(in crate::client) fn rename_tab_start(&mut self) {
        let i = self.screen.tab;
        if let Some(tab) = self.screen.tabs.get(i) {
            let name = self
                .term_info(tab.focus)
                .map(|t| t.label.trim().to_string())
                .unwrap_or_default();
            self.mode = Mode::RenameTab(Box::new(TabName { tab: i, name }));
        }
    }

    /// Typing a session's name: Enter keeps it (empty: named after its worktree), Esc doesn't.
    pub(in crate::client) fn on_rename_tab_key(&mut self, mut nt: TabName, k: &KeyEvent) {
        match k.code {
            KeyCode::Enter => {
                let name = nt.name.trim().to_string();
                let found = self.screen.tabs.get(nt.tab).and_then(|tab| {
                    let focus = tab.focus;
                    let snap = self.snap_of(focus.machine)?;
                    snap.workspaces.iter().find_map(|w| {
                        w.tabs
                            .iter()
                            .find(|t| t.layout.contains(focus.term))
                            .map(|t| (w.id, t.id))
                    })
                });
                if let Some(tab) = self.screen.tabs.get_mut(nt.tab) {
                    let focus = tab.focus;
                    // The daemon owns the name: it outlives this view's tab being rebuilt, and
                    // `screen_sync` reads it back from the snapshot.
                    // A name an older version left on the tab would hide the session's.
                    match found {
                        Some((ws, tab)) => self.cmd_on(
                            focus.machine,
                            Command::RenameTab {
                                ws,
                                tab,
                                name: String::new(),
                            },
                        ),
                        None => tab.name = String::new(),
                    }
                    self.cmd_on(
                        focus.machine,
                        Command::RenamePane {
                            term: focus.term,
                            name,
                        },
                    );
                }
                self.last_click = None;
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Esc => {
                self.last_click = None;
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Backspace => {
                nt.name.pop();
            }
            KeyCode::Char(c)
                if !k.modifiers.contains(KeyModifiers::CONTROL)
                    && nt.name.chars().count() < NAME_MAX =>
            {
                nt.name.push(c)
            }
            _ => {}
        }
        self.mode = Mode::RenameTab(Box::new(nt));
    }
}
