//! How the app reaches other computers: the `[[machines]]` definition and its checks, shared
//! by the terminal app and the desktop app.

mod def;

pub use def::{
    LOCAL_MACHINE, MACHINE_NAME_MAX, MachineDef, VIA_NOT_ON_WINDOWS, VIA_PLACEHOLDER,
    validate_machines, validate_machines_on,
};
