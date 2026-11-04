//! The task that keeps one machine's line up: dial, hand the line to the window, wait for it to
//! end, dial again. It never touches the window's state; it only sends events and hears
//! commands, so a dead machine cannot hold up drawing.

use super::state::{Backoff, ConnectionState, Event, next};
use super::{Command, LinkEvent};
use crate::connections::MachineId;
use crate::ipc::{self, Machine};
use crate::link_failure::{FailureClass, classify};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

/// The timings of one supervisor; the real ones are `Timing::DEFAULT`, tests use tiny ones.
#[derive(Debug, Clone, Copy)]
pub struct Timing {
    /// How long one dial may take.
    pub connect: Duration,
    pub backoff: Backoff,
}

impl Timing {
    pub const DEFAULT: Timing = Timing {
        connect: ipc::CONNECT_BUDGET,
        backoff: Backoff::DEFAULT,
    };
}

/// What a supervisor does between dials.
enum Pause {
    Elapsed,
    /// A person asked to try now.
    Retry,
    /// Nobody is listening any more.
    Closed,
}

/// Whether a dial may start the daemon on the far side. It exits when nothing is attached, so
/// a machine that never connected has none to attach to; but after a line was lost the daemon
/// is gone only if a person stopped it, and a redial must not bring it back.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct MayStart(bool);

impl MayStart {
    const FIRST: MayStart = MayStart(true);

    fn line_ended(&mut self) {
        self.0 = false;
    }

    fn retried(&mut self) {
        self.0 = true;
    }

    fn machine_for_dial(self, machine: &Machine) -> Machine {
        if self.0 {
            machine.starting()
        } else {
            machine.clone()
        }
    }
}

pub(super) async fn supervise(
    id: MachineId,
    name: String,
    machine: Machine,
    timing: Timing,
    events: mpsc::UnboundedSender<LinkEvent>,
    mut commands: mpsc::UnboundedReceiver<Command>,
) {
    let mut link = Link {
        id,
        name,
        state: ConnectionState::Connecting,
        events,
    };
    link.report();
    let mut failures = 0u32;
    let mut may_start = MayStart::FIRST;
    loop {
        match pause(timing.backoff.delay(failures), &mut commands).await {
            Pause::Elapsed => {}
            Pause::Retry => {
                failures = 0;
                may_start.retried();
                link.apply(Event::Retry);
            }
            Pause::Closed => return,
        }
        match dial(&may_start.machine_for_dial(&machine), timing.connect).await {
            Ok(halves) => {
                failures = 0;
                if link.events.send(LinkEvent::Line(id, halves)).is_err() {
                    return;
                }
                link.apply(Event::ConnectOk);
                match line_end(&mut commands).await {
                    Some(End::Stalled) => {
                        may_start.line_ended();
                        link.apply(Event::WatchdogFired);
                    }
                    Some(End::Closed) => {
                        may_start.line_ended();
                        link.apply(Event::LineDropped);
                    }
                    Some(End::Refused(why)) => {
                        link.apply(Event::Refused(why));
                        if !held_until_retry(&mut commands).await {
                            return;
                        }
                        failures = 0;
                        may_start.retried();
                        link.apply(Event::Retry);
                    }
                    None => return,
                }
            }
            Err(class) => {
                failures = failures.saturating_add(1);
                link.apply(Event::ConnectFailed(class));
            }
        }
    }
}

struct Link {
    id: MachineId,
    name: String,
    state: ConnectionState,
    events: mpsc::UnboundedSender<LinkEvent>,
}

impl Link {
    fn apply(&mut self, event: Event) {
        let next = next(&self.state, event, Instant::now());
        if next != self.state {
            self.state = next;
            self.report();
        }
    }

    fn report(&self) {
        let _ = self.events.send(LinkEvent::State {
            machine: self.id,
            name: self.name.clone(),
            state: self.state.clone(),
        });
    }
}

/// One attempt, bounded: ssh has its own connect timeout, and this one is a little longer, so
/// a dial that never answers (a stuck handshake) is dropped, and with it ssh.
async fn dial(
    machine: &Machine,
    within: Duration,
) -> Result<(ipc::Reader, ipc::Writer), FailureClass> {
    match tokio::time::timeout(within, ipc::open_to(machine, true)).await {
        Ok(Ok(halves)) => Ok(halves),
        Ok(Err(e)) => Err(classify(&e)),
        Err(_) => Err(FailureClass::Unreachable(format!(
            "no answer within {:?}",
            within
        ))),
    }
}

async fn pause(delay: Duration, commands: &mut mpsc::UnboundedReceiver<Command>) -> Pause {
    let sleep = tokio::time::sleep(delay);
    tokio::pin!(sleep);
    loop {
        tokio::select! {
            () = &mut sleep => return Pause::Elapsed,
            command = commands.recv() => match command {
                Some(Command::Retry) => return Pause::Retry,
                Some(_) => {}
                None => return Pause::Closed,
            },
        }
    }
}

enum End {
    Closed,
    Stalled,
    Refused(String),
}

/// Waits for the line just handed over to end. `None`: nobody is listening any more.
async fn line_end(commands: &mut mpsc::UnboundedReceiver<Command>) -> Option<End> {
    loop {
        match commands.recv().await? {
            Command::LineEnded { stalled: true } => return Some(End::Stalled),
            Command::LineEnded { stalled: false } => return Some(End::Closed),
            Command::Refuse(why) => return Some(End::Refused(why)),
            Command::Retry => {}
        }
    }
}

/// After a refusal no dial is made until a person asks. False: nobody is listening any more.
async fn held_until_retry(commands: &mut mpsc::UnboundedReceiver<Command>) -> bool {
    loop {
        match commands.recv().await {
            Some(Command::Retry) => return true,
            Some(_) => {}
            None => return false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn attached() -> Machine {
        Machine::ssh("box", vec!["ssh".into()], "maitty").attach_only()
    }

    #[test]
    fn the_first_dial_may_start_the_daemon() {
        let base = attached();
        assert_eq!(MayStart::FIRST.machine_for_dial(&base), base.starting());
    }

    #[test]
    fn a_redial_after_a_lost_line_only_attaches() {
        let base = attached();
        let mut may_start = MayStart::FIRST;
        may_start.line_ended();
        assert_eq!(may_start.machine_for_dial(&base), base);
    }

    #[test]
    fn a_retry_may_start_the_daemon_again() {
        let base = attached();
        let mut may_start = MayStart::FIRST;
        may_start.line_ended();
        may_start.retried();
        assert_eq!(may_start.machine_for_dial(&base), base.starting());
    }
}
