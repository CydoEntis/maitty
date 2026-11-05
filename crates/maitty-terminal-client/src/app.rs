use crate::brand::APP_COMMAND;
use crate::cli::target::{Choice, PaneAddress, Target, TargetError};
use crate::{alert, cli, client, config, ipc, protocol, restart, reveal, sync};
use clap::{Parser, Subcommand};
use std::path::PathBuf;

/// An agent-aware terminal multiplexer.
#[derive(Parser)]
#[command(name = env!("APP_COMMAND"), version = env!("APP_VERSION"), about)]
struct Args {
    /// Directory to open as a workspace (switches to it if already open).
    path: Option<PathBuf>,
    /// Work on another machine over SSH (user@host): the UI here, agents there. Needs
    /// this app installed there too.
    #[arg(long, global = true)]
    remote: Option<String>,
    /// Run the command on a configured machine ([[machines]] in config.local.toml) instead of
    /// this computer; `local` is this computer. A pane there is written `machine/id`
    /// (`--pane build/4`). Works with ls, read, send, wait, split, handoff, new, focus, close,
    /// grant, teach, popup and kill-server.
    #[arg(long, global = true, value_name = "MACHINE")]
    on: Option<String>,
    #[command(subcommand)]
    cmd: Option<Cmd>,
}

#[derive(Subcommand)]
enum Cmd {
    /// Start, submit to, or read a daemon-owned structured Codex conversation.
    Conversation {
        #[command(subcommand)]
        cmd: cli::conversation::Cmd,
    },
    /// Run one command with a secret the user gave this pane.
    Secret {
        #[command(subcommand)]
        cmd: SecretCmd,
    },
    /// Link a pane to a pull request.
    Pr {
        #[command(subcommand)]
        cmd: PrCmd,
    },
    /// Attach to the running server (starting it if needed).
    Attach { path: Option<PathBuf> },
    /// Run the server in the foreground (normally started automatically).
    Daemon,
    /// List workspaces, tabs and panes.
    Ls {
        #[arg(long)]
        json: bool,
        /// This computer and every configured machine; one that is down is reported on stderr.
        #[arg(long)]
        all: bool,
    },
    /// Print a pane's screen. Another machine's screen loses its escape sequences unless --raw.
    Read {
        pane: Option<PaneAddress>,
        /// Print another machine's screen exactly as it sent it.
        #[arg(long)]
        raw: bool,
    },
    /// Type text into a pane and press Enter.
    Send {
        #[arg(long, short)]
        pane: Option<PaneAddress>,
        /// Don't press Enter afterwards.
        #[arg(long)]
        no_enter: bool,
        /// Send key presses instead of text, e.g. `--key ctrl+space --key %` (no Enter).
        #[arg(long = "key", short = 'k')]
        keys: Vec<String>,
        /// Wait for the agent to finish its turn, then print its reply.
        #[arg(long)]
        wait: bool,
        /// Give up waiting after this many seconds (exit code 2).
        #[arg(long, default_value_t = 1800)]
        timeout: u64,
        text: Vec<String>,
    },
    /// Wait for an agent to finish its turn (or need you), or for text on its screen.
    /// Prints the reply (or the matching line). Exit code 2 on timeout.
    Wait {
        #[arg(long, short)]
        pane: Option<PaneAddress>,
        /// Wait for this regex on the screen instead.
        #[arg(long)]
        regex: Option<String>,
        #[arg(long, default_value_t = 1800)]
        timeout: u64,
    },
    /// Split a pane, optionally running a command in the new one.
    Split {
        #[arg(long, short)]
        pane: Option<PaneAddress>,
        /// Split downwards instead of to the right.
        #[arg(long, short)]
        down: bool,
        #[arg(trailing_var_arg = true)]
        command: Vec<String>,
    },
    /// Start another agent beside a pane, in the same checkout, with a short text brief of
    /// what the pane was doing (task, last prompt, branch, changed files, recent output) as
    /// its first prompt. Text only: the new agent gets none of the old one's conversation,
    /// memory or tool results. The old pane keeps running.
    Handoff {
        pane: PaneAddress,
        /// The agent to start: a [quick] agent or a known agent kind (claude, codex, …).
        #[arg(long)]
        to: String,
    },
    /// Open a new workspace.
    New {
        path: Option<PathBuf>,
        #[arg(long, short)]
        name: Option<String>,
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Focus a pane (switches workspace and tab).
    Focus { pane: PaneAddress },
    /// The program in a pane (default: this one) is an agent: learn it, by its
    /// program (any alias) or, run by node / python, by its script.
    Teach { pane: Option<PaneAddress> },
    /// Run a command in a floating pane over everything (it closes when the command exits):
    /// `popup -- fzf`, `popup -- lazygit`.
    Popup {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, required = true)]
        cmd: Vec<String>,
    },
    /// What a pane's agent may do through the app: a comma list of read, write, start, respond,
    /// admin (`grant 4 read,write`), or `default` for the config's ([mcp] grants).
    Grant { pane: PaneAddress, grants: String },
    /// Ask the person a question and wait for their answer (printed): from an agent in a
    /// pane. It shows in the Inbox and on the pane, with these answers (default Yes / No).
    AskHuman {
        question: String,
        /// An allowed answer (repeat for each).
        #[arg(short = 'o', long = "option")]
        options: Vec<String>,
    },
    /// Go to a session and bring the app's window forward: what clicking a notification runs.
    /// Takes a link from a notification, or a pane number.
    Reveal { target: String },
    /// Close a pane.
    Close { pane: Option<PaneAddress> },
    /// Report agent lifecycle from an agent's hook. Reads the hook JSON from stdin.
    Hook {
        /// Hook dialect: claude, codex, or any name with --status.
        agent: String,
        /// Explicit status: working, blocked, done, idle, gone.
        #[arg(long)]
        status: Option<String>,
        /// Extra payload (codex `notify` passes JSON as an argument).
        payload: Option<String>,
    },
    /// Install status hooks into an agent's config (every installed agent, and MCP, when none is named).
    Integrate {
        /// claude, codex, gemini, qwen, pi, omp, opencode, mastracode, kilo, kimi or mcp
        agent: Option<String>,
        #[arg(long)]
        uninstall: bool,
    },
    /// Config helpers.
    Config {
        #[command(subcommand)]
        cmd: ConfigCmd,
    },
    /// Create a git worktree for a branch and open it as a workspace.
    Worktree {
        /// Branch for the worktree (with --move, optional: named for you).
        #[arg(default_value = "")]
        branch: String,
        /// Move the agent running this command into the new worktree: it restarts there,
        /// resumed, when its turn ends. For agents to run themselves.
        #[arg(long = "move")]
        move_here: bool,
        /// Start a new branch from this ref (default: current HEAD).
        #[arg(long)]
        base: Option<String>,
        /// Workspace whose repo to use (default: this pane's, else the active one).
        #[arg(long)]
        ws: Option<protocol::WsId>,
        /// Command for the new workspace's first pane, e.g. `-- claude`.
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Close a worktree workspace and remove its checkout.
    WorktreeRemove {
        #[arg(long)]
        ws: Option<protocol::WsId>,
        /// Remove even with uncommitted changes.
        #[arg(long)]
        force: bool,
    },
    /// Stop the server and every pane. The session is kept for restore unless --forget.
    /// On another machine (--on) it stops all of that machine's sessions, so it needs --yes.
    KillServer {
        #[arg(long)]
        forget: bool,
        /// Confirm stopping another machine's server.
        #[arg(long)]
        yes: bool,
    },
    /// Show a desktop notification and play the needs-you and done sounds, to check them.
    TestAlert,
    /// Debugging: print the colours a pane's program draws.
    #[command(hide = true)]
    DebugColors { pane: u32 },
    /// Check that everything the app relies on is in place, and how to fix what isn't.
    Doctor,
    /// Run a command as one of your accounts: an agent entry in config with an `env` of its
    /// own (`account "claude work" -- claude`). Agents on such an account start this way.
    Account {
        name: String,
        #[arg(last = true)]
        command: Vec<String>,
    },
    /// Install or update the app on a configured machine ([[machines]]) over ssh.
    Machine {
        #[command(subcommand)]
        cmd: MachineCmd,
    },
    /// (Run over ssh by `machine update`) restart this machine's server if no session is open.
    #[command(hide = true)]
    RestartIfIdle,
    /// Install the newest release over this one.
    Update {
        /// Only say whether a newer version is out.
        #[arg(long)]
        check: bool,
        /// Reinstall the latest release even if this is it (repairs an install).
        #[arg(long)]
        force: bool,
    },
    /// Let this repo's project-file hooks run (they don't until you allow them).
    Allow {
        /// The repo (default: here).
        dir: Option<PathBuf>,
    },
    /// (Claude Code's status line) hand what the session used to the app, then show the
    /// status line you had before.
    #[command(hide = true)]
    Statusline,
    /// (Run by `--remote` over ssh) connect stdin/stdout to this machine's server.
    #[command(hide = true)]
    Proxy {
        /// Fail instead of starting this machine's server (a configured machine's is yours to start).
        #[arg(long)]
        attach_only: bool,
    },
    /// (Run by a pane's shell before an agent starts) print the folder it should start in:
    /// a new worktree when it's started in a repo's main checkout, else nothing.
    #[command(hide = true)]
    AgentDir {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// (Run by the launchers on Windows) start an agent in the folder `agent-dir` names.
    #[command(hide = true)]
    AgentLaunch {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        cmd: Vec<String>,
    },
    /// Run as an MCP server (stdio) so agents can see and steer the others.
    /// `integrate mcp` registers it with Claude Code, Codex, Qwen Code and Gemini CLI.
    Mcp,
    /// Share your config between machines through a private GitHub repo:
    /// `sync setup [repo]`, `sync` (pull + push now), `sync off`.
    Sync {
        action: Option<String>,
        name: Option<String>,
    },
}

impl Cmd {
    /// The pane the command was given, which may name its machine.
    fn pane(&self) -> Option<&PaneAddress> {
        match self {
            Cmd::Read { pane, .. }
            | Cmd::Teach { pane }
            | Cmd::Close { pane }
            | Cmd::Send { pane, .. }
            | Cmd::Wait { pane, .. }
            | Cmd::Split { pane, .. } => pane.as_ref(),
            Cmd::Handoff { pane, .. } | Cmd::Focus { pane } | Cmd::Grant { pane, .. } => Some(pane),
            _ => None,
        }
    }

    /// Whether `--on` has a meaning for the command; one that ignored it would act on this
    /// computer while the person meant another.
    fn runs_on_a_machine(&self) -> bool {
        matches!(
            self,
            Cmd::Ls { .. }
                | Cmd::Read { .. }
                | Cmd::Send { .. }
                | Cmd::Wait { .. }
                | Cmd::Split { .. }
                | Cmd::Handoff { .. }
                | Cmd::New { .. }
                | Cmd::Focus { .. }
                | Cmd::Teach { .. }
                | Cmd::Popup { .. }
                | Cmd::Grant { .. }
                | Cmd::Close { .. }
                | Cmd::KillServer { .. }
        )
    }
}

#[derive(Subcommand)]
enum MachineCmd {
    /// Install this version on a machine, or replace an older one there. It looks first and
    /// installs nothing without a yes. The machine's running sessions are not stopped. Windows
    /// machines are not supported as a remote.
    Update {
        /// The machine's name in config.
        name: String,
        /// Install it where it is not installed yet.
        #[arg(long)]
        install: bool,
        /// Replace an older version without asking.
        #[arg(long)]
        yes: bool,
        /// Replace a newer version there with this one (a downgrade).
        #[arg(long)]
        force: bool,
    },
}

#[derive(Subcommand)]
enum ConfigCmd {
    /// Print the config file location.
    Path,
    /// Write the annotated example config (won't overwrite).
    Init,
    /// Print the annotated example config.
    Example,
}

#[derive(Subcommand)]
enum SecretCmd {
    /// Spend a one-use secret reference and set it in an environment variable for the command.
    Run {
        reference: String,
        #[arg(long = "env")]
        env_name: String,
        #[arg(last = true, required = true, allow_hyphen_values = true)]
        command: Vec<String>,
    },
}

#[derive(Subcommand)]
enum PrCmd {
    /// Follow an explicit pull request URL, or use the pane's branch when URL is omitted.
    Link {
        pane: Option<protocol::TermId>,
        url: Option<String>,
    },
}

/// A mistake in how the command was given: said plainly, exit code 2.
fn usage_error(message: impl std::fmt::Display) -> ! {
    eprintln!("{APP_COMMAND}: {message}");
    std::process::exit(2);
}

fn term_of(pane: Option<PaneAddress>) -> Option<protocol::TermId> {
    pane.map(|p| p.term)
}

/// The config that names machines. Where no machine is named it is never needed, so a broken
/// config does not stop a command that does not use it.
fn machines_config() -> config::Config {
    config::Config::load().unwrap_or_else(|e| usage_error(format!("config: {e:#}")))
}

/// The machine the command talks to, from `--on`, its pane's address, `--remote`, or the
/// default of this computer. A mistake in naming it is a usage error.
fn command_target(args: &Args) -> Target {
    let pane = args.cmd.as_ref().and_then(Cmd::pane);
    let lists_all = matches!(args.cmd, Some(Cmd::Ls { all: true, .. }));
    let names_a_machine = args.on.is_some() || pane.is_some_and(|p| p.machine.is_some());
    if args.on.is_some() && !args.cmd.as_ref().is_some_and(Cmd::runs_on_a_machine) {
        usage_error(
            "--on works with ls, read, send, wait, split, handoff, new, focus, close, grant, teach, popup and kill-server",
        );
    }
    if lists_all {
        if names_a_machine || args.remote.is_some() {
            usage_error(TargetError::AllNeedsNoTarget);
        }
        return Target::current();
    }
    let cfg = if names_a_machine {
        machines_config()
    } else {
        config::Config::default()
    };
    let choice = Choice {
        on: args.on.as_deref(),
        remote: args.remote.as_deref(),
        pane,
    };
    Target::resolve(&choice, &cfg, ipc::SSH_PROGRAM).unwrap_or_else(|e| usage_error(e))
}

/// `machine update`: a mistake in how it was asked is a usage error, a step that did not work
/// exits 1 with what became of the machine.
fn machine_update(name: &str, install: bool, yes: bool, force: bool) -> anyhow::Result<()> {
    use cli::machine_update::{Failure, Flags};
    let flags = Flags {
        install,
        yes,
        force,
    };
    match cli::machine_update::run(name, &machines_config(), flags) {
        Ok(()) => Ok(()),
        Err(Failure::Usage(message)) => usage_error(message),
        Err(Failure::Failed(message)) => Err(anyhow::anyhow!(message)),
    }
}

pub(crate) fn run() {
    let args = Args::parse();
    if let Err(error) = config::initialize_data_dir() {
        eprintln!(concat!(env!("APP_COMMAND"), ": {:#}"), error);
        std::process::exit(2);
    }
    if let Some(r) = &args.remote {
        // SAFETY: set once at startup, before any thread is started.
        unsafe { std::env::set_var(concat!(env!("APP_ENV"), "_REMOTE"), r) };
    }
    config::migrate_old_names();
    cli::update::tidy();
    // `<app> <dir>` opens a directory; a typo'd subcommand shouldn't silently attach.
    if let Some(p) = args.path.as_ref().or(match &args.cmd {
        Some(Cmd::Attach { path }) => path.as_ref(),
        _ => None,
    }) && !p.is_dir()
    {
        eprintln!(
            concat!(
                env!("APP_COMMAND"),
                ": `{}` is not a directory or a command (see `",
                env!("APP_COMMAND"),
                " --help`)"
            ),
            p.display()
        );
        std::process::exit(2);
    }
    let target = command_target(&args);
    let result = match args.cmd {
        None => open_client(args.path),
        Some(Cmd::Attach { path }) => open_client(path),
        Some(Cmd::Daemon) => maitty_core::run_daemon(|| {
            if !cfg!(test) {
                cli::refresh_claude_hooks();
            }
        }),
        Some(Cmd::Conversation { cmd }) => cli::conversation::run(cmd),
        Some(Cmd::Secret {
            cmd:
                SecretCmd::Run {
                    reference,
                    env_name,
                    command,
                },
        }) => match cli::secret_run::run(&reference, &env_name, &command) {
            Ok(0) => Ok(()),
            Ok(code) => std::process::exit(code),
            Err(error) => Err(error),
        },
        Some(Cmd::Pr {
            cmd: PrCmd::Link { pane, url },
        }) => cli::pr_link(pane, url),
        Some(Cmd::Ls { json, all: true }) => {
            cli::machines_ls::all(&machines_config(), ipc::SSH_PROGRAM, json)
        }
        Some(Cmd::Ls { json, .. }) => cli::ls(&target, json),
        Some(Cmd::Read { pane, raw }) => cli::read(&target, term_of(pane), raw),
        Some(Cmd::Send { pane, keys, .. }) if !keys.is_empty() => {
            cli::send_keys(&target, term_of(pane), keys)
        }
        Some(Cmd::Send {
            pane,
            no_enter,
            text,
            wait: true,
            timeout,
            ..
        }) => cli::send_wait(&target, term_of(pane), text.join(" "), !no_enter, timeout),
        Some(Cmd::Send {
            pane,
            no_enter,
            text,
            ..
        }) => cli::send(&target, term_of(pane), text.join(" "), !no_enter),
        Some(Cmd::Wait {
            pane,
            regex,
            timeout,
        }) => cli::wait(&target, term_of(pane), regex, timeout),
        Some(Cmd::Split {
            pane,
            down,
            command,
        }) => cli::split(&target, term_of(pane), down, command),
        Some(Cmd::Handoff { pane, to }) => cli::handoff(&target, pane.term, to),
        Some(Cmd::New {
            path,
            name,
            command,
        }) => cli::new_workspace(&target, path, name, command),
        Some(Cmd::Focus { pane }) => cli::focus(&target, pane.term),
        Some(Cmd::Reveal { target }) => cli::reveal::run(&target),
        Some(Cmd::Teach { pane }) => cli::teach(&target, term_of(pane)),
        Some(Cmd::Statusline) => {
            cli::statusline();
            Ok(())
        }
        Some(Cmd::AgentDir { cmd }) => {
            cli::agent_dir(&cmd);
            Ok(())
        }
        Some(Cmd::AgentLaunch { cmd }) => std::process::exit(cli::agent_launch(&cmd)),
        Some(Cmd::AskHuman { question, options }) => cli::ask_human(question, options),
        Some(Cmd::Grant { pane, grants }) => cli::grant(&target, pane.term, &grants),
        Some(Cmd::Popup { cmd }) => cli::popup(&target, cmd),
        Some(Cmd::Close { pane }) => cli::close(&target, term_of(pane)),
        Some(Cmd::Hook {
            agent,
            status,
            payload,
        }) => {
            // Hooks run inline in the agent's turn: never fail, never print.
            let _ = cli::hook(&agent, status.as_deref(), payload.as_deref());
            Ok(())
        }
        Some(Cmd::Integrate { agent, uninstall }) => cli::integrate(agent.as_deref(), uninstall),
        Some(Cmd::Config { cmd }) => match cmd {
            ConfigCmd::Path => {
                println!("{}", config::config_path().display());
                Ok(())
            }
            ConfigCmd::Init => cli::config_init(),
            ConfigCmd::Example => {
                print!("{}", config::EXAMPLE);
                Ok(())
            }
        },
        Some(Cmd::Worktree {
            branch,
            move_here: true,
            ..
        }) => cli::move_to_worktree(branch),
        Some(Cmd::Worktree {
            branch,
            base,
            ws,
            command,
            ..
        }) => cli::worktree(branch, base, ws, command),
        Some(Cmd::WorktreeRemove { ws, force }) => cli::worktree_remove(ws, force),
        Some(Cmd::KillServer { forget, yes }) => {
            if target.name().is_some() && target.is_remote() && !yes {
                usage_error(TargetError::NeedsYes {
                    machine: target.machine().name().into(),
                });
            }
            cli::kill_server(&target, forget)
        }
        Some(Cmd::Mcp) => cli::mcp::run(),
        Some(Cmd::Doctor) => cli::doctor(),
        Some(Cmd::Account { name, command }) => cli::account(&name, &command),
        Some(Cmd::Update { check, force }) => cli::update::run(check, force),
        Some(Cmd::Machine {
            cmd:
                MachineCmd::Update {
                    name,
                    install,
                    yes,
                    force,
                },
        }) => machine_update(&name, install, yes, force),
        Some(Cmd::RestartIfIdle) => cli::restart_if_idle(),
        Some(Cmd::Allow { dir }) => cli::allow(dir),
        Some(Cmd::Proxy { attach_only }) => match cli::block_on(ipc::proxy(attach_only)) {
            Err(e) if ipc::is_no_daemon_here(&e) => {
                eprintln!("{APP_COMMAND}: {e:#}");
                std::process::exit(ipc::NO_DAEMON_EXIT);
            }
            done => done,
        },
        Some(Cmd::DebugColors { pane }) => cli::debug_colors(pane),
        Some(Cmd::Sync { action, name }) => sync::command(action.as_deref(), name.as_deref()),
        Some(Cmd::TestAlert) => {
            let (cfg, _) = config::Config::load_or_default();
            println!(concat!(
                "notification: the one you get when no ",
                env!("APP_COMMAND"),
                " window is open"
            ));
            // With a server running, clicking it goes to the session you're on.
            let link = cli::focused_pane().map(reveal::link);
            if link.is_some() {
                println!("click the notification to go to the session you're on");
            }
            alert::notify(
                "claude needs you",
                concat!(env!("APP_COMMAND"), " · this is a test"),
                link.as_deref(),
            );
            match alert::phone_url(&cfg.notify.phone_server, &cfg.notify.phone_topic) {
                None => println!("phone: off (notify.phone_topic)"),
                Some(url) => match alert::phone(
                    &cfg.notify,
                    "claude needs you",
                    concat!(env!("APP_COMMAND"), " · this is a test"),
                ) {
                    Ok(()) => println!("phone: sent to {url}"),
                    Err(e) => println!("phone: didn't go to {url}: {e}"),
                },
            }
            if !cfg.notify.sound {
                println!("sounds: off (notify.sound); playing them anyway");
            }
            for (what, kind) in [
                ("needs you", alert::Kind::Needs),
                ("finished", alert::Kind::Done),
            ] {
                let volume = alert::Volume::of(&cfg.notify);
                match alert::tone_file(kind, volume) {
                    Some(f) => {
                        println!("{what} sound: {}", f.display());
                        alert::play(kind, volume);
                    }
                    None => {
                        println!("{what} sound: couldn't be written to {APP_COMMAND}'s data folder")
                    }
                }
            }
            Ok(())
        }
    };
    if let Err(e) = result {
        eprintln!("{APP_COMMAND}: {e:#}");
        std::process::exit(1);
    }
}

/// Open the window. A server of another version is replaced on the way in: without asking
/// when nothing is mid-turn (it says `Updating <app>…`), after one Enter when something is.
/// When this window was told to give way to a newly installed the app, that one opens here.
fn open_client(open: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    run_client(open)?;
    if cli::restart::relaunch_requested() {
        // The old server is stopping; the installed the app starts the new one.
        let _ = cli::restart::wait_until_server_gone();
        eprintln!(concat!("Restarting ", env!("APP_COMMAND"), "…"));
        cli::restart::relaunch()?;
    }
    Ok(())
}

fn run_client(open: Option<std::path::PathBuf>) -> anyhow::Result<()> {
    use std::io::IsTerminal;
    let terminal = std::io::stdin().is_terminal() && std::io::stdout().is_terminal();
    let local = !ipc::Machine::current().is_remote();
    // An older server of a protocol this the app speaks is replaced at once when it is idle.
    if terminal
        && local
        && restart::replace_older_idle_server(restart::read().as_ref(), env!("APP_VERSION"))
    {
        replace_server()?;
    }
    let err = match client::run(client::Options { open: open.clone() }) {
        Err(e) => e,
        done => return done,
    };
    let Some(daemon) = client::restart_prompt::wanted(&err, terminal, !local) else {
        return Err(err);
    };
    match restart::plan_for_mismatch(daemon, restart::read().as_ref(), restart::saved_agents()) {
        restart::Plan::Replace => replace_server()?,
        restart::Plan::Ask { resume, busy } => {
            match client::restart_prompt::offer(daemon, resume, busy)? {
                client::restart_prompt::Choice::Quit => return Err(err),
                client::restart_prompt::Choice::Restart => {
                    cli::kill_server(&Target::current(), false)?
                }
            }
        }
    }
    client::run(client::Options { open })
}

/// Stop this socket's server (saving its sessions first when it can be asked; one it can't
/// talk to is stopped by its process, as `<app> kill-server` does) so the next start is
/// this version, with the sessions restored.
fn replace_server() -> anyhow::Result<()> {
    eprintln!(concat!("Updating ", env!("APP_COMMAND"), "…"));
    cli::kill_server(&Target::current(), false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn proxy_takes_the_flag_the_ssh_command_passes() {
        let args = Args::try_parse_from([APP_COMMAND, "proxy", ipc::PROXY_ATTACH_ONLY_FLAG])
            .expect("the proxy accepts the flag the client sends");
        assert!(matches!(args.cmd, Some(Cmd::Proxy { attach_only: true })));
        let plain = Args::try_parse_from([APP_COMMAND, "proxy"]).unwrap();
        assert!(matches!(plain.cmd, Some(Cmd::Proxy { attach_only: false })));
    }

    fn parse(args: &[&str]) -> Args {
        Args::try_parse_from([APP_COMMAND].into_iter().chain(args.iter().copied()))
            .expect("the command line parses")
    }

    fn address_of(args: &Args) -> (Option<String>, protocol::TermId) {
        let pane = args.cmd.as_ref().and_then(Cmd::pane).expect("a pane");
        (pane.machine.clone(), pane.term)
    }

    #[test]
    fn on_comes_before_or_after_the_subcommand() {
        let before = parse(&["--on", "build", "send", "--pane", "build/4", "hi"]);
        let after = parse(&["send", "--on", "build", "--pane", "build/4", "hi"]);
        for args in [before, after] {
            assert_eq!(args.on.as_deref(), Some("build"));
            assert_eq!(address_of(&args), (Some("build".into()), 4));
        }
    }

    #[test]
    fn a_positional_pane_takes_an_address_or_a_bare_number() {
        assert_eq!(
            address_of(&parse(&["read", "build/4"])),
            (Some("build".into()), 4)
        );
        assert_eq!(address_of(&parse(&["read", "4"])), (None, 4));
        assert_eq!(
            address_of(&parse(&["grant", "gpu/2", "read"])),
            (Some("gpu".into()), 2)
        );
    }

    #[test]
    fn a_bad_address_is_refused_by_the_parser() {
        for bad in ["build/", "/4", "a/b/4", "x"] {
            assert!(
                Args::try_parse_from([APP_COMMAND, "focus", bad]).is_err(),
                "{bad}"
            );
        }
    }

    #[test]
    fn only_commands_that_take_a_machine_accept_on() {
        assert!(parse(&["ls"]).cmd.unwrap().runs_on_a_machine());
        assert!(parse(&["kill-server"]).cmd.unwrap().runs_on_a_machine());
        assert!(!parse(&["worktree", "x"]).cmd.unwrap().runs_on_a_machine());
        assert!(!parse(&["doctor"]).cmd.unwrap().runs_on_a_machine());
    }
}
