//! The one-shot commands against another machine's real daemon behind the fake-ssh shim,
//! reached the way `--on` reaches it: a configured machine resolved by name (see TESTING,
//! "testing over ssh"). The commands run in-process; this computer's own daemon is never
//! dialled, and the pane id and token of the process running the tests are never used.

use super::remote_status_tests::{Runtime, Sandbox};
use crate::cli::machines_ls::{self, Listing};
use crate::cli::target::{Target, resolve_machine};
use crate::cli::{self, Waited};
use crate::config::Config;
use crate::ipc;
use crate::link_failure::{FIX_INSTALL, FIX_LOGIN, FIX_NO_DAEMON};
use crate::protocol::{ClientMsg, Query, Reply, Snapshot, TermId};
use maitty_settings::config::MachineDef;
use std::path::PathBuf;
use std::time::{Duration, Instant};

const WAIT: Duration = Duration::from_secs(30);

/// A real daemon on a private socket and the config that names it `build`.
pub(super) struct Build {
    pub(super) sandbox: Sandbox,
    pub(super) calls: PathBuf,
    pub(super) cfg: Config,
}

fn def(sandbox: &Sandbox, name: &str, mode: &str, calls: &std::path::Path) -> MachineDef {
    MachineDef {
        name: name.into(),
        target: format!("{name}.example"),
        ssh_args: vec![
            format!("--mode={mode}"),
            format!("--maitty={}", sandbox.maitty.display()),
            format!("--env=MAITTY_SOCKET={}", sandbox.socket),
            format!(
                "--env=MAITTY_DATA_DIR={}",
                sandbox.dir.join("data").display()
            ),
            format!(
                "--env=MAITTY_CONFIG={}",
                sandbox.dir.join("config.toml").display()
            ),
            format!("--calls={}", calls.display()),
        ],
        ..MachineDef::default()
    }
}

impl Build {
    /// `build` reaches the daemon; `others` adds a machine each, named and failing as given.
    pub(super) fn start(others: &[(&str, &str)]) -> Build {
        let sandbox = Sandbox::new();
        let calls = sandbox.dir.join("calls.txt");
        let mut machines = vec![def(&sandbox, "build", "proxy", &calls)];
        machines.extend(
            others
                .iter()
                .map(|(name, mode)| def(&sandbox, name, mode, &calls)),
        );
        let build = Build {
            cfg: Config {
                machines,
                ..Config::default()
            },
            calls,
            sandbox,
        };
        build.start_daemon();
        build
    }

    /// Configured machines only attach, so the daemon is started the way the window would.
    fn start_daemon(&self) {
        let runtime = Runtime::new();
        let halves = runtime.block_on(ipc::open_to(&self.sandbox.machine(), true));
        drop(halves.expect("start the daemon through the shim"));
    }

    pub(super) fn target(&self, name: &str) -> Target {
        let shim = self.sandbox.shim.to_string_lossy();
        Target::of(resolve_machine(name, &self.cfg, &shim).expect("a configured machine"))
    }

    pub(super) fn dials(&self) -> usize {
        std::fs::read_to_string(&self.calls).map_or(0, |text| text.lines().count())
    }
}

fn list(target: &Target) -> Snapshot {
    match cli::block_on(cli::request_to(target, ClientMsg::Query(Query::List))) {
        Ok(Reply::List(snap)) => snap,
        other => panic!("the machine did not list: {other:?}"),
    }
}

pub(super) fn until<T>(what: &str, mut found: impl FnMut() -> Option<T>) -> T {
    let start = Instant::now();
    loop {
        if let Some(value) = found() {
            return value;
        }
        assert!(start.elapsed() < WAIT, "timed out waiting for {what}");
        std::thread::sleep(Duration::from_millis(100));
    }
}

pub(super) fn panes(target: &Target) -> Vec<TermId> {
    list(target).terms.keys().copied().collect()
}

pub(super) fn open_workspace(target: &Target, name: &str) -> TermId {
    cli::new_workspace(target, None, Some(name.into()), Vec::new()).expect("open a workspace");
    until("the workspace's pane", || panes(target).first().copied())
}

#[test]
fn new_split_focus_popup_and_close_act_on_the_named_machine() {
    let build = Build::start(&[]);
    let target = build.target("build");
    let first = open_workspace(&target, "on-build");
    assert!(build.dials() > 0, "the commands went through the shim");

    cli::split(&target, Some(first), false, Vec::new()).expect("split");
    let both = until("two panes", || {
        Some(panes(&target)).filter(|p| p.len() == 2)
    });
    let second = *both.iter().find(|p| **p != first).unwrap();

    cli::focus(&target, first).expect("focus");
    let snap = list(&target);
    assert_eq!(
        snap.active().and_then(|w| w.tab()).map(|t| t.focus),
        Some(first)
    );

    cli::popup(&target, vec!["echo".into(), "popped".into()]).expect("popup");

    cli::close(&target, Some(second)).expect("close");
    until("the pane to close", || {
        (!panes(&target).contains(&second)).then_some(())
    });
}

#[test]
fn send_and_wait_use_the_remote_pane_s_screen() {
    let build = Build::start(&[]);
    let target = build.target("build");
    let pane = open_workspace(&target, "talk");

    cli::send(&target, Some(pane), "echo remote-marker-77".into(), true).expect("send");
    let matched =
        cli::wait_on(&target, pane, Some(r"^remote-marker-77\s*$"), WAIT, false).expect("wait");
    assert_eq!(matched, Waited::Matched("remote-marker-77".into()));
    cli::read(&target, Some(pane), false).expect("read");
}

#[test]
fn ls_tags_the_machine_and_writes_panes_as_addresses() {
    let build = Build::start(&[]);
    let target = build.target("build");
    let pane = open_workspace(&target, "tagged");
    let snap = list(&target);

    let text = cli::workspace_lines(&snap, &target);
    assert_eq!(text[0], "machine build");
    assert!(
        text.iter()
            .any(|l| l.contains(&format!("pane build/{pane} "))),
        "{text:?}"
    );
    let json = cli::workspaces_json(&snap, target.name());
    assert!(json.iter().all(|w| w["machine"] == "build"), "{json:?}");
    assert!(
        cli::workspaces_json(&snap, None)
            .iter()
            .all(|w| w.get("machine").is_none()),
        "no machine key unless one was named"
    );
}

#[test]
fn ls_all_lists_the_machine_that_answers_and_reports_the_one_that_does_not() {
    let build = Build::start(&[("gpu", "unreachable")]);
    let (good, dead) = (build.target("build"), build.target("gpu"));
    open_workspace(&good, "listed");

    let Listing { out, warnings } = machines_ls::list(&[good, dead], false).expect("one answered");
    assert!(out.starts_with("machine build\n"), "{out}");
    assert_eq!(warnings.len(), 1, "{warnings:?}");
    assert!(
        warnings[0].starts_with("gpu: unreachable ("),
        "{warnings:?}"
    );

    let (good, dead) = (build.target("build"), build.target("gpu"));
    let Listing { out, .. } = machines_ls::list(&[good, dead], true).expect("one answered");
    let items: serde_json::Value = serde_json::from_str(&out).unwrap();
    assert!(
        items
            .as_array()
            .unwrap()
            .iter()
            .all(|w| w["machine"] == "build"),
        "{out}"
    );
}

#[test]
fn ls_all_fails_when_no_machine_answers() {
    let build = Build::start(&[("gpu", "unreachable"), ("locked", "auth-fail")]);
    let error = machines_ls::list(&[build.target("gpu"), build.target("locked")], false)
        .err()
        .expect("nothing answered");
    let said = format!("{error:#}");
    assert!(said.contains("gpu: unreachable ("), "{said}");
    assert!(said.contains("locked: needs a login"), "{said}");
    assert!(said.contains("no machine answered"), "{said}");
}

#[test]
fn a_machine_that_cannot_be_reached_gets_one_line_with_its_fix() {
    let build = Build::start(&[("locked", "auth-fail"), ("bare", "not-installed")]);
    let locked = cli::ls(&build.target("locked"), false).unwrap_err();
    assert_eq!(
        format!("{locked:#}"),
        format!("locked: needs a login; {FIX_LOGIN}")
    );
    let bare = cli::ls(&build.target("bare"), false).unwrap_err();
    assert_eq!(
        format!("{bare:#}"),
        format!("bare: has no app; {FIX_INSTALL}")
    );
}

#[test]
fn a_machine_with_no_daemon_running_says_how_to_start_it() {
    let sandbox = Sandbox::new();
    let calls = sandbox.dir.join("calls.txt");
    let cfg = Config {
        machines: vec![def(&sandbox, "build", "proxy", &calls)],
        ..Config::default()
    };
    let shim = sandbox.shim.to_string_lossy().into_owned();
    let target = Target::of(resolve_machine("build", &cfg, &shim).unwrap());
    let error = cli::ls(&target, false).unwrap_err();
    assert_eq!(
        format!("{error:#}"),
        format!("build: is not running; {FIX_NO_DAEMON}")
    );
}

#[test]
fn a_pane_here_is_never_the_default_on_a_named_machine() {
    let build = Build::start(&[]);
    let target = build.target("build");
    let before = build.dials();
    let needs = "--on build needs --pane build/<id>";
    for result in [
        cli::read(&target, None, false),
        cli::send(&target, None, "hi".into(), true),
        cli::send_keys(&target, None, vec!["enter".into()]),
        cli::wait(&target, None, None, 1),
        cli::split(&target, None, false, Vec::new()),
        cli::close(&target, None),
        cli::teach(&target, None),
    ] {
        let said = format!("{:#}", result.unwrap_err());
        assert!(said.contains(needs), "{said}");
    }
    assert_eq!(build.dials(), before, "nothing was sent to the machine");
}

#[test]
fn kill_server_stops_the_named_machine_s_daemon() {
    let build = Build::start(&[]);
    let target = build.target("build");
    open_workspace(&target, "doomed");
    cli::kill_server(&target, false).expect("kill-server");
    until("the daemon to be gone", || {
        cli::ls(&target, false).err().map(|_| ())
    });
}
