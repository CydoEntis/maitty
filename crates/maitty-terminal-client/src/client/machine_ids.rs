//! Which daemon each saved machine name was first seen with: the store and its check live in
//! `maitty-machines`. This is where the terminal app says which file it keeps them in, and
//! where its own tests are kept from touching a real one.

pub use maitty_machines::machine_ids::*;

pub fn load() -> MachineIds {
    if cfg!(test) {
        return MachineIds::default();
    }
    MachineIds::load(&MachineIds::file_in(&crate::config::data_dir()))
}

pub fn save(ids: &MachineIds) {
    if cfg!(test) {
        return;
    }
    ids.save(&MachineIds::file_in(&crate::config::data_dir()));
}
