pub mod alert;
pub mod brand;
pub mod config;
mod daemon;
pub mod gitfs;
pub mod installed_executable;
pub mod ipc;
mod login_path;
pub use maitty_machines::machine_probe;
pub mod proc;
pub mod project;
pub mod prompt;
pub mod references;
pub mod remote_install;
pub mod remote_paste;
pub mod restart;
pub mod reveal;
pub mod secret_files;
pub mod shim;
pub mod snapshot;
pub mod sync;
pub mod tidy;
pub mod version;

pub use daemon::run as run_daemon;
pub use daemon::{first_failing_line, forget_session};
pub use maitty_machines::link_failure;
pub use maitty_settings::{keys, theme};
pub use maitty_shared::{clock, conversation, hidden, layout, output_line, protocol, repo};

#[cfg(test)]
mod test_support;
