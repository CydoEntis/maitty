//! Sending a pasted image, or any file, to another machine (ADR 0049, 0050). The window cannot
//! put a picture or a file on a remote machine, so it goes over the same hardened ssh line the
//! installer uses, as a file the far side keeps, and the path it prints is typed into the pane.
//! Nothing here runs ssh; `Machine::paste_argv` builds the line and the caller runs it with the
//! file on stdin.

use crate::remote_install::PASTE_MARKER;
use maitty_shared::hidden::{is_invisible, sanitize};

/// The most bytes of one pasted image that are sent. A screenshot is a few megabytes.
pub const MAX_PASTE_BYTES: u64 = 25 * 1024 * 1024;

/// The most bytes of one file that is sent (ADR 0050).
pub const MAX_SEND_BYTES: u64 = 100 * 1024 * 1024;

/// The most characters of the original name kept in a sent file's name.
const ORIGINAL_MAX: usize = 48;

/// The longest path the far side may name for the file.
const PATH_MAX: usize = 4096;

/// The most characters of what ssh said that are shown in a toast.
const SAID_MAX: usize = 200;

/// The name of a pasted file: `stamp` is the time in milliseconds and `salt` tells two pastes
/// of the same millisecond apart. Only characters the paste script accepts.
pub fn paste_file_name(stamp: u128, salt: u32) -> String {
    format!("paste-{stamp}-{salt:08x}.png")
}

/// The name a sent file gets on the far side: the paste name's stamp and salt, then the original
/// name reduced to what the paste script accepts, so two files called `a.log` are two files and
/// nothing the person's file is called can become a path or an option.
pub fn sent_file_name(stamp: u128, salt: u32, original: &str) -> String {
    format!("paste-{stamp}-{salt:08x}-{}", clean_original(original))
}

/// `original` with every character the script would refuse replaced by `_`, leading dots
/// dropped, cut to `ORIGINAL_MAX` keeping the extension; `file` when nothing is left.
fn clean_original(original: &str) -> String {
    let plain: String = original
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let plain = plain.trim_start_matches('.');
    if plain.is_empty() {
        return "file".into();
    }
    if plain.len() <= ORIGINAL_MAX {
        return plain.to_owned();
    }
    // An extension longer than a few characters is not one; cut the whole name then.
    let ext = plain
        .rsplit_once('.')
        .map(|(_, ext)| ext)
        .filter(|ext| ext.len() <= 8)
        .map_or(String::new(), |ext| format!(".{ext}"));
    let stem: String = plain.chars().take(ORIGINAL_MAX - ext.len()).collect();
    format!("{stem}{ext}")
}

/// The path the paste script printed: the line after its marker. The far side is someone else's
/// computer, so it must be an absolute path with nothing in it that is not text.
pub fn pasted_path(stdout: &str) -> Result<String, String> {
    let mut lines = stdout.lines();
    lines
        .by_ref()
        .find(|line| *line == PASTE_MARKER)
        .ok_or("the machine did not say where it kept the file")?;
    let path = lines
        .next()
        .ok_or("the machine did not say where it kept the file")?;
    if !path.starts_with('/') {
        return Err(format!(
            "the machine named {path:?}, which is not a full path"
        ));
    }
    if path.len() > PATH_MAX || path.chars().any(|c| c.is_control() || is_invisible(c)) {
        return Err("the machine named a path that cannot be used".into());
    }
    Ok(path.to_owned())
}

/// The last thing ssh said, as one clean line. It came from another machine, so it is cut short
/// first and then loses every control and invisible character, as a remote's snapshot does.
fn said_by(stderr: &str) -> String {
    let line = stderr
        .lines()
        .map(str::trim)
        .rfind(|line| !line.is_empty())
        .unwrap_or("");
    let head: String = line.chars().take(SAID_MAX).collect();
    sanitize(&head, false).trim().to_owned()
}

/// What went wrong, in words, from how the paste script ended (`None` when the line itself was
/// ended by a signal or its time limit) and what ssh said. `what` is "the image" or "the file".
pub fn paste_failure(what: &str, machine: &str, code: Option<i32>, stderr: &str) -> String {
    let said = said_by(stderr);
    let said = said.as_str();
    match code {
        Some(70) => format!("{what} reached {machine} incomplete; try again"),
        Some(72) => format!("{machine} could not keep {what} (no writable cache folder there)"),
        None => format!("{machine} did not take {what} in time"),
        _ if said.is_empty() => format!("could not send {what} to {machine}"),
        _ => format!("could not send {what} to {machine}: {said}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_paste_is_named_with_plain_characters_only() {
        let name = paste_file_name(1_762_000_000_000, 0xAB);
        assert_eq!(name, "paste-1762000000000-000000ab.png");
        assert!(
            name.chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        );
    }

    #[test]
    fn a_sent_file_keeps_a_plain_version_of_its_name_after_a_unique_prefix() {
        let name = |original: &str| sent_file_name(1_762_000_000_000, 0xAB, original);
        assert_eq!(
            name("report.pdf"),
            "paste-1762000000000-000000ab-report.pdf"
        );
        assert_eq!(
            name("my report (final).pdf"),
            "paste-1762000000000-000000ab-my_report__final_.pdf"
        );
        assert_eq!(name(".bashrc"), "paste-1762000000000-000000ab-bashrc");
        assert_eq!(
            name("../../etc/passwd"),
            "paste-1762000000000-000000ab-_.._etc_passwd"
        );
        assert_eq!(name(""), "paste-1762000000000-000000ab-file");
        assert_eq!(name("..."), "paste-1762000000000-000000ab-file");
        assert_eq!(name("día.txt"), "paste-1762000000000-000000ab-d_a.txt");
    }

    #[test]
    fn every_sent_name_is_one_the_script_and_the_line_accept() {
        for original in [
            "a b;c\"'`$(x)",
            "\u{202e}rtl.txt",
            &"long".repeat(100),
            &format!("{}.tar.gz", "x".repeat(200)),
            "-rf",
            "\n",
        ] {
            let name = sent_file_name(1_762_000_000_000, 0xAB, original);
            assert!(name.len() <= 96, "{name}");
            assert!(
                name.chars()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_')),
                "{name}"
            );
            assert!(!name.starts_with('.'));
        }
        let long = sent_file_name(1, 1, &format!("{}.tar.gz", "x".repeat(200)));
        assert!(
            long.ends_with(".gz"),
            "the extension survives a cut: {long}"
        );
    }

    #[test]
    fn the_path_is_the_line_after_the_marker_whatever_a_login_printed_before() {
        let said =
            format!("Last login: Mon\nWelcome\n{PASTE_MARKER}\n/home/me/.cache/x/pastes/p.png\n");
        assert_eq!(
            pasted_path(&said).unwrap(),
            "/home/me/.cache/x/pastes/p.png"
        );
    }

    #[test]
    fn a_path_the_far_side_makes_up_is_refused() {
        let with = |path: &str| format!("{PASTE_MARKER}\n{path}\n");
        assert!(pasted_path("").is_err());
        assert!(pasted_path("/home/me/p.png\n").is_err(), "no marker");
        assert!(
            pasted_path(&format!("{PASTE_MARKER}\n")).is_err(),
            "no path"
        );
        assert!(pasted_path(&with("relative/p.png")).is_err());
        assert!(pasted_path(&with("~/p.png")).is_err());
        assert!(pasted_path(&with("/a/\u{1b}[31m/p.png")).is_err());
        assert!(pasted_path(&with(&format!("/{}", "a".repeat(PATH_MAX)))).is_err());
    }

    #[test]
    fn a_failure_says_what_to_do() {
        assert!(paste_failure("the image", "box", Some(70), "").contains("incomplete"));
        assert!(paste_failure("the image", "box", Some(72), "").contains("could not keep"));
        assert!(paste_failure("the image", "box", None, "").contains("in time"));
        assert_eq!(
            paste_failure(
                "the image",
                "box",
                Some(255),
                "x\nPermission denied (publickey).\n"
            ),
            "could not send the image to box: Permission denied (publickey)."
        );
        assert_eq!(
            paste_failure("the image", "box", Some(1), ""),
            "could not send the image to box"
        );
    }

    #[test]
    fn what_ssh_said_is_shown_as_one_clean_short_line() {
        let hostile = "x\n\u{1b}[2J\u{202e}Permission\u{200b} denied\u{7}\u{0}";
        let said = paste_failure("the image", "box", Some(255), hostile);
        assert_eq!(
            said,
            "could not send the image to box: [2JPermission denied"
        );
        assert!(
            !said
                .chars()
                .any(|c| c.is_control() || maitty_shared::hidden::is_invisible(c))
        );
        let long = format!("{}\n", "e".repeat(5000));
        let said = paste_failure("the image", "box", Some(255), &long);
        assert_eq!(
            said.chars().count(),
            "could not send the image to box: ".len() + SAID_MAX
        );
        // A line of nothing but hidden characters says nothing at all.
        assert_eq!(
            paste_failure("the image", "box", Some(255), "\u{1b}\u{200b}\u{202e}\n"),
            "could not send the image to box"
        );
    }

    #[test]
    fn a_path_with_an_invisible_character_is_refused() {
        for hidden in ['\u{202e}', '\u{200b}', '\u{feff}', '\u{2066}'] {
            let said = format!("{PASTE_MARKER}\n/home/me/p{hidden}ng.png\n");
            assert!(pasted_path(&said).is_err(), "{hidden:?}");
        }
    }

    #[cfg(unix)]
    mod the_script {
        use super::*;
        use crate::remote_install::PASTE_SCRIPT;
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;
        use std::process::{Command, Stdio};

        struct Run {
            code: Option<i32>,
            stdout: String,
        }

        fn run(home: &std::path::Path, name: &str, want: usize, body: &[u8]) -> Run {
            let mut child = Command::new("sh")
                .args(["-c", PASTE_SCRIPT, "sh", name, &want.to_string()])
                .env("HOME", home)
                .env_remove("XDG_CACHE_HOME")
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .expect("run sh");
            // A script that refuses its arguments ends before it reads the image: a closed pipe is
            // how that looks from here, not a failure of the test.
            let _ = child.stdin.take().unwrap().write_all(body);
            let out = child.wait_with_output().unwrap();
            Run {
                code: out.status.code(),
                stdout: String::from_utf8_lossy(&out.stdout).into_owned(),
            }
        }

        fn home(label: &str) -> std::path::PathBuf {
            let dir = std::env::temp_dir().join(format!(
                concat!(env!("APP_COMMAND"), "-paste-{}-{}"),
                label,
                std::process::id()
            ));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            dir
        }

        fn pastes(home: &std::path::Path) -> std::path::PathBuf {
            home.join(".cache").join(env!("APP_COMMAND")).join("pastes")
        }

        #[test]
        fn it_keeps_the_image_privately_and_says_where() {
            let home = home("keep");
            let ran = run(&home, "paste-1-0000000a.png", 5, b"\x89PNG!");
            assert_eq!(ran.code, Some(0));
            let path = pasted_path(&ran.stdout).expect("a path after the marker");
            assert_eq!(
                path,
                pastes(&home)
                    .join("paste-1-0000000a.png")
                    .display()
                    .to_string()
            );
            assert_eq!(std::fs::read(&path).unwrap(), b"\x89PNG!");
            let mode = std::fs::metadata(&path).unwrap().permissions().mode() & 0o777;
            assert_eq!(mode, 0o600, "readable by its owner alone");
            let folder = std::fs::metadata(pastes(&home))
                .unwrap()
                .permissions()
                .mode()
                & 0o777;
            assert_eq!(folder, 0o700);
            let _ = std::fs::remove_dir_all(&home);
        }

        #[test]
        fn an_image_that_arrives_short_is_not_kept() {
            let home = home("short");
            let ran = run(&home, "paste-2-0000000b.png", 99, b"short");
            assert_eq!(ran.code, Some(70));
            assert!(!pastes(&home).join("paste-2-0000000b.png").exists());
            let left: Vec<_> = std::fs::read_dir(pastes(&home))
                .unwrap()
                .flatten()
                .collect();
            assert!(left.is_empty(), "no half-written file is left: {left:?}");
            let _ = std::fs::remove_dir_all(&home);
        }

        #[test]
        fn a_name_that_is_not_plain_is_refused_before_anything_is_written() {
            let home = home("name");
            for bad in ["", ".x", "a/b", "../x", "a b", "x;y"] {
                assert_eq!(run(&home, bad, 1, b"x").code, Some(72), "{bad:?}");
            }
            assert!(!pastes(&home).exists(), "nothing was made");
            let _ = std::fs::remove_dir_all(&home);
        }

        #[test]
        fn a_machine_with_no_home_keeps_nothing() {
            let home = home("nohome");
            let ran = Command::new("sh")
                .args(["-c", PASTE_SCRIPT, "sh", "paste-3-0000000c.png", "1"])
                .env_remove("HOME")
                .env_remove("XDG_CACHE_HOME")
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()
                .unwrap();
            assert_eq!(ran.code(), Some(72));
            let _ = std::fs::remove_dir_all(&home);
        }

        #[test]
        fn pastes_older_than_a_week_are_cleared_and_newer_ones_stay() {
            let home = home("old");
            let dir = pastes(&home);
            std::fs::create_dir_all(&dir).unwrap();
            let (old, new, other) = (
                dir.join("paste-old.png"),
                dir.join("paste-new.png"),
                dir.join("notes.txt"),
            );
            for file in [&old, &new, &other] {
                std::fs::write(file, b"x").unwrap();
            }
            for file in [&old, &other] {
                let status = Command::new("touch")
                    .args(["-d", "10 days ago"])
                    .arg(file)
                    .status()
                    .unwrap();
                assert!(status.success());
            }
            assert_eq!(run(&home, "paste-4-0000000d.png", 1, b"x").code, Some(0));
            assert!(!old.exists(), "a week-old paste is cleared");
            assert!(new.exists());
            assert!(other.exists(), "only pastes are cleared, never other files");
            let _ = std::fs::remove_dir_all(&home);
        }

        #[test]
        fn a_folder_that_belongs_to_someone_else_is_refused_and_nothing_is_written() {
            let home = home("owner");
            let app = home.join(".cache").join(env!("APP_COMMAND"));
            std::fs::create_dir_all(&app).unwrap();
            // `/tmp` is a folder anyone may write in but root owns: standing in for a folder
            // another user made, which this user could still put a file in.
            std::os::unix::fs::symlink("/tmp", app.join("pastes")).unwrap();
            let name = format!("paste-6-{:08x}.png", std::process::id());
            let ran = run(&home, &name, 1, b"x");
            let planted = std::path::Path::new("/tmp").join(&name);
            let written = planted.exists();
            let _ = std::fs::remove_file(&planted);
            assert!(
                !written,
                "a file was written into a folder someone else owns"
            );
            assert_eq!(ran.code, Some(72));
            assert!(ran.stdout.is_empty(), "{}", ran.stdout);
            let _ = std::fs::remove_dir_all(&home);
        }

        #[test]
        fn half_sent_files_a_killed_upload_left_are_swept_when_stale_and_a_fresh_one_stays() {
            let home = home("stale");
            let dir = pastes(&home);
            std::fs::create_dir_all(&dir).unwrap();
            let (stale, fresh) = (dir.join(".new.stale1"), dir.join(".new.fresh2"));
            for file in [&stale, &fresh] {
                std::fs::write(file, b"x").unwrap();
            }
            let status = Command::new("touch")
                .args(["-d", "3 days ago"])
                .arg(&stale)
                .status()
                .unwrap();
            assert!(status.success());
            assert_eq!(run(&home, "paste-7-00000010.png", 1, b"x").code, Some(0));
            assert!(!stale.exists(), "a leftover of a day or more is swept");
            assert!(
                fresh.exists(),
                "an upload that may be running is left alone"
            );
            let _ = std::fs::remove_dir_all(&home);
        }

        #[test]
        fn the_cache_folder_the_far_side_names_is_used_when_it_is_a_full_path() {
            let home = home("xdg");
            let cache = home.join("elsewhere");
            let mut child = Command::new("sh")
                .args(["-c", PASTE_SCRIPT, "sh", "paste-5-0000000e.png", "1"])
                .env("HOME", &home)
                .env("XDG_CACHE_HOME", &cache)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
                .unwrap();
            let _ = child.stdin.take().unwrap().write_all(b"x");
            let out = child.wait_with_output().unwrap();
            let path = pasted_path(&String::from_utf8_lossy(&out.stdout)).unwrap();
            assert!(path.starts_with(&cache.display().to_string()), "{path}");
            let _ = std::fs::remove_dir_all(&home);
        }
    }
}
