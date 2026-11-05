//! What is kept for a chat message, what is refused, and what a hostile name comes to.

use super::*;
use std::sync::atomic::{AtomicU32, Ordering};

const PNG: &[u8] = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDRrest-of-the-image";

fn folder(label: &str) -> PathBuf {
    static NEXT: AtomicU32 = AtomicU32::new(0);
    let n = NEXT.fetch_add(1, Ordering::Relaxed);
    let dir = std::env::temp_dir().join(format!("attachment-{label}-{}-{n}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("make a test folder");
    dir
}

fn write(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
    let path = dir.join(name);
    std::fs::write(&path, bytes).expect("write a source file");
    path
}

#[test]
fn a_hostile_name_is_cleaned_to_plain_characters() {
    assert_eq!(clean_name("../../etc/passwd"), "passwd");
    assert_eq!(clean_name("C:\\Users\\me\\shot one.png"), "shot_one.png");
    assert_eq!(clean_name("a\nb\u{202e}gnp.exe"), "a_b_gnp.exe");
    assert_eq!(clean_name(".bashrc"), "bashrc");
    assert_eq!(clean_name("$(rm -rf ~).txt"), "__rm_-rf___.txt");
    assert_eq!(clean_name("..."), "file");
    assert_eq!(clean_name(""), "file");
    let long = format!("{}.png", "x".repeat(200));
    let cleaned = clean_name(&long);
    assert_eq!(cleaned.chars().count(), NAME_MAX);
    assert!(cleaned.ends_with(".png"));
}

#[test]
fn a_file_is_copied_into_the_private_folder_and_described() {
    let source = folder("src");
    let kept = folder("kept").join("attachments");
    let path = write(&source, "Design Final.zip", b"PK\x03\x04zipbytes");
    let got = attach(&kept, &path).expect("a zip is a file");
    assert_eq!((got.name.as_str(), got.kind, got.size), ("Design_Final.zip", Kind::Archive, 12));
    let copy = PathBuf::from(&got.path);
    assert_eq!(copy.parent(), Some(kept.as_path()));
    assert!(copy.file_name().unwrap().to_string_lossy().starts_with(PREFIX));
    assert!(got.path.ends_with("-Design_Final.zip"));
    // Maitty never unpacks it: the kept file is the archive itself, byte for byte.
    assert_eq!(std::fs::read(&copy).unwrap(), b"PK\x03\x04zipbytes");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&copy).unwrap().permissions().mode() & 0o777, 0o600);
        assert_eq!(std::fs::metadata(&kept).unwrap().permissions().mode() & 0o777, 0o700);
    }
}

#[test]
fn the_kind_comes_from_the_bytes_not_the_name() {
    let source = folder("kind");
    let kept = folder("kind-kept");
    assert_eq!(attach(&kept, &write(&source, "notes.txt", PNG)).unwrap().kind, Kind::Image);
    assert_eq!(attach(&kept, &write(&source, "shot.png", b"just text")).unwrap().kind, Kind::File);
}

#[test]
fn a_folder_a_missing_path_and_an_empty_file_are_refused() {
    let source = folder("refuse");
    let kept = folder("refuse-kept");
    assert!(attach(&kept, &source).unwrap_err().contains("not a file"));
    assert!(attach(&kept, &source.join("gone.png")).unwrap_err().contains("could not read"));
    assert!(attach(&kept, &write(&source, "empty.txt", b"")).unwrap_err().contains("empty"));
    assert_eq!(std::fs::read_dir(&kept).unwrap().count(), 0, "a refusal leaves nothing behind");
}

#[test]
fn a_file_that_outgrows_its_cap_while_copied_leaves_nothing() {
    let kept = folder("grow");
    let copy = kept.join("attach-1-x");
    let mut input = std::io::repeat(7u8);
    assert!(copy_private(&mut input, b"head", &copy, 100).is_err());
    assert!(!copy.exists());
    let mut small = std::io::Cursor::new(vec![1u8; 10]);
    assert_eq!(copy_private(&mut small, b"head", &copy, 100).unwrap(), 14);
}

#[test]
fn an_image_over_its_cap_is_refused_before_it_is_copied() {
    let source = folder("big");
    let kept = folder("big-kept");
    let path = source.join("huge.png");
    // Sparse: the length counts, the disk does not. The first bytes make it an image.
    let mut file = std::fs::File::create(&path).unwrap();
    file.set_len(MAX_PASTE_BYTES + 1).unwrap();
    file.write_all(PNG).unwrap();
    drop(file);
    let message = attach(&kept, &path).unwrap_err();
    assert!(message.contains("more than the 25 MB"), "{message}");
    assert_eq!(std::fs::read_dir(&kept).unwrap().count(), 0);
}

#[test]
fn only_a_week_old_attachment_is_swept_and_other_files_stay() {
    let kept = folder("sweep");
    let old = write(&kept, "attach-1-old.png", b"x");
    let other = write(&kept, "notes.txt", b"x");
    let later = SystemTime::now() + KEEP + Duration::from_secs(60);
    // `now` is moved forward past the week; the file that is made "current" then is not old.
    let fresh = write(&kept, "attach-2-new.png", b"x");
    std::fs::File::options().write(true).open(&fresh).unwrap().set_modified(later).unwrap();
    sweep(&kept, later);
    assert!(!old.exists());
    assert!(other.exists());
    assert!(fresh.exists());
}

#[test]
fn only_kept_files_are_reported_present() {
    let kept = folder("exist");
    let elsewhere = folder("exist-else");
    let a = write(&kept, "attach-1-a.png", b"x");
    let outside = write(&elsewhere, "secret.txt", b"x");
    let traversal = format!("{}/../{}/secret.txt", kept.display(), elsewhere.file_name().unwrap().to_string_lossy());
    let paths = vec![
        a.display().to_string(),
        kept.join("attach-9-gone.png").display().to_string(),
        outside.display().to_string(),
        traversal,
        kept.display().to_string(),
    ];
    assert_eq!(exist(&[kept], &paths), vec![true, false, false, false, false]);
}
