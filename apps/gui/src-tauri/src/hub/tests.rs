//! The hub against the fake `ssh` shim: connect, drop, reconnect with backoff, turn a different
//! daemon away, and say what a person has to do. Each machine's shim settings ride in its
//! `ssh_args`, so several live in one process.

use super::*;
use maitty_machines::link::Backoff;
use maitty_machines::machine_config::Grant;
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
    removed: Vec<MachineId>,
}

impl Watch {
    fn new(defs: &[MachineDef], sandbox: &Sandbox, timing: Timing) -> Watch {
        Watch {
            hub: Hub::start(defs, &shim(), ids_file(sandbox), timing, None),
            states: Vec::new(),
            msgs: Vec::new(),
            up: Vec::new(),
            ended: Vec::new(),
            removed: Vec::new(),
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
            Update::Removed { machine } => self.removed.push(machine),
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

#[test]
fn a_fix_names_the_machine_so_the_command_can_be_run_as_it_stands() {
    let told = describe(
        MachineId(2),
        "build",
        &ConnectionState::NeedsUpdate {
            why: maitty_machines::link_failure::FIX_INSTALL.into(),
        },
    );
    let why = told.why.expect("a fix");
    assert!(why.contains("`maitty machine update build --install`"), "{why}");
    assert!(!why.contains("<name>"), "{why}");
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
    assert!(
        watch.hub.handle().is_refused(BOX),
        "the window is told refused, not just unreachable"
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

    // A person asking again clears the refusal and dials.
    watch.hub.handle().retry("BOX");
    let dialled = async {
        while sandbox.calls_made() == parked {
            let _ = tokio::time::timeout(Duration::from_millis(50), watch.step()).await;
        }
    };
    tokio::time::timeout(WAIT, dialled)
        .await
        .expect("retry dials a parked machine");
}

#[tokio::test]
async fn removing_a_machine_stops_it_and_forgets_it_and_its_daemon_id() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    watch
        .until("connected and a snapshot", |w| {
            w.is(BOX, "connected") && w.snapshots(BOX) > 0
        })
        .await;
    assert!(MachineIds::load(&ids_file(&sandbox)).forget("box"));

    let handle = watch.hub.handle();
    handle.remove("Box");
    watch
        .until("the machine is removed", |w| w.removed == [BOX])
        .await;
    assert!(!handle.is_up(BOX));
    assert_eq!(handle.name_of(BOX), None);
    assert!(!watch.hub.retry(BOX), "nothing supervises it any more");
    assert!(
        !MachineIds::load(&ids_file(&sandbox)).forget("box"),
        "the daemon id recorded for the name is gone"
    );
}

#[tokio::test]
async fn a_machine_added_while_the_app_runs_connects_under_a_new_number() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let mut watch = Watch::new(&[], &sandbox, quick(WAIT));
    assert!(!watch.hub.any());

    watch.hub.handle().add(machine("late", &sandbox, "proxy"));
    watch
        .until("connected and a snapshot", |w| {
            w.is(MachineId(1), "connected") && w.snapshots(MachineId(1)) > 0
        })
        .await;
    assert_eq!(watch.hub.name_of(MachineId(1)), Some("late"));
    assert_eq!(watch.hub.handle().name_of(MachineId(1)).as_deref(), Some("late"));
    assert_eq!(watch.kinds(MachineId(1)), ["connecting", "connected"]);

    // Adding the same name again changes nothing.
    watch.hub.handle().add(machine("LATE", &sandbox, "proxy"));
    let quiet = tokio::time::timeout(Duration::from_millis(300), watch.step()).await;
    assert!(quiet.is_err());
}

#[tokio::test]
async fn a_machines_opt_ins_are_what_its_entry_says_and_nothing_for_a_stranger() {
    let sandbox = Sandbox::new();
    let opted = MachineDef {
        notices: true,
        ..machine("box", &sandbox, "proxy")
    };
    let watch = Watch::new(&[opted, machine("other", &sandbox, "proxy")], &sandbox, quick(WAIT));
    let handle = watch.hub.handle();
    // Ids follow the names in order: box, then other.
    assert!(handle.opts_of(MachineId(1)).notices);
    assert!(!handle.opts_of(MachineId(1)).sounds);
    assert_eq!(handle.opts_of(MachineId(2)), Default::default());
    assert_eq!(handle.opts_of(MachineId(9)), Default::default());
}

#[tokio::test]
async fn a_grant_changes_what_the_hub_lets_through_at_once_and_only_for_that_machine() {
    let sandbox = Sandbox::new();
    let watch = Watch::new(
        &[machine("box", &sandbox, "hang"), machine("other", &sandbox, "hang")],
        &sandbox,
        quick(WAIT),
    );
    let handle = watch.hub.handle();
    assert_eq!(handle.id_of("BOX"), Some(MachineId(1)));
    assert_eq!(handle.id_of("this computer"), None);
    assert!(!handle.opts_of(MachineId(1)).clipboard);
    assert_eq!(handle.set_grant("box", Grant::Clipboard, true), Some(MachineId(1)));
    assert!(handle.opts_of(MachineId(1)).clipboard);
    assert!(!handle.opts_of(MachineId(2)).clipboard);
    handle.set_grant("box", Grant::Clipboard, false);
    assert!(!handle.opts_of(MachineId(1)).clipboard);
    assert_eq!(handle.set_grant("ghost", Grant::Notices, true), None);
}

#[tokio::test]
async fn removing_a_machine_that_is_not_there_says_nothing() {
    let sandbox = Sandbox::new();
    let mut watch = Watch::new(&[], &sandbox, quick(WAIT));
    watch.hub.handle().remove("ghost");
    let quiet = tokio::time::timeout(Duration::from_millis(300), watch.step()).await;
    assert!(quiet.is_err(), "no update for a machine that was never there");
}

#[tokio::test]
async fn an_outage_is_dated_from_its_first_state_and_ends_when_the_machine_is_connected() {
    let sandbox = Sandbox::new();
    let hub = Hub::start(&[], &shim(), ids_file(&sandbox), quick(WAIT), None);
    let handle = hub.handle();
    assert_eq!(handle.down_since(BOX, false, 1_000), Some(1_000));
    assert_eq!(
        handle.down_since(BOX, false, 9_000),
        Some(1_000),
        "a later state of the same outage keeps its start"
    );
    assert_eq!(handle.down_since(BOX, true, 10_000), None);
    assert_eq!(
        handle.down_since(BOX, false, 20_000),
        Some(20_000),
        "a new outage starts afresh"
    );
}

#[tokio::test]
async fn a_refused_machine_is_let_in_once_it_is_removed_and_added_again() {
    let mut sandbox = Sandbox::new();
    sandbox.start_daemon();
    let mut known = MachineIds::default();
    check_daemon_id(&mut known, "box", Some("aaaaaaaa11111111aaaaaaaa11111111")).unwrap();
    known.save(&ids_file(&sandbox));

    let mut watch = Watch::new(&[machine("box", &sandbox, "proxy")], &sandbox, quick(WAIT));
    watch
        .until("turned away", |w| w.is(BOX, "unreachable"))
        .await;
    let handle = watch.hub.handle();
    assert!(handle.is_refused(BOX), "told to the window as refused");

    // The person removes it and adds it again, as the refusal says.
    handle.remove("box");
    watch
        .until("removed", |w| w.removed == [BOX])
        .await;
    assert!(!handle.is_refused(BOX), "the refusal went with the machine");
    handle.add(machine("box", &sandbox, "proxy"));
    let again = MachineId(2);
    watch
        .until("connected and a snapshot", |w| {
            w.is(again, "connected") && w.snapshots(again) > 0
        })
        .await;
    assert!(!handle.is_refused(again));
    let mut recorded = MachineIds::load(&ids_file(&sandbox));
    assert!(
        check_daemon_id(&mut recorded, "box", Some("aaaaaaaa11111111aaaaaaaa11111111")).is_err(),
        "the daemon that answers now is the one recorded, not the one from before"
    );
}

#[tokio::test]
async fn a_computer_without_ssh_says_so_instead_of_hanging() {
    let sandbox = Sandbox::new();
    let missing = "maitty-no-such-ssh-program";
    let mut watch = Watch {
        hub: Hub::start(
            &[MachineDef {
                name: "box".into(),
                target: "box".into(),
                ..MachineDef::default()
            }],
            missing,
            ids_file(&sandbox),
            quick(WAIT),
            None,
        ),
        states: Vec::new(),
        msgs: Vec::new(),
        up: Vec::new(),
        ended: Vec::new(),
        removed: Vec::new(),
    };
    watch
        .until("unreachable", |w| w.is(BOX, "unreachable"))
        .await;
    let Some(ConnectionState::Unreachable { why, .. }) = watch.state_of(BOX) else {
        panic!("{:?}", watch.state_of(BOX));
    };
    assert!(why.contains("is OpenSSH installed?"), "{why}");
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
            attempt: None,
            unsent: None,
            since: None,
            clipboard: false,
            notices: false,
            sounds: false
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

mod commands;
mod end_to_end;

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
    async fn an_opted_in_machines_notice_is_one_clean_line_led_by_its_name() {
        let sandbox = Sandbox::new();
        let mut def = machine("box", &sandbox, "hang");
        def.notices = true;
        let mut hub = Hub::start(&[def], &shim(), ids_file(&sandbox), quick(WAIT), None);
        let hostile = format!("\u{1b}[31mdone\u{7}\n{}", "x".repeat(5000));
        let Some(ServerMsg::Notice(text)) = hub.admit(BOX, ServerMsg::Notice(hostile)) else {
            panic!("an opted-in notice is let through");
        };
        assert!(text.starts_with("[box] "), "{text}");
        assert!(text.contains("done"), "{text}");
        assert!(!text.chars().any(char::is_control), "{text:?}");
        assert!(text.chars().count() < 400, "cut short: {}", text.len());
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
            removed: Vec::new(),
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
