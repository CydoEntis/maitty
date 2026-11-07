//! The fixed scripts the ssh lines run on another machine. They are constants: the destination
//! and the byte count reach the install script as positional arguments, never as part of its text.

/// Run on the far side as `sh -c <this> sh`.
pub const PLATFORM_SCRIPT: &str = "printf '%s\\n' maitty-platform; uname -sm";

/// Where the far side's login shell finds the app, for `doctor`: a fixed script, so nothing a
/// person or a machine supplies can reach the remote shell.
pub const WHERE_SCRIPT: &str = concat!("command -v ", env!("APP_COMMAND"));

/// `$1` is the destination (a leading `~/` is the far side's home), `$2` the byte count
/// expected on stdin. The new file is written beside the destination so the final `mv` is a
/// rename, and is made executable by its owner alone until it has run `--version`.
pub const INSTALL_SCRIPT: &str = concat!(
    "umask 077\n",
    "dest=$1\n",
    "want=$2\n",
    "case $dest in\n",
    "  \"~/\"*) dest=$HOME/$(printf '%s' \"$dest\" | cut -c3-) ;;\n",
    "esac\n",
    "[ ! -d \"$dest\" ] || exit 72\n",
    "dir=$(dirname -- \"$dest\")\n",
    "mkdir -p -- \"$dir\" || exit 72\n",
    "tmp=$(mktemp \"$dir/.",
    env!("APP_COMMAND"),
    "-new.XXXXXX\") || exit 72\n",
    "trap 'rm -f -- \"$tmp\"' EXIT\n",
    "trap 'exit 72' HUP INT TERM\n",
    "cat > \"$tmp\" || exit 72\n",
    "got=$(wc -c < \"$tmp\" | tr -d ' ')\n",
    "[ \"$got\" = \"$want\" ] || exit 70\n",
    "chmod 700 -- \"$tmp\" || exit 72\n",
    "case $(\"$tmp\" --version 2>/dev/null) in\n",
    "  \"",
    env!("APP_COMMAND"),
    " \"*) ;;\n",
    "  *) exit 71 ;;\n",
    "esac\n",
    "chmod 755 -- \"$tmp\" || exit 72\n",
    "mv -f -- \"$tmp\" \"$dest\" || exit 72\n",
    "echo OK\n",
);

/// The line the get script prints before the file's size, so text a login shell prints first
/// (a banner, a message of the day) is never taken for the file.
pub const GET_MARKER: &str = concat!(env!("APP_COMMAND"), "-get");

/// Exit statuses of `GET_SCRIPT`.
pub const GET_MISSING: i32 = 73;
pub const GET_NOT_A_FILE: i32 = 74;
pub const GET_UNREADABLE: i32 = 75;
pub const GET_TOO_BIG: i32 = 76;
pub const GET_READ_FAILED: i32 = 77;

/// Prints one regular file from the far side: `GET_MARKER`, its size in bytes, then exactly
/// that many bytes. `$1` is the path (a leading `~/` is the far side's home), `$2` the most
/// bytes it may have. It reads and signals nothing else, and writes nothing.
pub const GET_SCRIPT: &str = concat!(
    "path=$1\n",
    "max=$2\n",
    "case $path in\n",
    "  \"~/\"*) path=$HOME/$(printf '%s' \"$path\" | cut -c3-) ;;\n",
    "esac\n",
    "[ -e \"$path\" ] || exit 73\n",
    "[ -f \"$path\" ] || exit 74\n",
    "[ -r \"$path\" ] || exit 75\n",
    "size=$(wc -c < \"$path\" | tr -d ' ') || exit 75\n",
    "[ \"$size\" -le \"$max\" ] || exit 76\n",
    "printf '%s\\n%s\\n' '",
    env!("APP_COMMAND"),
    "-get' \"$size\"\n",
    "cat -- \"$path\" || exit 77\n",
);

/// The line the paste script prints before the path, so text a login shell prints first (a
/// banner, a message of the day) is never taken for it.
pub const PASTE_MARKER: &str = concat!(env!("APP_COMMAND"), "-paste");

/// Keeps a pasted image or a sent file (ADR 0007, 0055) on the far side. `$1` is the file's name (letters, digits, `.`, `_`
/// and `-`, not starting with a dot), `$2` the byte count expected on stdin. The file goes in
/// the app's folder in the far side's cache, which must belong to the user running the script
/// (a folder someone else owns is refused), readable by its owner alone, and is written beside
/// its place and renamed into it so a half-sent file is never there. Pastes older than a week
/// are cleared out, and so are half-sent files a killed upload left behind (older than a day). It
/// prints `PASTE_MARKER` and then the file's absolute path.
pub const PASTE_SCRIPT: &str = concat!(
    "umask 077\n",
    "name=$1\n",
    "want=$2\n",
    "case $name in\n",
    "  ''|.*|*[!A-Za-z0-9._-]*) exit 72 ;;\n",
    "esac\n",
    "case ${XDG_CACHE_HOME:-} in\n",
    "  /*) base=$XDG_CACHE_HOME ;;\n",
    "  *) base=${HOME:-}/.cache ;;\n",
    "esac\n",
    "case $base in\n",
    "  /*) ;;\n",
    "  *) exit 72 ;;\n",
    "esac\n",
    "dir=$base/",
    env!("APP_COMMAND"),
    "/pastes\n",
    "mkdir -p -- \"$dir\" || exit 72\n",
    "[ -O \"$dir\" ] || exit 72\n",
    "find \"$dir\" -type f -name 'paste-*' -mtime +7 -exec rm -f {} + 2>/dev/null\n",
    "find \"$dir\" -type f -name '.new.*' -mtime +1 -exec rm -f {} + 2>/dev/null\n",
    "tmp=$(mktemp \"$dir/.new.XXXXXX\") || exit 72\n",
    "trap 'rm -f -- \"$tmp\"' EXIT\n",
    "trap 'exit 72' HUP INT TERM\n",
    "cat > \"$tmp\" || exit 72\n",
    "got=$(wc -c < \"$tmp\" | tr -d ' ')\n",
    "[ \"$got\" = \"$want\" ] || exit 70\n",
    "mv -f -- \"$tmp\" \"$dir/$name\" || exit 72\n",
    "printf '%s\\n%s\\n' '",
    env!("APP_COMMAND"),
    "-paste' \"$dir/$name\"\n",
);

#[cfg(test)]
mod get_script_tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn has_sh() -> bool {
        Command::new("sh").arg("-c").arg("true").status().is_ok_and(|s| s.success())
    }

    fn dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("get-script-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn run(dir: &std::path::Path, path: &str, max: &str) -> (Option<i32>, Vec<u8>) {
        let out = Command::new("sh")
            .current_dir(dir)
            .args(["-c", GET_SCRIPT, "sh", path, max])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (out.status.code(), out.stdout)
    }

    #[test]
    fn the_script_is_a_constant_that_reads_one_file_and_changes_nothing() {
        for placeholder in ['{', '}'] {
            assert!(!GET_SCRIPT.contains(placeholder), "{placeholder}");
        }
        for word in ["kill", "pkill", "killall", "pgrep", "systemctl", "rm ", "mv ", "> "] {
            assert!(!GET_SCRIPT.contains(word), "{word}");
        }
        assert!(GET_SCRIPT.contains("path=$1") && GET_SCRIPT.contains("max=$2"));
        let checked = GET_SCRIPT.find("-le \"$max\"").unwrap();
        let sent = GET_SCRIPT.find("cat --").unwrap();
        assert!(checked < sent, "the size is checked before a byte is sent");
    }

    #[test]
    fn a_file_comes_out_as_the_marker_its_size_and_exactly_its_bytes() {
        if !has_sh() {
            return;
        }
        let dir = dir("whole");
        std::fs::write(dir.join("out.log"), b"line one\nline two\n").unwrap();
        let (code, stdout) = run(&dir, "out.log", "1000");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(code, Some(0));
        assert_eq!(
            String::from_utf8(stdout).unwrap().replace("\r\n", "\n"),
            format!("{GET_MARKER}\n18\nline one\nline two\n")
        );
    }

    #[test]
    fn a_name_that_starts_with_a_dash_is_a_file_not_an_option() {
        if !has_sh() {
            return;
        }
        let dir = dir("dash");
        std::fs::write(dir.join("-n"), b"x").unwrap();
        let (code, stdout) = run(&dir, "-n", "10");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(code, Some(0));
        assert!(String::from_utf8_lossy(&stdout).ends_with('x'));
    }

    #[test]
    fn each_refusal_has_its_own_status_and_prints_no_file_bytes() {
        if !has_sh() {
            return;
        }
        let dir = dir("refuse");
        std::fs::write(dir.join("big"), vec![b'a'; 50]).unwrap();
        std::fs::create_dir(dir.join("folder")).unwrap();
        let cases = [
            ("nope", "100", GET_MISSING),
            ("folder", "100", GET_NOT_A_FILE),
            ("big", "49", GET_TOO_BIG),
        ];
        let got: Vec<_> = cases
            .iter()
            .map(|(path, max, _)| run(&dir, path, max))
            .collect();
        let _ = std::fs::remove_dir_all(&dir);
        for ((code, stdout), (path, _, want)) in got.iter().zip(cases) {
            assert_eq!(*code, Some(want), "{path}");
            assert!(stdout.is_empty(), "{path}: nothing was printed");
        }
    }
}
