//! The Machines dialog's choices end to end, against the fake-ssh shim and real daemons: adding
//! from the picker, the look at a machine before it is added, the first login in a popup,
//! removing, and retrying. Nothing here touches the user's config: each test writes its own
//! `config.local.toml` in a sandbox.

use super::*;
use crate::client::connections::Connections;
use crate::client::ids::Outgoing;
use crate::client::link::{Backoff, Links, Timing};
use crate::client::machine_ids::{IdCheck, check_daemon_id};
use crate::client::reference_fixture::term;
use crate::client::remote_status_tests::{Runtime, Sandbox};
use crate::client::screen::machines::{MachinesView, Outcome};
use crate::client::ssh_hosts::SshHost;
use crate::link_failure::{FIX_INSTALL, FIX_LOGIN};
use crate::protocol::{ServerMsg, SessionKey, Snapshot, Status};
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use std::path::Path;
use std::process::Command as Process;

const WAIT: Duration = Duration::from_secs(60);

fn quick() -> Timing {
    Timing {
        connect: Duration::from_secs(20),
        backoff: Backoff {
            first: Duration::from_millis(10),
            most: Duration::from_millis(40),
        },
    }
}

/// The shim in `mode` as the program new machines are reached with, recording its calls.
fn prefix(sandbox: &Sandbox, mode: &str, calls: &Path) -> Vec<String> {
    vec![
        sandbox.shim.to_string_lossy().into_owned(),
        format!("--mode={mode}"),
        format!("--maitty={}", sandbox.maitty.display()),
        format!("--calls={}", calls.display()),
        format!("--env=MAITTY_SOCKET={}", sandbox.socket),
        format!(
            "--env=MAITTY_DATA_DIR={}",
            sandbox.dir.join("data").display()
        ),
        format!(
            "--env=MAITTY_CONFIG={}",
            sandbox.dir.join("config.toml").display()
        ),
    ]
}

fn host(alias: &str) -> SshHost {
    SshHost {
        alias: alias.into(),
        host_name: None,
        user: None,
    }
}

struct Harness {
    app: App,
    bg: mpsc::UnboundedReceiver<Bg>,
    out: mpsc::UnboundedReceiver<Outgoing>,
    sent: Vec<Outgoing>,
    sandbox: Sandbox,
    links: Option<Links>,
    connections: Connections,
}

impl Harness {
    /// A window whose new machines are reached through the shim in `mode`.
    fn new(mode: &str) -> Harness {
        let sandbox = Sandbox::new();
        let (tx, out) = mpsc::unbounded_channel();
        let (bg_tx, bg) = mpsc::unbounded_channel();
        let mut app = App::new(Config::default(), tx, None, bg_tx);
        app.splash = false;
        app.machine_flow.local_config = sandbox.dir.join("config.local.toml");
        app.machine_flow.ssh = prefix(&sandbox, mode, &sandbox.dir.join("ssh-calls.txt"));
        Harness {
            app,
            bg,
            out,
            sent: Vec::new(),
            sandbox,
            links: None,
            connections: Connections::default(),
        }
    }

    /// The same, with this sandbox's daemon running and supervisors that start at once. Runs
    /// inside the runtime.
    async fn live(mode: &str) -> Harness {
        let mut h = Harness::new(mode);
        let started = ipc::open_to(&h.sandbox.machine(), true).await;
        assert!(started.is_ok(), "starting the daemon: {:?}", started.err());
        drop(started);
        let links = Links::start(Vec::new(), quick(), || ipc::terminal_dial(true));
        h.app.links = links.control();
        h.links = Some(links);
        h
    }

    fn calls(&self) -> PathBuf {
        self.sandbox.dir.join("ssh-calls.txt")
    }

    fn open(&mut self, hosts: Vec<SshHost>) {
        self.app.mode = Mode::Machines(Box::new(MachinesView {
            hosts: Some(hosts),
            ..MachinesView::new(false)
        }));
    }

    fn view(&self) -> &MachinesView {
        match &self.app.mode {
            Mode::Machines(v) => v,
            _ => panic!("the dialog is not open"),
        }
    }

    fn key(&mut self, code: KeyCode) {
        self.app.on_key(KeyEvent::new(code, KeyModifiers::NONE));
    }

    fn type_text(&mut self, text: &str) {
        text.chars().for_each(|c| self.key(KeyCode::Char(c)));
    }

    fn local_config(&self) -> Option<String> {
        std::fs::read_to_string(&self.app.machine_flow.local_config).ok()
    }

    /// What the dialog says, or the toast when it is closed (a menu in the sidebar answers there).
    fn notice(&self) -> String {
        match &self.app.mode {
            Mode::Machines(v) => v.notice.as_ref().map(|n| n.text.clone()),
            _ => self.app.notice.as_ref().map(|(text, ..)| text.clone()),
        }
        .unwrap_or_default()
    }

    fn take_sent(&mut self) {
        while let Ok(out) = self.out.try_recv() {
            self.sent.push(out);
        }
    }

    /// The popup commands asked of a daemon so far, with the machine each was sent to.
    fn popups(&mut self) -> Vec<(MachineId, String)> {
        self.take_sent();
        self.sent
            .iter()
            .filter_map(|o| match &o.msg {
                ClientMsg::Command(Command::Popup { cmd, .. }) => Some((o.machine, cmd.clone())),
                _ => None,
            })
            .collect()
    }

    /// Wait (outside a runtime) for the next background result and take it in.
    fn next_bg(&mut self) {
        let started = Instant::now();
        loop {
            if let Ok(b) = self.bg.try_recv() {
                return self.app.on_bg(b);
            }
            assert!(started.elapsed() < WAIT, "no background result came");
            std::thread::sleep(Duration::from_millis(10));
        }
    }

    fn state_of(&self, name: &str) -> Option<ConnectionState> {
        self.app
            .remotes
            .values()
            .find(|r| r.name == name)
            .map(|r| r.state.clone())
    }

    fn connected(&self, name: &str) -> bool {
        self.state_of(name).is_some_and(|s| s.is_connected())
    }

    fn id_of(&self, name: &str) -> Option<MachineId> {
        self.app.links.id_of(name)
    }

    async fn pump(&mut self) {
        let Harness {
            app,
            bg,
            links,
            connections,
            ..
        } = self;
        let links = links.as_mut().expect("a live window");
        tokio::select! {
            Some(b) = bg.recv() => app.on_bg(b),
            event = links.next(), if links.any() => app.on_link_event(event, connections),
            burst = connections.recv_burst(64) => {
                for msg in burst.msgs {
                    app.on_machine_server(burst.machine, msg);
                }
            }
            () = tokio::time::sleep(Duration::from_millis(25)) => {}
        }
        for machine in app.take_dropped_machines() {
            connections.remove(machine);
        }
        self.take_sent();
    }

    async fn until(&mut self, what: &str, done: impl Fn(&Harness) -> bool) {
        let run = async {
            while !done(self) {
                self.pump().await;
            }
        };
        if tokio::time::timeout(WAIT, run).await.is_err() {
            let states: Vec<_> = self.app.remotes.values().map(|r| &r.state).collect();
            panic!(
                "timed out waiting for {what}; states {states:?}, notice {:?}",
                self.view_notice()
            );
        }
    }

    fn view_notice(&self) -> Option<String> {
        match &self.app.mode {
            Mode::Machines(v) => v.notice.as_ref().map(|n| n.text.clone()),
            _ => None,
        }
    }

    /// Add the host `alias` the way a person does: type it, Enter.
    fn add_by_enter(&mut self, alias: &str) {
        self.open(vec![host(alias)]);
        // Down from the input is the first host; Enter connects to it.
        self.key(KeyCode::Down);
        self.key(KeyCode::Enter);
    }
}

fn alive(pid: u32) -> bool {
    #[cfg(windows)]
    {
        let out = Process::new("tasklist")
            .args(["/FI", &format!("PID eq {pid}"), "/NH", "/FO", "CSV"])
            .output()
            .expect("run tasklist");
        String::from_utf8_lossy(&out.stdout).contains(&format!("\"{pid}\""))
    }
    #[cfg(unix)]
    {
        Process::new("kill")
            .args(["-0", &pid.to_string()])
            .output()
            .is_ok_and(|o| o.status.success())
    }
}

/// The pid of the shim's last call, once it has recorded one.
fn shim_pid(calls: &Path) -> u32 {
    let started = Instant::now();
    loop {
        if let Some(pid) = std::fs::read_to_string(calls)
            .ok()
            .and_then(|t| t.lines().last().map(str::to_owned))
            .and_then(|l| l.split_whitespace().next().map(str::to_owned))
            .and_then(|p| p.parse().ok())
        {
            return pid;
        }
        assert!(started.elapsed() < WAIT, "the shim never ran");
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn gone(pid: u32) -> bool {
    let started = Instant::now();
    while started.elapsed() < Duration::from_secs(20) {
        if !alive(pid) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    false
}

fn machine_entries(config: &str) -> Vec<(String, String)> {
    let table: toml::Table = toml::from_str(config).expect("a config that parses");
    table
        .get("machines")
        .and_then(|m| m.as_array())
        .map(|list| {
            list.iter()
                .map(|m| {
                    (
                        m["name"].as_str().unwrap().to_owned(),
                        m["target"].as_str().unwrap().to_owned(),
                    )
                })
                .collect()
        })
        .unwrap_or_default()
}

// ---- adding: what is written, and what is refused first ------------------------------------

#[test]
fn adding_from_the_picker_writes_one_entry_after_the_existing_ones_and_never_the_synced_file() {
    let mut h = Harness::new("proxy");
    let synced = h.sandbox.dir.join("config.toml");
    std::fs::write(&synced, "# shared between computers\ntheme = \"night\"\n").unwrap();
    let local = "# mine\n[[machines]]\nname = 'old'\ntarget = 'old.example'\n";
    std::fs::write(&h.app.machine_flow.local_config, local).unwrap();
    h.app.cfg.machines = vec![MachineDef {
        name: "old".into(),
        target: "old.example".into(),
        ..MachineDef::default()
    }];
    h.open(vec![host("build")]);
    h.key(KeyCode::Down);
    h.key(KeyCode::Enter);
    assert!(
        matches!(h.view().step, MachineStep::Checking { .. }),
        "the machine is looked at before anything is written"
    );
    assert_eq!(h.local_config().as_deref(), Some(local));
    h.next_bg();
    let written = h.local_config().unwrap();
    assert!(written.starts_with(local), "{written}");
    assert_eq!(
        machine_entries(&written),
        [
            ("old".to_string(), "old.example".to_string()),
            ("build".to_string(), "build".to_string())
        ],
        "name and target are the alias, so ssh's own config for it still applies"
    );
    assert_eq!(written.matches("[[machines]]").count(), 2);
    assert_eq!(
        std::fs::read_to_string(&synced).unwrap(),
        "# shared between computers\ntheme = \"night\"\n",
        "the synced file is byte for byte as it was"
    );
    assert_eq!(h.app.cfg.machines.len(), 2);
    assert_eq!(h.app.cfg.machines[1].target, "build");
    assert!(h.popups().is_empty(), "a host that logs in needs no popup");
    assert!(h.notice().contains("Added build"), "{}", h.notice());
    assert!(
        matches!(&h.view().step, MachineStep::Added { name, how: Outcome::Ready(_), .. } if name == "build"),
        "the dialog shows the checklist done"
    );
}

#[test]
fn a_typed_host_is_added_under_a_name_made_from_it() {
    let mut h = Harness::new("proxy");
    h.open(Vec::new());
    h.type_text("dev@box.example.com");
    h.key(KeyCode::Enter);
    h.next_bg();
    assert_eq!(
        machine_entries(&h.local_config().unwrap()),
        [(
            "box.example.com".to_string(),
            "dev@box.example.com".to_string()
        )]
    );
}

#[test]
fn a_name_that_is_taken_gets_a_number() {
    let mut h = Harness::new("proxy");
    h.app.cfg.machines = vec![MachineDef {
        name: "Build".into(),
        target: "elsewhere".into(),
        ..MachineDef::default()
    }];
    h.open(vec![host("build")]);
    h.key(KeyCode::Down);
    h.key(KeyCode::Enter);
    h.next_bg();
    assert_eq!(
        machine_entries(&h.local_config().unwrap()),
        [("build-2".to_string(), "build".to_string())]
    );
}

#[test]
fn a_typed_target_that_ssh_would_read_as_an_option_is_refused_before_anything_runs() {
    for target in ["-oProxyCommand=x", "-F/etc/passwd", "box other"] {
        let mut h = Harness::new("proxy");
        h.open(Vec::new());
        h.type_text(target);
        h.key(KeyCode::Enter);
        let notice = h.notice();
        assert!(
            notice.contains("starts with `-`") || notice.contains("whitespace"),
            "{target:?}: {notice}"
        );
        assert!(h.view().notice.as_ref().unwrap().error);
        assert!(matches!(h.view().step, MachineStep::Browse));
        assert!(h.local_config().is_none(), "{target:?} wrote a file");
        assert!(h.app.cfg.machines.is_empty());
        assert!(h.bg.try_recv().is_err(), "{target:?} ran ssh");
        assert!(!h.calls().exists(), "{target:?} ran ssh");
    }
}

#[test]
fn a_target_with_no_name_in_it_asks_for_a_name_and_refuses_a_bad_one_inline() {
    let mut h = Harness::new("proxy");
    h.open(Vec::new());
    h.type_text("@@@");
    h.key(KeyCode::Enter);
    assert!(matches!(&h.view().step, MachineStep::Name { name, .. } if name.is_empty()));
    h.type_text("bad name");
    h.key(KeyCode::Enter);
    let MachineStep::Name { error, .. } = &h.view().step else {
        panic!("still asking for a name");
    };
    assert!(
        error
            .as_deref()
            .is_some_and(|e| e.contains("`name` is up to"))
    );
    assert!(h.local_config().is_none());
    for _ in 0.."bad name".len() {
        h.key(KeyCode::Backspace);
    }
    h.type_text("dash");
    h.key(KeyCode::Enter);
    h.next_bg();
    assert_eq!(
        machine_entries(&h.local_config().unwrap()),
        [("dash".to_string(), "@@@".to_string())]
    );
}

// ---- the look at a machine before it is added ----------------------------------------------

#[test]
fn a_bad_host_shows_ssh_s_error_adds_nothing_and_does_not_hang() {
    let mut h = Harness::new("unreachable");
    h.add_by_enter("box");
    h.next_bg();
    assert!(h.notice().contains("Connection refused"), "{}", h.notice());
    assert!(h.view().notice.as_ref().unwrap().error);
    assert!(matches!(h.view().step, MachineStep::Browse));
    assert!(h.local_config().is_none() && h.app.cfg.machines.is_empty());
}

#[test]
fn a_probe_that_hangs_is_ended_at_the_limit_and_ssh_is_gone() {
    let mut h = Harness::new("hang");
    h.app.machine_flow.probe_limit = Duration::from_millis(400);
    h.add_by_enter("box");
    h.next_bg();
    assert!(h.notice().contains("no answer within"), "{}", h.notice());
    assert!(gone(shim_pid(&h.calls())), "ssh was left running");
    assert!(h.local_config().is_none() && h.app.cfg.machines.is_empty());
}

#[test]
fn escape_ends_a_probe_that_is_waiting_and_what_it_finds_later_is_ignored() {
    let mut h = Harness::new("hang");
    h.add_by_enter("box");
    let pid = shim_pid(&h.calls());
    assert!(alive(pid), "the probe is running");
    h.key(KeyCode::Esc);
    assert!(matches!(h.view().step, MachineStep::Browse));
    assert!(h.notice().contains("Cancelled"), "{}", h.notice());
    assert!(gone(pid), "ssh was left running");
    h.next_bg();
    assert!(h.app.cfg.machines.is_empty() && h.local_config().is_none());
    assert!(matches!(h.app.mode, Mode::Machines(_)), "the dialog stays");
}

#[test]
fn a_login_that_needs_a_person_opens_ssh_in_a_popup_on_this_computer() {
    let mut h = Harness::new("auth-fail");
    h.add_by_enter("box");
    h.next_bg();
    let popups = h.popups();
    assert_eq!(popups.len(), 1, "{popups:?}");
    let (machine, cmd) = &popups[0];
    assert_eq!(
        *machine,
        MachineId::LOCAL,
        "ssh runs here, not on the remote"
    );
    assert!(!cmd.contains("BatchMode"), "it must be able to ask: {cmd}");
    for kept in ["ForwardAgent=no", "ConnectTimeout=15", " -a ", " -T "] {
        assert!(cmd.contains(kept), "{kept:?} missing: {cmd}");
    }
    assert!(
        cmd.ends_with("-- box 'maitty --version'"),
        "`--` comes before the target and the command is last: {cmd}"
    );
    assert_eq!(
        machine_entries(&h.local_config().unwrap()),
        [("box".to_string(), "box".to_string())],
        "the machine is configured while it waits for the login"
    );
    assert!(
        matches!(h.app.mode, Mode::Normal) && !h.app.shortcut_held,
        "the dialog closes so the popup has the keys"
    );
    assert!(h.app.machine_login_waiting("box"));
    assert!(
        h.app
            .notice
            .as_ref()
            .is_some_and(|(text, ..)| text.contains("use the popup")),
        "a toast says where to look"
    );
}

#[test]
fn a_remote_command_that_needs_quoting_cannot_use_the_popup_and_says_how_to_log_in() {
    let mut h = Harness::new("auth-fail");
    h.app.cfg.machines.clear();
    let def = MachineDef {
        name: "odd".into(),
        target: "box".into(),
        remote_cmd: Some("/opt/my tools/maitty".into()),
        ..MachineDef::default()
    };
    let machine = ipc::Machine::from_def_via(&def, &h.app.machine_flow.ssh);
    assert!(machine.first_login_command().is_err());
}

#[test]
fn a_missing_app_adds_the_machine_so_its_state_can_say_what_to_install() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("not-installed").await;
        h.add_by_enter("box");
        h.until("the machine to say what it needs", |h| {
            matches!(h.state_of("box"), Some(ConnectionState::NeedsUpdate { .. }))
        })
        .await;
        assert_eq!(
            h.state_of("box"),
            Some(ConnectionState::NeedsUpdate {
                why: FIX_INSTALL.into()
            })
        );
        assert!(h.popups().is_empty());
        assert_eq!(machine_entries(&h.local_config().unwrap()).len(), 1);
    });
}

// ---- live: connecting, the popup's end, removing, retrying ---------------------------------

#[test]
fn a_key_based_host_is_added_in_one_step_shows_connecting_at_once_and_connects() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("proxy").await;
        h.add_by_enter("box");
        h.until("the machine to be added", |h| {
            !h.app.cfg.machines.is_empty()
        })
        .await;
        assert_eq!(
            h.state_of("box"),
            Some(ConnectionState::Connecting),
            "its group is in the sidebar model the moment it is added"
        );
        assert!(h.id_of("box").is_some(), "a supervisor runs for it");
        h.until("the machine to connect", |h| h.connected("box"))
            .await;
        assert!(
            h.popups().is_empty(),
            "no popup for a host that works with keys"
        );
        assert_eq!(machine_entries(&h.local_config().unwrap()).len(), 1);
    });
}

fn popup_snapshot(open: bool) -> Snapshot {
    let mut login = term(7, "login", "", Status::Idle, Path::new("/"), 0);
    login.popup = true;
    Snapshot {
        terms: if open {
            [(7, login)].into_iter().collect()
        } else {
            Default::default()
        },
        ..Snapshot::default()
    }
}

#[test]
fn when_the_login_popup_closes_the_machine_connects_with_batch_mode() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("auth-fail").await;
        h.add_by_enter("box");
        h.until("the popup to be asked for", |h| {
            h.app.machine_login_waiting("box")
        })
        .await;
        assert!(
            h.id_of("box").is_none(),
            "no supervisor dials before the login"
        );

        // The person logs in; ssh now works without a prompt.
        let calls = h.calls();
        h.app.machine_flow.ssh = prefix(&h.sandbox, "proxy", &calls);
        h.app
            .on_machine_server(MachineId::LOCAL, ServerMsg::State(popup_snapshot(true)));
        assert!(h.app.machine_login_waiting("box"), "the popup is open");
        assert!(h.id_of("box").is_none(), "still no dial while it is open");
        assert_eq!(
            h.app.typing_to(),
            Some(ClientTerm::local(7)),
            "typing goes to the popup, so ssh's prompts can be answered"
        );
        h.app
            .on_key(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        h.take_sent();
        assert!(
            h.sent.iter().any(|o| o.machine == MachineId::LOCAL
                && matches!(&o.msg, ClientMsg::Input { term: 7, data } if data == b"x")),
            "a key typed now reaches the popup's pane"
        );
        h.app
            .on_machine_server(MachineId::LOCAL, ServerMsg::State(popup_snapshot(false)));
        assert!(!h.app.machine_login_waiting("box"));
        h.until("the machine to connect", |h| h.connected("box"))
            .await;
    });
}

#[test]
fn a_popup_that_closes_on_a_failed_login_leaves_the_machine_with_its_state_and_the_fix() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("auth-fail").await;
        h.add_by_enter("box");
        h.until("the popup to be asked for", |h| {
            h.app.machine_login_waiting("box")
        })
        .await;
        h.app
            .on_machine_server(MachineId::LOCAL, ServerMsg::State(popup_snapshot(true)));
        h.app
            .on_machine_server(MachineId::LOCAL, ServerMsg::State(popup_snapshot(false)));
        h.until("needs login", |h| {
            matches!(h.state_of("box"), Some(ConnectionState::NeedsLogin { .. }))
        })
        .await;
        assert_eq!(
            h.state_of("box"),
            Some(ConnectionState::NeedsLogin {
                why: FIX_LOGIN.into()
            })
        );
        assert_eq!(h.app.cfg.machines.len(), 1, "it stays configured");
    });
}

#[test]
fn a_popup_that_is_never_seen_does_not_leave_the_machine_undialled_for_ever() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("auth-fail").await;
        h.add_by_enter("box");
        h.until("the popup to be asked for", |h| {
            h.app.machine_login_waiting("box")
        })
        .await;
        h.app.expire_unseen_logins();
        assert!(
            h.app.machine_login_waiting("box"),
            "not yet: it may still show up"
        );
        h.app.machine_flow.logins[0].since = Instant::now()
            .checked_sub(POPUP_UNSEEN_FOR)
            .expect("an instant that far back");
        h.app.expire_unseen_logins();
        assert!(!h.app.machine_login_waiting("box") && h.id_of("box").is_some());
    });
}

async fn terms_the_daemon_has(sandbox: &Sandbox) -> usize {
    let halves = ipc::open_to(&sandbox.machine(), true)
        .await
        .expect("a second connection to the daemon");
    let mut connections = Connections::default();
    connections.add(MachineId(9), halves);
    let look = async {
        loop {
            for msg in connections.recv_burst(16).await.msgs {
                if let ServerMsg::State(snap) = msg {
                    return snap.terms.len();
                }
            }
        }
    };
    tokio::time::timeout(WAIT, look)
        .await
        .expect("the daemon's state")
}

#[test]
fn removing_forgets_the_machine_here_and_leaves_its_sessions_and_the_saved_layout_alone() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("proxy").await;
        h.add_by_enter("box");
        h.until("the machine to connect", |h| h.connected("box"))
            .await;
        let id = h.id_of("box").unwrap();
        h.connections
            .send(Outgoing {
                machine: id,
                msg: ClientMsg::Command(Command::NewWorkspace {
                    cwd: None,
                    name: Some("work".into()),
                    cmd: None,
                }),
            })
            .expect("the machine is connected");
        h.until("a pane there", |h| {
            h.app
                .remotes
                .get(&id)
                .is_some_and(|r| !r.snap.terms.is_empty())
        })
        .await;
        assert!(
            h.app.screen_model().iter().any(|p| p.machine == id),
            "the machine's folder is in the sidebar"
        );
        h.app
            .screen
            .saved
            .session_order_mut("box")
            .push(SessionKey::Pane(5));
        assert!(
            check_daemon_id(&mut h.app.machine_ids, "box", Some("ab")).is_err(),
            "its daemon id was recorded"
        );

        // From the machine's line in the sidebar: its right-click menu.
        h.app.menu_for_machine(id, (5, 5));
        assert!(
            h.local_config().unwrap().contains("[[machines]]"),
            "opening the menu removes nothing"
        );
        let Mode::ScreenMenu(m) = &h.app.mode else {
            panic!("the machine's menu")
        };
        let remove = m.items.iter().position(|(l, _)| l == "Remove").unwrap();
        assert!(
            m.items.iter().any(|(l, _)| l == "Reconnect") && remove == m.items.len() - 1,
            "reconnect, then remove last"
        );
        h.app.menu_pick(remove);
        h.pump().await;

        let kept = h.local_config().unwrap_or_default();
        assert!(!kept.contains("box"), "{kept}");
        assert!(h.app.cfg.machines.is_empty());
        assert!(!h.app.links.supervises(id), "its supervisor is stopped");
        assert!(h.app.remotes.is_empty() && h.app.refused.is_empty());
        assert!(
            h.app.screen_model().iter().all(|p| p.machine != id),
            "its folders are out of the sidebar"
        );
        assert_eq!(
            check_daemon_id(&mut h.app.machine_ids, "box", Some("cd")),
            Ok(IdCheck::Recorded),
            "the next daemon at that name is taken as the machine"
        );
        assert_eq!(
            h.app.screen.saved.session_order_of("box"),
            [SessionKey::Pane(5)],
            "the saved layout lane is kept"
        );
        assert_eq!(
            h.notice(),
            "Removes box from this list. Its sessions keep running on box."
        );
        assert!(
            !h.connections.is_connected(id),
            "the connection is let go of"
        );
        assert!(
            terms_the_daemon_has(&h.sandbox).await >= 1,
            "the remote daemon's pane is still alive"
        );
    });
}

#[test]
fn retry_clears_a_refusal_and_dials_again() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("proxy").await;
        h.add_by_enter("box");
        h.until("the machine to connect", |h| h.connected("box"))
            .await;
        let id = h.id_of("box").unwrap();
        let dials = |h: &Harness| {
            std::fs::read_to_string(h.calls())
                .unwrap_or_default()
                .lines()
                .count()
        };
        let before = dials(&h);

        let why = "a different machine answers at box".to_string();
        h.app.links.refuse(id, why.clone());
        h.app.refused.insert(id, why);
        h.until("the refusal to hold the machine", |h| {
            matches!(h.state_of("box"), Some(ConnectionState::Unreachable { .. }))
        })
        .await;
        assert_eq!(dials(&h), before, "a refused machine is not dialled");

        h.app.menu_for_machine(id, (5, 5));
        let Mode::ScreenMenu(m) = &h.app.mode else {
            panic!("the machine's menu")
        };
        let reconnect = m.items.iter().position(|(l, _)| l == "Reconnect").unwrap();
        h.app.menu_pick(reconnect);
        assert!(h.app.refused.is_empty(), "the refusal is cleared");
        assert_eq!(h.notice(), "Trying box again.");
        h.until("the machine to connect again", |h| h.connected("box"))
            .await;
        assert_eq!(dials(&h), before + 1, "it dialled once more");
    });
}

#[test]
fn retry_on_a_connected_machine_says_so_and_a_machine_with_no_supervisor_gets_one() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("proxy").await;
        h.app.cfg.machines = vec![MachineDef {
            name: "box".into(),
            target: "box".into(),
            ..MachineDef::default()
        }];
        h.open(Vec::new());
        assert_eq!(h.app.machine_retry("box").text, "Connecting to box.");
        h.until("the machine to connect", |h| h.connected("box"))
            .await;
        assert_eq!(h.app.machine_retry("box").text, "box is connected.");
        assert!(h.app.machine_retry("nobody").error);
    });
}

// ---- your machines in the dialog: the arrows, reconnect, update, remove ----------------------

#[test]
fn the_dialog_lists_your_machines_and_its_keys_update_and_remove_the_chosen_one() {
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut h = Harness::live("not-installed").await;
        h.add_by_enter("box");
        h.until("the machine to say what it needs", |h| {
            matches!(h.state_of("box"), Some(ConnectionState::NeedsUpdate { .. }))
        })
        .await;

        // Up from the input is your machines, Down from the last of them is the input again.
        h.open(Vec::new());
        assert_eq!(h.view().machine_sel, None, "the keys start in the input");
        h.key(KeyCode::Up);
        assert_eq!(h.view().machine_sel, Some(0));
        h.key(KeyCode::Down);
        assert_eq!(h.view().machine_sel, None);
        h.type_text("r");
        assert_eq!(h.view().query, "r", "in the input a letter is typed");

        // `u` runs the update in a popup on this computer, with --install for a machine that
        // has no app, and the machine is dialled when the popup closes.
        h.open(Vec::new());
        h.key(KeyCode::Up);
        h.type_text("u");
        let popups = h.popups();
        let (machine, cmd) = popups.last().expect("a popup");
        assert_eq!(*machine, MachineId::LOCAL);
        assert!(cmd.ends_with(" machine update box --install"), "{cmd}");
        assert!(
            !matches!(h.app.mode, Mode::Machines(_)),
            "the popup takes the keys, so the dialog is closed"
        );
        assert!(h.notice().starts_with("Updating box"), "{}", h.notice());
        assert!(h.app.machine_login_waiting("box"));

        // `r` dials it again and says so in the dialog.
        h.open(Vec::new());
        h.key(KeyCode::Up);
        h.type_text("r");
        assert_eq!(h.notice(), "Trying box again.");
        assert_eq!(h.view().machine_sel, Some(0), "the keys stay on the machine");

        // `e` asks where the machine is. A target ssh would read as an option is refused inline
        // and nothing is written; a good one is saved in place and dialled at once.
        h.type_text("e");
        assert!(
            matches!(&h.view().step, MachineStep::Edit { target, .. } if target == "box"),
            "the field starts with where it is now"
        );
        for _ in 0.."box".len() {
            h.key(KeyCode::Backspace);
        }
        h.type_text("-oProxyCommand=x");
        h.key(KeyCode::Enter);
        assert!(
            matches!(&h.view().step, MachineStep::Edit { error: Some(_), .. }),
            "refused where it was typed"
        );
        assert_eq!(
            machine_entries(&h.local_config().unwrap()),
            [("box".to_string(), "box".to_string())]
        );
        for _ in 0.."-oProxyCommand=x".len() {
            h.key(KeyCode::Backspace);
        }
        let old_id = h.id_of("box").expect("its connection");
        h.type_text("me@box");
        h.key(KeyCode::Enter);
        assert_eq!(h.notice(), "box now connects to me@box. Connecting.");
        assert_eq!(h.view().step, MachineStep::Browse);
        assert_eq!(h.view().machine_sel, Some(0), "the keys are back on the machine");
        assert_eq!(
            machine_entries(&h.local_config().unwrap()),
            [("box".to_string(), "me@box".to_string())]
        );
        assert_eq!(h.app.cfg.machines[0].target, "me@box");
        let new_id = h.id_of("box").expect("it is dialled at the new address");
        assert_ne!(new_id, old_id, "the old connection was let go");
        assert!(!h.app.remotes.contains_key(&old_id));

        // `x` removes it from the config; with none left the keys are back in the input.
        h.type_text("x");
        assert!(h.notice().starts_with("Removes box from this list."));
        assert_eq!(h.view().machine_sel, None);
        assert!(machine_entries(&h.local_config().unwrap()).is_empty());
        assert!(h.app.cfg.machines.is_empty());
    });
}
