//! Fleets and ships. A fleet is a named group of sessions on one machine; the daemon already
//! keeps one: a workspace. There is one daemon per machine, so each machine's workspaces are its
//! fleets. A ship is one session in a fleet, an agent or a shell. This file reads them off the
//! snapshots for the sidebar, the collapsed column and the tab row, and starts a new shell in a
//! fleet. It draws nothing.

use super::*;
use crate::protocol::WsId;

/// The fold key and sort key for this computer: it has no configured name.
pub(super) const LOCAL_KEY: &str = "local";
/// The one fleet that has no colour of its own: where a shell goes when nothing else is open.
const SCRATCH: &str = "Scratch";
/// What the machine heading calls the computer the window runs on.
const THIS_COMPUTER: &str = "This computer";
/// The colours a fleet cycles through (teal, violet, blue, orange).
const FLEET_COLOURS: [Color; 4] = [
    Color::Rgb(0x2e, 0xc4, 0xb6),
    Color::Rgb(0xa7, 0x8b, 0xfa),
    Color::Rgb(0x60, 0xa5, 0xfa),
    Color::Rgb(0xfb, 0x92, 0x3c),
];

/// A fleet: the machine's daemon and the workspace on it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub(in crate::client) struct FleetId {
    pub machine: MachineId,
    pub ws: WsId,
}

#[derive(Debug, Clone)]
pub(super) struct Fleet {
    pub id: FleetId,
    pub name: String,
    /// `None` is the muted Scratch fleet.
    pub colour: Option<Color>,
    /// Every pane in it, in creation order (ids only count up).
    pub ships: Vec<ClientTerm>,
}

/// A machine and its fleets, as a heading with rows under it.
#[derive(Debug, Clone)]
pub(super) struct MachineGroup {
    pub id: MachineId,
    /// Its fold key without the prefix.
    pub key: String,
    pub label: String,
    pub down: bool,
    pub fleets: Vec<Fleet>,
}

impl MachineGroup {
    pub fn fold_key(&self) -> String {
        format!("m:{}", self.key)
    }
}

impl Fleet {
    /// The fold key of its ship group.
    pub fn fold_key(&self, machine_key: &str) -> String {
        format!("c:{machine_key}/{}", self.id.ws)
    }
}

/// The key a machine's folds are saved under.
pub(super) fn machine_key(name: &str) -> String {
    if name.is_empty() {
        LOCAL_KEY.to_string()
    } else {
        name.to_string()
    }
}

/// A fleet's colour from its workspace's palette index; Scratch has none.
pub(super) fn colour_of(ws_colour: u8, name: &str) -> Option<Color> {
    if name.trim().eq_ignore_ascii_case(SCRATCH) {
        None
    } else {
        Some(FLEET_COLOURS[ws_colour as usize % FLEET_COLOURS.len()])
    }
}

/// What a fleet is called: its name, else the folder it started in.
fn fleet_name(ws: &crate::protocol::WorkspaceInfo) -> String {
    let named = ws.name.trim();
    if named.is_empty() {
        folder_name(&ws.cwd)
    } else {
        named.to_string()
    }
}

impl App {
    /// Every machine with its fleets, this computer first.
    pub(super) fn fleet_groups(&self) -> Vec<MachineGroup> {
        let held_back = |term: ClientTerm| {
            self.screen.pending_split.is_some() && !self.screen.split_known.contains(&term)
        };
        self.machines()
            .into_iter()
            .map(|m| {
                let fleets = m
                    .snap
                    .workspaces
                    .iter()
                    .map(|w| {
                        let mut ships: Vec<ClientTerm> = w
                            .tabs
                            .iter()
                            .flat_map(|t| t.layout.leaves())
                            .filter(|id| m.snap.terms.get(id).is_some_and(|t| !t.popup && !t.archived))
                            .map(|id| ClientTerm::on(m.id, id))
                            .filter(|t| !held_back(*t))
                            .collect();
                        ships.sort_by_key(|t| t.term);
                        ships.dedup();
                        let name = fleet_name(w);
                        Fleet {
                            id: FleetId {
                                machine: m.id,
                                ws: w.id,
                            },
                            colour: colour_of(w.color, &name),
                            name,
                            ships,
                        }
                    })
                    .collect();
                MachineGroup {
                    id: m.id,
                    key: machine_key(m.name),
                    label: if m.id == MachineId::LOCAL {
                        THIS_COMPUTER.to_string()
                    } else {
                        self.machine_label_of(m.id)
                    },
                    down: self.machine_down(m.id),
                    fleets,
                }
            })
            .collect()
    }

    /// The fleet a pane is in.
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

    /// The fleet you are in: the focused pane's, else the daemon's selected workspace.
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

    pub(super) fn fleet_info(&self, id: FleetId) -> Option<&crate::protocol::WorkspaceInfo> {
        self.snap_of(id.machine)?.workspaces.iter().find(|w| w.id == id.ws)
    }

    /// A fleet's pane that opens when you pick the fleet: the one you were last on in it, else
    /// its first.
    pub(super) fn fleet_target(&self, id: FleetId) -> Option<ClientTerm> {
        let groups = self.fleet_groups();
        let fleet = groups
            .iter()
            .flat_map(|g| &g.fleets)
            .find(|f| f.id == id)?;
        self.screen
            .fleet_active
            .get(&id)
            .copied()
            .filter(|t| fleet.ships.contains(t))
            .or_else(|| fleet.ships.first().copied())
    }

    /// Open a fleet: its last active session.
    pub(super) fn open_fleet(&mut self, id: FleetId) {
        match self.fleet_target(id) {
            Some(term) => {
                self.screen_focus(term);
                self.screen_side_leave();
            }
            None => self.notify("that fleet has nothing running".into(), true),
        }
    }

    /// Remember that `term` is the one you are on in its fleet.
    pub(super) fn note_fleet_active(&mut self, term: ClientTerm) {
        if let Some(fleet) = self.fleet_of(term) {
            self.screen.fleet_active.insert(fleet, term);
        }
    }

    /// A shell at once in `fleet` (the current one when none is given), on the fleet's machine:
    /// it becomes the fleet's active session and shows as a new tab.
    pub(in crate::client) fn new_ship(&mut self, fleet: Option<FleetId>) {
        self.leave_shortcut_mode();
        self.screen.cursor = None;
        self.screen.cursor_proj = None;
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
        let name = fleet_name(info);
        // The folder you are working in if you are in this fleet, else where it last was, else
        // the fleet's own.
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
        self.notify(format!("New shell in {name} · {}", tilde(&folder)), false);
    }

    /// With no fleet to put a shell in, start the Scratch fleet.
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
        self.notify(format!("New shell in {SCRATCH} · ~"), false);
    }

    /// The `+` on a machine heading: a shell in that machine's first fleet.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_fleet_cycles_through_four_colours_and_scratch_has_none() {
        assert_eq!(colour_of(0, "AeVox launch"), Some(FLEET_COLOURS[0]));
        assert_eq!(colour_of(5, "Maitty"), Some(FLEET_COLOURS[1]));
        assert_eq!(colour_of(3, "API"), Some(FLEET_COLOURS[3]));
        assert_eq!(colour_of(2, "Scratch"), None);
        assert_eq!(colour_of(2, " scratch "), None);
    }

    #[test]
    fn folds_are_keyed_by_machine_and_fleet() {
        assert_eq!(machine_key(""), "local");
        assert_eq!(machine_key("devbox"), "devbox");
    }
}
