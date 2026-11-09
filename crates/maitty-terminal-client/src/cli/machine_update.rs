//! `<app> machine update <name>`: install or update the app on a configured machine over ssh
//! (ADR 0007, D8). It looks first (the machine's platform, the version there), says what it
//! found, and installs nothing without a yes. The binary is this computer's own when the
//! platform matches, else the release fetched here and checked; it is streamed through ssh to
//! a fixed script that swaps it in only after it has arrived whole and run. The far side's
//! install script never signals a process. Afterwards the far daemon is asked to restart: only
//! if no session is open, or with `--restart` also when its sessions are saved, because the new
//! daemon restores them and resumes their agents (a turn in progress is lost).

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
    /// Restart the far daemon even with sessions open (they are saved and come back).
    pub restart: bool,
    /// Keep asking the far daemon to restart until nothing is open, up to half an hour.
    pub wait: bool,
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
    /// How far along the update is, 0 to 100, at each stage it finishes. Nothing by default: only the desktop app asks for it.
    fn progress(&mut self, _percent: u8) {}
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

/// `--hold`: say how it ended and wait for Enter. The window's popup closes the moment the
/// command ends, which would take the outcome off the screen before it could be read.
pub(crate) fn hold(result: &Result<(), Failure>) {
    if let Err(Failure::Usage(why) | Failure::Failed(why)) = result {
        eprintln!("{APP_COMMAND}: {why}");
    }
    eprint!("Press Enter to close.");
    let mut answer = String::new();
    if let Err(e) = std::io::stdin().read_line(&mut answer) {
        eprintln!("\ncould not read the keyboard ({e}); closing");
    }
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
    talk.progress(stage::PROBING);
    let platform = probe_platform(machine, &name)?;
    let installed = probe_installed(machine, &name)?;
    talk.say(&found_line(&name, &platform, &installed));
    talk.progress(stage::FOUND);
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
            // The file being current says nothing about the daemon: an earlier update may have
            // swapped it and then left the old daemon running because sessions were open.
            ask_daemon_to_restart(machine, &remote_cmd, &name, flags, talk);
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
    talk.progress(stage::FETCHED);
    let to = destination(&remote_cmd);
    talk.say(&format!(
        "sending {:.1} MB (sha256 {})…",
        binary.len as f64 / (1024.0 * 1024.0),
        &binary.sha256[..12]
    ));
    send(machine, &name, &to, &binary, had_old)?;
    talk.say("verified: it arrived whole and runs there");
    talk.say(&format!("installed to {to}"));
    talk.progress(stage::SENT);
    check_reachable(machine, &remote_cmd, &to, &name, talk);
    talk.progress(stage::REACHABLE);
    if had_old {
        ask_daemon_to_restart(machine, &to, &name, flags, talk);
    }
    Ok(Outcome::Installed { destination: to })
}

/// Ask the far daemon to take the version installed at `program`. The far side leaves a daemon
/// that already runs it alone.
fn ask_daemon_to_restart(
    machine: &Machine,
    program: &str,
    name: &str,
    flags: Flags,
    talk: &mut dyn Talk,
) {
    if flags.wait {
        restart_when_idle(machine, program, name, IDLE_POLL, talk);
    } else {
        restart_daemon(machine, program, name, flags.restart, talk);
    }
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

/// How long `--wait` pauses between asking, and how many times it asks (about half an hour).
const IDLE_POLL: std::time::Duration = if cfg!(test) {
    std::time::Duration::from_millis(5)
} else {
    std::time::Duration::from_secs(30)
};
const MOST_IDLE_ASKS: u32 = 60;

/// Ask the far daemon to restart if idle, again and again until it does. The new file is
/// already installed, so each ask runs the new binary, which can read the daemon's activity.
/// Giving up is a note, not a failure: the update is done and the daemon was left running.
fn restart_when_idle(
    machine: &Machine,
    to: &str,
    name: &str,
    pause: std::time::Duration,
    talk: &mut dyn Talk,
) {
    talk.say(&format!(
        "waiting for {name} to be idle, then restarting its daemon…"
    ));
    for ask in 0..MOST_IDLE_ASKS {
        let asked = machine
            .restart_if_idle_argv(to, false)
            .map_err(|e| format!("{e:#}"))
            .and_then(|argv| ssh_run::run(&argv, None, ASK_LIMIT));
        match asked {
            Ok(ran) if ran.code == Some(0) => {
                let line = last_line(&ran.stdout);
                if ["restarted", "no server", "current"].iter().any(|done| line.starts_with(done)) {
                    talk.say(&format!("daemon: {line}"));
                    return;
                }
            }
            Ok(ran) => {
                talk.say(&format!(
                    "note: could not ask the daemon ({}); running sessions are untouched, and it takes the new version when it next restarts",
                    last_line(&ran.stderr)
                ));
                return;
            }
            Err(why) => {
                talk.say(&format!("note: could not ask the daemon ({why}); running sessions are untouched"));
                return;
            }
        }
        if ask + 1 < MOST_IDLE_ASKS {
            std::thread::sleep(pause);
        }
    }
    talk.say(&format!(
        "note: {name} was still busy after half an hour; its daemon was left running and takes the new version when it next restarts"
    ));
}

/// The old daemon keeps running; the new binary restarts it only when no session is open.
/// A failure to ask is a note, not a failure: the update itself is done.
fn restart_daemon(machine: &Machine, to: &str, name: &str, now: bool, talk: &mut dyn Talk) {
    talk.say(&if now {
        format!("restarting the daemon on {name}; its saved sessions come back…")
    } else {
        format!("asking the daemon on {name} to restart if no session is open…")
    });
    let asked = machine
        .restart_if_idle_argv(to, now)
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

/// `<app> machine activity <name>`: how many sessions are open on the machine and how many are
/// mid-turn, as the one line the far side prints. A machine whose app is too old to answer, or
/// that has none, gives `activity: unknown`; only a machine that cannot be reached is a failure.
pub(crate) fn activity(name: &str, cfg: &Config) -> Result<String, Failure> {
    let machine =
        resolve_machine(name, cfg, SSH_PROGRAM).map_err(|e| Failure::Usage(e.to_string()))?;
    if !machine.is_remote() || machine.transport_program().is_some() {
        return Err(Failure::Usage(
            "name a machine from [[machines]] in config.local.toml that is reached over ssh".into(),
        ));
    }
    let remote_cmd = machine.remote_cmd().unwrap_or(APP_COMMAND).to_string();
    let argv = machine
        .activity_argv(&destination(&remote_cmd))
        .map_err(|e| failed(format!("{e:#}")))?;
    let ran = ssh_run::run(&argv, None, ASK_LIMIT).map_err(Failure::Failed)?;
    reached(&ran, name)?;
    Ok(activity_line(ran.code, &ran.stdout))
}

/// The far side's answer when it ran and printed one; anything else (an older app that does not
/// know the word, a missing app) is "unknown", never a guess.
fn activity_line(code: Option<i32>, stdout: &str) -> String {
    let line = last_line(stdout);
    if code == Some(0) && line.starts_with("activity: ") {
        return sanitize(&line, false);
    }
    "activity: unknown".into()
}

/// Where the update stands as each stage ends, for the desktop app's progress line. The numbers are the stages
/// themselves, not a measurement: the download and the upload have no byte count to report.
mod stage {
    pub const PROBING: u8 = 5;
    pub const FOUND: u8 = 15;
    pub const FETCHED: u8 = 55;
    pub const SENT: u8 = 85;
    pub const REACHABLE: u8 = 95;
}

/// The line the desktop app reads, on stderr beside the human lines; printed only when it asks for it.
fn progress_line(percent: u8) -> String {
    format!("progress: {percent}")
}

/// The real terminal: progress on stderr, a question answered on stdin when there is a person.
struct Terminal {
    interactive: bool,
    /// The desktop app asked for `progress:` lines (it sets `<APP>_UPDATE_PROGRESS=1`).
    machine_readable: bool,
}

impl Terminal {
    fn new() -> Terminal {
        use std::io::IsTerminal;
        Terminal {
            interactive: std::io::stdin().is_terminal() && std::io::stderr().is_terminal(),
            machine_readable: std::env::var(concat!(env!("APP_ENV"), "_UPDATE_PROGRESS")).as_deref() == Ok("1"),
        }
    }
}

impl Talk for Terminal {
    fn say(&mut self, line: &str) {
        eprintln!("{line}");
    }

    fn progress(&mut self, percent: u8) {
        if self.machine_readable {
            eprintln!("{}", progress_line(percent));
        }
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


#[cfg(test)]
mod activity_tests {
    use super::activity_line;

    #[test]
    fn the_far_sides_line_is_passed_on_and_anything_else_is_unknown() {
        assert_eq!(
            activity_line(Some(0), "login banner\nactivity: sessions=3 busy=1\n"),
            "activity: sessions=3 busy=1"
        );
        assert_eq!(activity_line(Some(2), "error: unrecognized subcommand"), "activity: unknown");
        assert_eq!(activity_line(Some(0), "something else"), "activity: unknown");
        assert_eq!(activity_line(None, ""), "activity: unknown");
    }
}

#[cfg(test)]
mod progress_tests {
    use super::{progress_line, stage};

    #[test]
    fn the_line_the_desktop_app_reads_is_one_number() {
        assert_eq!(progress_line(55), "progress: 55");
    }

    #[test]
    fn the_stages_only_go_forward_and_leave_room_for_the_restart() {
        let stages = [stage::PROBING, stage::FOUND, stage::FETCHED, stage::SENT, stage::REACHABLE];
        assert!(stages.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(stages.last().copied().unwrap_or(0) < 100);
    }
}
