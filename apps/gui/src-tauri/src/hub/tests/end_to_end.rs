//! The whole host without a window: a real daemon for this computer and two real daemons behind
//! the fake `ssh` shim, the hub, and the same `react` the window's glue runs. Snapshots arrive
//! per machine, typing reaches the right machine's pane, a pane's history is built from a replay,
//! and after one machine's link is killed it reconnects, its pane is replayed again and the others
//! are never touched.

use super::*;
use crate::streams::{SEQ_BYTES, Streams};
use crate::terminal_stream::{Reaction, Wire, flush, react};
use maitty_desktop_client::PaneCommand;
use maitty_machines::Machine;
use maitty_shared::protocol::{ClientMsg, Command, FolderListing, Snapshot, TermId};
use std::collections::BTreeMap;
use std::sync::Arc;

const HERE: MachineId = MachineId::LOCAL;
const ALPHA: MachineId = MachineId(1);
const BETA: MachineId = MachineId(2);

struct Frame {
    machine: u16,
    term: TermId,
    seq: u64,
    replay: bool,
    bytes: Vec<u8>,
}

/// What the window's glue holds, driven by a test instead of a window.
struct Host {
    hub: Hub,
    streams: Streams,
    wire: Wire,
    states: Vec<(u16, &'static str)>,
    frames: Vec<Frame>,
    sessions: BTreeMap<u16, Option<Snapshot>>,
    folders: Vec<(u16, u64, FolderListing)>,
    errors: Vec<(u16, String)>,
}

impl Host {
    fn new(here: &Sandbox, alpha: &Sandbox, beta: &Sandbox) -> Host {
        let machine = Machine::from_def(&machine("here", here, "proxy"), &shim());
        let local: LocalDial = Arc::new(move || {
            let machine = machine.clone();
            Box::pin(async move { maitty_machines::open_remote(&machine, dial()?).await })
        });
        let defs = [machine_def("alpha", alpha), machine_def("beta", beta)];
        let hub = Hub::start(&defs, &shim(), ids_file(here), quick(WAIT), Some(local));
        let wire = Wire::reaching(hub.handle());
        Host {
            hub,
            streams: Streams::default(),
            wire,
            states: Vec::new(),
            frames: Vec::new(),
            sessions: BTreeMap::new(),
            folders: Vec::new(),
            errors: Vec::new(),
        }
    }

    async fn step(&mut self) {
        let handle = self.hub.handle();
        let wait = self
            .streams
            .due()
            .map_or(Duration::from_millis(100), |at| {
                at.saturating_duration_since(Instant::now())
            })
            .max(Duration::from_millis(1));
        let reactions = match tokio::time::timeout(wait, self.hub.next()).await {
            Ok(update) => react(&mut self.streams, &handle, update, Instant::now()),
            Err(_) => flush(&mut self.streams, &handle, Instant::now()),
        };
        for reaction in reactions {
            match reaction {
                Reaction::State(told) => self.states.push((told.machine, told.state)),
                Reaction::Frame(frame) => self.frames.push(decode(&frame)),
                Reaction::Error { machine, text } => self.errors.push((machine.0, text)),
                Reaction::Removed { .. }
                | Reaction::Notice { .. }
                | Reaction::Attention { .. }
                | Reaction::Clipboard { .. }
                | Reaction::ConversationChanged { .. }
                | Reaction::Settings(_) => {}
                Reaction::Sessions { machine, snapshot } => {
                    self.sessions.insert(machine.0, snapshot.map(|s| *s));
                }
                Reaction::Folders { machine, tag, listing } => self.folders.push((machine.0, tag, listing)),
            }
        }
    }

    async fn until(&mut self, what: &str, done: impl Fn(&Host) -> bool) {
        let waited = tokio::time::timeout(WAIT, async {
            while !done(self) {
                self.step().await;
            }
        })
        .await;
        assert!(
            waited.is_ok(),
            "timed out waiting for {what}; states: {:?}",
            self.states
        );
    }

    fn kinds(&self, machine: MachineId) -> Vec<&'static str> {
        self.states
            .iter()
            .filter(|(m, _)| *m == machine.0)
            .map(|(_, kind)| *kind)
            .collect()
    }

    fn pane_of(&self, machine: MachineId) -> Option<TermId> {
        self.sessions
            .get(&machine.0)?
            .as_ref()?
            .terms
            .keys()
            .next()
            .copied()
    }

    /// What the window would open a pane on: the sequence number, then everything shown.
    fn history(&self, machine: MachineId, term: TermId) -> Option<(u64, String)> {
        let body = self.streams.history(machine, term)?;
        let seq = u64::from_be_bytes(body[..SEQ_BYTES].try_into().unwrap());
        Some((
            seq,
            String::from_utf8_lossy(&body[SEQ_BYTES..]).into_owned(),
        ))
    }

    fn type_into(&self, machine: MachineId, term: TermId, line: &str) {
        let machine = (machine != HERE).then_some(machine.0);
        self.wire
            .command(
                machine,
                term,
                PaneCommand::Input(format!("{line}\r").into_bytes()),
            )
            .expect("the machine takes typing");
    }
}

fn machine_def(name: &str, sandbox: &Sandbox) -> MachineDef {
    machine(name, sandbox, "proxy")
}

fn decode(frame: &[u8]) -> Frame {
    Frame {
        machine: u16::from_be_bytes(frame[0..2].try_into().unwrap()),
        term: u32::from_be_bytes(frame[2..6].try_into().unwrap()),
        seq: u64::from_be_bytes(frame[6..14].try_into().unwrap()),
        replay: frame[14] == 1,
        bytes: frame[crate::streams::FRAME_HEADER_BYTES..].to_vec(),
    }
}

fn occurrences(text: &str, token: &str) -> usize {
    text.matches(token).count()
}

#[tokio::test]
async fn three_machines_snapshots_typing_and_a_replay_after_one_reconnects() {
    let (mut here, mut alpha, mut beta) = (Sandbox::new(), Sandbox::new(), Sandbox::new());
    for sandbox in [&mut here, &mut alpha, &mut beta] {
        sandbox.start_daemon();
    }
    let mut host = Host::new(&here, &alpha, &beta);

    // Every machine connects and gives its snapshot, this computer's as machine 0.
    host.until("every machine connected, each with a snapshot", |h| {
        [HERE, ALPHA, BETA].iter().all(|m| {
            h.kinds(*m).last() == Some(&"connected")
                && h.sessions.get(&m.0).is_some_and(Option::is_some)
        })
    })
    .await;

    // A pane on each, made through the same wire the window uses; the machines number them alike.
    let handle = host.hub.handle();
    for machine in [HERE, ALPHA, BETA] {
        handle
            .send(Outgoing {
                machine,
                msg: ClientMsg::Command(Command::NewWorkspace {
                    cwd: None,
                    name: None,
                    cmd: None,
                }),
            })
            .expect("the line is up");
    }
    host.until("a pane on each machine", |h| {
        [HERE, ALPHA, BETA].iter().all(|m| h.pane_of(*m).is_some())
    })
    .await;
    let panes = [
        (HERE, host.pane_of(HERE).unwrap()),
        (ALPHA, host.pane_of(ALPHA).unwrap()),
        (BETA, host.pane_of(BETA).unwrap()),
    ];

    // A view opening on each pane finds no history and asks for the replay; it comes as that
    // machine's frame.
    for (machine, term) in panes {
        assert!(
            host.history(machine, term).is_none(),
            "nothing before a replay"
        );
        let machine_arg = (machine != HERE).then_some(machine.0);
        host.wire
            .command(machine_arg, term, PaneCommand::WantReplay)
            .expect("the line is up");
    }
    host.until("a history for each pane", |h| {
        panes.iter().all(|(m, t)| h.history(*m, *t).is_some())
    })
    .await;

    // Typing reaches the pane of the machine it was for, and only that one.
    let tokens = [
        (HERE, "ECHO_HERE_91"),
        (ALPHA, "ECHO_ALPHA_92"),
        (BETA, "ECHO_BETA_93"),
    ];
    for ((machine, term), (_, token)) in panes.iter().zip(tokens) {
        host.type_into(*machine, *term, &format!("echo {token}"));
    }
    host.until("each shell to print its own token", |h| {
        panes.iter().zip(tokens).all(|((m, t), (_, token))| {
            h.history(*m, *t)
                .is_some_and(|(_, text)| occurrences(&text, token) >= 2)
        })
    })
    .await;
    for ((machine, term), (_, token)) in panes.iter().zip(tokens) {
        let (_, text) = host.history(*machine, *term).unwrap();
        for (_, other) in tokens.iter().filter(|(_, other)| *other != token) {
            assert_eq!(
                occurrences(&text, other),
                0,
                "{other} reached machine {}; its pane read: {text:?}",
                machine.0
            );
        }
    }
    for frame in &host.frames {
        let text = String::from_utf8_lossy(&frame.bytes);
        for (machine, token) in tokens {
            if text.contains(token) {
                assert_eq!(
                    frame.machine, machine.0,
                    "{token} came in another machine's frame"
                );
            }
        }
    }

    // One machine's link dies. It reconnects, its watched pane is asked for again at once and
    // replayed, the sequence runs on, and the other machines never notice.
    let (alpha_term, alpha_token) = (panes[1].1, tokens[1].1);
    let (seq_before, _) = host.history(ALPHA, alpha_term).unwrap();
    let frames_before = host.frames.len();
    alpha.kill_last_connection();
    host.until("alpha reconnecting and connected again", |h| {
        h.kinds(ALPHA).ends_with(&["reconnecting", "connected"])
    })
    .await;
    host.until("alpha's pane replayed with what it had printed", |h| {
        h.frames[frames_before..].iter().any(|f| {
            f.machine == ALPHA.0
                && f.term == alpha_term
                && f.replay
                && String::from_utf8_lossy(&f.bytes).contains(alpha_token)
        })
    })
    .await;
    let (seq_after, text) = host.history(ALPHA, alpha_term).unwrap();
    assert!(
        seq_after > seq_before,
        "the sequence ran on: {seq_before} then {seq_after}"
    );
    assert!(
        occurrences(&text, alpha_token) >= 2,
        "history rebuilt from the replay"
    );
    assert_eq!(
        host.pane_of(ALPHA),
        Some(alpha_term),
        "the same pane, still running"
    );
    assert_eq!(host.kinds(HERE), ["connecting", "connected"]);
    assert_eq!(host.kinds(BETA), ["connecting", "connected"]);
    assert_eq!(
        host.kinds(ALPHA),
        ["connecting", "connected", "reconnecting", "connected"]
    );
    assert_eq!(
        alpha.calls_made(),
        2,
        "one dial for the line, one to redial at once"
    );
    assert_eq!(beta.calls_made(), 1);

    // Each machine's pane was numbered on its own, and only ever upwards, through all of it.
    for machine in [HERE, ALPHA, BETA] {
        let seqs: Vec<u64> = host
            .frames
            .iter()
            .filter(|f| f.machine == machine.0)
            .map(|f| f.seq)
            .collect();
        assert!(seqs.len() > 1, "machine {} gave several frames", machine.0);
        assert!(
            seqs.windows(2).all(|pair| pair[0] < pair[1]),
            "machine {} numbered {seqs:?}",
            machine.0
        );
    }

    // Typing works on the machine that came back, and on the others as before.
    for (machine, term) in [panes[1], panes[2], panes[0]] {
        host.type_into(machine, term, "echo AFTER_9000");
    }
    host.until("the shells to print it", |h| {
        panes.iter().all(|(m, t)| {
            h.history(*m, *t)
                .is_some_and(|(_, text)| occurrences(&text, "AFTER_9000") >= 2)
        })
    })
    .await;
}

#[tokio::test]
async fn each_machine_lists_the_folders_inside_a_folder_and_the_answer_names_the_question() {
    let (mut here, mut alpha, mut beta) = (Sandbox::new(), Sandbox::new(), Sandbox::new());
    for sandbox in [&mut here, &mut alpha, &mut beta] {
        sandbox.start_daemon();
    }
    let mut host = Host::new(&here, &alpha, &beta);
    host.until("every machine connected", |h| {
        [HERE, ALPHA, BETA].iter().all(|m| h.kinds(*m).last() == Some(&"connected"))
    })
    .await;

    // Each sandbox has a `data` folder; ask each machine about its own, by tag.
    let ask = |host: &Host, machine: MachineId, sandbox: &Sandbox, tag: u64| {
        let machine = (machine != HERE).then_some(machine.0);
        host.wire
            .list_folders(machine, &sandbox.dir.to_string_lossy(), tag)
            .expect("the machine takes the question");
    };
    ask(&host, HERE, &here, 1);
    ask(&host, ALPHA, &alpha, 2);
    ask(&host, BETA, &beta, 3);
    host.until("all three answered", |h| h.folders.len() >= 3).await;

    for (machine, tag, sandbox) in [(HERE, 1, &here), (ALPHA, 2, &alpha), (BETA, 3, &beta)] {
        let (_, _, listing) = host
            .folders
            .iter()
            .find(|(m, t, _)| *m == machine.0 && *t == tag)
            .unwrap_or_else(|| panic!("no answer from machine {}", machine.0));
        assert!(listing.exists, "{listing:?}");
        assert!(listing.entries.iter().any(|e| e.name == "data"), "{listing:?}");
        assert_eq!(listing.path, sandbox.dir.to_string_lossy().replace('\\', "/"));
    }

    // A folder that is not there is an answer too, not an error.
    let missing = here.dir.join("nope");
    host.wire.list_folders(None, &missing.to_string_lossy(), 4).unwrap();
    host.until("the missing folder answered", |h| h.folders.iter().any(|(_, t, _)| *t == 4)).await;
    assert!(!host.folders.iter().find(|(_, t, _)| *t == 4).unwrap().2.exists);
}

#[tokio::test]
async fn a_daemon_takes_a_message_to_hold_for_a_pane_from_the_window_and_refuses_one_for_no_pane() {
    let (mut here, mut alpha, mut beta) = (Sandbox::new(), Sandbox::new(), Sandbox::new());
    for sandbox in [&mut here, &mut alpha, &mut beta] {
        sandbox.start_daemon();
    }
    let mut host = Host::new(&here, &alpha, &beta);
    host.until("every machine connected", |h| {
        [HERE, ALPHA, BETA].iter().all(|m| h.kinds(*m).last() == Some(&"connected"))
    })
    .await;
    let handle = host.hub.handle();
    for machine in [HERE, ALPHA] {
        handle
            .send(Outgoing {
                machine,
                msg: ClientMsg::Command(Command::NewWorkspace { cwd: None, name: None, cmd: None }),
            })
            .expect("the line is up");
    }
    host.until("a pane here and on alpha", |h| h.pane_of(HERE).is_some() && h.pane_of(ALPHA).is_some()).await;
    let (ours, theirs) = (host.pane_of(HERE).unwrap(), host.pane_of(ALPHA).unwrap());
    for (machine, term) in [(None, ours), (Some(ALPHA.0), theirs)] {
        host.wire.command(machine, term, PaneCommand::WantReplay).expect("the line is up");
    }
    host.until("a history for each pane", |h| {
        h.history(HERE, ours).is_some() && h.history(ALPHA, theirs).is_some()
    })
    .await;

    // No agent runs in these panes, so the message is held and never typed: the shell shows none of it.
    let hold = |machine: Option<u16>, term: TermId| {
        host.wire
            .command(machine, term, PaneCommand::SendWhenReady("held-for-the-agent".into()))
            .expect("the machine takes the command");
    };
    hold(None, ours);
    hold(Some(ALPHA.0), theirs);
    hold(Some(ALPHA.0), theirs + 1000);
    host.until("the refusal for the pane that is not there", |h| !h.errors.is_empty()).await;
    assert_eq!(host.errors.len(), 1, "only the missing pane is refused: {:?}", host.errors);
    assert_eq!(host.errors[0].0, ALPHA.0);

    // The lines still carry typing after it, in order, and the held text was not typed into a shell.
    for (machine, term) in [(HERE, ours), (ALPHA, theirs)] {
        host.type_into(machine, term, "echo AFTER_HELD_7");
    }
    host.until("both panes ran what was typed after", |h| {
        [(HERE, ours), (ALPHA, theirs)]
            .iter()
            .all(|(m, t)| h.history(*m, *t).is_some_and(|(_, text)| occurrences(&text, "AFTER_HELD_7") >= 2))
    })
    .await;
    for (machine, term) in [(HERE, ours), (ALPHA, theirs)] {
        let (_, text) = host.history(machine, term).unwrap();
        assert!(!text.contains("held-for-the-agent"), "{text}");
    }
    assert_eq!(host.errors.len(), 1, "{:?}", host.errors);
}
