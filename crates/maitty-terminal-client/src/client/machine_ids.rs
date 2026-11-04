//! Which daemon each saved machine name was first seen with. A machine's saved layout is kept
//! under its name, so a host or DNS name that now reaches a different machine must not pick
//! that layout up. The id detects a swapped host; it does not authenticate one.
//!
//! Kept per user in `machine-ids.json` in the data folder, never synced.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

const FILE_NAME: &str = "machine-ids.json";
/// How much of an id is ever shown or logged.
const PREFIX_CHARS: usize = 8;
const MOST_ID_CHARS: usize = 64;

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct MachineIds {
    #[serde(default)]
    ids: BTreeMap<String, String>,
}

#[derive(Debug, PartialEq, Eq)]
pub enum IdCheck {
    /// First sight of this machine: its id is now recorded and the store needs saving.
    Recorded,
    Known,
    /// An old daemon with no id, on a machine never recorded with one.
    Unchecked,
}

#[derive(Debug, PartialEq, Eq)]
pub struct DaemonIdMismatch {
    pub machine: String,
    pub recorded_prefix: String,
    /// `None` when the daemon now answering reports no id.
    pub seen_prefix: Option<String>,
}

impl fmt::Display for DaemonIdMismatch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let seen = self.seen_prefix.as_deref().unwrap_or("none");
        write!(
            f,
            "a different machine answers at {} (id {seen} instead of {}). If you replaced that \
             machine, open Machines, remove it and add it again; otherwise check the host name \
             and DNS.",
            self.machine, self.recorded_prefix
        )
    }
}

impl MachineIds {
    pub fn load() -> MachineIds {
        if cfg!(test) {
            return MachineIds::default();
        }
        crate::config::read_state(&crate::config::data_dir().join(FILE_NAME))
    }

    pub fn save(&self) {
        if cfg!(test) {
            return;
        }
        let file = crate::config::data_dir().join(FILE_NAME);
        if let Ok(text) = serde_json::to_string_pretty(self)
            && let Err(e) = crate::config::write_atomic(&file, text)
        {
            tracing::warn!("couldn't save the machine ids: {e}");
        }
    }

    /// Drop what is recorded for `machine`, so the next daemon to answer at that name is taken
    /// as the machine. True when something was recorded.
    pub fn forget(&mut self, machine: &str) -> bool {
        self.ids.remove(machine).is_some()
    }
}

/// Compare the id a machine's daemon reports with the one recorded for its name. An old
/// daemon with no id is refused once an id is recorded: a downgrade at a saved name is the
/// swapped-host case too.
pub fn check_daemon_id(
    known: &mut MachineIds,
    machine: &str,
    seen: Option<&str>,
) -> Result<IdCheck, DaemonIdMismatch> {
    let seen = seen.filter(|id| is_id(id));
    let recorded = known.ids.get(machine);
    match (recorded, seen) {
        (None, None) => Ok(IdCheck::Unchecked),
        (None, Some(id)) => {
            known.ids.insert(machine.to_owned(), id.to_owned());
            Ok(IdCheck::Recorded)
        }
        (Some(recorded), Some(id)) if recorded == id => Ok(IdCheck::Known),
        (Some(recorded), seen) => Err(DaemonIdMismatch {
            machine: machine.to_owned(),
            recorded_prefix: prefix(recorded),
            seen_prefix: seen.map(prefix),
        }),
    }
}

fn is_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= MOST_ID_CHARS && id.bytes().all(|b| b.is_ascii_hexdigit())
}

fn prefix(id: &str) -> String {
    id.chars().take(PREFIX_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const A: &str = "aaaaaaaa11111111aaaaaaaa11111111";
    const B: &str = "bbbbbbbb22222222bbbbbbbb22222222";

    #[test]
    fn first_sight_records_and_the_same_id_is_fine_after() {
        let mut known = MachineIds::default();
        assert_eq!(
            check_daemon_id(&mut known, "box", Some(A)),
            Ok(IdCheck::Recorded)
        );
        assert_eq!(
            check_daemon_id(&mut known, "box", Some(A)),
            Ok(IdCheck::Known)
        );
    }

    #[test]
    fn a_different_id_at_the_name_is_refused_with_what_to_do() {
        let mut known = MachineIds::default();
        check_daemon_id(&mut known, "box", Some(A)).unwrap();
        let error = check_daemon_id(&mut known, "box", Some(B)).unwrap_err();
        assert_eq!(
            error.to_string(),
            "a different machine answers at box (id bbbbbbbb instead of aaaaaaaa). If you \
             replaced that machine, open Machines, remove it and add it again; otherwise check \
             the host name and DNS."
        );
        assert_eq!(
            check_daemon_id(&mut known, "box", Some(A)),
            Ok(IdCheck::Known),
            "a refusal records nothing"
        );
    }

    #[test]
    fn an_old_daemon_connects_on_a_machine_never_recorded_and_records_nothing() {
        let mut known = MachineIds::default();
        assert_eq!(
            check_daemon_id(&mut known, "box", None),
            Ok(IdCheck::Unchecked)
        );
        assert!(known.ids.is_empty());
    }

    #[test]
    fn an_old_daemon_answering_after_an_id_was_recorded_is_refused() {
        let mut known = MachineIds::default();
        check_daemon_id(&mut known, "box", Some(A)).unwrap();
        let error = check_daemon_id(&mut known, "box", None).unwrap_err();
        assert_eq!(error.seen_prefix, None);
        assert!(error.to_string().contains("id none instead of aaaaaaaa"));
    }

    #[test]
    fn an_id_that_is_not_hex_counts_as_no_id() {
        let mut known = MachineIds::default();
        assert_eq!(
            check_daemon_id(&mut known, "box", Some("\x1b[31mred")),
            Ok(IdCheck::Unchecked)
        );
    }

    #[test]
    fn forgetting_a_machine_lets_the_next_daemon_be_taken_as_it() {
        let mut known = MachineIds::default();
        check_daemon_id(&mut known, "box", Some(A)).unwrap();
        assert!(known.forget("box"));
        assert!(!known.forget("box"));
        assert_eq!(
            check_daemon_id(&mut known, "box", Some(B)),
            Ok(IdCheck::Recorded)
        );
    }

    #[test]
    fn the_file_round_trips_and_tolerates_unknown_fields() {
        let mut known = MachineIds::default();
        check_daemon_id(&mut known, "box", Some(A)).unwrap();
        let text = serde_json::to_string(&known).unwrap();
        let back: MachineIds = serde_json::from_str(&text).unwrap();
        assert_eq!(back.ids, known.ids);
        let later: MachineIds =
            serde_json::from_str(r#"{"ids":{"box":"aa"},"added_later":[1,2]}"#).unwrap();
        assert_eq!(later.ids["box"], "aa");
    }

    #[test]
    fn a_corrupt_file_starts_empty() {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-machine-ids-{}"),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let file = dir.join(FILE_NAME);
        std::fs::write(&file, "{ not json").unwrap();
        let known: MachineIds = crate::config::read_state(&file);
        assert!(known.ids.is_empty());
    }
}
