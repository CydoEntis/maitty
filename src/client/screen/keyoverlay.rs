//! The one key overlay. `Ctrl+Space` opens the key map (what each key does after the leader;
//! pressing one runs it). Typing, or `Space`, searches every command, keyed or not. `?` lists
//! every key, the ones that need no leader and the commands that have no key.

use super::*;
use crate::client::design::{keycap, long_key};
use crate::layout::Dir;

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
const DEFAULT_RECENT: [fn() -> Action; 4] = [|| Action::Jump, || Action::Message, || Action::SplitRight, || Action::ToggleSidebar];

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
                r("New session", vec![A::StartSession], true),
                r("New session with options…", vec![A::NewPane], true),
                r("Message this session", vec![A::Message], true),
                r("Why this status", vec![A::Why], true),
                r("Menu", vec![A::Menu], true),
                r("Inbox", vec![A::Jump], true),
                r("Rename selected session", vec![A::RenameWorkspace], false),
                r("Stop agent", vec![A::StopAgent], false),
                r("Focus the sidebar", vec![A::BrowseTree], false),
            ],
        ),
        (
            "PANES",
            vec![
                r("Split side by side", vec![A::SplitRight], true),
                r("Split stacked", vec![A::SplitDown], true),
                r("Close pane", vec![A::ClosePane], true),
                r("Move focus", vec![A::Focus(Dir::Left), A::Focus(Dir::Up), A::Focus(Dir::Down), A::Focus(Dir::Right)], true),
                r("Collapse sidebar", vec![A::ToggleSidebar], true),
                r("Hide sidebar", vec![A::HideSidebar], true),
                r("Resize pane", vec![A::Resize(Dir::Left), A::Resize(Dir::Down), A::Resize(Dir::Up), A::Resize(Dir::Right)], false),
                r("Select text", vec![A::CopyMode], false),
                r("Scroll", vec![A::ScrollUp, A::ScrollDown], false),
                r("Jump between commands", vec![A::PrevPrompt, A::NextPrompt], false),
                r("Paste image", vec![A::PasteImage], false),
            ],
        ),
        (
            "TABS",
            vec![
                r("New tab", vec![A::NewTab], true),
                r("Next tab", vec![A::NextTab], true),
                r("Previous tab", vec![A::PrevTab], true),
                r("Rename session", vec![A::RenameTab], true),
                r("Close tab", vec![A::CloseTab], false),
            ],
        ),
        (
            "PROJECT",
            vec![
                r("Worktrees ›", vec![A::Worktrees], true),
                r("Files", vec![A::Files], true),
                r("Changes", vec![A::Changes], true),
                r("New worktree", vec![A::NewWorktree(None)], false),
                r("Find a file", vec![A::Find(0)], false),
                r("Search the code", vec![A::Find(1)], false),
                r("Switch branch", vec![A::Branches], false),
            ],
        ),
        (
            "VIBECASSO",
            vec![
                r("Search commands", vec![A::Palette], true),
                r("Settings", vec![A::Settings], true),
                r("All keys", vec![A::Help], true),
                r("Quit, agents keep running", vec![A::Detach], true),
                r("What happened", vec![A::History], false),
                r("Reload config", vec![A::ReloadConfig], false),
            ],
        ),
    ]
}

/// The group a command belongs to when no row of `key_groups` lists it.
fn group_of(a: &Action) -> &'static str {
    use Action as A;
    if let Some((g, _)) = key_groups().into_iter().find(|(_, rows)| rows.iter().any(|r| r.acts.contains(a))) {
        return g;
    }
    match a {
        A::NewWorkspace | A::CloseWorkspace | A::NewSession | A::AgentStatusSetup => "SESSIONS",
        A::SplitLeft | A::SplitUp | A::FocusNext | A::FocusPrev | A::CloseSplit | A::Search | A::PaneInfo | A::Spawn(..) | A::SideMove(_) => "PANES",
        A::SpawnTab(_) | A::SelectTab(_) => "TABS",
        A::RemoveWorktree | A::TidyWorktrees | A::NewWorktree(_) => "PROJECT",
        _ => "VIBECASSO",
    }
}

/// A group name as it reads in a list: "Sessions".
fn sentence(group: &str) -> String {
    let mut c = group.chars();
    c.next().map(|f| f.to_string() + &c.as_str().to_lowercase()).unwrap_or_default()
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
    app.keymap.prefixed.values().chain(app.keymap.global.values()).any(|b| b == a)
}

/// The rows of the key map, with the keys they have now; unbound ones have empty `keys`.
fn leader_entries(app: &App) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut listed: Vec<Action> = Vec::new();
    for (group, rows) in key_groups() {
        for row in rows {
            listed.extend(row.acts.iter().cloned());
            let also = row.acts.iter().map(Action::describe).collect::<Vec<_>>().join(" ");
            out.push(Entry { group: sentence(group), label: row.label.into(), also, keys: key_text(app, &row.acts), act: (row.acts.len() == 1).then(|| row.acts[0].clone()) });
        }
    }
    // Keys of the user's own (a spawn binding, an alias for something else) belong to a group too.
    let mut seen: Vec<Action> = Vec::new();
    for key in &app.keymap.prefixed_order {
        let Some(a) = app.keymap.prefixed.get(key) else { continue };
        if matches!(a, Action::None | Action::SendPrefix | Action::SideMove(_)) || listed.contains(a) || seen.contains(a) {
            continue;
        }
        seen.push(a.clone());
        out.push(Entry { group: sentence(group_of(a)), label: a.describe(), also: String::new(), keys: key_text(app, std::slice::from_ref(a)), act: Some(a.clone()) });
    }
    out
}

/// The keys that need no leader.
fn direct_entries(app: &App) -> Vec<Entry> {
    let mut out = Vec::new();
    let mut tabs = false;
    let mut focus = false;
    let mut keys: Vec<_> = app.keymap.global.iter().filter(|(_, a)| **a != Action::None).collect();
    keys.sort_by_key(|(key, _)| key.to_config());
    for (key, a) in keys {
        match a {
            Action::SelectTab(_) => {
                if !tabs && let Some((k1, _)) = app.keymap.global.iter().find(|(_, b)| **b == Action::SelectTab(1)) {
                    tabs = true;
                    let one = long_key(k1);
                    out.push(Entry { group: "No leader".into(), label: "Go to tab 1–9".into(), also: String::new(), keys: format!("{}1-9", one.trim_end_matches('1')), act: None });
                }
            }
            Action::Focus(_) => {
                if !focus {
                    focus = true;
                    let mods = long_key(key);
                    let arrows = mods.rsplit_once('+').map(|(m, _)| format!("{m}+arrows")).unwrap_or_else(|| "arrows".into());
                    out.push(Entry { group: "No leader".into(), label: "Move focus".into(), also: String::new(), keys: arrows, act: None });
                }
            }
            other => out.push(Entry { group: "No leader".into(), label: other.describe(), also: String::new(), keys: long_key(key), act: Some(other.clone()) }),
        }
    }
    out
}

/// Commands with no key: every one the palette used to list, and the rows of the map that
/// the user has unbound.
fn search_only_entries(app: &App, leader: &[Entry]) -> Vec<Entry> {
    let mut out: Vec<Entry> = leader.iter().filter(|e| e.keys.is_empty()).cloned().collect();
    for a in app.palette_commands() {
        if is_bound(app, &a) || key_groups().iter().any(|(_, rows)| rows.iter().any(|r| r.acts.contains(&a))) {
            continue;
        }
        out.push(Entry { group: sentence(group_of(&a)), label: a.describe(), also: String::new(), keys: String::new(), act: Some(a) });
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
        rows.into_iter().filter(|r| r.acts.len() > 1).flat_map(move |r| r.acts.into_iter().map(move |a| (group, a)))
    });
    let singles: Vec<Entry> = singles
        .map(|(group, a)| Entry { group: sentence(group), label: a.describe(), also: String::new(), keys: key_text(app, std::slice::from_ref(&a)), act: Some(a) })
        .collect();
    let mut out: Vec<Entry> = leader.into_iter().filter(|e| !e.keys.is_empty() && e.act.is_some()).chain(singles).chain(direct).chain(rest).collect();
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
    all_entries(app).into_iter().map(|e| (e.label, e.keys)).collect()
}

/// The commands the search shows: the latest run when nothing is typed, else the ones whose
/// name or group has every typed word (names before groups).
fn found(app: &App, query: &str) -> Vec<Entry> {
    let all = all_entries(app);
    let words: Vec<String> = query.to_lowercase().split_whitespace().map(str::to_string).collect();
    if words.is_empty() {
        let mut out: Vec<Entry> = Vec::new();
        let names = app.screen.saved.recent.iter().filter_map(|n| n.parse::<Action>().ok()).chain(DEFAULT_RECENT.iter().map(|f| f()));
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
    let mut hits: Vec<(bool, Entry)> = all
        .into_iter()
        .filter_map(|e| {
            let label = format!("{} {}", e.label, e.also).to_lowercase();
            let group = e.group.to_lowercase();
            words.iter().all(|w| label.contains(w.as_str()) || group.contains(w.as_str())).then(|| (words.iter().all(|w| label.contains(w.as_str())), e))
        })
        .collect();
    hits.sort_by_key(|(by_name, _)| !*by_name);
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
fn grid_sections(app: &App, all: bool) -> Vec<Section> {
    let leader = leader_entries(app);
    let core: Vec<&str> = key_groups().into_iter().flat_map(|(_, rows)| rows.into_iter().filter(|r| r.core).map(|r| r.label)).collect();
    let mut out: Vec<Section> = Vec::new();
    for (i, (group, _)) in key_groups().into_iter().enumerate() {
        let name = sentence(group);
        let rows: Vec<GridRow> = leader
            .iter()
            .filter(|e| e.group == name && !e.keys.is_empty() && (all || core.contains(&e.label.as_str())))
            .map(|e| GridRow { keys: e.keys.clone(), label: e.label.clone(), act: e.act.clone() })
            .collect();
        if !rows.is_empty() {
            out.push(Section { title: group.to_string(), dim: false, note: "", slots: [(0, 0), (1, 0), (1, 1), (0, 1), (2, 1)][i], rows });
        }
    }
    if all {
        let direct: Vec<GridRow> = direct_entries(app).into_iter().map(|e| GridRow { keys: e.keys, label: e.label, act: e.act }).collect();
        if !direct.is_empty() {
            out.push(Section { title: "WITHOUT THE LEADER".into(), dim: true, note: "press directly", slots: (2, 1), rows: direct });
        }
        let only: Vec<GridRow> = search_only_entries(app, &leader).into_iter().map(|e| GridRow { keys: String::new(), label: e.label, act: e.act }).collect();
        if !only.is_empty() {
            out.push(Section { title: "SEARCH ONLY".into(), dim: true, note: "", slots: (2, 1), rows: only });
        }
    }
    out
}

/// What each shown row runs, in the order its rows are numbered (a click on row `i`).
pub(in crate::client) fn keymap_actions(app: &App, km: &KeyMap) -> Vec<Option<Action>> {
    if let Some(step) = km.step {
        return step_items(app, step).into_iter().map(|(_, _, _, a)| Some(a)).collect();
    }
    match km.view {
        KeyView::Search => found(app, &km.query).into_iter().map(|e| e.act).collect(),
        view => grid_sections(app, view == KeyView::All).into_iter().flat_map(|s| s.rows.into_iter().map(|r| r.act)).collect(),
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
                ('n', "New worktree", "own branch, new folder, new agent".into(), Action::NewWorktree(None)),
                ('s', "Switch to…", format!("{n} worktree{}", if n == 1 { "" } else { "s" }), Action::Jump),
                ('m', "Merge into main", branch, Action::Changes),
                ('d', "Delete worktree", "keeps the branch".into(), Action::RemoveWorktree),
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
fn frame(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, title: &str, want_h: u16) -> (Rect, Rect) {
    dim_all(buf, area, t);
    hit(app, area, ScreenHit::Close);
    let w = MAX_W.min(area.width.saturating_sub(4));
    let h = want_h.min(area.height.saturating_sub(2));
    let r = Rect { x: area.x + (area.width - w) / 2, y: area.y + (area.height - h) / 2, width: w, height: h };
    let mut c = Card::new(t, title).lit(t.sky());
    c.title_fg = t.accent;
    c.bg = t.card;
    let inside = card(app, buf, r, &c, t);
    hit(app, r, ScreenHit::Noop);
    (r, inside)
}

/// The typing row: `› ` and what is typed on a lifted pill.
fn input_row(app: &App, buf: &mut Buffer, r: Rect, sy: u16, t: &Theme, typed: Vec<Seg>) {
    let look = Look::of(&app.cfg.ui);
    row_pill(look, buf, r.x + 3, sy, r.width.saturating_sub(6), t.card2, t.card);
    put(buf, r.x + 5, sy, &typed, r.right().saturating_sub(5));
}

fn prompt(t: &Theme, tail: Vec<Seg>) -> Vec<Seg> {
    let ps = Style::default().bg(t.card2);
    let mut typed = vec![seg("› ", ps.fg(t.accent).add_modifier(Modifier::BOLD))];
    typed.extend(tail);
    typed
}

pub(in crate::client) fn draw_keymap(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, km: &KeyMap) {
    let buf = f.buffer_mut();
    if let Some(step) = km.step {
        return draw_step(app, buf, area, t, km, step);
    }
    match km.view {
        KeyView::Search => draw_search(app, buf, area, t, km),
        view => draw_grid(app, buf, area, t, km, view == KeyView::All),
    }
}

/// The bar of key hints in the overlay's last row.
fn bar(buf: &mut Buffer, inside: Rect, t: &Theme, keys: &[(&str, &str)]) {
    let row = Rect { y: inside.bottom().saturating_sub(1), height: 1, ..inside };
    action_bar(buf, row, t, t.card2, keys);
}

fn draw_grid(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, km: &KeyMap, all: bool) {
    let sections = grid_sections(app, all);
    let lead = app.keymap.prefix.to_string().replace("C-", "Ctrl+");
    let w = MAX_W.min(area.width.saturating_sub(4));
    let cols = if w >= THREE_COLUMNS_FROM { 3 } else { 2 };
    let col_w = w.saturating_sub(6) / cols as u16;
    let col_of = |s: &Section| if cols == 3 { s.slots.0 } else { s.slots.1 };
    // Rows the tallest column needs, to size the card.
    let column_rows = |c: usize| -> usize { sections.iter().filter(|s| col_of(s) == c).map(|s| s.rows.len() + 2).sum::<usize>().saturating_sub(1) };
    let tallest = (0..cols).map(column_rows).max().unwrap_or(0) as u16;
    // Roomy: air round the typing row, the banner's rule and a note above the bar. A short
    // screen drops those, then the rows scroll (All keys) or the card is as tall as it can be.
    let chrome: u16 = if all { 9 } else { 10 };
    let roomy = area.height.saturating_sub(2) >= chrome + tallest;
    let want = if roomy { chrome + tallest } else { tallest + chrome / 2 };
    let title = if all { "All keys".to_string() } else { lead.clone() };
    let (r, inside) = frame(app, buf, area, t, &title, want);
    let cs = Style::default().bg(t.card);
    let hint = if all { " type to search" } else { " press a key, or type to search" };
    let input_y = if roomy { r.y + 2 } else { r.y + 1 };
    input_row(app, buf, r, input_y, t, prompt(t, vec![seg("█", Style::default().bg(t.card2).fg(t.accent)), seg(hint, Style::default().bg(t.card2).fg(t.muted))]));

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
    let scroll = if all { km.scroll.min(longest.saturating_sub(body_h)) } else { 0 };
    for c in 0..cols {
        let x = r.x + 3 + c as u16 * col_w;
        let mut line = 0usize;
        for (si, s) in sections.iter().enumerate().filter(|(_, s)| col_of(s) == c) {
            // The key column is as wide as this group's widest key, not the column's.
            let kw = s.rows.iter().map(|row| if row.keys.is_empty() { 1 } else { segs_width(&keycaps(t, &row.keys, t.card)) }).max().unwrap_or(1) + 2;
            let mut head = vec![seg(s.title.clone(), cs.fg(if s.dim { t.muted } else { t.strong }).add_modifier(Modifier::BOLD))];
            if !s.note.is_empty() {
                head.push(seg(format!("  {}", s.note), cs.fg(t.muted)));
            }
            let place = |line: usize| line.checked_sub(scroll).filter(|l| *l < body_h).map(|l| y0 + l as u16);
            if let Some(y) = place(line) {
                put(buf, x, y, &head, x + col_w);
            }
            line += 1;
            for (ri, row) in s.rows.iter().enumerate() {
                if let Some(y) = place(line) {
                    let rr = Rect { x, y, width: col_w.saturating_sub(1), height: 1 };
                    let on = row.act.is_some() && hovered(app, rr);
                    let bg = if on { t.sel_on(t.card) } else { t.card };
                    if on {
                        row_pill(Look::of(&app.cfg.ui), buf, x.saturating_sub(1), y, rr.width + 1, bg, t.card);
                    }
                    let caps = if row.keys.is_empty() { vec![seg("—", Style::default().bg(bg).fg(t.line))] } else { keycaps(t, &row.keys, bg) };
                    put(buf, x, y, &caps, x + col_w);
                    let label = truncate(&row.label, col_w.saturating_sub(kw + 1) as usize);
                    put(buf, x + kw, y, &[seg(label, Style::default().bg(bg).fg(if row.keys.is_empty() { t.muted } else { t.text }))], x + col_w);
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
            let more = if longest > body_h { "  ↑↓ scroll" } else { "" };
            put(buf, r.x + 3, note_y, &[seg(format!("Type to search. A dash means no key.{more}"), cs.fg(t.muted))], r.right().saturating_sub(3));
        }
        bar(buf, inside, t, &[("↑↓", "scroll"), ("Tab", "search"), ("Esc", "close")]);
    } else {
        if roomy {
            put(buf, r.x + 3, note_y, &[seg("Keys with › open a second step.", cs.fg(t.muted))], r.right().saturating_sub(3));
        }
        bar(buf, inside, t, &[("key", "run it"), ("Tab", "search"), ("?", "show all"), ("Esc", "close")]);
    }
}

/// `label` with the first place `needle` (any case) occurs in the accent.
fn highlighted(label: &str, needle: &str, plain: Style, lit: Style) -> Vec<Seg> {
    let hay = label.to_lowercase();
    let at = if needle.is_empty() { None } else { hay.find(needle).filter(|_| hay.len() == label.len()) };
    match at {
        Some(i) if label.is_char_boundary(i) && label.is_char_boundary(i + needle.len()) => {
            [(&label[..i], plain), (&label[i..i + needle.len()], lit), (&label[i + needle.len()..], plain)]
                .into_iter()
                .filter(|(s, _)| !s.is_empty())
                .map(|(s, st)| seg(s, st))
                .collect()
        }
        _ => vec![seg(label, plain)],
    }
}

fn draw_search(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, km: &KeyMap) {
    let list = found(app, &km.query);
    let want = 12 + list.len().min(12) as u16;
    let (r, inside) = frame(app, buf, area, t, "Search commands", want);
    let cs = Style::default().bg(t.card);
    let ps = Style::default().bg(t.card2);
    let mut typed = vec![seg(km.query.clone(), ps.fg(t.strong).add_modifier(Modifier::BOLD)), seg("█", ps.fg(t.accent))];
    if km.query.is_empty() {
        typed.push(seg(" type what you want to do", ps.fg(t.muted)));
    }
    input_row(app, buf, r, r.y + 2, t, prompt(t, typed));
    let head_y = r.y + 4;
    let head = if km.query.is_empty() { "RECENT".to_string() } else { format!("{} match{}", list.len(), if list.len() == 1 { "" } else { "es" }) };
    put(buf, r.x + 4, head_y, &[seg(head, cs.fg(t.muted).add_modifier(Modifier::BOLD))], r.right().saturating_sub(3));
    let cap = (inside.bottom().saturating_sub(2)).saturating_sub(head_y + 1) as usize;
    let start = km.sel.saturating_sub(cap.saturating_sub(1));
    let needle = km.query.trim().to_lowercase();
    let first_word = needle.split_whitespace().next().unwrap_or("").to_string();
    for (i, e) in list.iter().enumerate().skip(start).take(cap) {
        let y = head_y + 1 + (i - start) as u16;
        let rr = Rect { x: r.x + 3, y, width: r.width.saturating_sub(6), height: 1 };
        let on = i == km.sel || hovered(app, rr);
        let bg = if on { t.sel_on(t.card) } else { t.card };
        if on {
            row_pill(Look::of(&app.cfg.ui), buf, rr.x, y, rr.width, bg, t.card);
            put(buf, rr.x + 1, y, &[seg("▎", Style::default().bg(bg).fg(t.accent).add_modifier(Modifier::BOLD))], rr.x + 2);
        }
        let plain = Style::default().bg(bg).fg(if e.keys.is_empty() && e.act.is_none() { t.muted } else { t.strong });
        let lit = Style::default().bg(bg).fg(t.accent).add_modifier(Modifier::BOLD);
        let n = if e.label.to_lowercase().contains(&needle) { &needle } else { &first_word };
        let label = highlighted(&e.label, n, plain, lit);
        put(buf, r.x + 5, y, &label, r.right().saturating_sub(34));
        put(buf, r.right().saturating_sub(32), y, &[seg(e.group.clone(), Style::default().bg(bg).fg(t.muted))], r.right().saturating_sub(4));
        if !e.keys.is_empty() {
            let caps = keycaps(t, &e.keys, bg);
            let cw = segs_width(&caps);
            put(buf, r.right().saturating_sub(cw + 5), y, &caps, r.right().saturating_sub(4));
        }
        hit(app, rr, ScreenHit::KeyRow(i));
    }
    if list.is_empty() {
        put(buf, r.x + 5, head_y + 1, &[seg(format!("Nothing matches \"{}\".", km.query.trim()), cs.fg(t.muted))], r.right().saturating_sub(3));
    }
    bar(buf, inside, t, &[("↑↓", "move"), ("Enter", "run"), ("Tab", "key map"), ("Esc", "close")]);
}

fn draw_step(app: &mut App, buf: &mut Buffer, area: Rect, t: &Theme, km: &KeyMap, step: Step) {
    let look = Look::of(&app.cfg.ui);
    let lead = app.keymap.prefix.to_string().replace("C-", "Ctrl+");
    let (r, inside) = frame(app, buf, area, t, &lead, 16);
    let cs = Style::default().bg(t.card);
    let ps = Style::default().bg(t.card2);
    let typed = vec![seg(step_key(app, step), ps.fg(t.strong).add_modifier(Modifier::BOLD)), seg(format!("  {}", step_name(step)), ps.fg(t.muted)), seg("█", ps.fg(t.accent))];
    input_row(app, buf, r, r.y + 2, t, prompt(t, typed));
    let model = app.screen_model();
    let proj = app.focused().and_then(|f| find(&model, f)).map(|(p, _, _)| p.name.clone()).unwrap_or_default();
    put(buf, r.x + 4, r.y + 5, &[seg(format!("{} IN {}", step_name(step).to_uppercase(), proj.to_uppercase()), cs.fg(t.muted).add_modifier(Modifier::BOLD))], r.right().saturating_sub(2));
    for (i, (key, label, detail, _)) in step_items(app, step).into_iter().enumerate() {
        let y = r.y + 7 + i as u16 * 2;
        if y >= inside.bottom().saturating_sub(3) {
            break;
        }
        let rr = Rect { x: r.x + 3, y, width: r.width.saturating_sub(6), height: 1 };
        let sel = i == km.sel || hovered(app, rr);
        let bg = if sel { t.sel_on(t.card) } else { t.card };
        if sel {
            row_pill(look, buf, rr.x, y, rr.width, bg, t.card);
        }
        put(buf, r.x + 5, y, &keycap(t, &key.to_string()), r.right());
        let st = Style::default().bg(bg);
        put(buf, r.x + 11, y, &[seg(label, st.fg(if sel { t.strong } else { t.text }).add_modifier(if sel { Modifier::BOLD } else { Modifier::empty() }))], r.x + 31);
        put(buf, r.x + 32, y, &[seg(detail, st.fg(t.muted))], r.right().saturating_sub(3));
        hit(app, rr, ScreenHit::KeyRow(i));
    }
    let mut path = vec![seg(lead, cs.fg(t.accent).add_modifier(Modifier::BOLD)), seg("  ›  ", cs.fg(t.muted))];
    path.extend(keycap(t, &step_key(app, step)));
    path.extend([seg("  ›  ", cs.fg(t.muted)), seg("_", cs.fg(t.muted)), seg("   the path so far", cs.fg(t.muted).add_modifier(Modifier::ITALIC))]);
    put(buf, r.x + 4, inside.bottom().saturating_sub(2), &path, r.right().saturating_sub(2));
    bar(buf, inside, t, &[("key", "do it"), ("↑↓", "move"), ("Backspace", "back"), ("Esc", "close")]);
}

impl App {
    /// The leader was pressed `since` and the key map shows by itself now.
    pub(in crate::client) fn keymap_shown(&self, since: Instant) -> bool {
        self.cfg.ui.which_key && since.elapsed() >= std::time::Duration::from_millis(self.cfg.ui.which_key_delay_ms)
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
        if !matches!(a, Action::Palette | Action::Help | Action::Menu | Action::None)
            && let Some(name) = a.to_config()
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
        if let Some(Some(a)) = keymap_actions(self, km).get(i).cloned() {
            self.keymap_do(a);
        }
    }

    pub(in crate::client) fn on_keymap_key(&mut self, mut km: KeyMap, k: &KeyEvent) {
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        let n = keymap_actions(self, &km).len();
        let typed = match k.code {
            KeyCode::Char(c) if !k.modifiers.intersects(KeyModifiers::CONTROL | KeyModifiers::ALT) => Some(c),
            _ => None,
        };
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Tab if km.step.is_none() => {
                km = KeyMap::new(if km.view == KeyView::Search { KeyView::Map } else { KeyView::Search }, None);
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
                        && let Some(i) = step_items(self, step).iter().position(|(key, ..)| *key == c)
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
