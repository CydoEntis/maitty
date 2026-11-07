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
mod remote_browse;
mod remote_folder;
mod remote_get;
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
pub use brand::APP_COMMAND;
pub use remote_browse::{Index, Listing, index_files, list_folder, parse_index, parse_listing};
pub use remote_folder::get_folder;
pub use remote_get::{MAX_GET_BYTES, get_file, get_path, local_name};
pub use remote_scripts::{
    GET_MARKER, GET_MISSING, GET_NOT_A_FILE, GET_READ_FAILED, GET_SCRIPT, GET_TOO_BIG,
    GET_UNREADABLE, PUT_FOLDER_SCRIPT, FOLDER_MARKER, FOLDER_MISSING, FOLDER_NOT_A_FOLDER, FOLDER_SCRIPT,
    FOLDER_UNREADABLE, INDEX_DEPTH, INDEX_MARKER, INDEX_MISSING, INDEX_NOT_A_FOLDER, INDEX_SCRIPT,
    INDEX_UNREADABLE, INSTALL_SCRIPT, LIST_FILES_MARKER, LIST_MARKER, LIST_MISSING,
    LIST_NOT_A_FOLDER, LIST_SCRIPT, LIST_UNREADABLE, PASTE_MARKER, PASTE_SCRIPT, PLATFORM_SCRIPT,
    WHERE_SCRIPT,
};

/// Windows: start a console program without opening a console window.
#[cfg(windows)]
pub const CREATE_NO_WINDOW: u32 = 0x0800_0000;
