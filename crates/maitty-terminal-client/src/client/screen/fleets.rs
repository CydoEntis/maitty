//! Where a new shell goes. The daemon groups panes in workspaces (one daemon per machine, ADR
//! 0007); the sidebar no longer shows them, but a shell still starts in one: the workspace of the
//! pane you are on, else the machine's first, else a new Scratch one. This file draws nothing.

use super::*;
use crate::protocol::WsId;

/// The workspace a shell starts in when nothing else is open.
const SCRATCH: &str = "Scratch";

/// A workspace: the machine's daemon and the workspace on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::client) struct FleetId {
    pub machine: MachineId,
    pub ws: WsId,
}

impl App {
    /// Every pane of a workspace, in creation order (ids only count up).
    pub(super) fn fleet_ships(&self, id: FleetId) -> Vec<ClientTerm> {
        let Some(snap) = self.snap_of(id.machine) else {
            return Vec::new();
        };
        let held_back = |term: ClientTerm| {
            self.screen.pending_split.is_some() && !self.screen.split_known.contains(&term)
        };
        let mut ships: Vec<ClientTerm> = snap
            .workspaces
            .iter()
            .filter(|w| w.id == id.ws)
            .flat_map(|w| w.tabs.iter().flat_map(|t| t.layout.leaves()))
            .filter(|t| snap.terms.get(t).is_some_and(|t| !t.popup && !t.archived))
            .map(|t| ClientTerm::on(id.machine, t))
            .filter(|t| !held_back(*t))
            .collect();
        ships.sort_by_key(|t| t.term);
        ships.dedup();
        ships
    }

    /// The workspace a pane is in.
    pub(super) fn fleet_of(&self, term: ClientTerm) -> Option<FleetId> {
        self.snap_of(term.machine)?
            .workspaces
            .iter()
            .find(|w| w.tabs.iter().any(|t| t.layout.contains(term.term)))
            .map(|w| FleetId {
                machine: term.machine,
                ws: w.id,
            })
    }

    /// The workspace you are in: the focused pane's, else the daemon's selected one.
    pub(super) fn current_fleet(&self) -> Option<FleetId> {
        if let Some(f) = self.focused().and_then(|t| self.fleet_of(t)) {
            return Some(f);
        }
        let machine = self.focus_machine();
        self.snap_of(machine)?.active().map(|w| FleetId {
            machine,
            ws: w.id,
        })
    }

    fn fleet_info(&self, id: FleetId) -> Option<&crate::protocol::WorkspaceInfo> {
        self.snap_of(id.machine)?.workspaces.iter().find(|w| w.id == id.ws)
    }

    /// The pane you were last on in a workspace, else its first.
    fn fleet_target(&self, id: FleetId) -> Option<ClientTerm> {
        let ships = self.fleet_ships(id);
        self.screen
            .fleet_active
            .get(&id)
            .copied()
            .filter(|t| ships.contains(t))
            .or_else(|| ships.first().copied())
    }

    /// Remember that `term` is the one you are on in its workspace.
    pub(super) fn note_fleet_active(&mut self, term: ClientTerm) {
        if let Some(fleet) = self.fleet_of(term) {
            self.screen.fleet_active.insert(fleet, term);
        }
    }

    /// A shell at once in `fleet` (the current one when none is given), on its machine, in the
    /// folder you are working in there.
    pub(in crate::client) fn new_ship(&mut self, fleet: Option<FleetId>) {
        self.leave_shortcut_mode();
        self.screen.cursor = None;
        self.mode = Mode::Normal;
        let Some(fleet) = fleet.or_else(|| self.current_fleet()) else {
            self.new_scratch_ship(self.focus_machine());
            return;
        };
        if self.machine_down(fleet.machine) {
            let who = self.machine_label_of(fleet.machine);
            let who = if who.is_empty() { "this computer".into() } else { who };
            self.notify(format!("{who} is offline; a shell can't start there"), true);
            return;
        }
        let Some(info) = self.fleet_info(fleet) else {
            return;
        };
        // The folder you are working in if you are in this workspace, else where it last was,
        // else the workspace's own.
        let folder = self
            .focused()
            .filter(|t| self.fleet_of(*t) == Some(fleet))
            .or_else(|| self.fleet_target(fleet))
            .and_then(|t| self.term_info(t))
            .map_or_else(|| info.cwd.clone(), |t| t.cwd.clone());
        self.cmd_on(
            fleet.machine,
            Command::NewShip {
                ws: fleet.ws,
                cwd: Some(folder.clone()),
                cmd: None,
            },
        );
        self.notify(format!("New shell · {}", tilde(&folder)), false);
    }

    /// With no workspace to put a shell in, start the Scratch one.
    fn new_scratch_ship(&mut self, machine: MachineId) {
        if self.machine_down(machine) {
            self.notify("that machine is offline; a shell can't start there".into(), true);
            return;
        }
        self.cmd_on(
            machine,
            Command::NewWorkspace {
                cwd: self.cfg.start_dir(),
                name: Some(SCRATCH.into()),
                cmd: None,
            },
        );
        self.notify("New shell · ~".into(), false);
    }

    /// The `+` on a machine heading: a shell at once in that machine's first workspace.
    pub(super) fn new_ship_on(&mut self, machine: MachineId) {
        let first = self
            .snap_of(machine)
            .and_then(|s| s.workspaces.first())
            .map(|w| FleetId {
                machine,
                ws: w.id,
            });
        match first {
            Some(f) => self.new_ship(Some(f)),
            None => {
                self.leave_shortcut_mode();
                self.new_scratch_ship(machine);
            }
        }
    }
}
