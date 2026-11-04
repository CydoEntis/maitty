//! What a message from another machine's daemon may do on this one. That daemon is someone
//! else's computer: it can forge any message, so each kind is decided here, before it reaches
//! the code that acts on it, and the default for anything unjustified is to ignore it.
//!
//! - Apply: changes only what the window keeps for that machine (its snapshot, its panes'
//!   screens), after cleaning.
//! - Gated: acts on this computer (clipboard, toasts, window focus, sounds), so only when the
//!   machine's `[[machines]]` entry opts in. A machine nothing names has nothing on.
//! - Answer: replies to something this window sent that machine, so only while one is waiting.
//!   What it says is cleaned like any toast or screen text.
//! - Ignore: would touch this computer's state, UI or files, or was never asked for.

use crate::hidden::sanitize;
use crate::protocol::{Reply, ServerMsg};
use maitty_settings::config::MachineDef;

use super::remote_snapshot::{ITEMS_MAX, PANE_DIM_MAX, sanitize_snapshot, sanitize_terms};

/// Most bytes of a remote clipboard write that are honoured.
pub const CLIPBOARD_MAX_BYTES: usize = 64 * 1024;
/// Longest remote toast, in characters.
pub const NOTICE_MAX: usize = 300;
/// Most bytes of a remote reply's text (a check's output) that are kept.
pub const REPLY_TEXT_MAX: usize = 256 * 1024;

/// What a machine is allowed to do here. Raise (window focus) rides on `notices`: it is the
/// same "this machine may get your attention" grant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MachineOpts {
    pub clipboard: bool,
    pub notices: bool,
    pub sounds: bool,
}

impl MachineOpts {
    /// This computer's own daemon is trusted with all of it.
    pub const LOCAL: MachineOpts = MachineOpts {
        clipboard: true,
        notices: true,
        sounds: true,
    };

    /// The opt-ins of the entry called `name`; nothing on when none is.
    pub fn named(machines: &[MachineDef], name: &str) -> MachineOpts {
        machines
            .iter()
            .find(|m| !name.is_empty() && m.name.eq_ignore_ascii_case(name))
            .map_or(MachineOpts::default(), |m| MachineOpts {
                clipboard: m.clipboard,
                notices: m.notices,
                sounds: m.sounds,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Apply,
    Answer,
    Ignore(&'static str),
}

fn gate(on: bool, off: &'static str) -> Verdict {
    if on {
        Verdict::Apply
    } else {
        Verdict::Ignore(off)
    }
}

/// Whether a message from a machine that is not this one may be acted on. Every variant is
/// named, so a new message cannot be added without deciding here what a stranger may do with it.
pub fn allow(opts: &MachineOpts, msg: &ServerMsg) -> Verdict {
    match msg {
        ServerMsg::Welcome { .. }
        | ServerMsg::State(_)
        | ServerMsg::TermsChanged { .. }
        | ServerMsg::Replay { .. }
        | ServerMsg::Output { .. }
        // Queues that machine's alert; the sound and the toast are gated where they fire.
        | ServerMsg::Attention { .. }
        | ServerMsg::Bye => Verdict::Apply,
        ServerMsg::Clipboard { .. } => gate(opts.clipboard, "clipboard is off for this machine"),
        ServerMsg::Notice(_) => gate(opts.notices, "notices are off for this machine"),
        ServerMsg::Raise => gate(opts.notices, "notices are off for this machine"),
        ServerMsg::Error(_) | ServerMsg::Reply(_) => Verdict::Answer,
        ServerMsg::AskRemoveWorktree { .. } => {
            Verdict::Ignore("a worktree prompt names a folder on this computer")
        }
        ServerMsg::AutoWorkspace(_) => {
            Verdict::Ignore("a new workspace here is not another machine's to announce")
        }
    }
}

/// The message to act on, or `None` when `machine`'s daemon may not send it. What stays is
/// cleaned: the snapshot, sizes that allocate a screen, the clipboard text, the toast. `asked`
/// is whether this window has a request out to that machine that is still waiting for its answer.
pub fn vet(opts: &MachineOpts, asked: bool, machine: &str, msg: ServerMsg) -> Option<ServerMsg> {
    match allow(opts, &msg) {
        Verdict::Ignore(why) => {
            tracing::debug!(why, "ignored a message from another machine");
            return None;
        }
        Verdict::Answer if !asked => {
            tracing::debug!("ignored an answer to nothing this window asked");
            return None;
        }
        Verdict::Answer | Verdict::Apply => {}
    }
    Some(match msg {
        ServerMsg::State(snap) => match sanitize_snapshot(snap) {
            Ok(snap) => ServerMsg::State(snap),
            Err(why) => {
                tracing::debug!(why, "refused a snapshot from another machine");
                return None;
            }
        },
        ServerMsg::TermsChanged { changed, removed } => {
            if removed.len() > ITEMS_MAX {
                tracing::debug!("refused a pane delta from another machine: too many removals");
                return None;
            }
            match sanitize_terms(changed) {
                Ok(changed) => ServerMsg::TermsChanged { changed, removed },
                Err(why) => {
                    tracing::debug!(why, "refused a pane delta from another machine");
                    return None;
                }
            }
        }
        ServerMsg::Replay {
            term,
            cols,
            rows,
            data,
        } => ServerMsg::Replay {
            term,
            cols: cols.clamp(1, PANE_DIM_MAX),
            rows: rows.clamp(1, PANE_DIM_MAX),
            data,
        },
        ServerMsg::Clipboard { term, text } => ServerMsg::Clipboard {
            term,
            text: cut_bytes(text, CLIPBOARD_MAX_BYTES),
        },
        ServerMsg::Notice(text) => ServerMsg::Notice(tagged(machine, &text)),
        ServerMsg::Error(text) => ServerMsg::Error(tagged(machine, &text)),
        ServerMsg::Reply(Reply::Text(text)) => ServerMsg::Reply(Reply::Text(sanitize(
            &cut_bytes(text, REPLY_TEXT_MAX),
            true,
        ))),
        other => other,
    })
}

/// A remote's words for a toast: one clean line, cut short, led by the machine so the source
/// is never in doubt.
pub fn tagged(machine: &str, text: &str) -> String {
    let head: String = text.chars().take(NOTICE_MAX).collect();
    format!("[{}] {}", sanitize(machine, false), sanitize(&head, false))
}

fn cut_bytes(mut text: String, max: usize) -> String {
    if text.len() > max {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::App;
    use crate::client::ids::{ClientTerm, MachineId, Remote};
    use crate::client::reference_fixture::{Stage, world};
    use crate::protocol::{Reply, Snapshot, Status};
    use std::path::PathBuf;

    const THERE: MachineId = MachineId(1);

    fn nothing_on() -> MachineOpts {
        MachineOpts::default()
    }

    fn every_message() -> Vec<ServerMsg> {
        vec![
            ServerMsg::Welcome { version: 1 },
            ServerMsg::State(Snapshot::default()),
            ServerMsg::TermsChanged {
                changed: Vec::new(),
                removed: Vec::new(),
            },
            ServerMsg::Replay {
                term: 1,
                cols: 80,
                rows: 24,
                data: Vec::new(),
            },
            ServerMsg::Output {
                term: 1,
                data: Vec::new(),
            },
            ServerMsg::Attention {
                term: 1,
                status: Status::Blocked,
            },
            ServerMsg::Reply(Reply::Ok),
            ServerMsg::Reply(Reply::Text("t".into())),
            ServerMsg::Reply(Reply::Worktrees(Vec::new())),
            ServerMsg::Error("e".into()),
            ServerMsg::Notice("n".into()),
            ServerMsg::AskRemoveWorktree {
                path: PathBuf::from("/home/me/repo-wt"),
                name: "wt".into(),
                why: "unmerged".into(),
            },
            ServerMsg::Clipboard {
                term: 1,
                text: "x".into(),
            },
            ServerMsg::AutoWorkspace("api".into()),
            ServerMsg::Raise,
            ServerMsg::Bye,
        ]
    }

    fn label(msg: &ServerMsg) -> &'static str {
        match msg {
            ServerMsg::Welcome { .. } => "Welcome",
            ServerMsg::State(_) => "State",
            ServerMsg::TermsChanged { .. } => "TermsChanged",
            ServerMsg::Replay { .. } => "Replay",
            ServerMsg::Output { .. } => "Output",
            ServerMsg::Attention { .. } => "Attention",
            ServerMsg::Reply(_) => "Reply",
            ServerMsg::Error(_) => "Error",
            ServerMsg::Notice(_) => "Notice",
            ServerMsg::AskRemoveWorktree { .. } => "AskRemoveWorktree",
            ServerMsg::Clipboard { .. } => "Clipboard",
            ServerMsg::AutoWorkspace(_) => "AutoWorkspace",
            ServerMsg::Raise => "Raise",
            ServerMsg::Bye => "Bye",
        }
    }

    #[test]
    fn what_a_machine_with_nothing_on_may_send() {
        let mut applied = Vec::new();
        let mut answers = Vec::new();
        let mut ignored = Vec::new();
        for msg in every_message() {
            match allow(&nothing_on(), &msg) {
                Verdict::Apply => applied.push(label(&msg)),
                Verdict::Answer => answers.push(label(&msg)),
                Verdict::Ignore(_) => ignored.push(label(&msg)),
            }
        }
        applied.dedup();
        answers.dedup();
        ignored.dedup();
        assert_eq!(
            applied,
            [
                "Welcome",
                "State",
                "TermsChanged",
                "Replay",
                "Output",
                "Attention",
                "Bye"
            ]
        );
        assert_eq!(
            answers,
            ["Reply", "Error"],
            "a reply or error counts only as the answer to a request this window sent"
        );
        assert_eq!(
            ignored,
            [
                "Notice",
                "AskRemoveWorktree",
                "Clipboard",
                "AutoWorkspace",
                "Raise"
            ]
        );
    }

    #[test]
    fn opt_ins_open_only_their_own_message() {
        let allowed = |opts: MachineOpts| {
            let mut names: Vec<_> = every_message()
                .iter()
                .filter(|m| allow(&opts, m) == Verdict::Apply)
                .map(label)
                .collect();
            names.dedup();
            names
        };
        let base = allowed(nothing_on());
        let with = |names: &[&'static str]| {
            let mut all: Vec<&'static str> =
                base.iter().copied().chain(names.iter().copied()).collect();
            all.sort();
            all
        };
        let sorted = |mut v: Vec<&'static str>| {
            v.sort();
            v
        };
        let clipboard = MachineOpts {
            clipboard: true,
            ..nothing_on()
        };
        let notices = MachineOpts {
            notices: true,
            ..nothing_on()
        };
        let sounds = MachineOpts {
            sounds: true,
            ..nothing_on()
        };
        assert_eq!(sorted(allowed(clipboard)), with(&["Clipboard"]));
        assert_eq!(sorted(allowed(notices)), with(&["Notice", "Raise"]));
        assert_eq!(
            sorted(allowed(sounds)),
            sorted(base.clone()),
            "a sound is gated where it rings, not by a message"
        );
        for always_ignored in ["AskRemoveWorktree", "AutoWorkspace"] {
            assert!(
                !allowed(MachineOpts::LOCAL).contains(&always_ignored),
                "{always_ignored} is ignored even with everything on"
            );
        }
    }

    #[test]
    fn opt_ins_come_from_the_machines_entry_and_default_to_off() {
        let defs = vec![MachineDef {
            name: "build".into(),
            clipboard: true,
            ..MachineDef::default()
        }];
        let build = MachineOpts::named(&defs, "Build");
        assert_eq!(
            build,
            MachineOpts {
                clipboard: true,
                ..nothing_on()
            }
        );
        assert_eq!(MachineOpts::named(&defs, "stranger"), nothing_on());
        assert_eq!(MachineOpts::named(&defs, ""), nothing_on());
    }

    #[test]
    fn a_toast_is_one_clean_line_led_by_its_machine() {
        let hostile = format!("\u{1b}[2Jfake\nline{}", "x".repeat(1000));
        let toast = tagged("build", &hostile);
        assert!(toast.starts_with("[build] [2Jfake line"));
        assert!(toast.chars().count() <= "[build] ".len() + NOTICE_MAX);
        assert!(!toast.chars().any(char::is_control));
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
    fn a_remote_clipboard_write_is_ignored_unless_the_machine_opted_in() {
        let clip = || ServerMsg::Clipboard {
            term: 1,
            text: "from afar".into(),
        };
        let mut off = app_with_machine(|_| {});
        copied();
        off.on_machine_server(THERE, clip());
        assert_eq!(copied(), None, "off by default");

        let mut on = app_with_machine(|d| d.clipboard = true);
        on.on_machine_server(THERE, clip());
        assert_eq!(copied().as_deref(), Some("from afar"));
        assert!(
            on.notice
                .as_ref()
                .is_some_and(|(text, ..)| text.starts_with("[build] ")),
            "{:?}",
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
        ServerMsg::TermsChanged { changed, removed }
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
