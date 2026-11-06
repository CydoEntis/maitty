//! This computer's machines, as the window knows them: names only. The list is read from
//! `config.local.toml` once, when the app starts, through `maitty-machines` (the terminal app
//! reads it the same way and shares it, ADR 0007); a hand edit applies on the next start. A bad
//! entry leaves no machines and an error the window shows, and never stops the app.

use maitty_machines::local_machines::MachineList;
use serde::Serialize;

/// What the window is told: a machine's name is all it ever knows of one.
#[derive(Debug, Serialize, PartialEq)]
pub struct MachineNames {
    pub names: Vec<String>,
    pub error: Option<String>,
}

pub fn names_of(list: &MachineList) -> MachineNames {
    MachineNames {
        names: list.machines.iter().map(|m| m.name.clone()).collect(),
        error: list.error.clone(),
    }
}

/// The list read at start.
pub struct Machines(pub MachineList);

#[tauri::command]
pub fn list_machines(machines: tauri::State<'_, Machines>) -> MachineNames {
    names_of(&machines.0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_machines::MachineDef;
    use maitty_machines::local_machines::machines_beside;

    fn def(name: &str) -> MachineDef {
        MachineDef {
            name: name.into(),
            target: format!("{name}.example"),
            ..MachineDef::default()
        }
    }

    #[test]
    fn the_window_is_given_names_and_never_where_they_point() {
        let list = MachineList {
            machines: vec![def("build"), def("box")],
            error: None,
        };
        let told = names_of(&list);
        assert_eq!(told.names, ["build", "box"]);
        assert_eq!(told.error, None);
        let json = serde_json::to_string(&told).unwrap();
        assert!(!json.contains("example"), "{json}");
    }

    #[test]
    fn a_bad_entry_reaches_the_window_as_an_error_and_no_machines() {
        let dir = std::env::temp_dir().join(format!("maitty-gui-machines-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("config.local.toml"),
            "[[machines]]\nname = 'bad'\ntarget = 'has space'\n",
        )
        .unwrap();
        let told = names_of(&machines_beside(&dir.join("config.toml")));
        assert!(told.names.is_empty());
        assert!(told.error.is_some_and(|e| e.contains("bad")));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn no_machine_file_is_an_empty_list_without_an_error() {
        let dir =
            std::env::temp_dir().join(format!("maitty-gui-nomachines-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let told = names_of(&machines_beside(&dir.join("config.toml")));
        assert_eq!(
            told,
            MachineNames {
                names: Vec::new(),
                error: None
            }
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
