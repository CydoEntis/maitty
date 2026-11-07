//! The one key overlay. The prefix opens the key map (what each key does after it; pressing one
//! runs it). Ctrl+K (or Space in the map) is Search commands: a fuzzy search over every command,
//! keyed or not, and over the sessions. `?` is the keys help: three columns of the keys that matter.

use super::*;
use crate::client::design::{keycap, long_key};
use crate::layout::Dir;

/// Sessions listed before anything is typed.
const SESSIONS_SHOWN: usize = 5;
/// The most Search commands is tall, and its widest.
const SEARCH_MAX_H: u16 = 26;
const SEARCH_MAX_W: u16 = 110;
/// How long typed words may get in the search.
const QUERY_MAX: usize = 40;
/// The widest the overlay gets.
const MAX_W: u16 = 112;
/// A card this wide lays the keys out in three columns, else two.
const THREE_COLUMNS_FROM: u16 = 100;
/// Recent commands listed when nothing is typed.
const RECENT_SHOWN: usize = 5;
/// Recent commands remembered.
const RECENT_KEPT: usize = 8;
/// Rows a page key moves in a list.
const PAGE: usize = 5;
/// Shown until the user has run something from the overlay.
const DEFAULT_RECENT: [fn() -> Action; 4] = [
    || Action::Jump,
    || Action::Message,
    || Action::SplitRight,
    || Action::ToggleSidebar,
];

/// Which of the overlay's three faces is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum KeyView {
    /// The groups of leader keys; a key runs.
    Map,
    /// Typing filters every command.
    Search,
    /// The map, the keys that need no leader, and the commands with no key.
    All,
}

/// A row of the key map: what it is called, the actions it stands for (several when one row
/// covers a set of keys such as the four arrows) and whether the short map shows it.
pub(in crate::client) struct KeyRow {
    pub label: &'static str,
    pub acts: Vec<Action>,
    pub core: bool,
}

/// The key map's groups, the single list behind the map, All keys, the search and Settings'
/// Keys page. A row with no key bound is found by search only.
pub(in crate::client) fn key_groups() -> Vec<(&'static str, Vec<KeyRow>)> {
    use Action as A;
    let r = |label, acts: Vec<Action>, core| KeyRow { label, acts, core };
    vec![
        (
            "SESSIONS",
            vec![
                r("New shell here", vec![A::ShellHere], true),
                r("New session", vec![A::PickSession], false),
                r("Next tab", vec![A::NextTab], true),
                r("Previous tab", vec![A::PrevTab], true),
                r("Undo close", vec![A::UndoClose], true),
                r("Message this session", vec![A::Message], true),
                r("Menu", vec![A::Menu], true),
                r("Inbox", vec![A::Jump], true),
                r("Next or previous session", vec![A::NextSession, A::PrevSession], false),
                r("Rename selected session", vec![A::RenameWorkspace], false),
                r("Stop agent", vec![A::StopAgent], false),
                r("Focus the sidebar", vec![A::BrowseTree], false),
            ],
        ),
        (
            "PANES",
            vec![
                r("Split up", vec![A::SplitUp], true),
                r("Split down", vec![A::SplitDown], true),
                r("Split left", vec![A::SplitLeft], true),
                r("Split right", vec![A::SplitRight], true),
                r("Close pane", vec![A::ClosePane], true),
                r(
                    "Move focus",
                    vec![
                        A::Focus(Dir::Left),
                        A::Focus(Dir::Up),
                        A::Focus(Dir::Down),
                        A::Focus(Dir::Right),
                    ],
                    true,
                ),
                r(
                    "Resize pane",
                    vec![
                        A::Resize(Dir::Left),
                        A::Resize(Dir::Down),
                        A::Resize(Dir::Up),
                        A::Resize(Dir::Right),
                    ],
                    false,
                ),
                r(
                    "Swap pane",
                    vec![
                        A::Swap(Dir::Left),
                        A::Swap(Dir::Down),
                        A::Swap(Dir::Up),
                        A::Swap(Dir::Right),
                    ],
                    false,
                ),
                r("Find in pane", vec![A::Search], false),
                r("Select text", vec![A::CopyMode], false),
                r("Scroll", vec![A::ScrollUp, A::ScrollDown], false),
                r(
                    "Jump between commands",
                    vec![A::PrevPrompt, A::NextPrompt],
                    false,
                ),
                r("Paste image", vec![A::PasteImage], false),
                r("Send a file to the machine", vec![A::SendFile], false),
            ],
        ),
        (
            "SIDEBAR",
            vec![
                r("Collapse sidebar", vec![A::ToggleSidebar], true),
                r("Widen or narrow the sidebar", vec![A::SidebarWider, A::SidebarNarrower], false),
                r("Hide sidebar", vec![A::HideSidebar], false),
            ],
        ),
        (
            "TOOLS",
            vec![
                r("Files", vec![A::Files], true),
                r("Changes", vec![A::Changes], true),
                r("Branches", vec![A::Branches], true),
                r("History", vec![A::History], true),
                r("References", vec![A::References], true),
                r("Worktrees ›", vec![A::Worktrees], true),
                r("New worktree", vec![A::NewWorktree(None)], false),
                r("Find a file", vec![A::Find(0)], false),
                r("Search the code", vec![A::Find(1)], false),
            ],
        ),
        (
            env!("APP_ENV"),
            vec![
                r("Search commands", vec![A::Palette], true),
                r("Settings", vec![A::Settings], true),
                r("All keys", vec![A::Help], true),
                r("Updates", vec![A::Updates], true),
                r("Quit, agents keep running", vec![A::Detach], true),
                r("Machines", vec![A::Machines], true),
                r("Reload config", vec![A::ReloadConfig], false),
            ],
        ),
    ]
}

/// The group a command belongs to when no row of `key_groups` lists it.
fn group_of(a: &Action) -> &'static str {
    use Action as A;
    if let Some((g, _)) = key_groups()
        .into_iter()
        .find(|(_, rows)| rows.iter().any(|r| r.acts.contains(a)))
    {
        return g;
    }
    match a {
        A::NewWorkspace | A::CloseWorkspace | A::NewSession | A::AgentStatusSetup => "SESSIONS",
        A::SplitLeft
        | A::SplitUp
        | A::FocusNext
        | A::FocusPrev
        | A::CloseSplit
        | A::Search
        | A::PaneInfo
        | A::Spawn(..)
        | A::SideMove(_) => "PANES",
        A::NewTab | A::NextTab | A::PrevTab | A::CloseTab | A::RenameTab | A::SpawnTab(_) | A::SelectTab(_) => "PANES",
        A::RemoveWorktree | A::TidyWorktrees | A::NewWorktree(_) => "TOOLS",
        _ => env!("APP_ENV"),
    }
}

/// The category column of Search commands.
fn category_of(act: Option<&Action>) -> &'static str {
    use Action as A;
    match act {
        Some(
            A::SplitRight | A::SplitDown | A::SplitLeft | A::SplitUp | A::ClosePane | A::Focus(_)
            | A::FocusNext | A::FocusPrev | A::Resize(_) | A::Swap(_) | A::ScrollUp | A::ScrollDown
            | A::CopyMode | A::CloseSplit | A::PaneInfo | A::PrevPrompt | A::NextPrompt | A::Spawn(..)
            | A::UndoClose | A::NewTab | A::NextTab | A::PrevTab | A::CloseTab | A::SelectTab(_),
        ) => "Panes",
        Some(
            A::Files | A::Changes | A::Branches | A::History | A::References | A::Worktrees
            | A::NewWorktree(_) | A::RemoveWorktree | A::TidyWorktrees | A::Machines,
        ) => "Tools",
        Some(A::Jump | A::Palette | A::Help | A::Find(_) | A::Search | A::Menu) => "Find",
        Some(
            A::ToggleSidebar | A::HideSidebar | A::BrowseTree | A::SidebarWider | A::SidebarNarrower
            | A::SideMove(_),
        ) => "Sidebar",
        Some(A::ToggleAppearance) => "Appearance",
        _ => "General",
    }
}

/// A session the search offers.
#[derive(Clone)]
struct SessionHit {
    term: ClientTerm,
    state: super::status_mark::State,
    name: String,
    /// `project · machine`.
    place: String,
}

/// The sessions whose name, project or machine match, in the sidebar's order. With nothing
/// typed the first few are listed.
fn session_hits(app: &App, query: &str) -> Vec<SessionHit> {
    let q: String = query.split_whitespace().collect();
    let machine_name = |m: MachineId| {
        app.remotes
            .get(&m)
            .map_or_else(|| "local".to_string(), |r| r.name.clone())
    };
    let mut out: Vec<SessionHit> = app
        .screen_model()
        .iter()
        .flat_map(|p| p.sessions().map(move |s| (p, s)))
        .filter_map(|(p, s)| {
            let term = s.native_term?;
            let place = format!("{} · {}", p.name, machine_name(p.machine));
            let exit = app.term_info(term).is_some_and(|i| i.exit.is_some());
            let failed = super::status_mark::failed(exit, s.status, s.check.as_ref());
            let state = super::status_mark::State::of(s.status, failed, s.down);
            Some(SessionHit { term, state, name: s.name.clone(), place })
        })
        .filter(|h| {
            q.is_empty()
                || super::new_session::fuzzy_indices(&q, &h.name).is_some()
                || super::new_session::fuzzy_indices(&q, &h.place).is_some()
        })
        .collect();
    if q.is_empty() {
        out.truncate(SESSIONS_SHOWN);
    }
    out
}

/// A group name as it reads in a list: "Sessions".
fn sentence(group: &str) -> String {
    let mut c = group.chars();
    c.next()
        .map(|f| f.to_string() + &c.as_str().to_lowercase())
        .unwrap_or_default()
}

/// One command: where it lives, what it is called, the keys that run it and the action
/// (None when a row stands for several, so Enter cannot pick one).
#[derive(Clone)]
struct Entry {
    group: String,
    label: String,
    /// Other words that find it: what the action is called elsewhere ("New shell here").
    also: String,
    keys: String,
    act: Option<Action>,
}

fn is_bound(app: &App, a: &Action) -> bool {
    app.keymap
        .prefixed
        .values()
        .chain(app.keymap.global.values())
        .any(|b| b == a)
}

/// The rows of the key map, with the keys they have now; unbound ones have empty `keys`.
fn leader_entries(app: &App) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut listed: Vec<Action> = Vec::new();
    for (group, rows) in key_groups() {
        for row in rows {
            listed.extend(row.acts.iter().cloned());
            let also = row
                .acts
                .iter()
                .map(Action::describe)
                .collect::<Vec<_>>()
                .join(" ");
            out.push(Entry {
                group: sentence(group),
                label: row.label.into(),
                also,
                keys: key_text(app, &row.acts),
                act: (row.acts.len() == 1).then(|| row.acts[0].clone()),
            });
        }
    }
    // Keys of the user's own (a spawn binding, an alias for something else) belong to a group too.
    let mut seen: Vec<Action> = Vec::new();
    for key in &app.keymap.prefixed_order {
        let Some(a) = app.keymap.prefixed.get(key) else {
            continue;
        };
        if matches!(a, Action::None | Action::SendPrefix | Action::SideMove(_))
            || listed.contains(a)
            || seen.contains(a)
        {
            continue;
        }
        seen.push(a.clone());
        out.push(Entry {
            group: sentence(group_of(a)),
            label: a.describe(),
            also: String::new(),
            keys: key_text(app, std::slice::from_ref(a)),
            act: Some(a.clone()),
        });
    }
    out
}

/// The keys that need no leader.
fn direct_entries(app: &App) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut tabs = false;
    let mut focus = false;
    let mut keys: Vec<_> = app
        .keymap
        .global
        .iter()
        .filter(|(_, a)| **a != Action::None)
        .collect();
    keys.sort_by_key(|(key, _)| key.to_config());
    for (key, a) in keys {
        match a {
            Action::SelectTab(_) => {
                if !tabs
                    && let Some((k1, _)) = app
                        .keymap
                        .global
                        .iter()
                        .find(|(_, b)| **b == Action::SelectTab(1))
                {
                    tabs = true;
                    let one = long_key(k1);
                    out.push(Entry {
                        group: "No leader".into(),
                        label: "Go to tab 1–9".into(),
                        also: String::new(),
                        keys: format!("{}1-9", one.trim_end_matches('1')),
                        act: None,
                    });
                }
            }
            Action::Focus(_) => {
                if !focus {
                    focus = true;
                    let mods = long_key(key);
                    let arrows = mods
                        .rsplit_once('+')
                        .map(|(m, _)| format!("{m}+arrows"))
                        .unwrap_or_else(|| "arrows".into());
                    out.push(Entry {
                        group: "No leader".into(),
                        label: "Move focus".into(),
                        also: String::new(),
                        keys: arrows,
                        act: None,
                    });
                }
            }
            other => out.push(Entry {
                group: "No leader".into(),
                label: other.describe(),
                also: String::new(),
                keys: long_key(key),
                act: Some(other.clone()),
            }),
        }
    }
    out
}

/// Commands with no key: every one the palette used to list, and the rows of the map that
/// the user has unbound.
fn search_only_entries(app: &App, leader: &[Entry]) -> Vec<Entry> {
    let mut out: Vec<Entry> = leader
        .iter()
        .filter(|e| e.keys.is_empty())
        .cloned()
        .collect();
    for a in app.palette_commands() {
        if is_bound(app, &a)
            || key_groups()
                .iter()
                .any(|(_, rows)| rows.iter().any(|r| r.acts.contains(&a)))
        {
            continue;
        }
        out.push(Entry {
            group: sentence(group_of(&a)),
            label: a.describe(),
            also: String::new(),
            keys: String::new(),
            act: Some(a),
        });
    }
    out
}

/// Every command, for the search: keyed rows first in the map's order, then the rest.
fn all_entries(app: &App) -> Vec<Entry> {
    let leader = leader_entries(app);
    let direct = direct_entries(app);
    let rest = search_only_entries(app, &leader);
    // A row that stands for a set of keys is found, and run, one key at a time.
    let singles = key_groups().into_iter().flat_map(|(group, rows)| {
        rows.into_iter()
            .filter(|r| r.acts.len() > 1)
            .flat_map(move |r| r.acts.into_iter().map(move |a| (group, a)))
    });
    let singles: Vec<Entry> = singles
        .map(|(group, a)| Entry {
            group: sentence(group),
            label: a.describe(),
            also: String::new(),
            keys: key_text(app, std::slice::from_ref(&a)),
            act: Some(a),
        })
        .collect();
    let mut out: Vec<Entry> = leader
        .into_iter()
        .filter(|e| !e.keys.is_empty() && e.act.is_some())
        .chain(singles)
        .chain(direct)
        .chain(rest)
        .collect();
    let mut seen: Vec<String> = Vec::new();
    out.retain(|e| {
        let fresh = !seen.contains(&e.label);
        seen.push(e.label.clone());
        fresh
    });
    out
}

/// Every command as (name, keys), for tests of what the overlay can reach.
#[cfg(test)]
pub(in crate::client) fn keymap_all_rows(app: &App) -> Vec<(String, String)> {
    all_entries(app)
        .into_iter()
        .map(|e| (e.label, e.keys))
        .collect()
}

/// The commands the search shows: the latest run when nothing is typed, else the ones whose
/// name or group has every typed word (names before groups).
fn found(app: &App, query: &str) -> Vec<Entry> {
    let all = all_entries(app);
    let words: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(str::to_string)
        .collect();
    if words.is_empty() {
        let mut out: Vec<Entry> = Vec::new();
        let names = app
            .screen
            .saved
            .recent
            .iter()
            .filter_map(|n| n.parse::<Action>().ok())
            .chain(DEFAULT_RECENT.iter().map(|f| f()));
        for a in names {
            if let Some(e) = all.iter().find(|e| e.act.as_ref() == Some(&a))
                && !out.iter().any(|o| o.label == e.label)
            {
                out.push(e.clone());
            }
            if out.len() == RECENT_SHOWN {
                break;
            }
        }
        return out;
    }
    // In-order subsequence, case-blind: "gs" finds "Go to session". Names that begin with what is
    // typed come first, then names that contain it as a run, then the rest.
    let q: String = words.concat();
    let mut hits: Vec<(u8, Entry)> = all
        .into_iter()
        .filter_map(|e| {
            let label = e.label.to_lowercase();
            let rank = if label.starts_with(&q) {
                0
            } else if label.contains(&q) {
                1
            } else if super::new_session::fuzzy_indices(&q, &label).is_some() {
                2
            } else if super::new_session::fuzzy_indices(&q, &format!("{label} {}", e.also.to_lowercase())).is_some()
                || super::new_session::fuzzy_indices(&q, &category_of(e.act.as_ref()).to_lowercase()).is_some()
            {
                3
            } else {
                return None;
            };
            Some((rank, e))
        })
        .collect();
    hits.sort_by_key(|(rank, _)| *rank);
    hits.into_iter().map(|(_, e)| e).collect()
}

/// A row of the grid: its keys, its name and what a click runs.
struct GridRow {
    keys: String,
    label: String,
    act: Option<Action>,
}

/// A group of the grid and the column it goes in.
struct Section {
    title: String,
    dim: bool,
    note: &'static str,
    /// Its column when the card is three wide, and when it is two.
    slots: (usize, usize),
    rows: Vec<GridRow>,
}

/// The map (`all` false: the short one) or All keys, group by group.
/// The sidebar's own new-session keys (`Mode::Side` reads them); not in the keymap.
const SIDEBAR_KEYS: [(&str, &str); 1] = [("n", "New session")];

fn grid_sections(app: &App, all: bool) -> Vec<Section> {
    let leader = leader_entries(app);
    let core: Vec<&str> = key_groups()
        .into_iter()
        .flat_map(|(_, rows)| rows.into_iter().filter(|r| r.core).map(|r| r.label))
        .collect();
    let mut out: Vec<Section> = Vec::new();
    for (i, (group, _)) in key_groups().into_iter().enumerate() {
        let name = sentence(group);
        let rows: Vec<GridRow> = leader
            .iter()
            .filter(|e| {
                e.group == name && !e.keys.is_empty() && (all || core.contains(&e.label.as_str()))
            })
            .map(|e| GridRow {
                keys: e.keys.clone(),
                label: e.label.clone(),
                act: e.act.clone(),
            })
            .collect();
        if !rows.is_empty() {
            out.push(Section {
                title: group.to_string(),
                dim: false,
                note: "",
                slots: [(0, 0), (1, 0), (1, 1), (0, 1), (2, 1)][i],
                rows,
            });
        }
    }
    if all {
        let direct: Vec<GridRow> = direct_entries(app)
            .into_iter()
            .map(|e| GridRow {
                keys: e.keys,
                label: e.label,
                act: e.act,
            })
            .collect();
        if !direct.is_empty() {
            out.push(Section {
                title: "WITHOUT THE LEADER".into(),
                dim: true,
                note: "press directly",
                slots: (2, 1),
                rows: direct,
            });
        }
        out.push(Section {
            title: "IN THE SIDEBAR".into(),
            dim: true,
            note: "press directly",
            slots: (2, 1),
            rows: SIDEBAR_KEYS
                .iter()
                .map(|&(keys, label)| GridRow {
                    keys: keys.into(),
                    label: label.into(),
                    act: None,
                })
                .collect(),
        });
        let only: Vec<GridRow> = search_only_entries(app, &leader)
            .into_iter()
            .map(|e| GridRow {
                keys: String::new(),
                label: e.label,
                act: e.act,
            })
            .collect();
        if !only.is_empty() {
            out.push(Section {
                title: "SEARCH ONLY".into(),
                dim: true,
                note: "",
                slots: (2, 1),
                rows: only,
            });
        }
    }
    out
}

/// What each shown row runs, in the order its rows are numbered (a click on row `i`).
pub(in crate::client) fn keymap_actions(app: &App, km: &KeyMap) -> Vec<Option<Action>> {
    if let Some(step) = km.step {
        return step_items(app, step)
            .into_iter()
            .map(|(_, _, _, a)| Some(a))
            .collect();
    }
    match km.view {
        KeyView::Search => found(app, &km.query).into_iter().map(|e| e.act).collect(),
        view => grid_sections(app, view == KeyView::All)
            .into_iter()
            .flat_map(|s| s.rows.into_iter().map(|r| r.act))
            .collect(),
    }
}

/// A second step's choices: key, what it does, a dim detail, the action.
fn step_items(app: &App, step: Step) -> Vec<(char, &'static str, String, Action)> {
    match step {
        Step::Worktrees => {
            let model = app.screen_model();
            let here = app.focused().and_then(|f| find(&model, f));
            let n = here.map(|(p, _, _)| p.wts.len()).unwrap_or(0);
            let branch = here.map(|(_, w, _)| w.branch.clone()).unwrap_or_default();
            vec![
                (
                    'n',
                    "New worktree",
                    "own branch, new folder, new agent".into(),
                    Action::NewWorktree(None),
                ),
                (
                    's',
                    "Switch to…",
                    format!("{n} worktree{}", if n == 1 { "" } else { "s" }),
                    Action::Jump,
                ),
                ('m', "Merge into main", branch, Action::Changes),
                (
                    'd',
                    "Delete worktree",
                    "keeps the branch".into(),
                    Action::RemoveWorktree,
                ),
            ]
        }
    }
}

fn step_name(step: Step) -> &'static str {
    match step {
        Step::Worktrees => "worktrees",
    }
}

fn step_key(app: &App, step: Step) -> String {
    match step {
        Step::Worktrees => k(app, &Action::Worktrees),
    }
}

/// The overlay's card, centred, and the row inside it where the typing goes.
fn frame(
    app: &mut App,
    buf: &mut Buffer,
    area: Rect,
    t: &Theme,
    title: &str,
    want_h: u16,
) -> (Rect, Rect) {
    dim_all(app, buf, area, t);
    hit(app, area, ScreenHit::Close);
    let w = MAX_W.min(area.width.saturating_sub(4));
    let h = want_h.min(area.height.saturating_sub(2));
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    let mut c = Card::new(t, title).lit(t.sky());
    c.title_fg = t.accent;
    // No fill: the card's own ground is the app's, and the modal's lifted rows are flattened to it.
    c.bg = t.bg;
    c.close = Some(ScreenHit::Close);
    app.screen.modal_rects.push(r);
    let inside = card(app, buf, r, &c, t);
    hit(app, r, ScreenHit::Noop);
    (r, inside)
}

/// The typing row: `› ` and what is typed on a lifted pill.
fn input_row(app: &App, buf: &mut Buffer, r: Rect, sy: u16, t: &Theme, typed: Vec<Seg>) {
    let look = Look::of(&app.cfg.ui);
    row_pill(
        look,
        buf,
        r.x + 3,
        sy,
        r.width.saturating_sub(6),
        t.card2,
        t.card,
    );
    put(buf, r.x + 5, sy, &typed, r.right().saturating_sub(5));
}

fn prompt(t: &Theme, tail: Vec<Seg>) -> Vec<Seg> {
    let ps = Style::default().bg(t.card2);
    let mut typed = vec![seg("› ", ps.fg(t.accent).add_modifier(Modifier::BOLD))];
    typed.extend(tail);
    typed
}

pub(in crate::client) fn draw_keymap(
    app: &mut App,
    f: &mut Frame,
    area: Rect,
    t: &Theme,
    km: &KeyMap,
) {
    let buf = f.buffer_mut();
    if let Some(step) = km.step {
        return draw_step(app, buf, area, t, km, step);
    }
    match km.view {
        KeyView::Search => draw_search(app, buf, area, t, km),
        KeyView::All => draw_keys_help(app, buf, area, t),
        KeyView::Map => draw_grid(app, buf, area, t, km, false),
    }
}

/// The Keys dialog's widest, and the cells a key takes before its words.
const HELP_W: u16 = 92;
const HELP_KEY_W: u16 = 13;
const HELP_NOTE: &str = "Keys only work in key mode ({lead}). Everything else goes to the pane.";

/// A column of the keys help: its title, then each row's words and the actions its key runs.
type HelpColumn = (&'static str, Vec<(&'static str, Vec<Action>)>);

/// The keys help's three columns. `1–9` stands for the nine session keys, `j / k` for the pair.
fn help_columns() -> [HelpColumn; 3] {
    use Action as A;
    [
        (
            "sessions",
            vec![
                ("new session", vec![A::PickSession]),
                ("next / previous", vec![A::NextSession, A::PrevSession]),
                ("jump to session", vec![A::SelectTab(1), A::SelectTab(9)]),
                ("rename", vec![A::RenameWorkspace]),
                ("filter", vec![A::FilterSessions]),
                ("inbox", vec![A::Jump]),
            ],
        ),
        (
            "panes",
            vec![
                ("split right", vec![A::SplitRight]),
                ("split down", vec![A::SplitDown]),
                ("focus next pane", vec![A::FocusNext]),
                ("zoom pane", vec![A::ZoomPane]),
                ("close pane", vec![A::ClosePane]),
                ("move pane", vec![A::SwapNext]),
            ],
        ),
        (
            "window",
            vec![
                ("files", vec![A::Files]),
                ("changes", vec![A::Changes]),
                ("usage and cost", vec![A::Usage]),
                ("settings", vec![A::Settings]),
                ("toggle sidebar", vec![A::ToggleSidebar]),
                ("this help", vec![A::Help]),
            ],
        ),
    ]
}

/// A row's key as bound now: `j / k` for a pair, `1–9` for the jump keys; empty when unbound.
fn help_key(app: &App, acts: &[Action]) -> String {
    let keys: Vec<String> = acts
        .iter()
        .filter_map(|a| {
            app.keymap
                .prefixed_order
                .iter()
                .find(|k| app.keymap.prefixed.get(k) == Some(a))
                .map(super::mode_bar::short_key)
        })
        .collect();
    match (acts.first(), keys.as_slice()) {
        (Some(Action::SelectTab(_)), [first, last]) => format!("{first}–{last}"),
        (_, [one]) => one.clone(),
        (_, [a, b]) => format!("{a} / {b}"),
        _ => String::new(),
    }
}

/// `?`: the keys that matter, three columns (sessions, panes, window) of each key as bound and
/// what it does, on the panel ground in an accent box; `keys` on the top border at the left and
/// `esc close` at the right. Search commands (Ctrl+K) holds every other command.
fn draw_keys_help(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme) {
    dim_all(app, buf, area, t);
    hit(app, area, ScreenHit::Close);
    let columns = help_columns();
    let rows = columns.iter().map(|(_, r)| r.len()).max().unwrap_or(0) as u16;
    let w = HELP_W.min(area.width.saturating_sub(4));
    let h = (rows + 6).min(area.height.saturating_sub(2));
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + area.height.saturating_sub(h) / 2,
        width: w,
        height: h,
    };
    let mut c = Card::new(t, "").lit(t.accent);
    c.bg = t.sidebar_bg;
    let inside = card(app, buf, r, &c, t);
    fill(buf, inside, t.sidebar_bg);
    hit(app, r, ScreenHit::Noop);
    let ground = Style::default().bg(t.sidebar_bg);
    let title = [seg(" keys ", ground.fg(t.strong).add_modifier(Modifier::BOLD))];
    put(buf, r.x + 2, r.y, &title, r.right().saturating_sub(2));
    let close = " esc close ";
    let cx = r.right().saturating_sub(2 + close.width() as u16);
    put(buf, cx, r.y, &[seg(close, ground.fg(t.muted))], r.right().saturating_sub(2));
    hit(app, Rect { x: cx, y: r.y, width: close.width() as u16, height: 1 }, ScreenHit::Close);
    let (x0, end) = (inside.x + 2, inside.right().saturating_sub(2));
    let gap = 4u16;
    let col_w = end.saturating_sub(x0).saturating_sub(2 * gap) / 3;
    // As wide as the design's key column, or as the widest key and a cell where room is short.
    let widest = columns
        .iter()
        .flat_map(|(_, rows)| rows.iter().map(|(_, acts)| help_key(app, acts).width() as u16))
        .max()
        .unwrap_or(1);
    let key_w = if col_w >= HELP_KEY_W + 16 { HELP_KEY_W } else { widest + 1 };
    for (i, (label, items)) in columns.iter().enumerate() {
        let x = x0 + i as u16 * (col_w + gap);
        let col_end = x + col_w;
        put(buf, x, inside.y + 1, &[seg(*label, ground.fg(t.muted))], col_end);
        for (j, (words, acts)) in items.iter().enumerate() {
            let y = inside.y + 2 + j as u16;
            if y >= inside.bottom() {
                break;
            }
            put(buf, x, y, &[seg(help_key(app, acts), ground.fg(t.accent).add_modifier(Modifier::BOLD))], x + key_w);
            put(buf, x + key_w + 1, y, &[seg(*words, ground.fg(t.text))], col_end);
        }
    }
    let note = HELP_NOTE.replace("{lead}", &long_key(&app.keymap.prefix));
    let ny = inside.y + 3 + rows;
    if ny < inside.bottom() {
        put(buf, x0, ny, &[seg(note, ground.fg(t.muted))], end);
    }
}

/// The key hints as chips on the card's bottom border, the way the other modals wear theirs.
fn bar(buf: &mut Buffer, inside: Rect, t: &Theme, keys: &[(&str, &str)]) {
    let border = inside.bottom();
    let segs = cap_hints_gap(t, t.bg, " ", keys);
    put(buf, inside.x + 1, border, &segs, inside.right());
}

fn draw_grid(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, km: &KeyMap, all: bool) {
    let sections = grid_sections(app, all);
    let lead = long_key(&app.keymap.prefix);
    let w = MAX_W.min(area.width.saturating_sub(4));
    let cols = if w >= THREE_COLUMNS_FROM { 3 } else { 2 };
    let col_w = w.saturating_sub(6) / cols as u16;
    let col_of = |s: &Section| if cols == 3 { s.slots.0 } else { s.slots.1 };
    // Rows the tallest column needs, to size the card.
    let column_rows = |c: usize| -> usize {
        sections
            .iter()
            .filter(|s| col_of(s) == c)
            .map(|s| s.rows.len() + 2)
            .sum::<usize>()
            .saturating_sub(1)
    };
    let tallest = (0..cols).map(column_rows).max().unwrap_or(0) as u16;
    // Roomy: air round the typing row, the banner's rule and a note above the bar. A short
    // screen drops those, then the rows scroll (All keys) or the card is as tall as it can be.
    let chrome: u16 = if all { 9 } else { 10 };
    let roomy = area.height.saturating_sub(2) >= chrome + tallest;
    let want = if roomy {
        chrome + tallest
    } else {
        tallest + chrome / 2
    };
    let title = if all {
        "All keys".to_string()
    } else {
        lead.clone()
    };
    let (r, inside) = frame(app, buf, area, t, &title, want);
    let cs = Style::default().bg(t.card);
    let hint = if all {
        " type to search"
    } else {
        " press a key, or type to search"
    };
    let input_y = if roomy { r.y + 2 } else { r.y + 1 };
    input_row(
        app,
        buf,
        r,
        input_y,
        t,
        prompt(
            t,
            vec![
                seg("█", Style::default().bg(t.card2).fg(t.accent)),
                seg(hint, Style::default().bg(t.card2).fg(t.muted)),
            ],
        ),
    );

    let mut y0 = input_y + if roomy { 2 } else { 1 };
    if !all {
        let banner_of = |lead_in: &str, gap: &str| {
            let mut banner = vec![seg(format!("{lead_in}{gap}"), cs.fg(t.text))];
            banner.extend(keycap(t, &lead));
            banner.push(seg(format!("{gap}then{gap}"), cs.fg(t.muted)));
            banner.extend(keycap(t, &k(app, &Action::SplitRight)));
            banner.push(seg(format!("{gap}= split"), cs.fg(t.muted)));
            banner
        };
        let room = r.width.saturating_sub(6);
        let mut banner = banner_of("Every key below goes after the leader:", "  ");
        if segs_width(&banner) > room {
            banner = banner_of("Every key goes after the leader:", " ");
        }
        put(buf, r.x + 3, y0, &banner, r.right().saturating_sub(3));
        y0 += 1;
        if roomy {
            for x in r.x + 3..r.right().saturating_sub(3) {
                put(buf, x, y0, &[seg("─", cs.fg(t.line))], x + 1);
            }
            y0 += 1;
        }
    }
    if roomy {
        y0 += 1;
    }
    let foot = if roomy { 2 } else { 1 };
    let body_h = (inside.bottom().saturating_sub(foot)).saturating_sub(y0) as usize;
    // Index of each row's action in `keymap_actions`, which numbers them section by section.
    let mut index = 0usize;
    let mut starts = Vec::new();
    for s in &sections {
        starts.push(index);
        index += s.rows.len();
    }
    let longest = (0..cols).map(column_rows).max().unwrap_or(0);
    let scroll = if all {
        km.scroll.min(longest.saturating_sub(body_h))
    } else {
        0
    };
    for c in 0..cols {
        let x = r.x + 3 + c as u16 * col_w;
        let mut line = 0usize;
        for (si, s) in sections.iter().enumerate().filter(|(_, s)| col_of(s) == c) {
            // The key column is as wide as this group's widest key, not the column's.
            let kw = s
                .rows
                .iter()
                .map(|row| {
                    if row.keys.is_empty() {
                        1
                    } else {
                        segs_width(&keycaps(t, &row.keys, t.card))
                    }
                })
                .max()
                .unwrap_or(1)
                + 2;
            let mut head = vec![seg(
                s.title.clone(),
                cs.fg(if s.dim { t.muted } else { t.strong })
                    .add_modifier(Modifier::BOLD),
            )];
            if !s.note.is_empty() {
                head.push(seg(format!("  {}", s.note), cs.fg(t.muted)));
            }
            let place = |line: usize| {
                line.checked_sub(scroll)
                    .filter(|l| *l < body_h)
                    .map(|l| y0 + l as u16)
            };
            if let Some(y) = place(line) {
                put(buf, x, y, &head, x + col_w);
            }
            line += 1;
            for (ri, row) in s.rows.iter().enumerate() {
                if let Some(y) = place(line) {
                    let rr = Rect {
                        x,
                        y,
                        width: col_w.saturating_sub(1),
                        height: 1,
                    };
                    let on = row.act.is_some() && hovered(app, rr);
                    let bg = if on { t.sel_on(t.card) } else { t.card };
                    if on {
                        row_pill(
                            Look::of(&app.cfg.ui),
                            buf,
                            x.saturating_sub(1),
                            y,
                            rr.width + 1,
                            bg,
                            t.card,
                        );
                    }
                    let caps = if row.keys.is_empty() {
                        vec![seg("—", Style::default().bg(bg).fg(t.line))]
                    } else {
                        keycaps(t, &row.keys, bg)
                    };
                    put(buf, x, y, &caps, x + col_w);
                    let label = truncate(&row.label, col_w.saturating_sub(kw + 1) as usize);
                    put(
                        buf,
                        x + kw,
                        y,
                        &[seg(
                            label,
                            Style::default().bg(bg).fg(if row.keys.is_empty() {
                                t.muted
                            } else {
                                t.text
                            }),
                        )],
                        x + col_w,
                    );
                    if row.act.is_some() {
                        hit(app, rr, ScreenHit::KeyRow(starts[si] + ri));
                    }
                }
                line += 1;
            }
            line += 1;
        }
    }
    let note_y = inside.bottom().saturating_sub(2);
    if all {
        if let Mode::KeyMap(m) = &mut app.mode {
            m.scroll = scroll;
        }
        if roomy {
            let more = if longest > body_h {
                "  ↑↓ scroll"
            } else {
                ""
            };
            put(
                buf,
                r.x + 3,
                note_y,
                &[seg(
                    format!("Type to search. A dash means no key.{more}"),
                    cs.fg(t.muted),
                )],
                r.right().saturating_sub(3),
            );
        }
        bar(
            buf,
            inside,
            t,
            &[("↑↓", "scroll"), ("Tab", "search"), ("Esc", "close")],
        );
    } else {
        if roomy {
            put(
                buf,
                r.x + 3,
                note_y,
                &[seg("Keys with › open a second step.", cs.fg(t.muted))],
                r.right().saturating_sub(3),
            );
        }
        bar(
            buf,
            inside,
            t,
            &[
                ("key", "run it"),
                ("Tab", "search"),
                ("?", "show all"),
                ("Esc", "close"),
            ],
        );
    }
}

/// The Search commands box: amber border, its title and a pink close on the top edge, the typing
/// row, COMMANDS (RESULTS while typing) and SESSIONS, and the keys on the bottom edge.
fn draw_search(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, km: &KeyMap) {
    let list = found(app, &km.query);
    let sessions = session_hits(app, &km.query);
    dim_all(app, buf, area, t);
    hit(app, area, ScreenHit::Close);
    let w = SEARCH_MAX_W.min(area.width.saturating_sub(4));
    let want = 8 + (list.len() + sessions.len()).min(usize::from(SEARCH_MAX_H)) as u16;
    let h = want.min(SEARCH_MAX_H).min(area.height.saturating_sub(2));
    let r = Rect {
        x: area.x + (area.width - w) / 2,
        y: area.y + (area.height - h) / 2,
        width: w,
        height: h,
    };
    let mut c = Card::new(t, "Search commands").lit(t.blocked);
    c.bg = t.bg;
    c.close = Some(ScreenHit::Close);
    app.screen.modal_rects.push(r);
    let inside = card(app, buf, r, &c, t);
    hit(app, r, ScreenHit::Noop);
    let cs = Style::default().bg(t.card);
    let ps = Style::default().bg(t.card2);
    let mut typed = vec![
        seg(km.query.clone(), ps.fg(t.strong).add_modifier(Modifier::BOLD)),
        seg("█", ps.fg(t.accent)),
    ];
    if km.query.is_empty() {
        typed.push(seg(" type what you want to do", ps.fg(t.muted)));
    }
    input_row(app, buf, r, r.y + 2, t, prompt(t, typed));

    // Rows: a group head, its rows, then the next group. `row` numbers the selectable ones.
    let first_y = r.y + 4;
    let cap = inside.bottom().saturating_sub(1).saturating_sub(first_y) as usize;
    let head_style = cs.fg(t.muted).add_modifier(Modifier::BOLD);
    let mut lines: Vec<(Option<usize>, Vec<Seg>)> = Vec::new();
    let commands_head = if km.query.is_empty() { "COMMANDS" } else { "RESULTS" };
    if !list.is_empty() {
        lines.push((None, vec![seg(commands_head, head_style)]));
        lines.extend((0..list.len()).map(|i| (Some(i), Vec::new())));
    }
    if !sessions.is_empty() {
        lines.push((None, vec![seg("SESSIONS", head_style)]));
        lines.extend((0..sessions.len()).map(|i| (Some(list.len() + i), Vec::new())));
    }
    let at = lines.iter().position(|(i, _)| *i == Some(km.sel)).unwrap_or(0);
    let start = at.saturating_sub(cap.saturating_sub(1));
    let needle: String = km.query.split_whitespace().collect::<String>().to_lowercase();
    for (offset, (_, (index, head))) in lines.iter().enumerate().skip(start).take(cap).enumerate() {
        let y = first_y + offset as u16;
        let Some(i) = *index else {
            put(buf, r.x + 4, y, head, r.right().saturating_sub(3));
            continue;
        };
        let rr = Rect { x: r.x + 3, y, width: r.width.saturating_sub(6), height: 1 };
        let on = i == km.sel || hovered(app, rr);
        let bg = if on { t.sel_on(t.card) } else { t.card };
        if on {
            row_pill(Look::of(&app.cfg.ui), buf, rr.x, y, rr.width, bg, t.card);
            put(buf, rr.x, y, &[seg("▌", Style::default().bg(bg).fg(t.accent).add_modifier(Modifier::BOLD))], rr.x + 1);
        }
        let lit = Style::default().bg(bg).fg(t.accent).add_modifier(Modifier::BOLD);
        if let Some(e) = list.get(i) {
            let plain = Style::default().bg(bg).fg(if e.keys.is_empty() && e.act.is_none() { t.muted } else { t.strong });
            put(buf, r.x + 5, y, &fuzzy_segs(&e.label, &needle, plain, lit), r.right().saturating_sub(34));
            put(
                buf,
                r.right().saturating_sub(32),
                y,
                &[seg(category_of(e.act.as_ref()), Style::default().bg(bg).fg(t.muted))],
                r.right().saturating_sub(4),
            );
            if !e.keys.is_empty() {
                let caps = keycaps(t, &e.keys, bg);
                let cw = segs_width(&caps);
                put(buf, r.right().saturating_sub(cw + 5), y, &caps, r.right().saturating_sub(4));
            }
        } else if let Some(sh) = sessions.get(i - list.len()) {
            let mark = app.state_mark(sh.state, t);
            let dim = sh.state == super::status_mark::State::Unknown;
            let plain = Style::default().bg(bg).fg(if dim { t.idle } else { t.strong });
            put(buf, r.x + 5, y, &[seg(mark.glyph, Style::default().bg(bg).fg(mark.color))], r.x + 7);
            put(buf, r.x + 7, y, &fuzzy_segs(&sh.name, &needle, plain, lit), r.right().saturating_sub(34));
            put(buf, r.right().saturating_sub(32), y, &[seg(sh.place.clone(), Style::default().bg(bg).fg(t.muted))], r.right().saturating_sub(4));
        }
        hit(app, rr, ScreenHit::KeyRow(i));
    }
    if list.is_empty() && sessions.is_empty() {
        put(
            buf,
            r.x + 5,
            first_y,
            &[seg(format!("Nothing matches \"{}\".", km.query.trim()), cs.fg(t.muted))],
            r.right().saturating_sub(3),
        );
    }
    bar(buf, inside, t, &[("↑ ↓", "move"), ("Enter", "run"), ("Esc", "close")]);
}

/// `label` with the letters a fuzzy search matched in the accent.
fn fuzzy_segs(label: &str, needle: &str, plain: Style, lit: Style) -> Vec<Seg> {
    let Some(at) = (!needle.is_empty()).then(|| super::new_session::fuzzy_indices(needle, label)).flatten() else {
        return vec![seg(label, plain)];
    };
    label
        .chars()
        .enumerate()
        .map(|(i, ch)| seg(ch.to_string(), if at.contains(&i) { lit } else { plain }))
        .collect()
}

fn draw_step(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, km: &KeyMap, step: Step) {
    let look = Look::of(&app.cfg.ui);
    let lead = long_key(&app.keymap.prefix);
    let (r, inside) = frame(app, buf, area, t, &lead, 16);
    let cs = Style::default().bg(t.card);
    let ps = Style::default().bg(t.card2);
    let typed = vec![
        seg(
            step_key(app, step),
            ps.fg(t.strong).add_modifier(Modifier::BOLD),
        ),
        seg(format!("  {}", step_name(step)), ps.fg(t.muted)),
        seg("█", ps.fg(t.accent)),
    ];
    input_row(app, buf, r, r.y + 2, t, prompt(t, typed));
    let model = app.screen_model();
    let proj = app
        .focused()
        .and_then(|f| find(&model, f))
        .map(|(p, _, _)| p.name.clone())
        .unwrap_or_default();
    put(
        buf,
        r.x + 4,
        r.y + 5,
        &[seg(
            format!(
                "{} IN {}",
                step_name(step).to_uppercase(),
                proj.to_uppercase()
            ),
            cs.fg(t.muted).add_modifier(Modifier::BOLD),
        )],
        r.right().saturating_sub(2),
    );
    for (i, (key, label, detail, _)) in step_items(app, step).into_iter().enumerate() {
        let y = r.y + 7 + i as u16 * 2;
        if y >= inside.bottom().saturating_sub(3) {
            break;
        }
        let rr = Rect {
            x: r.x + 3,
            y,
            width: r.width.saturating_sub(6),
            height: 1,
        };
        let sel = i == km.sel || hovered(app, rr);
        let bg = if sel { t.sel_on(t.card) } else { t.card };
        if sel {
            row_pill(look, buf, rr.x, y, rr.width, bg, t.card);
        }
        put(buf, r.x + 5, y, &keycap(t, &key.to_string()), r.right());
        let st = Style::default().bg(bg);
        put(
            buf,
            r.x + 11,
            y,
            &[seg(
                label,
                st.fg(if sel { t.strong } else { t.text })
                    .add_modifier(if sel {
                        Modifier::BOLD
                    } else {
                        Modifier::empty()
                    }),
            )],
            r.x + 31,
        );
        put(
            buf,
            r.x + 32,
            y,
            &[seg(detail, st.fg(t.muted))],
            r.right().saturating_sub(3),
        );
        hit(app, rr, ScreenHit::KeyRow(i));
    }
    let mut path = vec![
        seg(lead, cs.fg(t.accent).add_modifier(Modifier::BOLD)),
        seg("  ›  ", cs.fg(t.muted)),
    ];
    path.extend(keycap(t, &step_key(app, step)));
    path.extend([
        seg("  ›  ", cs.fg(t.muted)),
        seg("_", cs.fg(t.muted)),
        seg(
            "   the path so far",
            cs.fg(t.muted).add_modifier(Modifier::ITALIC),
        ),
    ]);
    put(
        buf,
        r.x + 4,
        inside.bottom().saturating_sub(2),
        &path,
        r.right().saturating_sub(2),
    );
    bar(
        buf,
        inside,
        t,
        &[
            ("key", "do it"),
            ("↑↓", "move"),
            ("Backspace", "back"),
            ("Esc", "close"),
        ],
    );
}

impl App {
    /// The leader was pressed `since` and the key map shows by itself now.
    pub(in crate::client) fn keymap_shown(&self, since: Instant) -> bool {
        self.cfg.ui.which_key
            && since.elapsed() >= std::time::Duration::from_millis(self.cfg.ui.which_key_delay_ms)
    }

    /// Open the key map (from the leader's pause, or the leader key's button), or one of its
    /// second steps.
    pub(in crate::client) fn open_keymap(&mut self, step: Option<Step>) {
        self.mode = Mode::KeyMap(Box::new(KeyMap::new(KeyView::Map, step)));
    }

    /// Open the overlay on `view`: Space's search, or `?`'s All keys.
    pub(in crate::client) fn open_key_view(&mut self, view: KeyView) {
        self.mode = Mode::KeyMap(Box::new(KeyMap::new(view, None)));
    }

    /// Run a command picked in the overlay, and remember it for RECENT.
    fn keymap_do(&mut self, a: Action) {
        if !matches!(
            a,
            Action::Palette | Action::Help | Action::Menu | Action::None
        ) && let Some(name) = a.to_config()
        {
            let recent = &mut self.screen.saved.recent;
            recent.retain(|n| *n != name);
            recent.insert(0, name);
            recent.truncate(RECENT_KEPT);
            self.screen.save();
        }
        self.mode = Mode::Normal;
        self.act(a);
    }

    /// Run what the overlay's row `i` stands for (a click, or Enter).
    pub(in crate::client) fn keymap_run(&mut self, km: &KeyMap, i: usize) {
        let commands = keymap_actions(self, km);
        if km.view == KeyView::Search && km.step.is_none() && i >= commands.len() {
            // A row of the SESSIONS group: go to it.
            if let Some(hit) = session_hits(self, &km.query).get(i - commands.len()) {
                let term = hit.term;
                self.mode = Mode::Normal;
                self.screen_focus(term);
            }
            return;
        }
        if let Some(Some(a)) = commands.get(i).cloned() {
            self.keymap_do(a);
        }
    }

    /// How many rows the overlay's list has: the commands, then in Search the sessions.
    fn keymap_rows(&self, km: &KeyMap) -> usize {
        let n = keymap_actions(self, km).len();
        if km.view == KeyView::Search && km.step.is_none() {
            n + session_hits(self, &km.query).len()
        } else {
            n
        }
    }

    pub(in crate::client) fn on_keymap_key(&mut self, mut km: KeyMap, k: &KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let n = self.keymap_rows(&km);
        let typed = match k.code {
            KeyCode::Char(c)
                if !k
                    .modifiers
                    .intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) =>
            {
                Some(c)
            }
            _ => None,
        };
        match k.code {
            // The key map is shortcut mode's own overlay: Esc leaves both.
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                self.shortcut_held = false;
                return;
            }
            KeyCode::Tab if km.step.is_none() => {
                km = KeyMap::new(
                    if km.view == KeyView::Search {
                        KeyView::Map
                    } else {
                        KeyView::Search
                    },
                    None,
                );
            }
            _ if km.step.is_some() => match k.code {
                KeyCode::Backspace => km.step = None,
                KeyCode::Up => km.sel = km.sel.saturating_sub(1),
                KeyCode::Down => km.sel = (km.sel + 1).min(n.saturating_sub(1)),
                KeyCode::Enter => {
                    let sel = km.sel;
                    return self.keymap_run(&km, sel);
                }
                KeyCode::Char(c) => {
                    if let Some(step) = km.step
                        && let Some(i) = step_items(self, step)
                            .iter()
                            .position(|(key, ..)| *key == c)
                    {
                        return self.keymap_run(&km, i);
                    }
                }
                _ => {}
            },
            _ => match km.view {
                KeyView::Search => match k.code {
                    KeyCode::Up => km.sel = km.sel.saturating_sub(1),
                    KeyCode::Down => km.sel = (km.sel + 1).min(n.saturating_sub(1)),
                    KeyCode::Char('p') if ctrl => km.sel = km.sel.saturating_sub(1),
                    KeyCode::Char('n') if ctrl => km.sel = (km.sel + 1).min(n.saturating_sub(1)),
                    KeyCode::PageUp => km.sel = km.sel.saturating_sub(PAGE),
                    KeyCode::PageDown => km.sel = (km.sel + PAGE).min(n.saturating_sub(1)),
                    KeyCode::Enter => {
                        let sel = km.sel;
                        return self.keymap_run(&km, sel);
                    }
                    KeyCode::Backspace => {
                        if km.query.pop().is_none() {
                            km = KeyMap::new(KeyView::Map, None);
                        }
                        km.sel = 0;
                    }
                    KeyCode::Char('u') if ctrl => {
                        km.query.clear();
                        km.sel = 0;
                    }
                    _ => {
                        if let Some(c) = typed
                            && km.query.chars().count() < QUERY_MAX
                        {
                            km.query.push(c);
                            km.sel = 0;
                        }
                    }
                },
                KeyView::All => match k.code {
                    KeyCode::Up => km.scroll = km.scroll.saturating_sub(1),
                    KeyCode::Down => km.scroll = km.scroll.saturating_add(1),
                    KeyCode::PageUp => km.scroll = km.scroll.saturating_sub(PAGE * 2),
                    KeyCode::PageDown => km.scroll = km.scroll.saturating_add(PAGE * 2),
                    KeyCode::Home => km.scroll = 0,
                    KeyCode::End => km.scroll = usize::MAX / 2,
                    KeyCode::Backspace => km = KeyMap::new(KeyView::Map, None),
                    _ => {
                        if let Some(c) = typed {
                            km = KeyMap::new(KeyView::Search, None);
                            km.query.push(c);
                        }
                    }
                },
                KeyView::Map => {
                    if k.code == KeyCode::Backspace {
                        self.mode = Mode::Normal;
                        return;
                    }
                    if let Some(a) = self.keymap.prefixed.get(&KeySpec::from_event(k)).cloned() {
                        return self.keymap_do(a);
                    }
                    // An unbound letter starts a search with it.
                    if let Some(c) = typed {
                        km = KeyMap::new(KeyView::Search, None);
                        km.query.push(c);
                    }
                }
            },
        }
        self.mode = Mode::KeyMap(Box::new(km));
    }
}
