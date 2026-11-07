//! A remote agent's status as its own daemon reports it. A real daemon runs behind the fake-ssh
//! shim (see TESTING, "testing over ssh"); a pane in it reports through the same `hook` command
//! an agent uses, and the window under test takes the daemon's states as it would over ssh.
//! What the window lists for that machine must be the daemon's word, with no step of its own.

use super::*;
use crate::client::connections::Connections;
use crate::client::screen::Proj;
use std::path::Path;
use std::process::{Command as Process, Stdio};

const WAIT: Duration = Duration::from_secs(20);
const BOX: MachineId = MachineId(1);

/// A daemon's worth of environment: its own socket, data folder and config.
pub(super) struct Sandbox {
    pub(super) dir: PathBuf,
    pub(super) socket: String,
    pub(super) maitty: PathBuf,
    pub(super) shim: PathBuf,
}

impl Sandbox {
    pub(super) fn new() -> Sandbox {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let socket = format!("remote-status-{}-{n}", std::process::id());
        let dir = std::env::temp_dir().join(&socket);
        std::fs::create_dir_all(dir.join("data")).expect("create the sandbox");
        let (maitty, shim) = built();
        Sandbox {
            dir,
            socket,
            maitty,
            shim,
        }
    }

    /// The machine behind the shim whose daemon is this sandbox's.
    pub(super) fn machine(&self) -> ipc::Machine {
        ipc::Machine::ssh(
            "box",
            vec![
                self.shim.to_string_lossy().into_owned(),
                "--mode=proxy".into(),
                format!("--maitty={}", self.maitty.display()),
                format!("--env=MAITTY_SOCKET={}", self.socket),
                format!("--env=MAITTY_DATA_DIR={}", self.dir.join("data").display()),
                format!(
                    "--env=MAITTY_CONFIG={}",
                    self.dir.join("config.toml").display()
                ),
            ],
            "maitty",
        )
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        // Must not panic: a failed test is already unwinding.
        let _ = Process::new(&self.maitty)
            .arg("kill-server")
            .env("MAITTY_SOCKET", &self.socket)
            .env("MAITTY_DATA_DIR", self.dir.join("data"))
            .env("MAITTY_CONFIG", self.dir.join("config.toml"))
            .env_remove("MAITTY_TERM_ID")
            .env_remove("MAITTY_PANE_TOKEN")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = std::fs::remove_dir_all(&self.dir);
    }
}

/// The `maitty` binary and the shim, built here: `cargo test -p <this crate>` builds neither.
pub(super) fn built() -> (PathBuf, PathBuf) {
    static BUILT: std::sync::OnceLock<(PathBuf, PathBuf)> = std::sync::OnceLock::new();
    BUILT
        .get_or_init(|| {
            let exe = std::env::current_exe().expect("the test's own path");
            let profile = exe
                .parent()
                .and_then(Path::parent)
                .expect("target/<profile>");
            let suffix = std::env::consts::EXE_SUFFIX;
            let maitty = profile.join(format!("maitty{suffix}"));
            let shim = profile.join("examples").join(format!("fake-ssh{suffix}"));
            let cargo = std::env::var_os("CARGO").unwrap_or_else(|| "cargo".into());
            let status = Process::new(cargo)
                .args([
                    "build",
                    "--package",
                    "maitty",
                    "--bin",
                    "maitty",
                    "--example",
                    "fake-ssh",
                ])
                .status()
                .expect("run cargo");
            assert!(
                status.success(),
                "building maitty and the fake-ssh shim failed"
            );
            (maitty, shim)
        })
        .clone()
}

/// The window under test, fed from the daemon's connection the way the event loop feeds it.
struct Window {
    app: App,
    connections: Connections,
}

impl Window {
    async fn connect(sandbox: &Sandbox) -> Window {
        let (tx, _rx) = mpsc::unbounded_channel();
        let (bg_tx, _bg_rx) = mpsc::unbounded_channel();
        let mut app = App::new(Config::default(), tx, None, bg_tx);
        app.splash = false;
        app.remotes.insert(
            BOX,
            Remote {
                name: "box".into(),
                // The sidebar lists a machine's sessions only while its line is up.
                state: crate::client::link::ConnectionState::Connected,
                ..Remote::default()
            },
        );
        let mut connections = Connections::default();
        let halves = ipc::open_to(&sandbox.machine(), true)
            .await
            .expect("connect through the shim");
        connections.add(BOX, halves);
        Window { app, connections }
    }

    fn send(&self, msg: ClientMsg) {
        self.connections
            .send(Outgoing { machine: BOX, msg })
            .expect("the machine is connected");
    }

    /// Block until the pane's shell runs what is typed. A login shell can still be running its
    /// profile when the pane exists, and input typed then is dropped, so each round types a
    /// fresh probe and waits briefly for its output.
    async fn wait_for_shell(&mut self, term: TermId) {
        const PROBE: Duration = Duration::from_secs(2);
        let start = std::time::Instant::now();
        for round in 0.. {
            let marker = format!("shell-ready-{round}");
            self.send(ClientMsg::Input {
                term,
                data: format!("echo {marker}\r").into_bytes(),
            });
            let seen = |app: &App| {
                app.parsers
                    .values()
                    .any(|p| p.screen().contents().lines().any(|l| l.trim() == marker))
            };
            let probe = async {
                while !seen(&self.app) {
                    let burst = self.connections.recv_burst(64).await;
                    assert!(!burst.lost, "the line was lost waiting for the shell");
                    for msg in burst.msgs {
                        self.app.on_machine_server(BOX, msg);
                    }
                }
            };
            if tokio::time::timeout(PROBE, probe).await.is_ok() {
                return;
            }
            assert!(
                start.elapsed() < WAIT,
                "timed out waiting for the pane's shell to run a command"
            );
        }
    }

    /// Take what the daemon says until `done` holds of the window.
    async fn until(&mut self, what: &str, done: impl Fn(&App) -> bool) {
        let run = async {
            while !done(&self.app) {
                let burst = self.connections.recv_burst(64).await;
                assert!(!burst.lost, "the line was lost waiting for {what}");
                for msg in burst.msgs {
                    self.app.on_machine_server(BOX, msg);
                }
            }
        };
        if tokio::time::timeout(WAIT, run).await.is_err() {
            let screens: Vec<String> = self
                .app
                .parsers
                .values()
                .map(|p| p.screen().contents())
                .collect();
            panic!("timed out waiting for {what}; the pane shows {screens:?}");
        }
    }
}

/// A runtime that is never waited for on drop, not even when the test fails: on Windows its
/// blocking threads read the pipes of the daemon the shim started, and that outlives the test.
pub(super) struct Runtime(Option<tokio::runtime::Runtime>);

impl Runtime {
    pub(super) fn new() -> Runtime {
        Runtime(Some(
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("a runtime"),
        ))
    }

    pub(super) fn block_on<T>(&self, future: impl std::future::Future<Output = T>) -> T {
        self.0.as_ref().expect("a runtime").block_on(future)
    }
}

impl Drop for Runtime {
    fn drop(&mut self) {
        if let Some(runtime) = self.0.take() {
            runtime.shutdown_background();
        }
    }
}

/// What the window lists for the machine's one pane: its name, and the status of its row.
fn listed(app: &App) -> Option<(String, Status)> {
    app.screen_model()
        .iter()
        .filter(|p| p.machine == BOX)
        .flat_map(Proj::sessions)
        .next()
        .map(|s| (s.name.clone(), s.status))
}

fn reported(app: &App) -> Option<Status> {
    app.remotes[&BOX]
        .snap
        .terms
        .values()
        .next()
        .map(|t| t.status)
}

/// Type `maitty ask-human` in the pane: the daemon marks the pane as asking and keeps it so
/// until the question is answered. The path is spelled with forward slashes and no quotes,
/// which every shell the daemon starts runs.
fn ask_in_pane(sandbox: &Sandbox, term: TermId, question: &str) -> ClientMsg {
    let exe = sandbox.maitty.to_string_lossy().replace('\\', "/");
    assert!(
        !exe.contains(' '),
        "the test types the path unquoted: {exe}"
    );
    ClientMsg::Input {
        term,
        data: format!("{exe} ask-human \"{question}\"\r").into_bytes(),
    }
}

#[test]
fn a_remote_agents_row_is_what_its_daemon_reported_and_follows_its_next_report() {
    let sandbox = Sandbox::new();
    let runtime = Runtime::new();
    runtime.block_on(async {
        let mut window = Window::connect(&sandbox).await;
        window.send(ClientMsg::Command(Command::NewWorkspace {
            cwd: None,
            name: Some("report".into()),
            cmd: None,
        }));
        window.until("a pane", |app| reported(app).is_some()).await;
        let term = *window.app.remotes[&BOX].snap.terms.keys().next().unwrap();

        window.wait_for_shell(term).await;

        window.send(ask_in_pane(&sandbox, term, "use redis or memcached?"));
        window
            .until("the daemon to say it asks", |app| {
                reported(app) == Some(Status::Blocked)
            })
            .await;
        let (name, status) = listed(&window.app).expect("the machine's row");
        assert_eq!(
            status,
            Status::Blocked,
            "the row says what the daemon said: {name}"
        );
        assert_eq!(
            window.app.snap.terms.len(),
            0,
            "none of it is this computer's"
        );
        let asked = window.app.remotes[&BOX].snap.questions[0].clone();
        let drawn = frame_text(&mut window.app);
        assert!(drawn.contains("▤ box"), "its machine heads its fleets: {drawn}");

        window.send(ClientMsg::Command(Command::AnswerHuman {
            id: asked.id,
            choice: 0,
        }));
        window
            .until("the daemon to say it no longer asks", |app| {
                reported(app).is_some_and(|status| status != Status::Blocked)
            })
            .await;
        assert_eq!(
            listed(&window.app).map(|(_, status)| status),
            reported(&window.app),
            "the second report arrives and updates the row"
        );
        assert!(!frame_text(&mut window.app).contains("asks: use red"));
    });
}

/// The window drawn at 100x30 as text, one string.
fn frame_text(app: &mut App) -> String {
    let mut terminal =
        ratatui::Terminal::new(ratatui::backend::TestBackend::new(160, 45)).expect("a terminal");
    terminal.draw(|f| render::draw(app, f)).expect("a frame");
    let text = terminal
        .backend()
        .buffer()
        .content()
        .iter()
        .map(|c| c.symbol())
        .collect::<String>();
    if std::env::var(concat!(env!("APP_ENV"), "_SHOW")).is_ok() {
        println!("{text}");
    }
    text
}
