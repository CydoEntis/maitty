mod app;
mod cli;
mod client;
pub use maitty_core::{
    alert, brand, config, gitfs, installed_executable, ipc, proc, project, restart, reveal,
    secret_files, shim, snapshot, sync, tidy, version,
};
pub use maitty_shared::{clock, conversation, hidden, layout, output_line, protocol};
mod keys;
pub use maitty_settings::theme;
mod references;

pub use conversation::{ApprovalDecision, ProviderRequestId};

pub fn run() {
    app::run();
}
