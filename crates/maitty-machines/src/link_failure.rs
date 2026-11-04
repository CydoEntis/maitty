//! Why a connection to another machine failed, told from ssh's exit status and from whether
//! the far side ever spoke, not from ssh's wording: its words change with version and locale.
//! `classify` is the only place that reads ssh's text, and only for the three tokens that mark a
//! login ssh could not finish without a person.
//!
//! The fix text of each class is one constant, so the screen, the tests and the docs agree.

use crate::{NO_DAEMON_EXIT, other_version};

/// Exit status ssh gives when it could not connect or log in, and its own errors.
const SSH_ERROR: i32 = 255;
/// Exit status a POSIX shell gives when the command is not on the PATH.
const COMMAND_NOT_FOUND: i32 = 127;

const LOGIN_TOKENS: [&str; 3] = [
    "Permission denied",
    "Host key verification failed",
    "Too many authentication failures",
];

pub const FIX_LOGIN: &str = "ssh needs a login it cannot do without you; make plain `ssh` to that machine work in a terminal (load your key, accept its host key), then retry";
/// `<name>` is the machine's name in the config; a fix text is one constant for every machine.
pub const FIX_UPDATE: &str = "run `maitty machine update <name>`, then retry";
pub const FIX_INSTALL: &str =
    "run `maitty machine update <name> --install` (or set `remote_cmd` to its path), then retry";
pub const FIX_NO_DAEMON: &str = "maitty is not running on that machine; retry to start it there";

/// Longest reason kept: ssh's last line is shown on screen.
const MOST_WHY_CHARS: usize = 200;

/// What ssh ended with, carried on the error `open_to` returns.
#[derive(Debug)]
pub struct SshEnd {
    /// ssh's exit status; `None` while it was still running, or when a signal ended it.
    pub code: Option<i32>,
    /// Everything ssh wrote to stderr.
    pub said: String,
    /// The far side sent its stream marker or a frame before the end.
    pub streamed: bool,
    /// The text shown for the failure.
    pub message: String,
    /// The machine is reached through its own `via` program, not ssh: ssh's words and its
    /// exit status 255 mean nothing from it.
    pub custom: bool,
}

impl std::fmt::Display for SshEnd {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for SshEnd {}

/// What to do about a failed connection.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailureClass {
    /// ssh could not log in without a prompt.
    Login,
    /// The far side runs another version of the app.
    Update,
    /// The far side has no app on its PATH.
    Install,
    /// The app is installed there but its daemon is not running.
    NoDaemon,
    /// The machine did not answer, with why.
    Unreachable(String),
}

pub fn classify(error: &anyhow::Error) -> FailureClass {
    if other_version(error).is_some() {
        return FailureClass::Update;
    }
    let Some(end) = error.downcast_ref::<SshEnd>() else {
        return FailureClass::Unreachable(clean(&format!("{error:#}")));
    };
    match end.code {
        Some(COMMAND_NOT_FOUND) => FailureClass::Install,
        Some(NO_DAEMON_EXIT) => FailureClass::NoDaemon,
        Some(SSH_ERROR) if !end.custom && !end.streamed && says_login_failed(&end.said) => {
            FailureClass::Login
        }
        _ => FailureClass::Unreachable(why_unreachable(end)),
    }
}

fn says_login_failed(said: &str) -> bool {
    LOGIN_TOKENS.iter().any(|token| said.contains(token))
}

fn why_unreachable(end: &SshEnd) -> String {
    end.said
        .lines()
        .rev()
        .map(clean)
        .find(|line| !line.is_empty())
        .unwrap_or_else(|| clean(&end.message))
}

/// Text from another machine, made safe to put on screen: no control characters (so no escape
/// sequence), trimmed and cut short.
pub(crate) fn clean(text: &str) -> String {
    let plain: String = text.chars().filter(|c| !c.is_control()).collect();
    plain.trim().chars().take(MOST_WHY_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::OtherVersion;

    fn ssh_ended(code: Option<i32>, said: &str, streamed: bool) -> anyhow::Error {
        anyhow::anyhow!("closed").context(SshEnd {
            code,
            said: said.into(),
            streamed,
            message: format!("ssh box: {said}"),
            custom: false,
        })
    }

    fn class(code: Option<i32>, said: &str) -> FailureClass {
        classify(&ssh_ended(code, said, false))
    }

    fn is_unreachable(class: &FailureClass) -> bool {
        matches!(class, FailureClass::Unreachable(_))
    }

    #[test]
    fn a_login_ssh_could_not_finish_is_needs_login() {
        for said in [
            "box: Permission denied (publickey).",
            "Host key verification failed.",
            "Received disconnect: Too many authentication failures",
        ] {
            assert_eq!(class(Some(255), said), FailureClass::Login, "{said}");
        }
    }

    #[test]
    fn the_login_words_alone_do_not_decide_it() {
        assert!(is_unreachable(&classify(&ssh_ended(
            Some(255),
            "Permission denied",
            true
        ))));
        assert!(is_unreachable(&class(Some(1), "Permission denied")));
        assert!(is_unreachable(&class(None, "Permission denied")));
    }

    #[test]
    fn a_custom_transport_is_never_needs_login_whatever_it_prints() {
        let end = |code, said: &str| {
            anyhow::anyhow!("closed").context(SshEnd {
                code,
                said: said.into(),
                streamed: false,
                message: format!("box: {said}"),
                custom: true,
            })
        };
        let denied = "Permission denied (publickey).";
        assert_eq!(
            classify(&end(Some(255), denied)),
            FailureClass::Unreachable(denied.into())
        );
        assert_eq!(classify(&end(Some(127), "no such")), FailureClass::Install);
        assert_eq!(
            classify(&end(Some(1), "relay: down")),
            FailureClass::Unreachable("relay: down".into())
        );
        assert_eq!(
            classify(&end(Some(NO_DAEMON_EXIT), "")),
            FailureClass::NoDaemon
        );
    }

    #[test]
    fn another_version_needs_an_update() {
        let error = anyhow::Error::from(OtherVersion { daemon: 3 });
        assert_eq!(classify(&error), FailureClass::Update);
    }

    #[test]
    fn exit_127_means_the_app_is_not_installed_whatever_was_said() {
        assert_eq!(
            class(Some(127), "bash: maitty: command not found"),
            FailureClass::Install
        );
        assert_eq!(
            class(Some(127), "auf Deutsch: Befehl nicht gefunden"),
            FailureClass::Install
        );
    }

    #[test]
    fn the_install_and_update_fixes_name_the_command_that_does_it() {
        assert!(FIX_INSTALL.contains("maitty machine update <name> --install"));
        assert!(FIX_UPDATE.contains("maitty machine update <name>"));
        assert!(!FIX_UPDATE.contains("by hand"));
    }

    #[test]
    fn the_proxy_s_exit_code_means_no_daemon_is_running() {
        assert_eq!(class(Some(NO_DAEMON_EXIT), ""), FailureClass::NoDaemon);
    }

    #[test]
    fn anything_else_is_unreachable_with_ssh_s_last_line() {
        let said = "warning: something\nssh: connect to host box port 22: Connection refused\n";
        assert_eq!(
            class(Some(255), said),
            FailureClass::Unreachable(
                "ssh: connect to host box port 22: Connection refused".into()
            )
        );
    }

    #[test]
    fn what_ssh_said_is_cleaned_before_it_can_be_shown() {
        let said = "ssh: \u{1b}[31mred\u{1b}[0m \u{7}bell\r\n\u{1b}]0;title\u{7}\n";
        let FailureClass::Unreachable(why) = class(Some(255), said) else {
            panic!("not unreachable");
        };
        assert!(!why.chars().any(char::is_control), "{why:?}");
        assert_eq!(why, "]0;title");
        let long = "x".repeat(10 * MOST_WHY_CHARS);
        let FailureClass::Unreachable(why) = class(Some(255), &long) else {
            panic!("not unreachable");
        };
        assert_eq!(why.chars().count(), MOST_WHY_CHARS);
    }

    #[test]
    fn with_nothing_on_stderr_the_failure_says_so_itself() {
        let error = anyhow::anyhow!("closed").context(SshEnd {
            code: Some(255),
            said: String::new(),
            streamed: false,
            message: "couldn't reach maitty on box over ssh".into(),
            custom: false,
        });
        assert_eq!(
            classify(&error),
            FailureClass::Unreachable("couldn't reach maitty on box over ssh".into())
        );
    }

    #[test]
    fn an_error_that_is_not_from_ssh_is_unreachable_with_its_words() {
        let error = anyhow::anyhow!("running ssh (is OpenSSH installed?)");
        assert_eq!(
            classify(&error),
            FailureClass::Unreachable("running ssh (is OpenSSH installed?)".into())
        );
    }
}
