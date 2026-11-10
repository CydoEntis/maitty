//! What the window is given of every machine's panes, in one place and without Tauri: each
//! pane's history, its sequence number, each machine's last whole snapshot, and the coalescing
//! that turns a pane's many small reads into one frame (ADR 0007).
//!
//! A frame is the machine as 2 big-endian bytes, the pane as 4, the sequence number as 8, one
//! byte that is 1 for a replay, then the output. `apps/gui/src/api/terminalStream.ts` decodes it.
//! The header is the host-to-window format only; the daemon protocol is unchanged.
//!
//! All machines share one channel and one timer, so a machine that talks a great deal must not
//! slow the others. This computer's own output is given as it always was. Another machine gets
//! at most `REMOTE_BYTES_PER_FLUSH` into each flush, and when more than `REMOTE_PENDING_MAX` of
//! its output waits the pane's waiting output is dropped and its daemon is asked for a fresh
//! replay, which puts the view back in step.

use maitty_machines::MachineId;
use maitty_machines::remote_snapshot::sanitize_snapshot;
use maitty_shared::protocol::{FolderListing, Reply, ServerMsg, Snapshot, Status, TermId};
use maitty_shared::settings_view::SharedSettings;
use std::collections::{BTreeMap, HashMap};
use std::time::{Duration, Instant};

/// More history than this for one pane is dropped; a view then asks the daemon for a fresh replay.
pub const HISTORY_LIMIT_BYTES: usize = 8 * 1024 * 1024;
/// Output waits this long for more before it is sent: under a frame, so typing still echoes at once.
pub const FLUSH_EVERY: Duration = Duration::from_millis(12);
/// A pane of this computer with this much waiting is sent without waiting for the timer.
pub const FLUSH_AT_BYTES: usize = 64 * 1024;
/// The most output of one other machine that goes to the window in one flush.
pub const REMOTE_BYTES_PER_FLUSH: usize = 256 * 1024;
/// The most output of one other machine that may wait to go to the window.
pub const REMOTE_PENDING_MAX: usize = 4 * 1024 * 1024;
/// The history reply starts with the sequence number it ends at, as big-endian bytes.
pub const SEQ_BYTES: usize = std::mem::size_of::<u64>();
/// A frame's header: the machine, the pane, the sequence number and the replay flag.
pub const FRAME_HEADER_BYTES: usize =
    std::mem::size_of::<u16>() + std::mem::size_of::<u32>() + SEQ_BYTES + 1;

type PaneKey = (MachineId, TermId);

#[derive(Default)]
struct Pane {
    /// The sequence number of the last chunk, counting from 1.
    seq: u64,
    /// The pane's last replay followed by every chunk of output since.
    history: Vec<u8>,
    /// False once the history overflowed: it no longer starts at a replay.
    complete: bool,
    /// A fresh replay has been asked for after output was dropped and has not come yet.
    replay_wanted: bool,
}

/// What the owner of the window's channel has to do.
#[derive(Debug)]
pub enum Out {
    /// A frame for the window.
    Frame(Vec<u8>),
    /// A machine's whole snapshot as it now stands, or none when its line ended (this computer's
    /// only: another machine keeps its last one, shown as down).
    Sessions {
        machine: MachineId,
        snapshot: Option<Box<Snapshot>>,
    },
    /// What another machine's daemon said went wrong with a request of ours, already cleaned and
    /// led by the machine's name.
    Error { machine: MachineId, text: String },
    /// What another machine's daemon wants shown, already cleaned and led by the machine's name.
    Notice { machine: MachineId, text: String },
    /// A daemon decided a pane needs a person (it needs them, or it finished) and told this window.
    Attention {
        machine: MachineId,
        term: TermId,
        status: Status,
    },
    /// Text another machine's daemon asks to be put on this computer's clipboard (it was allowed to).
    Clipboard { machine: MachineId, text: String },
    /// Ask the machine's daemon for the pane's replay.
    WantReplay { machine: MachineId, term: TermId },
    /// The shared settings of this computer's daemon, as asked for or as they just changed.
    Settings(SharedSettings),
    /// A conversation of this computer's daemon changed: its open chat reads it again.
    ConversationChanged { conversation: u64, generation: u64 },
    /// A machine's answer to a question about a folder, already cleaned.
    Folders {
        machine: MachineId,
        tag: u64,
        listing: FolderListing,
    },
}

#[derive(Default)]
pub struct Streams {
    panes: HashMap<PaneKey, Pane>,
    last: HashMap<MachineId, Snapshot>,
    /// Output the daemons sent that the window has not been given yet.
    pending: BTreeMap<PaneKey, Vec<u8>>,
    /// When the oldest waiting output arrived; None while nothing waits.
    since: Option<Instant>,
}

impl Streams {
    /// Whether `machine`'s last list has the pane.
    fn lists(&self, machine: MachineId, term: TermId) -> bool {
        self.last
            .get(&machine)
            .is_some_and(|list| list.terms.contains_key(&term))
    }

    /// When the waiting output is due to be sent, if any waits.
    pub fn due(&self) -> Option<Instant> {
        self.since.map(|since| since + FLUSH_EVERY)
    }

    /// Drop everything held for a machine that is no longer one: its panes, snapshot and waiting output.
    pub fn forget(&mut self, machine: MachineId) {
        self.panes.retain(|(m, _), _| *m != machine);
        self.pending.retain(|(m, _), _| *m != machine);
        if self.pending.is_empty() {
            self.since = None;
        }
        self.last.remove(&machine);
    }

    /// What `machine` said, already vetted.
    pub fn on_message(&mut self, machine: MachineId, msg: ServerMsg, now: Instant) -> Vec<Out> {
        let mut out = Vec::new();
        match msg {
            ServerMsg::Replay { term, data, .. } => {
                if machine != MachineId::LOCAL {
                    if !self.lists(machine, term) {
                        return out;
                    }
                    // A screen's replay is never this large; a machine that sends one is made to
                    // start the pane over rather than being handed the window's channel.
                    if data.len() > HISTORY_LIMIT_BYTES {
                        self.drop_pane(machine, term, &mut out);
                        return out;
                    }
                }
                // A replay replaces the pane, so output held from before it lands first.
                if machine == MachineId::LOCAL {
                    self.flush_into(now, &mut out);
                } else {
                    self.take_pane(machine, term, &mut out);
                }
                self.emit(machine, term, data, true, &mut out);
            }
            ServerMsg::Output { term, data } => {
                // Another machine's output for a pane it never listed would add an entry per
                // number it makes up, so only the panes of its last list are tracked.
                if machine == MachineId::LOCAL || self.lists(machine, term) {
                    self.output(machine, term, data, now, &mut out);
                }
            }
            // Only another machine's: this computer's requests are answered on their own connection.
            ServerMsg::Error(text) if machine != MachineId::LOCAL => {
                out.push(Out::Error { machine, text });
            }
            ServerMsg::Clipboard { text, .. } if machine != MachineId::LOCAL => {
                out.push(Out::Clipboard { machine, text });
            }
            ServerMsg::Notice(text) if machine != MachineId::LOCAL => {
                out.push(Out::Notice { machine, text });
            }
            ServerMsg::Attention { term, status } => out.push(Out::Attention { machine, term, status }),
            // This computer's only: the settings shown are never another machine's.
            ServerMsg::Reply(Reply::Settings(settings)) | ServerMsg::SettingsChanged(settings)
                if machine == MachineId::LOCAL =>
            {
                out.push(Out::Settings(settings));
            }
            // This computer's only: the chats read here are never another machine's.
            ServerMsg::ConversationChanged { id, generation } if machine == MachineId::LOCAL => {
                out.push(Out::ConversationChanged {
                    conversation: id.0,
                    generation,
                });
            }
            ServerMsg::Reply(Reply::Folders { tag, listing }) => {
                out.push(Out::Folders { machine, tag, listing });
            }
            ServerMsg::State(snapshot) => self.publish(machine, snapshot, &mut out),
            ServerMsg::TermsChanged { changed, removed } => {
                let merged = self.last.get(&machine).cloned().map(|mut merged| {
                    for term in removed {
                        merged.terms.remove(&term);
                    }
                    for term in changed {
                        merged.terms.insert(term.id, term);
                    }
                    merged
                });
                // A delta before any whole snapshot has nothing to apply to; the next whole
                // one comes with the next reconnect.
                if let Some(merged) = merged {
                    // Each delta was capped on its own, but merged into the list they would grow
                    // it without end, so the merged list meets the caps of a whole one.
                    let merged = if machine == MachineId::LOCAL {
                        merged
                    } else {
                        match sanitize_snapshot(merged) {
                            Ok(merged) => merged,
                            Err(why) => {
                                tracing::debug!(
                                    why,
                                    "refused a pane delta from another machine: the list it makes is past the caps"
                                );
                                return out;
                            }
                        }
                    };
                    self.publish(machine, merged, &mut out);
                }
            }
            _ => {}
        }
        out
    }

    /// Give the window what waits. Another machine's output is held to its budget; what is left
    /// waits for the next flush.
    pub fn flush(&mut self, now: Instant) -> Vec<Out> {
        let mut out = Vec::new();
        self.flush_into(now, &mut out);
        out
    }

    /// A machine's line came up: its histories start over, but the sequence numbers run on (a
    /// view drops anything numbered at or below what it has seen, so a restart from 1 would
    /// leave it deaf). Returns the panes a view was watching, to be asked for again; the replay
    /// that comes puts each view back in step.
    pub fn line_up(&mut self, machine: MachineId) -> Vec<TermId> {
        self.pending.retain(|(m, _), _| *m != machine);
        if self.pending.is_empty() {
            self.since = None;
        }
        self.last.remove(&machine);
        self.panes
            .iter_mut()
            .filter(|((m, _), _)| *m == machine)
            .filter_map(|((_, term), pane)| {
                let watched = pane.complete;
                pane.history.clear();
                pane.complete = false;
                pane.replay_wanted = false;
                watched.then_some(*term)
            })
            .collect()
    }

    /// A machine's line ended: what it had said is given first. This computer's list is stale
    /// now and the window asks again; another machine keeps its last snapshot, shown as down.
    pub fn line_ended(&mut self, machine: MachineId, now: Instant) -> Vec<Out> {
        let mut out = Vec::new();
        let terms: Vec<TermId> = self
            .pending
            .keys()
            .filter(|(m, _)| *m == machine)
            .map(|(_, term)| *term)
            .collect();
        for term in terms {
            self.take_pane(machine, term, &mut out);
        }
        if self.pending.is_empty() {
            self.since = None;
        } else if self.since.is_none() {
            self.since = Some(now);
        }
        if machine == MachineId::LOCAL {
            out.push(Out::Sessions {
                machine,
                snapshot: None,
            });
        }
        out
    }

    /// The whole snapshot a machine last gave, for a window that was not there to hear it.
    pub fn last_snapshot(&self, machine: MachineId) -> Option<&Snapshot> {
        self.last.get(&machine)
    }

    /// What a view needs to open on a pane: the sequence number it ends at (`SEQ_BYTES`,
    /// big-endian), then everything shown so far. None when the pane has no usable history.
    pub fn history(&self, machine: MachineId, term: TermId) -> Option<Vec<u8>> {
        self.panes
            .get(&(machine, term))
            .filter(|pane| pane.complete)
            .map(|pane| {
                // One copy under the lock; live output waits for a memcpy, not for an encoding.
                let mut body = Vec::with_capacity(SEQ_BYTES + pane.history.len());
                body.extend_from_slice(&pane.seq.to_be_bytes());
                body.extend_from_slice(&pane.history);
                body
            })
    }

    fn output(
        &mut self,
        machine: MachineId,
        term: TermId,
        data: Vec<u8>,
        now: Instant,
        out: &mut Vec<Out>,
    ) {
        let key = (machine, term);
        let waiting = self.pending.entry(key).or_default();
        waiting.extend_from_slice(&data);
        self.since.get_or_insert(now);
        if machine == MachineId::LOCAL {
            if waiting.len() >= FLUSH_AT_BYTES {
                self.take_pane(machine, term, out);
                if self.pending.is_empty() {
                    self.since = None;
                }
            }
        } else if self.waiting_of(machine) > REMOTE_PENDING_MAX {
            self.drop_pane(machine, term, out);
        }
    }

    fn waiting_of(&self, machine: MachineId) -> usize {
        self.pending
            .range((machine, 0)..=(machine, TermId::MAX))
            .map(|(_, bytes)| bytes.len())
            .sum()
    }

    /// Output that cannot be given in time is dropped, and the pane is put right by a replay.
    fn drop_pane(&mut self, machine: MachineId, term: TermId, out: &mut Vec<Out>) {
        self.pending.remove(&(machine, term));
        if self.pending.is_empty() {
            self.since = None;
        }
        let pane = self.panes.entry((machine, term)).or_default();
        pane.history.clear();
        pane.complete = false;
        if !pane.replay_wanted {
            pane.replay_wanted = true;
            out.push(Out::WantReplay { machine, term });
        }
    }

    /// Everything waiting for one pane, now.
    fn take_pane(&mut self, machine: MachineId, term: TermId, out: &mut Vec<Out>) {
        if let Some(data) = self.pending.remove(&(machine, term)) {
            self.emit(machine, term, data, false, out);
        }
    }

    fn flush_into(&mut self, now: Instant, out: &mut Vec<Out>) {
        self.since = None;
        for ((machine, term), data) in std::mem::take(&mut self.pending) {
            if machine == MachineId::LOCAL {
                self.emit(machine, term, data, false, out);
            } else {
                self.pending.insert((machine, term), data);
            }
        }
        let remote: Vec<MachineId> = {
            let mut machines: Vec<MachineId> = self.pending.keys().map(|(m, _)| *m).collect();
            machines.dedup();
            machines
        };
        for machine in remote {
            let mut budget = REMOTE_BYTES_PER_FLUSH;
            let terms: Vec<TermId> = self
                .pending
                .range((machine, 0)..=(machine, TermId::MAX))
                .map(|((_, term), _)| *term)
                .collect();
            for term in terms {
                if budget == 0 {
                    break;
                }
                let Some(waiting) = self.pending.get_mut(&(machine, term)) else {
                    continue;
                };
                let take = waiting.len().min(budget);
                let data: Vec<u8> = waiting.drain(..take).collect();
                budget -= take;
                if waiting.is_empty() {
                    self.pending.remove(&(machine, term));
                }
                self.emit(machine, term, data, false, out);
            }
        }
        if !self.pending.is_empty() {
            self.since = Some(now);
        }
    }

    fn publish(&mut self, machine: MachineId, snapshot: Snapshot, out: &mut Vec<Out>) {
        self.panes
            .retain(|(m, term), _| *m != machine || snapshot.terms.contains_key(term));
        out.push(Out::Sessions {
            machine,
            snapshot: Some(Box::new(snapshot.clone())),
        });
        self.last.insert(machine, snapshot);
    }

    /// Adds the output to the pane's history and gives its frame for the window.
    fn emit(
        &mut self,
        machine: MachineId,
        term: TermId,
        data: Vec<u8>,
        replay: bool,
        out: &mut Vec<Out>,
    ) {
        let pane = self.panes.entry((machine, term)).or_default();
        pane.seq += 1;
        if replay {
            pane.history.clear();
            pane.complete = true;
            pane.replay_wanted = false;
        }
        // Output before a replay is not kept: without the replay it starts nowhere a view can use.
        if pane.complete {
            pane.history.extend_from_slice(&data);
        }
        if pane.history.len() > HISTORY_LIMIT_BYTES {
            pane.history.clear();
            pane.complete = false;
        }
        let mut frame = Vec::with_capacity(FRAME_HEADER_BYTES + data.len());
        frame.extend_from_slice(&machine.0.to_be_bytes());
        frame.extend_from_slice(&term.to_be_bytes());
        frame.extend_from_slice(&pane.seq.to_be_bytes());
        frame.push(u8::from(replay));
        frame.extend_from_slice(&data);
        out.push(Out::Frame(frame));
    }
}

#[cfg(test)]
mod tests;
