//! `<app> machine update <name>`: install or update the app on a configured machine over ssh
//! (ADR 0047, D8). It looks first (the machine's platform, the version there), says what it
//! found, and installs nothing without a yes. The binary is this computer's own when the
//! platform matches, else the release fetched here and checked; it is streamed through ssh to
//! a fixed script that swaps it in only after it has arrived whole and run. The far side's
//! daemon is never signalled: it keeps its sessions and is asked to restart only if none is open.

pub(crate) mod binary;

use crate::brand::{APP_COMMAND, APP_VERSION};
use crate::cli::target::resolve_machine;
use crate::config::Config;
use crate::ipc::{Machine, SSH_PROGRAM};
use crate::remote_install::{
    Ask, Consent, Decision, Installed, Platform, State, UploadError, classify_version, decide,
    destination, names_destination, parse_platform, state_of, upload_result,
};
use binary::{Github, ReleaseSources, Sources};
use maitty_machines::ssh_run::{self, ASK_LIMIT, Ran, UPLOAD_LIMIT};
use maitty_shared::hidden::sanitize;

/// What the person asked with.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct Flags {
    pub install: bool,
    pub yes: bool,
    pub force: bool,
}

/// A mistake in how it was asked (exit 2), or a step that did not work (exit 1).
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Failure {
    Usage(String),
    Failed(String),
}

fn failed(message: impl Into<String>) -> Failure {
    Failure::Failed(message.into())
}

/// How it ended well: the last line, for stdout.
#[derive(Debug, PartialEq, Eq)]
pub(crate) enum Outcome {
    UpToDate { version: String },
    Installed { destination: String },
}

impl Outcome {
    pub(crate) fn line(&self, machine: &str) -> String {
        match self {
            Outcome::UpToDate { version } => {
                format!("{machine}: {APP_COMMAND} {version} is already up to date")
            }
            Outcome::Installed { destination } => {
                format!("{machine}: {APP_COMMAND} {APP_VERSION} installed to {destination}")
            }
        }
    }
}

/// Where progress goes and who answers a question: the terminal, or a test.
pub(crate) trait Talk {
    fn say(&mut self, line: &str);
    /// Whether a person is there to answer `confirm`.
    fn can_ask(&self) -> bool;
    fn confirm(&mut self, question: &str) -> bool;
}

/// `<app> machine update <name>`: the entry point from the command line. Exits through the
/// returned failure; the caller prints it.
pub(crate) fn run(name: &str, cfg: &Config, flags: Flags) -> Result<(), Failure> {
    let machine =
        resolve_machine(name, cfg, SSH_PROGRAM).map_err(|e| Failure::Usage(e.to_string()))?;
    if !machine.is_remote() {
        return Err(Failure::Usage(
            "this computer is not reached over ssh; name a machine from [[machines]] in config.local.toml".into(),
        ));
    }
    let github = Github;
    let sources = ReleaseSources::this_computer(&github);
    let outcome = update(&machine, flags, &sources, &mut Terminal::new())?;
    println!("{}", outcome.line(machine.name()));
    Ok(())
}

/// Look at the machine, decide, send, swap, and tell the daemon.
pub(crate) fn update(
    machine: &Machine,
    flags: Flags,
    sources: &dyn Sources,
    talk: &mut dyn Talk,
) -> Result<Outcome, Failure> {
    let name = machine.name().to_string();
    if machine.transport_program().is_some() {
        return Err(Failure::Usage(format!(
            "installing over a custom transport is not supported; install {APP_COMMAND} on {name} yourself"
        )));
    }
    let remote_cmd = machine.remote_cmd().unwrap_or(APP_COMMAND).to_string();
    talk.say(&format!("probing {name}…"));
    let platform = probe_platform(machine, &name)?;
    let installed = probe_installed(machine, &name)?;
    talk.say(&found_line(&name, &platform, &installed));
    let state = state_of(&installed, APP_VERSION).ok_or_else(|| {
        let Installed::Broken(why) = &installed else {
            return failed("the version there could not be compared");
        };
        failed(format!(
            "the {APP_COMMAND} on {name} is not usable: {why}; fix or remove it, then retry"
        ))
    })?;
    let consent = Consent {
        install: flags.install,
        yes: flags.yes,
        force: flags.force,
        can_ask: talk.can_ask(),
    };
    match decide(&state, consent) {
        Decision::UpToDate => {
            let version = installed_version(&installed);
            return Ok(Outcome::UpToDate { version });
        }
        Decision::Refuse(why) => return Err(Failure::Usage(why.message(&name))),
        Decision::Ask(what) => {
            if !talk.confirm(&question(what, &name, &remote_cmd)) {
                return Err(failed(format!(
                    "not confirmed; nothing was changed on {name}"
                )));
            }
        }
        Decision::Go => {}
    }
    let had_old = state != State::Missing;
    let binary = sources
        .binary(&platform, &mut |line| talk.say(line))
        .map_err(|why| failed(format!("{why}: nothing was sent to {name}")))?;
    let to = destination(&remote_cmd);
    talk.say(&format!(
        "sending {:.1} MB (sha256 {})…",
        binary.len as f64 / (1024.0 * 1024.0),
        &binary.sha256[..12]
    ));
    send(machine, &name, &to, &binary, had_old)?;
    talk.say("verified: it arrived whole and runs there");
    talk.say(&format!("installed to {to}"));
    check_reachable(machine, &remote_cmd, &to, &name, talk);
    if had_old {
        restart_if_idle(machine, &to, &name, talk);
    }
    Ok(Outcome::Installed { destination: to })
}

fn question(what: Ask, name: &str, remote_cmd: &str) -> String {
    let to = destination(remote_cmd);
    match what {
        Ask::Install => {
            format!("{APP_COMMAND} is not installed on {name}. Install {APP_VERSION} to {to}?")
        }
        Ask::Update => format!("Replace the {APP_COMMAND} on {name} with {APP_VERSION}?"),
    }
}

fn found_line(name: &str, platform: &Platform, installed: &Installed) -> String {
    let there = match installed {
        Installed::Missing => format!("{APP_COMMAND} is not installed"),
        Installed::Version(v) => format!("{APP_COMMAND} {v}"),
        Installed::Broken(_) => format!("{APP_COMMAND} does not answer"),
    };
    format!("{name}: {platform}, {there} (this is {APP_VERSION})")
}

fn installed_version(installed: &Installed) -> String {
    match installed {
        Installed::Version(v) => v.clone(),
        _ => APP_VERSION.to_string(),
    }
}

fn probe_platform(machine: &Machine, name: &str) -> Result<Platform, Failure> {
    let argv = machine
        .platform_argv()
        .map_err(|e| failed(format!("{e:#}")))?;
    let ran = ssh_run::run(&argv, None, ASK_LIMIT).map_err(Failure::Failed)?;
    reached(&ran, name)?;
    parse_platform(&ran.stdout).map_err(|e| failed(e.to_string()))
}

fn probe_installed(machine: &Machine, name: &str) -> Result<Installed, Failure> {
    let argv = machine.probe_argv().map_err(|e| failed(format!("{e:#}")))?;
    let ran = ssh_run::run(&argv, None, ASK_LIMIT).map_err(Failure::Failed)?;
    reached(&ran, name)?;
    Ok(classify_version(ran.code, &ran.stdout))
}

/// ssh's own failure (status 255, or killed by the limit) is the machine not being reached.
fn reached(ran: &Ran, name: &str) -> Result<(), Failure> {
    match ran.code {
        Some(255) | None => Err(failed(format!(
            "could not reach {name}: {}; make plain `ssh` to that machine work in a terminal, then retry",
            last_line(&ran.stderr)
        ))),
        Some(_) => Ok(()),
    }
}

fn last_line(said: &str) -> String {
    said.lines()
        .rev()
        .map(|line| sanitize(line, false))
        .map(|line| line.trim().chars().take(200).collect::<String>())
        .find(|line| !line.is_empty())
        .unwrap_or_else(|| "no answer".into())
}

fn send(
    machine: &Machine,
    name: &str,
    to: &str,
    binary: &binary::Binary,
    had_old: bool,
) -> Result<(), Failure> {
    let argv = machine
        .install_argv(to, binary.len)
        .map_err(|e| failed(format!("{e:#}")))?;
    let ran = ssh_run::run(&argv, Some(&binary.path), UPLOAD_LIMIT).map_err(Failure::Failed)?;
    upload_result(ran.code, &ran.stdout)
        .map_err(|why| failed(upload_failure(&why, name, had_old, &ran)))
}

fn upload_failure(why: &UploadError, name: &str, had_old: bool, ran: &Ran) -> String {
    let line = why.line(name, had_old);
    match why {
        UploadError::Ssh | UploadError::Other(_) => {
            format!("{line} (ssh said: {})", last_line(&ran.stderr))
        }
        _ => line,
    }
}

/// After the swap: does the machine find the app by its configured command? A default
/// install lands in a folder that may not be on its PATH; that is told, never fixed silently.
fn check_reachable(machine: &Machine, remote_cmd: &str, to: &str, name: &str, talk: &mut dyn Talk) {
    let Ok(argv) = machine.probe_argv() else {
        return;
    };
    let Ok(ran) = ssh_run::run(&argv, None, ASK_LIMIT) else {
        return;
    };
    match classify_version(ran.code, &ran.stdout) {
        Installed::Version(v) if v == APP_VERSION => {}
        Installed::Missing if !names_destination(remote_cmd) => talk.say(&format!(
            "note: `{remote_cmd}` is not on {name}'s PATH; set remote_cmd = \"{to}\" for that machine in config.local.toml"
        )),
        Installed::Version(v) => talk.say(&format!(
            "note: `{remote_cmd}` on {name} still runs {APP_COMMAND} {v}, not the file just installed at {to}; set remote_cmd = \"{to}\" for that machine in config.local.toml"
        )),
        _ => {}
    }
}

/// The old daemon keeps running; the new binary restarts it only when no session is open.
/// A failure to ask is a note, not a failure: the update itself is done.
fn restart_if_idle(machine: &Machine, to: &str, name: &str, talk: &mut dyn Talk) {
    talk.say(&format!(
        "asking the daemon on {name} to restart if no session is open…"
    ));
    let asked = machine
        .restart_if_idle_argv(to)
        .map_err(|e| format!("{e:#}"))
        .and_then(|argv| ssh_run::run(&argv, None, ASK_LIMIT));
    match asked {
        Ok(ran) if ran.code == Some(0) => {
            talk.say(&format!("daemon: {}", last_line(&ran.stdout)));
        }
        Ok(ran) => talk.say(&format!(
            "note: could not ask the daemon ({}); running sessions are untouched, and it takes the new version when it next restarts",
            last_line(&ran.stderr)
        )),
        Err(why) => talk.say(&format!("note: could not ask the daemon ({why}); running sessions are untouched")),
    }
}

/// The real terminal: progress on stderr, a question answered on stdin when there is a person.
struct Terminal {
    interactive: bool,
}

impl Terminal {
    fn new() -> Terminal {
        use std::io::IsTerminal;
        Terminal {
            interactive: std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
        }
    }
}

impl Talk for Terminal {
    fn say(&mut self, line: &str) {
        eprintln!("{line}");
    }

    fn can_ask(&self) -> bool {
        self.interactive
    }

    fn confirm(&mut self, question: &str) -> bool {
        eprint!("{question} [y/N] ");
        let mut answer = String::new();
        std::io::stdin().read_line(&mut answer).is_ok() && is_yes(&answer)
    }
}

fn is_yes(answer: &str) -> bool {
    matches!(answer.trim().to_lowercase().as_str(), "y" | "yes")
}
