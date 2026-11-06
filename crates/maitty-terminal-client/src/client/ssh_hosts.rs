//! The named hosts in `~/.ssh/config`, for choosing a machine to add (ADR 0007): the parser
//! lives in `maitty-machines`. This is where the terminal app reads it off its UI thread.

use super::{App, Bg};

pub use maitty_machines::ssh_hosts::*;

/// Reads `~/.ssh/config` off the UI thread and hands the hosts to `done` on it.
pub fn load_in_background(app: &App, done: impl FnOnce(&mut App, Vec<SshHost>) + Send + 'static) {
    app.spawn_bg(move || {
        let hosts = user_hosts();
        Bg::Then(Box::new(move |app: &mut App| done(app, hosts)))
    });
}
