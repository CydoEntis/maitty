//! Keeping each configured machine's connection up. One supervisor task per machine dials
//! through ssh (attach only), hands the line to the window, and dials again when it ends; the
//! window learns of it through `LinkEvent`s and tells the supervisors through `LinkControl`.
//! Nothing here is shared with the draw loop, and with no configured machine nothing starts.

mod state;
mod supervisor;

pub use state::{Backoff, ConnectionState, Event, RECONNECT_DIALS, next};
pub use supervisor::Timing;

use crate::{Dial, Machine, MachineDef, MachineId};
use maitty_ipc_transport::{Reader, Writer};
use std::collections::BTreeMap;
use std::sync::{Arc, Mutex, MutexGuard};
use tokio::runtime::Handle;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// How a supervisor learns what to say about this process in each Hello. The terminal app
/// reads its pane and conversation identity from its environment, the desktop app has none;
/// either way it is the caller's to say, never read here.
type Opener = Arc<dyn Fn() -> anyhow::Result<Dial> + Send + Sync>;

/// What a supervisor tells the window.
pub enum LinkEvent {
    /// A new line to register for the machine; its `State(Connected)` follows.
    Line(MachineId, (Reader, Writer)),
    State {
        machine: MachineId,
        name: String,
        state: ConnectionState,
    },
}

/// What the window tells a supervisor.
#[derive(Debug)]
pub(super) enum Command {
    /// The line it was handed ended; `stalled` when a frame stopped arriving.
    LineEnded { stalled: bool },
    /// The window will not take this machine (a different daemon answers at its name): stop
    /// until a person asks.
    Refuse(String),
    /// Dial now, whatever the wait.
    Retry,
}

/// How the window reaches the supervisors, and starts or stops one while it runs. Cheap to
/// clone; one made with `default()` reaches nothing and starts nothing.
#[derive(Clone, Default)]
pub struct LinkControl {
    shared: Option<Arc<Shared>>,
}

struct Shared {
    supervisors: Mutex<Supervisors>,
    runtime: Handle,
    events: mpsc::UnboundedSender<LinkEvent>,
    timing: Timing,
    dial: Opener,
}

#[derive(Default)]
struct Supervisors {
    running: BTreeMap<MachineId, Running>,
    /// The id the next machine added while the window runs gets. Never reused, so a pane
    /// kept for a machine that was removed cannot be taken for the one added after it.
    next_id: u16,
}

struct Running {
    /// Lowercased, as names are compared.
    name: String,
    commands: mpsc::UnboundedSender<Command>,
    task: JoinHandle<()>,
}

impl LinkControl {
    /// Whether `machine` has a supervisor.
    pub fn supervises(&self, machine: MachineId) -> bool {
        self.with(|s| s.running.contains_key(&machine))
            .unwrap_or(false)
    }

    /// The id of the machine configured under `name`, in any case.
    pub fn id_of(&self, name: &str) -> Option<MachineId> {
        let name = name.to_lowercase();
        self.with(|s| {
            s.running
                .iter()
                .find(|(_, running)| running.name == name)
                .map(|(id, _)| *id)
        })
        .flatten()
    }

    /// The machine's line ended. False when nothing supervises it.
    pub fn line_ended(&self, machine: MachineId, stalled: bool) -> bool {
        self.tell(machine, Command::LineEnded { stalled })
    }

    pub fn refuse(&self, machine: MachineId, why: String) -> bool {
        self.tell(machine, Command::Refuse(why))
    }

    /// Dial the machine now and forget how long it has been failing.
    pub fn retry(&self, machine: MachineId) -> bool {
        self.tell(machine, Command::Retry)
    }

    /// Start a supervisor for a machine added while the window runs, under a new id. None
    /// when this control reaches no runtime (tests of the window alone).
    pub fn start(&self, machine: Machine) -> Option<MachineId> {
        let shared = self.shared.as_ref()?;
        let mut supervisors = lock(&shared.supervisors);
        let id = MachineId(supervisors.next_id.max(1));
        supervisors.next_id = id.0.checked_add(1)?;
        supervisors.running.insert(id, shared.spawn(id, machine));
        Some(id)
    }

    /// Stop a machine's supervisor, and with it any ssh it holds. False when it had none.
    pub fn stop(&self, machine: MachineId) -> bool {
        let Some(shared) = &self.shared else {
            return false;
        };
        let removed = lock(&shared.supervisors).running.remove(&machine);
        removed.is_some_and(|running| {
            running.task.abort();
            true
        })
    }

    fn tell(&self, machine: MachineId, command: Command) -> bool {
        self.with(|s| {
            s.running
                .get(&machine)
                .is_some_and(|running| running.commands.send(command).is_ok())
        })
        .unwrap_or(false)
    }

    fn with<T>(&self, f: impl FnOnce(&Supervisors) -> T) -> Option<T> {
        self.shared.as_ref().map(|s| f(&lock(&s.supervisors)))
    }
}

impl Shared {
    fn spawn(&self, id: MachineId, machine: Machine) -> Running {
        let (commands, commands_rx) = mpsc::unbounded_channel();
        let name = machine.name().to_lowercase();
        let task = self.runtime.spawn(supervisor::supervise(
            id,
            machine.name().to_owned(),
            machine,
            self.timing,
            self.dial.clone(),
            self.events.clone(),
            commands_rx,
        ));
        Running {
            name,
            commands,
            task,
        }
    }
}

fn lock(supervisors: &Mutex<Supervisors>) -> MutexGuard<'_, Supervisors> {
    supervisors
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// The supervisors and the events they raise. Dropping it stops them, and with them any ssh.
pub struct Links {
    events: mpsc::UnboundedReceiver<LinkEvent>,
    control: LinkControl,
}

impl Links {
    /// Start a supervisor for each machine. Must run inside the runtime.
    pub fn start(
        machines: Vec<(MachineId, Machine)>,
        timing: Timing,
        dial: impl Fn() -> anyhow::Result<Dial> + Send + Sync + 'static,
    ) -> Links {
        let (events_tx, events) = mpsc::unbounded_channel();
        let shared = Shared {
            supervisors: Mutex::default(),
            runtime: Handle::current(),
            events: events_tx,
            timing,
            dial: Arc::new(dial),
        };
        let mut supervisors = Supervisors::default();
        for (id, machine) in machines {
            supervisors.next_id = supervisors.next_id.max(id.0.saturating_add(1));
            supervisors.running.insert(id, shared.spawn(id, machine));
        }
        *lock(&shared.supervisors) = supervisors;
        Links {
            events,
            control: LinkControl {
                shared: Some(Arc::new(shared)),
            },
        }
    }

    pub fn control(&self) -> LinkControl {
        self.control.clone()
    }

    /// Whether any machine is supervised: with none there is nothing to wait for.
    pub fn any(&self) -> bool {
        self.control
            .with(|s| !s.running.is_empty())
            .unwrap_or(false)
    }

    /// The next event. Waits for ever with no supervisor.
    pub async fn next(&mut self) -> LinkEvent {
        if !self.any() {
            return std::future::pending().await;
        }
        match self.events.recv().await {
            Some(event) => event,
            None => std::future::pending().await,
        }
    }
}

impl Drop for Links {
    fn drop(&mut self) {
        if let Some(shared) = &self.control.shared {
            lock(&shared.supervisors)
                .running
                .values()
                .for_each(|running| running.task.abort());
        }
    }
}

/// The configured machines with the ids the window gives them: 1, 2, ... in name order, so a
/// run's ids follow from its config alone. Ids are never saved (layouts are kept under the
/// name), so a machine added in the middle renumbers the ones after it without harm. This
/// computer is `MachineId::LOCAL` (0) and is never in the list.
pub fn configured_machines(defs: &[MachineDef], ssh: &str) -> Vec<(MachineId, Machine)> {
    let mut sorted: Vec<&MachineDef> = defs.iter().collect();
    sorted.sort_by_key(|def| def.name.to_lowercase());
    sorted
        .into_iter()
        .zip(1u16..)
        .map(|(def, id)| (MachineId(id), Machine::from_def(def, ssh)))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(name: &str) -> MachineDef {
        MachineDef {
            name: name.into(),
            target: format!("{name}.example"),
            ..MachineDef::default()
        }
    }

    #[test]
    fn machines_get_ids_in_name_order_whatever_order_the_config_lists_them() {
        let ids = |names: &[&str]| -> Vec<(u16, String)> {
            let defs: Vec<_> = names.iter().map(|n| def(n)).collect();
            configured_machines(&defs, "ssh")
                .into_iter()
                .map(|(id, m)| (id.0, m.name().to_owned()))
                .collect()
        };
        let want = vec![
            (1, "build".to_owned()),
            (2, "gpu".to_owned()),
            (3, "Web".to_owned()),
        ];
        assert_eq!(ids(&["gpu", "Web", "build"]), want);
        assert_eq!(ids(&["build", "Web", "gpu"]), want);
    }

    #[test]
    fn no_configured_machine_starts_nothing() {
        assert!(configured_machines(&[], "ssh").is_empty());
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let links = Links::start(Vec::new(), Timing::DEFAULT, dial);
            assert!(!links.any());
            assert!(!links.control().supervises(MachineId(1)));
        });
    }
    fn dial() -> anyhow::Result<Dial> {
        Ok(Dial {
            attach: true,
            surface: maitty_shared::protocol::Surface::Terminal,
            replays: true,
            term: None,
            token: None,
            structured: None,
            newest_only: false,
        })
    }

    fn nowhere(name: &str) -> Machine {
        Machine::ssh(name, vec!["no-such-ssh-program".into()], "maitty")
    }

    #[test]
    fn a_machine_added_while_running_gets_a_new_id_and_a_stopped_one_is_not_dialled() {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let mut links = Links::start(
                vec![(MachineId(1), nowhere("Build"))],
                Timing::DEFAULT,
                dial,
            );
            let control = links.control();
            assert_eq!(control.id_of("build"), Some(MachineId(1)));

            let gpu = control.start(nowhere("gpu")).expect("a supervisor starts");
            assert_eq!(gpu, MachineId(2), "after the ids the window started with");
            assert!(control.supervises(gpu) && links.any());
            let mut named = Vec::new();
            while named.len() < 2 {
                if let LinkEvent::State { machine, name, .. } = links.next().await {
                    named.push((machine, name));
                }
            }
            assert!(named.contains(&(gpu, "gpu".to_string())), "{named:?}");

            assert!(control.stop(gpu));
            assert!(!control.stop(gpu), "nothing is left to stop");
            assert!(!control.supervises(gpu) && !control.retry(gpu));
            assert_eq!(control.id_of("gpu"), None);
            let again = control.start(nowhere("gpu")).unwrap();
            assert_eq!(again, MachineId(3), "an id is never given twice in a run");
        });
    }

    #[test]
    fn a_control_with_no_window_behind_it_starts_and_stops_nothing() {
        let control = LinkControl::default();
        assert_eq!(control.start(nowhere("box")), None);
        assert!(!control.stop(MachineId(1)) && !control.retry(MachineId(1)));
        assert_eq!(control.id_of("box"), None);
    }
}
