//! How the app reaches other computers: the `[[machines]]` definition and its checks, the ssh
//! line and its hardening, the failure classes and the dial, shared by the terminal app and
//! the desktop app.

mod brand;
mod def;
pub mod link_failure;
mod machine;
mod remote_scripts;
mod ssh_command;

pub use def::{
    LOCAL_MACHINE, MACHINE_NAME_MAX, MachineDef, VIA_NOT_ON_WINDOWS, VIA_PLACEHOLDER,
    validate_machines, validate_machines_on,
};
pub use machine::{
    CONNECT_BUDGET, Dial, Machine, NO_DAEMON_EXIT, OtherVersion, PROXY_ATTACH_ONLY_FLAG,
    SSH_PROGRAM, hello, open_remote, other_version,
};
pub use remote_scripts::{INSTALL_SCRIPT, PLATFORM_SCRIPT, WHERE_SCRIPT};

/// Windows: start a console program without opening a console window.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
