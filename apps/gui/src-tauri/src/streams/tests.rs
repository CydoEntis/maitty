use super::*;
use maitty_machines::remote_snapshot::ITEMS_MAX;
use maitty_shared::protocol::TermInfo;

const LOCAL: MachineId = MachineId::LOCAL;
const BOX: MachineId = MachineId(1);
const OTHER: MachineId = MachineId(2);

fn output(term: TermId, bytes: &[u8]) -> ServerMsg {
    ServerMsg::Output {
        term,
        data: bytes.to_vec(),
    }
}

fn replay(term: TermId, bytes: &[u8]) -> ServerMsg {
    ServerMsg::Replay {
        term,
        cols: 80,
        rows: 24,
        data: bytes.to_vec(),
    }
}

fn pane(id: TermId) -> TermInfo {
    serde_json::from_value(serde_json::json!({
        "id": id, "cols": 80, "rows": 24, "title": "t", "process": "bash",
        "agent": null, "status": "Idle", "cwd": "/home/me", "summary": "", "name": "n",
        "model": "", "said": "", "branch": null, "linked": false, "subagents": [],
        "root": null, "top": null, "since": 0, "asleep": false, "win32_input": false
    }))
    .expect("a pane")
}

fn snapshot_of(ids: &[TermId]) -> Snapshot {
    let mut snapshot = Snapshot::default();
    for id in ids {
        snapshot.terms.insert(*id, pane(*id));
    }
    snapshot
}

type Parts = (u16, u32, u64, bool, Vec<u8>);

/// A frame taken apart: machine, pane, sequence number, replay flag and output.
fn parts(out: &Out) -> Parts {
    let Out::Frame(frame) = out else {
        panic!("not a frame: {out:?}");
    };
    (
        u16::from_be_bytes(frame[0..2].try_into().unwrap()),
        u32::from_be_bytes(frame[2..6].try_into().unwrap()),
        u64::from_be_bytes(frame[6..14].try_into().unwrap()),
        frame[14] == 1,
        frame[FRAME_HEADER_BYTES..].to_vec(),
    )
}

fn frames(outs: &[Out]) -> Vec<Parts> {
    outs.iter()
        .filter(|out| matches!(out, Out::Frame(_)))
        .map(parts)
        .collect()
}

#[test]
fn a_frame_carries_its_machine_pane_sequence_and_replay_flag_before_the_output() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[7])), now);
    let replayed = streams.on_message(BOX, replay(7, b"screen"), now);
    assert_eq!(frames(&replayed), [(1, 7, 1, true, b"screen".to_vec())]);
    streams.on_message(BOX, output(7, b"more"), now);
    let flushed = streams.flush(now);
    assert_eq!(frames(&flushed), [(1, 7, 2, false, b"more".to_vec())]);
    assert_eq!(FRAME_HEADER_BYTES, 15);
}

#[test]
fn two_machines_number_their_panes_apart() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[3])), now);
    streams.on_message(LOCAL, replay(3, b"mine"), now);
    streams.on_message(BOX, replay(3, b"theirs"), now);
    streams.on_message(BOX, output(3, b" more"), now);
    streams.flush(now);
    let local = streams.history(LOCAL, 3).unwrap();
    let there = streams.history(BOX, 3).unwrap();
    assert_eq!(&local[..SEQ_BYTES], 1u64.to_be_bytes());
    assert_eq!(&local[SEQ_BYTES..], b"mine");
    assert_eq!(&there[..SEQ_BYTES], 2u64.to_be_bytes());
    assert_eq!(&there[SEQ_BYTES..], b"theirs more");
    assert!(streams.history(OTHER, 3).is_none());
}

#[test]
fn this_computers_output_waits_for_the_timer_unless_a_pane_has_a_lot() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(LOCAL, replay(1, b""), now);
    assert!(
        streams
            .on_message(LOCAL, output(1, b"echo"), now)
            .is_empty()
    );
    assert_eq!(streams.due(), Some(now + FLUSH_EVERY));
    let big = vec![b'x'; FLUSH_AT_BYTES];
    let sent = streams.on_message(LOCAL, output(2, &big), now);
    assert_eq!(frames(&sent).len(), 1, "a full pane goes at once");
    assert_eq!(streams.due(), Some(now + FLUSH_EVERY), "pane 1 still waits");
    let flushed = streams.flush(now);
    assert_eq!(frames(&flushed), [(0, 1, 2, false, b"echo".to_vec())]);
    assert_eq!(streams.due(), None);
}

#[test]
fn a_replay_of_this_computer_lands_after_the_output_held_before_it() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(LOCAL, output(1, b"old"), now);
    streams.on_message(LOCAL, output(2, b"other"), now);
    let sent = streams.on_message(LOCAL, replay(1, b"fresh"), now);
    let got = frames(&sent);
    assert_eq!(got.len(), 3);
    assert_eq!(got[0].4, b"old");
    assert_eq!(got[1].4, b"other");
    assert_eq!((got[2].4.as_slice(), got[2].3), (&b"fresh"[..], true));
}

#[test]
fn a_replay_of_another_machine_only_puts_that_pane_in_order() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1, 2])), now);
    streams.on_message(BOX, output(1, b"old"), now);
    streams.on_message(BOX, output(2, b"other"), now);
    let sent = streams.on_message(BOX, replay(1, b"fresh"), now);
    let got = frames(&sent);
    assert_eq!(
        got.iter().map(|f| f.4.clone()).collect::<Vec<_>>(),
        [b"old".to_vec(), b"fresh".to_vec()],
        "pane 2 keeps waiting for its flush"
    );
    assert!(streams.due().is_some());
}

#[test]
fn a_pane_without_a_replay_keeps_no_history_for_a_view_to_open_on() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[5])), now);
    streams.on_message(BOX, output(5, b"no replay yet"), now);
    streams.flush(now);
    assert!(streams.history(BOX, 5).is_none());
    streams.on_message(BOX, replay(5, b"screen"), now);
    assert!(streams.history(BOX, 5).is_some());
}

#[test]
fn history_past_the_limit_is_dropped_for_a_fresh_replay() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(LOCAL, replay(1, &vec![b'a'; HISTORY_LIMIT_BYTES]), now);
    assert!(streams.history(LOCAL, 1).is_some());
    streams.on_message(LOCAL, output(1, &vec![b'b'; FLUSH_AT_BYTES]), now);
    assert!(streams.history(LOCAL, 1).is_none());
}

#[test]
fn a_new_line_restarts_the_histories_of_that_machine_alone_and_the_sequence_runs_on() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1, 2])), now);
    streams.on_message(LOCAL, replay(1, b"here"), now);
    streams.on_message(BOX, replay(1, b"there"), now);
    streams.on_message(BOX, replay(2, b"two"), now);
    streams.on_message(BOX, output(2, b"x"), now);
    let mut watched = streams.line_up(BOX);
    watched.sort_unstable();
    assert_eq!(watched, [1, 2]);
    assert!(streams.history(BOX, 1).is_none());
    assert!(
        streams.history(LOCAL, 1).is_some(),
        "this computer was not touched"
    );
    assert!(
        streams.due().is_none(),
        "output of the old line is not given"
    );
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1, 2])), now);
    let sent = streams.on_message(BOX, replay(1, b"again"), now);
    assert_eq!(frames(&sent)[0].2, 2, "a view would drop a restart from 1");
    assert!(streams.line_up(BOX).contains(&1));
}

#[test]
fn a_whole_snapshot_is_given_and_a_delta_is_merged_into_that_machines_own() {
    let mut streams = Streams::default();
    let now = Instant::now();
    let none = streams.on_message(
        BOX,
        ServerMsg::TermsChanged {
            changed: vec![pane(9)],
            removed: Vec::new(),
            reasons: Default::default(),
            revision: 0,
        },
        now,
    );
    assert!(
        matches!(&none[..], [Out::WantList { machine }] if *machine == BOX),
        "a delta before a snapshot has nothing to apply to, so the whole list is asked for"
    );

    streams.on_message(LOCAL, ServerMsg::State(snapshot_of(&[1])), now);
    let given = streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1, 2])), now);
    assert!(matches!(
        &given[..],
        [Out::Sessions { machine, snapshot: Some(s) }] if *machine == BOX && s.terms.len() == 2
    ));
    let merged = streams.on_message(
        BOX,
        ServerMsg::TermsChanged {
            changed: vec![pane(3)],
            removed: vec![1, 4040],
            reasons: Default::default(),
            revision: 0,
        },
        now,
    );
    let [
        Out::Sessions {
            machine,
            snapshot: Some(snapshot),
        },
    ] = &merged[..]
    else {
        panic!("{merged:?}");
    };
    assert_eq!(*machine, BOX);
    assert_eq!(snapshot.terms.keys().copied().collect::<Vec<_>>(), [2, 3]);
    let local = streams.on_message(
        LOCAL,
        ServerMsg::TermsChanged {
            changed: Vec::new(),
            removed: vec![1],
            reasons: Default::default(),
            revision: 0,
        },
        now,
    );
    let [
        Out::Sessions {
            snapshot: Some(snapshot),
            ..
        },
    ] = &local[..]
    else {
        panic!("{local:?}");
    };
    assert!(snapshot.terms.is_empty(), "the local list is its own");
}

#[test]
fn a_merged_delta_says_why_its_own_panes_left_not_why_an_earlier_states_did() {
    use maitty_shared::protocol::RemovalReason;
    let mut streams = Streams::default();
    let now = Instant::now();
    let mut held = snapshot_of(&[1, 2]);
    held.removed = BTreeMap::from([(9, RemovalReason::Closed)]);
    streams.on_message(LOCAL, ServerMsg::State(held), now);
    let merged = streams.on_message(
        LOCAL,
        ServerMsg::TermsChanged {
            changed: Vec::new(),
            removed: vec![1],
            reasons: BTreeMap::from([(1, RemovalReason::Exited)]),
            revision: 0,
        },
        now,
    );
    let [
        Out::Sessions {
            snapshot: Some(snapshot),
            ..
        },
    ] = &merged[..]
    else {
        panic!("{merged:?}");
    };
    assert_eq!(snapshot.removed, BTreeMap::from([(1, RemovalReason::Exited)]));
}

#[test]
fn a_snapshot_forgets_only_that_machines_panes_that_went() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(LOCAL, replay(1, b"a"), now);
    streams.on_message(BOX, replay(1, b"b"), now);
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[2])), now);
    assert!(streams.history(BOX, 1).is_none(), "gone from its machine");
    assert!(streams.history(LOCAL, 1).is_some());
}

#[test]
fn this_computers_line_ending_empties_its_list_and_another_machines_does_not() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1])), now);
    streams.on_message(LOCAL, output(1, b"last words"), now);
    let local = streams.line_ended(LOCAL, now);
    assert_eq!(frames(&local).len(), 1, "what it said is given first");
    assert!(matches!(
        local.last(),
        Some(Out::Sessions { machine, snapshot: None }) if *machine == LOCAL
    ));
    streams.on_message(BOX, output(1, b"last words"), now);
    let there = streams.line_ended(BOX, now);
    assert_eq!(frames(&there).len(), 1);
    assert!(there.iter().all(|out| matches!(out, Out::Frame(_))));
}

#[test]
fn another_machine_gets_a_budget_per_flush_and_the_rest_waits() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1])), now);
    streams.on_message(BOX, replay(1, b""), now);
    streams.on_message(
        BOX,
        output(1, &vec![b'x'; REMOTE_BYTES_PER_FLUSH * 2 + 5]),
        now,
    );
    let first = frames(&streams.flush(now));
    assert_eq!(first.len(), 1);
    assert_eq!(first[0].4.len(), REMOTE_BYTES_PER_FLUSH);
    assert!(streams.due().is_some(), "the rest is still due");
    let second = frames(&streams.flush(now));
    assert_eq!(second[0].4.len(), REMOTE_BYTES_PER_FLUSH);
    let third = frames(&streams.flush(now));
    assert_eq!(third[0].4.len(), 5);
    assert!(streams.due().is_none());
    assert_eq!(
        streams.history(BOX, 1).unwrap().len(),
        SEQ_BYTES + REMOTE_BYTES_PER_FLUSH * 2 + 5,
        "nothing was lost, only held back"
    );
}

#[test]
fn a_machine_that_floods_is_dropped_to_a_fresh_replay_asked_for_once() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1])), now);
    streams.on_message(BOX, replay(1, b"screen"), now);
    let chunk = vec![b'x'; 1024 * 1024];
    let mut wanted = 0;
    for _ in 0..40 {
        for out in streams.on_message(BOX, output(1, &chunk), now) {
            if matches!(out, Out::WantReplay { machine, term: 1 } if machine == BOX) {
                wanted += 1;
            }
        }
    }
    assert_eq!(wanted, 1, "asked once, not once per dropped chunk");
    assert!(streams.waiting_of(BOX) <= REMOTE_PENDING_MAX);
    assert!(
        streams.history(BOX, 1).is_none(),
        "the history no longer starts at a replay"
    );
    let fresh = streams.on_message(BOX, replay(1, b"fresh"), now);
    assert_eq!(frames(&fresh).last().unwrap().4, b"fresh");
    // Answered, so a later flood may ask again.
    let mut again = 0;
    for _ in 0..8 {
        again += streams
            .on_message(BOX, output(1, &chunk), now)
            .iter()
            .filter(|out| matches!(out, Out::WantReplay { .. }))
            .count();
    }
    assert_eq!(again, 1);
}

/// What the window is given of this computer over a run of ticks, with `flood` as another
/// machine's output arriving all the while.
fn local_frames_over_ticks(flood: bool) -> Vec<Vec<Parts>> {
    let mut streams = Streams::default();
    let mut now = Instant::now();
    streams.on_message(LOCAL, replay(1, b""), now);
    streams.on_message(BOX, replay(1, b""), now);
    let mut given = Vec::new();
    for tick in 0u8..50 {
        let line = format!("line {tick}\r\n");
        let mut outs = streams.on_message(LOCAL, output(1, line.as_bytes()), now);
        if flood {
            for _ in 0..4 {
                outs.extend(streams.on_message(BOX, output(1, &vec![b'f'; 512 * 1024]), now));
            }
        }
        now += FLUSH_EVERY;
        outs.extend(streams.flush(now));
        given.push(
            frames(&outs)
                .into_iter()
                .filter(|frame| frame.0 == 0)
                .collect(),
        );
    }
    given
}

#[test]
fn a_flooding_machine_leaves_this_computers_output_exactly_as_it_was() {
    let alone = local_frames_over_ticks(false);
    let flooded = local_frames_over_ticks(true);
    assert_eq!(alone.len(), 50);
    assert!(
        alone.iter().all(|tick| tick.len() == 1),
        "one frame per tick"
    );
    assert_eq!(
        flooded, alone,
        "same frames, same numbers, in the same ticks"
    );
}

#[test]
fn what_the_flooding_machine_gets_through_in_a_flush_is_held_to_its_budget() {
    let mut streams = Streams::default();
    let mut now = Instant::now();
    streams.on_message(BOX, replay(1, b""), now);
    streams.on_message(OTHER, replay(1, b""), now);
    for _ in 0..20 {
        for machine in [BOX, OTHER] {
            streams.on_message(machine, output(1, &vec![b'f'; 512 * 1024]), now);
        }
        now += FLUSH_EVERY;
        let sent = frames(&streams.flush(now));
        for machine in [1u16, 2] {
            let bytes: usize = sent
                .iter()
                .filter(|frame| frame.0 == machine)
                .map(|frame| frame.4.len())
                .sum();
            assert!(
                bytes <= REMOTE_BYTES_PER_FLUSH,
                "machine {machine} sent {bytes}"
            );
        }
    }
}

#[test]
fn a_machines_github_report_is_given_with_its_machine_and_tag() {
    use maitty_shared::protocol::{GitHubAccount, GitHubReport};
    let mut streams = Streams::default();
    let report = GitHubReport {
        installed: true,
        accounts: vec![GitHubAccount { login: "me".into(), active: true }],
        problem: None,
    };
    let answer = ServerMsg::Reply(Reply::GitHubAccounts { tag: 7, report: report.clone() });
    let said = streams.on_message(BOX, answer, Instant::now());
    assert!(matches!(
        &said[..],
        [Out::GitHubAccounts { machine, tag: 7, report: given }] if *machine == BOX && *given == report
    ));
}

#[test]
fn what_another_machine_says_went_wrong_is_given_and_this_computers_is_not() {
    let mut streams = Streams::default();
    let now = Instant::now();
    let said = streams.on_message(BOX, ServerMsg::Error("[box] no such pane".into()), now);
    assert!(matches!(
        &said[..],
        [Out::Error { machine, text }] if *machine == BOX && text == "[box] no such pane"
    ));
    assert!(
        streams
            .on_message(LOCAL, ServerMsg::Error("x".into()), now)
            .is_empty()
    );
}

#[test]
fn the_daemon_s_decision_to_alert_reaches_the_window_from_this_computer_and_from_another() {
    use maitty_shared::protocol::Status;
    let mut streams = Streams::default();
    let now = Instant::now();
    for machine in [LOCAL, BOX] {
        let told = streams.on_message(
            machine,
            ServerMsg::Attention {
                term: 7,
                status: Status::Done,
            },
            now,
        );
        assert!(
            matches!(
                &told[..],
                [Out::Attention { machine: m, term: 7, status: Status::Done }] if *m == machine
            ),
            "{told:?}"
        );
    }
}

#[test]
fn what_another_machine_wants_shown_is_given_and_this_computers_is_not() {
    let mut streams = Streams::default();
    let now = Instant::now();
    let said = streams.on_message(BOX, ServerMsg::Notice("[box] saved".into()), now);
    assert!(matches!(
        &said[..],
        [Out::Notice { machine, text }] if *machine == BOX && text == "[box] saved"
    ));
    assert!(
        streams
            .on_message(LOCAL, ServerMsg::Notice("x".into()), now)
            .is_empty()
    );
}

#[test]
fn what_a_program_copies_is_given_from_another_machine_and_from_this_computer() {
    let mut streams = Streams::default();
    let now = Instant::now();
    let msg = || ServerMsg::Clipboard { term: 1, text: "ls".into() };
    let said = streams.on_message(BOX, msg(), now);
    assert!(matches!(
        &said[..],
        [Out::Clipboard { machine, text }] if *machine == BOX && text == "ls"
    ));
    let here = streams.on_message(LOCAL, msg(), now);
    assert!(matches!(
        &here[..],
        [Out::Clipboard { machine, text }] if *machine == LOCAL && text == "ls"
    ));
}

#[test]
fn another_machines_pane_deltas_cannot_grow_its_list_past_the_cap() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1])), now);
    let mut refused = 0;
    for batch in 0..20u32 {
        let changed = (0..1_000)
            .map(|i| pane(1_000 + batch * 1_000 + i))
            .collect();
        let out = streams.on_message(
            BOX,
            ServerMsg::TermsChanged {
                changed,
                removed: Vec::new(),
                reasons: Default::default(),
                revision: 0,
            },
            now,
        );
        if out.is_empty() {
            refused += 1;
        }
    }
    assert!(
        streams.last[&BOX].terms.len() <= ITEMS_MAX,
        "deltas that are each within the cap grew the list to {}",
        streams.last[&BOX].terms.len()
    );
    assert!(refused > 0, "no delta was refused");
}

#[test]
fn this_computers_pane_deltas_are_not_held_to_another_machines_cap() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(LOCAL, ServerMsg::State(snapshot_of(&[1])), now);
    let changed = (0..ITEMS_MAX as u32 + 5).map(|i| pane(10 + i)).collect();
    let out = streams.on_message(
        LOCAL,
        ServerMsg::TermsChanged {
            changed,
            removed: Vec::new(),
            reasons: Default::default(),
            revision: 0,
        },
        now,
    );
    assert!(matches!(&out[..], [Out::Sessions { .. }]));
}

#[test]
fn another_machines_output_and_replays_for_panes_it_never_listed_are_dropped() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1])), now);

    let out = streams.on_message(BOX, output(77, b"x"), now);
    assert!(out.is_empty());
    assert!(streams.pending.is_empty());
    assert!(!streams.panes.contains_key(&(BOX, 77)));

    let out = streams.on_message(BOX, replay(77, b"x"), now);
    assert!(out.is_empty());
    assert!(!streams.panes.contains_key(&(BOX, 77)));

    // Before any list at all, nothing of another machine's is tracked either.
    let out = streams.on_message(OTHER, output(1, b"x"), now);
    assert!(out.is_empty());
    assert!(!streams.panes.contains_key(&(OTHER, 1)));

    // This computer's own panes are not held to a list, and a listed pane still gets its frames.
    streams.on_message(LOCAL, output(5, b"x"), now);
    assert!(streams.pending.contains_key(&(LOCAL, 5)));
    let out = streams.on_message(BOX, replay(1, b"screen"), now);
    assert_eq!(frames(&out), vec![(1, 1, 1, true, b"screen".to_vec())]);
}

#[test]
fn a_replay_of_another_machine_past_the_history_limit_is_not_passed_on_and_is_asked_for_once() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1])), now);
    let big = vec![b'x'; HISTORY_LIMIT_BYTES + 1];

    let out = streams.on_message(BOX, replay(1, &big), now);
    assert!(frames(&out).is_empty(), "the window was handed the replay");
    assert!(matches!(
        &out[..],
        [Out::WantReplay { machine, term }] if *machine == BOX && *term == 1
    ));

    let again = streams.on_message(BOX, replay(1, &big), now);
    assert!(
        again.is_empty(),
        "asked again for a replay already asked for"
    );
}

fn delta(revision: u64, changed: &[TermId], removed: &[TermId]) -> ServerMsg {
    ServerMsg::TermsChanged {
        changed: changed.iter().map(|id| pane(*id)).collect(),
        removed: removed.to_vec(),
        reasons: Default::default(),
        revision,
    }
}

fn numbered(revision: u64, ids: &[TermId]) -> Snapshot {
    Snapshot {
        revision,
        ..snapshot_of(ids)
    }
}

fn listed(streams: &Streams, machine: MachineId) -> Vec<TermId> {
    streams
        .last_snapshot(machine)
        .map(|list| list.terms.keys().copied().collect())
        .unwrap_or_default()
}

#[test]
fn a_missed_update_is_made_up_by_asking_for_the_whole_list_once() {
    let mut streams = Streams::default();
    let now = Instant::now();
    let mut outs = streams.on_message(LOCAL, ServerMsg::State(numbered(1, &[1])), now);
    outs.extend(streams.on_message(LOCAL, delta(2, &[2], &[]), now));
    // Revision 3, which added pane 3, never came.
    let after_the_gap = streams.on_message(LOCAL, delta(4, &[4], &[]), now);
    assert!(
        matches!(&after_the_gap[..], [Out::WantList { machine }] if *machine == LOCAL),
        "{after_the_gap:?}"
    );
    assert_eq!(listed(&streams, LOCAL), [1, 2], "a delta past a gap is not merged");
    outs.extend(after_the_gap);

    let whole = ServerMsg::Reply(Reply::List(numbered(4, &[1, 2, 3, 4])));
    let given = streams.on_message(LOCAL, whole, now);
    assert!(
        matches!(&given[..], [Out::Sessions { machine, snapshot: Some(s) }] if *machine == LOCAL && s.terms.len() == 4),
        "{given:?}"
    );
    outs.extend(given);
    outs.extend(streams.on_message(LOCAL, delta(5, &[], &[1]), now));

    let asked = outs.iter().filter(|out| matches!(out, Out::WantList { .. })).count();
    assert_eq!(asked, 1);
    assert_eq!(listed(&streams, LOCAL), [2, 3, 4]);
}

#[test]
fn a_delta_before_any_snapshot_asks_for_the_whole_list() {
    let mut streams = Streams::default();
    let now = Instant::now();
    let early = streams.on_message(BOX, delta(7, &[9], &[]), now);
    assert!(
        matches!(&early[..], [Out::WantList { machine }] if *machine == BOX),
        "{early:?}"
    );
    streams.on_message(BOX, ServerMsg::Reply(Reply::List(numbered(7, &[8, 9]))), now);
    streams.on_message(BOX, delta(8, &[], &[8]), now);
    assert_eq!(listed(&streams, BOX), [9]);
}

#[test]
fn a_daemon_that_numbers_nothing_has_its_deltas_merged_as_they_come() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(snapshot_of(&[1])), now);
    let first = streams.on_message(BOX, delta(0, &[2], &[]), now);
    let second = streams.on_message(BOX, delta(0, &[3], &[]), now);
    assert!(matches!(&first[..], [Out::Sessions { .. }]), "{first:?}");
    assert!(matches!(&second[..], [Out::Sessions { .. }]), "{second:?}");
    assert_eq!(listed(&streams, BOX), [1, 2, 3]);
}

fn asked(outs: &[Out]) -> usize {
    outs.iter().filter(|out| matches!(out, Out::WantList { .. })).count()
}

#[test]
fn a_burst_of_deltas_after_a_gap_asks_for_the_list_once() {
    let mut streams = Streams::default();
    let now = Instant::now();
    let mut outs = streams.on_message(BOX, ServerMsg::State(numbered(1, &[1])), now);
    for (revision, pane) in [(2, 2), (4, 4), (5, 5), (6, 6)] {
        outs.extend(streams.on_message(BOX, delta(revision, &[pane], &[]), now));
    }
    assert_eq!(asked(&outs), 1);
    assert_eq!(listed(&streams, BOX), [1, 2], "nothing past the gap is merged");

    streams.on_message(BOX, ServerMsg::Reply(Reply::List(numbered(6, &[1, 2, 3, 4, 5, 6]))), now);
    let next_gap = streams.on_message(BOX, delta(9, &[9], &[]), now);
    assert_eq!(asked(&next_gap), 1, "the list that came settles the asking, so a new gap asks again");
}

#[test]
fn a_list_that_never_comes_is_asked_for_again_after_a_while_and_on_a_new_line() {
    let mut streams = Streams::default();
    let now = Instant::now();
    streams.on_message(BOX, ServerMsg::State(numbered(1, &[1])), now);
    assert_eq!(asked(&streams.on_message(BOX, delta(3, &[3], &[]), now)), 1);

    let soon = now + LIST_ASKED_AGAIN_AFTER - Duration::from_millis(1);
    assert_eq!(asked(&streams.on_message(BOX, delta(4, &[4], &[]), soon)), 0);
    let later = now + LIST_ASKED_AGAIN_AFTER;
    assert_eq!(asked(&streams.on_message(BOX, delta(5, &[5], &[]), later)), 1);

    streams.line_up(BOX);
    assert_eq!(
        asked(&streams.on_message(BOX, delta(6, &[6], &[]), later)),
        1,
        "what was asked of the old line is not waited for on the new one"
    );
    streams.forget(BOX);
    assert_eq!(asked(&streams.on_message(BOX, delta(7, &[7], &[]), later)), 1);
}
