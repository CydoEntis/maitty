mod app;
mod cli;
mod client;
pub use client::connections;
pub use client::link;
pub use client::ssh_hosts;
pub use maitty_core::{
    alert, brand, config, gitfs, installed_executable, ipc, link_failure, machine_probe, proc,
    project, prompt, references, remote_install, remote_paste, restart, reveal, secret_files, shim, snapshot,
    shared_settings, sync, tidy, version,
};
pub use maitty_shared::{clock, conversation, hidden, layout, output_line, protocol};
mod keys;
pub use maitty_settings::theme;

pub use conversation::{ApprovalDecision, ProviderRequestId};

pub fn run() {
    app::run();
}
