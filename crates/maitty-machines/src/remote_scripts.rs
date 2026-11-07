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

/// The line the list script prints first, so text a login shell prints before it is never taken
/// for the listing. The folders follow the folder's own path, and this line (ending in a NUL)
/// stands between them and the other entries.
pub const LIST_MARKER: &str = concat!(env!("APP_COMMAND"), "-list");
pub const LIST_FILES_MARKER: &str = concat!(env!("APP_COMMAND"), "-list-files");

/// Exit statuses of `LIST_SCRIPT`.
pub const LIST_MISSING: i32 = 73;
pub const LIST_NOT_A_FOLDER: i32 = 74;
pub const LIST_UNREADABLE: i32 = 75;

/// Prints what is in one folder of the far side, and changes nothing. `$1` is the folder (empty
/// or `~` is the far side's home, a leading `~/` is below it), `$2` the most bytes it may print.
/// Output, at most `$2` bytes in all: `LIST_MARKER` and a line break, the folder's resolved path
/// and a NUL, each sub-folder's full path and a NUL, `LIST_FILES_MARKER` and a NUL, then every
/// other entry's full path and a NUL.
pub const LIST_SCRIPT: &str = concat!(
    "dir=$1\n",
    "max=$2\n",
    "case $dir in\n",
    "  '') dir=$HOME ;;\n",
    "  '~') dir=$HOME ;;\n",
    "  \"~/\"*) dir=$HOME/$(printf '%s' \"$dir\" | cut -c3-) ;;\n",
    "esac\n",
    "[ -e \"$dir\" ] || exit 73\n",
    "[ -d \"$dir\" ] || exit 74\n",
    "[ -r \"$dir\" ] || exit 75\n",
    "dir=$(cd -- \"$dir\" && pwd -P) || exit 75\n",
    "( printf '%s\\n' '",
    env!("APP_COMMAND"),
    "-list'\n",
    "  printf '%s\\0' \"$dir\"\n",
    "  find \"$dir\" -mindepth 1 -maxdepth 1 -type d -print0\n",
    "  printf '",
    env!("APP_COMMAND"),
    "-list-files\\0'\n",
    "  find \"$dir\" -mindepth 1 -maxdepth 1 ! -type d -print0 ) | head -c \"$max\"\n",
);

/// The line the index script prints first (see `LIST_MARKER`).
pub const INDEX_MARKER: &str = concat!(env!("APP_COMMAND"), "-index");

/// Exit statuses of `INDEX_SCRIPT`.
pub const INDEX_MISSING: i32 = 73;
pub const INDEX_NOT_A_FOLDER: i32 = 74;
pub const INDEX_UNREADABLE: i32 = 75;

/// The deepest level below the folder whose files the index lists.
pub const INDEX_DEPTH: u32 = 6;

/// Prints the paths of the regular files below one folder of the far side, for a search, and
/// changes nothing. `$1` is the folder (empty or `~` is the far side's home, a leading `~/` is
/// below it), `$2` the most bytes it may print. Folders that are never worth searching (`.git`,
/// `node_modules`, build output, caches) are skipped, symlinks are not followed, and nothing
/// deeper than `INDEX_DEPTH` is listed. Output, at most `$2` bytes in all: `INDEX_MARKER` and a
/// line break, the folder's resolved path and a NUL, then each file's full path and a NUL.
pub const INDEX_SCRIPT: &str = concat!(
    "dir=$1\n",
    "max=$2\n",
    "case $dir in\n",
    "  '') dir=$HOME ;;\n",
    "  '~') dir=$HOME ;;\n",
    "  \"~/\"*) dir=$HOME/$(printf '%s' \"$dir\" | cut -c3-) ;;\n",
    "esac\n",
    "[ -e \"$dir\" ] || exit 73\n",
    "[ -d \"$dir\" ] || exit 74\n",
    "[ -r \"$dir\" ] || exit 75\n",
    "dir=$(cd -- \"$dir\" && pwd -P) || exit 75\n",
    "( printf '%s\\n' '",
    env!("APP_COMMAND"),
    "-index'\n",
    "  printf '%s\\0' \"$dir\"\n",
    "  find \"$dir\" -mindepth 1 -maxdepth 6 ",
    "\\( -name .git -o -name node_modules -o -name target -o -name dist -o -name build ",
    "-o -name .cache -o -name __pycache__ -o -name .venv \\) -prune -o -type f -print0 ",
    ") | head -c \"$max\"\n",
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

#[cfg(test)]
mod list_script_tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn has_sh() -> bool {
        Command::new("sh")
            .args(["-c", "command -v find head cut printf cd pwd >/dev/null"])
            .status()
            .is_ok_and(|s| s.success())
    }

    fn tree(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("list-script-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("a")).unwrap();
        std::fs::create_dir_all(dir.join("b c")).unwrap();
        std::fs::write(dir.join("x.txt"), b"x").unwrap();
        std::fs::write(dir.join(".hidden"), b"h").unwrap();
        std::fs::write(dir.join("-dash"), b"d").unwrap();
        dir
    }

    fn run(dir: &std::path::Path, folder: &str, max: &str) -> (Option<i32>, Vec<u8>) {
        let out = Command::new("sh")
            .current_dir(dir)
            .args(["-c", LIST_SCRIPT, "sh", folder, max])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (out.status.code(), out.stdout)
    }

    /// The pieces after the marker line, split at each NUL.
    fn pieces(stdout: &[u8]) -> Vec<String> {
        let text = String::from_utf8_lossy(stdout).replace("\r\n", "\n");
        let after = text.strip_prefix(&format!("{LIST_MARKER}\n")).expect("the marker line comes first");
        after.split('\0').filter(|p| !p.is_empty()).map(str::to_owned).collect()
    }

    #[test]
    fn the_script_is_a_constant_that_only_reads() {
        for placeholder in ['{', '}'] {
            assert!(!LIST_SCRIPT.contains(placeholder), "{placeholder}");
        }
        for word in ["kill", "pkill", "killall", "pgrep", "systemctl", "rm ", "mv ", "> ", "-delete", "-exec"] {
            assert!(!LIST_SCRIPT.contains(word), "{word}");
        }
        assert!(LIST_SCRIPT.contains("dir=$1") && LIST_SCRIPT.contains("max=$2"));
        assert!(!LIST_SCRIPT.contains("-printf"), "BSD find has no -printf");
    }

    #[test]
    fn a_folder_lists_its_subfolders_then_the_marker_then_the_rest_with_hidden_files() {
        if !has_sh() {
            return;
        }
        let dir = tree("whole");
        let (code, stdout) = run(&dir, ".", "100000");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(code, Some(0));
        let got = pieces(&stdout);
        let folder = got[0].clone();
        let marker = got.iter().position(|p| p == LIST_FILES_MARKER).expect("the files marker");
        let mut dirs: Vec<_> = got[1..marker].iter().map(|p| p.strip_prefix(&format!("{folder}/")).unwrap().to_owned()).collect();
        let mut rest: Vec<_> = got[marker + 1..].iter().map(|p| p.strip_prefix(&format!("{folder}/")).unwrap().to_owned()).collect();
        dirs.sort();
        rest.sort();
        assert_eq!(dirs, ["a", "b c"]);
        assert_eq!(rest, ["-dash", ".hidden", "x.txt"]);
    }

    #[test]
    fn each_refusal_has_its_own_status_and_prints_no_listing() {
        if !has_sh() {
            return;
        }
        let dir = tree("refuse");
        let cases = [("nope", LIST_MISSING), ("x.txt", LIST_NOT_A_FOLDER)];
        let got: Vec<_> = cases.iter().map(|(folder, _)| run(&dir, folder, "1000")).collect();
        let _ = std::fs::remove_dir_all(&dir);
        for ((code, stdout), (folder, want)) in got.iter().zip(cases) {
            assert_eq!(*code, Some(want), "{folder}");
            assert!(stdout.is_empty(), "{folder}: nothing was printed");
        }
    }

    #[test]
    fn the_byte_limit_cuts_the_listing() {
        if !has_sh() {
            return;
        }
        let dir = tree("cap");
        let (_, stdout) = run(&dir, ".", "30");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(stdout.len() <= 30, "{} bytes", stdout.len());
    }

    #[test]
    fn an_empty_folder_argument_means_the_home_folder() {
        if !has_sh() {
            return;
        }
        let dir = tree("home");
        let out = Command::new("sh")
            .current_dir(&dir)
            .env("HOME", ".")
            .args(["-c", LIST_SCRIPT, "sh", "", "100000"])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(out.status.code(), Some(0));
        assert!(pieces(&out.stdout).iter().any(|p| p.ends_with("/x.txt")));
    }
}

#[cfg(test)]
mod index_script_tests {
    use super::*;
    use std::process::{Command, Stdio};

    fn has_sh() -> bool {
        Command::new("sh")
            .args(["-c", "command -v find head cut printf cd pwd >/dev/null"])
            .status()
            .is_ok_and(|s| s.success())
    }

    fn put(root: &std::path::Path, relative: &str) {
        let path = root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, b"x").unwrap();
    }

    fn tree(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("index-script-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        for file in [
            "top.txt",
            "src/a.rs",
            "src/deep/b.rs",
            ".git/config",
            "node_modules/pkg/index.js",
            "app/target/debug/out",
            "dist/bundle.js",
            "d1/d2/d3/d4/d5/six.txt",
            "d1/d2/d3/d4/d5/d6/seven.txt",
        ] {
            put(&dir, file);
        }
        dir
    }

    fn run(dir: &std::path::Path, folder: &str, max: &str) -> (Option<i32>, Vec<u8>) {
        let out = Command::new("sh")
            .current_dir(dir)
            .args(["-c", INDEX_SCRIPT, "sh", folder, max])
            .stdin(Stdio::null())
            .output()
            .unwrap();
        (out.status.code(), out.stdout)
    }

    /// The paths after the marker line and the folder, relative to the folder.
    fn files(stdout: &[u8]) -> Vec<String> {
        let text = String::from_utf8_lossy(stdout).replace("\r\n", "\n");
        let after = text.strip_prefix(&format!("{INDEX_MARKER}\n")).expect("the marker line comes first");
        let mut parts = after.split('\0').filter(|p| !p.is_empty());
        let folder = parts.next().expect("the folder comes second").to_owned();
        let mut got: Vec<String> = parts
            .map(|p| p.strip_prefix(&format!("{folder}/")).unwrap_or(p).to_owned())
            .collect();
        got.sort();
        got
    }

    #[test]
    fn the_script_is_a_constant_that_only_reads() {
        for placeholder in ['{', '}'] {
            assert!(!INDEX_SCRIPT.contains(placeholder), "{placeholder}");
        }
        for word in ["kill", "pkill", "killall", "pgrep", "systemctl", "rm ", "mv ", "> ", "-delete", "-exec", " -L"] {
            assert!(!INDEX_SCRIPT.contains(word), "{word}");
        }
        assert!(INDEX_SCRIPT.contains(&format!("-maxdepth {INDEX_DEPTH}")));
    }

    #[test]
    fn files_are_listed_but_not_the_ones_in_folders_never_worth_searching() {
        if !has_sh() {
            return;
        }
        let dir = tree("skip");
        let (code, stdout) = run(&dir, ".", "100000");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(code, Some(0));
        assert_eq!(
            files(&stdout),
            ["d1/d2/d3/d4/d5/six.txt", "src/a.rs", "src/deep/b.rs", "top.txt"],
            "no .git, node_modules, target or dist, and nothing past six levels"
        );
    }

    #[test]
    fn a_folder_with_a_skipped_name_is_still_indexed_itself() {
        if !has_sh() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("index-script-{}-root", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        put(&dir.join("target"), "inside.txt");
        let (code, stdout) = run(&dir.join("target"), ".", "100000");
        let _ = std::fs::remove_dir_all(&dir);
        assert_eq!(code, Some(0));
        assert_eq!(files(&stdout), ["inside.txt"]);
    }

    #[cfg(unix)]
    #[test]
    fn a_link_to_a_folder_elsewhere_is_not_followed() {
        if !has_sh() {
            return;
        }
        let dir = tree("link");
        let outside = std::env::temp_dir().join(format!("index-script-{}-outside", std::process::id()));
        put(&outside, "secret.txt");
        std::os::unix::fs::symlink(&outside, dir.join("elsewhere")).unwrap();
        let (_, stdout) = run(&dir, ".", "100000");
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&outside);
        assert!(!files(&stdout).iter().any(|f| f.contains("secret")));
    }

    #[test]
    fn each_refusal_has_its_own_status_and_prints_nothing() {
        if !has_sh() {
            return;
        }
        let dir = tree("refuse");
        let cases = [("nope", INDEX_MISSING), ("top.txt", INDEX_NOT_A_FOLDER)];
        let got: Vec<_> = cases.iter().map(|(folder, _)| run(&dir, folder, "1000")).collect();
        let _ = std::fs::remove_dir_all(&dir);
        for ((code, stdout), (folder, want)) in got.iter().zip(cases) {
            assert_eq!(*code, Some(want), "{folder}");
            assert!(stdout.is_empty(), "{folder}");
        }
    }

    #[test]
    fn the_byte_limit_cuts_the_index() {
        if !has_sh() {
            return;
        }
        let dir = tree("cap");
        let (_, stdout) = run(&dir, ".", "40");
        let _ = std::fs::remove_dir_all(&dir);
        assert!(stdout.len() <= 40, "{} bytes", stdout.len());
    }
}
