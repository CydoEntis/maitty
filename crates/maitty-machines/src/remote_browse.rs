//! Looking into another machine's folders from this computer, read-only, over its own hardened
//! ssh line (ADR 0007): what is in one folder, and an index of the files below a folder for a
//! search. The answer comes from a machine this computer does not trust, so it is parsed here under
//! rules: every entry must lie inside the folder the machine says it listed, a name with a control
//! or invisible character is never offered (and counted), results are capped and say when they
//! were cut, and what is shown is exactly what is sent back as a path.

use crate::ssh_run::{self, ASK_LIMIT, Watch};
use crate::{
    GET_MISSING, GET_NOT_A_FILE, GET_UNREADABLE, INDEX_MARKER, LIST_FILES_MARKER, LIST_MARKER,
    Machine,
};
use maitty_shared::hidden::{is_invisible, sanitize};
use std::io::Read;

/// The most bytes of a folder listing asked for.
pub const MOST_LIST_BYTES: u64 = 512 * 1024;
/// The most entries a listing returns.
pub const MOST_LIST_ENTRIES: usize = 2000;
/// The most bytes of a file index asked for.
pub const MOST_INDEX_BYTES: u64 = 1024 * 1024;
/// The most files an index returns.
pub const MOST_INDEX_FILES: usize = 5000;

/// The most text accepted before the marker line (a login banner, a message of the day).
const PREFACE_MAX: u64 = 64 * 1024;

/// What is in one folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Listing {
    /// The folder as the machine resolved it.
    pub folder: String,
    /// Sub-folders' names, sorted without regard to case.
    pub dirs: Vec<String>,
    /// Everything else's names, sorted the same way.
    pub files: Vec<String>,
    /// There was more than is shown.
    pub truncated: bool,
    /// Entries left out because their names have control or invisible characters.
    pub hidden: usize,
}

/// The files below a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Index {
    /// The folder as the machine resolved it.
    pub folder: String,
    /// Paths below the folder, with `/` between the parts, sorted without regard to case.
    pub files: Vec<String>,
    /// There was more than is shown.
    pub truncated: bool,
    /// Files left out because their paths have control or invisible characters.
    pub hidden: usize,
}

/// One part of a path that may be shown and sent back: not empty, not `.` or `..`, no `/`, and no
/// control or invisible character.
fn clean_part(part: &str) -> bool {
    !part.is_empty()
        && part != "."
        && part != ".."
        && !part.contains('/')
        && !part.chars().any(|c| c.is_control() || is_invisible(c))
}

/// Case-insensitive order, then the plain order so two names never tie.
fn sort_names(names: &mut [String]) {
    names.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()).then_with(|| a.cmp(b)));
}

/// The bytes after `marker` and its line break, and what the machine printed before it is
/// ignored. `None` when the marker never came.
fn after_marker<'a>(answer: &'a [u8], marker: &str) -> Option<&'a [u8]> {
    let mut line = format!("{marker}\n").into_bytes();
    if answer.starts_with(&line) {
        return Some(&answer[line.len()..]);
    }
    line.insert(0, b'\n');
    answer
        .windows(line.len())
        .position(|window| window == line.as_slice())
        .map(|at| &answer[at + line.len()..])
}

/// The NUL-ended pieces of `rest`, and whether the answer stopped in the middle of one (a cut
/// answer). A piece that is not text is `None`, so the caller can count it as hidden.
fn pieces(rest: &[u8]) -> (Vec<Option<&str>>, bool) {
    let cut = !rest.is_empty() && !rest.ends_with(&[0]);
    let mut parts: Vec<&[u8]> = rest.split(|byte| *byte == 0).collect();
    // The text after the last NUL is empty when the answer ended cleanly, and a half entry when not.
    parts.pop();
    (parts.into_iter().map(|part| std::str::from_utf8(part).ok()).collect(), cut)
}

/// `folder` followed by `/`, so an entry's name is what follows it. A root folder already ends
/// in one.
fn folder_prefix(folder: &str) -> String {
    format!("{}/", folder.trim_end_matches('/'))
}

/// The resolved folder and the pieces after it.
fn folder_and_pieces<'a>(
    answer: &'a [u8],
    marker: &str,
) -> Result<(String, Vec<Option<&'a str>>, bool), String> {
    let rest = after_marker(answer, marker).ok_or("the machine did not answer with a listing")?;
    let (mut parts, cut) = pieces(rest);
    if parts.is_empty() {
        return Err("the machine's answer was cut before it named the folder".to_owned());
    }
    let folder = match parts.remove(0) {
        Some(folder) if folder.starts_with('/') && !folder.chars().any(char::is_control) => folder.to_owned(),
        _ => return Err("the machine named a folder that cannot be used".to_owned()),
    };
    Ok((folder, parts, cut))
}

/// Read a folder listing as the list script prints it (see `LIST_SCRIPT`).
pub fn parse_listing(answer: &[u8]) -> Result<Listing, String> {
    let (folder, parts, cut) = folder_and_pieces(answer, LIST_MARKER)?;
    let prefix = folder_prefix(&folder);
    let (mut dirs, mut files) = (Vec::new(), Vec::new());
    let (mut into_files, mut hidden, mut truncated) = (false, 0, cut);
    for part in parts {
        if part == Some(LIST_FILES_MARKER) {
            into_files = true;
            continue;
        }
        let name = part
            .and_then(|path| path.strip_prefix(prefix.as_str()))
            .filter(|name| clean_part(name));
        match name {
            Some(_) if dirs.len() + files.len() >= MOST_LIST_ENTRIES => truncated = true,
            Some(name) if into_files => files.push(name.to_owned()),
            Some(name) => dirs.push(name.to_owned()),
            None => hidden += 1,
        }
    }
    sort_names(&mut dirs);
    sort_names(&mut files);
    Ok(Listing { folder, dirs, files, truncated, hidden })
}

/// Read a file index as the index script prints it (see `INDEX_SCRIPT`).
pub fn parse_index(answer: &[u8]) -> Result<Index, String> {
    let (folder, parts, cut) = folder_and_pieces(answer, INDEX_MARKER)?;
    let prefix = folder_prefix(&folder);
    let (mut files, mut hidden, mut truncated) = (Vec::new(), 0, cut);
    for part in parts {
        let relative = part
            .and_then(|path| path.strip_prefix(prefix.as_str()))
            .filter(|relative| relative.split('/').all(clean_part));
        match relative {
            Some(_) if files.len() >= MOST_INDEX_FILES => truncated = true,
            Some(relative) => files.push(relative.to_owned()),
            None => hidden += 1,
        }
    }
    sort_names(&mut files);
    Ok(Index { folder, files, truncated, hidden })
}

/// What went wrong, in words, from how a script ended (`None` when the line was ended by a signal
/// or its time limit) and what ssh said.
fn failure(machine: &str, path: &str, code: Option<i32>, stderr: &str) -> String {
    let shown = if path.is_empty() { "the home folder".to_owned() } else { sanitize(path, false) };
    let said = crate::remote_paste::said_by(stderr);
    match code {
        Some(GET_MISSING) => format!("{shown} does not exist on {machine}"),
        Some(GET_NOT_A_FILE) => format!("{shown} on {machine} is not a folder"),
        Some(GET_UNREADABLE) => format!("{shown} on {machine} cannot be read by you"),
        None => format!("{machine} did not answer in time"),
        _ if said.is_empty() => format!("could not look into {shown} on {machine}"),
        _ => format!("could not look into {shown} on {machine}: {said}"),
    }
}

/// Run `argv`, keeping at most `limit` bytes of what it prints.
fn ask(argv: &[String], limit: u64) -> Result<ssh_run::Streamed<Vec<u8>>, String> {
    ssh_run::run_reading(argv, ASK_LIMIT, &Watch::default(), |out| {
        let mut answer = Vec::new();
        out.take(limit)
            .read_to_end(&mut answer)
            .map_err(|e| format!("reading the answer failed: {e}"))?;
        Ok(answer)
    })
}

/// What is in the folder `path` on `machine` (called `name` in words for the person). An empty
/// `path` is the machine's home folder.
pub fn list_folder(machine: &Machine, name: &str, path: &str) -> Result<Listing, String> {
    let argv = machine
        .list_argv(path, MOST_LIST_BYTES)
        .map_err(|e| format!("looking into {name} needs ssh: {e}"))?;
    let streamed = ask(&argv, MOST_LIST_BYTES + PREFACE_MAX)?;
    match (streamed.code, streamed.result) {
        (Some(0), Ok(answer)) => parse_listing(&answer).map_err(|why| format!("{name}: {why}")),
        (code, _) => Err(failure(name, path, code, &streamed.stderr)),
    }
}

/// The files below the folder `folder` on `machine`, for a search.
pub fn index_files(machine: &Machine, name: &str, folder: &str) -> Result<Index, String> {
    let argv = machine
        .index_argv(folder, MOST_INDEX_BYTES)
        .map_err(|e| format!("looking into {name} needs ssh: {e}"))?;
    let streamed = ask(&argv, MOST_INDEX_BYTES + PREFACE_MAX)?;
    match (streamed.code, streamed.result) {
        (Some(0), Ok(answer)) => parse_index(&answer).map_err(|why| format!("{name}: {why}")),
        (code, _) => Err(failure(name, folder, code, &streamed.stderr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn listing(parts: &[&str]) -> Vec<u8> {
        let mut bytes = format!("{LIST_MARKER}\n").into_bytes();
        for part in parts {
            bytes.extend_from_slice(part.as_bytes());
            bytes.push(0);
        }
        bytes
    }

    fn index(parts: &[&str]) -> Vec<u8> {
        let mut bytes = format!("{INDEX_MARKER}\n").into_bytes();
        for part in parts {
            bytes.extend_from_slice(part.as_bytes());
            bytes.push(0);
        }
        bytes
    }

    #[test]
    fn a_listing_has_the_folder_its_sorted_subfolders_and_its_sorted_other_entries() {
        let got = parse_listing(&listing(&[
            "/srv/app",
            "/srv/app/src",
            "/srv/app/Docs",
            "/srv/app/.git",
            LIST_FILES_MARKER,
            "/srv/app/b.txt",
            "/srv/app/A.txt",
            "/srv/app/-dash",
        ]))
        .unwrap();
        assert_eq!(got.folder, "/srv/app");
        assert_eq!(got.dirs, [".git", "Docs", "src"]);
        assert_eq!(got.files, ["-dash", "A.txt", "b.txt"]);
        assert!(!got.truncated);
        assert_eq!(got.hidden, 0);
    }

    #[test]
    fn the_root_folder_has_names_that_follow_one_slash() {
        let got = parse_listing(&listing(&["/", "/etc", "/bin", LIST_FILES_MARKER, "/swap"])).unwrap();
        assert_eq!((got.dirs, got.files), (vec!["bin".to_owned(), "etc".to_owned()], vec!["swap".to_owned()]));
    }

    #[test]
    fn text_before_the_marker_is_ignored() {
        let mut bytes = b"Welcome to box\nlast login: never\n".to_vec();
        bytes.extend(listing(&["/srv", "/srv/a"]));
        assert_eq!(parse_listing(&bytes).unwrap().dirs, ["a"]);
    }

    #[test]
    fn an_answer_with_no_marker_or_no_folder_is_an_error() {
        assert!(parse_listing(b"just some text\n").is_err());
        assert!(parse_listing(&listing(&[])).is_err());
        assert!(parse_listing(&listing(&["relative/folder"])).is_err());
        assert!(parse_listing(&listing(&["/a\u{1b}[31mb"])).is_err());
    }

    #[test]
    fn an_entry_outside_the_folder_with_a_slash_or_with_odd_characters_is_left_out_and_counted() {
        let got = parse_listing(&listing(&[
            "/srv/app",
            "/srv/app/ok",
            "/etc/passwd",
            "/srv/app/sub/deeper",
            "/srv/app/..",
            "/srv/app/a\u{1b}[31mred",
            "/srv/app/rtl\u{202e}gpj.exe",
            "/srv/app/zero\u{200b}width",
            "/srv/appendix/x",
            LIST_FILES_MARKER,
            "/srv/app/fine.txt",
        ]))
        .unwrap();
        assert_eq!(got.dirs, ["ok"]);
        assert_eq!(got.files, ["fine.txt"]);
        assert_eq!(got.hidden, 7);
    }

    #[test]
    fn a_name_that_is_not_text_is_left_out_and_counted() {
        let mut bytes = listing(&["/srv/app", "/srv/app/ok"]);
        bytes.extend_from_slice(b"/srv/app/\xff\xfe\0");
        let got = parse_listing(&bytes).unwrap();
        assert_eq!((got.dirs.len(), got.hidden), (1, 1));
    }

    #[test]
    fn an_answer_that_stops_in_the_middle_of_a_name_says_it_was_cut_and_drops_that_name() {
        let mut bytes = listing(&["/srv/app", "/srv/app/one"]);
        bytes.extend_from_slice(b"/srv/app/tw");
        let got = parse_listing(&bytes).unwrap();
        assert_eq!(got.dirs, ["one"]);
        assert!(got.truncated);
    }

    #[test]
    fn more_entries_than_the_cap_returns_the_first_ones_and_says_so() {
        let names: Vec<String> = (0..MOST_LIST_ENTRIES + 5).map(|n| format!("/srv/app/f{n}")).collect();
        let mut parts = vec!["/srv/app"];
        parts.extend(names.iter().map(String::as_str));
        let got = parse_listing(&listing(&parts)).unwrap();
        assert_eq!(got.dirs.len(), MOST_LIST_ENTRIES);
        assert!(got.truncated);
    }

    #[test]
    fn an_index_has_paths_below_the_folder_sorted_and_only_clean_ones() {
        let got = parse_index(&index(&[
            "/srv/app",
            "/srv/app/src/b.rs",
            "/srv/app/README.md",
            "/srv/app/src/../../etc/passwd",
            "/srv/app/src//double",
            "/srv/app/src/./dot",
            "/srv/app/src/a\u{1b}[0m.rs",
            "/etc/passwd",
        ]))
        .unwrap();
        assert_eq!(got.files, ["README.md", "src/b.rs"]);
        assert_eq!(got.hidden, 5);
    }

    #[test]
    fn an_index_over_the_cap_returns_the_first_files_and_says_so() {
        let names: Vec<String> = (0..MOST_INDEX_FILES + 3).map(|n| format!("/srv/app/d/f{n}")).collect();
        let mut parts = vec!["/srv/app"];
        parts.extend(names.iter().map(String::as_str));
        let got = parse_index(&index(&parts)).unwrap();
        assert_eq!(got.files.len(), MOST_INDEX_FILES);
        assert!(got.truncated);
    }

    #[test]
    fn each_script_status_has_its_own_plain_words() {
        let say = |code| failure("box", "/srv/x", code, "");
        assert_eq!(say(Some(GET_MISSING)), "/srv/x does not exist on box");
        assert_eq!(say(Some(GET_NOT_A_FILE)), "/srv/x on box is not a folder");
        assert!(say(Some(GET_UNREADABLE)).contains("cannot be read"));
        assert!(say(None).contains("in time"));
        assert_eq!(failure("box", "", Some(255), ""), "could not look into the home folder on box");
        assert_eq!(
            failure("box", "/a", Some(255), "Permission denied\u{1b}[31m (publickey)"),
            "could not look into /a on box: Permission denied[31m (publickey)"
        );
    }
}
