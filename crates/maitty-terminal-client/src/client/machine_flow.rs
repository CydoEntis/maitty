//! What the Machines dialog does with a choice. Adding looks at the machine first, with no
//! prompt (a host that works with keys is added in that one step); a login that needs a person
//! opens ssh in a popup on this computer, and the machine connects when it closes. Removing
//! takes the machine out of the config, the sidebar and the window's records, and leaves the
//! machine itself, its sessions and its saved layout alone.

use super::screen::machines::{
    MachineNotice, MachineStep, MachinesView, Outcome, connect_target, parse_target, removal_note,
};
use super::*;
use crate::client::link::ConnectionState;
use crate::client::machine_config;
use crate::client::machine_ids;
use crate::link_failure::FailureClass;
use crate::machine_probe::{self, Probe};
use maitty_settings::config::MachineDef;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

/// The longest a look at a machine may take, whatever ssh does: past ssh's own connect timeout.
const PROBE_LIMIT: Duration = Duration::from_secs(12);
/// How long a popup may fail to show up before the machine is dialled anyway: a popup that
/// ended between two reports of the daemon is never seen.
const POPUP_UNSEEN_FOR: Duration = Duration::from_secs(20);
/// A name that no configured machine has, for telling a bad target from a bad name.
const SPARE_NAME: &str = "machine-to-add";

/// The probe in flight. Its result is taken only while `token` is the current one.
struct Probing {
    token: u64,
    cancel: Arc<AtomicBool>,
}

/// A first login open in a popup: the popup's pane once it has shown up, and the popups
/// that were there before, which are not it.
struct Login {
    name: String,
    before: HashSet<TermId>,
    term: Option<TermId>,
    since: Instant,
}

pub(super) struct Flow {
    /// The program and leading arguments new machines are reached with.
    pub ssh: Vec<String>,
    /// Where machines are written.
    pub local_config: PathBuf,
    pub probe_limit: Duration,
    probe: Option<Probing>,
    next_token: u64,
    logins: Vec<Login>,
    /// Machines removed, whose connections the event loop lets go of.
    dropped: Vec<MachineId>,
}

impl Default for Flow {
    fn default() -> Flow {
        Flow {
            ssh: vec![ipc::SSH_PROGRAM.to_string()],
            local_config: machine_config::local_config_path(),
            probe_limit: PROBE_LIMIT,
            probe: None,
            next_token: 0,
            logins: Vec::new(),
            dropped: Vec::new(),
        }
    }
}

fn say(text: impl Into<String>, error: bool) -> MachineNotice {
    MachineNotice {
        text: text.into(),
        error,
    }
}

impl App {
    pub(super) fn open_machines(&mut self) {
        self.mode = Mode::Machines(Box::new(MachinesView::new(cfg!(windows))));
        ssh_hosts::load_in_background(self, |app, hosts| {
            app.with_machines_view(|v| v.hosts = Some(hosts));
        });
    }

    pub(super) fn with_machines_view(&mut self, f: impl FnOnce(&mut MachinesView)) {
        if let Mode::Machines(v) = &mut self.mode {
            f(v);
            self.dirty = true;
        }
    }

    /// Tell the person: in the dialog when it is open, else as a toast.
    fn machines_say(&mut self, notice: MachineNotice) {
        if matches!(self.mode, Mode::Machines(_)) {
            self.with_machines_view(|v| v.notice = Some(notice));
        } else {
            self.notify(notice.text, notice.error);
        }
    }

    /// Enter on the input: what was typed, or the chosen suggestion.
    pub(in crate::client) fn machines_add_chosen(&mut self, v: &mut MachinesView) {
        if parse_target(&v.query).is_some_and(|p| p.path.is_some()) {
            v.notice = Some(say(
                "Opening a folder after connecting is not built yet: type just user@host.",
                true,
            ));
            return;
        }
        let Some(target) = connect_target(v, &self.cfg.machines) else {
            v.notice = Some(say("Type user@host, or press ↓ to pick a host.", true));
            return;
        };
        let taken: Vec<String> = self.cfg.machines.iter().map(|m| m.name.clone()).collect();
        match machine_config::suggest_name(&target, &taken) {
            Some(name) => self.machines_add_named(v, target, name),
            None => {
                v.step = MachineStep::Name {
                    target,
                    name: String::new(),
                    error: None,
                }
            }
        }
    }

    /// Check `target` and, if it holds, look at the machine before anything is written.
    pub(in crate::client) fn machines_add_named(
        &mut self,
        v: &mut MachinesView,
        target: String,
        name: String,
    ) {
        if let Some(why) = machine_config::refusal(&self.cfg.machines, SPARE_NAME, &target) {
            v.step = MachineStep::Browse;
            v.notice = Some(say(why, true));
            return;
        }
        if let Some(why) = machine_config::refusal(&self.cfg.machines, &name, &target) {
            v.step = MachineStep::Name {
                target,
                name,
                error: Some(why),
            };
            return;
        }
        v.notice = None;
        v.step = MachineStep::Checking {
            target: target.clone(),
        };
        self.machines_probe(MachineDef {
            name,
            target,
            ..MachineDef::default()
        });
    }

    fn machines_probe(&mut self, def: MachineDef) {
        let token = self.machine_flow.next_token;
        self.machine_flow.next_token += 1;
        let cancel = Arc::new(AtomicBool::new(false));
        self.machine_flow.probe = Some(Probing {
            token,
            cancel: cancel.clone(),
        });
        let machine = ipc::Machine::from_def_via(&def, &self.machine_flow.ssh);
        let limit = self.machine_flow.probe_limit;
        self.spawn_bg(move || {
            let result = machine_probe::probe(&machine, limit, &cancel);
            Bg::Then(Box::new(move |app: &mut App| {
                app.machines_probed(token, def, result)
            }))
        });
    }

    /// Esc while checking: ssh is ended and whatever it says is ignored.
    pub(in crate::client) fn machines_cancel_probe(&mut self, v: &mut MachinesView) {
        self.end_probe();
        v.step = MachineStep::Browse;
        v.notice = Some(say("Cancelled. Nothing was added.", false));
    }

    fn end_probe(&mut self) {
        if let Some(probe) = self.machine_flow.probe.take() {
            probe.cancel.store(true, Ordering::Relaxed);
        }
    }

    pub(super) fn machines_probed(&mut self, token: u64, def: MachineDef, result: Probe) {
        if self.machine_flow.probe.as_ref().map(|p| p.token) != Some(token) {
            return;
        }
        self.machine_flow.probe = None;
        self.with_machines_view(|v| v.step = MachineStep::Browse);
        let notice = match result {
            Probe::Cancelled => return,
            Probe::Ready { version } => {
                self.machine_added(&def, Added::Connect, &version, Outcome::Ready(version.clone()))
            }
            Probe::Failed(FailureClass::Login) => {
                self.machine_added(&def, Added::Login, "", Outcome::Login)
            }
            Probe::Failed(FailureClass::Unreachable(why)) => {
                say(format!("{}: {why}", def.target), true)
            }
            // Reached, but the app is missing, old, or not running: the connection's own state
            // says which, with the fix, once it dials.
            Probe::Failed(_) => self.machine_added(&def, Added::Connect, "", Outcome::Fix),
        };
        self.machines_say(notice);
    }

    /// Write the machine to the config and bring it up: dialled now, or after a login in a popup.
    fn machine_added(
        &mut self,
        def: &MachineDef,
        how: Added,
        version: &str,
        outcome: Outcome,
    ) -> MachineNotice {
        if let Err(e) =
            machine_config::add_machine(&self.machine_flow.local_config, &def.name, &def.target)
        {
            return say(format!("Could not add {}: {e:#}", def.name), true);
        }
        self.cfg.machines.push(def.clone());
        self.with_machines_view(|v| {
            v.step = MachineStep::Added {
                name: def.name.clone(),
                target: def.target.clone(),
                how: outcome,
            }
        });
        if how == Added::Login {
            match self.machine_first_login(def) {
                Ok(()) => {
                    return say(
                        format!(
                            "Added {}. ssh needs you to log in: use the popup.",
                            def.name
                        ),
                        false,
                    );
                }
                Err(e) => {
                    self.machine_start(def);
                    return say(
                        format!(
                            "Added {}. Run `ssh {}` once in a terminal to log in, then retry ({e:#}).",
                            def.name, def.target
                        ),
                        true,
                    );
                }
            }
        }
        self.machine_start(def);
        let there = if version.is_empty() {
            String::new()
        } else {
            format!(" {version} answered there.")
        };
        say(format!("Added {}.{there} Connecting.", def.name), false)
    }

    /// Start dialling a configured machine. It shows as connecting at once.
    fn machine_start(&mut self, def: &MachineDef) {
        let machine = ipc::Machine::from_def_via(def, &self.machine_flow.ssh);
        let Some(id) = self.links.start(machine) else {
            return;
        };
        let remote = self.remotes.entry(id).or_default();
        remote.name = def.name.clone();
        remote.state = ConnectionState::Connecting;
        self.screen_fresh();
        self.dirty = true;
    }

    /// Open ssh in a popup on this computer, where its prompts can be answered.
    fn machine_first_login(&mut self, def: &MachineDef) -> anyhow::Result<()> {
        let cmd = ipc::Machine::from_def_via(def, &self.machine_flow.ssh).first_login_command()?;
        self.machine_flow.logins.push(Login {
            name: def.name.clone(),
            before: self.popup_terms(),
            term: None,
            since: Instant::now(),
        });
        // A popup has the keys only in plain typing mode; the dialog closes and the toast says why.
        self.leave_shortcut_mode();
        self.cmd_on(MachineId::LOCAL, Command::Popup { cmd, cwd: None });
        Ok(())
    }

    fn popup_terms(&self) -> HashSet<TermId> {
        self.snap
            .terms
            .iter()
            .filter(|(_, t)| t.popup)
            .map(|(id, _)| *id)
            .collect()
    }

    #[cfg(test)]
    pub(super) fn machine_login_waiting(&self, name: &str) -> bool {
        self.machine_flow
            .logins
            .iter()
            .any(|l| l.name.eq_ignore_ascii_case(name))
    }

    /// This computer's daemon reported: a login's popup has shown up, or has closed.
    pub(super) fn on_login_snapshot(&mut self) {
        if self.machine_flow.logins.is_empty() {
            return;
        }
        let popups = self.popup_terms();
        let mut finished = Vec::new();
        for login in &mut self.machine_flow.logins {
            match login.term {
                None => login.term = popups.difference(&login.before).next().copied(),
                Some(term) if !popups.contains(&term) => finished.push(login.name.clone()),
                Some(_) => {}
            }
        }
        for name in finished {
            self.machine_login_closed(&name);
        }
    }

    /// A login whose popup was never seen is taken as over once it has been waited for long enough.
    pub(super) fn expire_unseen_logins(&mut self) {
        let unseen: Vec<String> = self
            .machine_flow
            .logins
            .iter()
            .filter(|l| l.term.is_none() && l.since.elapsed() >= POPUP_UNSEEN_FOR)
            .map(|l| l.name.clone())
            .collect();
        for name in unseen {
            self.machine_login_closed(&name);
        }
    }

    /// The popup closed: dial the machine. Whether the login
    /// worked is the connection's to say, with its own fix if it did not.
    pub(super) fn machine_login_done(&mut self, name: &str) -> Option<MachineNotice> {
        self.machine_flow
            .logins
            .retain(|l| !l.name.eq_ignore_ascii_case(name));
        let def = self.configured(name)?;
        Some(self.machine_retry(&def.name))
    }

    /// The popup closed on its own: dial, and tell the person.
    fn machine_login_closed(&mut self, name: &str) {
        if let Some(notice) = self.machine_login_done(name) {
            self.machines_say(notice);
        }
    }

    fn configured(&self, name: &str) -> Option<MachineDef> {
        self.cfg
            .machines
            .iter()
            .find(|m| m.name.eq_ignore_ascii_case(name))
            .cloned()
    }

    /// Dial a machine now, letting a daemon that was refused be looked at again. A machine
    /// with no supervisor gets one.
    pub(in crate::client) fn machine_retry(&mut self, name: &str) -> MachineNotice {
        let Some(def) = self.configured(name) else {
            return say(format!("{name} is not in your machines."), true);
        };
        match self.links.id_of(&def.name) {
            Some(id)
                if self
                    .remotes
                    .get(&id)
                    .is_some_and(|r| r.state.is_connected()) =>
            {
                say(format!("{} is connected.", def.name), false)
            }
            Some(id) => {
                self.refused.remove(&id);
                self.links.retry(id);
                say(format!("Trying {} again.", def.name), false)
            }
            None => {
                self.machine_start(&def);
                say(format!("Connecting to {}.", def.name), false)
            }
        }
    }

    /// Take a machine out of the config, the sidebar and the window's records. Its sessions,
    /// its daemon and its saved layout stay where they are.
    pub(in crate::client) fn machines_remove(&mut self, name: &str) -> MachineNotice {
        let Some(def) = self.configured(name) else {
            return say(format!("{name} is not in your machines."), true);
        };
        match machine_config::remove_machine(&self.machine_flow.local_config, &def.name) {
            Ok(true) => {}
            Ok(false) => {
                return say(
                    format!(
                        "{} is not in config.local.toml, so it cannot be removed here.",
                        def.name
                    ),
                    true,
                );
            }
            Err(e) => return say(format!("Could not remove {}: {e:#}", def.name), true),
        }
        self.cfg
            .machines
            .retain(|m| !m.name.eq_ignore_ascii_case(&def.name));
        self.machine_flow
            .logins
            .retain(|l| !l.name.eq_ignore_ascii_case(&def.name));
        if let Some(id) = self.machine_id_named(&def.name) {
            self.machine_forgotten(id);
        }
        if self.machine_ids.forget(&def.name) {
            machine_ids::save(&self.machine_ids);
        }
        self.screen_sync();
        self.screen_fresh();
        self.dirty = true;
        say(removal_note(&def.name), false)
    }

    fn machine_id_named(&self, name: &str) -> Option<MachineId> {
        self.links.id_of(name).or_else(|| {
            self.remotes
                .iter()
                .find(|(_, r)| r.name.eq_ignore_ascii_case(name))
                .map(|(id, _)| *id)
        })
    }

    /// Everything the window holds for a machine that is no longer one.
    fn machine_forgotten(&mut self, id: MachineId) {
        self.links.stop(id);
        self.remotes.remove(&id);
        self.refused.remove(&id);
        self.unsent_input.remove(&id);
        self.sync_terms(id, &Snapshot::default());
        self.machine_flow.dropped.push(id);
    }

    /// The connections to let go of: those of machines removed since the last look.
    pub(super) fn take_dropped_machines(&mut self) -> Vec<MachineId> {
        std::mem::take(&mut self.machine_flow.dropped)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Added {
    Connect,
    Login,
}

#[cfg(test)]
mod tests;
