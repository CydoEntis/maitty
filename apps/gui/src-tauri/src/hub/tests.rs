//! The hub against the fake `ssh` shim: connect, drop, reconnect with backoff, turn a different
//! daemon away, and say what a person has to do. Each machine's shim settings ride in its
//! `ssh_args`, so several live in one process.

use super::*;
use maitty_machines::link::Backoff;
use maitty_machines::link_failure::{FIX_LOGIN, FIX_UPDATE};
use maitty_test_support::{Sandbox, fake_ssh};
use std::time::Duration;

const WAIT: Duration = Duration::from_secs(60);
const BOX: MachineId = MachineId(1);

fn quick(connect: Duration) -> Timing {
    Timing {
        connect,
        backoff: Backoff {
            first: Duration::from_millis(10),
            most: Duration::from_millis(40),
        },
    }
}

fn shim() -> String {
    fake_ssh().to_string_lossy().into_owned()
}

fn machine(name: &str, sandbox: &Sandbox, mode: &str) -> MachineDef {
    MachineDef {
        name: name.into(),
        target: name.into(),
        ssh_args: sandbox.shim_args(mode),
        ..MachineDef::default()
    }
}

fn ids_file(sandbox: &Sandbox) -> PathBuf {
    MachineIds::file_in(&sandbox.data())
}

/// A hub, and what it has told so far.
struct Watch {
    hub: Hub,
    states: Vec<(MachineId, ConnectionState)>,
    msgs: Vec<(MachineId, ServerMsg)>,
    up: Vec<MachineId>,
    ended: Vec<MachineId>,
}

impl Watch {
    fn new(defs: &[MachineDef], sandbox: &Sandbox, timing: Timing) -> Watch {
        Watch {
            hub: Hub::start(defs, &shim(), ids_file(sandbox), timing, None),
            states: Vec::new(),
            msgs: Vec::new(),
            up: Vec::new(),
            ended: Vec::new(),
        }
    }

    async fn step(&mut self) {
        match self.hub.next().await {
            Update::State { machine, state, .. } => self.states.push((machine, state)),
            Update::Messages { machine, msgs } => {
                self.msgs.extend(msgs.into_iter().map(|msg| (machine, msg)));
            }
            Update::LineUp { machine } => self.up.push(machine),
            Update::LineEnded { machine } => self.ended.push(machine),
        }
    }

    async fn until(&mut self, what: &str, done: impl Fn(&Watch) -> bool) {
        let waited = tokio::time::timeout(WAIT, async {
            while !done(self) {
                self.step().await;
            }
        })
        .await;
        assert!(
            waited.is_ok(),
            "timed out waiting for {what}; states: {:?}",
            self.kinds(BOX)
        );
    }

    fn kinds(&self, machine: MachineId) -> Vec<&'static str> {
        self.states
            .iter()
            .filter(|(m, _)| *m == machine)
            .map(|(_, state)| state.kind())
            .collect()
    }

    fn is(&self, machine: MachineId, kind: &str) -> bool {
        self.kinds(machine).last() == Some(&kind)
    }

    fn state_of(&self, machine: MachineId) -> Option<&ConnectionState> {
        self.states
            .iter()
            .rev()
            .find(|(m, _)| *m == machine)
            .map(|(_, state)| state)
    }

    fn snapshots(&self, machine: MachineId) -> usize {
        self.msgs
            .iter()
            .filter(|(m, msg)| *m == machine && matches!(msg, ServerMsg::State(_)))
            .count()
    }
}

#[tokio::test]
async fn a_machine_that_answers_connects_and_its_snapshot_comes_through() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    watch
        .until("connected and a snapshot", |w| {
            w.is(BOX, "connected") && w.snapshots(BOX) > 0
        })
        .await;
    assert_eq!(watch.kinds(BOX), ["connecting", "connected"]);
    assert_eq!(watch.hub.name_of(BOX), Some("box"));
    assert!(
        ids_file(&sandbox).is_file(),
        "the first snapshot records which daemon answers at the name"
    );
}

#[tokio::test]
async fn killing_the_ssh_process_shows_reconnecting_then_connected_and_the_pane_comes_again() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    watch
        .until("connected", |w| {
            w.is(BOX, "connected") && w.snapshots(BOX) > 0
        })
        .await;
    let before = watch.snapshots(BOX);

    sandbox.kill_last_connection();
    watch
        .until("reconnecting and connected again", |w| {
            w.kinds(BOX).ends_with(&["reconnecting", "connected"])
        })
        .await;
    watch
        .until("a fresh snapshot on the new line", |w| {
            w.snapshots(BOX) > before
        })
        .await;
    assert_eq!(
        watch.kinds(BOX),
        ["connecting", "connected", "reconnecting", "connected"]
    );
    assert_eq!(watch.ended, [BOX], "the dropped line was reported once");
    assert_eq!(
        sandbox.calls_made(),
        2,
        "one dial for the line, one to redial at once"
    );
}

#[tokio::test]
async fn a_failed_login_needs_login_with_the_fix_and_the_retries_are_quiet() {
    let sandbox = Sandbox::new();
    let mut watch = Watch::new(
        &[machine("box", &sandbox, "auth-fail")],
        &sandbox,
        quick(WAIT),
    );
    watch
        .until("needs-login", |w| w.is(BOX, "needs-login"))
        .await;
    assert_eq!(
        watch.state_of(BOX),
        Some(&ConnectionState::NeedsLogin {
            why: FIX_LOGIN.into()
        })
    );
    let more = async {
        while sandbox.calls_made() < 3 {
            // Quiet retries tell the host nothing, so do not wait for news.
            let _ = tokio::time::timeout(Duration::from_millis(50), watch.step()).await;
        }
    };
    tokio::time::timeout(WAIT, more)
        .await
        .expect("it keeps dialling");
    assert_eq!(watch.kinds(BOX), ["connecting", "needs-login"]);
}

#[tokio::test]
async fn a_daemon_of_another_version_needs_an_update_with_the_fix() {
    let sandbox = Sandbox::new();
    let mut watch = Watch::new(
        &[machine("box", &sandbox, "mismatch")],
        &sandbox,
        quick(WAIT),
    );
    watch
        .until("needs-update", |w| w.is(BOX, "needs-update"))
        .await;
    assert_eq!(
        watch.state_of(BOX),
        Some(&ConnectionState::NeedsUpdate {
            why: FIX_UPDATE.into()
        })
    );
}

#[tokio::test]
async fn a_machine_that_does_not_answer_is_unreachable_and_the_hung_ssh_is_not_waited_for() {
    let sandbox = Sandbox::new();
    let mut watch = Watch::new(
        &[machine("box", &sandbox, "hang")],
        &sandbox,
        quick(Duration::from_millis(300)),
    );
    watch
        .until("unreachable", |w| w.is(BOX, "unreachable"))
        .await;
    let Some(ConnectionState::Unreachable { why, .. }) = watch.state_of(BOX) else {
        panic!("{:?}", watch.state_of(BOX));
    };
    assert!(why.starts_with("no answer within"), "{why}");
}

#[tokio::test]
async fn a_different_daemon_at_a_known_name_is_turned_away_and_says_nothing_that_counts() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    // The name was first seen with another daemon.
    let mut known = MachineIds::default();
    check_daemon_id(&mut known, "box", Some("aaaaaaaa11111111aaaaaaaa11111111")).unwrap();
    known.save(&ids_file(&sandbox));

    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    watch
        .until("turned away", |w| w.is(BOX, "unreachable"))
        .await;
    let Some(ConnectionState::Unreachable { why, .. }) = watch.state_of(BOX) else {
        panic!("{:?}", watch.state_of(BOX));
    };
    assert!(
        why.contains("a different machine answers at box"),
        "the reason is the refusal's own words: {why}"
    );
    assert_eq!(
        watch.snapshots(BOX),
        0,
        "nothing the wrong daemon said was kept"
    );

    // Parked: it is not dialled again until a person asks.
    let parked = sandbox.calls_made();
    let quiet = tokio::time::timeout(Duration::from_millis(500), async {
        loop {
            watch.step().await;
        }
    })
    .await;
    assert!(quiet.is_err(), "the watch only ends by timing out");
    assert_eq!(sandbox.calls_made(), parked, "no dial while parked");
    assert_eq!(watch.snapshots(BOX), 0);
}

#[tokio::test]
async fn with_no_machine_nothing_starts() {
    let sandbox = Sandbox::new();
    let hub = Hub::start(&[], &shim(), ids_file(&sandbox), quick(WAIT), None);
    assert!(!hub.any());
}

#[tokio::test]
async fn a_machine_that_is_not_one_of_ours_is_not_sent_to() {
    let sandbox = Sandbox::new();
    let hub = Hub::start(&[], &shim(), ids_file(&sandbox), quick(WAIT), None);
    let sent = hub.send(Outgoing {
        machine: MachineId(7),
        msg: maitty_shared::protocol::ClientMsg::WantReplay { term: 1 },
    });
    assert!(sent.is_err());
}

#[test]
fn the_window_is_told_the_kind_and_what_a_person_must_do() {
    let at = std::time::Instant::now();
    let told = |state| describe(MachineId(2), "box", &state);
    assert_eq!(
        told(ConnectionState::Connected),
        MachineState {
            machine: 2,
            name: "box".into(),
            state: "connected",
            why: None,
            attempt: None
        }
    );
    assert_eq!(
        told(ConnectionState::Reconnecting {
            attempt: 2,
            since: at
        })
        .attempt,
        Some(2)
    );
    for (state, kind) in [
        (
            ConnectionState::NeedsLogin {
                why: "log in".into(),
            },
            "needs-login",
        ),
        (
            ConnectionState::NeedsUpdate {
                why: "update".into(),
            },
            "needs-update",
        ),
        (
            ConnectionState::Unreachable {
                since: at,
                why: "no route".into(),
            },
            "unreachable",
        ),
    ] {
        let told = told(state);
        assert_eq!(told.state, kind);
        assert!(told.why.is_some(), "{kind} says what to do");
    }
    assert_eq!(told(ConnectionState::Connecting).state, "connecting");
}

mod vetting {
    use super::*;
    use maitty_shared::protocol::{Snapshot, TermInfo};

    /// A pane as a daemon encodes it, with the fields that have no default.
    fn pane(id: u32, title: &str, cols: u16) -> TermInfo {
        serde_json::from_value(serde_json::json!({
            "id": id, "cols": cols, "rows": 24, "title": title, "process": "bash",
            "agent": null, "status": "Idle", "cwd": "/home/me", "summary": "", "name": "n",
            "model": "", "said": "", "branch": null, "linked": false, "subagents": [],
            "root": null, "top": null, "since": 0, "asleep": false, "win32_input": false
        }))
        .expect("a pane")
    }

    /// A hub with one machine called `box` that never gets a line: only `admit` is exercised.
    async fn hub_with_box(sandbox: &Sandbox, opted_in: bool) -> Hub {
        let mut def = machine("box", sandbox, "hang");
        def.clipboard = opted_in;
        Hub::start(&[def], &shim(), ids_file(sandbox), quick(WAIT), None)
    }

    #[tokio::test]
    async fn what_a_remote_may_do_to_this_computer_is_off_unless_the_machine_opts_in() {
        let sandbox = Sandbox::new();
        let mut hub = hub_with_box(&sandbox, false).await;
        let clipboard = ServerMsg::Clipboard {
            term: 1,
            text: "rm -rf".into(),
        };
        assert!(hub.admit(BOX, clipboard).is_none());
        for msg in [ServerMsg::Raise, ServerMsg::Notice("hi".into())] {
            assert!(hub.admit(BOX, msg).is_none());
        }
        let worktree = ServerMsg::AskRemoveWorktree {
            path: "/home/me/x".into(),
            name: "x".into(),
            why: "not merged".into(),
        };
        assert!(hub.admit(BOX, worktree).is_none());

        let mut opted_in = hub_with_box(&sandbox, true).await;
        let clipboard = ServerMsg::Clipboard {
            term: 1,
            text: "hello".into(),
        };
        assert!(opted_in.admit(BOX, clipboard).is_some());
    }

    #[tokio::test]
    async fn an_answer_nobody_asked_for_is_ignored() {
        let sandbox = Sandbox::new();
        let mut hub = hub_with_box(&sandbox, false).await;
        assert!(hub.admit(BOX, ServerMsg::Error("boom".into())).is_none());
        assert!(
            hub.admit(BOX, ServerMsg::Reply(maitty_shared::protocol::Reply::Ok))
                .is_none()
        );
    }

    #[tokio::test]
    async fn a_remote_snapshot_is_cleaned_before_the_host_keeps_it() {
        let sandbox = Sandbox::new();
        let mut hub = hub_with_box(&sandbox, false).await;
        let mut snapshot = Snapshot::default();
        snapshot
            .terms
            .insert(3, pane(3, "\u{1b}[2Jtitle\u{202e}", u16::MAX));
        let Some(ServerMsg::State(kept)) = hub.admit(BOX, ServerMsg::State(snapshot)) else {
            panic!("a snapshot is let through");
        };
        let term = &kept.terms[&3];
        assert_eq!(term.title, "[2Jtitle");
        assert_eq!(term.cols, 1000);
    }

    #[tokio::test]
    async fn a_message_from_a_machine_that_is_not_ours_is_ignored() {
        let sandbox = Sandbox::new();
        let mut hub = hub_with_box(&sandbox, false).await;
        assert!(hub.admit(MachineId(9), ServerMsg::Bye).is_none());
    }
}

mod local_line {
    use super::*;
    use maitty_machines::Machine;
    use maitty_shared::protocol::{ClientMsg, Command};
    use std::sync::Arc;

    /// This computer's line, made through the shim to the sandbox's daemon: the hub is given
    /// halves and does not care how they were made.
    fn through_the_shim(sandbox: &Sandbox) -> LocalDial {
        let machine = Machine::from_def(&machine("here", sandbox, "proxy"), &shim());
        Arc::new(move || {
            let machine = machine.clone();
            Box::pin(async move { maitty_machines::open_remote(&machine, dial()?).await })
        })
    }

    fn here(sandbox: &Sandbox) -> Watch {
        Watch {
            hub: Hub::start(
                &[],
                &shim(),
                ids_file(sandbox),
                quick(WAIT),
                Some(through_the_shim(sandbox)),
            ),
            states: Vec::new(),
            msgs: Vec::new(),
            up: Vec::new(),
            ended: Vec::new(),
        }
    }

    #[tokio::test]
    async fn this_computers_line_is_machine_zero_and_comes_up_before_anything_it_says() {
        let mut sandbox = Sandbox::new();
        sandbox.start_daemon();
        let mut watch = here(&sandbox);
        assert!(watch.hub.any());
        watch
            .until("this computer's snapshot", |w| {
                w.is(MachineId::LOCAL, "connected") && w.snapshots(MachineId::LOCAL) > 0
            })
            .await;
        assert_eq!(
            watch.kinds(MachineId::LOCAL),
            ["connecting", "connected"],
            "named local, id 0"
        );
        assert_eq!(watch.up, [MachineId::LOCAL]);
        assert_eq!(watch.hub.name_of(MachineId::LOCAL), Some("local"));
        assert!(watch.hub.handle().is_up(MachineId::LOCAL));
        assert!(
            !ids_file(&sandbox).exists(),
            "this computer's daemon is not checked against a recorded id"
        );
    }

    #[tokio::test]
    async fn the_handle_reaches_a_line_that_is_up_and_refuses_one_that_is_not() {
        let mut sandbox = Sandbox::new();
        sandbox.start_daemon();
        let mut watch = here(&sandbox);
        let handle = watch.hub.handle();
        let early = handle.send(Outgoing {
            machine: MachineId::LOCAL,
            msg: ClientMsg::WantReplay { term: 1 },
        });
        assert!(early.is_err(), "no line yet, so nothing is sent");
        watch
            .until("this computer's snapshot", |w| {
                w.is(MachineId::LOCAL, "connected") && w.snapshots(MachineId::LOCAL) > 0
            })
            .await;
        let sent = handle.send(Outgoing {
            machine: MachineId::LOCAL,
            msg: ClientMsg::Command(Command::NewWorkspace {
                cwd: None,
                name: None,
                cmd: None,
            }),
        });
        assert!(sent.is_ok());
        watch
            .until("a pane made through the handle", |w| {
                w.msgs.iter().any(|(m, msg)| {
                    *m == MachineId::LOCAL
                        && (matches!(msg, ServerMsg::State(s) if !s.terms.is_empty())
                            || matches!(msg, ServerMsg::TermsChanged { changed, .. } if !changed.is_empty()))
                })
            })
            .await;
        assert!(
            handle
                .send(Outgoing {
                    machine: MachineId(5),
                    msg: ClientMsg::WantReplay { term: 1 },
                })
                .is_err(),
            "a machine with no line takes nothing"
        );
    }

    #[tokio::test]
    async fn a_dropped_line_of_this_computer_is_made_again() {
        let mut sandbox = Sandbox::new();
        sandbox.start_daemon();
        let mut watch = here(&sandbox);
        watch
            .until("connected", |w| w.is(MachineId::LOCAL, "connected"))
            .await;
        sandbox.kill_last_connection();
        watch
            .until("reconnecting and connected again", |w| {
                w.kinds(MachineId::LOCAL)
                    .ends_with(&["reconnecting", "connected"])
            })
            .await;
        assert_eq!(watch.ended, [MachineId::LOCAL]);
        assert_eq!(watch.up, [MachineId::LOCAL, MachineId::LOCAL]);
        assert!(!watch.hub.handle().is_up(MachineId(3)));
    }
}
