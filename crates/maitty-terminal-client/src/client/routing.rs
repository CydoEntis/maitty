//! Which machine a message goes to, and what a pane's machine lets this window do. A pane's
//! files, git and agent hooks live on its own machine, so anything that reads or writes them
//! asks here first.

use super::*;
use std::cell::{Cell, RefCell};
use std::collections::{BTreeMap, VecDeque};

/// How long a reply to a command or query is believed. Past it, a machine that answers is
/// answering nothing this window asked.
const ANSWER_WINDOW: Duration = Duration::from_secs(30);

/// Most unanswered requests kept per machine: commands nothing replies to must not pile up.
const ASKED_MAX: usize = 64;

/// What the window remembers about where it is talking. Behind cells because messages are sent
/// from methods that only borrow the app.
#[derive(Default)]
pub(super) struct Routing {
    focus: Cell<MachineId>,
    asked: RefCell<BTreeMap<MachineId, VecDeque<Instant>>>,
}

impl Routing {
    pub(super) fn focus(&self) -> MachineId {
        self.focus.get()
    }

    pub(super) fn set_focus(&self, machine: MachineId) {
        self.focus.set(machine);
    }

    /// A command or query went to `machine`: a reply or error from it may now be its answer.
    pub(super) fn asked(&self, machine: MachineId) {
        let mut asked = self.asked.borrow_mut();
        let queue = asked.entry(machine).or_default();
        queue.push_back(Instant::now());
        while queue.len() > ASKED_MAX {
            queue.pop_front();
        }
    }

    /// Whether this window has a request out to `machine` that is still waiting for an answer.
    pub(super) fn awaiting(&self, machine: MachineId) -> bool {
        let mut asked = self.asked.borrow_mut();
        let Some(queue) = asked.get_mut(&machine) else {
            return false;
        };
        while queue
            .front()
            .is_some_and(|at| at.elapsed() >= ANSWER_WINDOW)
        {
            queue.pop_front();
        }
        !queue.is_empty()
    }

    /// An answer from `machine` was taken: one request is no longer waiting.
    pub(super) fn answered(&self, machine: MachineId) {
        if let Some(queue) = self.asked.borrow_mut().get_mut(&machine) {
            queue.pop_front();
        }
    }
}

/// Whether a command moves the focus on the machine it is sent to.
pub(super) fn moves_focus(c: &Command) -> bool {
    matches!(
        c,
        Command::FocusPane { .. }
            | Command::SelectWorkspace { .. }
            | Command::SelectTab { .. }
            | Command::NewWorkspace { .. }
            | Command::NewTab { .. }
            | Command::NewShip { .. }
            | Command::NewFleet { .. }
            | Command::NewWorktree { .. }
            | Command::AgentWorktree { .. }
    )
}

/// A thing that reads or writes the files of a pane, or runs git there, which this window can
/// only do for the machine it runs on.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Feature {
    Files,
    Changes,
    Find,
    Branches,
    References,
    AgentStatus,
    Worktrees,
}

impl Feature {
    pub(super) fn of(action: &Action) -> Option<Feature> {
        Some(match action {
            Action::Files => Feature::Files,
            Action::Changes => Feature::Changes,
            Action::Find(_) => Feature::Find,
            Action::Branches => Feature::Branches,
            Action::References => Feature::References,
            Action::AgentStatusSetup => Feature::AgentStatus,
            Action::NewWorktree(_) | Action::RemoveWorktree | Action::TidyWorktrees => {
                Feature::Worktrees
            }
            _ => return None,
        })
    }

    fn label(self) -> &'static str {
        match self {
            Feature::Files => "Files",
            Feature::Changes => "Changes",
            Feature::Find => "Find",
            Feature::Branches => "Branches",
            Feature::References => "References",
            Feature::AgentStatus => "Connecting an agent",
            Feature::Worktrees => "Worktrees",
        }
    }
}

/// The toast for a feature that cannot work on `machine`'s files from here.
pub(super) fn local_only_message(feature: Feature, machine: &str) -> String {
    format!(
        "{} is local-only in v1: the files are on {machine}, not on this computer.",
        feature.label()
    )
}

/// What a message is, in words, for telling that it was not sent.
fn describe(msg: &ClientMsg) -> Option<String> {
    let debug = match msg {
        ClientMsg::Command(c) => format!("{c:?}"),
        ClientMsg::Query(q) => format!("{q:?}"),
        _ => return None,
    };
    let name = debug.split(|c: char| !c.is_alphanumeric()).next()?;
    let mut words = String::new();
    for (i, c) in name.chars().enumerate() {
        if c.is_uppercase() && i > 0 {
            words.push(' ');
        }
        words.extend(c.to_lowercase());
    }
    Some(words)
}

impl App {
    /// The machine whose active workspace and tab hold the window's focus. It follows the last
    /// command that moved a focus, and is this computer when that machine is gone.
    pub(super) fn focus_machine(&self) -> MachineId {
        let machine = self.route.focus();
        if self.snap_of(machine).is_some() {
            machine
        } else {
            MachineId::LOCAL
        }
    }

    /// Where something new starts: the machine of the session selected in the sidebar, else of
    /// the focused one; this computer when neither is anywhere else.
    pub(super) fn target_machine(&self) -> MachineId {
        self.screen
            .cursor
            .map(|session| session.machine)
            .filter(|machine| self.snap_of(*machine).is_some())
            .unwrap_or_else(|| self.focus_machine())
    }

    /// Whether a check's output could be `machine`'s answer: this computer's always may be;
    /// another's only when the open sheet or the failure waiting for it is on that machine.
    pub(super) fn check_asked_of(&self, machine: MachineId) -> bool {
        let sheet = matches!(&self.view, Some(View::Check(v)) if v.term.machine == machine);
        let failure = self
            .screen
            .failure_ask
            .as_ref()
            .is_some_and(|ask| ask.term.machine == machine);
        machine == MachineId::LOCAL || sheet || failure
    }

    /// The machine a chat belongs to. Chat numbers are each daemon's own, so the selected
    /// session's machine is asked first, then the others in order; this computer when none
    /// lists it.
    pub(super) fn conversation_machine(
        &self,
        id: crate::conversation::ConversationId,
    ) -> MachineId {
        let has = |machine: MachineId| {
            self.snap_of(machine)
                .is_some_and(|snap| snap.conversations.iter().any(|c| c.id == id))
        };
        let target = self.target_machine();
        if has(target) {
            return target;
        }
        self.machines()
            .iter()
            .map(|m| m.id)
            .find(|m| has(*m))
            .unwrap_or(MachineId::LOCAL)
    }

    /// The name of the machine whose disk holds what `machine` runs, when that is not this
    /// computer. A window attached to a remote daemon has no local machine: its own is remote.
    fn files_elsewhere(&self, machine: MachineId) -> Option<String> {
        if machine != MachineId::LOCAL {
            let name = self.machine_name(machine);
            return Some(
                if name.is_empty() {
                    "another machine"
                } else {
                    name
                }
                .to_owned(),
            );
        }
        self.machine
            .is_remote()
            .then(|| self.machine.name().to_owned())
    }

    pub(super) fn files_are_here(&self, machine: MachineId) -> bool {
        self.files_elsewhere(machine).is_none()
    }

    pub(super) fn is_local_pane(&self, term: ClientTerm) -> bool {
        self.files_are_here(term.machine)
    }

    /// Whether `feature` may run for a pane on `machine`: the toast text when it may not.
    pub(super) fn local_only(&self, machine: MachineId, feature: Feature) -> Result<(), String> {
        // A window attached to one remote daemon has that daemon make its worktrees, as it
        // always has; only a second machine's worktrees wait for a later version.
        let name = match feature {
            Feature::Worktrees if machine == MachineId::LOCAL => None,
            _ => self.files_elsewhere(machine),
        };
        match name {
            Some(name) => Err(local_only_message(feature, &name)),
            None => Ok(()),
        }
    }

    /// True to go ahead; otherwise the person is told why not.
    pub(super) fn guard_local(&mut self, machine: MachineId, feature: Feature) -> bool {
        match self.local_only(machine, feature) {
            Ok(()) => true,
            Err(why) => {
                self.notify(why, true);
                false
            }
        }
    }

    /// The folder a pane runs in, as a path on this computer: none for a pane on another
    /// machine, whose folder may not exist here or may be someone else's.
    pub(super) fn local_cwd(&self, term: ClientTerm) -> Option<PathBuf> {
        if !self.is_local_pane(term) {
            return None;
        }
        self.term_info(term).map(|t| t.cwd.clone())
    }

    /// The git projects whose folders are on this computer's disk: the only ones git is run
    /// in, since another machine's paths mean nothing here.
    pub(super) fn local_git_projects<'a>(
        &'a self,
        model: &'a [screen::Proj],
    ) -> impl Iterator<Item = &'a screen::Proj> {
        model
            .iter()
            .filter(|p| p.git && self.files_are_here(p.machine))
    }

    /// Hand a message to its machine's connection. Typing for a machine that is down is
    /// counted; any other request says it was not sent.
    pub(super) fn deliver(&mut self, out: Outgoing, connections: &Connections) {
        let typed = matches!(out.msg, ClientMsg::Input { .. });
        let what = describe(&out.msg);
        let Err(NotConnected(machine)) = connections.send(out) else {
            return;
        };
        tracing::debug!("dropped a message for machine {}: not connected", machine.0);
        if typed {
            self.note_unsent_input(machine);
        } else if let Some(what) = what {
            self.tell_not_sent(machine, &what);
        }
    }

    fn tell_not_sent(&mut self, machine: MachineId, what: &str) {
        let name = match self.machine_name(machine) {
            "" => "This computer's daemon".to_owned(),
            name => name.to_owned(),
        };
        self.notify(
            format!("{name} is disconnected; {what} was not sent."),
            true,
        );
    }
}
