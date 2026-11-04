//! Installing and updating the app on another machine (ADR 0047, D6 and D8): what the probe
//! of the far side means, whether to go ahead, and the one script that puts a binary there.
//!
//! Everything here is a pure function of strings and numbers, so every client OS checks the
//! same rules. The script is a constant: the destination and the byte count reach it as
//! positional arguments (quoted once, in `ssh_command`), never as part of its text, and the
//! binary reaches it on stdin. It never signals a process: a daemon already running there
//! keeps its sessions, because the old file is replaced by a rename and the running one holds
//! its own copy.

use crate::brand::APP_COMMAND;
use crate::version::parse;
use maitty_shared::hidden::sanitize;

/// The line the platform probe prints before `uname`, so login text ahead of it is skipped.
pub const PLATFORM_MARKER: &str = "maitty-platform";

pub use maitty_machines::{INSTALL_SCRIPT, PASTE_MARKER, PASTE_SCRIPT, PLATFORM_SCRIPT, WHERE_SCRIPT};


/// Most bytes ever streamed to a machine (the binary is some 15 MB).
pub const MOST_BINARY_BYTES: u64 = 200 * 1024 * 1024;

/// Where the app goes on a machine whose `remote_cmd` is only a name on the PATH.
pub const DEFAULT_DESTINATION: &str = concat!("~/.local/bin/", env!("APP_COMMAND"));

/// Exit statuses of `INSTALL_SCRIPT`.
pub const EXIT_BYTE_COUNT: i32 = 70;
pub const EXIT_WONT_RUN: i32 = 71;
pub const EXIT_CANNOT_WRITE: i32 = 72;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Os {
    Linux,
    Macos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Arch {
    X86_64,
    Aarch64,
}

/// A machine the app can be installed on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Platform {
    pub os: Os,
    pub arch: Arch,
}

impl Platform {
    /// The names Rust gives them (`std::env::consts`), which the release assets follow.
    pub fn os_name(&self) -> &'static str {
        match self.os {
            Os::Linux => "linux",
            Os::Macos => "macos",
        }
    }

    pub fn arch_name(&self) -> &'static str {
        match self.arch {
            Arch::X86_64 => "x86_64",
            Arch::Aarch64 => "aarch64",
        }
    }

    /// This machine, when it could be a remote's twin (Windows never is).
    pub fn of_this_machine() -> Option<Platform> {
        from_rust_names(std::env::consts::OS, std::env::consts::ARCH)
    }
}

impl std::fmt::Display for Platform {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.os_name(), self.arch_name())
    }
}

fn from_rust_names(os: &str, arch: &str) -> Option<Platform> {
    let os = match os {
        "linux" => Os::Linux,
        "macos" => Os::Macos,
        _ => return None,
    };
    let arch = match arch {
        "x86_64" => Arch::X86_64,
        "aarch64" => Arch::Aarch64,
        _ => return None,
    };
    Some(Platform { os, arch })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PlatformError {
    /// The far side is a Windows machine (or a Unix layer on one).
    Windows,
    /// A system we publish no build for, as it said so (cleaned for the screen).
    Unsupported(String),
    /// The marker never appeared: the answer is not a platform probe's.
    NoAnswer,
}

impl std::fmt::Display for PlatformError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PlatformError::Windows => {
                write!(f, "Windows machines are not supported as a remote in v1")
            }
            PlatformError::Unsupported(said) => {
                write!(f, "unsupported remote platform \"{said}\"")
            }
            PlatformError::NoAnswer => write!(f, "the machine did not report its platform"),
        }
    }
}

impl std::error::Error for PlatformError {}

const MOST_PLATFORM_CHARS: usize = 40;

/// The platform from the probe's stdout: the line after the last marker line, `<kernel>
/// <machine>` as `uname -sm` prints it.
pub fn parse_platform(stdout: &str) -> Result<Platform, PlatformError> {
    let lines: Vec<&str> = stdout.lines().map(str::trim).collect();
    let marker = lines
        .iter()
        .rposition(|line| *line == PLATFORM_MARKER)
        .ok_or(PlatformError::NoAnswer)?;
    let answer = lines[marker + 1..]
        .iter()
        .find(|line| !line.is_empty())
        .ok_or(PlatformError::NoAnswer)?;
    let mut words = answer.split_whitespace();
    let (kernel, machine) = (words.next().unwrap_or(""), words.next().unwrap_or(""));
    if is_windows_kernel(kernel) {
        return Err(PlatformError::Windows);
    }
    let os = match kernel {
        "Linux" => "linux",
        "Darwin" => "macos",
        _ => "",
    };
    let arch = match machine {
        "x86_64" | "amd64" => "x86_64",
        "aarch64" | "arm64" => "aarch64",
        _ => "",
    };
    from_rust_names(os, arch).ok_or_else(|| {
        let said = sanitize(answer, false);
        PlatformError::Unsupported(said.chars().take(MOST_PLATFORM_CHARS).collect())
    })
}

fn is_windows_kernel(kernel: &str) -> bool {
    let kernel = kernel.to_ascii_uppercase();
    ["MINGW", "CYGWIN", "MSYS", "WINDOWS"]
        .iter()
        .any(|prefix| kernel.starts_with(prefix))
}

/// What running the installed app's `--version` there showed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Installed {
    /// The shell found no such command (status 127, not a reading of its words).
    Missing,
    /// The version it reports, e.g. `0.58.0`.
    Version(String),
    /// It is there but did not answer as the app: why.
    Broken(String),
}

/// `code` is the exit status of `<remote_cmd> --version`; login text may precede the answer.
pub fn classify_version(code: Option<i32>, stdout: &str) -> Installed {
    match code {
        Some(127) => Installed::Missing,
        Some(0) => stdout
            .lines()
            .filter_map(|line| line.trim().strip_prefix(APP_COMMAND)?.strip_prefix(' '))
            .map(str::trim)
            .find(|version| parse(version).is_some())
            .map_or_else(
                || Installed::Broken(format!("it did not report a {APP_COMMAND} version")),
                |version| Installed::Version(sanitize(version, false)),
            ),
        Some(code) => Installed::Broken(format!("`--version` ended with status {code}")),
        None => Installed::Broken("`--version` was ended before it answered".into()),
    }
}

/// How the far side's install compares with this app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum State {
    Missing,
    TooOld { remote: String },
    Matching,
    Newer { remote: String },
}

/// `None` when the far side's version cannot be compared (it is `Broken`).
pub fn state_of(installed: &Installed, ours: &str) -> Option<State> {
    match installed {
        Installed::Missing => Some(State::Missing),
        Installed::Broken(_) => None,
        Installed::Version(remote) => {
            let (theirs, ours) = (parse(remote)?, parse(ours)?);
            Some(match theirs.cmp(&ours) {
                std::cmp::Ordering::Less => State::TooOld {
                    remote: remote.clone(),
                },
                std::cmp::Ordering::Equal => State::Matching,
                std::cmp::Ordering::Greater => State::Newer {
                    remote: remote.clone(),
                },
            })
        }
    }
}

/// What the person has already agreed to, and whether they can be asked.
#[derive(Debug, Clone, Copy, Default)]
pub struct Consent {
    /// `--install`: a first install is wanted.
    pub install: bool,
    /// `--yes`: replacing an older version is wanted.
    pub yes: bool,
    /// `--force`: replacing a newer version is wanted.
    pub force: bool,
    /// A person is at the keyboard to answer a question.
    pub can_ask: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ask {
    Install,
    Update,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Refusal {
    NeedsInstallFlag,
    NeedsYesFlag,
    RemoteIsNewer,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    UpToDate,
    Go,
    Ask(Ask),
    Refuse(Refusal),
}

/// Nothing is installed or replaced without a yes: a flag, or an answer when someone can give one.
pub fn decide(state: &State, consent: Consent) -> Decision {
    match state {
        State::Matching => Decision::UpToDate,
        State::Missing if consent.install => Decision::Go,
        State::Missing if consent.can_ask => Decision::Ask(Ask::Install),
        State::Missing => Decision::Refuse(Refusal::NeedsInstallFlag),
        State::TooOld { .. } if consent.yes => Decision::Go,
        State::TooOld { .. } if consent.can_ask => Decision::Ask(Ask::Update),
        State::TooOld { .. } => Decision::Refuse(Refusal::NeedsYesFlag),
        State::Newer { .. } if consent.force => Decision::Go,
        State::Newer { .. } => Decision::Refuse(Refusal::RemoteIsNewer),
    }
}

impl Refusal {
    pub fn message(&self, machine: &str) -> String {
        match self {
            Refusal::NeedsInstallFlag => format!(
                "{APP_COMMAND} is not installed on {machine}; run `{APP_COMMAND} machine update {machine} --install`"
            ),
            Refusal::NeedsYesFlag => format!(
                "{machine} has an older {APP_COMMAND}; run `{APP_COMMAND} machine update {machine} --yes` to replace it"
            ),
            Refusal::RemoteIsNewer => format!(
                "{machine} has a newer {APP_COMMAND} than this one; update this computer, or run `{APP_COMMAND} machine update {machine} --force` to downgrade it"
            ),
        }
    }
}

/// Where the app goes on a machine: the path its `remote_cmd` names when that is absolute or
/// under `~/` (the person chose where it lives), else the default folder.
pub fn destination(remote_cmd: &str) -> String {
    if remote_cmd.starts_with('/') || remote_cmd.starts_with("~/") {
        remote_cmd.to_string()
    } else {
        DEFAULT_DESTINATION.to_string()
    }
}

/// Whether `remote_cmd` finds the app at `destination` without a PATH lookup.
pub fn names_destination(remote_cmd: &str) -> bool {
    destination(remote_cmd) == remote_cmd
}

/// Why the install script did not finish, from its exit status.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UploadError {
    ByteCount,
    WontRun,
    CannotWrite,
    /// ssh itself failed (status 255): the machine was not reached, or the link dropped.
    Ssh,
    Other(Option<i32>),
    /// The script ended well but did not say so.
    NoOk,
}

impl UploadError {
    pub fn reason(&self) -> String {
        match self {
            UploadError::ByteCount => {
                "the file that arrived was not the size that was sent".to_string()
            }
            UploadError::WontRun => format!(
                "the new {APP_COMMAND} did not run on that machine (wrong system or a missing library)"
            ),
            UploadError::CannotWrite => "could not write to the install folder there".to_string(),
            UploadError::Ssh => "ssh could not keep the connection open".to_string(),
            UploadError::Other(Some(code)) => {
                format!("the install script ended with status {code}")
            }
            UploadError::Other(None) => "the install script was ended early".to_string(),
            UploadError::NoOk => "the install script did not report success".to_string(),
        }
    }

    /// The reason, then what that means for the machine.
    pub fn line(&self, machine: &str, had_old: bool) -> String {
        let left = if had_old {
            "the old version is unchanged"
        } else {
            "nothing was installed"
        };
        format!(
            "{}; the install on {machine} did not finish; {left}",
            self.reason()
        )
    }
}

/// Whether the install script finished: status 0 and its last line `OK`.
pub fn upload_result(code: Option<i32>, stdout: &str) -> Result<(), UploadError> {
    match code {
        Some(0) if stdout.lines().rev().find(|l| !l.trim().is_empty()) == Some("OK") => Ok(()),
        Some(0) => Err(UploadError::NoOk),
        Some(EXIT_BYTE_COUNT) => Err(UploadError::ByteCount),
        Some(EXIT_WONT_RUN) => Err(UploadError::WontRun),
        Some(EXIT_CANNOT_WRITE) => Err(UploadError::CannotWrite),
        Some(255) => Err(UploadError::Ssh),
        other => Err(UploadError::Other(other)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn platform(os: Os, arch: Arch) -> Platform {
        Platform { os, arch }
    }

    #[test]
    fn the_platform_comes_from_the_line_after_the_marker() {
        let table = [
            ("maitty-platform\nLinux x86_64\n", Os::Linux, Arch::X86_64),
            ("maitty-platform\nLinux aarch64\n", Os::Linux, Arch::Aarch64),
            ("maitty-platform\nDarwin arm64\n", Os::Macos, Arch::Aarch64),
            ("maitty-platform\nDarwin x86_64\n", Os::Macos, Arch::X86_64),
            ("maitty-platform\nLinux amd64\n", Os::Linux, Arch::X86_64),
            (
                "Last login: Mon\nWelcome!\nLinux x86_64\nmaitty-platform\nDarwin arm64\nbye\n",
                Os::Macos,
                Arch::Aarch64,
            ),
            (
                "maitty-platform\r\nLinux x86_64\r\n",
                Os::Linux,
                Arch::X86_64,
            ),
        ];
        for (out, os, arch) in table {
            assert_eq!(parse_platform(out), Ok(platform(os, arch)), "{out:?}");
        }
    }

    #[test]
    fn a_windows_like_kernel_is_not_a_remote_in_v1() {
        for kernel in [
            "MINGW64_NT-10.0-26300",
            "CYGWIN_NT-10.0",
            "MSYS_NT-10.0",
            "Windows_NT",
        ] {
            let error = parse_platform(&format!("maitty-platform\n{kernel} x86_64\n"));
            assert_eq!(error, Err(PlatformError::Windows), "{kernel}");
        }
        assert_eq!(
            PlatformError::Windows.to_string(),
            "Windows machines are not supported as a remote in v1"
        );
    }

    #[test]
    fn anything_else_is_unsupported_and_shown_cleaned() {
        assert_eq!(
            parse_platform("maitty-platform\nFreeBSD amd64\n")
                .unwrap_err()
                .to_string(),
            "unsupported remote platform \"FreeBSD amd64\""
        );
        assert_eq!(
            parse_platform("maitty-platform\nLinux riscv64\n")
                .unwrap_err()
                .to_string(),
            "unsupported remote platform \"Linux riscv64\""
        );
        let shown = parse_platform("maitty-platform\nPlan9 \u{1b}[31mx\u{7}\n")
            .unwrap_err()
            .to_string();
        assert!(!shown.chars().any(char::is_control), "{shown:?}");
        let long = format!("maitty-platform\nBeOS {}\n", "z".repeat(500));
        assert!(parse_platform(&long).unwrap_err().to_string().len() < 100);
    }

    #[test]
    fn no_marker_or_nothing_after_it_is_no_answer() {
        for out in [
            "",
            "Linux x86_64\n",
            "maitty-platform\n",
            "maitty-platform\n\n  \n",
        ] {
            assert_eq!(parse_platform(out), Err(PlatformError::NoAnswer), "{out:?}");
        }
    }

    #[test]
    fn the_probe_script_prints_its_marker_then_asks_uname() {
        assert!(PLATFORM_SCRIPT.contains(PLATFORM_MARKER));
        assert!(PLATFORM_SCRIPT.ends_with("uname -sm"));
    }

    #[test]
    fn this_computer_is_a_twin_only_on_linux_or_macos() {
        assert_eq!(from_rust_names("windows", "x86_64"), None);
        assert_eq!(from_rust_names("linux", "riscv64"), None);
        assert_eq!(
            from_rust_names("linux", "aarch64"),
            Some(platform(Os::Linux, Arch::Aarch64))
        );
        assert_eq!(
            platform(Os::Macos, Arch::Aarch64).to_string(),
            "macos aarch64"
        );
    }

    #[test]
    fn a_missing_install_is_the_shell_s_status_not_its_words() {
        assert_eq!(classify_version(Some(127), ""), Installed::Missing);
        assert_eq!(
            classify_version(Some(127), "totally different words"),
            Installed::Missing
        );
        assert!(matches!(
            classify_version(Some(1), "bash: maitty: command not found"),
            Installed::Broken(_)
        ));
        assert!(matches!(classify_version(None, ""), Installed::Broken(_)));
    }

    #[test]
    fn the_version_is_found_after_login_noise() {
        assert_eq!(
            classify_version(Some(0), "Welcome\nmaitty 0.58.0\n"),
            Installed::Version("0.58.0".into())
        );
        assert!(matches!(
            classify_version(Some(0), "Welcome\nsomething 1.0.0\n"),
            Installed::Broken(_)
        ));
        assert!(matches!(
            classify_version(Some(0), "maitty banana\n"),
            Installed::Broken(_)
        ));
    }

    fn state(remote: &str) -> Option<State> {
        state_of(&Installed::Version(remote.into()), "0.61.0")
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(
            state("0.58.0"),
            Some(State::TooOld {
                remote: "0.58.0".into()
            })
        );
        assert_eq!(
            state("0.9.0"),
            Some(State::TooOld {
                remote: "0.9.0".into()
            })
        );
        assert_eq!(state("0.61.0"), Some(State::Matching));
        assert_eq!(
            state("0.100.0"),
            Some(State::Newer {
                remote: "0.100.0".into()
            })
        );
        assert_eq!(
            state_of(&Installed::Missing, "0.61.0"),
            Some(State::Missing)
        );
        assert_eq!(state_of(&Installed::Broken("x".into()), "0.61.0"), None);
    }

    fn consent(install: bool, yes: bool, force: bool, can_ask: bool) -> Consent {
        Consent {
            install,
            yes,
            force,
            can_ask,
        }
    }

    #[test]
    fn nothing_is_installed_or_replaced_without_a_yes() {
        let old = State::TooOld {
            remote: "0.1.0".into(),
        };
        let newer = State::Newer {
            remote: "9.0.0".into(),
        };
        let rows = [
            (
                State::Matching,
                consent(false, false, false, false),
                Decision::UpToDate,
            ),
            (
                State::Matching,
                consent(true, true, true, true),
                Decision::UpToDate,
            ),
            (
                State::Missing,
                consent(false, false, false, false),
                Decision::Refuse(Refusal::NeedsInstallFlag),
            ),
            (
                State::Missing,
                consent(false, true, false, false),
                Decision::Refuse(Refusal::NeedsInstallFlag),
            ),
            (
                State::Missing,
                consent(false, false, false, true),
                Decision::Ask(Ask::Install),
            ),
            (
                State::Missing,
                consent(true, false, false, false),
                Decision::Go,
            ),
            (
                old.clone(),
                consent(false, false, false, false),
                Decision::Refuse(Refusal::NeedsYesFlag),
            ),
            (
                old.clone(),
                consent(true, false, false, false),
                Decision::Refuse(Refusal::NeedsYesFlag),
            ),
            (
                old.clone(),
                consent(false, false, false, true),
                Decision::Ask(Ask::Update),
            ),
            (old, consent(false, true, false, false), Decision::Go),
            (
                newer.clone(),
                consent(false, true, false, true),
                Decision::Refuse(Refusal::RemoteIsNewer),
            ),
            (newer, consent(false, false, true, false), Decision::Go),
        ];
        for (state, consent, want) in rows {
            assert_eq!(decide(&state, consent), want, "{state:?} {consent:?}");
        }
    }

    #[test]
    fn the_refusals_name_the_flag_that_would_do_it() {
        assert_eq!(
            Refusal::NeedsInstallFlag.message("build"),
            "maitty is not installed on build; run `maitty machine update build --install`"
        );
        assert!(Refusal::NeedsYesFlag.message("build").contains("--yes"));
        assert!(Refusal::RemoteIsNewer.message("build").contains("--force"));
    }

    #[test]
    fn the_destination_follows_remote_cmd_when_it_names_a_place() {
        let rows = [
            ("maitty", "~/.local/bin/maitty"),
            ("bin/maitty", "~/.local/bin/maitty"),
            ("./maitty", "~/.local/bin/maitty"),
            ("/opt/maitty/bin/maitty", "/opt/maitty/bin/maitty"),
            ("~/bin/maitty", "~/bin/maitty"),
            ("/opt/my tools/it's/maitty", "/opt/my tools/it's/maitty"),
            ("~/my tools/maitty", "~/my tools/maitty"),
            ("~root/maitty", "~/.local/bin/maitty"),
        ];
        for (cmd, want) in rows {
            assert_eq!(destination(cmd), want, "{cmd}");
        }
        assert!(names_destination("/opt/maitty") && !names_destination("maitty"));
    }

    #[test]
    fn the_script_is_a_constant_with_no_place_for_user_data() {
        for placeholder in ['{', '}'] {
            assert!(!INSTALL_SCRIPT.contains(placeholder), "{placeholder}");
            assert!(!PLATFORM_SCRIPT.contains(placeholder), "{placeholder}");
        }
        assert!(INSTALL_SCRIPT.contains("dest=$1") && INSTALL_SCRIPT.contains("want=$2"));
    }

    #[test]
    fn the_script_replaces_by_rename_and_never_signals_a_process() {
        for word in ["kill", "pkill", "killall", "pgrep", "systemctl", "rm -rf"] {
            assert!(!INSTALL_SCRIPT.contains(word), "{word}");
        }
        let swap = INSTALL_SCRIPT.find("mv -f -- \"$tmp\" \"$dest\"").unwrap();
        let ran = INSTALL_SCRIPT.find("--version").unwrap();
        let counted = INSTALL_SCRIPT.find("wc -c").unwrap();
        assert!(counted < ran && ran < swap, "check size, run, then swap");
        assert!(INSTALL_SCRIPT.contains("chmod 700") && INSTALL_SCRIPT.contains("chmod 755"));
        assert!(
            INSTALL_SCRIPT.find("chmod 755").unwrap() > ran,
            "others get to execute it only after it has run"
        );
    }

    #[test]
    fn the_script_exits_with_the_codes_the_client_reads() {
        for code in [EXIT_BYTE_COUNT, EXIT_WONT_RUN, EXIT_CANNOT_WRITE] {
            assert!(INSTALL_SCRIPT.contains(&format!("exit {code}")), "{code}");
        }
    }

    #[test]
    fn each_exit_status_has_its_own_message() {
        assert_eq!(upload_result(Some(0), "OK\n"), Ok(()));
        assert_eq!(upload_result(Some(0), "Welcome\nOK\n"), Ok(()));
        assert_eq!(upload_result(Some(0), "fine\n"), Err(UploadError::NoOk));
        assert_eq!(upload_result(Some(70), ""), Err(UploadError::ByteCount));
        assert_eq!(upload_result(Some(71), ""), Err(UploadError::WontRun));
        assert_eq!(upload_result(Some(72), ""), Err(UploadError::CannotWrite));
        assert_eq!(upload_result(Some(255), ""), Err(UploadError::Ssh));
        assert_eq!(upload_result(Some(3), ""), Err(UploadError::Other(Some(3))));
        assert_eq!(upload_result(None, ""), Err(UploadError::Other(None)));
        let reasons: std::collections::HashSet<_> = [
            UploadError::ByteCount,
            UploadError::WontRun,
            UploadError::CannotWrite,
            UploadError::Ssh,
            UploadError::NoOk,
            UploadError::Other(Some(3)),
        ]
        .iter()
        .map(UploadError::reason)
        .collect();
        assert_eq!(reasons.len(), 6);
    }

    #[test]
    fn a_failure_says_what_became_of_the_machine() {
        assert_eq!(
            UploadError::ByteCount.line("build", true),
            "the file that arrived was not the size that was sent; the install on build did not finish; the old version is unchanged"
        );
        assert!(
            UploadError::WontRun
                .line("build", false)
                .ends_with("nothing was installed")
        );
    }
}
