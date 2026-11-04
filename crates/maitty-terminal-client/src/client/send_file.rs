//! Sending a file from this computer to the machine the focused pane is on (ADR 0050). The leader
//! key opens a prompt for the file's path (a file dropped on the terminal pastes its path into
//! it); the upload runs off the UI thread over the machine's own ssh line (`remote_image`), shows
//! its progress in a toast, and can be stopped by the same key or by closing the toast. When it
//! lands, the path on the far side is typed into the pane, as a local paste types a local path.

use super::screen::typed_path;
use super::{App, Bg, ClientTerm, Mode, PromptKind, files, remote_image};
use crate::ipc;
use maitty_machines::ssh_run::Watch;
use std::sync::atomic::Ordering;
use std::time::Instant;

/// The one file being sent: what the progress toast says and how to stop it.
pub(super) struct Sending {
    file: String,
    machine: String,
    watch: Watch,
}

impl Sending {
    /// The toast text: the share sent so far once the size is known.
    fn progress(&self) -> String {
        let total = self.watch.total.load(Ordering::Relaxed);
        let sent = self.watch.sent.load(Ordering::Relaxed);
        let (file, machine) = (&self.file, &self.machine);
        if total == 0 {
            return format!("sending {file} to {machine}…");
        }
        let percent = (sent.min(total) * 100 / total).min(100);
        format!("sending {file} to {machine}… {percent}% · press the send key again to stop")
    }
}

impl App {
    /// The leader key: stop the file being sent, else ask which file to send to this pane's
    /// machine.
    pub(super) fn send_file_action(&mut self) {
        if self.cancel_send() {
            return;
        }
        let Some(term) = self.typing_to() else { return };
        if !remote_image::is_remote(term) {
            self.notify(
                "this pane is on this computer; there is nothing to send".into(),
                false,
            );
            return;
        }
        self.mode = Mode::Prompt {
            kind: PromptKind::SendFile(term),
            input: String::new(),
        };
    }

    /// Stop the send in progress; false when none is running.
    pub(super) fn cancel_send(&mut self) -> bool {
        let Some(sending) = &self.sending else {
            return false;
        };
        sending.watch.cancel.store(true, Ordering::Relaxed);
        let stopping = format!("stopping the send to {}…", sending.machine);
        self.notify(stopping, false);
        true
    }

    /// Keep the progress toast showing while a send runs; called about once a second.
    pub(super) fn show_send_progress(&mut self) {
        let Some(sending) = &self.sending else { return };
        // A stop already asked for says so, and is not overwritten by the percentage.
        if sending.watch.cancel.load(Ordering::Relaxed) {
            return;
        }
        // Not `notify`: progress is not news for the notification history.
        self.notice = Some((sending.progress(), Instant::now(), false));
        (self.notice_term, self.notice_restart, self.notice_card) = (None, false, None);
        self.notice_hint = false;
        self.dirty = true;
    }

    /// The prompt's answer: the path, quotes a terminal adds on a drop taken off, a relative one
    /// starting where maitty was started.
    pub(super) fn start_send(&mut self, term: ClientTerm, input: &str) {
        if self.sending.is_some() {
            self.notify("a file is already being sent".into(), true);
            return;
        }
        let here = std::env::current_dir().unwrap_or_default();
        let Some(path) = typed_path(input, &here) else {
            return;
        };
        let Some((machine, name)) = self.machine_for(term) else {
            return;
        };
        let watch = Watch::default();
        self.sending = Some(Sending {
            file: remote_image::shown(&path),
            machine: name.clone(),
            watch: watch.clone(),
        });
        self.show_send_progress();
        self.spawn_bg(move || {
            let sent = remote_image::send_file(&machine, &name, &path, &watch);
            Bg::Then(Box::new(move |app: &mut App| {
                // Only this send's own state is cleared: a newer one may have started.
                if app
                    .sending
                    .as_ref()
                    .is_some_and(|s| std::sync::Arc::ptr_eq(&s.watch.cancel, &watch.cancel))
                {
                    app.sending = None;
                }
                match sent {
                    Ok(remote_path) => {
                        app.type_path_into(term, &remote_path);
                        app.notify(
                            format!("sent {} to {name}", remote_image::shown(&path)),
                            false,
                        );
                    }
                    Err(e) => app.notify(e, true),
                }
            }))
        });
    }

    /// The machine a pane is on as the ssh route to it, with its name in words; a toast says so
    /// when this computer does not know it.
    pub(super) fn machine_for(&mut self, term: ClientTerm) -> Option<(ipc::Machine, String)> {
        let name = self.machine_name(term.machine).to_owned();
        let def = self
            .cfg
            .machines
            .iter()
            .find(|def| def.name.eq_ignore_ascii_case(&name))
            .cloned();
        let Some(def) = def else {
            self.notify(
                format!("{name} is not one of this computer's machines"),
                true,
            );
            return None;
        };
        Some((
            ipc::Machine::from_def_via(&def, &self.machine_flow.ssh),
            name,
        ))
    }

    /// Type `path` into `term` the way a local paste types a local path.
    pub(super) fn type_path_into(&mut self, term: ClientTerm, path: &str) {
        let text = files::quote_path(std::path::Path::new(path));
        let bracketed = self
            .parsers
            .get(&term)
            .is_some_and(|p| p.screen().bracketed_paste());
        let data = if bracketed {
            format!("\x1b[200~{text} \x1b[201~")
        } else {
            format!("{text} ")
        };
        self.send_input(term, data.into_bytes());
    }
}
