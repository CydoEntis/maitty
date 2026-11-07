//! The checks on an archive that came from another machine. The checker is tested on listings, and
//! the whole path (system `tar` lists it, the checker judges it, nothing is unpacked unless it
//! passes) on archives built by hand byte by byte, so a hostile one can be made exactly.

use super::*;
use std::io::Cursor;

fn has_tar() -> bool {
    tar().arg("--version").stdin(Stdio::null()).output().is_ok_and(|o| o.status.success())
}

fn folder(label: &str) -> PathBuf {
    static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("remote-folder-{label}-{}-{n}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

/// One entry of a ustar archive: a 512-byte header, then the data padded to 512.
fn entry(name: &str, kind: u8, data: &[u8], link: &str) -> Vec<u8> {
    let mut header = [0u8; 512];
    header[..name.len()].copy_from_slice(name.as_bytes());
    header[100..108].copy_from_slice(b"0000644\0");
    header[108..116].copy_from_slice(b"0000000\0");
    header[116..124].copy_from_slice(b"0000000\0");
    header[124..136].copy_from_slice(format!("{:011o}\0", data.len()).as_bytes());
    header[136..148].copy_from_slice(b"00000000000\0");
    header[156] = kind;
    header[157..157 + link.len()].copy_from_slice(link.as_bytes());
    header[257..263].copy_from_slice(b"ustar\0");
    header[263..265].copy_from_slice(b"00");
    header[148..156].fill(b' ');
    let sum: u32 = header.iter().map(|b| u32::from(*b)).sum();
    header[148..156].copy_from_slice(format!("{sum:06o}\0 ").as_bytes());
    let mut bytes = header.to_vec();
    bytes.extend_from_slice(data);
    bytes.resize(bytes.len().div_ceil(512) * 512, 0);
    bytes
}

fn archive(entries: &[Vec<u8>]) -> Vec<u8> {
    let mut bytes: Vec<u8> = entries.concat();
    bytes.extend_from_slice(&[0u8; 1024]);
    bytes
}

const DIR: u8 = b'5';
const FILE: u8 = b'0';

fn good() -> Vec<Vec<u8>> {
    vec![
        entry("proj/", DIR, b"", ""),
        entry("proj/a.txt", FILE, b"alpha", ""),
        entry("proj/sub/", DIR, b"", ""),
        entry("proj/sub/b.txt", FILE, b"beta", ""),
    ]
}

/// Unpack `bytes` as if it had arrived, and say what was left in the destination.
fn unpack_bytes(label: &str, bytes: &[u8]) -> (Result<PathBuf, String>, PathBuf) {
    let root = folder(label);
    let dest = root.join("dest");
    std::fs::create_dir_all(&dest).unwrap();
    let file = root.join("in.tar");
    std::fs::write(&file, bytes).unwrap();
    let result = unpack(&file, &root.join("staging"), &dest, "proj");
    (result, root)
}

fn left_in(root: &Path) -> Vec<String> {
    let mut names: Vec<String> = std::fs::read_dir(root.join("dest"))
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    names.sort();
    names
}

fn lines(entries: &[(&str, char)]) -> (String, String) {
    let names = entries.iter().map(|(n, _)| *n).collect::<Vec<_>>().join("\n");
    let verbose = entries
        .iter()
        .map(|(n, t)| format!("{t}rw-r--r-- 0 u g 10 Jan 1 00:00 {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    (names, verbose)
}

fn check(entries: &[(&str, char)]) -> Result<String, String> {
    let (names, verbose) = lines(entries);
    check_listing(&names, &verbose)
}

#[test]
fn a_plain_folder_passes_and_names_its_top_folder() {
    assert_eq!(
        check(&[("proj/", 'd'), ("proj/a.txt", '-'), ("proj/sub/", 'd'), ("proj/sub/b.txt", '-')]),
        Ok("proj".to_owned())
    );
}

#[test]
fn every_kind_of_entry_that_is_not_a_plain_file_or_folder_is_refused() {
    for kind in ['l', 'h', 'c', 'b', 'p', 's'] {
        let error = check(&[("proj/", 'd'), ("proj/x", kind)]).unwrap_err();
        assert!(error.contains("link or special file"), "{kind}: {error}");
    }
}

#[test]
fn a_path_that_climbs_out_starts_at_the_root_or_leaves_the_one_folder_is_refused() {
    for bad in ["/etc/passwd", "proj/../../evil", "../evil", "proj/./x", "other/x", "proj//x", "./proj/x"] {
        assert!(check(&[("proj/", 'd'), (bad, '-')]).is_err(), "{bad}");
    }
}

#[test]
fn a_name_this_computer_could_not_store_safely_is_refused_whatever_system_it_is() {
    for bad in [
        "proj/a\\b", "proj/a:b", "proj/con", "proj/NUL.txt", "proj/com1.log", "proj/ends.", "proj/ends ",
        "proj/a\u{1b}[31m", "proj/a\u{202e}b", "proj/a\u{200b}b",
    ] {
        assert!(check(&[("proj/", 'd'), (bad, '-')]).is_err(), "{bad:?}");
    }
    assert!(check(&[("proj/", 'd'), ("proj/console.txt", '-'), ("proj/.hidden", '-'), ("proj/a b.txt", '-')]).is_ok());
}

#[test]
fn the_first_entry_must_be_the_folder_and_the_archive_must_not_be_empty() {
    assert!(check(&[("proj", '-')]).is_err(), "a file is not a folder");
    assert!(check_listing("", "").is_err());
}

#[test]
fn a_name_with_a_line_break_breaks_the_line_count_and_is_refused() {
    let names = "proj/\nproj/x\n../../y\n";
    let verbose = "drwxr-xr-x 0 u g 0 Jan 1 00:00 proj/\n-rw-r--r-- 0 u g 0 Jan 1 00:00 proj/x\n../../y";
    assert!(check_listing(names, verbose).is_err());
}

#[test]
fn too_many_entries_or_too_deep_a_path_is_refused() {
    let many: Vec<String> = std::iter::once("proj/".to_owned()).chain((0..MAX_FOLDER_ENTRIES).map(|n| format!("proj/f{n}"))).collect();
    let names = many.join("\n");
    let verbose = many.iter().enumerate().map(|(i, n)| format!("{}rw-r--r-- 0 u g 0 Jan 1 00:00 {n}", if i == 0 { 'd' } else { '-' })).collect::<Vec<_>>().join("\n");
    assert!(check_listing(&names, &verbose).unwrap_err().contains("more than"));
    let deep = format!("proj/{}", vec!["d"; MAX_FOLDER_DEPTH].join("/"));
    assert!(check(&[("proj/", 'd'), (deep.as_str(), '-')]).is_err());
}

#[test]
fn a_good_archive_unpacks_whole_under_the_wanted_name_and_a_second_one_is_numbered() {
    if !has_tar() {
        return;
    }
    let bytes = archive(&good());
    let (first, root) = unpack_bytes("good", &bytes);
    let first = first.unwrap();
    assert_eq!(first, root.join("dest").join("proj"));
    assert_eq!(std::fs::read(first.join("a.txt")).unwrap(), b"alpha");
    assert_eq!(std::fs::read(first.join("sub").join("b.txt")).unwrap(), b"beta");
    let second = unpack(&root.join("in.tar"), &root.join("staging2"), &root.join("dest"), "proj").unwrap();
    assert_eq!(second, root.join("dest").join("proj-1"), "nothing existing is touched");
    assert_eq!(left_in(&root), ["proj", "proj-1"]);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn each_hostile_archive_is_refused_before_anything_is_unpacked() {
    if !has_tar() {
        return;
    }
    let cases: Vec<(&str, Vec<Vec<u8>>)> = vec![
        ("absolute", vec![entry("proj/", DIR, b"", ""), entry("/tmp/pwned-absolute", FILE, b"x", "")]),
        ("dotdot", vec![entry("proj/", DIR, b"", ""), entry("proj/../../pwned-dotdot", FILE, b"x", "")]),
        ("symlink", vec![entry("proj/", DIR, b"", ""), entry("proj/link", b'2', b"", "/etc/passwd")]),
        ("hardlink", vec![entry("proj/", DIR, b"", ""), entry("proj/a", FILE, b"x", ""), entry("proj/b", b'1', b"", "proj/a")]),
        ("device", vec![entry("proj/", DIR, b"", ""), entry("proj/dev", b'3', b"", "")]),
        ("fifo", vec![entry("proj/", DIR, b"", ""), entry("proj/pipe", b'6', b"", "")]),
        ("second top folder", vec![entry("proj/", DIR, b"", ""), entry("other/", DIR, b"", ""), entry("other/x", FILE, b"x", "")]),
        ("backslash", vec![entry("proj/", DIR, b"", ""), entry("proj/..\\..\\pwned", FILE, b"x", "")]),
        ("newline in a name", vec![entry("proj/", DIR, b"", ""), entry("proj/ok\n../../pwned", FILE, b"x", "")]),
    ];
    for (label, entries) in cases {
        let (result, root) = unpack_bytes("hostile", &archive(&entries));
        assert!(result.is_err(), "{label} was not refused");
        assert!(left_in(&root).is_empty(), "{label}: something reached the destination: {:?}", left_in(&root));
        assert!(!root.join("staging").exists(), "{label}: something was unpacked");
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[test]
fn a_cut_off_archive_is_refused() {
    if !has_tar() {
        return;
    }
    let mut bytes = archive(&good());
    bytes.truncate(700);
    let (result, root) = unpack_bytes("cut", &bytes);
    assert!(result.is_err());
    assert!(left_in(&root).is_empty());
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn the_archive_follows_the_marker_and_text_before_it_is_ignored() {
    let root = folder("receive");
    let mut bytes = b"Welcome to box\nlast login: never\n".to_vec();
    bytes.extend(format!("{FOLDER_MARKER}\n").into_bytes());
    bytes.extend_from_slice(b"TARDATA");
    let file = root.join("a.tar");
    let got = receive_archive(&mut Cursor::new(bytes), &file, 1000, &Watch::default()).unwrap();
    assert_eq!((got, std::fs::read(&file).unwrap()), (7, b"TARDATA".to_vec()));
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn no_marker_an_empty_archive_the_cap_and_a_cancel_are_each_an_error() {
    let root = folder("receive-bad");
    let watch = Watch::default();
    let with_marker = |body: &[u8]| {
        let mut bytes = format!("{FOLDER_MARKER}\n").into_bytes();
        bytes.extend_from_slice(body);
        Cursor::new(bytes)
    };
    assert!(receive_archive(&mut Cursor::new(b"just text\n".to_vec()), &root.join("1"), 100, &watch).is_err());
    assert!(receive_archive(&mut with_marker(b""), &root.join("2"), 100, &watch).is_err());
    let error = receive_archive(&mut with_marker(&[1u8; 200]), &root.join("3"), 100, &watch).unwrap_err();
    assert!(error.contains("more than"), "{error}");
    watch.cancel.store(true, Ordering::Relaxed);
    assert_eq!(receive_archive(&mut with_marker(b"x"), &root.join("4"), 100, &watch).unwrap_err(), "stopped");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn each_script_status_has_its_own_plain_words() {
    let say = |code| failure("box", "/srv/x", code, "");
    assert_eq!(say(Some(FOLDER_MISSING)), "/srv/x does not exist on box");
    assert_eq!(say(Some(FOLDER_NOT_A_FOLDER)), "/srv/x on box is not a folder");
    assert!(say(Some(FOLDER_UNREADABLE)).contains("cannot be read"));
    assert!(say(None).contains("in time"));
    assert_eq!(say(Some(255)), "could not get /srv/x from box");
}
