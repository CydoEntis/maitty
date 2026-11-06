//! Starting a session without the popup: `n` reads what the sidebar has selected and starts
//! right there (`quick_target`), and every way of starting one, the popup's included, ends in
//! `create_session`. Docs: UI.md, "New Session popup".

use super::session_popup::{RECENT_KEEP, typed};
use super::*;

/// What `n` does, decided from the sidebar's selected row.
#[derive(Debug, Clone, PartialEq)]
pub(in crate::client) enum QuickTarget {
    /// Start in this project's own checkout on that machine; `branch` is what it has out.
    Start {
        machine: MachineId,
        project: PathBuf,
        branch: Option<String>,
    },
    /// There is no project to reuse: pick a folder on that machine.
    Scoped(MachineId),
    /// The machine is down: dial it again.
    Reconnect(MachineId),
}

/// The sidebar row `quick_target` reads.
pub(in crate::client) enum Selected<'a> {
    /// A project header, or a session (or tab) of that project.
    Project(&'a Proj),
    /// A machine row. `down`: its connection is not up.
    Machine { id: MachineId, down: bool },
    Nothing,
}

/// The README's table: a project's session starts in its project, an online machine with nothing
/// running asks for a folder, a machine that is down is dialled again, and no selection asks
/// for a folder on this computer.
pub(in crate::client) fn quick_target_of(selected: Selected) -> QuickTarget {
    match selected {
        Selected::Project(p) => {
            let main = p.wts.iter().find(|w| w.main);
            QuickTarget::Start {
                machine: p.machine,
                project: main.map_or_else(|| p.path.clone(), |w| w.path.clone()),
                branch: main.map(|w| w.branch.clone()).filter(|b| !b.is_empty()),
            }
        }
        Selected::Machine { id, down: true } => QuickTarget::Reconnect(id),
        Selected::Machine { id, down: false } => QuickTarget::Scoped(id),
        Selected::Nothing => QuickTarget::Scoped(MachineId::LOCAL),
    }
}

/// A folder's last segment, whichever way the platform writes its separators.
fn folder_name(path: &str) -> &str {
    path.trim_end_matches(['/', '\\'])
        .rsplit(['/', '\\'])
        .next()
        .filter(|n| !n.is_empty())
        .unwrap_or(path)
}

impl App {
    /// The project the sidebar has selected (its cursor, even when the keys are in a pane), else
    /// the one you are in.
    fn selected_project(&self) -> Option<Proj> {
        let model = self.screen_model();
        let by_cursor = self.screen.cursor_proj.as_ref().and_then(|key| model.iter().find(|p| &p.key == key));
        let by_session = self
            .screen
            .cursor
            .and_then(|c| model.iter().find(|p| p.sessions().any(|s| s.key == c)));
        let current = self.screen.proj.as_ref().and_then(|key| model.iter().find(|p| &p.key == key));
        by_cursor.or(by_session).or(current).cloned()
    }

    pub(in crate::client) fn quick_target(&self) -> QuickTarget {
        match self.selected_project() {
            // A machine that is down is one row in the sidebar, not its projects: that row is
            // what is selected, and `n` dials it again.
            Some(p) if self.remotes.get(&p.machine).is_some_and(|r| r.state.is_down()) => {
                quick_target_of(Selected::Machine { id: p.machine, down: true })
            }
            Some(p) => quick_target_of(Selected::Project(&p)),
            None => quick_target_of(Selected::Nothing),
        }
    }

    fn machine_name_of(&self, machine: MachineId) -> String {
        self.remotes.get(&machine).map_or_else(|| "local".to_string(), |r| r.name.clone())
    }

    /// What `n` will do, for the sidebar's hint: `new in maitty`, `new in maitty · kingdom`,
    /// `new on kingdom…`, `reconnect pi-garage`.
    pub(in crate::client) fn quick_hint(&self) -> String {
        match self.quick_target() {
            QuickTarget::Start { machine, project, .. } => {
                let name = folder_name(&project.to_string_lossy()).to_string();
                if machine == MachineId::LOCAL {
                    format!("new in {name}")
                } else {
                    format!("new in {name} · {}", self.machine_name_of(machine))
                }
            }
            QuickTarget::Scoped(machine) => format!("new on {}…", self.machine_name_of(machine)),
            QuickTarget::Reconnect(machine) => format!("reconnect {}", self.machine_name_of(machine)),
        }
    }

    /// `n`: start now where the sidebar is, or ask where when there is no project to reuse.
    pub(in crate::client) fn quick_new(&mut self) {
        match self.quick_target() {
            QuickTarget::Start { machine, project, .. } => {
                self.create_session(machine, &project.to_string_lossy());
            }
            QuickTarget::Scoped(machine) => self.screen_open_scoped(machine),
            QuickTarget::Reconnect(machine) => {
                let name = self.machine_name_of(machine);
                let notice = self.machine_retry(&name);
                self.notify(notice.text, notice.error);
            }
        }
    }

    /// The picker already browsing a machine's home folder: `~/`, or `kingdom:~/`.
    pub(in crate::client) fn screen_open_scoped(&mut self, machine: MachineId) {
        let prefill = match self.remotes.get(&machine) {
            Some(remote) => format!("{}:~/", remote.name),
            None => "~/".to_string(),
        };
        self.screen_open_session_popup(&prefill);
    }

    /// The one way a session is started from `n`, the popup and the chips: the default agent in
    /// `path` on `machine`, then the terminal has the keys, the new session is the one selected
    /// in the sidebar (the daemon focuses it; `follow` scrolls it into view), a green line says
    /// what started, and the path is remembered as a recent one. It reuses the folder's own
    /// checkout: no worktree is made (UI.md records the open question).
    pub(in crate::client) fn create_session(&mut self, machine: MachineId, path: &str) {
        let agent = self.ns_default_agent();
        let cmd = np_command(self, &agent, 0, "");
        let project = folder_name(path).to_string();
        let shown = if machine == MachineId::LOCAL { tilde(Path::new(path)).replace('\\', "/") } else { path.to_string() };
        let remembered = typed(&self.ns_chips(), machine, &shown);
        self.screen.saved.recent_paths.retain(|p| *p != remembered);
        self.screen.saved.recent_paths.insert(0, remembered);
        self.screen.saved.recent_paths.truncate(RECENT_KEEP);
        self.screen.save();
        self.screen_new_session_on(machine, PathBuf::from(path), cmd, false);
        // The daemon focuses the new session; the sidebar's own cursor must not hold the
        // selection, and the focused card is scrolled into view when the focus arrives.
        self.screen.cursor_proj = None;
        self.screen.follow = true;
        let line = format!("+ {agent} · {project} on {}", self.machine_name_of(machine));
        self.bar_flash = Some((line.clone(), Instant::now()));
        self.notify(line, false);
    }
}
