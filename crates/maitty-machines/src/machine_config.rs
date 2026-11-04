//! A machine's `[[machines]]` entry in `config.local.toml`: added, removed, and named. The
//! entries live only in the local file, never in the config that is synced between computers,
//! and the rest of the file (comments included) is left as it was.

use crate::def::{LOCAL_MACHINE, MACHINE_NAME_MAX, MachineDef, validate_machines};
use anyhow::{Context, Result};
use std::path::Path;

const KEY: &str = "machines";

/// Append `[[machines]]` with this name and target. Nothing is written when the entry would
/// not load, or when the name is taken.
pub fn add_machine(file: &Path, name: &str, target: &str) -> Result<()> {
    let def = MachineDef {
        name: name.into(),
        target: target.into(),
        ..MachineDef::default()
    };
    let mut doc = read(file)?;
    let mut all = machines_of(&doc);
    all.push(def);
    validate_machines(&all).map_err(|e| anyhow::anyhow!("{e}"))?;
    if !doc.contains_key(KEY) {
        doc.insert(
            KEY,
            toml_edit::Item::ArrayOfTables(toml_edit::ArrayOfTables::new()),
        );
    }
    let list = doc[KEY]
        .as_array_of_tables_mut()
        .context("`machines` in config.local.toml isn't a list of [[machines]]")?;
    let mut entry = toml_edit::Table::new();
    entry.insert("name", toml_edit::value(name));
    entry.insert("target", toml_edit::value(target));
    // After what the file already says, not up among its first keys.
    entry.set_position(isize::MAX);
    list.push(entry);
    write(file, &doc)
}

/// Drop the entry named `name` (in any case). False when the file had none.
pub fn remove_machine(file: &Path, name: &str) -> Result<bool> {
    let mut doc = read(file)?;
    let Some(list) = doc
        .get_mut(KEY)
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    else {
        return Ok(false);
    };
    let before = list.len();
    list.retain(|t| {
        !t.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    });
    if list.len() == before {
        return Ok(false);
    }
    if list.is_empty() {
        doc.remove(KEY);
    }
    write(file, &doc)?;
    Ok(true)
}

fn machines_of(doc: &toml_edit::DocumentMut) -> Vec<MachineDef> {
    let text = doc.to_string();
    toml::from_str::<toml::Table>(&text)
        .ok()
        .and_then(|table| table.get(KEY).cloned())
        .and_then(|value| value.try_into().ok())
        .unwrap_or_default()
}

fn read(file: &Path) -> Result<toml_edit::DocumentMut> {
    let text = match std::fs::read_to_string(file) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e).context("reading config.local.toml"),
    };
    text.parse()
        .context("config.local.toml has a syntax error; fix it first")
}

fn write(file: &Path, doc: &toml_edit::DocumentMut) -> Result<()> {
    crate::files::write_atomic(file, doc.to_string()).context("writing config.local.toml")
}

/// What a machine may do to this computer, each off unless its entry says `true`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Grant {
    Clipboard,
    Notices,
    Sounds,
}

impl Grant {
    /// The key in the entry.
    pub fn key(self) -> &'static str {
        match self {
            Grant::Clipboard => "clipboard",
            Grant::Notices => "notices",
            Grant::Sounds => "sounds",
        }
    }

    pub fn parse(key: &str) -> Option<Grant> {
        [Grant::Clipboard, Grant::Notices, Grant::Sounds]
            .into_iter()
            .find(|grant| grant.key() == key)
    }
}

/// Turn one of the entry's grants on or off. Off removes the key (a missing key means off), so the
/// entry stays as short as it was. False when the file has no entry called `name`.
pub fn set_grant(file: &Path, name: &str, grant: Grant, on: bool) -> Result<bool> {
    let mut doc = read(file)?;
    let Some(list) = doc
        .get_mut(KEY)
        .and_then(toml_edit::Item::as_array_of_tables_mut)
    else {
        return Ok(false);
    };
    let Some(entry) = list.iter_mut().find(|t| {
        t.get("name")
            .and_then(|n| n.as_str())
            .is_some_and(|n| n.eq_ignore_ascii_case(name))
    }) else {
        return Ok(false);
    };
    if on {
        entry.insert(grant.key(), toml_edit::value(true));
    } else {
        entry.remove(grant.key());
    }
    write(file, &doc)?;
    Ok(true)
}

/// A name for a machine reached by `target`: its host without any user, with every character
/// the name may not have turned into `-`, and a number after it when `taken` has it already.
/// None when nothing usable is left of the target, and the person must type one.
pub fn suggest_name(target: &str, taken: &[String]) -> Option<String> {
    let host = target.rsplit('@').next().unwrap_or(target);
    let plain: String = host
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.') {
                c
            } else {
                '-'
            }
        })
        .collect();
    let base = plain.trim_start_matches(|c: char| !c.is_ascii_alphanumeric());
    let base = base.get(..base.len().min(MACHINE_NAME_MAX)).unwrap_or(base);
    if base.is_empty() {
        return None;
    }
    let base =
        if base.chars().all(|c| c.is_ascii_digit()) || base.eq_ignore_ascii_case(LOCAL_MACHINE) {
            format!("host-{base}")
        } else {
            base.to_string()
        };
    Some(first_free(&base, taken))
}

fn first_free(base: &str, taken: &[String]) -> String {
    let is_taken = |name: &str| taken.iter().any(|t| t.eq_ignore_ascii_case(name));
    if !is_taken(base) {
        return base.to_string();
    }
    (2..)
        .map(|n| {
            let suffix = format!("-{n}");
            let room = MACHINE_NAME_MAX.saturating_sub(suffix.len());
            format!("{}{suffix}", &base[..base.len().min(room)])
        })
        .find(|name| !is_taken(name))
        .unwrap_or_else(|| base.to_string())
}

/// Why `name` and `target` cannot be a machine alongside `existing`, in the loader's words.
pub fn refusal(existing: &[MachineDef], name: &str, target: &str) -> Option<String> {
    let mut all = existing.to_vec();
    all.push(MachineDef {
        name: name.into(),
        target: target.into(),
        ..MachineDef::default()
    });
    validate_machines(&all).err().map(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-machine-config-{}-{}"),
            label,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn adding_appends_one_entry_and_keeps_what_the_file_says() {
        let file = dir("add").join("config.local.toml");
        let before = "# my machines\ntheme = \"night\"  # mine\n\n[[machines]]\nname = 'old'\ntarget = 'old.example'\nssh_args = ['-p', '2222']\n";
        std::fs::write(&file, before).unwrap();
        add_machine(&file, "build", "build").unwrap();
        let after = std::fs::read_to_string(&file).unwrap();
        assert!(after.starts_with(before), "{after}");
        assert_eq!(
            after[before.len()..].trim(),
            "[[machines]]\nname = \"build\"\ntarget = \"build\""
        );
        assert_eq!(after.matches("[[machines]]").count(), 2);
    }

    #[test]
    fn a_grant_is_set_on_the_one_entry_and_removed_again_leaving_the_rest() {
        let file = dir("grant").join("config.local.toml");
        let before = "# mine\n[[machines]]\nname = 'old'\ntarget = 'old.example'\n\n[[machines]]\nname = 'build'\ntarget = 'build' # here\n";
        std::fs::write(&file, before).unwrap();
        assert!(set_grant(&file, "BUILD", Grant::Notices, true).unwrap());
        let on = std::fs::read_to_string(&file).unwrap();
        let loaded: toml::Table = toml::from_str(&on).unwrap();
        let machines = loaded["machines"].as_array().unwrap();
        assert_eq!(machines[1]["notices"].as_bool(), Some(true));
        assert!(machines[0].get("notices").is_none(), "{on}");
        assert!(on.contains("# mine") && on.contains("# here"), "{on}");
        assert!(set_grant(&file, "build", Grant::Notices, false).unwrap());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    }

    #[test]
    fn a_grant_for_a_machine_that_is_not_there_changes_nothing() {
        let file = dir("grant-none").join("config.local.toml");
        assert!(!set_grant(&file, "ghost", Grant::Clipboard, true).unwrap());
        assert!(!file.exists());
        add_machine(&file, "box", "box").unwrap();
        let before = std::fs::read_to_string(&file).unwrap();
        assert!(!set_grant(&file, "ghost", Grant::Clipboard, true).unwrap());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
        assert_eq!(Grant::parse("sounds"), Some(Grant::Sounds));
        assert_eq!(Grant::parse("mcp"), None);
    }

    #[test]
    fn adding_to_no_file_makes_one() {
        let file = dir("new").join("config.local.toml");
        add_machine(&file, "box", "me@box").unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        let table: toml::Table = toml::from_str(&text).unwrap();
        assert_eq!(table["machines"].as_array().unwrap().len(), 1);
    }

    #[test]
    fn an_entry_that_would_not_load_is_refused_before_anything_is_written() {
        let file = dir("bad").join("config.local.toml");
        for target in ["-oProxyCommand=x", "has space", "tab\t", ""] {
            assert!(add_machine(&file, "box", target).is_err(), "{target:?}");
        }
        assert!(add_machine(&file, "bad name", "box").is_err());
        assert!(!file.exists(), "nothing was written");
        add_machine(&file, "box", "box").unwrap();
        let before = std::fs::read_to_string(&file).unwrap();
        assert!(
            add_machine(&file, "BOX", "other").is_err(),
            "a name differing by case"
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), before);
    }

    #[test]
    fn a_file_with_a_syntax_error_is_left_alone() {
        let file = dir("syntax").join("config.local.toml");
        std::fs::write(&file, "[[machines\n").unwrap();
        assert!(add_machine(&file, "box", "box").is_err());
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "[[machines\n");
    }

    #[test]
    fn removing_drops_only_that_entry() {
        let file = dir("remove").join("config.local.toml");
        std::fs::write(&file, "theme = 'x' # keep\n\n[[machines]]\nname = 'a'\ntarget = 'a'\n\n[[machines]]\nname = 'b'\ntarget = 'b'\n").unwrap();
        assert!(remove_machine(&file, "A").unwrap());
        let after = std::fs::read_to_string(&file).unwrap();
        assert!(
            after.contains("theme = 'x' # keep")
                && after.contains("name = 'b'")
                && !after.contains("name = 'a'"),
            "{after}"
        );
        assert!(!remove_machine(&file, "a").unwrap());
        assert!(remove_machine(&file, "b").unwrap());
        let last = std::fs::read_to_string(&file).unwrap();
        assert!(
            !last.contains("machines") && last.contains("theme = 'x' # keep"),
            "{last}"
        );
    }

    #[test]
    fn a_name_comes_from_the_host_and_is_made_unique() {
        let taken = vec!["build".to_string(), "Build-2".to_string()];
        assert_eq!(suggest_name("me@box.example", &[]).unwrap(), "box.example");
        assert_eq!(suggest_name("build", &taken).unwrap(), "build-3");
        assert_eq!(suggest_name("my host!", &[]).unwrap(), "my-host-");
        assert_eq!(suggest_name("10.0.0.5", &[]).unwrap(), "10.0.0.5");
        assert_eq!(suggest_name("1234", &[]).unwrap(), "host-1234");
        assert_eq!(suggest_name("local", &[]).unwrap(), "host-local");
        assert_eq!(suggest_name("---", &[]), None);
        assert_eq!(suggest_name("me@", &[]), None);
    }

    #[test]
    fn a_long_host_is_cut_to_the_most_a_name_may_have_even_with_a_number() {
        let long = "h".repeat(60);
        let name = suggest_name(&long, &[]).unwrap();
        assert_eq!(name.len(), MACHINE_NAME_MAX);
        let again = suggest_name(&long, &[name]).unwrap();
        assert_eq!(again.len(), MACHINE_NAME_MAX);
        assert!(again.ends_with("-2"));
    }

    #[test]
    fn the_loader_s_words_say_why_a_machine_is_refused() {
        let existing = vec![MachineDef {
            name: "box".into(),
            target: "box".into(),
            ..MachineDef::default()
        }];
        assert!(refusal(&existing, "other", "ok").is_none());
        assert!(
            refusal(&existing, "Box", "ok")
                .unwrap()
                .contains("already named")
        );
        assert!(
            refusal(&existing, "x", "-oProxyCommand=x")
                .unwrap()
                .contains("starts with `-`")
        );
    }
}
