//! A machine of the local config (`[[machines]]`) and the checks that refuse a bad entry.

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Longest machine name; it is typed on the command line and shown in the sidebar.
pub const MACHINE_NAME_MAX: usize = 32;

/// The name of this machine in `machine/id` addresses; no `[[machines]]` entry may take it.
pub const LOCAL_MACHINE: &str = "local";

/// Another machine, reached over ssh (`[[machines]]`). Read only from the local config, never
/// the synced one: where it points is this computer's business.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct MachineDef {
    /// How the machine is addressed (`--on name`, `name/3`).
    pub name: String,
    /// The ssh destination: a host, `user@host`, or a `Host` alias from ssh's own config.
    pub target: String,
    /// Extra arguments for ssh, one list entry each (`["-p", "2222"]`); never split on spaces.
    pub ssh_args: Vec<String>,
    /// The app's command on the far side; unset, the app's own name from the remote PATH.
    pub remote_cmd: Option<String>,
    /// A command that carries stdin and stdout unchanged in place of ssh (`["fly", "ssh",
    /// "console", "-C", "{command}"]`): a program and its arguments, never a shell line. The
    /// element `{command}` is replaced by the remote command; without one it is appended last.
    /// It is code this app runs, so it is read only from the local config.
    pub via: Option<Vec<String>>,
    /// Let this machine's panes write to this computer's clipboard. On unless the entry says
    /// `clipboard = false`: a machine you added is one you chose, and a copy made there should
    /// reach the paste here without any setting.
    pub clipboard: bool,
    /// Let this machine raise notices.
    pub notices: bool,
    /// Let this machine play sounds.
    pub sounds: bool,
    /// Let an agent in a pane here reach this machine through its MCP tools (`machine` argument
    /// or a `name/id`), within that pane's own grants.
    pub mcp: bool,
}

impl Default for MachineDef {
    fn default() -> MachineDef {
        MachineDef {
            name: String::new(),
            target: String::new(),
            ssh_args: Vec::new(),
            remote_cmd: None,
            via: None,
            clipboard: true,
            notices: false,
            sounds: false,
            mcp: false,
        }
    }
}

/// Refuse a `[[machines]]` entry that could not name or reach a machine, so the mistake shows
/// when the config loads. Each message names the machine and what to change.
pub fn validate_machines(machines: &[MachineDef]) -> Result<()> {
    validate_machines_on(machines, cfg!(windows))
}

/// `validate_machines` for a given platform, so the Windows refusal of `via` is testable anywhere.
pub fn validate_machines_on(machines: &[MachineDef], windows: bool) -> Result<()> {
    for (i, m) in machines.iter().enumerate() {
        let n = i + 1;
        validate_machine_name(n, &m.name)?;
        anyhow::ensure!(
            !machines[..i]
                .iter()
                .any(|o| o.name.eq_ignore_ascii_case(&m.name)),
            "[[machines]] #{n}: another machine is already named `{}`; names differ by more than case",
            m.name
        );
        validate_machine_route(&m.name, m)?;
        validate_machine_via(&m.name, m, windows)?;
    }
    Ok(())
}

/// The element of `via` that is replaced by the remote command.
pub const VIA_PLACEHOLDER: &str = "{command}";

/// Why a `via` machine is refused on Windows, at load and at connect.
pub const VIA_NOT_ON_WINDOWS: &str =
    "a custom transport (`via`) is not supported on Windows in v1; use ssh";

fn validate_machine_via(name: &str, m: &MachineDef, windows: bool) -> Result<()> {
    let Some(via) = &m.via else { return Ok(()) };
    anyhow::ensure!(!windows, "[[machines]] `{name}`: {VIA_NOT_ON_WINDOWS}");
    anyhow::ensure!(
        m.ssh_args.is_empty(),
        "[[machines]] `{name}`: `ssh_args` has no effect with `via` (it is not ssh); remove one of them"
    );
    let Some(program) = via.first() else {
        anyhow::bail!(
            "[[machines]] `{name}`: `via` is empty; give the program and its arguments, or remove the key"
        );
    };
    anyhow::ensure!(
        !program.is_empty() && !program.starts_with('-') && program != VIA_PLACEHOLDER,
        "[[machines]] `{name}`: the first `via` entry is the program to run (got {program:?})"
    );
    for word in via {
        anyhow::ensure!(
            !word.chars().any(char::is_control),
            "[[machines]] `{name}`: each `via` entry is one argument without control characters (got {word:?})"
        );
        anyhow::ensure!(
            !word.contains(VIA_PLACEHOLDER) || word == VIA_PLACEHOLDER,
            "[[machines]] `{name}`: `{VIA_PLACEHOLDER}` must be an entry of its own in `via` (got {word:?})"
        );
    }
    anyhow::ensure!(
        via.iter().filter(|w| *w == VIA_PLACEHOLDER).count() <= 1,
        "[[machines]] `{name}`: `via` has more than one `{VIA_PLACEHOLDER}`"
    );
    Ok(())
}

fn validate_machine_name(n: usize, name: &str) -> Result<()> {
    anyhow::ensure!(!name.is_empty(), "[[machines]] #{n} needs a `name`");
    anyhow::ensure!(
        !name.eq_ignore_ascii_case(LOCAL_MACHINE),
        "[[machines]] #{n}: `{name}` is reserved for this computer; choose another `name`"
    );
    anyhow::ensure!(
        !name.chars().all(|c| c.is_ascii_digit()),
        "[[machines]] `{name}`: a name of only digits reads as a pane id; add a letter to `name`"
    );
    let starts_ok = name.starts_with(|c: char| c.is_ascii_alphanumeric());
    let chars_ok = name
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.'));
    anyhow::ensure!(
        starts_ok && chars_ok && name.len() <= MACHINE_NAME_MAX,
        "[[machines]] `{name}`: `name` is up to {MACHINE_NAME_MAX} letters, digits, `-`, `_` or `.`, starting with a letter or digit"
    );
    Ok(())
}

fn validate_machine_route(name: &str, m: &MachineDef) -> Result<()> {
    anyhow::ensure!(
        !m.target.is_empty(),
        "[[machines]] `{name}` needs a `target`"
    );
    anyhow::ensure!(
        !m.target.starts_with('-'),
        "[[machines]] `{name}`: `target` {:?} starts with `-`, which ssh would read as an option; give a host",
        m.target
    );
    anyhow::ensure!(
        !m.target
            .chars()
            .any(|c| c.is_whitespace() || c.is_control()),
        "[[machines]] `{name}`: `target` {:?} has whitespace or a control character; give a host, or put options in `ssh_args`",
        m.target
    );
    for arg in &m.ssh_args {
        anyhow::ensure!(
            !arg.is_empty() && !arg.chars().any(char::is_control),
            "[[machines]] `{name}`: each `ssh_args` entry is one non-empty argument without control characters (got {arg:?})"
        );
    }
    if let Some(cmd) = &m.remote_cmd {
        anyhow::ensure!(
            !cmd.trim().is_empty() && !cmd.chars().any(char::is_control),
            "[[machines]] `{name}`: `remote_cmd` is empty or has a control character; give the command, or remove the key"
        );
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{MachineDef, validate_machines, validate_machines_on};

    fn machine(name: &str, target: &str) -> MachineDef {
        MachineDef {
            name: name.into(),
            target: target.into(),
            ..MachineDef::default()
        }
    }

    fn refusal(machines: &[MachineDef]) -> String {
        validate_machines(machines).unwrap_err().to_string()
    }

    #[test]
    fn a_machine_reads_with_its_defaults_clipboard_on_and_the_other_opt_ins_off() {
        let m: MachineDef = toml::from_str("name = 'box'\ntarget = 'me@box'").unwrap();
        assert_eq!(m, machine("box", "me@box"));
        assert!(m.clipboard, "a copy made there reaches the paste here");
        assert!(!(m.notices || m.sounds) && m.ssh_args.is_empty());
        let off: MachineDef =
            toml::from_str("name = 'box'\ntarget = 'me@box'\nclipboard = false").unwrap();
        assert!(!off.clipboard, "and an entry can turn it off");
        validate_machines(&[m]).unwrap();
    }

    #[test]
    fn an_agent_may_reach_a_machine_only_when_it_says_mcp() {
        let plain: MachineDef = toml::from_str("name = 'box'\ntarget = 'me@box'").unwrap();
        assert!(!plain.mcp);
        let opted: MachineDef =
            toml::from_str("name = 'box'\ntarget = 'me@box'\nmcp = true").unwrap();
        assert!(opted.mcp);
        validate_machines(&[opted]).unwrap();
    }

    #[test]
    fn a_machine_keeps_ssh_args_as_a_list() {
        let m: MachineDef =
            toml::from_str("name = 'b'\ntarget = 'h'\nssh_args = ['-p', '2222', '-i', 'my key']")
                .unwrap();
        assert_eq!(m.ssh_args, ["-p", "2222", "-i", "my key"]);
        validate_machines(&[m]).unwrap();
    }

    #[test]
    fn two_machines_may_not_share_a_name_in_any_case() {
        let err = refusal(&[machine("box", "a"), machine("BOX", "b")]);
        assert!(err.contains("#2") && err.contains("`BOX`"), "{err}");
    }

    #[test]
    fn local_is_reserved_in_any_case() {
        for name in ["local", "LOCAL", "Local"] {
            let err = refusal(&[machine(name, "h")]);
            assert!(err.contains("reserved"), "{err}");
        }
    }

    #[test]
    fn a_name_needs_the_pattern() {
        let too_long = "a".repeat(33);
        for name in [
            "", "-box", ".box", "my box", "bo/x", "bo:x", "\u{e9}", &too_long,
        ] {
            assert!(
                validate_machines(&[machine(name, "h")]).is_err(),
                "{name:?}"
            );
        }
        let longest = "a".repeat(32);
        for name in ["a", "box-1", "web_2.prod", &longest] {
            validate_machines(&[machine(name, "h")]).unwrap();
        }
    }

    #[test]
    fn a_name_that_looks_like_a_pane_id_is_refused() {
        let err = refusal(&[machine("12", "h")]);
        assert!(err.contains("`12`") && err.contains("pane id"), "{err}");
        validate_machines(&[machine("12b", "h")]).unwrap();
    }

    #[test]
    fn a_target_is_required() {
        let err = refusal(&[machine("box", "")]);
        assert!(err.contains("`box`") && err.contains("`target`"), "{err}");
    }

    #[test]
    fn a_target_starting_with_a_dash_is_refused() {
        for target in ["-oProxyCommand=x", "-p", "--"] {
            let err = refusal(&[machine("box", target)]);
            assert!(err.contains("starts with `-`"), "{err}");
        }
    }

    #[test]
    fn a_target_with_whitespace_or_a_control_character_is_refused() {
        for target in ["a b", "a\tb", "host\n", "a\u{7}b", "a\u{0}b", "a\u{a0}b"] {
            let err = refusal(&[machine("box", target)]);
            assert!(
                err.contains("whitespace or a control character"),
                "{target:?}: {err}"
            );
        }
    }

    #[test]
    fn an_empty_ssh_arg_is_refused() {
        let mut m = machine("box", "h");
        m.ssh_args = vec!["-p".into(), String::new()];
        assert!(refusal(&[m]).contains("`ssh_args`"));
    }

    #[test]
    fn an_ssh_arg_with_a_control_character_is_refused() {
        let mut m = machine("box", "h");
        m.ssh_args = vec!["-o\nProxyCommand=x".into()];
        assert!(refusal(&[m]).contains("`ssh_args`"));
    }

    fn via_machine(via: &[&str]) -> MachineDef {
        let mut m = machine("box", "box");
        m.via = Some(via.iter().map(|w| w.to_string()).collect());
        m
    }

    fn via_refusal(via: &[&str]) -> String {
        validate_machines_on(&[via_machine(via)], false)
            .unwrap_err()
            .to_string()
    }

    #[test]
    fn a_via_is_a_list_of_arguments_and_off_unless_set() {
        let m: MachineDef = toml::from_str(
            "name = 'box'\ntarget = 'box'\nvia = ['fly', 'ssh', 'console', '-C', '{command}']",
        )
        .unwrap();
        assert_eq!(
            m.via.as_deref(),
            Some(&["fly", "ssh", "console", "-C", "{command}"].map(String::from)[..])
        );
        validate_machines_on(&[m], false).unwrap();
        let plain: MachineDef = toml::from_str("name = 'box'\ntarget = 'box'").unwrap();
        assert_eq!(plain.via, None);
    }

    #[test]
    fn a_via_with_a_program_and_one_placeholder_or_none_is_accepted() {
        for via in [
            &["relay"][..],
            &["relay", "--to", "box", "{command}"],
            &["relay", "{command}", "-x"],
        ] {
            validate_machines_on(&[via_machine(via)], false).unwrap();
        }
    }

    #[test]
    fn a_via_that_is_empty_or_has_no_program_is_refused() {
        assert!(via_refusal(&[]).contains("`via` is empty"));
        assert!(via_refusal(&[""]).contains("program"));
        assert!(via_refusal(&["-oProxyCommand=x"]).contains("program"));
    }

    #[test]
    fn a_via_entry_with_a_control_character_is_refused() {
        for via in [&["relay\n"][..], &["relay", "a\u{7}b"], &["relay", "x\ty"]] {
            assert!(via_refusal(via).contains("control characters"), "{via:?}");
        }
    }

    #[test]
    fn the_placeholder_is_one_entry_of_its_own_and_at_most_once() {
        assert!(via_refusal(&["relay", "{command}", "{command}"]).contains("more than one"));
        for via in [&["relay", "x{command}"][..], &["relay", "--cmd={command}"]] {
            assert!(via_refusal(via).contains("entry of its own"), "{via:?}");
        }
    }

    #[test]
    fn ssh_args_beside_a_via_are_refused_not_ignored() {
        let mut m = via_machine(&["relay"]);
        m.ssh_args = vec!["-p".into(), "22".into()];
        let err = validate_machines_on(&[m], false).unwrap_err().to_string();
        assert!(
            err.contains("`ssh_args`") && err.contains("remove one"),
            "{err}"
        );
    }

    #[test]
    fn a_via_is_refused_on_windows_whatever_it_says() {
        let err = validate_machines_on(&[via_machine(&["relay", "{command}"])], true)
            .unwrap_err()
            .to_string();
        assert!(
            err.contains("`box`")
                && err.contains("not supported on Windows in v1")
                && err.contains("use ssh"),
            "{err}"
        );
        validate_machines_on(&[machine("box", "box")], true).unwrap();
    }

    #[test]
    fn a_remote_cmd_must_be_text_without_control_characters() {
        for cmd in ["", "  ", "maitty\nrm -rf ~"] {
            let mut m = machine("box", "h");
            m.remote_cmd = Some(cmd.into());
            assert!(refusal(&[m]).contains("`remote_cmd`"), "{cmd:?}");
        }
        let mut ok = machine("box", "h");
        ok.remote_cmd = Some("~/.cargo/bin/maitty".into());
        validate_machines(&[ok]).unwrap();
    }
}
