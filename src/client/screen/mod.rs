//! The app's layout: projects (git repos) in the sidebar, each
//! project's worktrees with the sessions running in them, and the focused session full size
//! (optionally split with a second). Most tools are centred overlays over a dimmed
//! screen: jump, new pane, talk and keys. Settings uses a centred modal; review uses a right-hand sheet
//! beside the panes.
//!
//! The daemon only knows workspaces, tabs and terminals. Here every terminal is a session;
//! its project and worktree come from where it runs (`TermInfo::root` / `top`), so a shell
//! that `cd`s into another repo moves to that project by itself.

use super::design::{
    BtnKind, SRow, Seg, SettingsView, button, cap_hints, cap_hints_fit, cap_hints_gap, fill, glyph, key_text, keycaps, path_key, put,
    question, seg, segs_width, state_label, tilde,
};
use super::render::{blend, render_screen, truncate};
use crate::brand::{APP_COMMAND};
use super::{App, Hit, Mode};
use crate::keys::Action;
use crate::protocol::{CheckInfo, Status, SubagentActivity, TermId, TermInfo};
use crate::theme::Theme;
use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::time::Instant;
use unicode_width::UnicodeWidthStr;

use super::design::SettingsView as SView;
use crate::keys::KeySpec;
use crate::protocol::Command;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

// ---- state ---------------------------------------------------------------------------------

/// What the user chose that the daemon doesn't track: the colour order, folded rows.
/// Saved to `<app>-ui.json` in the data folder.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub(super) struct Saved {
    /// Project keys in the order they were first seen; a project's colour is its index.
    pub order: Vec<String>,
    /// Folded sidebar rows ("p:<project key>").
    pub closed: Vec<String>,
    /// Sessions in the order you dragged them to (within a group; what needs you still
    /// comes first).
    pub session_order: Vec<TermId>,
    /// Sessions you dragged into another group: the session, that group's folder, and
    /// whether it's a git repo. Kept while the session lives.
    pub placed: Vec<(TermId, PathBuf, bool)>,
    /// Projects whose BRANCHES list is unfolded.
    pub open_branches: Vec<String>,
    /// Sidebar width you dragged it to, and the split's share for the left (or top) half.
    pub side_w: Option<u16>,
    pub split: Option<f32>,
    /// A task typed into + New and not started yet.
    pub draft: String,
    /// The agent last started from the app (what `sessions.agent = last used` starts).
    pub last_agent: String,
    /// Projects you renamed: key -> name.
    pub names: HashMap<String, String>,
    /// Files marked reviewed in Changes: folder key -> file -> what it was like then.
    pub reviewed: HashMap<String, HashMap<String, u64>>,
    /// Commands run from the key overlay, latest first (config spelling), for its RECENT.
    pub recent: Vec<String>,
    /// The side sheet's width per kind of view (`SheetKind::key`), as you sized it.
    pub sheet_w: HashMap<String, u16>,
}

#[derive(Debug, Default)]
pub(super) struct Screen {
    /// A question asked from the sidebar: where its cursor goes after (the row below the
    /// one being closed), so the keys stay in the sidebar.
    pub side_return: Option<SideItem>,
    /// A question asked from Settings: the page to go back to once it is answered.
    pub settings_return: Option<Box<SView>>,
    pub saved: Saved,
    /// The project of what you're on (the default for + New).
    pub proj: Option<String>,
    /// Two sessions side by side: (left, right). One of them is the focused one.
    /// Tabs: each shows one session or any number side by side.
    pub tabs: Vec<ScreenTab>,
    /// Panes whose program gets right-clicks (no the app menu there; Shift+right-click for it).
    pub right_clicks: HashSet<TermId>,
    /// Which way the next "split" goes.
    pub split_dir: Option<crate::layout::Dir>,
    /// Where the bottom bar's "where you are" starts (lined up with the panes).
    pub crumb_x: u16,
    /// Where the file preview was drawn (wheel scrolls it; paging uses its height).
    pub preview_rect: Rect,
    pub tab: usize,
    /// The tab whose focused pane fills it (`⤢`).
    pub zoom: Option<usize>,
    /// A new tab was asked for in this session: the shell that answers it goes in it. Starting
    /// any other session clears it (`App::cmd`), so a session you start instead is never
    /// taken for the tab. A `RefCell` because `cmd` only borrows the app.
    pub new_tab: std::cell::RefCell<Option<PendingTab>>,
    /// Where the toast is drawn this frame (empty: none). It sits over the panes, so a click
    /// there is the toast's, never the program's under it.
    pub toast_rect: Rect,
    /// A session picked from the sidebar, the go-to list or a sheet: it shows its own tab, not
    /// the one you were last on in it. Taken by the next focus change.
    pub show_own: Option<TermId>,
    /// Counts frames, for `ScreenTab::used`.
    pub tick: u64,
    /// The modals drawn this frame, which are flattened onto the screen's ground at its end.
    pub modal_rects: Vec<Rect>,
    /// Where the sessions were drawn last frame, and the splits between them.
    pub leaf_rects: Vec<(TermId, Rect)>,
    pub dividers: Vec<(Rect, bool, Vec<bool>)>,
    /// Sidebar cursor (keyboard browsing; bare keys work while it's set).
    pub cursor: Option<TermId>,
    /// The sidebar cursor on a project row instead (its key).
    pub cursor_proj: Option<String>,
    /// The sidebar cursor on one of `cursor`'s session's tab lines (its place in `tabs`).
    pub cursor_tab: Option<usize>,
    /// What the sidebar cursor walks over, top to bottom.
    pub side_items: Vec<SideItem>,
    /// Projects opened this run that haven't been looked at (NEW chip).
    pub fresh: HashSet<String>,
    /// A session about to open beside this one: pair them when it appears.
    pub pending_split: Option<(TermId, Instant)>,
    /// The focus last time the state came in.
    pub last_focus: Option<TermId>,
    /// Sidebar scroll, and whether to bring the focused row into view on the next draw.
    pub side_scroll: u16,
    pub follow: bool,
    pub side_rect: Rect,
    /// The sidebar drew the Update (or Restart) chip this frame, so the bottom bar leaves it out.
    pub release_in_side: bool,
    /// Where the side sheet was drawn, and the widest it could be there (its edge's drag and
    /// `[` `]` keep within it).
    pub sheet_rect: Rect,
    pub sheet_widest: u16,
    /// The screen's width when it was drawn (Changes starts at half of it).
    pub sheet_screen_w: u16,
    /// Drawn this frame: sidebar sessions in order, project keys, worktree and branch rows.
    pub visible: Vec<TermId>,
    pub proj_keys: Vec<String>,
    pub wt_keys: Vec<(String, PathBuf)>,
    /// The keys the mode bar's suggestions press, drawn this frame.
    pub bar_keys: Vec<crate::keys::KeySpec>,
    pub branch_keys: Vec<(PathBuf, String)>,
    /// The sidebar model, built once per event / frame (see `screen_fresh`).
    pub model_cache: std::cell::RefCell<Option<Vec<Proj>>>,
    /// A recipe's worktree being made: (branch, the commands to start there, since).
    pub pending_recipe: Option<(String, Vec<String>, Instant)>,
    /// Files changed in more than one checkout, per repo folder, and when each repo was
    /// last looked at.
    pub overlaps: HashMap<PathBuf, Vec<crate::client::overlap::Overlap>>,
    /// Heads-ups you dismissed: (repo, file). One comes back once it clears and recurs.
    pub dismissed: HashSet<(PathBuf, String)>,
    /// A merge (or throw-away) asked inside an Inbox row: (session, merge?, under way?).
    pub inbox_confirm: Option<(TermId, bool, bool)>,
    /// Where the Inbox was (query, row) when the override or approval open over it came from
    /// it: Esc goes back there.
    pub inbox_return: Option<(String, usize)>,
    /// What you have typed into a secret's prompt so far. Cleared when the prompt closes.
    pub secret_typed: crate::protocol::SecretText,
    /// Secret requests already put in front of you, so Esc is not answered by the same one
    /// opening again.
    pub secret_offered: HashSet<u64>,
    /// While set and in the future, a prompt that opened by itself takes no keys: what you were
    /// typing a moment ago must not become the secret.
    pub secret_guard: Option<Instant>,
    pub overlap_at: HashMap<PathBuf, Instant>,
    /// Each checkout's lines added and removed and not committed (`+N −N` on its rows), and
    /// when they were last counted.
    pub lines_changed: HashMap<PathBuf, (i64, i64)>,
    pub lines_at: HashMap<PathBuf, Instant>,
    /// Dragging the sidebar edge or the split divider.
    pub drag: Option<Drag>,
    /// The group being dragged in the sidebar (its key).
    pub drag_key: Option<String>,
    /// The splash's selected button.
    pub splash_sel: usize,
    /// The colour of each project's +/− button, picked at random when it opens (key -> colour).
    pub fold_colors: HashMap<String, Color>,
    /// The scrollbar being dragged: (pane, track, lines of history).
    pub bar: Option<(TermId, Rect, usize)>,
    /// Where each agent row was drawn in the sidebar (for the follow-up box beside it).
    pub row_y: std::collections::HashMap<TermId, u16>,
    /// What was just sent or queued to each session, shown on its row for a few seconds.
    pub receipts: std::collections::HashMap<TermId, popover::Receipt>,
    /// Quiet agents you marked seen in the Inbox, by when they went quiet: listed again only
    /// once they go quiet anew.
    pub quiet_seen: std::collections::HashMap<TermId, u64>,
    /// A follow-up left unsent with Esc, kept per session until it is sent or cleared.
    pub drafts: std::collections::HashMap<TermId, String>,
    /// What you changed on Settings › Check and have not saved.
    pub check_page: Option<check_settings::CheckDraft>,
    /// A failed check waiting for its output before its message opens (`m`).
    pub failure_ask: Option<failure_note::FailureAsk>,
}

/// A row the sidebar cursor can rest on.
#[derive(Debug, Clone, PartialEq)]
pub(super) enum SideItem {
    Proj(String),
    Sess(TermId),
    /// A tab line under its session (owner, place in `tabs`).
    Tab(TermId, usize),
}

impl Screen {
    /// The tab line the sidebar cursor rests on, if it still belongs to the cursor's session.
    pub(super) fn cursor_tab_line(&self) -> Option<usize> {
        let (i, owner) = (self.cursor_tab?, self.cursor?);
        self.tabs.get(i).is_some_and(|t| t.owner == owner).then_some(i)
    }
}

/// A tab asked for and not yet answered: whose it is, and the panes that existed when it was
/// asked for, none of which can be its shell.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct PendingTab {
    pub at: Instant,
    pub owner: TermId,
    pub known: HashSet<TermId>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct ScreenTab {
    pub layout: crate::layout::Node,
    /// The one you were on in it.
    pub focus: TermId,
    /// How its panes are laid out.
    /// The session (sidebar row) it's a tab of. A session's tabs share its row; the tab
    /// bar shows the tabs of the session you're on, once it has more than one.
    pub owner: TermId,
    /// When it was last on screen (higher is later): a session's row brings that tab back.
    pub used: u64,
    /// What you called it; empty: named after what runs in it.
    pub name: String,
}

/// A tab being renamed in its pill.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct TabName {
    pub tab: usize,
    pub name: String,
}

/// The second step of a leader key that opens one (`w` worktrees).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Step {
    Worktrees,
}

/// The one key overlay: the key map, the search over every command, or All keys.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct KeyMap {
    pub view: KeyView,
    pub query: String,
    pub step: Option<Step>,
    /// The selected row (in a step, or while searching).
    pub sel: usize,
    /// How far All keys is scrolled, in rows.
    pub scroll: usize,
}

impl KeyMap {
    pub(super) fn new(view: KeyView, step: Option<Step>) -> KeyMap {
        KeyMap { view, query: String::new(), step, sel: 0, scroll: 0 }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum Drag {
    Side,
    /// The side sheet's left edge.
    Sheet,
    /// A divider between sessions (index into `dividers`).
    Divider(usize),
    /// A pane's scrollbar.
    Scroll(TermId),
    /// A group's header in the sidebar (its index when pressed; moved yet?): dragged onto
    /// another group it takes that place, released where it was it folds.
    Group(usize, bool),
    /// A session's row (moved yet?; what was pressed, the row or its `+N −N`): dragged onto
    /// another in its group it takes that place, released where it was the press acts.
    Session(TermId, bool, ScreenHit),
    /// A tab's pill (its index now): dragged onto another tab it takes that place.
    Tab(usize),
    /// A pane pressed by its title: the pane it would swap with if let go now, and where the
    /// pointer is (its name follows it).
    Pane { from: TermId, over: Option<TermId>, at: Position },
}

/// The sidebar's width limits.
pub(super) const SIDE_MIN: u16 = 24;
pub(super) const SIDE_MAX: u16 = 60;

fn saved_path() -> PathBuf {
    // Per server, like the saved session: a test server (<APP>_SOCKET) keeps its own.
    let name = match std::env::var(concat!(env!("APP_ENV"), "_SOCKET")) {
        Ok(l) if l != "default" => format!("{APP_COMMAND}-ui-{l}.json"),
        _ => concat!(env!("APP_COMMAND"), "-ui.json").into(),
    };
    crate::config::data_dir().join(name)
}

impl Screen {
    pub fn load() -> Screen {
        if cfg!(test) {
            return Screen::default();
        }
        let saved = crate::config::read_state(&saved_path());
        Screen { saved, ..Default::default() }
    }

    pub fn save(&self) {
        *self.model_cache.borrow_mut() = None;
        if cfg!(test) {
            return;
        }
        if let Ok(s) = serde_json::to_string_pretty(&self.saved)
            && let Err(e) = crate::config::write_atomic(&saved_path(), s)
        {
            tracing::warn!("couldn't save {APP_COMMAND}'s state: {e}");
        }
    }
}

/// The overlays of this layout.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Finder {
    pub q: String,
    pub sel: usize,
    /// The folder being listed and its subfolders: (name, is a git repo).
    pub dir: PathBuf,
    pub list: Vec<(String, bool)>,
    /// Picking a folder for this setting (Settings → Start folder) instead of opening one.
    pub for_setting: Option<&'static str>,
}

#[derive(Debug, Clone, PartialEq)]
pub(super) struct NewPaneHy {
    /// Project index (== count means "other folder…"), run index, row, open beside.
    pub p: usize,
    pub a: usize,
    /// 0 task, 1 run, 2 model, 3 project, 4 where, 5 open.
    pub row: u8,
    pub beside: bool,
    /// Where, once chosen: 0 a new worktree, 1 a new branch, 2 this branch, 3.. one of the
    /// project's worktrees. None: the default for what runs (agents get a worktree, shells
    /// this branch).
    pub place: Option<u8>,
    /// What to ask the agent to do (its first prompt); empty starts it waiting.
    pub task: String,
    /// The model (0: the agent's default).
    pub model: usize,
}

pub(super) const NP_ROWS: u8 = 6;

impl NewPaneHy {
    pub fn new(p: usize, beside: bool) -> NewPaneHy {
        NewPaneHy { p, a: 0, row: 0, beside, place: None, task: String::new(), model: 0 }
    }

    pub fn place_for(&self, agent: &str, per_agent: bool) -> u8 {
        self.place.unwrap_or(if agent != "shell" && per_agent { 0 } else { 2 })
    }
}

// ---- model ---------------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub(super) struct Session {
    pub term: TermId,
    /// What the row says: your name for it, else its worktree, else repo/branch on a
    /// feature branch, else the folder it's in.
    pub name: String,
    pub title: String,
    pub agent: String,
    pub status: Status,
    pub since: u64,
    /// Working with no output or report since then (unix seconds): it may be stuck.
    pub quiet_since: Option<u64>,
    /// You marked this quiet spell seen in the Inbox.
    pub quiet_seen: bool,
    pub question: Option<String>,
    pub is_agent: bool,
    pub asleep: bool,
    /// Subagents it's running right now.
    pub subagents: Vec<String>,
    pub subagent_activity: Vec<SubagentActivity>,
    /// Stopped by a plan limit: the app says "continue" then (unix seconds).
    pub resume_at: Option<u64>,
    /// The session whose agent started this one, if one did.
    pub started_by: Option<TermId>,
    /// Helpers it wants to start and is waiting on you for (0: not asking).
    pub helpers_wanted: usize,
    /// What the secret it asks you for is called (None: not asking).
    pub secret_ask: Option<String>,
    /// Follow-ups waiting for its turn to end, next first.
    pub queued: Vec<String>,
    /// The latest check the app ran after its turn: the headline of `checks`.
    pub check: Option<CheckInfo>,
    /// Every checkpoint, in the project's order.
    pub checks: Vec<CheckInfo>,
    /// How full the agent's context is, 0–100, when the agent reports it.
    pub context: Option<f32>,
}

#[derive(Debug, Clone)]
pub(super) struct Wt {
    pub key: String,
    pub path: PathBuf,
    /// The worktree's folder name (or "main folder").
    pub name: String,
    /// The branch checked out there.
    pub branch: String,
    pub main: bool,
    pub sessions: Vec<Session>,
}

#[derive(Debug, Clone)]
pub(super) struct Proj {
    pub key: String,
    pub path: PathBuf,
    pub name: String,
    pub color: Color,
    pub wts: Vec<Wt>,
    pub fresh: bool,
    /// A git repo (worktrees are possible).
    pub git: bool,
    /// Recent local branches not checked out anywhere.
    pub branches: Vec<String>,
}

impl Proj {
    pub fn sessions(&self) -> impl Iterator<Item = &Session> {
        self.wts.iter().flat_map(|w| w.sessions.iter())
    }
}

/// Attention order: needs you, done, working, idle.
/// What an agent's row says before it has been given anything to do.
pub(super) const WAITING: &str = "waiting for you";

pub(super) fn rank(s: Status) -> u8 {
    match s {
        Status::Blocked => 0,
        Status::Done => 1,
        Status::Working => 2,
        _ => 3,
    }
}

/// The sidebar has the keys, or the why popover opened from one of its rows does (it
/// gives them back to the sidebar when it closes).
pub(super) fn side_focused(app: &App) -> bool {
    matches!(app.mode, Mode::Side | Mode::Why { side: true, .. } | Mode::Helpers { side: true, .. } | Mode::Secret { side: true, .. })
}

/// A split is one session: the pane it was split from keeps its row, the panes opened
/// beside it don't get rows of their own. The row shows the most urgent of them, so an agent
/// in a split that needs you still says so (and what it asks).
pub(super) fn fold_splits(projs: &mut [Proj], tabs: &[ScreenTab]) {
    let mut owners: Vec<TermId> = tabs.iter().map(|t| t.owner).collect();
    owners.sort_unstable();
    owners.dedup();
    for owner in owners {
        // Everything in the session's tabs, besides the session itself.
        let beside: Vec<TermId> = tabs.iter().filter(|t| t.owner == owner).flat_map(|t| t.layout.leaves()).filter(|id| *id != owner).collect();
        let has_row = projs.iter().flat_map(|p| p.sessions()).any(|s| s.term == owner);
        if beside.is_empty() || !has_row {
            continue;
        }
        let mut members: Vec<Session> = Vec::new();
        for p in projs.iter_mut() {
            for w in &mut p.wts {
                w.sessions.retain(|s| {
                    let gone = beside.contains(&s.term);
                    if gone {
                        members.push(s.clone());
                    }
                    !gone
                });
            }
        }
        let Some(row) = projs.iter_mut().flat_map(|p| p.wts.iter_mut()).flat_map(|w| w.sessions.iter_mut()).find(|s| s.term == owner) else {
            continue;
        };
        // Another tab has its own sidebar line: its agent working or done is that line's to say,
        // not the owner's. Only a question still reaches the row, since the Inbox reads rows.
        let own_tab: Vec<TermId> = tabs.iter().filter(|t| t.owner == owner && t.layout.contains(owner)).flat_map(|t| t.layout.leaves()).collect();
        let reaches_row = |m: &Session| own_tab.contains(&m.term) || m.status == Status::Blocked;
        if let Some(m) = members.into_iter().filter(|m| reaches_row(m) && rank(m.status) < rank(row.status)).min_by_key(|m| rank(m.status)) {
            row.status = m.status;
            row.since = m.since;
            row.question = m.question;
        }
    }
}

pub(super) fn folder_name(p: &Path) -> String {
    p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_else(|| p.display().to_string())
}

/// Unix seconds now, or the time a test pinned for this thread so its frames read the same
/// on every run.
pub(in crate::client) fn now_secs() -> u64 {
    #[cfg(test)]
    if let Some(now) = FIXED_NOW.with(std::cell::Cell::get) {
        return now;
    }
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0)
}

#[cfg(test)]
thread_local! {
    pub(in crate::client) static FIXED_NOW: std::cell::Cell<Option<u64>> = const { std::cell::Cell::new(None) };
}

/// "40s", "3m", "1h", "2d".
pub(super) fn age(since: u64) -> String {
    let s = now_secs().saturating_sub(since);
    match s {
        0..=59 => format!("{s}s"),
        60..=3599 => format!("{}m", s / 60),
        3600..=86_399 => format!("{}h", s / 3600),
        _ => format!("{}d", s / 86_400),
    }
}

/// How long until `at` (unix seconds): "1h20m", "5m", "now".
pub(super) fn until(at: u64) -> String {
    let s = at.saturating_sub(now_secs());
    match s {
        0..=59 => "now".into(),
        60..=3599 => format!("{}m", s / 60),
        3600..=86_399 if (s % 3600) / 60 > 0 => format!("{}h{}m", s / 3600, (s % 3600) / 60),
        3600..=86_399 => format!("{}h", s / 3600),
        _ => format!("{}d", s / 86_400),
    }
}

/// An agent's numbered choices on screen ("❯ 1. Yes", "  2. Yes, and always allow …"),
/// shortened for buttons. Falls back to Yes / Always / No.
pub(super) fn options(parser: Option<&vt100::Parser>) -> Vec<String> {
    let found = choices(parser);
    if found.is_empty() {
        return vec!["Yes".into(), "Always".into(), "No".into()];
    }
    found
}

/// The numbered choices on an agent's screen, shortened for buttons; none when it isn't
/// asking anything there.
pub(super) fn choices(parser: Option<&vt100::Parser>) -> Vec<String> {
    let mut out: Vec<(u8, String)> = Vec::new();
    if let Some(p) = parser {
        let screen = p.screen();
        let (_, cols) = screen.size();
        let lines: Vec<String> = screen.rows(0, cols).collect();
        for l in lines.iter().rev().filter(|l| !l.trim().is_empty()).take(16) {
            let t = l.trim().trim_matches(|c: char| "│┃ ".contains(c)).trim_start_matches(['❯', '>', ' ']).trim();
            let mut chars = t.chars();
            let (Some(d), Some('.')) = (chars.next(), chars.next()) else { continue };
            let Some(n) = d.to_digit(10).filter(|n| (1..=9).contains(n)) else { continue };
            let rest = chars.as_str().trim();
            if rest.is_empty() || out.iter().any(|(k, _)| *k == n as u8) {
                continue;
            }
            let label = if rest.starts_with("Yes, and") || rest.to_lowercase().contains("always") {
                "Always".to_string()
            } else {
                let cut = rest.find([',', '(']).unwrap_or(rest.len());
                truncate(rest[..cut].trim(), 18)
            };
            out.push((n as u8, label));
        }
    }
    out.sort();
    if out.first().is_none_or(|(n, _)| *n != 1) {
        return Vec::new();
    }
    out.into_iter().take(9).map(|(_, l)| l).collect()
}

impl App {
    /// Every terminal, by project (repo) and worktree.
    /// Forget the cached model (things may have changed).
    pub(super) fn screen_fresh(&self) {
        *self.screen.model_cache.borrow_mut() = None;
    }

    /// Every terminal, by project (repo) and worktree. Cached until the next event.
    pub(super) fn screen_model(&self) -> Vec<Proj> {
        if let Some(m) = self.screen.model_cache.borrow().as_ref() {
            return m.clone();
        }
        let m = self.screen_model_build();
        *self.screen.model_cache.borrow_mut() = Some(m.clone());
        m
    }

    fn screen_model_build(&self) -> Vec<Proj> {
        let mut projs: Vec<Proj> = Vec::new();
        let order = &self.screen.saved.order;
        let fallback: Vec<Color> = self.cfg.ui.workspace_colors.iter().filter_map(|c| crate::theme::parse_color(c)).collect();
        let add_proj = |projs: &mut Vec<Proj>, path: &Path, git: bool| -> usize {
            let key = path_key(path);
            if let Some(i) = projs.iter().position(|p| p.key == key) {
                projs[i].git |= git;
                return i;
            }
            let ci = order.iter().position(|k| *k == key).unwrap_or(order.len() + projs.len());
            projs.push(Proj {
                key: key.clone(),
                path: path.to_path_buf(),
                name: self.screen.saved.names.get(&key).cloned().unwrap_or_else(|| folder_name(path)),
                color: self.theme.project(ci, &fallback),
                wts: Vec::new(),
                fresh: self.screen.fresh.contains(&key),
                git,
                branches: Vec::new(),
            });
            projs.len() - 1
        };
        let add_wt = |p: &mut Proj, path: &Path, branch: String, main: bool| -> usize {
            let key = path_key(path);
            if let Some(i) = p.wts.iter().position(|w| w.key == key) {
                return i;
            }
            // The repo folder goes by the branch it's on; a worktree by its folder.
            let name = if !branch.is_empty() && (main || folder_name(path).starts_with("spare-")) { branch.clone() } else { folder_name(path) };
            p.wts.push(Wt { key, path: path.to_path_buf(), name, branch, main, sessions: Vec::new() });
            p.wts.len() - 1
        };

        for w in &self.snap.workspaces {
            let leaves: Vec<TermId> = w.tabs.iter().flat_map(|t| t.layout.leaves()).collect();
            for id in &leaves {
                let Some(t) = self.snap.terms.get(id) else { continue };
                let placed = self.screen.saved.placed.iter().find(|(x, ..)| x == id);
                let (root, top, branch, main, git) = match (&t.root, &t.top) {
                    // Dragged into another group: it stays there.
                    _ if let Some((_, p, g)) = placed => {
                        let branch = if t.root.as_ref().is_some_and(|r| path_key(r) == path_key(p)) { t.branch.clone().unwrap_or_default() } else { String::new() };
                        (p.clone(), p.clone(), branch, true, *g)
                    }
                    (Some(r), Some(tp)) => {
                        let main = path_key(r) == path_key(tp);
                        (r.clone(), tp.clone(), t.branch.clone().unwrap_or_default(), main, true)
                    }
                    _ => {
                        let cwd = if t.cwd.as_os_str().is_empty() { w.cwd.clone() } else { t.cwd.clone() };
                        (cwd.clone(), cwd, String::new(), true, false)
                    }
                };
                // A session on another machine stays with the folder it was started from, named
                // by the machine unless you named it.
                let name = match &t.remote {
                    Some(host) if t.label.trim().is_empty() => host.clone(),
                    _ => row_name(t, &root, &top, main, git),
                };
                let pi = add_proj(&mut projs, &root, git);
                let wi = add_wt(&mut projs[pi], &top, branch, main);
                let title = session_title(t, if leaves.len() == 1 { &w.name } else { "" }, &top);
                // What it asks: its ask-human question, else what its screen asks.
                let secret = self.snap.secret_requests.iter().find(|r| r.term == *id).map(|r| r.label.clone());
                let question = if let Some(label) = &secret {
                    Some(secret_line(label))
                } else if t.helper_ask.is_empty() {
                    (t.status == Status::Blocked)
                        .then(|| self.snap.questions.iter().find(|q| q.term == *id).map(|q| q.text.clone()).or_else(|| self.parsers.get(id).and_then(question)))
                        .flatten()
                } else {
                    Some(helpers_line(t.helper_ask.len()))
                };
                projs[pi].wts[wi].sessions.push(Session {
                    term: *id,
                    name,
                    title,
                    agent: match &t.agent {
                        Some(a) => a.clone(),
                        None if t.is_shell() => "shell".into(),
                        None => t.display_name(),
                    },
                    status: t.status,
                    since: t.since,
                    quiet_since: t.quiet_since,
                    quiet_seen: t.quiet_since.is_some_and(|at| self.screen.quiet_seen.get(id) == Some(&at)),
                    question,
                    is_agent: t.agent.is_some(),
                    asleep: t.asleep,
                    subagents: t.subagents.clone(),
                    subagent_activity: t.subagent_activity.iter()
                        .filter(|activity| activity.parent.as_deref().is_none_or(|parent| Some(parent) == t.session.as_deref()))
                        .filter(|activity| activity.state.active())
                        .cloned()
                        .collect(),
                    resume_at: t.resume_at,
                    started_by: t.started_by,
                    helpers_wanted: t.helper_ask.len(),
                    secret_ask: secret,
                    queued: t.queued.clone(),
                    check: t.check().cloned(),
                    checks: t.checks.clone(),
                    context: t.usage.context,
                });
            }
            // The repo's other worktrees, even with nothing running in them.
            if let Some(g) = &w.git
                && let Some(p) = projs.iter_mut().find(|p| p.key == path_key(&g.root))
            {
                for e in &g.worktrees {
                    add_wt(p, &e.path, e.branch.clone(), e.main);
                }
            }
        }
        fold_splits(&mut projs, &self.screen.tabs);
        // Groups come from where sessions are: one left with none (its only pane is inside a
        // split elsewhere) goes.
        projs.retain(|p| p.sessions().next().is_some());
        for p in &mut projs {
            p.wts.sort_by_key(|w| (!w.main, w.name.clone()));
        }
        projs.sort_by_key(|p| order.iter().position(|k| *k == p.key).unwrap_or(usize::MAX));
        projs
    }

    /// Keep the remembered bits in step with a new state: project order, the current
    /// project, the split pair, the branch lists.
    /// The tabs to show for the daemon's: a split, or a pane in another session's tab (its
    /// `tab_of`), with that session's own pane as a tab of its own first. Whose tab each is
    /// comes from the daemon, which keeps it across restarts; else from this view's tabs, else
    /// the split's first pane.
    fn restored_tabs(&self) -> Vec<ScreenTab> {
        let previous = &self.screen.tabs;
        let owner_of = |leaves: &[TermId]| {
            let kept = leaves.iter().find_map(|id| self.snap.terms.get(id).and_then(|t| t.tab_of)).filter(|o| self.snap.terms.contains_key(o));
            let prior = previous.iter().find(|tab| tab.layout.leaves().iter().any(|id| leaves.contains(id))).map(|tab| tab.owner);
            kept.or(prior).unwrap_or(leaves[0])
        };
        let sources: Vec<&crate::protocol::TabInfo> = self.snap.workspaces.iter().flat_map(|w| &w.tabs).collect();
        let owners: Vec<TermId> = sources.iter().map(|s| owner_of(&s.layout.leaves())).collect();
        let shown = |i: usize| {
            let leaves = sources[i].layout.leaves();
            // A split, a tab of another session's, or a session's own pane that has other tabs.
            leaves.len() > 1 || owners[i] != leaves[0] || owners.iter().enumerate().any(|(j, o)| j != i && *o == leaves[0])
        };
        let mut out: Vec<ScreenTab> = (0..sources.len())
            .filter(|&i| shown(i))
            .map(|i| {
                let source = sources[i];
                let leaves = source.layout.leaves();
                let prior = previous.iter().find(|tab| tab.layout.leaves().iter().any(|id| leaves.contains(id)));
                ScreenTab {
                    layout: source.layout.clone(),
                    focus: if source.layout.contains(source.focus) { source.focus } else { source.layout.first_leaf() },
                    owner: owners[i],
                    used: prior.map_or(0, |tab| tab.used),
                    name: prior.map_or_else(|| source.name.clone(), |tab| if tab.name.is_empty() { source.name.clone() } else { tab.name.clone() }),
                }
            })
            .collect();
        // Each session's own tab before its others, keeping the daemon's order otherwise.
        out.sort_by_key(|tab| !tab.layout.contains(tab.owner));
        out
    }

    pub(super) fn screen_sync(&mut self) {
        // The daemon owns pane topology and restores it across client restarts. Rebuild the
        // visible tabs from that snapshot before handling focus changes, which otherwise
        // turns the first focused restored pane into a one-leaf tab and strands its siblings.
        if self.screen.last_focus.is_none() && !self.snap.workspaces.is_empty() {
            let restored = self.restored_tabs();
            if !restored.is_empty() {
                self.screen.tabs = restored;
                self.screen.tab = self.screen.tabs.iter().position(|tab| tab.layout.contains(self.focused().unwrap_or(tab.focus))).unwrap_or(0);
            }
        }
        let focus = self.focused();
        let model = self.screen_model();
        let mut changed = false;
        for p in &model {
            if !self.screen.saved.order.contains(&p.key) {
                self.screen.saved.order.push(p.key.clone());
                changed = true;
            }
        }
        if changed {
            self.screen.save();
        }
        // You closed the last session: a shell in your home folder takes its place, so there's
        // always somewhere to type (and closing a project never closes the app).
        if focus.is_none() && self.screen.last_focus.is_some() && self.snap.terms.is_empty() {
            self.screen.last_focus = None;
            self.splash = false;
            let home = self.cfg.start_dir().or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().to_path_buf())).unwrap_or_else(std::env::temp_dir);
            self.screen_start_pane(home, None, false);
            return;
        }
        let proj_of = |t: TermId| model.iter().find(|p| p.sessions().any(|s| s.term == t)).map(|p| p.key.clone());
        if focus != self.screen.last_focus {
            self.screen.follow = true;
            if let Some(f) = focus {
                if let Some(k) = proj_of(f) {
                    self.screen.fresh.remove(&k);
                    self.screen.proj = Some(k);
                }
                let prev = self.screen.last_focus;
                self.screen_place(f, prev);
                // Which panes share a split may have changed, and the sidebar folds those.
                self.screen_fresh();
            }
            self.screen.last_focus = focus;
        }
        // Sessions that ended leave their tabs; empty tabs go.
        let alive = |t: TermId| self.snap.terms.contains_key(&t);
        let mut tabs: Vec<ScreenTab> = self
            .screen
            .tabs
            .iter()
            .cloned()
            .filter_map(|tab| {
                let layout = tab.layout.map_leaves(&mut |id| alive(id).then_some(id))?;
                let focus = if layout.contains(tab.focus) { tab.focus } else { layout.first_leaf() };
                Some(ScreenTab { layout, focus, ..tab })
            })
            .collect();
        tabs.dedup_by(|a, b| a.layout == b.layout);
        // Views get rebuilt as you move between sessions; the name you gave lives in the daemon.
        for tab in tabs.iter_mut() {
            let saved = self.snap.workspaces.iter().flat_map(|w| &w.tabs).find(|s| s.layout.contains(tab.focus)).map(|s| s.name.as_str());
            if let Some(name) = saved {
                tab.name = name.to_string();
            }
        }
        // A session that ended with other tabs left: the first of those becomes the session.
        let gone: Vec<TermId> = tabs.iter().map(|t| t.owner).filter(|o| !alive(*o)).collect();
        for o in gone {
            if let Some(next) = tabs.iter().find(|t| t.owner == o).map(|t| t.layout.first_leaf()) {
                for t in tabs.iter_mut().filter(|t| t.owner == o) {
                    t.owner = next;
                }
            }
        }
        if tabs != self.screen.tabs {
            self.screen_fresh();
        }
        self.screen.tabs = tabs;
        self.screen.tab = self.screen.tab.min(self.screen.tabs.len().saturating_sub(1));
        if self.screen.proj.as_ref().is_none_or(|k| !model.iter().any(|p| &p.key == k)) {
            self.screen.proj = focus.and_then(proj_of).or_else(|| model.first().map(|p| p.key.clone()));
        }
        self.recipe_followup();
        if !cfg!(test) {
            self.check_overlaps(&model);
            self.count_changed_lines(&model);
        }
    }

    /// Once a minute per repo with two or more checkouts in use: which files more than one
    /// of them changed (off the UI thread; the answer comes back as `Bg::Overlaps`).
    fn check_overlaps(&mut self, model: &[Proj]) {
        let mut repos: HashMap<PathBuf, Vec<Wt>> = HashMap::new();
        for p in model.iter().filter(|p| p.git) {
            repos.entry(p.path.clone()).or_default().extend(p.wts.iter().filter(|w| !w.sessions.is_empty()).cloned());
        }
        for (repo, mut wts) in repos {
            wts.sort_by(|a, b| a.path.cmp(&b.path));
            wts.dedup_by(|a, b| a.path == b.path);
            if wts.len() < 2 || self.screen.overlap_at.get(&repo).is_some_and(|t| t.elapsed() < OVERLAP_EVERY) {
                continue;
            }
            self.screen.overlap_at.insert(repo.clone(), Instant::now());
            let tx = self.bg.clone();
            std::thread::spawn(move || {
                let checkouts: Vec<(String, PathBuf)> = wts.iter().map(|w| (w.name.clone(), w.path.clone())).collect();
                let _ = tx.send(super::Bg::Overlaps(repo, crate::client::overlap::between(&checkouts)));
            });
        }
    }

    /// Every few seconds per checkout with a session in it: its uncommitted lines, for the
    /// `+N −N` on its rows (off the UI thread; the answer comes back as `Bg::LinesChanged`).
    fn count_changed_lines(&mut self, model: &[Proj]) {
        let dirs: Vec<PathBuf> = model.iter().filter(|p| p.git).flat_map(|p| p.wts.iter()).filter(|w| !w.sessions.is_empty()).map(|w| w.path.clone()).collect();
        for dir in dirs {
            if self.screen.lines_at.get(&dir).is_some_and(|t| t.elapsed() < LINES_EVERY) {
                continue;
            }
            self.screen.lines_at.insert(dir.clone(), Instant::now());
            self.spawn_bg(move || {
                let counts = crate::client::tasks::uncommitted_lines(&dir).ok();
                super::Bg::LinesChanged(dir, counts)
            });
        }
    }
}

/// How often a checkout's uncommitted lines are counted again.
const LINES_EVERY: std::time::Duration = std::time::Duration::from_secs(5);

/// How often a repo's checkouts are compared for files changed in more than one.
const OVERLAP_EVERY: std::time::Duration = std::time::Duration::from_secs(60);

/// What to call a session: its name if renamed, else the agent's last prompt, else where a
/// shell is (inside its worktree), else the program.
/// A session's row name: what you renamed it to; else the worktree it's in; else the
/// folder under the project it's in; else (the project's own folder) its agent or "shell".
fn row_name(t: &TermInfo, root: &Path, top: &Path, main: bool, git: bool) -> String {
    if !t.label.trim().is_empty() {
        return t.label.trim().to_string();
    }
    if git && !main {
        return folder_name(top);
    }
    let cwd = if t.cwd.as_os_str().is_empty() { top } else { t.cwd.as_path() };
    // An agent by its name; a program (vim, a harness the app doesn't know) by its own.
    let kind = t.agent.clone().unwrap_or_else(|| if t.is_shell() || t.process.is_empty() { "shell".into() } else { t.process.clone() });
    match cwd.strip_prefix(root) {
        Ok(rel) if !rel.as_os_str().is_empty() => rel.display().to_string().replace('\\', "/"),
        // The project's own folder: the agent's (or shell's) name.
        Ok(_) => kind,
        Err(_) => folder_name(cwd),
    }
}


fn session_title(t: &TermInfo, name: &str, top: &Path) -> String {
    if !t.label.trim().is_empty() {
        return t.label.trim().to_string();
    }
    if !name.is_empty() {
        return name.to_string();
    }
    if t.agent.is_some() {
        // Its name (from /rename, else the first thing it was asked) stays put; the latest
        // prompt shows under it.
        let s = if t.name.trim().is_empty() { t.summary.trim() } else { t.name.trim() };
        return if s.is_empty() { WAITING.into() } else { s.to_string() };
    }
    if t.is_shell() {
        return match t.cwd.strip_prefix(top) {
            Ok(rel) if !rel.as_os_str().is_empty() => rel.display().to_string(),
            _ => folder_name(&t.cwd),
        };
    }
    t.display_name()
}

// ---- hits ----------------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) enum ScreenHit {
    /// The toast's ✕.
    ToastClose,
    /// Fold / unfold a project.
    ToggleProj(usize),
    /// The sidebar's "+ new": a plain shell in the selected project.
    Session(TermId),
    /// A button on the selected Inbox item: the key it stands for (`\n` Enter, `\x1b` Esc).
    InboxKey(char),
    CloseSplit(TermId),
    Divider(usize),
    /// The ⋯ on a hovered sidebar row: its menu.
    RowMenuSess(TermId),
    /// Start a shell in this project (its "nothing running" line).
    ShellIn(usize),
    RowMenuProj(usize),
    ConfirmYes,
    ConfirmNo,
    TabPick(usize),
    CloseTab(usize),
    NewTab,
    /// The tab row's `◫`: split the focused pane.
    TabSplit,
    /// The tab row's `⤢`: the focused pane fills the tab, or goes back.
    TabZoom,
    /// Outside an overlay: closes it.
    Close,
    /// Inside an overlay's panel: nothing.
    Noop,
    FinderPick(usize),
    NpProj(usize),
    NpRun(usize),
    NpBeside(usize),
    NpWhere(usize),
    NpGo,
    NpTask,
    NpModel(usize),
    SetTab(usize),
    SettingsBody,
    SetRow(usize),
    /// Settings row, value index.
    SetVal(usize, usize),
    ThemePick(usize),
    SpinnerPick(usize),
    /// A button on the splash screen.
    SplashKey(char),
    /// A splash card: a click chooses it, a click on the chosen one opens it.
    SplashCard(usize),
    /// A right-click menu item.
    MenuPick(usize),
    /// The ✕ of a view in the sheet (Changes): it closes.
    ViewClose,
    /// A tab of the side panel's strip (Files, Changes, Branch): it switches to that view.
    DrawerTab(usize),
    /// The sidebar's edge (drag to resize).
    SideEdge,
    /// The side sheet's left edge: drag it to resize the sheet.
    SheetEdge,
    /// The sidebar's empty space: the keys go to it.
    SideFocus,
    ExpandSidebar,
    /// Install the newer the app (in Settings › General).
    Update,
    /// Restart the daemon onto the installed version (after an update).
    Restart,
    NewShell,
    Why(TermId),
    /// The `+N −N` on a session's row: its checkout's Changes.
    RowChanges(TermId),
    /// The why popover of a session with a check: its evidence sheet.
    CheckEvidence(TermId),
    /// A row of the worktree clean-up list (tick or untick it), and its Remove button.
    TidyRow(usize),
    TidyGo,
    /// The sidebar's "menu".
    Actions,
    /// A row of the actions list.
    ActionRow(usize),
    /// The mode bar's SHORTCUT tag: the key map.
    Leader,
    /// The mode bar's TYPING: shortcut mode, as the leader starts it.
    ShortcutMode,
    /// A suggestion on the mode bar: its key in `Screen::bar_keys`, pressed in shortcut mode.
    BarKey(usize),
    /// A key in the key map (its row in the shown list).
    KeyRow(usize),
    /// A pane's scrollbar (click or drag).
    ScrollBar(TermId),
    /// "Back to bottom" on a pane scrolled up.
    ToBottom(TermId),
    FindTab(u8),
    FindRow(usize),
    BranchRow(usize),
    GoPick(usize),
    HistRow(usize),
    BranchChoice(usize),
    /// A button of the repo-command approval.
    Approve(check_approve::ApproveHit),
    /// A row's "wants to start N helpers" line: opens the question.
    HelperAsk(TermId),
    /// A choice in that question.
    HelperAnswer(TermId, crate::protocol::HelperChoice),
    /// The secret prompt's buttons.
    SecretGive(TermId),
    SecretDecline(TermId),
    /// A button or row of Settings › Check.
    CheckDraft(check_settings::DraftHit),
}

pub(super) fn hit(app: &mut App, r: Rect, h: ScreenHit) {
    app.hits.push((r, Hit::Screen(h)));
}

pub(super) fn hovered(app: &App, r: Rect) -> bool {
    app.hover.is_some_and(|p| r.contains(p))
}

/// A design button (" Label key "); records its hit.
#[allow(clippy::too_many_arguments)]
pub(super) fn btn(app: &mut App, buf: &mut Buffer, x: u16, y: u16, label: &str, key: &str, kind: BtnKind, h: ScreenHit, max_x: u16) -> u16 {
    let t = app.theme.clone();
    let segs = button(&t, label, key, kind);
    let r = Rect { x, y, width: segs_width(&segs).min(max_x.saturating_sub(x)), height: 1 };
    let nx = put(buf, x, y, &segs, max_x);
    hit(app, r, h);
    nx
}

/// The key bound to an action after the leader, as the design shows it ("j", "Space").
pub(super) fn k(app: &App, a: &Action) -> String {
    let s = super::design::key_of(app, a);
    match s.as_str() {
        "Space" | " " => "Space".into(),
        _ => s,
    }
}

mod rail;
mod behaviour;
mod branch_sheet;
mod references_sheet;
mod card;
mod check;
mod check_actions;
mod check_approve;
mod check_inbox;
#[cfg(test)]
mod check_modals_tests;
pub(in crate::client) mod check_settings;
mod check_sheet;
mod dialogs;
mod failure_note;
pub(in crate::client) mod glyphs;
mod keyoverlay;
mod newsession;
mod leader;
mod popups;
mod heads;
mod popover;
mod helpers_ask;
mod secret_ask;
mod draw;
mod hairline;
#[cfg(test)]
pub(in crate::client) use self::rail::RAIL_W;
pub(in crate::client) use self::hairline::{flat_theme, plain, plain_on, set_plain};
mod mode_bar;
mod side;
mod side_tabs;
mod sheet;
mod inbox;
mod splash;
mod tidy;
#[cfg(test)]
mod frame_tests;
#[cfg(test)]
mod panes_apart_tests;
#[cfg(test)]
mod reference;
pub(in crate::client) use self::{card::*, dialogs::*, leader::*, popups::*, draw::*, mode_bar::*, side::*, splash::*, sheet::*, popover::*, helpers_ask::*, secret_ask::*, heads::*, tidy::*, check_sheet::*, branch_sheet::*, references_sheet::*, check_actions::*, check_approve::*, keyoverlay::*};
