//! The New session dialog's model, with no drawing: two steps, the project and then the machine,
//! and a shell is started in the folder on that machine. Folder completion asks the machine's
//! daemon through `Query::ListFolders`; the answers arrive as messages and land in `cache`.
//! The view and the keys are in `new_session_ui.rs`.

use super::*;
use crate::protocol::FolderListing;
use std::collections::BTreeMap;

/// How many remembered folders are kept, newest first.
pub(in crate::client) const RECENT_KEEP: usize = 8;
/// The folder every machine's browse starts from.
pub(in crate::client) const ROOT: &str = "~/";
/// The most child folders the list shows below the typed one.
const CHILDREN_SHOWN: usize = 40;

/// A machine the dialog can start a session on.
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

impl ChipState {
    /// Only a machine that is up can be picked.
    pub fn pickable(self) -> bool {
        self == ChipState::Online
    }
}

/// The two steps; the active one is shown in reverse video in the header.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub(in crate::client) enum Step {
    #[default]
    Project,
    Machine,
}

/// What a folder's listing is, once asked for.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) enum Listing {
    Loading,
    Ready(FolderListing),
}

/// What is known about a folder on a machine.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::client) enum FolderState {
    /// Nothing asked yet, or the question is out.
    Pending,
    /// What is typed begins a folder's name but is not one yet: the completion offers the rest.
    Incomplete,
    Exists,
    Missing,
}

/// One line of the project list.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) enum Row {
    /// `Browse...`: opens the folder picker.
    Browse,
    /// A folder remembered or open: where it was and which machine.
    Recent { machine: MachineId, path: String },
    /// The folder the query names, first in the list while a path is typed.
    Typed { path: String, state: FolderState },
    /// A folder inside the one being typed or browsed.
    Child { name: String, path: String, git: Option<String>, has_children: bool },
    /// The folder the picker is in: choose it.
    UseHere { path: String },
    /// The picker's way up.
    Up { to: String },
}

/// What Enter does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(in crate::client) enum Go {
    /// Open the folder picker at this folder.
    Browse(String),
    /// Move the picker into this folder.
    Into(String),
    /// The project is chosen: on this machine at this absolute path.
    Choose { machine: MachineId, path: String },
    /// Create the shell.
    Start { machine: MachineId, path: String },
    Nothing,
}

/// The dialog's state.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) struct NewSession {
    pub step: Step,
    pub q: String,
    /// The selected row of the current step's list.
    pub sel: usize,
    /// The machine folders are completed on, and the one step 2 has selected.
    pub machine: MachineId,
    /// The folder the picker is in (with its trailing `/`), when it is open.
    pub browsing: Option<String>,
    /// The project step 1 chose: its machine and absolute path.
    pub project: Option<(MachineId, String)>,
    /// Listings asked for or received, by machine and folder as typed.
    pub cache: BTreeMap<(MachineId, String), Listing>,
    /// Questions out: tag to the folder it was about.
    pub asked: BTreeMap<u64, (MachineId, String)>,
    pub next_tag: u64,
}

/// Whether `q` names a folder (a path) rather than searching the remembered ones.
pub(in crate::client) fn is_path(q: &str) -> bool {
    q.starts_with(['~', '/', '.']) || q.chars().nth(1) == Some(':') && q.chars().nth(2).is_some_and(|c| c == '/' || c == '\\')
}

/// The folder a path is inside (up to its last `/`) and the part still being typed after it.
pub(in crate::client) fn split_folder(path: &str) -> (String, String) {
    if path == "~" {
        return (ROOT.to_string(), String::new());
    }
    match path.rfind('/') {
        Some(i) => (path[..=i].to_string(), path[i + 1..].to_string()),
        None => (ROOT.to_string(), path.to_string()),
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

/// A folder's last segment, whichever way the platform writes its separators.
pub(in crate::client) fn folder_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|n| !n.is_empty())
        .unwrap_or(path)
}

/// `a/b/c/` becomes `a/b/`; the root stays.
fn parent_of(folder: &str) -> String {
    let trimmed = folder.trim_end_matches('/');
    match trimmed.rfind('/') {
        Some(0) | None => {
            if folder.starts_with('~') { ROOT.to_string() } else { "/".to_string() }
        }
        Some(i) => trimmed[..=i].to_string(),
    }
}

/// A listing's folder joined with a name, as an absolute path.
fn join(listing: &FolderListing, name: &str) -> String {
    format!("{}/{name}", listing.path.trim_end_matches('/'))
}

impl NewSession {
    pub fn new(machine: MachineId) -> NewSession {
        NewSession {
            step: Step::Project,
            q: String::new(),
            sel: 0,
            machine,
            browsing: None,
            project: None,
            cache: BTreeMap::new(),
            asked: BTreeMap::new(),
            next_tag: 0,
        }
    }

    fn ready(&self, machine: MachineId, folder: &str) -> Option<&FolderListing> {
        match self.cache.get(&(machine, folder.to_string())) {
            Some(Listing::Ready(listing)) => Some(listing),
            _ => None,
        }
    }

    /// The folder whose listing the dialog needs now and has not asked for.
    pub fn wanted(&self) -> Option<(MachineId, String)> {
        let folder = match (&self.browsing, self.step) {
            (Some(dir), _) => dir.clone(),
            (None, Step::Project) if is_path(&self.q) => split_folder(&self.q).0,
            (None, Step::Machine) => {
                let (_, path) = self.project.as_ref()?;
                format!("{}/", path.trim_end_matches('/'))
            }
            _ => return None,
        };
        let machine = self.machine;
        (!self.cache.contains_key(&(machine, folder.clone()))).then_some((machine, folder))
    }

    /// Note that `folder` on `machine` is being asked for, and give the tag to ask with.
    pub fn ask(&mut self, machine: MachineId, folder: String) -> u64 {
        self.next_tag += 1;
        let tag = self.next_tag;
        self.cache.insert((machine, folder.clone()), Listing::Loading);
        self.asked.insert(tag, (machine, folder));
        tag
    }

    /// An answer came in; false when it answers nothing this dialog asked.
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

    /// An error came back while folders were being asked for: stop waiting for them.
    pub fn give_up(&mut self) {
        let waiting: Vec<_> = std::mem::take(&mut self.asked).into_values().collect();
        for key in waiting {
            let missing = FolderListing::missing(&key.1);
            self.cache.insert(key, Listing::Ready(missing));
        }
    }

    /// The folder the query names, once its parent has been read.
    fn typed_target(&self) -> Option<(String, FolderState)> {
        let (folder, partial) = split_folder(&self.q);
        let listing = self.ready(self.machine, &folder)?;
        if partial.is_empty() {
            let state = if listing.exists { FolderState::Exists } else { FolderState::Missing };
            return Some((listing.path.clone(), state));
        }
        let found = listing.exists && listing.entries.iter().any(|e| e.name == partial);
        let want = partial.to_lowercase();
        let begins = listing.exists && listing.entries.iter().any(|e| e.name.to_lowercase().starts_with(&want));
        let path = join(listing, &partial);
        let state = match (found, begins) {
            (true, _) => FolderState::Exists,
            (false, true) => FolderState::Incomplete,
            (false, false) => FolderState::Missing,
        };
        Some((path, state))
    }

    /// Whether the typed path is a folder: pending until its parent has been read.
    pub fn typed_state(&self) -> FolderState {
        if !is_path(&self.q) || self.step != Step::Project || self.browsing.is_some() {
            return FolderState::Exists;
        }
        self.typed_target().map_or(FolderState::Pending, |(_, s)| s)
    }

    /// What the chosen project is on the chosen machine, for step 2.
    pub fn project_state(&self) -> FolderState {
        let Some((_, path)) = &self.project else { return FolderState::Exists };
        match self.ready(self.machine, &format!("{}/", path.trim_end_matches('/'))) {
            Some(l) if l.exists => FolderState::Exists,
            Some(_) => FolderState::Missing,
            None => FolderState::Pending,
        }
    }

    /// The rest of the first folder that begins with what is typed, for the dim completion.
    pub fn ghost(&self) -> Option<String> {
        if self.step != Step::Project || self.browsing.is_some() || !is_path(&self.q) {
            return None;
        }
        let (folder, partial) = split_folder(&self.q);
        if partial.is_empty() {
            return None;
        }
        let listing = self.ready(self.machine, &folder)?;
        let want = partial.to_lowercase();
        listing
            .entries
            .iter()
            .filter(|e| e.name.to_lowercase().starts_with(&want))
            .map(|e| e.name.as_str())
            .min()
            .filter(|n| n.chars().count() > partial.chars().count() || !self.q.ends_with('/'))
            .map(|n| format!("{}/", n.chars().skip(partial.chars().count()).collect::<String>()))
    }

    /// The project list for the current query. `recent` is every folder to offer, newest first.
    pub fn rows(&self, recent: &[(MachineId, String)]) -> Vec<Row> {
        if let Some(dir) = &self.browsing {
            let mut rows = vec![];
            if let Some(listing) = self.ready(self.machine, dir) {
                rows.push(Row::UseHere { path: listing.path.clone() });
                if dir.trim_end_matches('/') != "~" && listing.path != "/" {
                    rows.push(Row::Up { to: parent_of(dir) });
                }
                rows.extend(listing.entries.iter().take(CHILDREN_SHOWN).map(|e| Row::Child {
                    name: e.name.clone(),
                    path: format!("{dir}{}/", e.name),
                    git: e.git.clone(),
                    has_children: e.has_children,
                }));
            }
            return rows;
        }
        if is_path(&self.q) {
            let (folder, partial) = split_folder(&self.q);
            let mut rows = Vec::new();
            match self.typed_target() {
                Some((path, state)) => rows.push(Row::Typed { path, state }),
                None => rows.push(Row::Typed { path: self.q.clone(), state: FolderState::Pending }),
            }
            if let Some(listing) = self.ready(self.machine, &folder) {
                let want = partial.to_lowercase();
                rows.extend(
                    listing
                        .entries
                        .iter()
                        .filter(|e| e.name.to_lowercase().starts_with(&want) && e.name != partial)
                        .take(CHILDREN_SHOWN)
                        .map(|e| Row::Child {
                            name: e.name.clone(),
                            path: format!("{folder}{}/", e.name),
                            git: e.git.clone(),
                            has_children: e.has_children,
                        }),
                );
            }
            return rows;
        }
        let q = self.q.trim();
        let mut rows = vec![Row::Browse];
        rows.extend(
            recent
                .iter()
                .filter(|(_, p)| q.is_empty() || fuzzy_indices(q, folder_name(p)).is_some() || fuzzy_indices(q, p).is_some())
                .map(|(machine, path)| Row::Recent { machine: *machine, path: path.clone() }),
        );
        rows
    }

    /// Move the selection by `delta` rows, staying inside the list.
    pub fn step_sel(&mut self, delta: isize, len: usize) {
        if len == 0 {
            self.sel = 0;
            return;
        }
        self.sel = self.sel.saturating_add_signed(delta).min(len - 1);
    }

    /// Move the machine selection to the next one that can be picked.
    pub fn step_machine(&mut self, delta: isize, chips: &[Chip]) {
        let at = chips.iter().position(|c| c.id == self.machine).unwrap_or(0) as isize;
        let mut i = at + delta;
        while (0..chips.len() as isize).contains(&i) {
            if chips[i as usize].state.pickable() {
                self.machine = chips[i as usize].id;
                return;
            }
            i += delta;
        }
    }

    /// What Enter does now.
    pub fn enter(&self, rows: &[Row], chips: &[Chip]) -> Go {
        match self.step {
            Step::Machine => {
                let Some((_, path)) = &self.project else { return Go::Nothing };
                let pickable = chips.iter().any(|c| c.id == self.machine && c.state.pickable());
                if pickable && self.project_state() == FolderState::Exists {
                    Go::Start { machine: self.machine, path: path.clone() }
                } else {
                    Go::Nothing
                }
            }
            Step::Project => match rows.get(self.sel) {
                Some(Row::Browse) => Go::Browse(ROOT.to_string()),
                Some(Row::Recent { machine, path }) => Go::Choose { machine: *machine, path: path.clone() },
                Some(Row::Typed { path, state: FolderState::Exists }) => {
                    Go::Choose { machine: self.machine, path: path.clone() }
                }
                Some(Row::Child { path, .. }) if self.browsing.is_some() => Go::Into(path.clone()),
                Some(Row::Child { name, path, .. }) => {
                    let (folder, _) = split_folder(path.trim_end_matches('/'));
                    match self.ready(self.machine, &folder) {
                        Some(l) => Go::Choose { machine: self.machine, path: join(l, name) },
                        None => Go::Nothing,
                    }
                }
                Some(Row::UseHere { path }) => Go::Choose { machine: self.machine, path: path.clone() },
                Some(Row::Up { to }) => Go::Into(to.clone()),
                Some(Row::Typed { .. }) | None => Go::Nothing,
            },
        }
    }

    /// The query with the ghost accepted, when there is one.
    pub fn accepted(&self) -> Option<String> {
        self.ghost().map(|g| format!("{}{g}", self.q))
    }
}

/// Which machine a remembered path was on, and the path: `build:/home/me/x` or a plain path.
pub(in crate::client) fn parse_recent(entry: &str, chips: &[Chip]) -> (MachineId, String) {
    if let Some((host, rest)) = entry.split_once(':')
        && !host.is_empty()
        && let Some(chip) = chips.iter().find(|c| !c.name.is_empty() && c.name.eq_ignore_ascii_case(host))
    {
        return (chip.id, rest.to_string());
    }
    (MachineId::LOCAL, entry.to_string())
}

/// A remembered path as it is stored: `host:path` on another machine.
pub(in crate::client) fn store_recent(chips: &[Chip], machine: MachineId, path: &str) -> String {
    match chips.iter().find(|c| c.id == machine) {
        Some(chip) if !chip.name.is_empty() => format!("{}:{path}", chip.name),
        _ => path.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::protocol::FolderEntry;

    fn listing(path: &str, names: &[&str]) -> FolderListing {
        FolderListing {
            path: path.into(),
            exists: true,
            entries: names
                .iter()
                .map(|n| FolderEntry { name: (*n).into(), git: None, has_children: true })
                .collect(),
            truncated: false,
        }
    }

    fn chips() -> Vec<Chip> {
        vec![
            Chip { id: MachineId::LOCAL, name: String::new(), state: ChipState::Online },
            Chip { id: MachineId(1), name: "build".into(), state: ChipState::Online },
            Chip { id: MachineId(2), name: "gpu-01".into(), state: ChipState::Offline },
        ]
    }

    fn with_home(q: &str) -> NewSession {
        let mut ns = NewSession::new(MachineId::LOCAL);
        ns.q = q.into();
        let tag = ns.ask(MachineId::LOCAL, ROOT.into());
        ns.arrived(MachineId::LOCAL, tag, listing("/home/me", &["code", "docs", "music"]));
        ns
    }

    #[test]
    fn a_plain_query_lists_browse_first_then_the_matching_recents() {
        let mut ns = NewSession::new(MachineId::LOCAL);
        let recent = vec![(MachineId::LOCAL, "/home/me/code/maitty".to_string()), (MachineId(1), "/srv/api".to_string())];
        assert_eq!(ns.rows(&recent).len(), 3);
        assert_eq!(ns.rows(&recent)[0], Row::Browse);
        ns.q = "mty".into();
        let rows = ns.rows(&recent);
        assert_eq!(rows.len(), 2, "mty is an in-order subsequence of maitty only");
        assert!(matches!(&rows[1], Row::Recent { path, .. } if path.ends_with("maitty")));
    }

    #[test]
    fn a_typed_path_completes_from_the_listing_and_tab_accepts_it() {
        let ns = with_home("~/co");
        assert_eq!(ns.ghost().as_deref(), Some("de/"));
        assert_eq!(ns.accepted().as_deref(), Some("~/code/"));
    }

    #[test]
    fn a_missing_folder_blocks_enter_and_an_existing_one_chooses_it() {
        let ns = with_home("~/nope");
        assert_eq!(ns.typed_state(), FolderState::Missing);
        assert_eq!(ns.enter(&ns.rows(&[]), &chips()), Go::Nothing);
        let ns = with_home("~/docs");
        assert_eq!(ns.typed_state(), FolderState::Exists);
        assert_eq!(
            ns.enter(&ns.rows(&[]), &chips()),
            Go::Choose { machine: MachineId::LOCAL, path: "/home/me/docs".into() }
        );
    }

    #[test]
    fn a_path_whose_folder_is_not_read_yet_waits() {
        let mut ns = NewSession::new(MachineId::LOCAL);
        ns.q = "~/co".into();
        assert_eq!(ns.wanted(), Some((MachineId::LOCAL, ROOT.to_string())));
        ns.ask(MachineId::LOCAL, ROOT.into());
        assert_eq!(ns.wanted(), None);
        assert_eq!(ns.typed_state(), FolderState::Pending);
        assert_eq!(ns.enter(&ns.rows(&[]), &chips()), Go::Nothing);
    }

    #[test]
    fn the_machine_step_skips_offline_machines() {
        let mut ns = NewSession::new(MachineId(1));
        ns.step_machine(1, &chips());
        assert_eq!(ns.machine, MachineId(1), "gpu-01 is offline and cannot be picked");
        ns.step_machine(-1, &chips());
        assert_eq!(ns.machine, MachineId::LOCAL);
    }

    #[test]
    fn enter_on_the_machine_step_starts_a_shell_only_where_the_folder_exists() {
        let mut ns = NewSession::new(MachineId(1));
        ns.step = Step::Machine;
        ns.project = Some((MachineId::LOCAL, "/home/me/code".into()));
        assert_eq!(ns.enter(&[], &chips()), Go::Nothing, "not read yet");
        let tag = ns.ask(MachineId(1), "/home/me/code/".into());
        ns.arrived(MachineId(1), tag, FolderListing::missing("/home/me/code"));
        assert_eq!(ns.project_state(), FolderState::Missing);
        assert_eq!(ns.enter(&[], &chips()), Go::Nothing);
        ns.cache.insert((MachineId(1), "/home/me/code/".into()), Listing::Ready(listing("/home/me/code", &[])));
        assert_eq!(ns.enter(&[], &chips()), Go::Start { machine: MachineId(1), path: "/home/me/code".into() });
    }

    #[test]
    fn the_picker_lists_use_here_then_up_then_the_folders() {
        let mut ns = with_home("");
        ns.browsing = Some(ROOT.into());
        let rows = ns.rows(&[]);
        assert!(matches!(rows[0], Row::UseHere { .. }));
        assert!(matches!(&rows[1], Row::Child { name, .. } if name == "code"), "home has no way up");
        ns.sel = 1;
        assert_eq!(ns.enter(&rows, &chips()), Go::Into("~/code/".into()));
    }

    #[test]
    fn remembered_paths_name_their_machine() {
        let c = chips();
        assert_eq!(parse_recent("build:/srv/api", &c), (MachineId(1), "/srv/api".into()));
        assert_eq!(parse_recent("/home/me/x", &c), (MachineId::LOCAL, "/home/me/x".into()));
        assert_eq!(store_recent(&c, MachineId(1), "/srv/api"), "build:/srv/api");
    }
}
