//! Adding a machine from the window: the hosts `~/.ssh/config` offers, and the look at a machine
//! before anything is written. The look logs in with what ssh already has and never asks anyone
//! anything (the shared `machine_probe`); a login that needs a person is the caller's to open.
//! Nothing here touches the config file or the hub: it decides, and the command acts.

use maitty_machines::MachineDef;
use maitty_machines::link_failure::FailureClass;
use maitty_machines::machine_config::{refusal, suggest_name};
use maitty_machines::machine_probe::Probe;
use maitty_machines::ssh_hosts::SshHost;
use maitty_machines::Machine;
use maitty_machines::machine_config::add_machine as write_entry;
use maitty_machines::machine_probe;
use maitty_machines::local_machines::{config_path, machines_beside};
use serde::Serialize;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::Duration;

/// How long ssh may take to answer before the look ends, whatever ssh does.
pub const PROBE_LIMIT: Duration = Duration::from_secs(12);

/// A name no machine has, for telling a bad target from a bad name.
const SPARE_NAME: &str = "machine-to-add";

/// A host of `~/.ssh/config` as the window is given it: the alias to connect with, and where it
/// points, for reading.
#[derive(Debug, Serialize, PartialEq, Eq)]
pub struct HostChoice {
    pub alias: String,
    pub detail: String,
}

/// The hosts to offer: not one that is already a machine's target.
pub fn host_choices(hosts: &[SshHost], existing: &[MachineDef]) -> Vec<HostChoice> {
    hosts
        .iter()
        .filter(|host| {
            !existing
                .iter()
                .any(|machine| machine.target.eq_ignore_ascii_case(&host.alias))
        })
        .map(|host| HostChoice {
            alias: host.alias.clone(),
            detail: match (&host.user, &host.host_name) {
                (Some(user), Some(name)) => format!("{user}@{name}"),
                (None, Some(name)) => name.clone(),
                (Some(user), None) => format!("{user}@{}", host.alias),
                (None, None) => String::new(),
            },
        })
        .collect()
}

/// How a machine that was looked at is to be brought up.
#[derive(Debug, PartialEq, Eq)]
pub enum How {
    /// Dial it now. `version` is what the app there said, when it answered.
    Connect { version: Option<String> },
    /// ssh needs a person: write the entry and open ssh where prompts can be answered.
    Login,
}

#[derive(Debug, PartialEq)]
pub enum Prepare {
    Ready { def: MachineDef, how: How },
    /// No usable name can be made from the target: ask for one.
    NeedsName,
    Cancelled,
}

/// Check `target` (and `name`, or one made from it) against the machines already there, then
/// look at the machine with `probe`. Writes nothing. An error is words for the person: a target
/// or name the loader refuses, or a machine ssh cannot reach (nothing is to be added then).
pub fn prepare(
    existing: &[MachineDef],
    target: &str,
    name: Option<&str>,
    probe: &dyn Fn(&MachineDef) -> Probe,
) -> Result<Prepare, String> {
    let target = target.trim();
    if let Some(why) = refusal(existing, SPARE_NAME, target) {
        return Err(why);
    }
    let name = match name.map(str::trim).filter(|name| !name.is_empty()) {
        Some(name) => name.to_owned(),
        None => {
            let taken: Vec<String> = existing.iter().map(|m| m.name.clone()).collect();
            match suggest_name(target, &taken) {
                Some(name) => name,
                None => return Ok(Prepare::NeedsName),
            }
        }
    };
    if let Some(why) = refusal(existing, &name, target) {
        return Err(why);
    }
    let def = MachineDef {
        name,
        target: target.to_owned(),
        ..MachineDef::default()
    };
    match probe(&def) {
        Probe::Cancelled => Ok(Prepare::Cancelled),
        Probe::Ready { version } => Ok(Prepare::Ready {
            def,
            how: How::Connect {
                version: Some(version).filter(|v| !v.is_empty()),
            },
        }),
        Probe::Failed(FailureClass::Login) => Ok(Prepare::Ready {
            def,
            how: How::Login,
        }),
        Probe::Failed(FailureClass::Unreachable(why)) => Err(format!("{}: {why}", def.target)),
        // Reached, but the app is missing, old or not running: the connection's own state says
        // which, with the fix, once it dials.
        Probe::Failed(_) => Ok(Prepare::Ready {
            def,
            how: How::Connect { version: None },
        }),
    }
}


/// The look in flight, so the window can end it.
#[derive(Default)]
pub struct AddProbe(Mutex<Option<Arc<AtomicBool>>>);

/// What adding a machine came to. A machine that could not be added is an error with the words
/// to show instead.
#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(tag = "outcome", rename_all = "kebab-case")]
pub enum Outcome {
    /// Written and dialling. `version` is what the app there answered, when it did.
    Added { name: String, version: Option<String> },
    /// Written, and ssh is open in a session on this computer where its prompts can be answered.
    Login { name: String },
    /// Written, but ssh could not be opened for the person: they run it once themselves.
    LoginManual { name: String, target: String, why: String },
    /// No name could be made from the target.
    NeedsName,
    Cancelled,
}

/// The hosts of `~/.ssh/config`, minus the ones that are already machines. Read off the UI thread.
#[tauri::command]
pub async fn list_ssh_hosts() -> Result<Vec<HostChoice>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let existing = machines_beside(&config_path()).machines;
        host_choices(&maitty_machines::ssh_hosts::user_hosts(), &existing)
    })
    .await
    .map_err(|error| format!("Could not read ~/.ssh/config: {error}"))
}

/// Look at `target`, and when it holds up write it to `config.local.toml` and start dialling it.
/// Nothing is written when the machine cannot be reached or the look is cancelled.
#[tauri::command]
pub async fn add_machine(
    app: tauri::AppHandle,
    wire: tauri::State<'_, crate::terminal_stream::Wire>,
    probing: tauri::State<'_, AddProbe>,
    target: String,
    name: Option<String>,
) -> Result<Outcome, String> {
    let handle = wire.handle()?.clone();
    let cancel = Arc::new(AtomicBool::new(false));
    *probing.0.lock().map_err(|_| "Adding is busy".to_string())? = Some(cancel.clone());
    let file = config_path().with_file_name("config.local.toml");
    let looked = {
        let cancel = cancel.clone();
        tauri::async_runtime::spawn_blocking(move || look_and_write(&file, &target, name.as_deref(), &cancel))
            .await
            .map_err(|error| format!("Could not look at the machine: {error}"))
    };
    if let Ok(mut slot) = probing.0.lock() {
        *slot = None;
    }
    let (def, how) = match looked?? {
        Prepare::Cancelled => return Ok(Outcome::Cancelled),
        Prepare::NeedsName => return Ok(Outcome::NeedsName),
        Prepare::Ready { def, how } => (def, how),
    };
    handle.add(def.clone());
    match how {
        How::Connect { version } => Ok(Outcome::Added { name: def.name, version }),
        How::Login => Ok(open_login(&app, &def).await),
    }
}

/// End the look that is running, if one is.
#[tauri::command]
pub fn cancel_add_machine(probing: tauri::State<'_, AddProbe>) {
    if let Ok(slot) = probing.0.lock()
        && let Some(cancel) = slot.as_ref()
    {
        cancel.store(true, Ordering::Relaxed);
    }
}

/// The blocking part: check, look, and only then write the entry.
fn look_and_write(
    file: &std::path::Path,
    target: &str,
    name: Option<&str>,
    cancel: &AtomicBool,
) -> Result<Prepare, String> {
    let listed = machines_beside(&config_path());
    if let Some(error) = listed.error {
        return Err(error);
    }
    let look = |def: &MachineDef| {
        let machine = Machine::from_def(def, maitty_machines::SSH_PROGRAM);
        machine_probe::probe(&machine, PROBE_LIMIT, cancel)
    };
    let prepared = prepare(&listed.machines, target, name, &look)?;
    if let Prepare::Ready { def, .. } = &prepared {
        write_entry(file, &def.name, &def.target)
            .map_err(|error| format!("Could not add {}: {error:#}", def.name))?;
    }
    Ok(prepared)
}

/// Open ssh in a session on this computer so a person can answer its prompts. The machine is
/// dialling already; it connects once the login has worked.
async fn open_login(app: &tauri::AppHandle, def: &MachineDef) -> Outcome {
    let manual = |why: String| Outcome::LoginManual {
        name: def.name.clone(),
        target: def.target.clone(),
        why,
    };
    let command = match Machine::from_def(def, maitty_machines::SSH_PROGRAM).first_login_command() {
        Ok(command) => command,
        Err(error) => return manual(format!("{error:#}")),
    };
    let executable = match crate::daemon_executable(app) {
        Ok(executable) => executable,
        Err(error) => return manual(error),
    };
    match maitty_desktop_client::start_command_session(command, format!("ssh {}", def.name), executable).await {
        Ok(_) => Outcome::Login { name: def.name.clone() },
        Err(error) => manual(format!("{error:#}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_machines::machine_probe::probe;
    use maitty_machines::Machine;
    use maitty_test_support::{Sandbox, fake_ssh};
    use std::sync::atomic::AtomicBool;

    /// The real look, through the fake ssh in `mode`.
    fn through_shim(sandbox: &Sandbox, mode: &str) -> impl Fn(&MachineDef) -> Probe {
        let shim = fake_ssh().display().to_string();
        let args = sandbox.shim_args(mode);
        move |def: &MachineDef| {
            let def = MachineDef {
                ssh_args: args.clone(),
                ..def.clone()
            };
            let ssh = [shim.clone()];
            probe(
                &Machine::from_def_via(&def, &ssh),
                Duration::from_secs(10),
                &AtomicBool::new(false),
            )
        }
    }

    fn def(name: &str, target: &str) -> MachineDef {
        MachineDef {
            name: name.into(),
            target: target.into(),
            ..MachineDef::default()
        }
    }

    fn look(mode: &str) -> Result<Prepare, String> {
        let sandbox = Sandbox::new();
        prepare(&[], "build.example", None, &through_shim(&sandbox, mode))
    }

    #[test]
    fn a_host_that_answers_is_added_with_the_version_it_gave() {
        let Ok(Prepare::Ready { def, how }) = look("proxy") else {
            panic!("ready");
        };
        assert_eq!((def.name.as_str(), def.target.as_str()), ("build.example", "build.example"));
        let How::Connect { version } = how else {
            panic!("connect");
        };
        assert!(version.is_some_and(|v| !v.is_empty()));
    }

    #[test]
    fn a_login_ssh_cannot_do_is_added_and_left_to_a_person() {
        assert!(matches!(
            look("auth-fail"),
            Ok(Prepare::Ready { how: How::Login, .. })
        ));
    }

    #[test]
    fn an_app_that_is_not_installed_is_still_added_so_its_state_can_say_how_to_fix_it() {
        assert!(matches!(
            look("not-installed"),
            Ok(Prepare::Ready { how: How::Connect { version: None }, .. })
        ));
    }

    #[test]
    fn a_machine_ssh_cannot_reach_adds_nothing_and_says_why() {
        let Err(why) = look("unreachable") else {
            panic!("an error");
        };
        assert!(why.starts_with("build.example: "), "{why}");
    }

    #[test]
    fn a_target_ssh_would_read_as_an_option_is_refused_before_anything_runs() {
        let never = |_: &MachineDef| -> Probe { panic!("nothing is to run") };
        assert!(prepare(&[], "-oProxyCommand=x", None, &never).is_err());
        assert!(prepare(&[], "a b", None, &never).is_err());
    }

    #[test]
    fn a_name_that_is_taken_is_refused_and_a_suggestion_steps_around_it() {
        let ok = |_: &MachineDef| Probe::Ready { version: "v".into() };
        let existing = [def("build", "other.example")];
        assert!(prepare(&existing, "x.example", Some("BUILD"), &ok).is_err());
        let Ok(Prepare::Ready { def, .. }) = prepare(&existing, "build", None, &ok) else {
            panic!("ready");
        };
        assert_eq!(def.name, "build-2");
    }

    #[test]
    fn a_target_nothing_can_name_asks_for_a_name() {
        let ok = |_: &MachineDef| Probe::Ready { version: String::new() };
        assert_eq!(prepare(&[], "me@", None, &ok), Ok(Prepare::NeedsName));
        assert!(matches!(
            prepare(&[], "me@", Some("lab"), &ok),
            Ok(Prepare::Ready { .. })
        ));
    }

    #[test]
    fn a_cancelled_look_adds_nothing() {
        let cancelled = |_: &MachineDef| Probe::Cancelled;
        assert_eq!(prepare(&[], "build", None, &cancelled), Ok(Prepare::Cancelled));
    }

    #[test]
    fn the_hosts_offered_leave_out_machines_already_added() {
        let host = |alias: &str| SshHost {
            alias: alias.into(),
            host_name: Some(format!("{alias}.example")),
            user: Some("me".into()),
        };
        let offered = host_choices(&[host("build"), host("box")], &[def("b", "BUILD")]);
        assert_eq!(
            offered,
            [HostChoice {
                alias: "box".into(),
                detail: "me@box.example".into()
            }]
        );
    }
}
