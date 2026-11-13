#[cfg(test)]
mod tests {
    use crate::client::App;
    use crate::client::ids::{ClientTerm, MachineId, Remote};
    use crate::client::reference_fixture::{Stage, world};
    use crate::protocol::{Reply, ServerMsg, Status};
    use maitty_machines::MachineDef;
    use maitty_machines::remote_policy::*;
    use maitty_machines::remote_snapshot::{ITEMS_MAX, PANE_DIM_MAX};
    use std::path::PathBuf;

    const THERE: MachineId = MachineId(1);

    fn nothing_on() -> MachineOpts {
        MachineOpts::default()
    }

    fn app_with_machine(opts: impl FnOnce(&mut MachineDef)) -> App {
        let mut app = world(Stage::None);
        app.remotes.insert(
            THERE,
            Remote {
                name: "build".into(),
                ..Remote::default()
            },
        );
        let mut def = MachineDef {
            name: "build".into(),
            ..MachineDef::default()
        };
        opts(&mut def);
        app.cfg.machines = vec![def];
        app
    }

    fn copied() -> Option<String> {
        crate::client::copy::LAST_COPY.with(|c| c.borrow_mut().take())
    }

    #[test]
    fn a_remote_clipboard_write_goes_through_unless_the_entry_turns_it_off() {
        let clip = || ServerMsg::Clipboard {
            term: 1,
            text: "from afar".into(),
        };
        let mut off = app_with_machine(|d| d.clipboard = false);
        copied();
        off.on_machine_server(THERE, clip());
        assert_eq!(copied(), None, "off when the entry says so");
        // Said, not silent: nothing was copied, and the toast says how to allow it.
        let told = off.notice_card.clone().expect("a refused copy is told");
        assert!(
            off.notice
                .as_ref()
                .is_some_and(|(title, ..)| title.contains("tried to copy"))
                && told.detail.contains("not copied")
                && told
                    .preview
                    .as_deref()
                    .is_some_and(|p| p.contains("clipboard = true")),
            "{:?} {told:?}",
            off.notice
        );

        let mut on = app_with_machine(|d| d.clipboard = true);
        on.on_machine_server(THERE, clip());
        assert_eq!(copied().as_deref(), Some("from afar"));
        assert!(
            on.notice
                .as_ref()
                .is_some_and(|(title, ..)| title == "Copied to clipboard")
                && on
                    .notice_card
                    .as_ref()
                    .is_some_and(|c| c.detail.contains("on build")),
            "the same card as a local copy, naming the machine: {:?}",
            on.notice
        );
    }

    #[test]
    fn a_remote_clipboard_write_is_cut_to_the_cap() {
        let mut app = app_with_machine(|d| d.clipboard = true);
        let big = "é".repeat(CLIPBOARD_MAX_BYTES);
        app.on_machine_server(THERE, ServerMsg::Clipboard { term: 1, text: big });
        let got = copied().unwrap();
        assert!(got.len() <= CLIPBOARD_MAX_BYTES && got.len() > CLIPBOARD_MAX_BYTES - 4);
    }

    #[test]
    fn this_computers_own_clipboard_write_is_unaffected() {
        let mut app = app_with_machine(|_| {});
        copied();
        app.on_server(ServerMsg::Clipboard {
            term: 1,
            text: "mine".into(),
        });
        assert_eq!(copied().as_deref(), Some("mine"));
        assert!(
            app.notice
                .as_ref()
                .is_some_and(|(text, ..)| !text.starts_with('['))
        );
    }

    #[test]
    fn a_remote_notice_shows_only_when_opted_in_and_names_its_machine() {
        let notice = || ServerMsg::Notice("\u{1b}[31mdeploy done".into());
        let mut off = app_with_machine(|_| {});
        off.notice = None;
        off.on_machine_server(THERE, notice());
        assert!(off.notice.is_none());

        let mut on = app_with_machine(|d| d.notices = true);
        on.notice = None;
        on.on_machine_server(THERE, notice());
        assert_eq!(on.notice.unwrap().0, "[build] [31mdeploy done");
    }

    #[test]
    fn a_remote_notice_does_not_touch_this_computers_undo_hint() {
        let mut app = app_with_machine(|d| d.notices = true);
        app.undo_hint = true;
        app.on_machine_server(THERE, ServerMsg::Notice("hi".into()));
        assert!(app.undo_hint);
    }

    #[test]
    fn a_remote_raise_needs_the_notices_opt_in() {
        // Raising the window is a no-op under test; what matters is the verdict it gets.
        assert_eq!(
            allow(&nothing_on(), &ServerMsg::Raise),
            Verdict::Ignore("notices are off for this machine")
        );
        let on = MachineOpts {
            notices: true,
            ..nothing_on()
        };
        assert_eq!(allow(&on, &ServerMsg::Raise), Verdict::Apply);
    }

    #[test]
    fn a_remote_alert_rings_only_with_sounds_and_toasts_only_with_notices() {
        let term = ClientTerm::on(THERE, 1);
        let blocked = |app: &mut App| {
            let mut snap = world(Stage::None).snap.clone();
            let t = snap.terms.values().next().unwrap().clone();
            snap.terms.clear();
            snap.terms.insert(
                1,
                crate::protocol::TermInfo {
                    id: 1,
                    status: Status::Blocked,
                    ..t
                },
            );
            app.on_machine_server(THERE, ServerMsg::State(snap));
            app.notice = None;
            app.last_alert = None;
            app.alert_now(term, Status::Blocked);
        };
        let mut none = app_with_machine(|_| {});
        blocked(&mut none);
        assert!(none.notice.is_none() && none.last_alert.is_none());

        let mut sound = app_with_machine(|d| d.sounds = true);
        blocked(&mut sound);
        assert!(sound.last_alert.is_some() && sound.notice.is_none());

        let mut toast = app_with_machine(|d| d.notices = true);
        blocked(&mut toast);
        assert!(toast.last_alert.is_none());
        assert!(
            toast
                .notice
                .as_ref()
                .is_some_and(|(t, ..)| t.starts_with("[build] ") && t.ends_with("needs you")),
            "{:?}",
            toast.notice
        );
    }

    #[test]
    fn a_machine_that_connects_with_toasts_off_says_so_once_and_its_alerts_still_raise_nothing() {
        let term = ClientTerm::on(THERE, 1);
        let state = || {
            let mut snap = world(Stage::None).snap.clone();
            let t = snap.terms.values().next().unwrap().clone();
            snap.terms.clear();
            snap.terms.insert(1, crate::protocol::TermInfo { id: 1, status: Status::Blocked, ..t });
            snap
        };
        let mut app = app_with_machine(|_| {});
        app.notice = None;
        app.on_machine_server(THERE, ServerMsg::State(state()));
        let said = app.notice.as_ref().map(|(text, ..)| text.clone()).unwrap_or_default();
        assert!(said.contains("Toasts from build are off") && said.contains("press n"), "{said}");

        // Said once for the machine this run; an alert from it still raises nothing.
        app.notice = None;
        app.on_machine_server(THERE, ServerMsg::State(state()));
        app.alert_now(term, Status::Blocked);
        assert!(app.notice.is_none(), "{:?}", app.notice);

        // A machine that was granted toasts needs no explaining.
        let mut granted = app_with_machine(|d| d.notices = true);
        granted.notice = None;
        granted.on_machine_server(THERE, ServerMsg::State(state()));
        assert!(granted.notice.is_none(), "{:?}", granted.notice);
    }

    #[test]
    fn a_remote_worktree_prompt_never_opens() {
        let mut app = app_with_machine(|d| {
            d.clipboard = true;
            d.notices = true;
            d.sounds = true;
        });
        app.on_machine_server(
            THERE,
            ServerMsg::AskRemoveWorktree {
                path: PathBuf::from("/home/me/precious"),
                name: "precious".into(),
                why: "unmerged".into(),
            },
        );
        assert!(matches!(app.mode, crate::client::Mode::Normal));
    }

    #[test]
    fn a_remote_reply_error_and_new_workspace_do_not_reach_this_computers_ui() {
        let mut app = app_with_machine(|d| d.notices = true);
        app.notice = None;
        app.undo_hint = false;
        for msg in [
            ServerMsg::Reply(Reply::Worktrees(Vec::new())),
            ServerMsg::Reply(Reply::Text("output".into())),
            ServerMsg::Error("boom".into()),
            ServerMsg::AutoWorkspace("api".into()),
        ] {
            app.on_machine_server(THERE, msg);
        }
        assert!(app.notice.is_none() && !app.undo_hint);
    }

    #[test]
    fn a_remote_replay_cannot_ask_for_an_enormous_screen() {
        let mut app = app_with_machine(|_| {});
        app.on_machine_server(
            THERE,
            ServerMsg::Replay {
                term: 9,
                cols: u16::MAX,
                rows: u16::MAX,
                data: b"hi".to_vec(),
            },
        );
        assert_eq!(
            app.sizes[&ClientTerm::on(THERE, 9)],
            (PANE_DIM_MAX, PANE_DIM_MAX)
        );
    }

    #[test]
    fn a_hostile_snapshot_from_a_machine_renders_safely() {
        let mut app = app_with_machine(|_| {});
        let hostile = "\u{1b}[2J\u{1b}]52;c;AAAA\u{7}\u{202e}\0\u{200b}";
        let mut snap = world(Stage::None).snap.clone();
        for t in snap.terms.values_mut() {
            t.title = format!("{hostile}{}", "T".repeat(10 * 1024 * 1024));
            t.name = format!("{hostile}name");
            t.label = format!("{hostile}label");
            t.summary = hostile.into();
            t.branch = Some(format!("{hostile}main"));
            t.cwd = PathBuf::from("..\\..\\x\u{1b}[31m");
            t.root = Some(PathBuf::from("/etc/passwd"));
            t.top = Some(PathBuf::from(r"\\evil\share"));
            t.cols = u16::MAX;
        }
        for w in &mut snap.workspaces {
            w.name = format!("{hostile}ws");
            w.group = Some(hostile.into());
        }
        app.on_machine_server(THERE, ServerMsg::State(snap));

        let kept = &app.remotes[&THERE].snap;
        assert!(!kept.terms.is_empty());
        for t in kept.terms.values() {
            for text in [&t.title, &t.name, &t.label, &t.summary] {
                assert!(!text.chars().any(char::is_control), "{text:?}");
            }
            assert!(t.title.chars().count() <= super::super::remote_snapshot::LINE_MAX);
            assert!(t.cols <= PANE_DIM_MAX);
        }
        app.screen_fresh();
        let model = app.screen_model();
        let drawn = format!("{model:?}");
        assert!(
            !drawn.contains("\\u{1b}") && !drawn.contains("\\u{202e}") && !drawn.contains("\\0"),
            "the model carries no control or hidden character"
        );
        assert!(
            drawn.contains("AAAAlabel"),
            "the machine's cleaned label is in the model"
        );
    }

    #[test]
    fn a_snapshot_that_cannot_be_made_safe_is_not_kept() {
        let mut app = app_with_machine(|_| {});
        let before = app.remotes[&THERE].snap.terms.len();
        let mut snap = world(Stage::None).snap.clone();
        let t = snap.terms.values_mut().next().unwrap();
        t.cpu = f32::NAN;
        app.on_machine_server(THERE, ServerMsg::State(snap));
        assert_eq!(app.remotes[&THERE].snap.terms.len(), before);
    }

    fn a_pane(id: u32) -> crate::protocol::TermInfo {
        let app = world(Stage::None);
        let t = app.snap.terms.values().next().unwrap().clone();
        crate::protocol::TermInfo { id, ..t }
    }

    fn delta(changed: Vec<crate::protocol::TermInfo>, removed: Vec<u32>) -> ServerMsg {
        ServerMsg::TermsChanged { changed, removed, revision: 0 }
    }

    #[test]
    fn a_remote_pane_delta_lands_in_that_machines_snapshot_and_not_this_computers() {
        let mut app = app_with_machine(|_| {});
        let local: Vec<_> = app.snap.terms.keys().copied().collect();
        app.on_machine_server(THERE, delta(vec![a_pane(77), a_pane(78)], Vec::new()));
        app.on_machine_server(THERE, delta(vec![a_pane(79)], vec![77, 4040]));
        let there: Vec<_> = app.remotes[&THERE].snap.terms.keys().copied().collect();
        assert_eq!(there, [78, 79]);
        assert_eq!(app.snap.terms.keys().copied().collect::<Vec<_>>(), local);
        assert!(app.parsers.contains_key(&ClientTerm::on(THERE, 78)));
        assert!(
            !app.parsers
                .contains_key(&ClientTerm::on(MachineId::LOCAL, 78))
        );
    }

    #[test]
    fn this_computers_own_pane_delta_is_merged_into_its_snapshot() {
        let mut app = app_with_machine(|_| {});
        let first = *app.snap.terms.keys().next().unwrap();
        let mut renamed = a_pane(first);
        renamed.name = "renamed".into();
        app.on_server(delta(vec![renamed, a_pane(900)], Vec::new()));
        assert_eq!(app.snap.terms[&first].name, "renamed");
        assert!(app.snap.terms.contains_key(&900));
        app.on_server(delta(Vec::new(), vec![900, 12345]));
        assert!(!app.snap.terms.contains_key(&900));
        assert!(app.remotes[&THERE].snap.terms.is_empty());
    }

    #[test]
    fn a_hostile_remote_pane_delta_is_cleaned() {
        let mut app = app_with_machine(|_| {});
        let hostile = "\u{1b}[2J\u{1b}]52;c;AAAA\u{7}\u{202e}\0\u{200b}";
        let mut t = a_pane(5);
        t.title = format!("{hostile}{}", "T".repeat(10 * 1024 * 1024));
        t.name = format!("{hostile}name");
        t.cols = u16::MAX;
        t.rows = 0;
        t.cpu = 900.0;
        app.on_machine_server(THERE, delta(vec![t], Vec::new()));
        let kept = &app.remotes[&THERE].snap.terms[&5];
        for text in [&kept.title, &kept.name] {
            assert!(!text.chars().any(char::is_control), "{text:?}");
            assert!(!text.contains('\u{202e}') && !text.contains('\u{200b}'));
        }
        assert!(kept.title.chars().count() <= super::super::remote_snapshot::LINE_MAX);
        assert_eq!((kept.cols, kept.rows), (PANE_DIM_MAX, 1));
        assert_eq!(kept.cpu, 100.0);
    }

    #[test]
    fn a_remote_pane_delta_that_cannot_be_made_safe_is_dropped_whole() {
        let mut app = app_with_machine(|_| {});
        app.on_machine_server(THERE, delta(vec![a_pane(1)], Vec::new()));
        let mut nan = a_pane(2);
        nan.cpu = f32::NAN;
        app.on_machine_server(THERE, delta(vec![a_pane(3), nan], vec![1]));
        let kept: Vec<_> = app.remotes[&THERE].snap.terms.keys().copied().collect();
        assert_eq!(kept, [1], "the last good snapshot stands");

        let too_many = (0..=ITEMS_MAX as u32).map(a_pane).collect();
        app.on_machine_server(THERE, delta(too_many, Vec::new()));
        app.on_machine_server(THERE, delta(Vec::new(), (0..=ITEMS_MAX as u32).collect()));
        let kept: Vec<_> = app.remotes[&THERE].snap.terms.keys().copied().collect();
        assert_eq!(kept, [1]);
    }

    #[test]
    fn endless_remote_pane_deltas_cannot_grow_a_snapshot_past_the_cap() {
        let mut app = app_with_machine(|_| {});
        for round in 0..3u32 {
            let start = round * ITEMS_MAX as u32;
            let batch = (start..start + ITEMS_MAX as u32).map(a_pane).collect();
            app.on_machine_server(THERE, delta(batch, Vec::new()));
            assert!(app.remotes[&THERE].snap.terms.len() <= ITEMS_MAX);
        }
    }

    #[test]
    fn a_refused_machines_pane_deltas_are_dropped() {
        let mut app = app_with_machine(|_| {});
        app.on_machine_server(THERE, delta(vec![a_pane(1)], Vec::new()));
        app.refused
            .insert(THERE, "a different machine answers".into());
        app.on_machine_server(THERE, delta(vec![a_pane(2)], vec![1]));
        let kept: Vec<_> = app.remotes[&THERE].snap.terms.keys().copied().collect();
        assert_eq!(kept, [1]);
    }

    #[test]
    fn a_remote_pane_delta_is_allowed_for_a_machine_with_nothing_on() {
        assert_eq!(
            allow(&nothing_on(), &delta(Vec::new(), Vec::new())),
            Verdict::Apply
        );
    }
}
