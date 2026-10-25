//! Right-click menus for the app's layout: a pane, an agent row, a project, a branch or a
//! worktree. Short, hoverable, and every item does something.

use super::design::{BtnKind, button, put, seg, segs_width};
use super::screen::{ScreenHit, find, hit};
use super::render::truncate;
use super::{App, Mode};
use crate::protocol::{Command, TermId};
use crate::theme::Theme;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use std::path::PathBuf;
use unicode_width::UnicodeWidthStr;

/// How many snapshots the list offers (each has its number as its key).
const SNAPSHOTS_LISTED: usize = 9;
/// Why a snapshot is not restored while its session's agent works.
pub(super) const RESTORE_WHILE_WORKING: &str = "Its agent is working; stop it or wait for the turn to end.";

#[derive(Debug, Clone, PartialEq)]
pub enum Act {
    Beside(TermId),
    /// The program in this pane is an agent: the app learns it.
    TeachAgent(TermId),
    /// End these (stops what runs in them).
    End(Vec<TermId>),
    Changes(PathBuf),
    RenamePane(TermId),
    /// Drop the follow-ups waiting for this agent's turn to end.
    ClearQueue(TermId),
    /// List a checkout's snapshots (its folder, and whose row asked).
    Snapshots(PathBuf, TermId),
    /// Put a checkout's files back as a snapshot has them: (folder, commit, when, whose row).
    Restore(PathBuf, String, String, TermId),
    Split(TermId, crate::layout::Dir),
    /// Trade the places of two panes of the tab on screen.
    Swap(TermId, TermId),
    RightClicks(TermId),
    RenameProject(String, String),
    /// End everything in a project and forget it.
    CloseProject(usize),
    NewWorktree(usize),
    /// git init and a first commit, so agents there can get worktrees.
    GitInit(PathBuf),
    OpenWorktree(usize),
    /// The clean-up list for a project's worktrees.
    TidyOpen(usize),
    /// Remove these worktrees (each: its folder, and whether to lose uncommitted changes).
    Tidy(Vec<(PathBuf, bool)>),
    /// Install the newest the app without interrupting the running sessions.
    Update,
    /// Stop the daemon so the window reconnects to the installed version.
    RestartDaemon,
    InstallStatus(String),
    /// Open a session's check output in the evidence sheet.
    CheckOutput(TermId),
    RerunCheck(TermId),
    /// Accept a failed checkpoint (None: the headline); the string is the reason typed in the
    /// confirm, possibly empty.
    OverrideCheck(TermId, String, Option<String>),
    /// Take an override back: the check reads as failed again.
    UndoOverride(TermId),
    /// Delete a reference: (the project root, the file, its name in the references).
    RemoveReference(PathBuf, PathBuf, String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Confirm {
    /// The title bar ("Close pane", "changes").
    pub title: String,
    /// Dim next to the title ("dsa-tool").
    pub sub: String,
    /// What it's about (bold) and a dim detail after it.
    pub what: String,
    pub detail: String,
    /// A plain line of explanation.
    pub note: String,
    /// A short list under the note ("What's new"), or none.
    pub list: Vec<String>,
    /// The button that does it, and its key ("Close", "Enter").
    pub yes: String,
    pub key: char,
    /// Red (destructive) or the accent.
    pub danger: bool,
    pub act: Act,
}

/// Items a confirm's list shows before "…and N more".
const CONFIRM_LIST_MAX: usize = 8;

/// A modal like everything else: what it's about, a line of explanation, the action
/// (red when it destroys something) and Cancel.
pub(super) fn draw_confirm(app: &mut App, f: &mut ratatui::Frame, area: ratatui::layout::Rect, t: &crate::theme::Theme, c: &Confirm) {
    if matches!(c.act, Act::OverrideCheck(..)) {
        return super::screen::draw_override(app, f, area, t, c);
    }
    use super::design::{put, seg};
    use super::screen::{ScreenHit, dim_all, hit, panel};
    use ratatui::layout::Rect;
    use ratatui::style::{Modifier, Style};
    let buf = f.buffer_mut();
    let lit = closing_lines(app, buf, c, t);
    dim_all(app, buf, area, t);
    for (x, y, cell) in lit {
        buf[(x, y)] = cell;
    }
    let widest_item = c.list.iter().map(|l| l.chars().count() + 10).max().unwrap_or(0);
    let w = (c.what.chars().count() + c.detail.chars().count() + 12).max(c.note.chars().count() + 8).max(widest_item).clamp(58, 90) as u16;
    // The list (its heading, at most CONFIRM_LIST_MAX items and a "more" line), then a gap.
    let shown = c.list.len().min(CONFIRM_LIST_MAX);
    let more = c.list.len() - shown;
    let list_h = if c.list.is_empty() { 0 } else { 2 + shown as u16 + u16::from(more > 0) };
    // The sub reads as part of the title, so it is cut first when the header runs out of room.
    let title = if c.sub.is_empty() { c.title.clone() } else { format!("{} · {}", c.title, c.sub) };
    let r = panel(app, buf, area, w, 9 + list_h, &title, &[], t);
    let card = Style::default().bg(t.card);
    put(buf, r.x + 3, r.y + 2, &[seg(c.what.clone(), card.fg(t.strong).add_modifier(Modifier::BOLD)), seg(format!("   {}", c.detail), card.fg(t.muted))], r.right() - 2);
    put(buf, r.x + 3, r.y + 3, &[seg(c.note.clone(), card.fg(t.text))], r.right() - 2);
    if !c.list.is_empty() {
        put(buf, r.x + 3, r.y + 5, &[seg("What's new".to_string(), card.fg(t.muted).add_modifier(Modifier::BOLD))], r.right() - 2);
        for (i, item) in c.list.iter().take(shown).enumerate() {
            put(buf, r.x + 3, r.y + 6 + i as u16, &[seg("• ".to_string(), card.fg(t.accent)), seg(item.clone(), card.fg(t.text))], r.right() - 2);
        }
        if more > 0 {
            put(buf, r.x + 5, r.y + 6 + shown as u16, &[seg(format!("…and {more} more"), card.fg(t.muted))], r.right() - 2);
        }
    }
    let buttons_y = r.y + 6 + list_h;
    let key = if c.key == '\n' { "Enter".to_string() } else { c.key.to_string() };
    let yes = button(t, &c.yes, &key, if c.danger { BtnKind::Danger } else { BtnKind::Primary });
    let yw = segs_width(&yes);
    let yr = Rect { x: r.x + 3, y: buttons_y, width: yw, height: 1 };
    put(buf, yr.x, yr.y, &yes, r.right());
    let nr = Rect { x: yr.right() + 2, y: buttons_y, width: 12, height: 1 };
    put(buf, nr.x, nr.y, &button(t, "Cancel", "Esc", BtnKind::Normal), r.right());
    hit(app, yr, ScreenHit::ConfirmYes);
    hit(app, nr, ScreenHit::ConfirmNo);
}

/// The lit lines of the panes a close confirm asks about, kept as drawn so the screen's dimming
/// behind the confirm leaves them bright.
fn closing_lines(app: &App, buf: &ratatui::buffer::Buffer, c: &Confirm, t: &Theme) -> Vec<(u16, u16, ratatui::buffer::Cell)> {
    let Act::End(ts) = &c.act else { return Vec::new() };
    app.pane_frames
        .iter()
        .filter(|(term, _)| ts.contains(term))
        .flat_map(|(_, r)| r.intersection(buf.area).positions())
        .filter(|p| buf[(p.x, p.y)].fg == t.err)
        .map(|p| (p.x, p.y, buf[(p.x, p.y)].clone()))
        .collect()
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScreenMenu {
    pub title: String,
    pub items: Vec<(String, Act)>,
    pub sel: usize,
    pub at: (u16, u16),
}

impl App {
    fn agent_working(&self, term: TermId) -> bool {
        self.snap.terms.get(&term).is_some_and(|x| x.status == crate::protocol::Status::Working)
    }

    pub(super) fn menu(&mut self, title: String, mut items: Vec<(String, Act)>, at: (u16, u16)) {
        // The destructive item last.
        items.sort_by_key(|(_, a)| matches!(a, Act::End(_) | Act::CloseProject(_)));
        self.mode = Mode::ScreenMenu(Box::new(ScreenMenu { title, items, sel: 0, at }));
    }

    /// Right-click on a pane's content.
    pub(super) fn menu_for_pane(&mut self, term: TermId, at: (u16, u16)) {
        if Some(term) != self.focused() {
            self.cmd(Command::FocusPane { term });
        }
        let model = self.screen_model();
        let agent = find(&model, term).map(|(_, _, s)| s.agent.clone()).unwrap_or_default();
        let mut items = vec![
            ("Rename pane".to_string(), Act::RenamePane(term)),
            ("Split side by side".to_string(), Act::Split(term, crate::layout::Dir::Right)),
            ("Split stacked".to_string(), Act::Split(term, crate::layout::Dir::Down)),
        ];
        use crate::layout::Dir;
        for (dir, way) in [(Dir::Left, "left"), (Dir::Right, "right"), (Dir::Up, "up"), (Dir::Down, "down")] {
            if let Some(to) = crate::layout::neighbor(&self.screen.leaf_rects, term, dir) {
                items.push((format!("Move pane {way}"), Act::Swap(term, to)));
            }
        }
        items.push((
            if self.screen.right_clicks.contains(&term) { "Stop sending right-clicks to pane" } else { "Send right-clicks to pane" }.to_string(),
            Act::RightClicks(term),
        ));
        // A program the app doesn't know (an agent started by an alias, say): teach it.
        if let Some(t) = self.snap.terms.get(&term).filter(|t| t.agent.is_none() && !t.is_shell() && !t.process.is_empty() && t.remote.is_none()) {
            items.push((format!("{} is an agent…", t.process), Act::TeachAgent(term)));
        }
        items.push(("Close pane".to_string(), Act::End(vec![term])));
        self.menu(if agent.is_empty() { "pane".into() } else { agent }, items, at);
    }

    /// Right-click on an agent (or shell) row.
    pub(super) fn menu_for_session(&mut self, term: TermId, at: (u16, u16)) {
        if let Some((title, items)) = self.session_items(term) {
            self.menu(title, items, at);
        }
    }

    /// A session's menu: its title and items, for right-click and for the sidebar's
    /// letter keys.
    pub(super) fn session_items(&self, term: TermId) -> Option<(String, Vec<(String, Act)>)> {
        let model = self.screen_model();
        let (_, _, s) = find(&model, term)?;
        let agent = s.agent.clone();
        let dir = find(&model, term).map(|(_, w, _)| w.path.clone());
        let info = self.snap.terms.get(&term);
        // A program the app doesn't know can be taught.
        let mut items = Vec::new();
        if let Some(t) = info.filter(|t| t.agent.is_none() && !t.is_shell() && !t.process.is_empty() && t.remote.is_none()) {
            items.push((format!("{} is an agent…", t.process), Act::TeachAgent(term)));
        }
        if self.focused().is_some_and(|f| f != term) {
            items.push(("Open beside".into(), Act::Beside(term)));
        }
        items.push(("Rename".to_string(), Act::RenamePane(term)));
        if let Some(d) = dir.filter(|d| crate::gitfs::head(d).is_some()) {
            items.push(("Changes".to_string(), Act::Changes(d.clone())));
            if s.is_agent && self.cfg.snapshots.enabled {
                items.push(("Snapshots…".to_string(), Act::Snapshots(d, term)));
            }
        }
        if !s.queued.is_empty() {
            items.push((format!("Drop queued follow-ups ({})", s.queued.len()), Act::ClearQueue(term)));
        }
        if let Some(check) = info.and_then(|t| t.check()) {
            items.extend(super::screen::check_menu_items(term, check));
        }
        items.push(("Close".to_string(), Act::End(vec![term])));
        // Titled by what it is (an agent, or the program's name), never its window title.
        let what = if s.is_agent { agent } else { info.map(|t| t.process.clone()).filter(|p| !p.is_empty()).unwrap_or_else(|| "shell".into()) };
        let title = if what == s.name { what } else { format!("{} · {}", s.name, truncate(&what, 30)) };
        Some((title, items))
    }

    /// A checkout's snapshots as a numbered list, newest first; picking one asks, then
    /// restores it.
    pub(super) fn snapshots_menu(&mut self, dir: PathBuf, term: TermId, kept: Vec<crate::snapshot::Snapshot>, at: (u16, u16)) {
        if kept.is_empty() {
            return self.notify("no snapshots here yet: one is kept each time an agent's turn begins".into(), false);
        }
        let items = kept
            .iter()
            .take(SNAPSHOTS_LISTED)
            .enumerate()
            .map(|(i, s)| {
                let when = format!("{} ago · {}", super::screen::age(s.at), s.why);
                (format!("{}  {when}", i + 1), Act::Restore(dir.clone(), s.commit.clone(), when, term))
            })
            .collect();
        self.menu(format!("snapshots · {}", super::screen::folder_name(&dir)), items, at);
    }

    /// Right-click on a project row.
    pub(super) fn menu_for_project(&mut self, pi: usize, at: (u16, u16)) {
        if let Some((title, items)) = self.project_items(pi) {
            self.menu(title, items, at);
        }
    }

    pub(super) fn project_items(&self, pi: usize) -> Option<(String, Vec<(String, Act)>)> {
        let model = self.screen_model();
        let p = model.get(pi)?;
        let mut items = vec![("Rename".to_string(), Act::RenameProject(p.key.clone(), p.name.clone())), ("Close".to_string(), Act::CloseProject(pi))];
        if p.git {
            items.push(("New worktree".to_string(), Act::NewWorktree(pi)));
            items.push(("Open worktree…".to_string(), Act::OpenWorktree(pi)));
            items.push(("Clean up worktrees…".to_string(), Act::TidyOpen(pi)));
        } else {
            items.push(("Make it a git repo…".to_string(), Act::GitInit(p.path.clone())));
        }
        Some((p.name.clone(), items))
    }

    pub(super) fn on_hy_menu_key(&mut self, mut m: ScreenMenu, k: &KeyEvent) {
        match k.code {
            KeyCode::Esc => self.mode = Mode::Normal,
            KeyCode::Down | KeyCode::Char('j') => {
                m.sel = (m.sel + 1) % m.items.len().max(1);
                self.mode = Mode::ScreenMenu(Box::new(m));
            }
            KeyCode::Up | KeyCode::Char('k') => {
                m.sel = (m.sel + m.items.len().max(1) - 1) % m.items.len().max(1);
                self.mode = Mode::ScreenMenu(Box::new(m));
            }
            KeyCode::Enter => {
                self.mode = Mode::Normal;
                if let Some((_, a)) = m.items.get(m.sel).cloned() {
                    self.menu_act(a);
                }
            }
            // Delete: its close item, like x.
            KeyCode::Delete if m.items.iter().any(|(_, a)| matches!(a, Act::End(_) | Act::CloseProject(_))) => {
                self.mode = Mode::Normal;
                if let Some((_, a)) = m.items.iter().find(|(_, a)| matches!(a, Act::End(_) | Act::CloseProject(_))).cloned() {
                    self.menu_act(a);
                }
            }
            // An item's key picks it: a bound key as the keymap has it, a letter as typed (O is
            // not o), else whatever the case.
            _ if pressed(&menu_keys(self, &m.items), k).is_some() => {
                let i = pressed(&menu_keys(self, &m.items), k).unwrap_or(0);
                self.mode = Mode::Normal;
                if let Some((_, a)) = m.items.get(i).cloned() {
                    self.menu_act(a);
                }
            }
            // A numbered list (presets): its number picks it.
            KeyCode::Char(c @ '1'..='9') if m.items.iter().any(|(l, _)| l.starts_with("1 ")) => {
                let i = c as usize - '1' as usize;
                if let Some((_, a)) = m.items.get(i).cloned() {
                    self.mode = Mode::Normal;
                    self.menu_act(a);
                } else {
                    self.mode = Mode::ScreenMenu(Box::new(m));
                }
            }
            _ => self.mode = Mode::ScreenMenu(Box::new(m)),
        }
    }

    pub(super) fn menu_pick(&mut self, i: usize) {
        let Mode::ScreenMenu(m) = std::mem::replace(&mut self.mode, Mode::Normal) else { return };
        if let Some((_, a)) = m.items.get(i).cloned() {
            self.menu_act(a);
        }
    }

    /// What a menu item does; closing things asks first.
    pub(super) fn menu_act(&mut self, a: Act) {
        let model = self.screen_model();
        let ask: Option<Confirm> = match &a {
            Act::End(ts) if ts.len() == 1 => {
                let name = super::screen::session_name(self, &model, ts[0]);
                let path = self.snap.terms.get(&ts[0]).map(|t| t.cwd.display().to_string()).unwrap_or_default();
                let running = self.snap.terms.get(&ts[0]).is_some_and(|t| t.status == crate::protocol::Status::Working);
                Some(Confirm {
                    title: super::screen::pane_number(self, ts[0]).map_or_else(|| "Close pane".into(), |n| format!("Close pane {n}")),
                    sub: name.clone(),
                    what: name,
                    detail: path,
                    note: if running { "It's still working; the program running in it will stop.".into() } else { "The program running in it will stop.".into() },
                    list: Vec::new(),
                    yes: "Close".into(),
                    key: '\n',
                    danger: true,
                    act: a.clone(),
                })
            }
            Act::End(ts) => Some(Confirm {
                title: "Close panes".into(),
                sub: String::new(),
                what: format!("{} panes", ts.len()),
                detail: String::new(),
                note: "The programs running in them will stop.".into(),
                list: Vec::new(),
                yes: "Close".into(),
                key: '\n',
                danger: true,
                act: a.clone(),
            }),
            // Refused while it works (menu_do says so): nothing to ask.
            Act::Restore(_, _, _, term) if self.agent_working(*term) => None,
            Act::Restore(dir, _, when, _) => Some(Confirm {
                title: "Restore snapshot".into(),
                sub: String::new(),
                what: super::screen::folder_name(dir),
                detail: when.clone(),
                note: "The files go back to how they were then. What's there now is kept as a snapshot first, so this can be taken back.".into(),
                list: Vec::new(),
                yes: "Restore".into(),
                key: '\n',
                danger: false,
                act: a.clone(),
            }),
            Act::RemoveReference(_, _, name) => Some(Confirm {
                title: "Remove reference".into(),
                sub: String::new(),
                what: name.clone(),
                detail: String::new(),
                note: "The file is deleted from the project's references, and agents no longer see it.".into(),
                list: Vec::new(),
                yes: "Remove".into(),
                key: '\n',
                danger: true,
                act: a.clone(),
            }),
            Act::GitInit(p) => Some(Confirm {
                title: "Make it a git repo".into(),
                sub: String::new(),
                what: p.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(),
                detail: p.display().to_string(),
                note: "git init, then what's there as the first commit. Agents then get worktrees.".into(),
                list: Vec::new(),
                yes: "Make it a git repo".into(),
                key: 'g',
                danger: false,
                act: a.clone(),
            }),
            Act::OverrideCheck(t, _, name) => match self.override_confirm(*t, name.clone()) {
                Some(c) => Some(c),
                None => return,
            },
            Act::CloseProject(pi) => model.get(*pi).map(|p| {
                let n = p.sessions().count();
                Confirm {
                    title: "Close project".into(),
                    sub: String::new(),
                    what: p.name.clone(),
                    detail: format!("{n} pane{}", if n == 1 { "" } else { "s" }),
                    note: "Everything running in it stops; the folder stays where it is.".into(),
                    list: Vec::new(),
                    yes: "Close".into(),
                    key: '\n',
                    danger: true,
                    act: a.clone(),
                }
            }),
            _ => None,
        };
        match ask {
            Some(c) => {
                // Asked from the sidebar: come back to it afterwards, on the next row (the
                // one asked about is going), or the same row when it stays.
                // The cursor is still on a row when the leader key or the actions list was
                // opened from the sidebar: that's the sidebar too.
                let from_side = self.mode == Mode::Side || self.screen.cursor.is_some() || self.screen.cursor_proj.is_some();
                self.screen.side_return = match &c.act {
                    _ if !from_side => None,
                    Act::Restore(_, _, _, t) | Act::OverrideCheck(t, ..) => Some(super::screen::SideItem::Sess(*t)),
                    _ => self.side_after_close(),
                };
                self.mode = Mode::Confirm(Box::new(c));
            }
            None => self.menu_do(a),
        }
    }

    pub(super) fn menu_do(&mut self, a: Act) {
        match a {
            Act::RenamePane(t) => {
                let cur = self.snap.terms.get(&t).map(|x| x.label.clone()).unwrap_or_default();
                self.mode = Mode::Prompt { kind: super::PromptKind::RenamePane(t), input: cur };
            }
            Act::Split(t, dir) => {
                let cwd = self.snap.terms.get(&t).map(|x| x.top.clone().unwrap_or_else(|| x.cwd.clone())).unwrap_or_else(|| self.here_dir());
                self.screen.split_dir = Some(dir);
                if self.focused() != Some(t) {
                    self.cmd(Command::FocusPane { term: t });
                }
                self.screen.pending_split = Some((t, std::time::Instant::now()));
                self.leave_shortcut_mode();
                self.cmd(Command::NewWorkspace { cwd: Some(cwd), name: None, cmd: None });
            }
            Act::Swap(a, b) => {
                self.swap_panes(a, b);
            }
            Act::Update => self.start_update(),
            Act::RestartDaemon => self.restart_daemon_now(),
            Act::InstallStatus(agent) => {
                self.notify(format!("setting up {agent} status…"), false);
                self.spawn_bg(move || {
                    let result = crate::cli::integrations::install_status(&agent);
                    super::Bg::Then(Box::new(move |app: &mut App| match result {
                        Ok(_) => {
                            // Settings reads Claude's hook state once; a connect from here must not leave it saying "not installed".
                            if agent == "claude" {
                                app.claude_hooks = Some(true);
                            }
                            let name = crate::cli::integrations::STATUS_AGENTS.iter().find(|a| a.id == agent).map_or(agent.as_str(), |a| a.name);
                            app.notify(format!("{name} status installed; restart {name} to load it (Pi also supports /reload)"), false)
                        }
                        Err(e) => app.notify(format!("couldn't set up {agent} status: {e:#}"), true),
                    }))
                });
            }
            Act::GitInit(dir) => {
                self.notify("making it a git repo…".into(), false);
                self.spawn_bg(move || {
                    let result = git_init(&dir);
                    super::Bg::Then(Box::new(move |app: &mut App| {
                        match result {
                            Ok(()) => app.notify("it's a git repo now; new agents get their own worktree".into(), false),
                            Err(e) => app.notify(e, true),
                        }
                        app.screen_fresh();
                    }))
                });
            }
            Act::TeachAgent(term) => self.cmd(Command::TeachAgent { term }),
            Act::TidyOpen(pi) => self.tidy_open(pi),
            Act::ClearQueue(term) => self.cmd(Command::ClearQueue { term }),
            Act::Snapshots(dir, term) => {
                let at = self.screen.row_y.get(&term).map(|y| (self.screen.saved.side_w.unwrap_or(0), *y)).unwrap_or((0, 0));
                self.spawn_bg(move || {
                    let kept = crate::snapshot::list(&dir);
                    super::Bg::Then(Box::new(move |app: &mut App| app.snapshots_menu(dir, term, kept, at)))
                });
            }
            // Its agent would keep writing into files being rolled back, even after you confirmed.
            Act::Restore(_, _, _, term) if self.agent_working(term) => self.notify(RESTORE_WHILE_WORKING.into(), true),
            Act::Restore(dir, commit, _, _) => {
                self.notify("restoring…".into(), false);
                self.spawn_bg(move || {
                    let result = crate::snapshot::restore(&dir, &commit);
                    super::Bg::Then(Box::new(move |app: &mut App| {
                        match result {
                            Ok(0) => app.notify("the files are already as they were then".into(), false),
                            Ok(n) => app.notify(format!("Restored {n} file{} · the newest snapshot takes it back", if n == 1 { "" } else { "s" }), false),
                            Err(e) => app.notify(format!("couldn't restore: {e}"), true),
                        }
                        app.branch_sheet_reload();
                        app.screen_fresh();
                    }))
                });
            }
            Act::RemoveReference(root, path, name) => self.remove_reference(root, path, name),
            Act::Tidy(items) => {
                self.notify("Cleaning up worktrees…".into(), false);
                self.cmd(Command::TidyWorktrees { items });
            }
            Act::RightClicks(t) => {
                if !self.screen.right_clicks.remove(&t) {
                    self.screen.right_clicks.insert(t);
                    self.notify(concat!("right-clicks go to this pane now (Shift+right-click for ", env!("APP_COMMAND"), "'s menu)").into(), false);
                }
            }
            Act::RenameProject(key, name) => self.mode = Mode::Prompt { kind: super::PromptKind::RenameProject(key), input: name },
            Act::CloseProject(pi) => {
                let model = self.screen_model();
                if let Some(p) = model.get(pi) {
                    for s in p.sessions() {
                        self.cmd(Command::ClosePane { term: s.term });
                    }
                }
            }
            Act::NewWorktree(pi) => {
                self.screen_new(pi, false);
                if let Mode::ScreenPane(np) = &mut self.mode {
                    np.place = Some(0);
                }
            }
            Act::OpenWorktree(pi) => {
                let model = self.screen_model();
                let Some(p) = model.get(pi) else { return };
                let key = super::design::path_key(&p.path);
                let ws = self.snap.workspaces.iter().find(|w| {
                    w.tabs.iter().flat_map(|t| t.layout.leaves()).any(|id| self.snap.terms.get(&id).and_then(|t| t.root.as_ref()).is_some_and(|r| super::design::path_key(r) == key))
                });
                match ws.map(|w| w.id) {
                    Some(ws) => {
                        self.send(crate::protocol::ClientMsg::Query(crate::protocol::Query::Worktrees { ws }));
                        self.mode = Mode::Worktrees { ws, cmd: None, items: None, query: String::new(), sel: 0 };
                    }
                    None => self.notify("start something in this project first".into(), true),
                }
            }
            Act::Beside(t) => {
                if let Some(f) = self.focused() {
                    self.screen_unshow(t);
                    self.screen.pending_split = Some((f, std::time::Instant::now()));
                    self.cmd(Command::FocusPane { term: t });
                }
            }
            Act::End(ts) => {
                for t in ts {
                    // Out of its split first: the pane beside it takes the room (and the
                    // focus), so nothing else slides into its place.
                    self.screen_unshow(t);
                    self.cmd(Command::ClosePane { term: t });
                }
            }
            Act::Changes(p) => self.open_changes(p),
            Act::CheckOutput(t) => self.open_check_sheet(t),
            Act::RerunCheck(t) => self.rerun_check(t),
            Act::OverrideCheck(t, reason, name) => self.override_check(t, name, reason),
            Act::UndoOverride(t) => self.undo_override(t, None),
        }
    }
}

/// What a new repo ignores when it has no .gitignore of its own, so its first commit
/// doesn't take in dependencies, build output or secrets.
const DEFAULT_GITIGNORE: &str = "node_modules/\ntarget/\ndist/\nbuild/\n.venv/\n__pycache__/\n.env\n.env.*\n*.log\n.DS_Store\n";

/// `git init`, a .gitignore when there isn't one, and a first commit (worktrees need one).
fn git_init(dir: &std::path::Path) -> Result<(), String> {
    let run = |args: &[&str]| {
        let mut c = std::process::Command::new("git");
        c.arg("-C").arg(dir).args(args);
        crate::proc::quiet(&mut c);
        match c.output() {
            Ok(o) if o.status.success() => Ok(()),
            Ok(o) => Err(String::from_utf8_lossy(&o.stderr).lines().next().unwrap_or("git failed").to_string()),
            Err(e) => Err(format!("couldn't run git: {e}")),
        }
    };
    run(&["init", "-q"])?;
    let ignore = dir.join(".gitignore");
    if !ignore.exists() {
        std::fs::write(&ignore, DEFAULT_GITIGNORE).map_err(|e| format!("couldn't write .gitignore: {e}"))?;
    }
    run(&["add", "-A"])?;
    run(&["commit", "-q", "-m", "Initial commit"]).map_err(|e| format!("the first commit failed ({e}); is git set up with your name and email?"))
}

/// A check's actions keep the keys they have on the row, in the evidence sheet and the Inbox.
fn check_key(a: &Act) -> Option<char> {
    match a {
        Act::CheckOutput(_) => Some('o'),
        Act::RerunCheck(_) => Some('r'),
        Act::OverrideCheck(..) => Some('O'),
        Act::UndoOverride(_) => Some('u'),
        _ => None,
    }
}

/// The key that runs a menu item: the leader key bound to the same action, from the live
/// keymap (pressed without the leader while the menu is open), else a letter of its own.
#[derive(Debug, Clone, PartialEq)]
pub enum MenuKey {
    Bound(crate::keys::KeySpec),
    Letter(char),
}

impl MenuKey {
    /// As its keycap reads: `v`, `Alt+←`.
    fn label(&self) -> String {
        match self {
            MenuKey::Bound(k) => super::design::long_key(k),
            MenuKey::Letter(c) => c.to_string(),
        }
    }

    /// The character it is typed as, when it is one with no modifier.
    #[cfg(test)]
    pub fn as_char(&self) -> Option<char> {
        match self {
            MenuKey::Letter(c) => Some(*c),
            MenuKey::Bound(k) if k.mods.is_empty() => if let KeyCode::Char(c) = k.code { Some(c) } else { None },
            MenuKey::Bound(_) => None,
        }
    }
}

/// Which item `k` runs: a bound key as the keymap has it, else a letter as typed, else the
/// letter whatever its case.
fn pressed(keys: &[Option<MenuKey>], k: &KeyEvent) -> Option<usize> {
    let spec = crate::keys::KeySpec::from_event(k);
    if let Some(i) = keys.iter().position(|key| *key == Some(MenuKey::Bound(spec))) {
        return Some(i);
    }
    let KeyCode::Char(c) = k.code else { return None };
    if k.modifiers.intersects(crossterm::event::KeyModifiers::CONTROL | crossterm::event::KeyModifiers::ALT) {
        return None;
    }
    let letter = |c: char| keys.iter().position(|key| *key == Some(MenuKey::Letter(c)));
    letter(c).or_else(|| letter(c.to_ascii_lowercase()))
}

/// The leader action an item does the same as, whose key it then shows: splitting, moving the
/// pane to a side, closing one pane, a checkout's changes.
fn bound_action(app: &App, act: &Act) -> Option<crate::keys::Action> {
    use crate::keys::Action;
    use crate::layout::Dir;
    match act {
        Act::Split(_, Dir::Right) => Some(Action::SplitRight),
        Act::Split(_, Dir::Down) => Some(Action::SplitDown),
        Act::Swap(a, b) => [Dir::Left, Dir::Right, Dir::Up, Dir::Down].into_iter().find(|d| crate::layout::neighbor(&app.screen.leaf_rects, *a, *d) == Some(*b)).map(Action::Swap),
        Act::End(ts) if ts.len() == 1 => Some(Action::ClosePane),
        Act::Changes(_) => Some(Action::Changes),
        _ => None,
    }
}

/// The first leader key bound to `action`, in the keymap's order.
fn bound_key(app: &App, action: &crate::keys::Action) -> Option<crate::keys::KeySpec> {
    app.keymap.prefixed_order.iter().find(|k| app.keymap.prefixed.get(k) == Some(action)).copied()
}

/// A key for each item: the leader key of the action it does the same as, else the first letter
/// of its label not taken yet (j and k move). Numbered lists (presets) use their numbers. On a
/// session with a check, its keys are reserved whichever of its actions apply, so the other
/// items' letters never move.
pub(super) fn menu_keys(app: &App, items: &[(String, Act)]) -> Vec<Option<MenuKey>> {
    let bound: Vec<Option<crate::keys::KeySpec>> = items.iter().map(|(_, a)| bound_action(app, a).and_then(|b| bound_key(app, &b))).collect();
    let letters = menu_letters(items, &bound);
    bound.into_iter().zip(letters).map(|(b, l)| b.map(MenuKey::Bound).or(l.map(MenuKey::Letter))).collect()
}

/// Each item's key as the character typed for it (None for a key with a modifier, or none).
#[cfg(test)]
pub(super) fn menu_chars(app: &App, items: &[(String, Act)]) -> Vec<Option<char>> {
    menu_keys(app, items).into_iter().map(|k| k.and_then(|k| k.as_char())).collect()
}

/// The letters of the items with no bound key, kept clear of the bound keys typed as letters.
fn menu_letters(items: &[(String, Act)], bound: &[Option<crate::keys::KeySpec>]) -> Vec<Option<char>> {
    // Taken already where a row's letters work (the sidebar): moving, actions, follow-up, why.
    let mut used: Vec<char> = vec!['j', 'k', 'a', 'm', 'i'];
    used.extend(bound.iter().flatten().filter(|k| k.mods.is_empty()).filter_map(|k| if let KeyCode::Char(c) = k.code { Some(c) } else { None }));
    // Closing is always x.
    let closes = |a: &Act| matches!(a, Act::End(_) | Act::CloseProject(_));
    if items.iter().any(|(_, a)| closes(a)) {
        used.push('x');
    }
    if items.iter().any(|(_, a)| check_key(a).is_some()) {
        used.extend(['r', 'o', 'u']);
    }
    items
        .iter()
        .zip(bound)
        .map(|((l, a), b)| {
            if b.is_some() {
                return None;
            }
            if closes(a) {
                return Some('x');
            }
            if let Some(key) = check_key(a) {
                return Some(key);
            }
            if l.chars().next().is_some_and(|c| c.is_ascii_digit()) {
                return None;
            }
            let c = l.chars().filter(|c| c.is_ascii_alphabetic()).map(|c| c.to_ascii_lowercase()).find(|c| !used.contains(c))?;
            used.push(c);
            Some(c)
        })
        .collect()
}

pub(super) fn draw_menu(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, m: &ScreenMenu) {
    let buf = f.buffer_mut();
    let keys: Vec<Vec<super::design::Seg>> = menu_keys(app, &m.items).iter().map(|k| k.as_ref().map(|k| super::design::keycap(t, &k.label())).unwrap_or_default()).collect();
    let keys_w = keys.iter().map(|k| segs_width(k)).max().unwrap_or(0);
    let danger = |a: &Act| matches!(a, Act::End(_) | Act::CloseProject(_));
    // The destructive item sits last, after a blank row.
    let gap = m.items.iter().any(|(_, a)| danger(a)) && m.items.len() > 1;
    let w = m.items.iter().map(|(l, _)| l.width() as u16).max().unwrap_or(10).max(m.title.width() as u16).clamp(16, 44) + 7 + keys_w.max(3);
    let h = m.items.len() as u16 + 3 + gap as u16;
    let x = m.at.0.min(area.right().saturating_sub(w + 1)).max(area.x);
    let y = if m.at.1 + h < area.bottom() { m.at.1 + 1 } else { m.at.1.saturating_sub(h) }.max(area.y);
    let r = Rect { x, y, width: w, height: h };
    // Clicking anywhere else closes it.
    hit(app, area, ScreenHit::Close);
    // A popover, as the why one: a raised title row over a card.
    let bgm = t.card;
    let title = truncate(&m.title, (w - 10) as usize);
    super::screen::popover_at(app, buf, r, &title, t);
    let look = super::screen::Look::of(&app.cfg.ui);
    let mut yy = r.y + 2;
    for (i, (label, act)) in m.items.iter().enumerate() {
        if gap && danger(act) && i + 1 == m.items.len() {
            yy += 1;
        }
        let row = Rect { x: r.x + 1, y: yy, width: w - 2, height: 1 };
        let on = i == m.sel;
        let bg = if on { t.hov } else { bgm };
        if on {
            super::screen::row_pill(look, f.buffer_mut(), row.x, yy, row.width, bg, bgm);
        }
        if i == m.sel {
            put(f.buffer_mut(), row.x + 1, yy, &[seg("›", Style::default().fg(t.accent).bg(bg).add_modifier(Modifier::BOLD))], row.right());
        }
        let mut ls = Style::default().fg(if danger(act) { t.err } else { t.strong }).bg(bg);
        if i == m.sel {
            ls = ls.add_modifier(Modifier::BOLD);
        }
        let key = keys.get(i).cloned().unwrap_or_default();
        let kw = segs_width(&key);
        put(f.buffer_mut(), row.x + 3, yy, &[seg(label.clone(), ls)], row.right().saturating_sub(kw + 2));
        put(f.buffer_mut(), row.right().saturating_sub(kw + 1), yy, &key, row.right());
        hit(app, row, ScreenHit::MenuPick(i));
        yy += 1;
    }
}
