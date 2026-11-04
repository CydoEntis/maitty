//! How the app reaches other computers: the `[[machines]]` definition and its checks, the ssh
//! line and its hardening, the failure classes and the dial, shared by the terminal app and
//! the desktop app.

mod brand;
pub mod connections;
mod def;
mod files;
mod ids;
pub mod link;
pub mod link_failure;
pub mod local_machines;
mod machine;
pub mod machine_config;
pub mod machine_ids;
pub mod machine_probe;
pub mod remote_paste;
pub mod remote_policy;
mod remote_scripts;
pub mod remote_send;
pub mod remote_snapshot;
mod ssh_command;
pub mod ssh_hosts;
pub mod ssh_run;

pub use def::{
    LOCAL_MACHINE, MACHINE_NAME_MAX, MachineDef, VIA_NOT_ON_WINDOWS, VIA_PLACEHOLDER,
    validate_machines, validate_machines_on,
};
pub use files::{read_state, write_atomic};
pub use ids::{ClientTerm, MachineId, Outgoing};
pub use machine::{
    CONNECT_BUDGET, Dial, Machine, NO_DAEMON_EXIT, OtherVersion, PROXY_ATTACH_ONLY_FLAG,
    SSH_PROGRAM, hello, open_remote, other_version,
};
pub use remote_scripts::{
    INSTALL_SCRIPT, PASTE_MARKER, PASTE_SCRIPT, PLATFORM_SCRIPT, WHERE_SCRIPT,
};

/// Windows: start a console program without opening a console window.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
