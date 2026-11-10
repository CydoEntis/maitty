//! Which machine a one-shot command talks to, and which pane it means there.
//!
//! A pane is addressed `machine/id` (or a bare `id`), and `--on <machine>` names the machine for
//! a command that has no pane. `--remote host` stays the ad-hoc route. The pane a command runs
//! in (`$MAITTY_TERM_ID`) belongs to the daemon on this computer, so it is only a default when
//! the command talks to this computer.

use crate::client::remote_snapshot::sanitize_snapshot;
use crate::config::Config;
use crate::ipc::Machine;
use crate::link_failure::{self, FailureClass};
use crate::protocol::{Reply, TermId};
use maitty_settings::config::LOCAL_MACHINE;
use std::str::FromStr;

/// A pane on a machine: `4`, `build/4`, `local/4`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct PaneAddress {
    pub machine: Option<String>,
    pub term: TermId,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum BadAddress {
    NoMachine(String),
    NoPane(String),
    Nested(String),
    NotAPane(String),
    NumericMachine(String),
}

impl std::fmt::Display for BadAddress {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BadAddress::NoMachine(s) => write!(f, "`{s}` has no machine before the `/`"),
            BadAddress::NoPane(s) => write!(f, "`{s}` has no pane after the `/`"),
            BadAddress::Nested(s) => {
                write!(f, "`{s}` is not `machine/id`: only one `/` is allowed")
            }
            BadAddress::NotAPane(s) => write!(f, "`{s}` is not a pane number"),
            BadAddress::NumericMachine(s) => {
                write!(
                    f,
                    "`{s}`: a machine name is never only digits, so this is not `machine/id`"
                )
            }
        }
    }
}

impl std::error::Error for BadAddress {}

impl FromStr for PaneAddress {
    type Err = BadAddress;

    fn from_str(s: &str) -> Result<Self, BadAddress> {
        let term = |part: &str| {
            part.parse::<TermId>()
                .map_err(|_| BadAddress::NotAPane(s.to_string()))
        };
        let parts: Vec<&str> = s.split('/').collect();
        match parts.as_slice() {
            [id] => Ok(PaneAddress {
                machine: None,
                term: term(id)?,
            }),
            ["", _] => Err(BadAddress::NoMachine(s.into())),
            [_, ""] => Err(BadAddress::NoPane(s.into())),
            [machine, _] if machine.chars().all(|c| c.is_ascii_digit()) => {
                Err(BadAddress::NumericMachine(s.into()))
            }
            [machine, id] => Ok(PaneAddress {
                machine: Some((*machine).into()),
                term: term(id)?,
            }),
            _ => Err(BadAddress::Nested(s.into())),
        }
    }
}

/// A machine name that is neither `local` nor configured.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnknownMachine {
    pub given: String,
    pub known: Vec<String>,
}

impl std::fmt::Display for UnknownMachine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.known.len() <= 1 {
            write!(
                f,
                "unknown machine \"{}\"; no machines are configured; add one with the Machines dialog (leader w) or [[machines]] in config.local.toml",
                self.given
            )
        } else {
            write!(
                f,
                "unknown machine \"{}\"; known: {}",
                self.given,
                self.known.join(", ")
            )
        }
    }
}

impl std::error::Error for UnknownMachine {}

/// Why the flags and the address do not name one machine. All of these are usage errors.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetError {
    Unknown(UnknownMachine),
    OnAndRemote,
    AddressAndRemote,
    OnAndAddress { on: String, address: String },
    NeedsPane { machine: String },
    AllNeedsNoTarget,
    NeedsYes { machine: String },
}

impl std::fmt::Display for TargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetError::Unknown(e) => e.fmt(f),
            TargetError::OnAndRemote => f.write_str(
                "--on and --remote both name a machine; give one (--on is for configured machines)",
            ),
            TargetError::AddressAndRemote => {
                f.write_str("a machine/id address and --remote both name a machine; give one")
            }
            TargetError::OnAndAddress { on, address } => write!(
                f,
                "--on {on} and the pane address {address} name different machines"
            ),
            TargetError::NeedsPane { machine } => write!(
                f,
                "--on {machine} needs --pane {machine}/<id>; ${} is a pane on this computer",
                concat!(env!("APP_ENV"), "_TERM_ID")
            ),
            TargetError::AllNeedsNoTarget => f.write_str(
                "--all lists every machine; leave out --on, --remote and a machine address",
            ),
            TargetError::NeedsYes { machine } => write!(
                f,
                "this stops every session on {machine}; add --yes to do it"
            ),
        }
    }
}

impl std::error::Error for TargetError {}

/// `local`, then the configured machines in name order: what an address may name.
fn known_names(cfg: &Config) -> Vec<String> {
    let mut names: Vec<String> = cfg.machines.iter().map(|m| m.name.clone()).collect();
    names.sort_by_key(|n| n.to_lowercase());
    names.insert(0, LOCAL_MACHINE.into());
    names
}

/// The machine `name` stands for: this computer (`local`, any case), or a configured machine
/// (names compare ignoring case, as the config's own check does). `ssh` is the program it is
/// reached through.
pub(crate) fn resolve_machine(
    name: &str,
    cfg: &Config,
    ssh: &str,
) -> Result<Machine, UnknownMachine> {
    if name.eq_ignore_ascii_case(LOCAL_MACHINE) {
        return Ok(Machine::local());
    }
    cfg.machines
        .iter()
        .find(|def| def.name.eq_ignore_ascii_case(name))
        .map(|def| Machine::from_def(def, ssh))
        .ok_or_else(|| UnknownMachine {
            given: name.into(),
            known: known_names(cfg),
        })
}

/// The flags and address that pick a machine.
#[derive(Default)]
pub(crate) struct Choice<'a> {
    pub on: Option<&'a str>,
    pub remote: Option<&'a str>,
    pub pane: Option<&'a PaneAddress>,
}

/// The machine a command talks to and how it was chosen.
pub(crate) struct Target {
    machine: Machine,
    /// Named by `--on` or an address; false for this computer by default and for `--remote`.
    named: bool,
}

impl Target {
    /// The machine `--remote` (or `$MAITTY_REMOTE`) names, else this computer.
    pub(crate) fn current() -> Target {
        Target {
            machine: Machine::current(),
            named: false,
        }
    }

    pub(crate) fn of(machine: Machine) -> Target {
        Target {
            machine,
            named: true,
        }
    }

    /// The address wins, then `--on`, then the `--remote` adapter, then this computer.
    pub(crate) fn resolve(choice: &Choice, cfg: &Config, ssh: &str) -> Result<Target, TargetError> {
        let by_address = choice.pane.and_then(|p| p.machine.as_deref());
        let name = match (choice.on, by_address) {
            (Some(on), Some(address)) if !on.eq_ignore_ascii_case(address) => {
                return Err(TargetError::OnAndAddress {
                    on: on.into(),
                    address: address.into(),
                });
            }
            (Some(name), _) | (None, Some(name)) => name,
            (None, None) => return Ok(Target::current()),
        };
        if choice.remote.is_some() {
            return Err(if choice.on.is_some() {
                TargetError::OnAndRemote
            } else {
                TargetError::AddressAndRemote
            });
        }
        let machine = resolve_machine(name, cfg, ssh).map_err(TargetError::Unknown)?;
        Ok(Target::of(machine))
    }

    pub(crate) fn machine(&self) -> &Machine {
        &self.machine
    }

    pub(crate) fn is_remote(&self) -> bool {
        self.machine.is_remote()
    }

    /// The machine's name when it was asked for by name.
    pub(crate) fn name(&self) -> Option<&str> {
        self.named.then(|| self.machine.name())
    }

    /// Whether `$MAITTY_TERM_ID` names a pane on this machine.
    pub(crate) fn owns_env_pane(&self) -> bool {
        !self.named || !self.is_remote()
    }

    pub(crate) fn needs_pane(&self) -> TargetError {
        TargetError::NeedsPane {
            machine: self.machine.name().into(),
        }
    }

    /// How a pane is written back to the person: an address when the machine was named, so the
    /// line can be pasted into `--pane`.
    pub(crate) fn address(&self, term: TermId) -> String {
        match self.name() {
            Some(name) => format!("{name}/{term}"),
            None => term.to_string(),
        }
    }

    /// Text another machine sent, made safe to print: nothing in it can move the cursor or
    /// change the terminal. Newlines stay.
    pub(crate) fn plain(&self, text: &str) -> String {
        if self.is_remote() {
            crate::hidden::sanitize(text, true)
        } else {
            text.to_string()
        }
    }

    /// A failed connection: one line with its fix for a machine asked for by name, else the
    /// error with where it happened.
    pub(crate) fn connect_error(&self, error: anyhow::Error) -> anyhow::Error {
        match self.name().filter(|_| self.is_remote()) {
            Some(name) => {
                anyhow::anyhow!("{name}: {}", failure_line(&link_failure::classify(&error)))
            }
            None if self.is_remote() => error.context("over ssh"),
            None => error.context(concat!(
                "couldn't reach the ",
                env!("APP_COMMAND"),
                " server"
            )),
        }
    }

    /// A reply from this machine as far as it can be trusted: another machine's snapshot is
    /// cleaned, or refused when it cannot be.
    pub(crate) fn safe_reply(&self, reply: Reply) -> anyhow::Result<Reply> {
        match reply {
            Reply::List(snap) if self.is_remote() => {
                sanitize_snapshot(snap).map(Reply::List).map_err(|why| {
                    anyhow::anyhow!(
                        "{} sent a list that could not be made safe: {why}",
                        self.machine.name()
                    )
                })
            }
            other => Ok(other),
        }
    }
}

/// What went wrong and what to do about it, on one line.
pub(crate) fn failure_line(class: &FailureClass) -> String {
    match class {
        FailureClass::Login => format!("needs a login; {}", link_failure::FIX_LOGIN),
        FailureClass::Update => format!("is another version; {}", link_failure::FIX_UPDATE),
        FailureClass::UpdateHere => format!("is newer; {}", link_failure::FIX_UPDATE_HERE),
        FailureClass::Install => format!("has no app; {}", link_failure::FIX_INSTALL),
        FailureClass::NoDaemon => format!("is not running; {}", link_failure::FIX_NO_DAEMON),
        FailureClass::Unreachable(why) => format!("unreachable ({why})"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_settings::config::MachineDef;

    fn address(s: &str) -> Result<PaneAddress, BadAddress> {
        s.parse()
    }

    fn cfg(names: &[&str]) -> Config {
        Config {
            machines: names
                .iter()
                .map(|name| MachineDef {
                    name: (*name).into(),
                    target: format!("{name}.example"),
                    ..MachineDef::default()
                })
                .collect(),
            ..Config::default()
        }
    }

    #[test]
    fn a_bare_number_is_a_pane_on_the_default_machine() {
        assert_eq!(
            address("4"),
            Ok(PaneAddress {
                machine: None,
                term: 4
            })
        );
    }

    #[test]
    fn a_machine_and_a_number_make_an_address() {
        for (text, machine) in [
            ("build/4", "build"),
            ("local/4", "local"),
            ("Gpu-1.x/12", "Gpu-1.x"),
        ] {
            let got = address(text).unwrap();
            assert_eq!(got.machine.as_deref(), Some(machine), "{text}");
        }
    }

    #[test]
    fn an_address_that_is_not_machine_slash_number_is_refused() {
        assert!(matches!(address("build/"), Err(BadAddress::NoPane(_))));
        assert!(matches!(address("/4"), Err(BadAddress::NoMachine(_))));
        assert!(matches!(address("a/b/4"), Err(BadAddress::Nested(_))));
        assert!(matches!(address("x"), Err(BadAddress::NotAPane(_))));
        assert!(matches!(address("build/x"), Err(BadAddress::NotAPane(_))));
        assert!(matches!(address(""), Err(BadAddress::NotAPane(_))));
        assert!(matches!(address("-1"), Err(BadAddress::NotAPane(_))));
        assert!(matches!(address("4/5"), Err(BadAddress::NumericMachine(_))));
    }

    #[test]
    fn local_is_this_computer_in_any_case() {
        for name in ["local", "LOCAL", "Local"] {
            let machine = resolve_machine(name, &cfg(&["build"]), "ssh").unwrap();
            assert!(!machine.is_remote(), "{name}");
        }
    }

    #[test]
    fn a_configured_machine_is_found_ignoring_case_and_only_attaches() {
        let machine = resolve_machine("BUILD", &cfg(&["build"]), "ssh").unwrap();
        assert!(machine.is_remote());
        assert_eq!(
            machine,
            Machine::from_def(&cfg(&["build"]).machines[0], "ssh")
        );
    }

    #[test]
    fn an_unknown_machine_lists_the_names_that_work() {
        let error = resolve_machine("bild", &cfg(&["gpu", "build"]), "ssh").unwrap_err();
        assert_eq!(
            error.to_string(),
            "unknown machine \"bild\"; known: local, build, gpu"
        );
    }

    #[test]
    fn with_no_machines_configured_the_error_says_how_to_add_one() {
        let error = resolve_machine("bild", &cfg(&[]), "ssh").unwrap_err();
        assert!(
            error
                .to_string()
                .contains("no machines are configured; add one with the Machines dialog (leader w) or [[machines]] in config.local.toml"),
            "{error}"
        );
    }

    fn choose(
        on: Option<&str>,
        remote: Option<&str>,
        pane: Option<&str>,
    ) -> Result<Target, TargetError> {
        let pane = pane.map(|p| address(p).unwrap());
        Target::resolve(
            &Choice {
                on,
                remote,
                pane: pane.as_ref(),
            },
            &cfg(&["build", "gpu"]),
            "ssh",
        )
    }

    #[test]
    fn on_and_remote_together_are_refused() {
        assert_eq!(
            choose(Some("build"), Some("me@box"), None).err(),
            Some(TargetError::OnAndRemote)
        );
        assert_eq!(
            choose(None, Some("me@box"), Some("build/4")).err(),
            Some(TargetError::AddressAndRemote)
        );
    }

    #[test]
    fn on_and_an_address_for_another_machine_are_refused() {
        assert!(matches!(
            choose(Some("gpu"), None, Some("build/4")).err(),
            Some(TargetError::OnAndAddress { .. })
        ));
        let same = choose(Some("BUILD"), None, Some("build/4")).unwrap();
        assert_eq!(same.name(), Some("build"));
    }

    #[test]
    fn the_address_names_the_machine_when_on_is_absent() {
        let target = choose(None, None, Some("gpu/2")).unwrap();
        assert_eq!(target.name(), Some("gpu"));
        assert_eq!(target.address(2), "gpu/2");
    }

    #[test]
    fn an_unknown_machine_in_on_or_an_address_is_a_typed_error() {
        for target in [
            choose(Some("bild"), None, None),
            choose(None, None, Some("bild/1")),
        ] {
            assert!(matches!(target.err(), Some(TargetError::Unknown(_))));
        }
    }

    #[test]
    fn naming_local_still_means_this_computer_with_its_env_pane() {
        let target = choose(Some("local"), None, None).unwrap();
        assert!(!target.is_remote());
        assert!(target.owns_env_pane());
        assert_eq!(target.address(3), "local/3");
    }

    #[test]
    fn a_named_remote_machine_never_owns_the_env_pane() {
        assert!(!choose(Some("build"), None, None).unwrap().owns_env_pane());
    }

    #[test]
    fn remote_text_loses_its_escape_sequences_and_local_text_is_untouched() {
        let hostile = "ok\u{1b}]52;c;AAAA\u{7}\u{1b}[2J\nnext";
        let remote = Target::of(Machine::ssh("box", vec!["ssh".into()], "maitty"));
        assert_eq!(remote.plain(hostile), "ok]52;c;AAAA[2J\nnext");
        assert_eq!(Target::of(Machine::local()).plain(hostile), hostile);
    }

    #[test]
    fn another_machine_s_list_is_cleaned_and_this_computer_s_is_not() {
        use crate::protocol::{Snapshot, WorkspaceInfo};
        let hostile = "ws\u{1b}]52;c;QQ==\u{7}\u{202e}name";
        let list = || {
            Reply::List(Snapshot {
                workspaces: vec![WorkspaceInfo {
                    id: 1,
                    name: hostile.into(),
                    cwd: "/x".into(),
                    tabs: Vec::new(),
                    active_tab: 0,
                    git: None,
                    worktree: false,
                    color: 0,
                    is_new: false,
                    group: None,
                }],
                ..Snapshot::default()
            })
        };
        let named = |reply: Reply| match reply {
            Reply::List(snap) => snap.workspaces[0].name.clone(),
            _ => panic!("a list"),
        };
        let remote = Target::of(Machine::ssh("box", vec!["ssh".into()], "maitty"));
        assert_eq!(
            named(remote.safe_reply(list()).unwrap()),
            "ws]52;c;QQ==name"
        );
        let here = Target::of(Machine::local());
        assert_eq!(named(here.safe_reply(list()).unwrap()), hostile);
    }

    #[test]
    fn stopping_a_machine_s_sessions_asks_for_yes() {
        let said = TargetError::NeedsYes {
            machine: "build".into(),
        }
        .to_string();
        assert_eq!(
            said,
            "this stops every session on build; add --yes to do it"
        );
    }
}
