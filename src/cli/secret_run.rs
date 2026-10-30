//! `<app> secret run <reference> --env NAME -- <command>`: run one command with a secret in its
//! environment (ADR-0036). The reference came from `<app>_request_secret`; it works once, so the
//! value reaches nothing but this command: not the chat, not a file, not our output.

use super::{block_on, request};
use crate::protocol::{ClientMsg, Command, Reply};
use anyhow::{Result, bail};

/// A variable name every shell accepts: letters, digits and underscores, not starting with a digit.
fn valid_variable(name: &str) -> bool {
    let mut chars = name.chars();
    chars.next().is_some_and(|c| c.is_ascii_alphabetic() || c == '_') && chars.all(|c| c.is_ascii_alphanumeric() || c == '_')
}

/// Spend `reference`, run `cmd` with the value in `env_name`, and return its exit code.
pub fn run(reference: &str, env_name: &str, cmd: &[String]) -> Result<i32> {
    if !valid_variable(env_name) {
        bail!("{env_name:?} isn't a variable name: use letters, digits and underscores, e.g. OPENAI_API_KEY");
    }
    let Some((program, args)) = cmd.split_first() else { bail!("say what to run after --") };
    // Spent before the command starts: a reference that has been used must not run anything twice.
    let value = match block_on(request(ClientMsg::Command(Command::SpendSecret { reference: reference.to_string() })))? {
        Reply::Text(v) => v,
        _ => bail!("unexpected reply"),
    };
    let status = std::process::Command::new(program).args(args).env(env_name, value).status();
    match status {
        Ok(s) => Ok(s.code().unwrap_or(1)),
        Err(e) => bail!("couldn't run {program}: {e}. The reference is used up; ask for the secret again"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_names_a_shell_accepts_are_variables() {
        for ok in ["OPENAI_API_KEY", "_x", "a1"] {
            assert!(valid_variable(ok), "{ok}");
        }
        for bad in ["", "1A", "A-B", "A B", "A=B", "PATH;ls"] {
            assert!(!valid_variable(bad), "{bad:?}");
        }
    }

    #[test]
    fn a_bad_name_or_no_command_runs_nothing_and_spends_nothing() {
        let cmd = vec!["echo".to_string()];
        assert!(run("secret-ref:00", "1BAD", &cmd).unwrap_err().to_string().contains("isn't a variable name"));
        assert!(run("secret-ref:00", "KEY", &[]).unwrap_err().to_string().contains("after --"));
    }
}
