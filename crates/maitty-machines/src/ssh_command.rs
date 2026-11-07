//! The one place the `ssh` command line for `<app> proxy` on another machine is built.
//!
//! Both halves are pure functions of strings, so a Windows, Linux or macOS client builds the
//! same remote command. The remote command is read by the remote user's login shell (bash,
//! zsh, fish or sh), never by the client's shell, so it is quoted for a POSIX shell whatever
//! the client runs. The machine is untrusted (ADR 0007): a host or `remote_cmd` must not
//! become an ssh option or extra shell code.

use self::SshCommandError::*;
use crate::VIA_PLACEHOLDER;

pub(crate) const CONNECT_TIMEOUT_SECS: u32 = 15;
const SERVER_ALIVE_INTERVAL_SECS: u32 = 15;
const SERVER_ALIVE_COUNT_MAX: u32 = 3;

/// The subcommand run on the far side.
const PROXY: &str = "proxy";

/// What the probe and the first login run there: it prints the version, and an app that is
/// not installed answers with the shell's "not found" instead.
const VERSION_FLAG: &str = "--version";

/// How long the probe waits to connect: it runs while a person watches a dialog.
pub(crate) const PROBE_TIMEOUT_SECS: u32 = 8;

/// Makes `proxy` fail with `NO_DAEMON_EXIT` instead of starting the daemon it finds missing.
pub(crate) const ATTACH_ONLY_FLAG: &str = "--attach-only";

/// What `proxy` does on the far side when no daemon answers there.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProxyMode {
    /// Start the daemon (`--remote host`, which has always done so).
    StartsDaemon,
    /// Only attach: a configured machine's daemon is the user's to start.
    AttachOnly,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum SshCommandError {
    EmptyTarget,
    /// A leading `-` is how a host becomes an ssh option (`-oProxyCommand=...`).
    TargetIsOption(String),
    TargetNotAHost(String),
    EmptyRemoteCommand,
    /// A `via` with no program to run.
    EmptyTransport,
    RemoteCommandHasControlCharacter,
    /// The first login runs the remote command unquoted, so it must be a plain word.
    RemoteCommandNotPlain(String),
    /// A word of the first login's command that no local shell would read the same way.
    CannotQuote(String),
    /// A pasted file's name must be a plain word that cannot be a path or an option.
    BadPasteName(String),
    /// A path to read on the far side: not empty, not too long, nothing that is not text.
    BadRemotePath(String),
}

impl std::fmt::Display for SshCommandError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EmptyTarget => write!(f, "no host to reach over ssh"),
            TargetIsOption(target) => write!(
                f,
                "{target:?} starts with `-`, which ssh would read as an option; give a host"
            ),
            TargetNotAHost(target) => write!(
                f,
                "{target:?} has whitespace or a control character; give a host, or put options in `ssh_args`"
            ),
            EmptyRemoteCommand => write!(f, "the remote command is empty"),
            EmptyTransport => write!(f, "`via` has no program to run"),
            RemoteCommandHasControlCharacter => write!(
                f,
                "the remote command has a control character or a newline; give one program path"
            ),
            RemoteCommandNotPlain(cmd) => write!(
                f,
                "`remote_cmd` {cmd:?} has characters that need quoting; run ssh to that machine in a terminal once to log in"
            ),
            CannotQuote(word) => write!(
                f,
                "{word:?} cannot be passed safely to a shell; run ssh to that machine in a terminal once to log in"
            ),
            BadRemotePath(path) => write!(
                f,
                "{path:?} is not a path a file can be read from: it must be 1 to {MOST_REMOTE_PATH_BYTES} bytes with no control characters"
            ),
            BadPasteName(name) => write!(
                f,
                "{name:?} is not a name a pasted file may have: letters, digits, `.`, `_` and `-`, not starting with a dot"
            ),
        }
    }
}

impl std::error::Error for SshCommandError {}

/// The full argv: `ssh` is the program then the user's own arguments, `target` the host.
///
/// ssh keeps the first value it sees for an option, so the two that must not be undone
/// (`BatchMode`: no prompts; `ForwardAgent`: a remote is untrusted) come before the user's
/// arguments, and the three that are only defaults come after them. `-a` follows the user's
/// arguments because it, unlike `-o`, overrides an earlier `-A`. `-T` is last for the same
/// reason against `-t`. After `--` the target is a host whatever it looks like.
pub(crate) fn ssh_argv(
    ssh: &[String],
    target: &str,
    remote_cmd: &str,
    mode: ProxyMode,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let remote = remote_command(remote_cmd, mode)?;
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// The argv of a custom transport (`via`): its own program and arguments, with the remote
/// command (`<remote_cmd> proxy [--attach-only]`) as the `{command}` element, else as the last
/// argument. Nothing of ssh is added: it is not ssh, and no shell on this side reads the line.
pub(crate) fn command_argv(
    via: &[String],
    remote_cmd: &str,
    mode: ProxyMode,
) -> Result<Vec<String>, SshCommandError> {
    if via.is_empty() {
        return Err(EmptyTransport);
    }
    let remote = remote_command(remote_cmd, mode)?;
    let mut argv = via.to_vec();
    match argv
        .iter_mut()
        .skip(1)
        .find(|word| *word == VIA_PLACEHOLDER)
    {
        Some(slot) => *slot = remote,
        None => argv.push(remote),
    }
    Ok(argv)
}

/// What differs between the lines ssh is run with: whether it may ask a person, how long it
/// waits to connect, and whether it keeps a long line alive.
struct Shape {
    batch: bool,
    connect_timeout_secs: u32,
    keep_alive: bool,
}

impl Shape {
    const PROXY: Shape = Shape {
        batch: true,
        connect_timeout_secs: CONNECT_TIMEOUT_SECS,
        keep_alive: true,
    };
    const PROBE: Shape = Shape {
        batch: true,
        connect_timeout_secs: PROBE_TIMEOUT_SECS,
        keep_alive: false,
    };
    const FIRST_LOGIN: Shape = Shape {
        batch: false,
        connect_timeout_secs: CONNECT_TIMEOUT_SECS,
        keep_alive: false,
    };
}

fn build(ssh: &[String], target: &str, remote: String, shape: Shape) -> Vec<String> {
    let (program, user_args) = match ssh.split_first() {
        Some((program, rest)) => (program.as_str(), rest),
        None => (crate::SSH_PROGRAM, &[][..]),
    };
    let mut argv = vec![program.to_string()];
    if shape.batch {
        argv.extend(option("BatchMode=yes"));
    }
    argv.extend(option("ForwardAgent=no"));
    argv.extend_from_slice(user_args);
    argv.extend(option(&format!(
        "ConnectTimeout={}",
        shape.connect_timeout_secs
    )));
    if shape.keep_alive {
        argv.extend(option(&format!(
            "ServerAliveInterval={SERVER_ALIVE_INTERVAL_SECS}"
        )));
        argv.extend(option(&format!(
            "ServerAliveCountMax={SERVER_ALIVE_COUNT_MAX}"
        )));
    }
    argv.extend(["-a", "-T", "--"].map(String::from));
    argv.push(target.to_string());
    argv.push(remote);
    argv
}

/// `ssh ... <remote_cmd> --version` that never asks a person: how a new machine is checked
/// before its first login, and whether the app is installed there.
pub(crate) fn ssh_argv_probe(
    ssh: &[String],
    target: &str,
    remote_cmd: &str,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let remote = format!("{} {VERSION_FLAG}", remote_program(remote_cmd)?);
    Ok(build(ssh, target, remote, Shape::PROBE))
}

/// The subcommand that restarts the far side's daemon when no session is open.
const RESTART_IF_IDLE: &str = "restart-if-idle";
/// Added to it to restart even with sessions open, when they are saved.
const RESTART_NOW: &str = "--now";
/// The subcommand that prints how many sessions are open on the far side and how many are busy.
const ACTIVITY: &str = "activity";

/// A fixed script for the remote shell, run as `sh -c <script> sh <args>`. The script is a
/// constant of the app; whatever varies is an argument, quoted as one literal word, so it
/// can never become shell code.
fn sh_script(script: &str, args: &[&str]) -> String {
    let mut line = format!("sh -c {} sh", quote_word(script));
    for arg in args {
        line.push(' ');
        line.push_str(&quote_word(arg));
    }
    line
}

/// Asks the far side for its platform (`uname -sm` behind a marker line), without asking a person.
pub(crate) fn ssh_argv_platform(
    ssh: &[String],
    target: &str,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let remote = sh_script(crate::remote_scripts::PLATFORM_SCRIPT, &[]);
    Ok(build(ssh, target, remote, Shape::PROBE))
}

/// Asks the far side where its login shell finds the app (`command -v`), without asking a person.
pub(crate) fn ssh_argv_where(ssh: &[String], target: &str) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let remote = sh_script(crate::remote_scripts::WHERE_SCRIPT, &[]);
    Ok(build(ssh, target, remote, Shape::PROBE))
}

/// The options every ssh line carries whatever the user's `ssh_args` say, as `doctor` shows them.
pub(crate) fn hardening_summary() -> String {
    format!(
        "BatchMode=yes ForwardAgent=no -a -T ConnectTimeout={CONNECT_TIMEOUT_SECS} ServerAlive={SERVER_ALIVE_INTERVAL_SECS}x{SERVER_ALIVE_COUNT_MAX}"
    )
}

/// Runs the install script on the far side; the binary goes in on stdin. `destination` and
/// `bytes` are its two arguments.
pub(crate) fn ssh_argv_install(
    ssh: &[String],
    target: &str,
    destination: &str,
    bytes: u64,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    if destination.trim().is_empty() {
        return Err(EmptyRemoteCommand);
    }
    if destination.chars().any(char::is_control) {
        return Err(RemoteCommandHasControlCharacter);
    }
    let remote = sh_script(
        crate::remote_scripts::INSTALL_SCRIPT,
        &[destination, &bytes.to_string()],
    );
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// The longest name a pasted or sent file may have: the 28-character stamp and salt (ADR 0007)
/// and the 48 kept of the original name fit with room to spare.
const PASTE_NAME_MAX: usize = 96;

/// The longest path to read on the far side.
const MOST_REMOTE_PATH_BYTES: usize = 4096;

/// Runs the get script on the far side: it prints the file at `path`, refusing one over
/// `max_bytes`. Both reach it as arguments, never as part of its text.
pub(crate) fn ssh_argv_get(
    ssh: &[String],
    target: &str,
    path: &str,
    max_bytes: u64,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    if path.is_empty()
        || path.len() > MOST_REMOTE_PATH_BYTES
        || path.chars().any(char::is_control)
    {
        return Err(BadRemotePath(path.chars().take(80).collect()));
    }
    let remote = sh_script(
        crate::remote_scripts::GET_SCRIPT,
        &[path, &max_bytes.to_string()],
    );
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// Runs the list script on the far side: it prints what is in the folder `path` (empty is the
/// far side's home), at most `max_bytes` of it. Both reach it as arguments, never as its text.
pub(crate) fn ssh_argv_list(
    ssh: &[String],
    target: &str,
    path: &str,
    max_bytes: u64,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    if path.len() > MOST_REMOTE_PATH_BYTES || path.chars().any(char::is_control) {
        return Err(BadRemotePath(path.chars().take(80).collect()));
    }
    let remote = sh_script(
        crate::remote_scripts::LIST_SCRIPT,
        &[path, &max_bytes.to_string()],
    );
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// Runs the index script on the far side: it prints the paths of the files below the folder
/// `path` (empty is the far side's home), at most `max_bytes` of it. Both reach it as arguments.
pub(crate) fn ssh_argv_index(
    ssh: &[String],
    target: &str,
    path: &str,
    max_bytes: u64,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    if path.len() > MOST_REMOTE_PATH_BYTES || path.chars().any(char::is_control) {
        return Err(BadRemotePath(path.chars().take(80).collect()));
    }
    let remote = sh_script(
        crate::remote_scripts::INDEX_SCRIPT,
        &[path, &max_bytes.to_string()],
    );
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// Runs the folder script on the far side: it prints the folder `path` as a tar archive. The path
/// reaches it as an argument, never as part of its text.
pub(crate) fn ssh_argv_get_folder(
    ssh: &[String],
    target: &str,
    path: &str,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    if path.is_empty() || path.len() > MOST_REMOTE_PATH_BYTES || path.chars().any(char::is_control) {
        return Err(BadRemotePath(path.chars().take(80).collect()));
    }
    let remote = sh_script(crate::remote_scripts::FOLDER_SCRIPT, &[path]);
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// Runs the paste script on the far side; the image goes in on stdin. `name` and `bytes` are
/// its two arguments.
pub(crate) fn ssh_argv_paste(
    ssh: &[String],
    target: &str,
    name: &str,
    bytes: u64,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let plain = !name.is_empty()
        && name.len() <= PASTE_NAME_MAX
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !plain {
        return Err(BadPasteName(name.to_owned()));
    }
    let remote = sh_script(
        crate::remote_scripts::PASTE_SCRIPT,
        &[name, &bytes.to_string()],
    );
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// Runs the put-folder script on the far side; the folder goes in on stdin as a tar archive.
/// `name` (a plain word, as for a pasted file) and `bytes` are its two arguments.
pub(crate) fn ssh_argv_put_folder(
    ssh: &[String],
    target: &str,
    name: &str,
    bytes: u64,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let plain = !name.is_empty()
        && name.len() <= PASTE_NAME_MAX
        && !name.starts_with('.')
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'));
    if !plain {
        return Err(BadPasteName(name.to_owned()));
    }
    let remote = sh_script(
        crate::remote_scripts::PUT_FOLDER_SCRIPT,
        &[name, &bytes.to_string()],
    );
    Ok(build(ssh, target, remote, Shape::PROXY))
}

/// `<program> restart-if-idle` on the far side: the installed app restarts its daemon when no
/// session is open, or with `now` also when its sessions are saved. `program` is the path it
/// was installed to.
pub(crate) fn ssh_argv_restart_if_idle(
    ssh: &[String],
    target: &str,
    program: &str,
    now: bool,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let mut remote = format!("{} {RESTART_IF_IDLE}", remote_program(program)?);
    if now {
        remote.push(' ');
        remote.push_str(RESTART_NOW);
    }
    Ok(build(ssh, target, remote, Shape::PROBE))
}

/// `<program> activity` on the far side: one line saying how many sessions are open and how
/// many are mid-turn. It changes nothing there.
pub(crate) fn ssh_argv_activity(
    ssh: &[String],
    target: &str,
    program: &str,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    let remote = format!("{} {ACTIVITY}", remote_program(program)?);
    Ok(build(ssh, target, remote, Shape::PROBE))
}

/// The same line as the probe, without `BatchMode`, to run in a terminal where ssh can ask
/// for a password, a code or a host key. The remote command is not quoted for the remote
/// shell (a local shell reads this line first, and quoting for two shells at once is how
/// words get lost), so `remote_cmd` must be a plain word.
pub(crate) fn ssh_argv_interactive(
    ssh: &[String],
    target: &str,
    remote_cmd: &str,
) -> Result<Vec<String>, SshCommandError> {
    validate_target(target)?;
    if remote_cmd.is_empty() || !remote_cmd.chars().all(is_plain) {
        return Err(RemoteCommandNotPlain(remote_cmd.to_string()));
    }
    let remote = format!("{remote_cmd} {VERSION_FLAG}");
    Ok(build(ssh, target, remote, Shape::FIRST_LOGIN))
}

/// A character every shell the app runs a pane with reads as part of a word, here and on the
/// far side.
fn is_plain(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '_' | '.' | '/' | '-' | '~' | ':' | '=' | ',' | '+')
}

/// `argv` as one line for the shell a pane runs a command with (bash, zsh, fish, PowerShell).
/// A word of plain characters stays bare; any other is one single-quoted word, which those
/// shells read literally. A word with a quote or a control character is refused rather than
/// escaped, since each shell escapes differently. The program is never quoted: PowerShell
/// would read a quoted program as a string.
pub(crate) fn popup_command(argv: &[String]) -> Result<String, SshCommandError> {
    let mut words = Vec::with_capacity(argv.len());
    for (i, word) in argv.iter().enumerate() {
        let bare = !word.is_empty()
            && !word.starts_with('~')
            && word
                .chars()
                .all(|c| is_plain(c) || c == '@' || is_path_separator(c));
        if bare {
            words.push(word.clone());
        } else if i > 0 && !word.is_empty() && !word.chars().any(|c| c == '\'' || c.is_control()) {
            words.push(format!("'{word}'"));
        } else {
            return Err(CannotQuote(word.clone()));
        }
    }
    Ok(words.join(" "))
}

/// A backslash is a path separator to a Windows shell and an escape to every other.
fn is_path_separator(c: char) -> bool {
    cfg!(windows) && c == '\\'
}

fn option(setting: &str) -> [String; 2] {
    ["-o".to_string(), setting.to_string()]
}

fn validate_target(target: &str) -> Result<(), SshCommandError> {
    if target.is_empty() {
        return Err(EmptyTarget);
    }
    if target.starts_with('-') {
        return Err(TargetIsOption(target.to_string()));
    }
    if target.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err(TargetNotAHost(target.to_string()));
    }
    Ok(())
}

/// `<remote_cmd> proxy [--attach-only]` as one string for the remote shell. `remote_cmd` is one program
/// (a name on the remote PATH, or a path): arguments in it are not supported, because the
/// whole of it is a single literal word. A leading `~/` stays outside the quotes so the
/// remote shell still expands it to the remote home.
pub(crate) fn remote_command(remote_cmd: &str, mode: ProxyMode) -> Result<String, SshCommandError> {
    let program = remote_program(remote_cmd)?;
    Ok(match mode {
        ProxyMode::StartsDaemon => format!("{program} {PROXY}"),
        ProxyMode::AttachOnly => format!("{program} {PROXY} {ATTACH_ONLY_FLAG}"),
    })
}

/// The remote command as one literal word for the remote shell.
fn remote_program(remote_cmd: &str) -> Result<String, SshCommandError> {
    if remote_cmd.trim().is_empty() {
        return Err(EmptyRemoteCommand);
    }
    if remote_cmd.chars().any(char::is_control) {
        return Err(RemoteCommandHasControlCharacter);
    }
    Ok(match remote_cmd.strip_prefix("~/") {
        Some(rest) => format!("~/{}", quote_word(rest)),
        None => quote_word(remote_cmd),
    })
}

/// One word for a POSIX shell: inside single quotes nothing is special, so `'` is closed,
/// escaped and reopened. Fish reads the same form the same way.
fn quote_word(word: &str) -> String {
    format!("'{}'", word.replace('\'', r"'\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn via(words: &[&str], remote_cmd: &str, mode: ProxyMode) -> Vec<String> {
        command_argv(&args(words), remote_cmd, mode).expect("a valid command")
    }

    #[test]
    fn a_custom_transport_gets_the_remote_command_as_the_placeholder_element() {
        let got = via(
            &["fly", "ssh", "console", "-C", "{command}"],
            "~/.cargo/bin/maitty",
            ProxyMode::AttachOnly,
        );
        assert_eq!(
            got,
            args(&[
                "fly",
                "ssh",
                "console",
                "-C",
                "~/'.cargo/bin/maitty' proxy --attach-only"
            ])
        );
    }

    #[test]
    fn a_custom_transport_without_a_placeholder_gets_it_as_the_last_argument() {
        let got = via(&["relay", "--to", "box"], "maitty", ProxyMode::AttachOnly);
        assert_eq!(
            got,
            args(&["relay", "--to", "box", "'maitty' proxy --attach-only"])
        );
        let starts = via(&["relay"], "maitty", ProxyMode::StartsDaemon);
        assert_eq!(starts, args(&["relay", "'maitty' proxy"]));
    }

    #[test]
    fn a_custom_transport_is_not_given_any_ssh_option() {
        let got = via(&["relay", "{command}"], "maitty", ProxyMode::AttachOnly);
        assert_eq!(got.len(), 2);
        for ssh_only in ["-o", "-a", "-T", "--", "BatchMode=yes", "ForwardAgent=no"] {
            assert!(!got.iter().any(|word| word == ssh_only), "{ssh_only}");
        }
    }

    #[test]
    fn a_hostile_remote_command_stays_one_quoted_element() {
        let got = via(
            &["relay", "{command}", "after"],
            "x'; touch /tmp/pwned; '",
            ProxyMode::AttachOnly,
        );
        assert_eq!(got.len(), 3);
        assert_eq!(
            got[1],
            r"'x'\''; touch /tmp/pwned; '\''' proxy --attach-only"
        );
        assert_eq!(got[2], "after");
    }

    #[test]
    fn a_custom_transport_needs_a_program_and_a_valid_remote_command() {
        assert_eq!(
            command_argv(&[], "maitty", ProxyMode::AttachOnly),
            Err(EmptyTransport)
        );
        assert_eq!(
            command_argv(&args(&["relay"]), "ma\u{7}itty", ProxyMode::AttachOnly),
            Err(RemoteCommandHasControlCharacter)
        );
    }

    fn args(words: &[&str]) -> Vec<String> {
        words.iter().map(|w| w.to_string()).collect()
    }

    fn argv(ssh: &[&str], target: &str, remote_cmd: &str) -> Vec<String> {
        ssh_argv(&args(ssh), target, remote_cmd, ProxyMode::StartsDaemon).expect("a valid command")
    }

    fn remote_command(remote_cmd: &str) -> Result<String, SshCommandError> {
        super::remote_command(remote_cmd, ProxyMode::StartsDaemon)
    }

    fn position(argv: &[String], word: &str) -> usize {
        argv.iter()
            .position(|a| a == word)
            .unwrap_or_else(|| panic!("{word} is not in {argv:?}"))
    }

    #[test]
    fn the_target_follows_dash_dash_and_the_command_is_last() {
        let argv = argv(&["ssh", "-p", "2222"], "me@box", "maitty");
        let dashes = position(&argv, "--");
        assert_eq!(argv[dashes + 1], "me@box");
        assert_eq!(argv[dashes + 2], "'maitty' proxy");
        assert_eq!(argv.len(), dashes + 3);
    }

    #[test]
    fn the_hardening_options_are_present() {
        let argv = argv(&["ssh"], "box", "maitty").join(" ");
        for want in [
            "-o BatchMode=yes",
            "-o ForwardAgent=no",
            "-o ConnectTimeout=15",
            "-o ServerAliveInterval=15",
            "-o ServerAliveCountMax=3",
            " -a ",
            " -T ",
        ] {
            assert!(argv.contains(want), "{want:?} missing from {argv}");
        }
    }

    #[test]
    fn the_user_cannot_undo_batch_mode_or_agent_forwarding_but_may_tune_the_timeouts() {
        let argv = argv(
            &["ssh", "-o", "BatchMode=no", "-A", "-o", "ConnectTimeout=3"],
            "box",
            "maitty",
        );
        let first = |word: &str| position(&argv, word);
        assert!(first("BatchMode=yes") < first("BatchMode=no"));
        assert!(first("ForwardAgent=no") < first("-A"));
        assert!(first("-A") < first("-a"), "-a comes after -A");
        assert!(first("ConnectTimeout=3") < first("ConnectTimeout=15"));
    }

    #[test]
    fn a_host_that_is_an_ssh_option_is_refused() {
        for target in [
            "-oProxyCommand=touch pwned",
            "-oProxyCommand=x",
            "-F/etc/passwd",
            "-",
        ] {
            assert!(
                matches!(
                    ssh_argv(&args(&["ssh"]), target, "maitty", ProxyMode::StartsDaemon),
                    Err(TargetIsOption(_))
                ),
                "{target}"
            );
        }
    }

    #[test]
    fn a_host_with_whitespace_or_nothing_is_refused() {
        for target in ["box other", "box\n", "box\t-x"] {
            assert!(matches!(
                ssh_argv(&args(&["ssh"]), target, "maitty", ProxyMode::StartsDaemon),
                Err(TargetNotAHost(_))
            ));
        }
        assert_eq!(
            ssh_argv(&args(&["ssh"]), "", "maitty", ProxyMode::StartsDaemon),
            Err(EmptyTarget)
        );
    }

    #[test]
    fn a_remote_command_with_shell_syntax_stays_one_literal_word() {
        assert_eq!(
            remote_command("/opt/my tools/maitty").unwrap(),
            "'/opt/my tools/maitty' proxy"
        );
        assert_eq!(
            remote_command("$(touch /tmp/x)`id`;rm -rf ~ && echo \"hi\" | cat").unwrap(),
            "'$(touch /tmp/x)`id`;rm -rf ~ && echo \"hi\" | cat' proxy"
        );
        assert_eq!(remote_command("it's").unwrap(), r"'it'\''s' proxy");
        assert_eq!(remote_command("''").unwrap(), r"''\'''\''' proxy");
    }

    #[test]
    fn a_leading_home_shorthand_still_expands_on_the_remote() {
        assert_eq!(
            remote_command("~/.cargo/bin/maitty").unwrap(),
            "~/'.cargo/bin/maitty' proxy"
        );
        assert_eq!(
            remote_command("~/my tools/maitty").unwrap(),
            "~/'my tools/maitty' proxy"
        );
        assert_eq!(
            remote_command("~root/maitty").unwrap(),
            "'~root/maitty' proxy",
            "only the plain `~/` form is expanded"
        );
    }

    #[test]
    fn a_remote_command_with_a_newline_or_nothing_is_refused() {
        for cmd in ["a\nb", "a\rb", "a\0b", "a\u{1b}b"] {
            assert_eq!(
                remote_command(cmd),
                Err(RemoteCommandHasControlCharacter),
                "{cmd:?}"
            );
        }
        assert_eq!(remote_command(""), Err(EmptyRemoteCommand));
        assert_eq!(remote_command("  "), Err(EmptyRemoteCommand));
    }

    #[test]
    fn a_hostile_remote_command_never_becomes_an_ssh_option() {
        let argv = argv(&["ssh"], "box", "-oProxyCommand=x");
        let dashes = position(&argv, "--");
        assert_eq!(argv[dashes + 2], "'-oProxyCommand=x' proxy");
        assert!(!argv[..dashes].iter().any(|a| a.contains("ProxyCommand")));
    }

    #[test]
    fn a_configured_machine_s_proxy_only_attaches() {
        let attach = |cmd| super::remote_command(cmd, ProxyMode::AttachOnly).unwrap();
        assert_eq!(attach("maitty"), "'maitty' proxy --attach-only");
        assert_eq!(attach("~/bin/maitty"), "~/'bin/maitty' proxy --attach-only");
        let full = ssh_argv(&args(&["ssh"]), "box", "maitty", ProxyMode::AttachOnly).unwrap();
        assert_eq!(full.last().unwrap(), "'maitty' proxy --attach-only");
        assert_eq!(remote_command("maitty").unwrap(), "'maitty' proxy");
    }

    /// A Windows client: its ssh program and arguments are Windows paths with backslashes.
    /// None of that reaches the remote string, which depends on `remote_cmd` alone.
    #[test]
    fn a_windows_client_builds_the_same_remote_command_as_any_other() {
        let windows = argv(
            &[
                r"C:\Program Files\OpenSSH\ssh.exe",
                "-i",
                r"C:\Users\me\.ssh\id ed25519",
            ],
            "me@linux-box",
            "~/.cargo/bin/maitty",
        );
        let unix = argv(
            &["ssh", "-i", "/home/me/.ssh/id"],
            "me@linux-box",
            "~/.cargo/bin/maitty",
        );
        assert_eq!(windows[0], r"C:\Program Files\OpenSSH\ssh.exe");
        assert!(windows.contains(&r"C:\Users\me\.ssh\id ed25519".to_string()));
        assert_eq!(windows.last(), unix.last());
        assert_eq!(windows.last().unwrap(), "~/'.cargo/bin/maitty' proxy");
    }

    #[test]
    fn a_windows_looking_remote_command_is_still_one_literal_word() {
        assert_eq!(
            remote_command(r"C:\Program Files\maitty\maitty.exe").unwrap(),
            r"'C:\Program Files\maitty\maitty.exe' proxy"
        );
    }

    /// What the remote user's `sh` makes of the string: the program is one word, and
    /// nothing in the path runs.
    #[cfg(unix)]
    #[test]
    fn the_remote_shell_receives_the_command_as_one_literal_program() {
        let dir = std::env::temp_dir().join(format!("ssh-command-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let program = dir.join("it's a $(touch pwned) `touch pwned2`; touch pwned3 && \"q\"");
        std::fs::write(&program, "#!/bin/sh\nprintf '%s' \"$*\"\n").unwrap();
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
        }
        let command = remote_command(program.to_str().unwrap()).unwrap();
        let out = std::process::Command::new("sh")
            .current_dir(&dir)
            .arg("-c")
            .arg(&command)
            .output()
            .unwrap();
        let ran = String::from_utf8_lossy(&out.stdout).into_owned();
        let pwned = ["pwned", "pwned2", "pwned3"].map(|f| dir.join(f).exists());
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(
            ran,
            "proxy",
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(pwned, [false; 3]);
    }

    /// `~/` is expanded by the shell even though the rest is quoted.
    #[cfg(unix)]
    #[test]
    fn the_remote_shell_still_expands_a_leading_home_shorthand() {
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!("echo {}", remote_command("~/x y").unwrap()))
            .output()
            .unwrap();
        let said = String::from_utf8_lossy(&out.stdout).into_owned();
        let home = std::env::var("HOME").unwrap_or_default();
        assert_eq!(said.trim_end(), format!("{home}/x y proxy"));
    }

    fn probe(ssh: &[&str], target: &str, cmd: &str) -> Result<Vec<String>, SshCommandError> {
        ssh_argv_probe(&args(ssh), target, cmd)
    }

    fn first_login(ssh: &[&str], target: &str, cmd: &str) -> Result<Vec<String>, SshCommandError> {
        ssh_argv_interactive(&args(ssh), target, cmd)
    }

    #[test]
    fn the_probe_never_asks_and_runs_the_version() {
        let argv = probe(&["ssh", "-p", "2222"], "me@box", "maitty").unwrap();
        let joined = argv.join(" ");
        assert!(joined.contains("-o BatchMode=yes") && joined.contains("-o ConnectTimeout=8"));
        assert!(!joined.contains("ServerAlive"));
        let dashes = position(&argv, "--");
        assert_eq!(argv[dashes + 1], "me@box");
        assert_eq!(argv[dashes + 2], "'maitty' --version");
    }

    #[test]
    fn the_first_login_may_ask_but_keeps_the_hardening() {
        let argv = first_login(&["ssh", "-A", "-p", "2222"], "me@box", "maitty").unwrap();
        let joined = argv.join(" ");
        assert!(!joined.contains("BatchMode"), "{joined}");
        for want in ["-o ForwardAgent=no", "-o ConnectTimeout=15", " -a ", " -T "] {
            assert!(joined.contains(want), "{want:?} missing from {joined}");
        }
        assert!(position(&argv, "ForwardAgent=no") < position(&argv, "-A"));
        let dashes = position(&argv, "--");
        assert_eq!(&argv[dashes + 1..], ["me@box", "maitty --version"]);
    }

    #[test]
    fn the_first_login_and_the_probe_refuse_a_hostile_host() {
        for target in ["-oProxyCommand=x", "box other", ""] {
            assert!(probe(&["ssh"], target, "maitty").is_err(), "{target}");
            assert!(first_login(&["ssh"], target, "maitty").is_err(), "{target}");
        }
    }

    #[test]
    fn the_first_login_needs_a_remote_command_that_is_a_plain_word() {
        assert!(first_login(&["ssh"], "box", "~/.cargo/bin/maitty").is_ok());
        for cmd in ["my tools/maitty", "$(id)", "a;b", "it's", "", "a`b`"] {
            assert!(
                matches!(
                    first_login(&["ssh"], "box", cmd),
                    Err(RemoteCommandNotPlain(_))
                ),
                "{cmd:?}"
            );
        }
    }

    #[test]
    fn a_popup_line_keeps_plain_words_bare_and_quotes_the_rest_once() {
        let argv = first_login(&["ssh", "-p", "2222"], "me@box", "maitty").unwrap();
        let line = popup_command(&argv).unwrap();
        assert_eq!(
            line,
            "ssh -o ForwardAgent=no -p 2222 -o ConnectTimeout=15 -a -T -- me@box 'maitty --version'"
        );
        assert_eq!(
            popup_command(&args(&["ssh", "-i", "my key"])).unwrap(),
            "ssh -i 'my key'"
        );
    }

    #[test]
    fn a_popup_line_refuses_a_word_it_cannot_quote_for_every_shell() {
        for word in ["it's", "a\nb", "a\u{1b}b", ""] {
            assert_eq!(
                popup_command(&args(&["ssh", word])),
                Err(CannotQuote(word.to_string())),
                "{word:?}"
            );
        }
        assert!(popup_command(&args(&["my ssh", "box"])).is_err());
    }

    #[test]
    fn a_hostile_target_that_passes_the_host_check_is_one_quoted_word_in_the_popup_line() {
        let argv = first_login(&["ssh"], "box;rm$IFS-rf", "maitty").unwrap();
        let line = popup_command(&argv).unwrap();
        assert!(line.contains("-- 'box;rm$IFS-rf' "), "{line}");
        let tilde = first_login(&["ssh"], "~root", "maitty").unwrap();
        assert!(popup_command(&tilde).unwrap().contains("-- '~root' "));
    }

    #[test]
    fn the_platform_probe_is_the_fixed_script_and_nothing_else() {
        let argv = ssh_argv_platform(&args(&["ssh", "-p", "2222"]), "me@box").unwrap();
        let dashes = position(&argv, "--");
        assert_eq!(argv[dashes + 1], "me@box");
        assert_eq!(
            argv[dashes + 2],
            r"sh -c 'printf '\''%s\n'\'' maitty-platform; uname -sm' sh"
        );
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert!(ssh_argv_platform(&args(&["ssh"]), "-oProxyCommand=x").is_err());
    }

    #[test]
    fn the_where_probe_is_the_fixed_script_and_nothing_else() {
        let argv = ssh_argv_where(&args(&["ssh", "-p", "2222"]), "me@box").unwrap();
        let dashes = position(&argv, "--");
        assert_eq!(argv[dashes + 1], "me@box");
        assert_eq!(argv[dashes + 2], "sh -c 'command -v maitty' sh");
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert!(ssh_argv_where(&args(&["ssh"]), "-oProxyCommand=x").is_err());
    }

    #[test]
    fn the_summary_names_the_options_every_connection_carries() {
        let line = argv(&["ssh"], "box", "maitty");
        let summary = hardening_summary();
        for option in [
            "BatchMode=yes",
            "ForwardAgent=no",
            "ConnectTimeout=15",
            "ServerAliveInterval=15",
            "ServerAliveCountMax=3",
        ] {
            assert!(line.contains(&option.to_string()), "{option} is not used");
        }
        for flag in ["-a", "-T"] {
            assert!(line.contains(&flag.to_string()) && summary.contains(flag));
        }
        assert_eq!(
            summary,
            "BatchMode=yes ForwardAgent=no -a -T ConnectTimeout=15 ServerAlive=15x3"
        );
    }

    #[test]
    fn the_install_line_passes_the_destination_and_count_as_separate_quoted_words() {
        let argv =
            ssh_argv_install(&args(&["ssh"]), "box", "/opt/it's mine/maitty", 14200000).unwrap();
        let remote = argv.last().unwrap();
        let tail = r" sh '/opt/it'\''s mine/maitty' '14200000'";
        let script = remote
            .strip_prefix("sh -c ")
            .and_then(|r| r.strip_suffix(tail))
            .unwrap_or_else(|| panic!("{remote}"));
        assert_eq!(
            script,
            quote_word(crate::remote_scripts::INSTALL_SCRIPT),
            "the script text is the constant, with no argument in it"
        );
        assert!(argv.contains(&"BatchMode=yes".to_string()));
        assert!(argv.contains(&"ForwardAgent=no".to_string()));
        assert!(argv.contains(&"ServerAliveInterval=15".to_string()));
    }

    #[test]
    fn a_hostile_destination_or_host_never_reaches_ssh_as_code() {
        for dest in ["", "  ", "a\nb", "a\u{1b}b"] {
            assert!(
                ssh_argv_install(&args(&["ssh"]), "box", dest, 1).is_err(),
                "{dest:?}"
            );
        }
        let hostile = ssh_argv_install(&args(&["ssh"]), "box", "$(touch pwned);`id`", 1).unwrap();
        assert!(
            hostile
                .last()
                .unwrap()
                .ends_with(" sh '$(touch pwned);`id`' '1'")
        );
        assert!(ssh_argv_install(&args(&["ssh"]), "-oProxyCommand=x", "/x", 1).is_err());
    }

    #[test]
    fn restart_if_idle_runs_the_installed_path_with_a_home_shorthand_kept() {
        let argv = ssh_argv_restart_if_idle(&args(&["ssh"]), "box", "~/.local/bin/maitty", false).unwrap();
        assert_eq!(
            argv.last().unwrap(),
            "~/'.local/bin/maitty' restart-if-idle"
        );
        assert!(ssh_argv_restart_if_idle(&args(&["ssh"]), "box", "", false).is_err());
    }

    /// A POSIX shell reads the popup line back as exactly the argv it was built from.
    #[cfg(unix)]
    #[test]
    fn a_posix_shell_reads_the_popup_line_as_the_same_words() {
        let argv: Vec<String> = [
            "printf",
            "%s|",
            "me@box",
            "a b",
            "$(touch pwned)",
            "x;y",
            "~",
        ]
        .map(String::from)
        .to_vec();
        let line = popup_command(&argv).unwrap();
        let out = std::process::Command::new("sh")
            .arg("-c")
            .arg(&line)
            .current_dir(std::env::temp_dir())
            .output()
            .unwrap();
        assert_eq!(
            String::from_utf8_lossy(&out.stdout),
            "me@box|a b|$(touch pwned)|x;y|~|"
        );
    }

    #[test]
    fn the_paste_line_runs_a_fixed_script_with_the_name_and_size_as_its_last_words() {
        let line = ssh_argv_paste(
            &args(&["ssh", "-p", "22"]),
            "box",
            "paste-1-000000ab.png",
            1234,
        )
        .unwrap();
        let dashes = position(&line, "--");
        assert_eq!(line[dashes + 1], "box", "the host follows `--`");
        let remote = line.last().unwrap();
        assert!(remote.starts_with("sh -c '"), "{remote}");
        assert!(
            remote.ends_with(" sh 'paste-1-000000ab.png' '1234'"),
            "{remote}"
        );
        assert!(remote.contains(crate::remote_scripts::PASTE_MARKER));
        assert!(
            line.iter().any(|word| word == "BatchMode=yes"),
            "never asks a person"
        );
        assert!(line.iter().any(|word| word == "ForwardAgent=no"));
    }

    #[test]
    fn a_pasted_files_name_cannot_be_a_path_an_option_or_shell_syntax() {
        let name = |name: &str| ssh_argv_paste(&args(&["ssh"]), "box", name, 1);
        for bad in [
            "",
            ".hidden",
            "a b",
            "../x",
            "a/b",
            "x;rm -rf ~",
            "x$(id)",
            "x\ny",
            "-oProxyCommand=x",
            &"a".repeat(97),
        ] {
            assert_eq!(name(bad), Err(BadPasteName(bad.to_owned())), "{bad:?}");
        }
        assert!(name("paste-1.png").is_ok());
        assert!(name("a_b-c.9").is_ok());
        assert!(ssh_argv_paste(&args(&["ssh"]), "-oProxyCommand=x", "p.png", 1).is_err());
    }

    #[test]
    fn restart_now_adds_one_fixed_flag_and_nothing_else() {
        let idle = ssh_argv_restart_if_idle(&args(&["ssh"]), "box", "~/.local/bin/maitty", false).unwrap();
        let now = ssh_argv_restart_if_idle(&args(&["ssh"]), "box", "~/.local/bin/maitty", true).unwrap();
        assert_eq!(now.last().unwrap(), &format!("{} --now", idle.last().unwrap()));
        assert_eq!(now[..now.len() - 1], idle[..idle.len() - 1]);
    }

    #[test]
    fn activity_runs_the_installed_program_with_one_fixed_word() {
        let argv = ssh_argv_activity(&args(&["ssh"]), "box", "~/.local/bin/maitty").unwrap();
        assert_eq!(argv.last().unwrap(), "~/'.local/bin/maitty' activity");
        assert!(ssh_argv_activity(&args(&["ssh"]), "box", "").is_err());
    }

    #[test]
    fn the_get_line_passes_the_path_and_the_cap_as_the_last_two_quoted_words() {
        let path = "~/out dir/it's $HOME `x` -n.log";
        let line = ssh_argv_get(&args(&["ssh"]), "box", path, 1000).unwrap();
        let remote = line.last().unwrap();
        assert!(
            remote.ends_with(&format!(" sh {} '1000'", quote_word(path))),
            "{remote}"
        );
        assert!(
            !remote.replace(&quote_word(path), "").contains("`x`"),
            "the path never appears outside its quotes"
        );
    }

    #[test]
    fn a_path_that_is_empty_too_long_or_holds_control_characters_is_refused_before_ssh() {
        let get = |path: &str| ssh_argv_get(&args(&["ssh"]), "box", path, 1);
        assert!(get("").is_err());
        assert!(get("a\nb").is_err());
        assert!(get("a\0b").is_err());
        assert!(get("a\u{1b}[31mb").is_err());
        assert!(get(&"a".repeat(4097)).is_err());
        assert!(get(&"a".repeat(4096)).is_ok());
        assert!(get("-rf").is_ok(), "a leading dash is a file name, the script uses --");
        assert!(ssh_argv_get(&args(&["ssh"]), "-oProxyCommand=x", "a", 1).is_err());
    }

    #[test]
    fn the_list_line_passes_the_folder_and_the_cap_as_the_last_two_quoted_words() {
        let folder = "~/my dir/it's $HOME `x` -n";
        let line = ssh_argv_list(&args(&["ssh"]), "box", folder, 5000).unwrap();
        let remote = line.last().unwrap();
        assert!(remote.ends_with(&format!(" sh {} '5000'", quote_word(folder))), "{remote}");
        let home = ssh_argv_list(&args(&["ssh"]), "box", "", 5000).unwrap();
        assert!(home.last().unwrap().ends_with(" sh '' '5000'"), "an empty folder means home");
    }

    #[test]
    fn a_folder_that_is_too_long_or_holds_control_characters_is_refused_before_ssh() {
        let list = |path: &str| ssh_argv_list(&args(&["ssh"]), "box", path, 1);
        assert!(list("a\nb").is_err());
        assert!(list("a\0b").is_err());
        assert!(list(&"a".repeat(4097)).is_err());
        assert!(list("-rf").is_ok(), "a leading dash is a folder name, the script quotes it");
        assert!(ssh_argv_list(&args(&["ssh"]), "-oProxyCommand=x", "a", 1).is_err());
    }

    #[test]
    fn the_index_line_passes_the_folder_and_the_cap_as_the_last_two_quoted_words() {
        let folder = "~/my dir/it's $HOME -n";
        let line = ssh_argv_index(&args(&["ssh"]), "box", folder, 900).unwrap();
        assert!(line.last().unwrap().ends_with(&format!(" sh {} '900'", quote_word(folder))));
        assert!(ssh_argv_index(&args(&["ssh"]), "box", "a\nb", 1).is_err());
        assert!(ssh_argv_index(&args(&["ssh"]), "box", &"a".repeat(4097), 1).is_err());
        assert!(ssh_argv_index(&args(&["ssh"]), "-oProxyCommand=x", "a", 1).is_err());
    }

    #[test]
    fn the_folder_line_passes_the_path_as_the_last_quoted_word() {
        let folder = "~/my dir/it's $HOME `x` -n";
        let line = ssh_argv_get_folder(&args(&["ssh"]), "box", folder).unwrap();
        assert!(line.last().unwrap().ends_with(&format!(" sh {}", quote_word(folder))));
        let get = |path: &str| ssh_argv_get_folder(&args(&["ssh"]), "box", path);
        assert!(get("").is_err());
        assert!(get("a\nb").is_err());
        assert!(get(&"a".repeat(4097)).is_err());
        assert!(ssh_argv_get_folder(&args(&["ssh"]), "-oProxyCommand=x", "a").is_err());
    }

    #[test]
    fn the_put_folder_line_passes_a_plain_name_and_the_size_as_its_last_two_words() {
        let line = ssh_argv_put_folder(&args(&["ssh"]), "box", "paste-1-000000ab-proj", 4096).unwrap();
        assert!(line.last().unwrap().ends_with(" sh 'paste-1-000000ab-proj' '4096'"), "{line:?}");
        let name = |name: &str| ssh_argv_put_folder(&args(&["ssh"]), "box", name, 1);
        for bad in ["", ".hidden", "../up", "a b", "a;b", "a'b", "$(x)", "a/b"] {
            assert!(name(bad).is_err(), "{bad:?}");
        }
        assert!(ssh_argv_put_folder(&args(&["ssh"]), "-oProxyCommand=x", "p", 1).is_err());
    }
}
