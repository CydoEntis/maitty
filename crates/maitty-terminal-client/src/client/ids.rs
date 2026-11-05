//! Ids the window keeps for what lives on a machine. Each daemon numbers its own panes from 1,
//! so a pane is only named by its machine together with its `TermId`. These never cross the wire.

use super::link::ConnectionState;
use crate::protocol::{SessionKey, Snapshot, TermId, TermInfo};
pub use maitty_machines::{ClientTerm, MachineId, Outgoing};

/// A session row (pane or conversation), named across machines.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ClientSession {
    pub machine: MachineId,
    pub key: SessionKey,
}

impl ClientSession {
    pub const fn on(machine: MachineId, key: SessionKey) -> Self {
        ClientSession { machine, key }
    }

    #[cfg(test)]
    pub const fn local(key: SessionKey) -> Self {
        ClientSession::on(MachineId::LOCAL, key)
    }
}

/// One machine's name and what its daemon last reported. Limits, spend, `daemon_version` and
/// the active workspace and tab are all in the snapshot, so they stay per machine.
#[derive(Clone, Copy)]
pub struct MachineSnap<'a> {
    pub id: MachineId,
    /// Its configured name; empty for this machine. Saved layout is kept under it.
    pub name: &'a str,
    pub snap: &'a Snapshot,
}

impl MachineSnap<'_> {
    pub fn term(&self, term: TermId) -> Option<&TermInfo> {
        self.snap.terms.get(&term)
    }
}

/// A machine other than this one, as its daemon last reported it. When its line drops the
/// snapshot stays, so its agents stay in the model until the line is back.
#[derive(Debug, Default)]
pub struct Remote {
    pub name: String,
    pub snap: Snapshot,
    pub state: ConnectionState,
    /// When it went down, kept across the failure states so "disconnected since" does not
    /// restart when a retry fails differently.
    pub down_since: Option<std::time::Instant>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::App;
    use crate::client::reference_fixture::{Stage, world};
    use crate::protocol::{ClientMsg, PROTOCOL_VERSION, Surface, encode};
    use ratatui::Terminal;
    use ratatui::backend::TestBackend;
    use std::collections::{HashMap, HashSet};

    /// The bytes as they leave the window, spelled out: a change here is a wire change.
    fn hex(msg: &ClientMsg) -> String {
        encode(msg)
            .unwrap()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn the_same_pane_number_on_two_machines_is_two_panes() {
        let (here, there) = (ClientTerm::local(1), ClientTerm::on(MachineId(1), 1));
        assert_ne!(here, there);
        let mut sizes = HashMap::from([(here, (80, 24))]);
        sizes.insert(there, (120, 40));
        assert_eq!(sizes[&here], (80, 24));
        assert_eq!(sizes[&there], (120, 40));
        let focus_report: HashSet<_> = [there].into();
        assert!(focus_report.contains(&there) && !focus_report.contains(&here));
    }

    #[test]
    fn a_pane_on_each_machine_keeps_its_own_output() {
        let (tx, _rx) = tokio::sync::mpsc::unbounded_channel();
        let (bg_tx, _bg_rx) = tokio::sync::mpsc::unbounded_channel();
        let mut app = crate::client::App::new(crate::config::Config::default(), tx, None, bg_tx);
        let (here, there) = (ClientTerm::local(1), ClientTerm::on(MachineId(1), 1));
        app.keep_raw(here, b"here", false);
        app.keep_raw(there, b"there", false);
        assert_eq!(app.raw[&here], b"here");
        assert_eq!(app.raw[&there], b"there");
    }

    #[test]
    fn typing_is_addressed_to_the_machine_of_the_pane() {
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
        let (bg_tx, _bg_rx) = tokio::sync::mpsc::unbounded_channel();
        let app = crate::client::App::new(crate::config::Config::default(), tx, None, bg_tx);
        let there = ClientTerm::on(MachineId(1), 7);
        app.send_input(there, b"ls".to_vec());
        let sent = rx.try_recv().unwrap();
        assert_eq!(sent.machine, MachineId(1));
        assert!(matches!(sent.msg, ClientMsg::Input { term: 7, data } if data == b"ls"));
    }

    #[test]
    fn machines_do_not_change_what_goes_over_the_wire() {
        assert_eq!(PROTOCOL_VERSION, 71);
        let hello = ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            attach: true,
            from: Some((3, "00ff".into())),
            structured: None,
            surface: Surface::Terminal,
            replays: true,
        };
        let input = ClientMsg::Input {
            term: 3,
            data: b"ls\r".to_vec(),
        };
        let resize = ClientMsg::Resize {
            term: 3,
            cols: 120,
            rows: 40,
        };
        let command = ClientMsg::Command(crate::protocol::Command::FocusPane { term: 3 });
        assert_eq!(
            hex(&hello),
            "81a548656c6c6f86a776657273696f6e46a6617474616368c3a466726f6d9203a430306666aa73747275637475726564c0a773757266616365a87465726d696e616ca77265706c617973c3"
        );
        assert_eq!(
            hex(&input),
            "81a5496e70757482a47465726d03a464617461c4036c730d"
        );
        assert_eq!(
            hex(&resize),
            "81a6526573697a6583a47465726d03a4636f6c7378a4726f777328"
        );
        assert_eq!(
            hex(&command),
            "81a7436f6d6d616e6481a9466f63757350616e6581a47465726d03"
        );
    }

    /// A saved file as `maitty-ui.json` was written before machines, from a real install (folders
    /// changed). Its sessions are bare pane numbers.
    const OLD_SAVED: &str = r#"{
  "order": [
    "c:\\dev",
    "c:\\dev\\maitty",
    "ssh:github.com",
    "\\\\?\\c:\\dev\\autobattler"
  ],
  "closed": ["p:c:\\dev"],
  "session_order": [2, 1],
  "placed": [[2, "c:\\dev\\maitty", true]],
  "open_branches": [],
  "side_w": 38,
  "split": null,
  "draft": "",
  "last_agent": "claude",
  "names": { "c:\\dev\\maitty": "maitty" },
  "reviewed": {},
  "recent": [],
  "sheet_w": {}
}"#;

    fn two_machines() -> App {
        let mut app = world(Stage::None);
        let there = app.snap.clone();
        app.remotes.insert(
            MachineId(1),
            Remote {
                name: "build".into(),
                snap: there,
                ..Remote::default()
            },
        );
        app
    }

    /// The sessions as the sidebar lists them, top to bottom.
    fn rows(app: &App) -> Vec<(MachineId, SessionKey)> {
        let model = app.screen_model();
        crate::client::screen::side_lines(app, &model, &app.theme)
            .iter()
            .filter_map(|line| crate::client::screen::line_term(&model, line))
            .map(|s| (s.machine, s.key))
            .collect()
    }

    #[test]
    fn an_old_saved_file_loads_unchanged_and_sorts_the_same() {
        let saved: crate::client::screen::Saved = serde_json::from_str(OLD_SAVED).unwrap();
        assert!(saved.machines.is_empty());
        assert_eq!(
            saved.session_order,
            [SessionKey::Pane(2), SessionKey::Pane(1)]
        );
        assert_eq!(saved.placed[0].0, SessionKey::Pane(2));
        // Saving writes the sessions in their current spelling and nothing else differs.
        let mut again: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&saved).unwrap()).unwrap();
        let mut old: serde_json::Value = serde_json::from_str(OLD_SAVED).unwrap();
        for file in [&mut again, &mut old] {
            let file = file.as_object_mut().unwrap();
            file.remove("session_order");
            file.remove("placed");
        }
        assert_eq!(again, old);

        let mut app = world(Stage::None);
        let unsorted = rows(&app);
        app.screen.saved = saved;
        app.screen_fresh();
        let sorted = rows(&app);
        assert_eq!(sorted.len(), unsorted.len());
        assert!(
            sorted
                .iter()
                .all(|(machine, _)| *machine == MachineId::LOCAL)
        );
        let at = |rows: &[(MachineId, SessionKey)], id| {
            rows.iter().position(|(_, k)| *k == SessionKey::Pane(id))
        };
        assert!(at(&unsorted, 1) < at(&unsorted, 2), "unsaved: {unsorted:?}");
        assert!(
            at(&sorted, 2) < at(&sorted, 1),
            "the saved order wins: {sorted:?}"
        );
    }

    #[test]
    fn entries_for_machines_that_are_not_connected_are_kept() {
        let gone = r#"{"machines":{"gone-box":{"session_order":[{"Pane":7}],"placed":[[{"Pane":7},"/srv/app",true]]}}}"#;
        let mut app = world(Stage::None);
        app.screen.saved = serde_json::from_str(gone).unwrap();
        let before = app.screen.saved.machines.clone();
        app.screen_sync();
        assert_eq!(app.screen.saved.machines, before);
        let written = serde_json::to_string(&app.screen.saved).unwrap();
        let read: crate::client::screen::Saved = serde_json::from_str(&written).unwrap();
        assert_eq!(read.machines, before);
    }

    #[test]
    fn each_machine_sorts_by_its_own_saved_order() {
        let mut app = two_machines();
        app.screen.saved.session_order = vec![SessionKey::Pane(2)];
        app.screen_fresh();
        let all = rows(&app);
        let order_on = |machine| -> Vec<SessionKey> {
            all.iter()
                .filter(|(m, _)| *m == machine)
                .map(|(_, k)| *k)
                .collect()
        };
        let (here, there) = (order_on(MachineId::LOCAL), order_on(MachineId(1)));
        let at = |rows: &[SessionKey], id| rows.iter().position(|k| *k == SessionKey::Pane(id));
        assert!(
            at(&here, 2) < at(&here, 1),
            "this machine's saved order: {here:?}"
        );
        assert!(
            at(&there, 1) < at(&there, 2),
            "the other has none of its own: {there:?}"
        );
        app.screen
            .saved
            .session_order_mut("build")
            .push(SessionKey::Pane(2));
        app.screen_fresh();
        let there: Vec<_> = rows(&app)
            .into_iter()
            .filter(|(m, _)| *m == MachineId(1))
            .collect();
        assert_eq!(there[0].1, SessionKey::Pane(2), "its own order: {there:?}");
    }

    #[test]
    fn the_same_pane_number_on_two_machines_is_two_sessions() {
        let mut app = two_machines();
        let model = app.screen_model();
        let sessions: Vec<_> = model.iter().flat_map(|p| p.sessions()).collect();
        let on = |machine| sessions.iter().filter(|s| s.key.machine == machine).count();
        assert!(on(MachineId::LOCAL) > 0);
        assert_eq!(on(MachineId::LOCAL), on(MachineId(1)));
        let keys: std::collections::HashSet<_> = sessions.iter().map(|s| s.key).collect();
        assert_eq!(keys.len(), sessions.len(), "no two sessions share a key");
        assert!(
            sessions
                .iter()
                .all(|s| s.native_term.is_none_or(|t| t.machine == s.key.machine)),
            "a row's pane is on its own machine"
        );
        assert!(
            model
                .iter()
                .all(|p| p.sessions().all(|s| s.key.machine == p.machine)),
            "a folder on one machine is not the same group as the same folder on another"
        );
        let mut terminal = Terminal::new(TestBackend::new(160, 45)).unwrap();
        terminal
            .draw(|f| crate::client::render::draw(&mut app, f))
            .unwrap();
        assert_eq!(
            app.screen.visible.len(),
            sessions.len(),
            "both machines' rows are drawn"
        );
    }

    fn state_with_id(id: Option<&str>) -> crate::protocol::ServerMsg {
        crate::protocol::ServerMsg::State(Snapshot {
            daemon_id: id.map(str::to_owned),
            ..Snapshot::default()
        })
    }

    const BOX_A: &str = "aaaaaaaa11111111aaaaaaaa11111111";
    const BOX_B: &str = "bbbbbbbb22222222bbbbbbbb22222222";

    #[test]
    fn a_different_daemon_at_a_machines_name_is_not_applied_and_the_machine_is_refused() {
        let mut app = two_machines();
        let with_panes = app.snap.clone();
        assert!(!with_panes.terms.is_empty());
        app.on_machine_server(MachineId(1), state_with_id(Some(BOX_A)));
        assert!(app.refused.is_empty());

        app.on_machine_server(MachineId(1), state_with_id(Some(BOX_B)));
        let reason = app
            .refused
            .get(&MachineId(1))
            .expect("refused with a reason");
        assert!(
            reason.starts_with("a different machine answers at build"),
            "{reason}"
        );
        assert!(app.notice.as_ref().is_some_and(|(text, ..)| text == reason));

        let mut other = with_panes;
        other.daemon_id = Some(BOX_B.into());
        app.on_machine_server(MachineId(1), crate::protocol::ServerMsg::State(other));
        assert!(
            app.remotes[&MachineId(1)].snap.terms.is_empty(),
            "a refused machine's state is dropped"
        );
        assert!(
            !app.snap.terms.is_empty(),
            "and another machine's state never reaches this machine's"
        );
    }

    #[test]
    fn a_pane_delta_keeps_the_daemon_id_the_machine_is_known_by() {
        let mut app = two_machines();
        app.on_machine_server(MachineId(1), state_with_id(Some(BOX_A)));
        let pane = app.snap.terms.values().next().unwrap().clone();
        let delta = crate::protocol::ServerMsg::TermsChanged {
            changed: vec![crate::protocol::TermInfo { id: 70, ..pane }],
            removed: Vec::new(),
        };
        app.on_machine_server(MachineId(1), delta);
        let held = &app.remotes[&MachineId(1)].snap;
        assert_eq!(held.daemon_id.as_deref(), Some(BOX_A));
        assert!(held.terms.contains_key(&70));
        assert!(app.refused.is_empty());
    }

    #[test]
    fn this_computers_daemon_id_survives_a_pane_delta() {
        let mut app = two_machines();
        app.snap.daemon_id = Some(BOX_A.into());
        let pane = app.snap.terms.values().next().unwrap().clone();
        app.on_server(crate::protocol::ServerMsg::TermsChanged {
            changed: vec![crate::protocol::TermInfo { id: 70, ..pane }],
            removed: Vec::new(),
        });
        assert_eq!(app.snap.daemon_id.as_deref(), Some(BOX_A));
        assert!(app.snap.terms.contains_key(&70));
    }

    #[test]
    fn a_pane_delta_before_any_state_cannot_slip_past_a_recorded_daemon_id() {
        let mut app = two_machines();
        app.on_machine_server(MachineId(1), state_with_id(Some(BOX_A)));
        app.remotes.get_mut(&MachineId(1)).unwrap().snap = Snapshot::default();
        let pane = app.snap.terms.values().next().unwrap().clone();
        app.on_machine_server(
            MachineId(1),
            crate::protocol::ServerMsg::TermsChanged {
                changed: vec![pane],
                removed: Vec::new(),
            },
        );
        assert!(app.refused.contains_key(&MachineId(1)));
        assert!(app.remotes[&MachineId(1)].snap.terms.is_empty());
    }

    #[test]
    fn a_pane_delta_from_a_machine_never_seen_before_is_accepted_without_recording_an_id() {
        let mut app = two_machines();
        let pane = app.snap.terms.values().next().unwrap().clone();
        app.on_machine_server(
            MachineId(1),
            crate::protocol::ServerMsg::TermsChanged {
                changed: vec![pane],
                removed: Vec::new(),
            },
        );
        assert!(app.refused.is_empty());
        assert!(!app.machine_ids.forget("build"), "no id was recorded");
    }

    #[test]
    fn a_remote_machines_state_is_kept_under_it_and_never_replaces_this_machines() {
        let mut app = two_machines();
        let local = app.snap.terms.keys().copied().collect::<Vec<_>>();
        let mut theirs = Snapshot::default();
        theirs
            .terms
            .insert(77, app.snap.terms.values().next().unwrap().clone());
        theirs.spent_today = 4.5;
        app.on_machine_server(MachineId(1), crate::protocol::ServerMsg::State(theirs));
        assert_eq!(app.snap.terms.keys().copied().collect::<Vec<_>>(), local);
        assert_ne!(app.snap.spent_today, 4.5);
        let there = &app.remotes[&MachineId(1)].snap;
        assert_eq!((there.terms.len(), there.spent_today), (1, 4.5));
        assert!(app.parsers.contains_key(&ClientTerm::on(MachineId(1), 77)));
        assert!(
            !app.parsers
                .contains_key(&ClientTerm::on(MachineId::LOCAL, 77)),
            "its pane is not one of ours"
        );
    }

    #[test]
    fn a_remote_state_does_not_drop_this_machines_pending_alerts_or_parsers() {
        let mut app = two_machines();
        let mine = *app.snap.terms.keys().next().unwrap();
        let term = ClientTerm::local(mine);
        app.settling.insert(
            term,
            (
                crate::protocol::Status::Blocked,
                std::time::Instant::now() + std::time::Duration::from_secs(60),
            ),
        );
        app.on_machine_server(MachineId(1), state_with_id(Some(BOX_A)));
        assert!(app.settling.contains_key(&term));
        assert!(app.parsers.contains_key(&term));
    }

    fn pinned_to(app: &mut App, id: &str) {
        app.on_machine_server(MachineId(1), state_with_id(Some(id)));
    }

    #[test]
    fn a_refusal_stays_for_the_same_wrong_daemon_and_clears_when_the_machine_entry_changes() {
        use maitty_settings::config::MachineDef;
        let def = |target: &str| MachineDef {
            name: "build".into(),
            target: target.into(),
            ..MachineDef::default()
        };
        let mut app = two_machines();
        app.cfg.machines = vec![def("old.example")];
        pinned_to(&mut app, BOX_A);
        pinned_to(&mut app, BOX_B);
        assert!(app.refused.contains_key(&MachineId(1)));

        let mut same = app.cfg.clone();
        same.machines = vec![def("old.example")];
        app.machines_edited(&same.machines);
        assert!(
            app.refused.contains_key(&MachineId(1)),
            "an untouched entry keeps the refusal"
        );
        pinned_to(&mut app, BOX_B);
        assert!(app.refused.contains_key(&MachineId(1)), "the same wrong id");

        app.machines_edited(&[def("new.example")]);
        assert!(app.refused.is_empty(), "a changed entry clears it");
        pinned_to(&mut app, BOX_B);
        assert!(
            app.refused.contains_key(&MachineId(1)),
            "and the wrong daemon is refused again on the re-check"
        );
        app.machines_edited(&[]);
        assert!(app.refused.is_empty(), "so does removing the entry");
    }

    #[test]
    fn a_new_line_for_a_refused_machine_is_checked_again() {
        let mut app = two_machines();
        pinned_to(&mut app, BOX_A);
        pinned_to(&mut app, BOX_B);
        assert!(app.refused.contains_key(&MachineId(1)));

        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let (window, _daemon) = tokio::io::duplex(1024);
            let (read, write) = tokio::io::split(window);
            let mut connections = crate::client::Connections::default();
            let line = crate::client::link::LinkEvent::Line(
                MachineId(1),
                (
                    maitty_ipc_transport::reader(read),
                    maitty_ipc_transport::writer(write),
                ),
            );
            app.on_link_event(line, &mut connections);
            assert!(connections.is_connected(MachineId(1)));
        });
        assert!(app.refused.is_empty(), "the next State is judged afresh");
        pinned_to(&mut app, BOX_A);
        assert!(app.refused.is_empty(), "the right daemon is let in");
    }

    #[test]
    fn typing_dropped_while_a_machine_was_down_is_told_once_when_it_is_back() {
        use super::super::link::ConnectionState::*;
        let mut app = two_machines();
        let state = |app: &mut App, s| app.on_link_state(MachineId(1), "build".into(), s);
        state(&mut app, Connected);
        state(
            &mut app,
            Reconnecting {
                attempt: 1,
                since: std::time::Instant::now(),
            },
        );
        app.note_unsent_input(MachineId(1));
        app.note_unsent_input(MachineId(1));
        app.note_unsent_input(MachineId(1));
        state(&mut app, Connected);
        let (text, _, error) = app.notice.clone().expect("a toast");
        assert_eq!(
            text,
            "3 keystrokes to build were not sent while it was disconnected"
        );
        assert!(error);

        state(
            &mut app,
            Reconnecting {
                attempt: 1,
                since: std::time::Instant::now(),
            },
        );
        state(&mut app, Connected);
        assert_eq!(app.notice.as_ref().unwrap().0, "Connected to build");
    }

    #[test]
    fn a_machines_state_is_kept_for_the_screen_and_its_agents_stay_when_it_drops() {
        use super::super::link::ConnectionState::*;
        let mut app = two_machines();
        let kept = app.remotes[&MachineId(1)].snap.terms.len();
        assert!(kept > 0);
        app.on_link_state(
            MachineId(1),
            "build".into(),
            Unreachable {
                since: std::time::Instant::now(),
                why: "no route".into(),
            },
        );
        let remote = &app.remotes[&MachineId(1)];
        assert_eq!(remote.snap.terms.len(), kept, "greyed, not deleted");
        assert!(matches!(remote.state, Unreachable { .. }));
        assert_eq!(
            app.notice.as_ref().unwrap().0,
            "build is unreachable: no route"
        );
    }

    #[test]
    fn a_daemon_that_matches_its_recorded_id_is_accepted() {
        let mut app = two_machines();
        app.on_machine_server(MachineId(1), state_with_id(Some(BOX_A)));
        app.on_machine_server(MachineId(1), state_with_id(Some(BOX_A)));
        assert!(app.refused.is_empty());
        assert!(app.notice.is_none());
    }

    #[test]
    fn this_machine_is_never_checked() {
        let mut app = two_machines();
        app.on_machine_server(MachineId::LOCAL, state_with_id(Some(BOX_A)));
        app.on_machine_server(MachineId::LOCAL, state_with_id(Some(BOX_B)));
        assert!(app.refused.is_empty());
    }

    #[test]
    fn limits_spend_version_and_the_active_workspace_stay_per_machine() {
        let mut app = two_machines();
        app.snap.spent_today = 1.5;
        app.snap.daemon_version = "0.51.0".into();
        app.snap.limits = vec![("claude".into(), Vec::new())];
        let there = app.remotes.get_mut(&MachineId(1)).unwrap();
        there.snap.spent_today = 9.0;
        there.snap.daemon_version = "0.50.0".into();
        there.snap.limits.clear();
        there.snap.active_ws = Some(77);
        app.screen_fresh();
        app.screen_model();
        let machines = app.machines();
        assert_eq!(
            (machines[0].id, machines[1].id),
            (MachineId::LOCAL, MachineId(1))
        );
        assert_eq!(
            (machines[0].snap.spent_today, machines[1].snap.spent_today),
            (1.5, 9.0)
        );
        assert_eq!(
            (
                machines[0].snap.daemon_version.as_str(),
                machines[1].snap.daemon_version.as_str()
            ),
            ("0.51.0", "0.50.0")
        );
        assert_eq!(
            (machines[0].snap.limits.len(), machines[1].snap.limits.len()),
            (1, 0)
        );
        assert_ne!(app.snap.active_ws, Some(77));
        assert_eq!(app.snap_of(MachineId(1)).unwrap().active_ws, Some(77));
        assert_eq!(app.focused().map(|t| t.machine), Some(MachineId::LOCAL));
    }
}
