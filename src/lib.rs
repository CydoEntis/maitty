mod alert;
mod app;
mod brand;
mod cli;
mod client;
mod clock;
mod config;
mod daemon;
pub use maitty_shared::{conversation, layout, protocol};
mod detection;
mod gitfs;
mod hidden;
mod ipc;
mod keys;
mod mcp;
mod output_line;
mod proc;
mod project;
mod references;
mod restart;
mod reveal;
mod secret_files;
mod shim;
mod snapshot;
mod sync;
mod theme;
mod tidy;
mod update;

#[cfg(test)]
mod test_support;

pub use conversation::{ApprovalDecision, ProviderRequestId};

pub fn run() {
    app::run();
}
