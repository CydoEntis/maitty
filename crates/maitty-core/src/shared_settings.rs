//! The settings both apps share: read off the loaded config, and changed in config.toml.
//!
//! The terminal app writes the same file through `config::edit_config`; this is the daemon's
//! typed route to it for the desktop app (`Command::SetSetting`). The value is checked before
//! the file is opened, so a refused change leaves it untouched.

use crate::config::{Config, NEW_AGENTS, edit_config_at, validate_spinner};
use crate::settings_view::{SETTING_TEXT_MAX_BYTES, SettingKey, SettingValue, SharedSettings};
use anyhow::{Context, Result};
use std::path::Path;

/// The longest theme name; the built-in ones are well under it.
const THEME_NAME_MAX_BYTES: usize = 64;

/// The shared settings as the loaded config has them.
pub fn read(cfg: &Config) -> SharedSettings {
    SharedSettings {
        default_agent: cfg.sessions.agent.clone(),
        new_session_view: cfg.ui.new_session_view,
        sidebar_side: if cfg.ui.sidebar_position == "right" {
            crate::settings_view::SidebarSide::Right
        } else {
            crate::settings_view::SidebarSide::Left
        },
        theme_dark: cfg.theme_dark.clone(),
        theme_light: cfg.theme_light.clone(),
        appearance: if cfg.appearance == "light" {
            crate::settings_view::Appearance::Light
        } else {
            crate::settings_view::Appearance::Dark
        },
        spinner: cfg.ui.spinner.clone(),
        start_dir: cfg.ui.start_dir.clone(),
        reduce_motion: !cfg.ui.motion,
        window_style: cfg.ui.window_style,
    }
}

/// Where one setting lives in config.toml, and what to write there.
struct Edit {
    path: &'static [&'static str],
    value: toml_edit::Value,
    /// Other spellings of the key that are read, and would clash with the one written.
    aliases: &'static [&'static str],
}

fn edit(path: &'static [&'static str], value: impl Into<toml_edit::Value>) -> Edit {
    Edit {
        path,
        value: value.into(),
        aliases: &[],
    }
}

fn theme_name(name: &str) -> Result<&str> {
    anyhow::ensure!(
        !name.is_empty()
            && name.len() <= THEME_NAME_MAX_BYTES
            && name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.')),
        "a theme name is 1 to {THEME_NAME_MAX_BYTES} letters, digits, dashes, dots or underscores"
    );
    Ok(name)
}

/// Check `value` for `key` and say what to write. Errors name the setting and what it takes.
fn plan(key: SettingKey, value: SettingValue) -> Result<Edit> {
    use SettingValue as V;
    Ok(match (key, value) {
        (SettingKey::DefaultAgent, V::Text(agent)) => {
            anyhow::ensure!(
                NEW_AGENTS.contains(&agent.as_str()),
                "default agent is \"{agent}\"; use one of: {}",
                NEW_AGENTS.join(", ")
            );
            edit(&["sessions", "agent"], agent)
        }
        (SettingKey::NewSessionView, V::NewSessionView(view)) => {
            edit(&["ui", "new_session_view"], view.as_str())
        }
        (SettingKey::SidebarSide, V::SidebarSide(side)) => Edit {
            aliases: &["sidebar_side"],
            ..edit(&["ui", "sidebar_position"], side.as_str())
        },
        (SettingKey::ThemeDark, V::Text(name)) => edit(&["theme_dark"], theme_name(&name)?),
        (SettingKey::ThemeLight, V::Text(name)) => edit(&["theme_light"], theme_name(&name)?),
        (SettingKey::Appearance, V::Appearance(mode)) => edit(&["appearance"], mode.as_str()),
        (SettingKey::Spinner, V::List(frames)) => {
            validate_spinner(&frames)?;
            edit(
                &["ui", "spinner"],
                frames
                    .iter()
                    .map(String::as_str)
                    .collect::<toml_edit::Array>(),
            )
        }
        (SettingKey::StartDir, V::Text(dir)) => {
            anyhow::ensure!(
                dir.len() <= SETTING_TEXT_MAX_BYTES && !dir.chars().any(char::is_control),
                "the start folder is at most {SETTING_TEXT_MAX_BYTES} bytes with no control characters"
            );
            edit(&["ui", "start_dir"], dir)
        }
        (SettingKey::ReduceMotion, V::Bool(reduce)) => Edit {
            aliases: &["reduce_motion"],
            ..edit(&["ui", "motion"], !reduce)
        },
        (SettingKey::WindowStyle, V::WindowStyle(style)) => {
            edit(&["ui", "window_style"], style.as_str())
        }
        (key, value) => anyhow::bail!("{key:?} does not take {value:?}"),
    })
}

/// Change one shared setting in `file`, keeping its comments and every other key. A missing
/// file is created.
pub fn apply_to(file: &Path, key: SettingKey, value: SettingValue) -> Result<()> {
    let change = plan(key, value)?;
    edit_config_at(file, |doc| {
        let (leaf, tables) = change.path.split_last().context("empty setting path")?;
        let mut table = doc.as_table_mut();
        for part in tables {
            if !table.contains_key(part) {
                let mut new = toml_edit::Table::new();
                new.set_implicit(true);
                table.insert(part, toml_edit::Item::Table(new));
            }
            table = table[*part]
                .as_table_mut()
                .with_context(|| format!("`{part}` in config.toml is not a table"))?;
        }
        for alias in change.aliases {
            table.remove(alias);
        }
        let mut value = change.value;
        if let Some(old) = table.get_mut(leaf).and_then(toml_edit::Item::as_value_mut) {
            // Replacing the whole entry discards comments attached to its key and value.
            *value.decor_mut() = old.decor().clone();
            *old = value;
        } else {
            table.insert(leaf, toml_edit::value(value));
        }
        Ok(())
    })
}

/// [`apply_to`] on the user's config.toml.
pub fn apply(key: SettingKey, value: SettingValue) -> Result<()> {
    apply_to(&crate::config::config_path(), key, value)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings_view::{Appearance, NewSessionView, SidebarSide, WindowStyle};

    const FIXTURE: &str = "# my config\ntheme = \"night\"\n\n[ui]\n# where the sidebar sits\nsidebar_position = \"left\" # keep\nsplash = true\nmotion = true\n\n[notify]\ndesktop = true\n";

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-shared-settings-{}-{}"),
            name,
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn load(file: &Path) -> Config {
        Config::load_at(file, false).unwrap()
    }

    #[test]
    fn a_config_from_before_the_new_keys_reads_their_defaults() {
        let dir = dir("old");
        let file = dir.join("config.toml");
        std::fs::write(&file, FIXTURE).unwrap();
        let settings = read(&load(&file));
        assert_eq!(settings.new_session_view, NewSessionView::Chat);
        assert_eq!(settings.window_style, WindowStyle::Flat);
        assert_eq!(settings.sidebar_side, SidebarSide::Left);
        assert!(!settings.reduce_motion);
        assert_eq!(settings.appearance, Appearance::Dark);
        assert_eq!(read(&Config::default()).default_agent, "last used");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn the_spec_names_are_read_as_aliases() {
        let dir = dir("alias");
        let file = dir.join("config.toml");
        std::fs::write(
            &file,
            "[ui]\nsidebar_side = \"right\"\nreduce_motion = true\nnew_session_view = \"terminal\"\nwindow_style = \"glass\"\n",
        )
        .unwrap();
        let settings = read(&load(&file));
        assert_eq!(settings.sidebar_side, SidebarSide::Right);
        assert!(settings.reduce_motion);
        assert_eq!(settings.new_session_view, NewSessionView::Terminal);
        assert_eq!(settings.window_style, WindowStyle::Glass);
        // The canonical name wins over the inverse spelling when a file has both.
        std::fs::write(&file, "[ui]\nmotion = true\nreduce_motion = true\n").unwrap();
        assert!(!read(&load(&file)).reduce_motion);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn setting_one_key_changes_only_that_line() {
        let dir = dir("one");
        let file = dir.join("config.toml");
        std::fs::write(&file, FIXTURE).unwrap();
        apply_to(
            &file,
            SettingKey::SidebarSide,
            SettingValue::SidebarSide(SidebarSide::Right),
        )
        .unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        assert_eq!(
            text,
            FIXTURE.replace(
                "sidebar_position = \"left\"",
                "sidebar_position = \"right\""
            )
        );
        assert_eq!(read(&load(&file)).sidebar_side, SidebarSide::Right);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_new_key_is_added_and_a_missing_file_is_created() {
        let dir = dir("new");
        let file = dir.join("nested").join("config.toml");
        apply_to(
            &file,
            SettingKey::WindowStyle,
            SettingValue::WindowStyle(WindowStyle::Glass),
        )
        .unwrap();
        apply_to(&file, SettingKey::ReduceMotion, SettingValue::Bool(true)).unwrap();
        let settings = read(&load(&file));
        assert_eq!(settings.window_style, WindowStyle::Glass);
        assert!(settings.reduce_motion);
        assert!(
            std::fs::read_to_string(&file)
                .unwrap()
                .contains("motion = false")
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn writing_a_canonical_key_drops_its_alias_so_the_file_still_loads() {
        let dir = dir("drop");
        let file = dir.join("config.toml");
        std::fs::write(
            &file,
            "[ui]\nsidebar_side = \"right\"\nreduce_motion = true\n",
        )
        .unwrap();
        apply_to(
            &file,
            SettingKey::SidebarSide,
            SettingValue::SidebarSide(SidebarSide::Left),
        )
        .unwrap();
        apply_to(&file, SettingKey::ReduceMotion, SettingValue::Bool(false)).unwrap();
        let settings = read(&load(&file));
        assert_eq!(settings.sidebar_side, SidebarSide::Left);
        assert!(!settings.reduce_motion);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_crlf_file_is_edited_without_losing_a_line() {
        let dir = dir("crlf");
        let file = dir.join("config.toml");
        let crlf = FIXTURE.replace('\n', "\r\n");
        std::fs::write(&file, &crlf).unwrap();
        apply_to(
            &file,
            SettingKey::StartDir,
            SettingValue::Text("C:\\code".into()),
        )
        .unwrap();
        let text = std::fs::read_to_string(&file).unwrap();
        // toml_edit writes LF (as the terminal app's own edits already do); no line is lost
        // and the new one is the only addition.
        assert_eq!(
            text.replace("start_dir = 'C:\\code'\n", ""),
            FIXTURE,
            "{text:?}"
        );
        assert_eq!(read(&load(&file)).start_dir, "C:\\code");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_refused_value_leaves_the_file_untouched() {
        let dir = dir("bad");
        let file = dir.join("config.toml");
        std::fs::write(&file, FIXTURE).unwrap();
        let too_many = vec!["a".to_string(); 65];
        let refused = [
            (
                SettingKey::DefaultAgent,
                SettingValue::Text("rm -rf".into()),
            ),
            (SettingKey::ThemeDark, SettingValue::Text("../evil".into())),
            (SettingKey::ThemeLight, SettingValue::Text(String::new())),
            (SettingKey::Spinner, SettingValue::List(too_many)),
            (
                SettingKey::Spinner,
                SettingValue::List(vec!["\u{1b}".into()]),
            ),
            (SettingKey::StartDir, SettingValue::Text("a\nb".into())),
            (SettingKey::StartDir, SettingValue::Text("x".repeat(5000))),
            // The wrong kind of value for the key.
            (SettingKey::ReduceMotion, SettingValue::Text("yes".into())),
            (
                SettingKey::WindowStyle,
                SettingValue::SidebarSide(SidebarSide::Left),
            ),
        ];
        for (key, value) in refused {
            assert!(
                apply_to(&file, key, value.clone()).is_err(),
                "{key:?} {value:?}"
            );
            assert_eq!(std::fs::read_to_string(&file).unwrap(), FIXTURE);
        }
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_config_with_a_syntax_error_is_left_alone() {
        let dir = dir("broken");
        let file = dir.join("config.toml");
        std::fs::write(&file, "[ui\n").unwrap();
        assert!(
            apply_to(
                &file,
                SettingKey::Appearance,
                SettingValue::Appearance(Appearance::Light)
            )
            .is_err()
        );
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "[ui\n");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
