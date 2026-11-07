//! Pulling a whole folder from another machine to this computer (ADR 0007). The machine prints the
//! folder as a tar archive; this computer saves it to a hidden file, checks the archive's listing,
//! and only then unpacks it, into a fresh hidden folder and never into the destination, with the
//! system `tar` (as the release installer does, so no library is added).
//!
//! The archive comes from a machine this computer does not trust, and unpacking it writes to this
//! disk, so the listing is checked first and the whole pull is refused, naming the first bad entry,
//! unless every entry is a plain file or folder with a path this computer can store safely. The
//! folder's local name is made here and never overwrites anything. A cancel, a failure or the size
//! cap leaves nothing behind.

use crate::remote_get::{MAX_GET_BYTES, local_name, numbered, reserved_name, unique};
use crate::remote_paste::said_by;
use crate::ssh_run::{self, Watch};
use crate::{
    FOLDER_MARKER, FOLDER_MISSING, FOLDER_NOT_A_FOLDER, FOLDER_UNREADABLE, Machine,
};
use maitty_shared::hidden::{is_invisible, sanitize};
use std::io::{BufRead, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::atomic::Ordering;
use std::time::Duration;

/// The most entries a folder may hold.
pub const MAX_FOLDER_ENTRIES: usize = 50_000;
/// The deepest path a folder may hold, in parts.
pub const MAX_FOLDER_DEPTH: usize = 64;
/// The longest one part of a path may be.
const MAX_PART_BYTES: usize = 255;

/// How long pulling one folder may take, connecting included.
const FOLDER_LIMIT: Duration = Duration::from_secs(30 * 60);
/// The most text accepted before the marker line (a login banner, a message of the day).
const PREFACE_MAX: u64 = 64 * 1024;
/// How much is copied between looks at the cancel flag.
const CHUNK: usize = 64 * 1024;

/// The system `tar`: on Windows its own in System32, so a Git Bash one earlier on PATH (which
/// reads `C:\...` as a host name) is not picked.
pub(crate) fn tar() -> Command {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("SystemRoot") {
        let path = Path::new(&root).join("System32").join("tar.exe");
        if path.is_file() {
            let mut command = Command::new(path);
            hide(&mut command);
            return command;
        }
    }
    let mut command = Command::new("tar");
    hide(&mut command);
    command
}

#[cfg(windows)]
fn hide(command: &mut Command) {
    use std::os::windows::process::CommandExt;
    command.creation_flags(crate::CREATE_NO_WINDOW);
}

#[cfg(not(windows))]
fn hide(_: &mut Command) {}

/// One name for a message: cleaned and cut short.
fn shown(name: &str) -> String {
    sanitize(name, false).chars().take(60).collect()
}

/// Whether one part of a path can be stored safely on any system this app runs on: not empty, not
/// `.` or `..`, no separator or drive character, no control or invisible character, not ending in a
/// dot or a space, not a Windows reserved name, and not too long.
fn safe_part(part: &str) -> bool {
    !part.is_empty()
        && part.len() <= MAX_PART_BYTES
        && part != "."
        && part != ".."
        && !part.contains(['\\', ':'])
        && !part.chars().any(|c| c.is_control() || is_invisible(c))
        && !part.ends_with(['.', ' '])
        && !reserved_name(part)
}

/// Whether an archive listing is safe to unpack, and the name of the one folder it holds. `names`
/// is `tar -tf`, `verbose` is `tar -tvf`: the verbose lines start with the entry's type, and both
/// must list the same number of entries (a name with a line break in it breaks that, and is
/// refused). Every entry must be a plain file or folder inside the single top-level folder.
pub(crate) fn check_listing(names: &str, verbose: &str) -> Result<String, String> {
    let entries: Vec<&str> = names.lines().collect();
    if entries.is_empty() {
        return Err("the folder arrived empty".into());
    }
    if entries.len() != verbose.lines().count() {
        return Err("the archive lists its entries in a way that cannot be checked".into());
    }
    if entries.len() > MAX_FOLDER_ENTRIES {
        return Err(format!("the folder has more than {MAX_FOLDER_ENTRIES} entries"));
    }
    let top = entries[0].trim_end_matches('/');
    if !safe_part(top) {
        return Err(format!("the archive's folder has a name that cannot be used ({})", shown(top)));
    }
    for (index, (name, line)) in entries.iter().zip(verbose.lines()).enumerate() {
        let kind = line.chars().next();
        if !matches!(kind, Some('-' | 'd')) {
            return Err(format!("the archive holds a link or special file ({})", shown(name)));
        }
        if index == 0 && kind != Some('d') {
            return Err("the archive does not start with a folder".into());
        }
        let plain = name.trim_end_matches('/');
        let parts: Vec<&str> = plain.split('/').collect();
        if parts.first() != Some(&top) || parts.len() > MAX_FOLDER_DEPTH {
            return Err(format!("the archive holds an unexpected path ({})", shown(name)));
        }
        if let Some(bad) = parts.iter().find(|part| !safe_part(part)) {
            return Err(format!("the archive holds a name this computer cannot store safely ({})", shown(bad)));
        }
    }
    Ok(top.to_owned())
}

/// Run the system `tar` with `args`, returning what it printed.
fn run_tar(args: &[&std::ffi::OsStr]) -> Result<String, String> {
    let output = tar()
        .args(args)
        .stdin(Stdio::null())
        .output()
        .map_err(|e| format!("could not run tar (is it installed?): {e}"))?;
    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    } else {
        Err(format!("tar could not read the folder: {}", said_by(&String::from_utf8_lossy(&output.stderr))))
    }
}

/// Check the archive at `archive`, unpack it into the fresh folder `staging`, and move its one
/// folder to `dest_dir` under `wanted`, numbered if that name is taken. Returns the final path.
pub(crate) fn unpack(archive: &Path, staging: &Path, dest_dir: &Path, wanted: &str) -> Result<PathBuf, String> {
    let list = |flags: &str| run_tar(&[flags.as_ref(), archive.as_os_str()]);
    let top = check_listing(&list("-tf")?, &list("-tvf")?)?;
    std::fs::create_dir(staging).map_err(|e| format!("could not make a folder to unpack in: {e}"))?;
    run_tar(&["-xf".as_ref(), archive.as_os_str(), "-C".as_ref(), staging.as_os_str(), "--no-same-owner".as_ref()])?;
    let unpacked = staging.join(&top);
    if !unpacked.is_dir() {
        return Err("the archive did not unpack to a folder".into());
    }
    for n in 0..1000 {
        let candidate = dest_dir.join(numbered(wanted, n));
        if candidate.exists() {
            continue;
        }
        match std::fs::rename(&unpacked, &candidate) {
            Ok(()) => return Ok(candidate),
            // Someone took the name in the meantime: try the next one.
            Err(_) if candidate.exists() => {}
            Err(e) => return Err(format!("could not finish {}: {e}", shown(wanted))),
        }
    }
    Err(format!("{} has too many folders called {}", shown(&dest_dir.display().to_string()), shown(wanted)))
}

/// Read up to and including the marker line; text before it is ignored, within a limit.
fn skip_to_marker(reader: &mut impl BufRead) -> Result<(), String> {
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
            return Err("the machine sent no folder".to_owned());
        }
        seen += got as u64;
        let text = line.strip_suffix(b"\n").map(|l| l.strip_suffix(b"\r").unwrap_or(l));
        if text == Some(FOLDER_MARKER.as_bytes()) {
            return Ok(());
        }
        if seen >= PREFACE_MAX {
            return Err("the machine's answer did not start with a folder".to_owned());
        }
    }
}

/// Save the archive that follows the marker line into the new file `archive`, stopping with an
/// error past `cap` bytes. Returns how many bytes arrived.
pub(crate) fn receive_archive(reader: &mut dyn Read, archive: &Path, cap: u64, watch: &Watch) -> Result<u64, String> {
    let mut reader = BufReader::new(reader);
    skip_to_marker(&mut reader)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(archive)
        .map_err(|e| format!("could not create a file to save the folder in: {e}"))?;
    let mut chunk = vec![0u8; CHUNK];
    let mut got = 0u64;
    loop {
        if watch.cancel.load(Ordering::Relaxed) {
            return Err("stopped".to_owned());
        }
        let n = reader.read(&mut chunk).map_err(|e| format!("the connection failed after {got} bytes: {e}"))?;
        if n == 0 {
            break;
        }
        got += n as u64;
        if got > cap {
            return Err(format!("the folder is more than the {} MB that can be pulled", cap / (1024 * 1024)));
        }
        file.write_all(&chunk[..n]).map_err(|e| format!("could not save the folder: {e}"))?;
        watch.sent.store(got, Ordering::Relaxed);
    }
    file.sync_all().map_err(|e| format!("could not save the folder: {e}"))?;
    if got == 0 {
        return Err("the folder arrived empty".into());
    }
    Ok(got)
}

/// The words for a path that is there but is not a folder.
pub(crate) fn not_a_folder(machine: &str, path: &str) -> String {
    failure(machine, path, Some(FOLDER_NOT_A_FOLDER), "")
}

/// What went wrong, in words, from how the folder script ended.
fn failure(machine: &str, path: &str, code: Option<i32>, stderr: &str) -> String {
    let path = sanitize(path, false);
    let said = said_by(stderr);
    match code {
        Some(FOLDER_MISSING) => format!("{path} does not exist on {machine}"),
        Some(FOLDER_NOT_A_FOLDER) => format!("{path} on {machine} is not a folder"),
        Some(FOLDER_UNREADABLE) => format!("{path} on {machine} cannot be read by you"),
        None => format!("{machine} did not send {path} in time"),
        _ if said.is_empty() => format!("could not get {path} from {machine}"),
        _ => format!("could not get {path} from {machine}: {said}"),
    }
}

/// Pull the folder `remote_path` from `machine` (called `name` in words for the person) into
/// `dest_dir` and return where it landed. `watch` reports bytes received and can end it.
pub fn get_folder(machine: &Machine, name: &str, remote_path: &str, dest_dir: &Path, watch: &Watch) -> Result<PathBuf, String> {
    get_folder_capped(machine, name, remote_path, dest_dir, watch, MAX_GET_BYTES)
}

pub(crate) fn get_folder_capped(
    machine: &Machine,
    name: &str,
    remote_path: &str,
    dest_dir: &Path,
    watch: &Watch,
    cap: u64,
) -> Result<PathBuf, String> {
    let argv = machine
        .get_folder_argv(remote_path)
        .map_err(|e| format!("getting a folder from {name} needs ssh: {e}"))?;
    std::fs::create_dir_all(dest_dir).map_err(|e| format!("could not create {}: {e}", dest_dir.display()))?;
    let wanted = local_name(remote_path);
    let hidden = format!(".{wanted}.{}", unique());
    let archive = dest_dir.join(format!("{hidden}.tar"));
    let staging = dest_dir.join(format!("{hidden}.dir"));
    let cleanup = || {
        let _ = std::fs::remove_file(&archive);
        let _ = std::fs::remove_dir_all(&staging);
    };

    let streamed = ssh_run::run_reading(&argv, FOLDER_LIMIT, watch, |out| receive_archive(out, &archive, cap, watch))?;
    let outcome = if streamed.cancelled {
        Err(format!("stopped getting {} from {name}", sanitize(remote_path, false)))
    } else {
        match (streamed.code, &streamed.result) {
            (Some(0), Ok(_)) => unpack(&archive, &staging, dest_dir, &wanted).map_err(|why| format!("{name}: {why}")),
            (Some(0), Err(why)) => Err(format!("{name}: {why}")),
            (code, _) => Err(failure(name, remote_path, code, &streamed.stderr)),
        }
    };
    cleanup();
    outcome
}

#[cfg(test)]
mod tests;
