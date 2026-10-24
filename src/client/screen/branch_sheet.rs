//! The Branch view in the side panel: where a session's checkout stands. Its branch, how far
//! it is from its upstream and its base branch, its last commits, its last check and its
//! snapshots. The first row opens the branch switcher; Enter on a snapshot asks, then restores
//! it. Git is read in a background task, so the sheet opens at once and fills in.

use super::*;
use super::check;
use crate::client::menu::Act;
use crate::client::{Bg, Btn, View};
use crate::proc::git;
use crate::snapshot::Snapshot;

const COMMITS_SHOWN: usize = 10;
const SWITCH_ROW: &str = "Switch branch…";
/// The label column of the Branch / Upstream / Base / Check rows.
const LABEL_W: usize = 10;
/// Rows under the list: a gap and the hints.
const FOOT_ROWS: u16 = 2;
const HINTS: [(&str, &str); 3] = [("Enter", "open"), ("↑↓", "choose"), ("Esc", "close")];

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Commit {
    pub hash: String,
    pub subject: String,
    /// Committed at (unix seconds).
    pub at: u64,
}

/// How far the checkout is from another branch: commits only here, and only there.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Gap {
    pub name: String,
    pub ahead: u32,
    pub behind: u32,
}

/// The branch it tracks.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Upstream {
    None,
    /// Set, but the remote branch is no longer there.
    Gone(String),
    At(Gap),
}

/// Each part is read on its own, so one git error shows in its place and not over the rest.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct Repo {
    pub branch: String,
    pub upstream: Result<Upstream, String>,
    /// None on the base branch itself.
    pub base: Result<Option<Gap>, String>,
    pub commits: Result<Vec<Commit>, String>,
    pub snapshots: Vec<Snapshot>,
}

/// What git said about the folder.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Facts {
    NotGit,
    /// git itself failed (not installed, a broken repo): its words.
    Failed(String),
    Repo(Box<Repo>),
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BranchSheet {
    pub dir: PathBuf,
    /// The session it was opened for: its check, and the agent a restore must wait for.
    pub term: TermId,
    /// None while git is being read.
    pub facts: Option<Facts>,
    /// The row the keys are on: 0 is "Switch branch…", then one per snapshot.
    pub sel: usize,
    /// The first line in view, so the chosen row stays on screen.
    scroll: usize,
}

impl BranchSheet {
    fn new(dir: PathBuf, term: TermId) -> BranchSheet {
        BranchSheet { dir, term, facts: None, sel: 0, scroll: 0 }
    }

    /// Rows Enter acts on. While git is read, the switcher is already there.
    fn rows(&self) -> usize {
        match &self.facts {
            None => 1,
            Some(Facts::Repo(r)) => 1 + r.snapshots.len(),
            Some(Facts::NotGit | Facts::Failed(_)) => 0,
        }
    }
}

/// Everything the view shows about `dir`, read with git. Slow: run it off the UI thread.
pub(crate) fn read(dir: &Path) -> Facts {
    if let Err(e) = git(dir, &["rev-parse", "--show-toplevel"]) {
        return if e.contains("not a git repository") { Facts::NotGit } else { Facts::Failed(e) };
    }
    match read_repo(dir) {
        Ok(r) => Facts::Repo(Box::new(r)),
        Err(e) => Facts::Failed(e),
    }
}

fn read_repo(dir: &Path) -> Result<Repo, String> {
    let name = git(dir, &["branch", "--show-current"])?.trim().to_string();
    let (branch, upstream) = if name.is_empty() {
        (format!("detached at {}", git(dir, &["rev-parse", "--short", "HEAD"])?.trim()), Ok(Upstream::None))
    } else {
        (name.clone(), upstream(dir, &name))
    };
    let base = base_branch(dir, &branch).map(|b| gap(dir, &b)).transpose();
    Ok(Repo { branch, upstream, base, commits: commits(dir), snapshots: crate::snapshot::list(dir) })
}

/// What `branch` tracks, read from its ref so an upstream that is gone reads as gone.
fn upstream(dir: &Path, branch: &str) -> Result<Upstream, String> {
    let out = git(dir, &["for-each-ref", "--format=%(upstream:short)%09%(upstream:track)", &format!("refs/heads/{branch}")])?;
    let (name, track) = out.trim_end().split_once('\t').unwrap_or((out.trim(), ""));
    if name.is_empty() {
        return Ok(Upstream::None);
    }
    if track == "[gone]" {
        return Ok(Upstream::Gone(name.to_string()));
    }
    let (ahead, behind) = parse_track(track)?;
    Ok(Upstream::At(Gap { name: name.to_string(), ahead, behind }))
}

/// `[ahead 1, behind 2]`, `[behind 3]`, or nothing when level.
fn parse_track(track: &str) -> Result<(u32, u32), String> {
    let (mut ahead, mut behind) = (0, 0);
    let inner = track.trim_start_matches('[').trim_end_matches(']');
    for part in inner.split(", ").filter(|p| !p.is_empty()) {
        let bad = || format!("git said {track:?} about the upstream");
        let (word, n) = part.split_once(' ').ok_or_else(bad)?;
        let n: u32 = n.parse().map_err(|_| bad())?;
        match word {
            "ahead" => ahead = n,
            "behind" => behind = n,
            _ => return Err(bad()),
        }
    }
    Ok((ahead, behind))
}

/// The branch this one would merge into: the main checkout's, else `main`, else `master`.
/// None on that branch itself.
fn base_branch(dir: &Path, current: &str) -> Option<String> {
    let main_checkouts = crate::gitfs::head(dir).and_then(|h| crate::gitfs::main_branch(&h.main_root));
    main_checkouts
        .into_iter()
        .chain(["main".to_string(), "master".to_string()])
        .filter(|b| b != current)
        .find(|b| git(dir, &["rev-parse", "--verify", "--quiet", &format!("refs/heads/{b}")]).is_ok())
}

/// How far HEAD is from `other`.
fn gap(dir: &Path, other: &str) -> Result<Gap, String> {
    let out = git(dir, &["rev-list", "--left-right", "--count", &format!("HEAD...{other}")])?;
    let mut n = out.split_whitespace().map(str::parse::<u32>);
    match (n.next(), n.next()) {
        (Some(Ok(ahead)), Some(Ok(behind))) => Ok(Gap { name: other.to_string(), ahead, behind }),
        _ => Err(format!("git said {:?} comparing with {other}", out.trim())),
    }
}

fn commits(dir: &Path) -> Result<Vec<Commit>, String> {
    // A branch with no commits yet has no HEAD to log: an empty list, not an error.
    if git(dir, &["rev-parse", "--verify", "--quiet", "HEAD"]).is_err() {
        return Ok(Vec::new());
    }
    let out = git(dir, &["log", &format!("-{COMMITS_SHOWN}"), "--format=%h%x09%ct%x09%s"])?;
    Ok(out
        .lines()
        .filter_map(|l| {
            let mut parts = l.splitn(3, '\t');
            let hash = parts.next()?.to_string();
            let at = parts.next()?.parse().ok()?;
            Some(Commit { hash, at, subject: parts.next().unwrap_or("").to_string() })
        })
        .collect())
}

/// "2m ago · before claude's turn", as the row menu's Snapshots… list says it.
fn snapshot_words(s: &Snapshot) -> String {
    format!("{} ago · {}", age(s.at), s.why)
}

impl App {
    /// `b`: the Branch view of the session you're on, or closed again when it's open.
    pub(in crate::client) fn toggle_branch_sheet(&mut self) {
        if matches!(self.view, Some(View::Branch(_))) {
            self.view = None;
            return;
        }
        let Some(term) = self.screen.cursor.or(self.focused()) else {
            return self.notify("no session to show the branch of".into(), true);
        };
        let dir = self.screen_target_dir();
        self.screen.cursor = None;
        self.mode = Mode::Normal;
        self.view = Some(View::Branch(Box::new(BranchSheet::new(dir.clone(), term))));
        self.read_branch_facts(dir);
    }

    fn read_branch_facts(&self, dir: PathBuf) {
        self.spawn_bg(move || {
            let facts = read(&dir);
            Bg::Then(Box::new(move |app: &mut App| app.branch_facts_arrived(&dir, facts)))
        });
    }

    pub(in crate::client) fn branch_facts_arrived(&mut self, dir: &Path, facts: Facts) {
        if let Some(View::Branch(v)) = &mut self.view
            && v.dir == dir
        {
            v.facts = Some(facts);
            v.sel = v.sel.min(v.rows().saturating_sub(1));
        }
    }

    /// After a switch or a restore, the open Branch view reads git again.
    pub(in crate::client) fn branch_sheet_reload(&self) {
        if let Some(View::Branch(v)) = &self.view {
            self.read_branch_facts(v.dir.clone());
        }
    }

    /// Keys in the Branch view; returns false when it closes.
    pub(in crate::client) fn on_branch_sheet_key(&mut self, v: &mut BranchSheet, k: &KeyEvent) -> bool {
        match k.code {
            KeyCode::Esc | KeyCode::Char('b') => return false,
            KeyCode::Down | KeyCode::Char('j') => v.sel = (v.sel + 1).min(v.rows().saturating_sub(1)),
            KeyCode::Up | KeyCode::Char('k') => v.sel = v.sel.saturating_sub(1),
            KeyCode::Enter => self.branch_sheet_enter(v),
            _ => {}
        }
        true
    }

    fn branch_sheet_enter(&mut self, v: &BranchSheet) {
        if v.sel >= v.rows() {
            return;
        }
        let Some(i) = v.sel.checked_sub(1) else { return self.open_branches(Some(v.dir.clone())) };
        if let Some(Facts::Repo(r)) = &v.facts
            && let Some(s) = r.snapshots.get(i)
        {
            self.menu_act(Act::Restore(v.dir.clone(), s.commit.clone(), snapshot_words(s), v.term));
        }
    }
}

/// One line of the sheet: its words, what sits at its right, and the row it is when Enter
/// can act on it.
#[derive(Default)]
struct Line {
    segs: Vec<Seg>,
    right: Vec<Seg>,
    row: Option<usize>,
}

fn text(segs: Vec<Seg>) -> Line {
    Line { segs, ..Line::default() }
}

fn labelled(label: &str, mut rest: Vec<Seg>, c: Style, t: &Theme) -> Line {
    let mut segs = vec![seg(format!("{label:<LABEL_W$}"), c.fg(t.muted))];
    segs.append(&mut rest);
    text(segs)
}

fn gap_words(g: &Gap, c: Style, t: &Theme) -> Vec<Seg> {
    let mut out = vec![seg(g.name.clone(), c.fg(t.text))];
    if g.ahead == 0 && g.behind == 0 {
        out.push(seg("  up to date", c.fg(t.muted)));
    } else {
        out.push(seg(format!("  ↑{} ↓{}", g.ahead, g.behind), c.fg(t.strong).add_modifier(Modifier::BOLD)));
    }
    out
}

fn git_error(e: &str, c: Style, t: &Theme) -> Vec<Seg> {
    vec![seg(format!("couldn't read: {e}"), c.fg(t.err))]
}

fn heading(name: &str, c: Style, t: &Theme) -> Line {
    text(vec![seg(name, c.fg(t.muted).add_modifier(Modifier::BOLD))])
}

fn repo_lines(app: &App, v: &BranchSheet, r: &Repo, c: Style, t: &Theme) -> Vec<Line> {
    let mut out = vec![labelled("Branch", vec![seg(r.branch.clone(), c.fg(t.strong).add_modifier(Modifier::BOLD))], c, t)];
    let upstream = match &r.upstream {
        Ok(Upstream::None) => vec![seg("none", c.fg(t.muted))],
        Ok(Upstream::Gone(name)) => vec![seg(name.clone(), c.fg(t.text)), seg("  gone from the remote", c.fg(t.err))],
        Ok(Upstream::At(g)) => gap_words(g, c, t),
        Err(e) => git_error(e, c, t),
    };
    out.push(labelled("Upstream", upstream, c, t));
    match &r.base {
        Ok(Some(g)) => out.push(labelled("Base", gap_words(g, c, t), c, t)),
        Ok(None) => {}
        Err(e) => out.push(labelled("Base", git_error(e, c, t), c, t)),
    }
    let check = match app.snap.terms.get(&v.term).and_then(|s| s.check()) {
        Some(info) => check::result_words(info, t).into_iter().map(|(s, st)| (s, st.bg(c.bg.unwrap_or(t.bg)))).collect(),
        None => vec![seg("none has run", c.fg(t.muted))],
    };
    out.push(labelled("Check", check, c, t));
    out.push(Line::default());
    out.push(heading("COMMITS", c, t));
    match &r.commits {
        Err(e) => out.push(text(git_error(e, c, t))),
        Ok(list) if list.is_empty() => out.push(text(vec![seg("no commits yet", c.fg(t.muted))])),
        Ok(list) => out.extend(list.iter().map(|cm| Line {
            segs: vec![seg(cm.hash.clone(), c.fg(t.accent)), seg(format!("  {}", cm.subject), c.fg(t.text))],
            right: vec![seg(age(cm.at), c.fg(t.muted))],
            row: None,
        })),
    }
    out.push(Line::default());
    out.push(heading("SNAPSHOTS", c, t));
    if r.snapshots.is_empty() {
        out.push(text(vec![seg("none yet: one is kept each time an agent's turn begins", c.fg(t.muted))]));
    }
    for (i, s) in r.snapshots.iter().enumerate() {
        out.push(Line { segs: vec![seg(snapshot_words(s), c.fg(t.text))], right: Vec::new(), row: Some(i + 1) });
    }
    out
}

fn lines(app: &App, v: &BranchSheet, c: Style, t: &Theme) -> Vec<Line> {
    match &v.facts {
        Some(Facts::NotGit) => {
            return vec![
                text(vec![seg(format!("{} isn't in a git repository.", folder_name(&v.dir)), c.fg(t.text))]),
                text(vec![seg("No branch, commits or snapshots to show.", c.fg(t.muted))]),
            ];
        }
        Some(Facts::Failed(e)) => return vec![text(git_error(e, c, t))],
        _ => {}
    }
    let mut out = vec![Line { segs: vec![seg(SWITCH_ROW, c.fg(t.accent).add_modifier(Modifier::BOLD))], right: Vec::new(), row: Some(0) }, Line::default()];
    match &v.facts {
        Some(Facts::Repo(r)) => out.extend(repo_lines(app, v, r, c, t)),
        _ => out.push(text(vec![seg("reading git…", c.fg(t.muted).add_modifier(Modifier::ITALIC))])),
    }
    out
}

/// The Branch view inside the sheet's card; `bg` is the card's ground.
pub(in crate::client) fn draw_branch(app: &mut App, buf: &mut Buffer, body: Rect, bg: Color, t: &Theme, v: &mut BranchSheet) {
    let c = Style::default().bg(bg);
    let list = Rect { x: body.x + 1, width: body.width.saturating_sub(2), height: body.height.saturating_sub(FOOT_ROWS), ..body };
    let all = lines(app, v, c, t);
    let h = list.height as usize;
    if let Some(at) = all.iter().position(|l| l.row == Some(v.sel)) {
        v.scroll = v.scroll.min(at).max((at + 1).saturating_sub(h));
    }
    for (i, line) in all.iter().enumerate().skip(v.scroll).take(h) {
        let rr = Rect { y: list.y + (i - v.scroll) as u16, height: 1, ..list };
        let lit = line.row.is_some_and(|row| row == v.sel || hovered(app, rr));
        let row_bg = if lit { t.hov } else { bg };
        let segs: Vec<Seg> = line.segs.iter().map(|(s, st)| (s.clone(), st.bg(row_bg))).collect();
        if let Some(row) = line.row {
            fill(buf, rr, row_bg);
            app.hits.push((rr, Hit::Button(Btn::Row(row))));
        }
        let rw = segs_width(&line.right);
        put(buf, rr.x + 1, rr.y, &segs, rr.right().saturating_sub(rw + 2));
        if rw > 0 {
            let right: Vec<Seg> = line.right.iter().map(|(s, st)| (s.clone(), st.bg(row_bg))).collect();
            put(buf, rr.right().saturating_sub(rw + 1), rr.y, &right, rr.right());
        }
    }
    status_bar(buf, body, t, &HINTS, &[]);
}

/// What follows the sheet's title: the checkout's folder.
pub(in crate::client) fn branch_title(app: &App) -> String {
    let Some(View::Branch(v)) = &app.view else { return String::new() };
    format!("· {}", folder_name(&v.dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::brand::APP_COMMAND;

    /// A repo on `main` with two commits, and a way to run git in it.
    fn repo(tag: &str) -> (PathBuf, impl Fn(&Path, &[&str]) -> String) {
        let root = std::env::temp_dir().join(format!("{APP_COMMAND}-branch-sheet-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let git = |dir: &Path, args: &[&str]| crate::proc::git(dir, args).unwrap();
        git(&root, &["init", "-q", "-b", "main"]);
        git(&root, &["config", "user.email", "t@t"]);
        git(&root, &["config", "user.name", "t"]);
        for (i, msg) in ["first", "second"].iter().enumerate() {
            std::fs::write(root.join("a.txt"), i.to_string()).unwrap();
            git(&root, &["add", "."]);
            git(&root, &["commit", "-qm", msg]);
        }
        (root, git)
    }

    #[test]
    fn a_branch_is_measured_against_its_base_and_its_upstream() {
        let (root, git) = repo("gaps");
        // A clone whose branch tracks origin/main, then moves on its own feature branch.
        let clone = root.with_extension("clone");
        let _ = std::fs::remove_dir_all(&clone);
        git(&root, &["clone", "-q", root.to_str().unwrap(), clone.to_str().unwrap()]);
        git(&clone, &["config", "user.email", "t@t"]);
        git(&clone, &["config", "user.name", "t"]);
        git(&clone, &["switch", "-q", "-c", "feature", "--track", "origin/main"]);
        std::fs::write(clone.join("b.txt"), "b").unwrap();
        git(&clone, &["add", "."]);
        git(&clone, &["commit", "-qm", "feature work"]);
        let Facts::Repo(r) = read(&clone) else { panic!("a repo") };
        assert_eq!(r.branch, "feature");
        assert_eq!(r.upstream, Ok(Upstream::At(Gap { name: "origin/main".into(), ahead: 1, behind: 0 })));
        assert_eq!(r.base, Ok(Some(Gap { name: "main".into(), ahead: 1, behind: 0 })), "main is its base");
        let commits = r.commits.as_ref().unwrap();
        assert_eq!(commits.iter().map(|c| c.subject.as_str()).collect::<Vec<_>>(), ["feature work", "second", "first"], "newest first");
        assert!(commits.iter().all(|c| c.hash.len() >= 7 && c.at > 0));
        // The branch it tracks goes from the remote: gone, not "none".
        git(&clone, &["config", "branch.feature.merge", "refs/heads/deleted"]);
        let Facts::Repo(r) = read(&clone) else { panic!("a repo") };
        assert_eq!(r.upstream, Ok(Upstream::Gone("origin/deleted".into())));
        let _ = std::fs::remove_dir_all(&root);
        let _ = std::fs::remove_dir_all(&clone);
    }

    #[test]
    fn the_upstream_track_is_read_or_refused() {
        assert_eq!(parse_track(""), Ok((0, 0)), "level");
        assert_eq!(parse_track("[ahead 1, behind 2]"), Ok((1, 2)));
        assert_eq!(parse_track("[behind 3]"), Ok((0, 3)));
        assert!(parse_track("[sideways 1]").is_err(), "words it doesn't know are an error, not zero");
    }

    #[test]
    fn a_git_error_is_shown_not_dropped() {
        let (root, _) = repo("err");
        let Err(e) = gap(&root, "no-such-branch") else { panic!("an error") };
        assert!(!e.is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn only_the_last_ten_commits_are_listed() {
        let (root, git) = repo("ten");
        for i in 0..12 {
            git(&root, &["commit", "-q", "--allow-empty", "-m", &format!("c{i}")]);
        }
        let Facts::Repo(r) = read(&root) else { panic!("a repo") };
        let commits = r.commits.as_ref().unwrap();
        assert_eq!(commits.len(), COMMITS_SHOWN);
        assert_eq!(commits[0].subject, "c11");
        assert_eq!((r.upstream, r.base), (Ok(Upstream::None), Ok(None)), "main with no remote: nothing to measure against");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_folder_outside_git_is_not_a_repo() {
        let dir = std::env::temp_dir().join(format!("{APP_COMMAND}-branch-sheet-plain-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // The temp folder may itself sit in a repo on some machines; then there is nothing to test.
        if crate::proc::git(&dir, &["rev-parse", "--show-toplevel"]).is_err() {
            assert_eq!(read(&dir), Facts::NotGit);
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn snapshots_are_listed_newest_first() {
        let (root, _) = repo("snaps");
        std::fs::write(root.join("a.txt"), "changed").unwrap();
        crate::snapshot::take(&root, "before claude's turn").unwrap();
        let Facts::Repo(r) = read(&root) else { panic!("a repo") };
        assert_eq!(r.snapshots.len(), 1);
        assert_eq!(r.snapshots[0].why, "before claude's turn");
        let _ = std::fs::remove_dir_all(&root);
    }
}
