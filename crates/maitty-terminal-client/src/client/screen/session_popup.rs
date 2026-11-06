//! The New Session popup: one place to start a session on an open project, in any folder, on any
//! machine. Every row is a path you can type, as scp writes it: `~/code/maitty` is this computer,
//! `kingdom:~/maitty` is the machine named kingdom. This file is the model, with no drawing.

use super::*;
use crate::protocol::FolderListing;
use std::collections::BTreeMap;

/// The most folders the plain search lists under FOLDERS.
const FOLDERS_SHOWN: usize = 9;
/// How many paths a session was started in are remembered.
pub(in crate::client) const RECENT_KEEP: usize = 5;
/// The most parent folders asked about when the popup opens, so a search can match folders that
/// are not yet open projects.
const INDEX_FOLDERS_MAX: usize = 8;
/// The folder every machine's index starts from.
const ROOT: &str = "~/";

/// A machine the popup can start a session on.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct Chip {
    pub id: MachineId,
    /// Empty for this computer.
    pub name: String,
    pub state: ChipState,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum ChipState {
    Online,
    Connecting,
    Offline,
}

/// An open project, as the popup lists it.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct ProjectRow {
    pub machine: MachineId,
    pub path: String,
    pub branch: Option<String>,
    pub sessions: usize,
    pub needs: bool,
}

/// What the query means.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::client) enum Scope {
    /// Fuzzy search across every machine.
    Plain,
    /// Browsing one machine's folders. `path` is what follows the host, `~/` when nothing does.
    Path { machine: MachineId, path: String },
}

/// What a folder's listing is, once asked for.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) enum Listing {
    Loading,
    Ready(FolderListing),
}

/// The popup's state.
#[derive(Debug, Clone, PartialEq, Default)]
pub(in crate::client) struct NewSession {
    pub q: String,
    /// The row selected; None means the first one that can be.
    pub sel: Option<usize>,
    /// The machine chip the arrow keys are on; None while a list row is selected.
    pub chip: Option<usize>,
    /// Listings asked for or received, by machine and folder as typed.
    pub cache: BTreeMap<(MachineId, String), Listing>,
    /// Questions out: tag to the folder it was about.
    pub asked: BTreeMap<u64, (MachineId, String)>,
    pub next_tag: u64,
}

/// One line of the list.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) enum Row {
    Section { label: &'static str, note: String },
    Blank,
    Note(&'static str),
    StartHere {
        machine: MachineId,
        /// The folder as typed, with its host.
        shown: String,
        /// Where the daemon says it is, once it has answered.
        resolved: Option<String>,
        branch: Option<String>,
    },
    Project {
        machine: MachineId,
        shown: String,
        branch: Option<String>,
        sessions: usize,
        needs: bool,
    },
    Folder {
        machine: MachineId,
        /// The path as typed, with its host and a trailing `/`.
        shown: String,
        /// How many characters of `shown` are the part already typed (path mode).
        typed: usize,
        /// Indices into the last segment of `shown` that the search matched (plain mode).
        matched: Vec<usize>,
        branch: Option<String>,
        has_children: bool,
        open: usize,
        needs: bool,
    },
}

impl Row {
    pub fn selectable(&self) -> bool {
        matches!(self, Row::StartHere { .. } | Row::Project { .. } | Row::Folder { .. })
    }

    /// What Tab puts in the query for this row.
    pub fn fill(&self) -> Option<String> {
        match self {
            Row::Project { shown, .. } | Row::Folder { shown, .. } => Some(shown.clone()),
            Row::StartHere { shown, .. } => Some(shown.clone()),
            _ => None,
        }
    }
}

/// What Enter does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::client) enum Go {
    Start { machine: MachineId, path: String },
    Fill(String),
    Reconnect(MachineId),
    Nothing,
}

/// How a typed host is told from a path on this computer.
pub(in crate::client) fn host_of(q: &str) -> Option<(&str, &str)> {
    let (host, rest) = q.split_once(':')?;
    let name_char = |c: char| c.is_alphanumeric() || matches!(c, '_' | '.' | '-');
    (!host.is_empty() && host.chars().all(name_char)).then_some((host, rest))
}

/// The query's meaning: `host:path` on a machine of that name, `~`, `/` or `.` first on this
/// computer, anything else a search.
pub(in crate::client) fn scope_of(q: &str, chips: &[Chip]) -> Scope {
    if let Some((host, rest)) = host_of(q)
        && let Some(chip) = chips
            .iter()
            .find(|c| !c.name.is_empty() && c.name.eq_ignore_ascii_case(host))
    {
        let path = if rest.is_empty() { ROOT } else { rest };
        return Scope::Path {
            machine: chip.id,
            path: path.to_string(),
        };
    }
    if q.starts_with(['~', '/', '.']) {
        return Scope::Path {
            machine: MachineId::LOCAL,
            path: q.to_string(),
        };
    }
    Scope::Plain
}

/// The folder a path is inside (up to its last `/`) and the part still being typed after it.
pub(in crate::client) fn split_folder(path: &str) -> (String, String) {
    match path.rfind('/') {
        Some(i) => (path[..=i].to_string(), path[i + 1..].to_string()),
        None => (ROOT.to_string(), path.to_string()),
    }
}

/// A path with its machine, as it is typed.
pub(in crate::client) fn typed(chips: &[Chip], machine: MachineId, path: &str) -> String {
    match chips.iter().find(|c| c.id == machine) {
        Some(chip) if !chip.name.is_empty() => format!("{}:{path}", chip.name),
        _ => path.to_string(),
    }
}

/// The in-order, case-insensitive match of `query` in `text`: the char indices that matched.
pub(in crate::client) fn fuzzy_indices(query: &str, text: &str) -> Option<Vec<usize>> {
    let mut wanted = query.chars().flat_map(char::to_lowercase).peekable();
    let mut at = Vec::new();
    for (i, c) in text.chars().enumerate() {
        let Some(&next) = wanted.peek() else { break };
        if c.to_lowercase().next() == Some(next) {
            at.push(i);
            wanted.next();
        }
    }
    wanted.peek().is_none().then_some(at)
}

/// The rest of the selected row's path after what is typed, when the row starts with it.
pub(in crate::client) fn ghost(q: &str, fill: &str) -> Option<String> {
    if q.is_empty() {
        return None;
    }
    let (head, tail) = (fill.get(..q.len())?, fill.get(q.len()..)?);
    (head.eq_ignore_ascii_case(q) && !tail.is_empty()).then(|| tail.to_string())
}

/// Ctrl or Alt + Backspace: back over one path segment.
pub(in crate::client) fn delete_segment(q: &str) -> String {
    // A host with no path after it goes whole.
    if q.ends_with(':') {
        return String::new();
    }
    let trimmed = q.trim_end_matches('/');
    match trimmed.rfind(['/', ':']) {
        Some(i) => trimmed[..=i].to_string(),
        None => String::new(),
    }
}

fn ends_in(name: &str, path: &str) -> String {
    if name.is_empty() {
        path.to_string()
    } else {
        format!("{name}:{path}")
    }
}

impl NewSession {
    pub fn new(q: &str) -> NewSession {
        NewSession {
            q: q.to_string(),
            ..NewSession::default()
        }
    }

    /// Folders to ask about when the popup opens: each machine's root and the folders its open
    /// projects sit in, so a search can find a folder that is not a project yet.
    pub fn index_folders(chips: &[Chip], projects: &[ProjectRow]) -> Vec<(MachineId, String)> {
        let mut out: Vec<(MachineId, String)> = Vec::new();
        for chip in chips.iter().filter(|c| c.state == ChipState::Online) {
            out.push((chip.id, ROOT.to_string()));
        }
        for p in projects {
            if out.len() >= INDEX_FOLDERS_MAX + chips.len() {
                break;
            }
            let (parent, _) = split_folder(p.path.trim_end_matches('/'));
            let key = (p.machine, parent);
            if !out.contains(&key) && chips.iter().any(|c| c.id == p.machine && c.state == ChipState::Online) {
                out.push(key);
            }
        }
        out
    }

    fn ready(&self, machine: MachineId, folder: &str) -> Option<&FolderListing> {
        match self.cache.get(&(machine, folder.to_string())) {
            Some(Listing::Ready(listing)) => Some(listing),
            _ => None,
        }
    }

    /// Whether the folder the query is in is still being read.
    pub fn loading(&self, chips: &[Chip]) -> bool {
        match scope_of(&self.q, chips) {
            Scope::Path { machine, path } => {
                let (folder, _) = split_folder(&path);
                matches!(self.cache.get(&(machine, folder)), Some(Listing::Loading))
            }
            Scope::Plain => false,
        }
    }

    /// The folder that must be read for the query's rows, if it has not been asked for.
    pub fn wanted(&self, chips: &[Chip]) -> Option<(MachineId, String)> {
        match scope_of(&self.q, chips) {
            Scope::Path { machine, path } => {
                let (folder, _) = split_folder(&path);
                (!self.cache.contains_key(&(machine, folder.clone()))).then_some((machine, folder))
            }
            Scope::Plain => None,
        }
    }

    /// Every row for the query, in order.
    pub fn rows(&self, chips: &[Chip], projects: &[ProjectRow], recent: &[String]) -> Vec<Row> {
        match scope_of(&self.q, chips) {
            Scope::Path { machine, path } => self.path_rows(chips, projects, machine, &path),
            Scope::Plain => self.plain_rows(chips, projects, recent),
        }
    }

    fn name_of(chips: &[Chip], machine: MachineId) -> &str {
        chips
            .iter()
            .find(|c| c.id == machine)
            .map_or("", |c| c.name.as_str())
    }

    fn open_at(projects: &[ProjectRow], machine: MachineId, path: &str) -> (usize, bool) {
        let wanted = path.trim_end_matches('/');
        projects
            .iter()
            .filter(|p| p.machine == machine && p.path.trim_end_matches('/') == wanted)
            .fold((0, false), |(n, needs), p| (n + p.sessions, needs || p.needs))
    }

    fn plain_rows(&self, chips: &[Chip], projects: &[ProjectRow], recent: &[String]) -> Vec<Row> {
        let q = self.q.trim();
        let mut open: Vec<Row> = Vec::new();
        for p in projects {
            let name = Self::name_of(chips, p.machine);
            let full = ends_in(name, &format!("{}/", p.path.trim_end_matches('/')));
            let last = p.path.trim_end_matches('/').rsplit('/').next().unwrap_or("");
            if !q.is_empty() && fuzzy_indices(q, last).is_none() && fuzzy_indices(q, &full).is_none() {
                continue;
            }
            open.push(Row::Project {
                machine: p.machine,
                shown: full,
                branch: p.branch.clone(),
                sessions: p.sessions,
                needs: p.needs,
            });
        }
        let mut rows = Vec::new();
        if !open.is_empty() {
            rows.push(Row::Section {
                label: "OPEN",
                note: open.len().to_string(),
            });
            rows.extend(open);
        }
        if q.is_empty() {
            let kept: Vec<_> = recent.iter().take(RECENT_KEEP).collect();
            if !kept.is_empty() {
                if !rows.is_empty() {
                    rows.push(Row::Blank);
                }
                rows.push(Row::Section {
                    label: "RECENT",
                    note: String::new(),
                });
                for path in kept {
                    let (machine, _) = recent_target(path, chips);
                    rows.push(Row::Folder {
                        machine,
                        shown: path.clone(),
                        typed: 0,
                        matched: Vec::new(),
                        branch: None,
                        has_children: true,
                        open: 0,
                        needs: false,
                    });
                }
            }
            if rows.is_empty() {
                rows.push(Row::Note("nothing yet · type ~/ to browse"));
            }
            return rows;
        }
        let already: Vec<String> = rows
            .iter()
            .filter_map(|r| match r {
                Row::Project { shown, .. } => Some(shown.clone()),
                _ => None,
            })
            .collect();
        let mut found: Vec<(bool, usize, Row)> = Vec::new();
        for ((machine, folder), listing) in &self.cache {
            let Listing::Ready(listing) = listing else { continue };
            if !chips.iter().any(|c| c.id == *machine && c.state == ChipState::Online) {
                continue;
            }
            let name = Self::name_of(chips, *machine);
            for entry in &listing.entries {
                let Some(at) = fuzzy_indices(q, &entry.name) else { continue };
                let shown = ends_in(name, &format!("{folder}{}/", entry.name));
                if already.contains(&shown) {
                    continue;
                }
                let prefix = entry.name.to_lowercase().starts_with(&q.to_lowercase());
                let depth = folder.matches('/').count();
                let path_only = format!("{folder}{}/", entry.name);
                let (open_count, needs) = Self::open_at(projects, *machine, &path_only);
                found.push((
                    !prefix,
                    depth,
                    Row::Folder {
                        machine: *machine,
                        shown,
                        typed: 0,
                        matched: at,
                        branch: entry.git.clone(),
                        has_children: entry.has_children,
                        open: open_count,
                        needs,
                    },
                ));
            }
        }
        found.sort_by(|a, b| (a.0, a.1).cmp(&(b.0, b.1)).then_with(|| row_key(&a.2).cmp(&row_key(&b.2))));
        found.truncate(FOLDERS_SHOWN);
        if !found.is_empty() {
            if !rows.is_empty() {
                rows.push(Row::Blank);
            }
            rows.push(Row::Section {
                label: "FOLDERS",
                note: "every machine".to_string(),
            });
            rows.extend(found.into_iter().map(|(_, _, row)| row));
        }
        if rows.is_empty() {
            rows.push(Row::Note("nothing here matches · ⌫ to go back"));
        }
        rows
    }

    fn path_rows(&self, chips: &[Chip], projects: &[ProjectRow], machine: MachineId, path: &str) -> Vec<Row> {
        let name = Self::name_of(chips, machine);
        let (folder, part) = split_folder(path);
        let Some(listing) = self.ready(machine, &folder) else {
            return Vec::new();
        };
        if !listing.exists {
            return vec![Row::Note("nothing here matches · ⌫ to go back")];
        }
        let mut rows = Vec::new();
        if part.is_empty() {
            let branch = folder_branch(self, machine, &folder);
            rows.push(Row::StartHere {
                machine,
                shown: ends_in(name, &folder),
                resolved: Some(listing.path.clone()),
                branch,
            });
            rows.push(Row::Blank);
        }
        let wanted = part.to_lowercase();
        let hidden_ok = part.starts_with('.');
        let visible = |n: &str| hidden_ok || !n.starts_with('.');
        let mut kids: Vec<_> = listing
            .entries
            .iter()
            .filter(|e| visible(&e.name) && e.name.to_lowercase().starts_with(&wanted))
            .map(|e| (e, Vec::new()))
            .collect();
        if kids.is_empty() && !part.is_empty() {
            kids = listing
                .entries
                .iter()
                .filter(|e| visible(&e.name))
                .filter_map(|e| fuzzy_indices(&part, &e.name).map(|at| (e, at)))
                .collect();
        }
        if kids.is_empty() {
            rows.push(Row::Note(if part.is_empty() { "no subfolders" } else { "no folder matches" }));
            return rows;
        }
        let head = ends_in(name, &folder);
        for (entry, matched) in kids {
            let full_path = format!("{folder}{}/", entry.name);
            let (open, needs) = Self::open_at(projects, machine, &full_path);
            rows.push(Row::Folder {
                machine,
                shown: format!("{head}{}/", entry.name),
                typed: head.chars().count() + part.chars().count(),
                matched,
                branch: entry.git.clone(),
                has_children: entry.has_children,
                open,
                needs,
            });
        }
        rows
    }

    /// The first row Enter or the arrows may land on.
    pub fn first(rows: &[Row]) -> Option<usize> {
        rows.iter().position(Row::selectable)
    }

    /// The row that is selected, if the selection is on a list row at all.
    pub fn selected<'a>(&self, rows: &'a [Row]) -> Option<(usize, &'a Row)> {
        if self.chip.is_some() {
            return None;
        }
        let i = match self.sel {
            Some(i) if rows.get(i).is_some_and(Row::selectable) => i,
            _ => Self::first(rows)?,
        };
        Some((i, &rows[i]))
    }

    /// Move through the list rows only.
    pub fn step(&mut self, rows: &[Row], down: bool) {
        self.chip = None;
        let Some((now, _)) = self.selected(rows).or_else(|| Self::first(rows).map(|i| (i, &rows[i]))) else {
            return;
        };
        let order: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].selectable()).collect();
        let at = order.iter().position(|&i| i == now).unwrap_or(0);
        let next = if down { (at + 1).min(order.len() - 1) } else { at.saturating_sub(1) };
        self.sel = order.get(next).copied();
    }

    /// What Enter does on the selected row.
    pub fn enter(&self, rows: &[Row], chips: &[Chip]) -> Go {
        if let Some(i) = self.chip {
            let Some(chip) = chips.get(i) else { return Go::Nothing };
            return match chip.state {
                ChipState::Online => Go::Fill(chip_root(chip)),
                ChipState::Connecting => Go::Nothing,
                ChipState::Offline => Go::Reconnect(chip.id),
            };
        }
        match self.selected(rows) {
            Some((_, Row::StartHere { machine, resolved, shown, .. })) => Go::Start {
                machine: *machine,
                path: resolved.clone().unwrap_or_else(|| strip_host(shown)),
            },
            Some((_, Row::Project { machine, shown, .. })) => Go::Start {
                machine: *machine,
                path: strip_host(shown),
            },
            Some((_, Row::Folder { machine, shown, branch, has_children, .. })) => {
                if branch.is_some() || !has_children {
                    Go::Start { machine: *machine, path: strip_host(shown) }
                } else {
                    Go::Fill(shown.clone())
                }
            }
            _ => Go::Nothing,
        }
    }

    /// What Tab or → puts in the query: the selected row's path.
    pub fn tab(&self, rows: &[Row]) -> Option<String> {
        self.selected(rows).and_then(|(_, row)| match row {
            Row::StartHere { .. } => None,
            other => other.fill(),
        })
    }

    /// Move between the machine chips.
    pub fn step_chip(&mut self, chips: &[Chip], right: bool) {
        if chips.is_empty() {
            return;
        }
        let last = chips.len() - 1;
        self.chip = Some(match (self.chip, right) {
            (None, true) => 0,
            (None, false) => last,
            (Some(i), true) => (i + 1).min(last),
            (Some(i), false) => i.saturating_sub(1),
        });
    }

    /// A listing arrived for a question this popup asked.
    pub fn arrived(&mut self, machine: MachineId, tag: u64, listing: FolderListing) -> bool {
        let Some((asked_of, folder)) = self.asked.remove(&tag) else {
            return false;
        };
        if asked_of != machine {
            return false;
        }
        self.cache.insert((machine, folder), Listing::Ready(listing));
        true
    }

    /// Note that `folder` on `machine` is being asked for, and give the tag to ask with.
    pub fn ask(&mut self, machine: MachineId, folder: String) -> u64 {
        self.next_tag += 1;
        let tag = self.next_tag;
        self.cache.insert((machine, folder.clone()), Listing::Loading);
        self.asked.insert(tag, (machine, folder));
        tag
    }
}

fn chip_root(chip: &Chip) -> String {
    if chip.name.is_empty() {
        ROOT.to_string()
    } else {
        format!("{}:{ROOT}", chip.name)
    }
}

/// `host:path` without the host.
fn strip_host(shown: &str) -> String {
    match host_of(shown) {
        Some((_, rest)) if !shown.starts_with(['~', '/', '.']) => rest.to_string(),
        _ => shown.to_string(),
    }
}

/// Which machine a remembered path was on, and the path.
fn recent_target(path: &str, chips: &[Chip]) -> (MachineId, String) {
    match scope_of(path, chips) {
        Scope::Path { machine, path } => (machine, path),
        Scope::Plain => (MachineId::LOCAL, path.to_string()),
    }
}

/// The branch of the folder the query is in, when its parent's listing knows it.
fn folder_branch(ns: &NewSession, machine: MachineId, folder: &str) -> Option<String> {
    let trimmed = folder.trim_end_matches('/');
    let (parent, name) = split_folder(trimmed);
    ns.ready(machine, &parent)?
        .entries
        .iter()
        .find(|e| e.name == name)
        .and_then(|e| e.git.clone())
}

fn row_key(row: &Row) -> String {
    match row {
        Row::Folder { shown, .. } => shown.to_lowercase(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::FolderEntry;

    fn chips() -> Vec<Chip> {
        vec![
            Chip { id: MachineId::LOCAL, name: String::new(), state: ChipState::Online },
            Chip { id: MachineId(1), name: "kingdom".into(), state: ChipState::Online },
            Chip { id: MachineId(2), name: "pi-garage".into(), state: ChipState::Offline },
        ]
    }

    fn listing(path: &str, names: &[(&str, Option<&str>, bool)]) -> FolderListing {
        FolderListing {
            path: path.into(),
            exists: true,
            entries: names
                .iter()
                .map(|(n, git, kids)| FolderEntry { name: (*n).into(), git: git.map(str::to_string), has_children: *kids })
                .collect(),
            truncated: false,
        }
    }

    fn with(ns: &mut NewSession, machine: MachineId, folder: &str, l: FolderListing) {
        ns.cache.insert((machine, folder.into()), Listing::Ready(l));
    }

    #[test]
    fn the_query_means_a_search_a_local_path_or_a_machine_path() {
        let c = chips();
        assert_eq!(scope_of("mai", &c), Scope::Plain);
        assert_eq!(scope_of("~/code", &c), Scope::Path { machine: MachineId::LOCAL, path: "~/code".into() });
        assert_eq!(scope_of("/srv/x", &c), Scope::Path { machine: MachineId::LOCAL, path: "/srv/x".into() });
        assert_eq!(scope_of("kingdom:", &c), Scope::Path { machine: MachineId(1), path: "~/".into() });
        assert_eq!(scope_of("KINGDOM:~/mai", &c), Scope::Path { machine: MachineId(1), path: "~/mai".into() });
        // A host that is not a machine is a search; so is this computer's own (unnamed) chip.
        assert_eq!(scope_of("nowhere:~/x", &c), Scope::Plain);
        assert_eq!(scope_of("c:\\x", &c), Scope::Plain);
    }

    #[test]
    fn a_path_splits_into_the_folder_and_what_is_being_typed() {
        assert_eq!(split_folder("~/code/mai"), ("~/code/".into(), "mai".into()));
        assert_eq!(split_folder("~/code/"), ("~/code/".into(), String::new()));
        assert_eq!(split_folder("~"), ("~/".into(), "~".into()));
    }

    #[test]
    fn fuzzy_matches_in_order_and_reports_where() {
        // The first `t` that fits is taken.
        assert_eq!(fuzzy_indices("mty", "maitty"), Some(vec![0, 3, 5]));
        assert_eq!(fuzzy_indices("MAI", "maitty"), Some(vec![0, 1, 2]));
        assert_eq!(fuzzy_indices("ytm", "maitty"), None);
        assert_eq!(fuzzy_indices("", "x"), Some(vec![]));
    }

    #[test]
    fn the_ghost_is_the_rest_of_the_selected_path() {
        assert_eq!(ghost("kingdom:~/mai", "kingdom:~/maitty/").as_deref(), Some("tty/"));
        assert_eq!(ghost("KINGDOM:~/MAI", "kingdom:~/maitty/").as_deref(), Some("tty/"));
        assert_eq!(ghost("kingdom:~/maitty/", "kingdom:~/maitty/"), None);
        assert_eq!(ghost("zz", "kingdom:~/maitty/"), None);
        assert_eq!(ghost("", "x/"), None);
    }

    #[test]
    fn ctrl_backspace_goes_back_one_segment() {
        assert_eq!(delete_segment("kingdom:~/code/maitty/"), "kingdom:~/code/");
        assert_eq!(delete_segment("~/code/maitty"), "~/code/");
        assert_eq!(delete_segment("kingdom:~/"), "kingdom:");
        assert_eq!(delete_segment("kingdom:"), "");
        assert_eq!(delete_segment("abc"), "");
    }

    #[test]
    fn an_empty_query_lists_open_projects_then_recent_paths() {
        let projects = vec![
            ProjectRow { machine: MachineId::LOCAL, path: "/home/me/code/shop".into(), branch: Some("main".into()), sessions: 2, needs: true },
            ProjectRow { machine: MachineId(1), path: "/home/dev/maitty".into(), branch: Some("fixes".into()), sessions: 1, needs: false },
        ];
        let rows = NewSession::new("").rows(&chips(), &projects, &["kingdom:~/old/".into()]);
        assert!(matches!(&rows[0], Row::Section { label: "OPEN", .. }));
        assert!(matches!(&rows[1], Row::Project { shown, needs: true, .. } if shown == "/home/me/code/shop/"));
        assert!(matches!(&rows[2], Row::Project { shown, .. } if shown == "kingdom:/home/dev/maitty/"));
        assert!(rows.iter().any(|r| matches!(r, Row::Section { label: "RECENT", .. })));
        assert!(rows.iter().any(|r| matches!(r, Row::Folder { shown, machine, .. } if shown == "kingdom:~/old/" && *machine == MachineId(1))));
    }

    #[test]
    fn a_search_finds_projects_and_folders_on_every_online_machine_without_repeating_a_project() {
        let projects = vec![ProjectRow { machine: MachineId(1), path: "/home/dev/maitty".into(), branch: None, sessions: 1, needs: false }];
        let mut ns = NewSession::new("mai");
        with(&mut ns, MachineId::LOCAL, "~/", listing("/home/me", &[("mail", None, true), ("zebra", None, false)]));
        with(&mut ns, MachineId(1), "/home/dev/", listing("/home/dev", &[("maitty", Some("fixes"), true), ("maitty-worktrees", None, true)]));
        with(&mut ns, MachineId(2), "~/", listing("/home/pi", &[("maintenance", None, false)]));

        let rows = ns.rows(&chips(), &projects, &[]);

        assert!(rows.iter().any(|r| matches!(r, Row::Project { shown, .. } if shown == "kingdom:/home/dev/maitty/")));
        let folders: Vec<_> = rows.iter().filter_map(|r| match r { Row::Folder { shown, .. } => Some(shown.as_str()), _ => None }).collect();
        // The project is not listed twice; the offline machine's folders are not searched.
        assert_eq!(folders, ["~/mail/", "kingdom:/home/dev/maitty-worktrees/"]);
    }

    #[test]
    fn a_search_lists_at_most_nine_folders_prefix_matches_and_shallow_paths_first() {
        let mut ns = NewSession::new("a");
        let names: Vec<(String, Option<&str>, bool)> = (0..20).map(|n| (format!("a{n:02}"), None, false)).collect();
        let refs: Vec<(&str, Option<&str>, bool)> = names.iter().map(|(n, g, k)| (n.as_str(), *g, *k)).collect();
        with(&mut ns, MachineId::LOCAL, "~/", listing("/home/me", &refs));
        with(&mut ns, MachineId::LOCAL, "~/deep/er/", listing("/home/me/deep/er", &[("a-deep", None, false)]));
        let rows = ns.rows(&chips(), &[], &[]);
        let folders: Vec<_> = rows.iter().filter_map(|r| match r { Row::Folder { shown, .. } => Some(shown.as_str()), _ => None }).collect();
        assert_eq!(folders.len(), FOLDERS_SHOWN);
        assert_eq!(folders[0], "~/a00/");
        assert!(!folders.contains(&"~/deep/er/a-deep/"));
    }

    #[test]
    fn path_mode_starts_with_start_here_then_the_subfolders_and_hides_dot_folders() {
        let mut ns = NewSession::new("kingdom:~/");
        with(&mut ns, MachineId(1), "~/", listing("/home/dev", &[(".cache", None, true), ("maitty", Some("fixes"), true), ("notes", None, false)]));
        let rows = ns.rows(&chips(), &[], &[]);
        assert!(matches!(&rows[0], Row::StartHere { shown, resolved: Some(r), .. } if shown == "kingdom:~/" && r == "/home/dev"));
        assert_eq!(rows[1], Row::Blank);
        let shown: Vec<_> = rows.iter().filter_map(|r| match r { Row::Folder { shown, .. } => Some(shown.as_str()), _ => None }).collect();
        assert_eq!(shown, ["kingdom:~/maitty/", "kingdom:~/notes/"]);
    }

    #[test]
    fn path_mode_narrows_by_prefix_then_falls_back_to_a_fuzzy_match() {
        let mut ns = NewSession::new("~/mai");
        with(&mut ns, MachineId::LOCAL, "~/", listing("/home/me", &[("maitty", None, true), ("mail", None, true), ("shop", None, true)]));
        let names = |ns: &NewSession| ns.rows(&chips(), &[], &[]).iter().filter_map(|r| match r { Row::Folder { shown, .. } => Some(shown.clone()), _ => None }).collect::<Vec<_>>();
        assert_eq!(names(&ns), ["~/maitty/", "~/mail/"]);
        ns.q = "~/sp".into();
        assert_eq!(names(&ns), ["~/shop/"]);
        ns.q = "~/zzz".into();
        let rows = ns.rows(&chips(), &[], &[]);
        assert_eq!(rows, vec![Row::Note("no folder matches")]);
    }

    #[test]
    fn a_folder_that_is_not_there_says_so() {
        let mut ns = NewSession::new("~/nope/");
        with(&mut ns, MachineId::LOCAL, "~/nope/", FolderListing::missing("~/nope/"));
        assert_eq!(ns.rows(&chips(), &[], &[]), vec![Row::Note("nothing here matches · ⌫ to go back")]);
    }

    #[test]
    fn enter_starts_on_start_here_a_project_and_a_repo_or_leaf_but_opens_a_plain_folder() {
        let mut ns = NewSession::new("~/");
        with(&mut ns, MachineId::LOCAL, "~/", listing("/home/me", &[("repo", Some("main"), true), ("leaf", None, false), ("dir", None, true)]));
        let c = chips();
        let rows = ns.rows(&c, &[], &[]);
        // The first selectable row is start here.
        assert_eq!(ns.enter(&rows, &c), Go::Start { machine: MachineId::LOCAL, path: "/home/me".into() });
        let pick = |ns: &mut NewSession, name: &str| {
            ns.sel = rows.iter().position(|r| matches!(r, Row::Folder { shown, .. } if shown == name));
            ns.enter(&rows, &c)
        };
        assert_eq!(pick(&mut ns, "~/repo/"), Go::Start { machine: MachineId::LOCAL, path: "~/repo/".into() });
        assert_eq!(pick(&mut ns, "~/leaf/"), Go::Start { machine: MachineId::LOCAL, path: "~/leaf/".into() });
        assert_eq!(pick(&mut ns, "~/dir/"), Go::Fill("~/dir/".into()));
    }

    #[test]
    fn enter_on_a_machine_chip_browses_an_online_one_and_reconnects_an_offline_one() {
        let c = chips();
        let mut ns = NewSession::new("");
        ns.chip = Some(1);
        assert_eq!(ns.enter(&[], &c), Go::Fill("kingdom:~/".into()));
        ns.chip = Some(0);
        assert_eq!(ns.enter(&[], &c), Go::Fill("~/".into()));
        ns.chip = Some(2);
        assert_eq!(ns.enter(&[], &c), Go::Reconnect(MachineId(2)));
    }

    #[test]
    fn the_arrows_skip_labels_and_blanks_and_never_land_on_the_chips() {
        let rows = vec![
            Row::Section { label: "OPEN", note: "2".into() },
            Row::Project { machine: MachineId::LOCAL, shown: "/a/".into(), branch: None, sessions: 1, needs: false },
            Row::Blank,
            Row::Section { label: "FOLDERS", note: String::new() },
            Row::Folder { machine: MachineId::LOCAL, shown: "~/b/".into(), typed: 0, matched: vec![], branch: None, has_children: false, open: 0, needs: false },
        ];
        let mut ns = NewSession::new("");
        assert_eq!(ns.selected(&rows).map(|(i, _)| i), Some(1));
        ns.step(&rows, true);
        assert_eq!(ns.sel, Some(4));
        ns.step(&rows, true);
        assert_eq!(ns.sel, Some(4));
        ns.step(&rows, false);
        assert_eq!(ns.sel, Some(1));
        ns.step_chip(&chips(), true);
        assert_eq!(ns.chip, Some(0));
        assert!(ns.selected(&rows).is_none());
        ns.step(&rows, true);
        assert_eq!(ns.chip, None);
    }

    #[test]
    fn a_listing_is_taken_only_for_a_question_this_popup_asked_of_that_machine() {
        let mut ns = NewSession::default();
        let tag = ns.ask(MachineId(1), "~/".into());
        assert_eq!(ns.cache.get(&(MachineId(1), "~/".into())), Some(&Listing::Loading));
        assert!(!ns.arrived(MachineId(2), tag, listing("/x", &[])));
        let tag = ns.ask(MachineId(1), "~/".into());
        assert!(!ns.arrived(MachineId(1), tag + 99, listing("/x", &[])));
        assert!(ns.arrived(MachineId(1), tag, listing("/x", &[])));
        assert!(matches!(ns.cache.get(&(MachineId(1), "~/".into())), Some(Listing::Ready(_))));
    }

    #[test]
    fn the_popup_asks_about_each_online_machines_root_and_the_folders_its_projects_are_in() {
        let projects = vec![
            ProjectRow { machine: MachineId::LOCAL, path: "/home/me/code/shop".into(), branch: None, sessions: 1, needs: false },
            ProjectRow { machine: MachineId(2), path: "/home/pi/x".into(), branch: None, sessions: 1, needs: false },
        ];
        let asked = NewSession::index_folders(&chips(), &projects);
        assert!(asked.contains(&(MachineId::LOCAL, "~/".into())));
        assert!(asked.contains(&(MachineId(1), "~/".into())));
        assert!(asked.contains(&(MachineId::LOCAL, "/home/me/code/".into())));
        // The offline machine is not asked.
        assert!(asked.iter().all(|(m, _)| *m != MachineId(2)));
    }
}
