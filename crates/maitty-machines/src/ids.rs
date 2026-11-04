//! Ids the window keeps for what lives on a machine. Each daemon numbers its own panes from 1,
//! so a pane is only named by its machine together with its `TermId`. These never cross the wire.

use maitty_shared::protocol::{ClientMsg, TermId};

/// Which machine's daemon something belongs to. The window starts with only `LOCAL`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MachineId(pub u16);

impl MachineId {
    pub const LOCAL: MachineId = MachineId(0);

    /// The high bits of a motion key, so two machines' panes animate apart. Zero for this machine.
    pub const fn motion_key(self) -> u64 {
        (self.0 as u64) << 48
    }
}

/// A pane, named across machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientTerm {
    pub machine: MachineId,
    pub term: TermId,
}

impl ClientTerm {
    pub const fn on(machine: MachineId, term: TermId) -> Self {
        ClientTerm { machine, term }
    }

    pub const fn local(term: TermId) -> Self {
        ClientTerm::on(MachineId::LOCAL, term)
    }

    pub const fn motion_key(self) -> u64 {
        self.machine.motion_key() | self.term as u64
    }
}

/// A message for one machine's daemon: the send side routes by `machine`.
#[derive(Debug)]
pub struct Outgoing {
    pub machine: MachineId,
    pub msg: ClientMsg,
}
