//! Clean up worktrees: every worktree of a repo, why each is still there, and which to
//! remove. The ones that hold nothing of their own come ticked; the rest are yours to tick.

use super::*;
use crate::client::Bg;
use crate::client::design::{BtnKind, path_key};
use crate::client::menu::{Act, Confirm};
use crate::tidy::Standing;

#[derive(Debug, Clone, PartialEq)]
pub struct TidyRow {
    pub path: PathBuf,
    pub name: String,
    pub branch: String,
    /// Sessions in it right now: with any, it stays.
    pub sessions: usize,
    pub standing: Standing,
    pub ticked: bool,
}

impl TidyRow {
    /// Nothing runs in it, so it can be picked.
    pub fn free(&self) -> bool {
        self.sessions == 0
    }

    fn why(&self) -> String {
        match self.sessions {
            0 => self.standing.says(),
            1 => "in use: 1 session".into(),
            n => format!("in use: {n} sessions"),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct TidyView {
    pub repo: PathBuf,
    /// The project's name, for the heading.
    pub name: String,
    /// None while git is still being asked about each one.
    pub rows: Option<Vec<TidyRow>>,
    pub sel: usize,
}

impl TidyView {
    fn picked(&self) -> Vec<&TidyRow> {
        self.rows.iter().flatten().filter(|r| r.ticked && r.free()).collect()
    }
}

impl App {
    /// The clean-up list for project `pi`. Each worktree is looked at off the UI thread.
    pub(in crate::client) fn tidy_open(&mut self, pi: usize) {
        let model = self.screen_model();
        let Some(p) = model.get(pi).filter(|p| p.git) else {
            self.notify("worktrees belong to a git repo: this folder isn't one".into(), true);
            return;
        };
        // Counted from the server's own list: a pane folded into a split has no row, and
        // must still keep its worktree.
        let sessions = |dir: &Path| self.snap.terms.values().filter(|t| t.top.as_deref().is_some_and(|top| path_key(top) == path_key(dir))).count();
        let found: Vec<(PathBuf, String, String, usize)> = p.wts.iter().filter(|w| !w.main).map(|w| (w.path.clone(), w.name.clone(), w.branch.clone(), sessions(&w.path))).collect();
        let repo = p.path.clone();
        self.mode = Mode::Tidy(Box::new(TidyView { repo: repo.clone(), name: p.name.clone(), rows: None, sel: 0 }));
        self.spawn_bg(move || {
            let mut rows: Vec<TidyRow> = found
                .into_iter()
                .map(|(path, name, branch, sessions)| {
                    let standing = crate::tidy::standing(&path);
                    let ticked = sessions == 0 && standing.safe();
                    TidyRow { path, name, branch, sessions, standing, ticked }
                })
                .collect();
            // What can go first, what's in use last.
            rows.sort_by_key(|r| (!r.ticked, !r.free(), r.name.to_lowercase()));
            Bg::Tidy(repo, rows)
        });
    }

    /// The clean-up list for the project you're in (or the sidebar cursor's).
    pub(in crate::client) fn tidy_open_here(&mut self) {
        let model = self.screen_model();
        let key = self.screen.cursor.and_then(|c| model.iter().find(|p| p.sessions().any(|s| s.term == c)).map(|p| p.key.clone())).or(self.screen.proj.clone());
        match model.iter().position(|p| Some(&p.key) == key.as_ref() && p.git).or_else(|| model.iter().position(|p| p.git)) {
            Some(pi) => self.tidy_open(pi),
            None => self.notify("no git repo is open to clean up".into(), true),
        }
    }

    pub(in crate::client) fn tidy_loaded(&mut self, repo: PathBuf, rows: Vec<TidyRow>) {
        if let Mode::Tidy(v) = &mut self.mode
            && v.repo == repo
        {
            v.rows = Some(rows);
            v.sel = 0;
        }
    }

    pub(in crate::client) fn on_tidy_key(&mut self, mut v: TidyView, k: &KeyEvent) {
        let n = v.rows.as_ref().map(Vec::len).unwrap_or(0);
        match k.code {
            KeyCode::Esc => {
                self.mode = Mode::Normal;
                return;
            }
            KeyCode::Up | KeyCode::Char('k') => v.sel = v.sel.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => v.sel = (v.sel + 1).min(n.saturating_sub(1)),
            KeyCode::Char(' ') => {
                let i = v.sel;
                tidy_toggle(&mut v, i);
            }
            KeyCode::Char('a') => v.rows.iter_mut().flatten().for_each(|r| r.ticked = r.free() && r.standing.safe()),
            KeyCode::Char('n') => v.rows.iter_mut().flatten().for_each(|r| r.ticked = false),
            KeyCode::Enter => return self.tidy_ask(v),
            _ => {}
        }
        self.mode = Mode::Tidy(Box::new(v));
    }

    /// A click on row `i`: it's the selected one, and its tick flips.
    pub(in crate::client) fn tidy_click(&mut self, i: usize) {
        if let Mode::Tidy(v) = &mut self.mode {
            v.sel = i;
            tidy_toggle(v, i);
        }
    }

    /// Ask before removing what's ticked, saying plainly when work would be lost.
    pub(in crate::client) fn tidy_ask(&mut self, v: TidyView) {
        let picked = v.picked();
        if picked.is_empty() {
            self.notify("nothing is ticked".into(), true);
            self.mode = Mode::Tidy(Box::new(v));
            return;
        }
        let losing = picked.iter().filter(|r| r.standing.dirty > 0).count();
        let unmerged = picked.iter().filter(|r| r.standing.dirty == 0 && !r.standing.merged).count();
        let note = match (losing, unmerged) {
            (0, 0) => "Their folders go. Nothing in them exists only there.".to_string(),
            (0, n) => format!("Their folders go. {n} {} work that isn't merged: its branch is kept.", if n == 1 { "has" } else { "have" }),
            (n, _) => format!("{n} {} uncommitted changes, which will be lost. Branches are kept unless already merged.", if n == 1 { "has" } else { "have" }),
        };
        let items: Vec<(PathBuf, bool)> = picked.iter().map(|r| (r.path.clone(), r.standing.dirty > 0)).collect();
        self.mode = Mode::Confirm(Box::new(Confirm {
            title: "Clean up worktrees".into(),
            sub: String::new(),
            what: format!("{} worktree{}", picked.len(), if picked.len() == 1 { "" } else { "s" }),
            detail: v.name.clone(),
            note,
            list: picked.iter().map(|r| if r.standing.dirty > 0 { format!("{} ({} uncommitted)", r.name, r.standing.dirty) } else { r.name.clone() }).collect(),
            yes: "Remove".into(),
            key: '\n',
            danger: true,
            act: Act::Tidy(items),
        }));
    }
}

fn tidy_toggle(v: &mut TidyView, i: usize) {
    if let Some(r) = v.rows.as_mut().and_then(|rows| rows.get_mut(i)).filter(|r| r.free()) {
        r.ticked = !r.ticked;
    }
}

pub(in crate::client) fn draw_tidy(app: &mut App, f: &mut Frame, area: Rect, t: &Theme, v: &TidyView) {
    let buf = f.buffer_mut();
    dim_all(buf, area, t);
    let n = v.rows.as_ref().map(Vec::len).unwrap_or(1) as u16;
    let h = (n + 9).max(11).min(area.height.saturating_sub(4));
    let r = panel(app, buf, area, 112, h, "Clean up worktrees", &[], t);
    let c = Style::default().bg(t.card);
    put(
        buf,
        r.x + 3,
        r.y + 2,
        &[seg(v.name.clone(), c.fg(t.strong).add_modifier(Modifier::BOLD)), seg("  a folder goes; its branch is kept unless it's already merged", c.fg(t.muted))],
        r.right().saturating_sub(2),
    );
    let list = Rect { x: r.x + 1, y: r.y + 4, width: r.width.saturating_sub(2), height: r.height.saturating_sub(9) };
    let quiet = c.fg(t.muted).add_modifier(Modifier::ITALIC);
    match &v.rows {
        None => {
            put(buf, list.x + 2, list.y, &[seg("looking at each worktree…", quiet)], list.right());
        }
        Some(rows) if rows.is_empty() => {
            put(buf, list.x + 2, list.y, &[seg("no worktrees besides the main folder", quiet)], list.right());
        }
        Some(rows) => {
            let start = v.sel.saturating_sub(list.height.saturating_sub(1) as usize);
            for (i, row) in rows.iter().enumerate().skip(start).take(list.height as usize) {
                let y = list.y + (i - start) as u16;
                let rr = Rect { y, height: 1, ..list };
                let bg = if i == v.sel || hovered(app, rr) { t.hov } else { t.card };
                fill(buf, rr, bg);
                let st = Style::default().bg(bg);
                let (mark, mark_c) = match (row.free(), row.ticked) {
                    (false, _) => ("·", t.muted),
                    (true, true) => ("✓", t.accent),
                    (true, false) => ("○", t.muted),
                };
                let why_c = match (row.free(), row.standing.dirty, row.standing.merged) {
                    (false, ..) => t.muted,
                    (_, 1.., _) => t.sky(),
                    (_, _, true) => t.done,
                    _ => t.text,
                };
                put(
                    buf,
                    rr.x + 2,
                    y,
                    &[
                        seg(format!("{mark} "), st.fg(mark_c).add_modifier(Modifier::BOLD)),
                        seg(format!("{:<28}", truncate(&row.name, 28)), st.fg(if row.free() { t.strong } else { t.muted }).add_modifier(Modifier::BOLD)),
                        seg(format!("  {:<30}", truncate(&row.branch, 30)), st.fg(t.muted)),
                        seg(format!("  {}", row.why()), st.fg(why_c)),
                    ],
                    rr.right().saturating_sub(1),
                );
                hit(app, rr, ScreenHit::TidyRow(i));
            }
        }
    }
    let picked = v.picked();
    let losing = picked.iter().any(|r| r.standing.dirty > 0);
    let by = r.bottom().saturating_sub(3);
    let label = format!("Remove {}", picked.len());
    btn(app, buf, r.x + 3, by, &label, "Enter", if losing { BtnKind::Danger } else { BtnKind::Primary }, ScreenHit::TidyGo, r.right());
    panel_bar(buf, r, t, &[("Space", "tick"), ("a", "all the safe ones"), ("n", "none"), ("Esc", "close")]);
}
