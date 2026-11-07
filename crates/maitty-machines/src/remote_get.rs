//! Pulling a file from another machine to this computer over its own hardened ssh line
//! (ADR 0007), shared by the terminal app and the desktop app. The caller runs it off its UI
//! thread: a file takes a while. The bytes come from a machine this computer does not trust, so
//! they are written under rules kept here: the local name is chosen here, never taken from the
//! machine; nothing is overwritten; the file appears under its name only when the announced
//! number of bytes arrived and not one more; and a cancel, a failure or the size cap leaves
//! nothing behind.

use crate::remote_paste::said_by;
use crate::ssh_run::{self, Watch};
use crate::{
    GET_MARKER, GET_MISSING, GET_NOT_A_FILE, GET_READ_FAILED, GET_TOO_BIG, GET_UNREADABLE, Machine,
};
use maitty_shared::hidden::sanitize;
use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// The most bytes one pulled file may have (ADR 0007). Checked on the bytes that arrive, not on
/// what the far side says.
pub const MAX_GET_BYTES: u64 = 1024 * 1024 * 1024;

/// How long pulling one file may take, connecting included.
const GET_LIMIT: Duration = Duration::from_secs(30 * 60);

/// The most text accepted before the marker line: a login banner, a message of the day.
const PREFACE_MAX: u64 = 64 * 1024;

/// How much is copied between looks at the cancel flag.
const CHUNK: usize = 64 * 1024;

/// The longest local file name.
const NAME_MAX: usize = 120;

/// The most numbered names tried before giving up on a free one.
const MOST_NAMES: u32 = 1000;

/// Names Windows will not let a file have, with or without an extension.
const RESERVED: [&str; 22] = [
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8",
    "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8", "LPT9",
];

/// The name a pulled file gets here: the last part of the remote path with every character
/// that is not a letter, digit, `.`, `_`, `-` or space replaced by `_`. It is never empty, never
/// a Windows reserved name, never ends in a dot or a space, and is at most 120 characters.
pub fn local_name(remote_path: &str) -> String {
    let last = remote_path.rsplit('/').next().unwrap_or_default();
    let mut name: String = last
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | ' ') {
                c
            } else {
                '_'
            }
        })
        .collect();
    if name.chars().count() > NAME_MAX {
        // Keep the start and the end, where the extension is.
        let chars: Vec<char> = name.chars().collect();
        let head: String = chars[..NAME_MAX - 20].iter().collect();
        let tail: String = chars[chars.len() - 20..].iter().collect();
        name = format!("{head}{tail}");
    }
    let name = name.trim_end_matches(['.', ' ']).to_owned();
    if name.is_empty() || name.chars().all(|c| c == '.' || c == ' ') {
        return "file".to_owned();
    }
    let stem = name.split('.').next().unwrap_or_default().trim_end();
    if RESERVED.iter().any(|reserved| stem.eq_ignore_ascii_case(reserved)) {
        return format!("_{name}");
    }
    name
}

/// `name`, or `name-1`, `name-2` before its extension. `n` of 0 is the name itself.
fn numbered(name: &str, n: u32) -> String {
    if n == 0 {
        return name.to_owned();
    }
    match name.rfind('.').filter(|dot| *dot > 0) {
        Some(dot) => format!("{}-{n}{}", &name[..dot], &name[dot..]),
        None => format!("{name}-{n}"),
    }
}

/// Claim a name in `dir` that nothing has: an empty file is created with create-new, so two
/// pulls of the same name can never share one. The caller fills it by renaming over it.
fn reserve(dir: &Path, name: &str) -> Result<PathBuf, String> {
    for n in 0..MOST_NAMES {
        let path = dir.join(numbered(name, n));
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(_) => return Ok(path),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("could not create {}: {e}", shown(&path))),
        }
    }
    Err(format!("{} has too many files called {name}", shown(dir)))
}

fn shown(path: &Path) -> String {
    path.file_name()
        .map_or_else(|| path.display().to_string(), |n| n.to_string_lossy().into_owned())
}

/// Read up to and including the marker line, then the size line. Text before the marker is
/// ignored, within a limit.
fn read_header(reader: &mut impl BufRead) -> Result<u64, String> {
    let mut seen = 0u64;
    let mut line = Vec::new();
    loop {
        line.clear();
        let got = reader
            .by_ref()
            .take(PREFACE_MAX - seen.min(PREFACE_MAX))
            .read_until(b'\n', &mut line)
            .map_err(|e| format!("reading the answer failed: {e}"))?;
        if got == 0 {
            return Err("the machine sent no file".to_owned());
        }
        seen += got as u64;
        if line.strip_suffix(b"\n").map(|l| l.strip_suffix(b"\r").unwrap_or(l)) == Some(GET_MARKER.as_bytes()) {
            break;
        }
        if seen >= PREFACE_MAX {
            return Err("the machine's answer did not start with a file".to_owned());
        }
    }
    line.clear();
    reader
        .by_ref()
        .take(32)
        .read_until(b'\n', &mut line)
        .map_err(|e| format!("reading the size failed: {e}"))?;
    String::from_utf8_lossy(&line)
        .trim()
        .parse::<u64>()
        .map_err(|_| "the machine gave no size for the file".to_owned())
}

/// Take the file from `reader` into `dest_dir`, under a name made from `remote_path`. Returns its
/// path, or an error having left nothing behind. `watch` reports bytes and can end it.
pub(crate) fn receive(
    reader: &mut dyn Read,
    dest_dir: &Path,
    remote_path: &str,
    cap: u64,
    watch: &Watch,
) -> Result<PathBuf, String> {
    let mut reader = BufReader::new(reader);
    let size = read_header(&mut reader)?;
    if size > cap {
        return Err(too_big(size, cap));
    }
    watch.sent.store(0, Ordering::Relaxed);
    watch.total.store(size, Ordering::Relaxed);

    let final_path = reserve(dest_dir, &local_name(remote_path))?;
    let part = dest_dir.join(format!(".{}.part", shown(&final_path)));
    let outcome = copy_exactly(&mut reader, &part, size, cap, watch)
        .and_then(|()| std::fs::rename(&part, &final_path).map_err(|e| format!("could not finish {}: {e}", shown(&final_path))));
    if let Err(error) = outcome {
        let _ = std::fs::remove_file(&part);
        let _ = std::fs::remove_file(&final_path);
        return Err(error);
    }
    Ok(final_path)
}

fn too_big(size: u64, cap: u64) -> String {
    format!(
        "the file is {} MB, more than the {} MB that can be pulled",
        size.div_ceil(1024 * 1024),
        cap / (1024 * 1024)
    )
}

/// Copy exactly `size` bytes into a new file at `part`, then check the stream has ended.
fn copy_exactly(
    reader: &mut impl Read,
    part: &Path,
    size: u64,
    cap: u64,
    watch: &Watch,
) -> Result<(), String> {
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(part)
        .map_err(|e| format!("could not create {}: {e}", shown(part)))?;
    let mut chunk = vec![0u8; CHUNK];
    let mut got = 0u64;
    while got < size {
        if watch.cancel.load(Ordering::Relaxed) {
            return Err("stopped".to_owned());
        }
        let want = usize::try_from((size - got).min(CHUNK as u64)).unwrap_or(CHUNK);
        let n = reader
            .read(&mut chunk[..want])
            .map_err(|e| format!("the connection failed after {got} of {size} bytes: {e}"))?;
        if n == 0 {
            return Err(format!("the file ended after {got} of {size} bytes"));
        }
        got += n as u64;
        if got > cap {
            return Err(too_big(got, cap));
        }
        file.write_all(&chunk[..n])
            .map_err(|e| format!("could not write {}: {e}", shown(part)))?;
        watch.sent.store(got, Ordering::Relaxed);
    }
    let mut extra = [0u8; 1];
    if reader.read(&mut extra).map_err(|e| format!("the connection failed at the end: {e}"))? > 0 {
        return Err(format!("the machine sent more than the {size} bytes it announced"));
    }
    file.sync_all()
        .map_err(|e| format!("could not save {}: {e}", shown(part)))
}

/// What went wrong, in words, from how the get script ended (`None` when the line itself was
/// ended by a signal or its time limit) and what ssh said.
fn get_failure(machine: &str, path: &str, code: Option<i32>, stderr: &str) -> String {
    let path = sanitize(path, false);
    let said = said_by(stderr);
    match code {
        Some(GET_MISSING) => format!("{path} does not exist on {machine}"),
        Some(GET_NOT_A_FILE) => format!("{path} on {machine} is not a file; only files can be pulled"),
        Some(GET_UNREADABLE) => format!("{path} on {machine} cannot be read by you"),
        Some(GET_TOO_BIG) => format!(
            "{path} on {machine} is bigger than the {} MB that can be pulled",
            MAX_GET_BYTES / (1024 * 1024)
        ),
        Some(GET_READ_FAILED) => format!("{machine} could not finish reading {path}"),
        None => format!("{machine} did not send {path} in time"),
        _ if said.is_empty() => format!("could not get {path} from {machine}"),
        _ => format!("could not get {path} from {machine}: {said}"),
    }
}

/// Pull `remote_path` from `machine` (called `name` in words for the person) into `dest_dir`
/// and return where it landed. `watch` reports bytes received and can end it.
pub fn get_file(
    machine: &Machine,
    name: &str,
    remote_path: &str,
    dest_dir: &Path,
    watch: &Watch,
) -> Result<PathBuf, String> {
    let argv = machine
        .get_argv(remote_path, MAX_GET_BYTES)
        .map_err(|e| format!("getting a file from {name} needs ssh: {e}"))?;
    std::fs::create_dir_all(dest_dir)
        .map_err(|e| format!("could not create {}: {e}", dest_dir.display()))?;
    let streamed = ssh_run::run_reading(&argv, GET_LIMIT, watch, |out| {
        receive(out, dest_dir, remote_path, MAX_GET_BYTES, watch)
    })?;
    if streamed.cancelled {
        return Err(format!("stopped getting {} from {name}", sanitize(remote_path, false)));
    }
    match (streamed.code, streamed.result) {
        (Some(0), Ok(path)) => Ok(path),
        (Some(0), Err(why)) => Err(format!("{name}: {why}")),
        (code, Ok(path)) => {
            let _ = std::fs::remove_file(&path);
            Err(get_failure(name, remote_path, code, &streamed.stderr))
        }
        (code, Err(_)) => Err(get_failure(name, remote_path, code, &streamed.stderr)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn dir(label: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("remote-get-{label}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn answer(size: u64, body: &[u8]) -> Vec<u8> {
        let mut bytes = format!("{GET_MARKER}\n{size}\n").into_bytes();
        bytes.extend_from_slice(body);
        bytes
    }

    fn files(dir: &Path) -> Vec<String> {
        let mut names: Vec<String> = std::fs::read_dir(dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    #[test]
    fn a_name_keeps_plain_characters_and_loses_everything_else() {
        assert_eq!(local_name("/home/me/out.log"), "out.log");
        assert_eq!(local_name("~/my report (final).pdf"), "my report _final_.pdf");
        assert_eq!(local_name("/a/b/../..\\x:y*z?.txt"), ".._x_y_z_.txt");
        assert_eq!(local_name("/tmp/日本語.txt"), "___.txt");
        assert_eq!(local_name("/tmp/"), "file");
        assert_eq!(local_name(".."), "file");
        assert_eq!(local_name("/x/ . "), "file");
        assert_eq!(local_name("/x/name. . "), "name");
    }

    #[test]
    fn windows_reserved_names_are_not_used_with_or_without_an_extension() {
        assert_eq!(local_name("/x/con"), "_con");
        assert_eq!(local_name("/x/NUL.txt"), "_NUL.txt");
        assert_eq!(local_name("/x/com1.log"), "_com1.log");
        assert_eq!(local_name("/x/console.txt"), "console.txt");
    }

    #[test]
    fn a_long_name_is_cut_in_the_middle_so_the_extension_stays() {
        let long = format!("/x/{}.tar.gz", "a".repeat(300));
        let name = local_name(&long);
        assert_eq!(name.chars().count(), NAME_MAX);
        assert!(name.ends_with(".tar.gz"));
    }

    #[test]
    fn a_taken_name_gets_a_number_before_its_extension() {
        assert_eq!(numbered("out.log", 0), "out.log");
        assert_eq!(numbered("out.log", 2), "out-2.log");
        assert_eq!(numbered("Makefile", 1), "Makefile-1");
        assert_eq!(numbered(".bashrc", 1), ".bashrc-1");
    }

    #[test]
    fn a_whole_file_arrives_under_a_clean_name_and_leaves_nothing_else() {
        let dir = dir("whole");
        let watch = Watch::default();
        let path = receive(
            &mut Cursor::new(answer(11, b"hello world")),
            &dir,
            "/srv/out file.txt",
            100,
            &watch,
        )
        .unwrap();
        assert_eq!(std::fs::read(&path).unwrap(), b"hello world");
        assert_eq!(files(&dir), ["out file.txt"]);
        assert_eq!(watch.sent.load(Ordering::Relaxed), 11);
        assert_eq!(watch.total.load(Ordering::Relaxed), 11);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn two_pulls_of_one_name_never_share_a_file_and_nothing_is_overwritten() {
        let dir = dir("twice");
        std::fs::write(dir.join("a.txt"), b"mine").unwrap();
        let first = receive(&mut Cursor::new(answer(1, b"1")), &dir, "/x/a.txt", 100, &Watch::default()).unwrap();
        let second = receive(&mut Cursor::new(answer(1, b"2")), &dir, "/x/a.txt", 100, &Watch::default()).unwrap();
        assert_eq!(shown(&first), "a-1.txt");
        assert_eq!(shown(&second), "a-2.txt");
        assert_eq!(std::fs::read(dir.join("a.txt")).unwrap(), b"mine");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn text_before_the_marker_is_ignored() {
        let dir = dir("banner");
        let mut bytes = b"Welcome to box\nlast login: never\n".to_vec();
        bytes.extend(answer(2, b"ok"));
        let path = receive(&mut Cursor::new(bytes), &dir, "/x/f", 100, &Watch::default()).unwrap();
        assert_eq!(std::fs::read(path).unwrap(), b"ok");
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_stream_with_no_marker_or_no_size_is_an_error_that_leaves_nothing() {
        let dir = dir("nomarker");
        for bytes in [b"just some text\n".to_vec(), Vec::new(), format!("{GET_MARKER}\nlots\n").into_bytes()] {
            assert!(receive(&mut Cursor::new(bytes), &dir, "/x/f", 100, &Watch::default()).is_err());
        }
        assert!(files(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_short_stream_leaves_no_file_not_even_a_part() {
        let dir = dir("short");
        let error = receive(&mut Cursor::new(answer(10, b"abc")), &dir, "/x/f", 100, &Watch::default()).unwrap_err();
        assert!(error.contains("ended after 3 of 10 bytes"), "{error}");
        assert!(files(&dir).is_empty(), "{:?}", files(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_long_stream_is_refused_even_though_the_announced_part_arrived() {
        let dir = dir("long");
        let error = receive(&mut Cursor::new(answer(3, b"abcdef")), &dir, "/x/f", 100, &Watch::default()).unwrap_err();
        assert!(error.contains("more than the 3 bytes it announced"), "{error}");
        assert!(files(&dir).is_empty(), "{:?}", files(&dir));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_size_over_the_cap_is_refused_before_a_file_is_made() {
        let dir = dir("cap");
        let error = receive(&mut Cursor::new(answer(101, b"")), &dir, "/x/f", 100, &Watch::default()).unwrap_err();
        assert!(error.contains("more than the"), "{error}");
        assert!(files(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn a_cancel_stops_the_copy_and_removes_what_was_written() {
        let dir = dir("cancel");
        let watch = Watch::default();
        watch.cancel.store(true, Ordering::Relaxed);
        let error = receive(&mut Cursor::new(answer(4, b"abcd")), &dir, "/x/f", 100, &watch).unwrap_err();
        assert_eq!(error, "stopped");
        assert!(files(&dir).is_empty());
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn each_script_status_has_its_own_plain_words() {
        let say = |code| get_failure("box", "/x/f", code, "");
        assert_eq!(say(Some(GET_MISSING)), "/x/f does not exist on box");
        assert!(say(Some(GET_NOT_A_FILE)).contains("not a file"));
        assert!(say(Some(GET_UNREADABLE)).contains("cannot be read"));
        assert!(say(Some(GET_TOO_BIG)).contains("1024 MB"));
        assert!(say(None).contains("in time"));
        assert_eq!(say(Some(255)), "could not get /x/f from box");
        assert_eq!(
            get_failure("box", "/x/f", Some(255), "Permission denied\u{1b}[31m (publickey)"),
            "could not get /x/f from box: Permission denied[31m (publickey)"
        );
    }
}
