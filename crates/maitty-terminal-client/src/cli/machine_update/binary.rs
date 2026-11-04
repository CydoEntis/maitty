//! The binary that goes to another machine: this computer's own when the platform matches,
//! otherwise the release for that platform, fetched here and checked against `sha256sums.txt`
//! before anything is sent. The machine being updated never reaches GitHub.

use crate::brand::{APP_COMMAND, APP_VERSION};
use crate::cli::update;
use crate::proc::system_tool;
use crate::remote_install::{MOST_BINARY_BYTES, Platform};
use std::path::{Path, PathBuf};

/// A file ready to stream, with the digest it was checked by.
#[derive(Debug)]
pub(crate) struct Binary {
    pub path: PathBuf,
    pub len: u64,
    pub sha256: String,
    /// The folder a fetched release was unpacked in, removed when this is dropped.
    _temp: Option<TempDir>,
}

#[derive(Debug)]
pub(crate) struct TempDir(PathBuf);

impl TempDir {
    fn new() -> Result<TempDir, String> {
        static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
        let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        let dir =
            std::env::temp_dir().join(format!("{APP_COMMAND}-send-{}-{n}", std::process::id()));
        std::fs::create_dir_all(&dir).map_err(|e| format!("making {}: {e}", dir.display()))?;
        Ok(TempDir(dir))
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[cfg(test)]
impl Binary {
    /// The same file, claiming another length: how a test sends fewer bytes than announced.
    pub(crate) fn with_len(mut self, len: u64) -> Binary {
        self.len = len;
        self
    }
}

/// Where the binary for a platform comes from.
pub(crate) trait Sources {
    fn binary(&self, platform: &Platform, say: &mut dyn FnMut(&str)) -> Result<Binary, String>;
}

/// The release server, as far as this needs it. Tests put a fake here: none touch the network.
pub(crate) trait Releases {
    fn latest_tag(&self) -> Result<String, String>;
    fn download(&self, tag: &str, name: &str, dir: &Path) -> Result<PathBuf, String>;
}

pub(crate) struct Github;

impl Releases for Github {
    fn latest_tag(&self) -> Result<String, String> {
        update::latest_tag()
    }

    fn download(&self, tag: &str, name: &str, dir: &Path) -> Result<PathBuf, String> {
        update::download(tag, name, dir).map_err(|e| format!("{e:#}"))
    }
}

/// This computer's own build for a twin, else a release.
pub(crate) struct ReleaseSources<'a> {
    pub releases: &'a dyn Releases,
    /// This computer's platform when it can be a remote's twin, and the file that is running.
    pub local: Option<(Platform, PathBuf)>,
}

impl ReleaseSources<'_> {
    pub(crate) fn this_computer(releases: &dyn Releases) -> ReleaseSources<'_> {
        ReleaseSources {
            releases,
            local: Platform::of_this_machine().zip(std::env::current_exe().ok()),
        }
    }
}

impl Sources for ReleaseSources<'_> {
    fn binary(&self, platform: &Platform, say: &mut dyn FnMut(&str)) -> Result<Binary, String> {
        match &self.local {
            Some((here, exe)) if here == platform => {
                say(&format!(
                    "same platform as this computer: sending {APP_COMMAND} {APP_VERSION}"
                ));
                measured(exe.clone(), None)
            }
            _ => self.fetch(platform, say),
        }
    }
}

impl ReleaseSources<'_> {
    fn fetch(&self, platform: &Platform, say: &mut dyn FnMut(&str)) -> Result<Binary, String> {
        let target = update::target_for(platform.os_name(), platform.arch_name())
            .ok_or_else(|| format!("there is no release build for {platform}"))?;
        let asset = update::asset_for(platform.os_name(), target);
        let temp = TempDir::new()?;
        let (tag, archive) = self.archive(&asset, &temp.0, say)?;
        let sums = self.releases.download(&tag, "sha256sums.txt", &temp.0)?;
        let sums =
            std::fs::read_to_string(&sums).map_err(|e| format!("reading the checksums: {e}"))?;
        verify_archive(&archive, &sums, &asset)?;
        say("checksum verified");
        let member = format!("{APP_COMMAND}-{target}/{APP_COMMAND}");
        unpack(&archive, &temp.0, &member)?;
        measured(temp.0.join(&member), Some(temp))
    }

    /// The archive of this app's own version when the release server has it, else the latest
    /// (said aloud: the far side then runs another version than this computer does).
    fn archive(
        &self,
        asset: &str,
        dir: &Path,
        say: &mut dyn FnMut(&str),
    ) -> Result<(String, PathBuf), String> {
        let own = format!("v{APP_VERSION}");
        say(&format!("fetching {asset} {own} on this computer…"));
        match self.releases.download(&own, asset, dir) {
            Ok(path) => Ok((own, path)),
            Err(why) => {
                say(&format!(
                    "release {own} is not available ({why}); using the latest"
                ));
                let latest = self.releases.latest_tag()?;
                let path = self.releases.download(&latest, asset, dir)?;
                Ok((latest, path))
            }
        }
    }
}

/// `path`'s size and digest, refusing one too big to send.
pub(crate) fn measured(path: PathBuf, temp: Option<TempDir>) -> Result<Binary, String> {
    let meta =
        std::fs::symlink_metadata(&path).map_err(|e| format!("reading {}: {e}", path.display()))?;
    if !meta.is_file() {
        return Err(format!("{} is not a plain file", path.display()));
    }
    if meta.len() == 0 || meta.len() > MOST_BINARY_BYTES {
        return Err(format!(
            "{} is {} bytes; only files up to {} MB are sent",
            path.display(),
            meta.len(),
            MOST_BINARY_BYTES / (1024 * 1024)
        ));
    }
    let sha256 = update::sha256_hex(&path).map_err(|e| format!("{e:#}"))?;
    Ok(Binary {
        len: meta.len(),
        path,
        sha256,
        _temp: temp,
    })
}

/// The archive is the one `sha256sums.txt` lists, byte for byte (the whole digest, exactly).
pub(crate) fn verify_archive(archive: &Path, sums: &str, asset: &str) -> Result<(), String> {
    let expected = update::expected_checksum(sums, asset)
        .ok_or_else(|| format!("no checksum is listed for {asset}"))?;
    let size = std::fs::metadata(archive)
        .map_err(|e| format!("reading {asset}: {e}"))?
        .len();
    if size > MOST_BINARY_BYTES {
        return Err(format!(
            "{asset} is larger than {} MB",
            MOST_BINARY_BYTES / (1024 * 1024)
        ));
    }
    let actual = update::sha256_hex(archive).map_err(|e| format!("{e:#}"))?;
    if actual != expected {
        return Err(format!("checksum mismatch for {asset}"));
    }
    Ok(())
}

/// Take `member` (and only it) out of `archive` into `dir`, after the listing is checked.
fn unpack(archive: &Path, dir: &Path, member: &str) -> Result<(), String> {
    let names = tar(&["-tf"], archive, None)?;
    let verbose = tar(&["-tvf"], archive, None)?;
    check_listing(&names, &verbose, member)?;
    tar(&["-xf"], archive, Some((dir, member))).map(drop)
}

fn tar(mode: &[&str], archive: &Path, extract: Option<(&Path, &str)>) -> Result<String, String> {
    let mut command = system_tool("tar");
    command.args(mode).arg(archive);
    if let Some((dir, member)) = extract {
        command.arg("-C").arg(dir).arg(member);
    }
    crate::proc::run(&mut command).map_err(|e| format!("unpacking: {e}"))
}

/// Whether an archive listing is safe to take `member` from: every entry is a plain file or
/// folder under `member`'s own folder (no absolute path, no `..`, no link of any kind), and
/// `member` is among them. `names` is `tar -tf`, `verbose` is `tar -tvf`: the verbose lines
/// start with the entry's type, and both must list the same number of entries.
pub(crate) fn check_listing(names: &str, verbose: &str, member: &str) -> Result<(), String> {
    let folder = member.rsplit_once('/').map_or("", |(folder, _)| folder);
    let entries: Vec<&str> = names.lines().collect();
    if entries.len() != verbose.lines().count() {
        return Err("the archive lists its entries in a way that cannot be checked".into());
    }
    for (name, line) in entries.iter().zip(verbose.lines()) {
        if !matches!(line.chars().next(), Some('-' | 'd')) {
            return Err(format!(
                "the archive holds a link or special file ({})",
                shown(name)
            ));
        }
        let plain = name
            .strip_prefix("./")
            .unwrap_or(name)
            .trim_end_matches('/');
        let escapes = name.starts_with('/')
            || name.contains('\\')
            || name.contains(':')
            || plain.split('/').any(|part| part == "..");
        let inside = plain == folder
            || plain
                .strip_prefix(folder)
                .is_some_and(|rest| rest.starts_with('/'));
        if escapes || !inside {
            return Err(format!(
                "the archive holds an unexpected path ({})",
                shown(name)
            ));
        }
    }
    if entries
        .iter()
        .any(|name| name.strip_prefix("./").unwrap_or(name) == member)
    {
        Ok(())
    } else {
        Err(format!("the archive has no {member}"))
    }
}

fn shown(name: &str) -> String {
    maitty_shared::hidden::sanitize(name, false)
        .chars()
        .take(60)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};

    const MEMBER: &str = "maitty-x86_64-unknown-linux-musl/maitty";

    fn listing(entries: &[(&str, char)]) -> (String, String) {
        let names = entries
            .iter()
            .map(|(n, _)| *n)
            .collect::<Vec<_>>()
            .join("\n");
        let verbose = entries
            .iter()
            .map(|(n, t)| format!("{t}rwxr-xr-x 0 u g 10 Jan 1 00:00 {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        (names, verbose)
    }

    fn check(entries: &[(&str, char)]) -> Result<(), String> {
        let (names, verbose) = listing(entries);
        check_listing(&names, &verbose, MEMBER)
    }

    #[test]
    fn the_release_layout_passes() {
        assert_eq!(
            check(&[
                ("maitty-x86_64-unknown-linux-musl/", 'd'),
                (MEMBER, '-'),
                ("maitty-x86_64-unknown-linux-musl/README.md", '-'),
                ("maitty-x86_64-unknown-linux-musl/LICENSE", '-'),
            ]),
            Ok(())
        );
        assert_eq!(check(&[(&format!("./{MEMBER}"), '-')]), Ok(()));
    }

    #[test]
    fn a_path_that_climbs_out_or_starts_at_the_root_is_refused() {
        for bad in [
            "../x",
            "maitty-x86_64-unknown-linux-musl/../../x",
            "/etc/passwd",
            "maitty-x86_64-unknown-linux-musl/..",
            "C:/x",
            "maitty-x86_64-unknown-linux-musl\\..\\x",
        ] {
            let result = check(&[(MEMBER, '-'), (bad, '-')]);
            assert!(result.is_err(), "{bad}: {result:?}");
        }
    }

    #[test]
    fn a_link_of_any_kind_is_refused() {
        for kind in ['l', 'h', 'c', 'b', 'p'] {
            let result = check(&[(MEMBER, '-'), ("maitty-x86_64-unknown-linux-musl/ln", kind)]);
            assert!(result.is_err(), "{kind}");
        }
        assert!(
            check(&[(MEMBER, 'l')]).is_err(),
            "the binary itself as a link"
        );
    }

    #[test]
    fn the_wrong_name_or_another_folder_is_refused() {
        assert!(check(&[("maitty-x86_64-unknown-linux-musl/other", '-')]).is_err());
        assert!(check(&[("maitty-aarch64-apple-darwin/maitty", '-')]).is_err());
        assert!(check(&[(MEMBER, '-'), ("elsewhere/file", '-')]).is_err());
        assert!(check(&[("maitty-x86_64-unknown-linux-musl-evil/maitty", '-')]).is_err());
    }

    #[test]
    fn a_listing_that_cannot_be_matched_up_is_refused() {
        assert!(check_listing(MEMBER, "-rwx a\n-rwx b", MEMBER).is_err());
    }

    fn scratch(_why: &str) -> TempDir {
        TempDir::new().unwrap()
    }

    fn file_with(dir: &Path, name: &str, bytes: &[u8]) -> PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }

    fn digest(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn a_checksum_must_match_the_whole_digest_exactly() {
        let temp = scratch("sum");
        let archive = file_with(&temp.0, "a.tar.gz", b"payload");
        let good = digest(b"payload");
        assert_eq!(
            verify_archive(&archive, &format!("{good}  a.tar.gz\n"), "a.tar.gz"),
            Ok(())
        );
        assert_eq!(
            verify_archive(
                &archive,
                &format!("{}  a.tar.gz\n", good.to_uppercase()),
                "a.tar.gz"
            ),
            Ok(()),
            "hex case does not matter"
        );
        let bad = digest(b"other");
        assert!(
            verify_archive(&archive, &format!("{bad}  a.tar.gz\n"), "a.tar.gz")
                .unwrap_err()
                .starts_with("checksum mismatch")
        );
        assert!(
            verify_archive(
                &archive,
                &format!("{}  a.tar.gz\n", &good[..40]),
                "a.tar.gz"
            )
            .is_err(),
            "a prefix of the digest is not the digest"
        );
        assert!(
            verify_archive(&archive, &format!("{good}  b.tar.gz\n"), "a.tar.gz")
                .unwrap_err()
                .contains("no checksum is listed")
        );
        assert!(verify_archive(&archive, "", "a.tar.gz").is_err());
    }

    #[test]
    fn a_file_over_the_cap_or_a_link_or_nothing_is_not_sent() {
        let temp = scratch("cap");
        assert!(measured(file_with(&temp.0, "empty", b""), None).is_err());
        assert!(measured(temp.0.join("missing"), None).is_err());
        assert!(
            measured(temp.0.clone(), None)
                .unwrap_err()
                .contains("not a plain file")
        );
        let big = temp.0.join("big");
        let file = std::fs::File::create(&big).unwrap();
        file.set_len(MOST_BINARY_BYTES + 1).unwrap();
        assert!(
            measured(big, None)
                .unwrap_err()
                .contains("only files up to")
        );
        let ok = measured(file_with(&temp.0, "ok", b"abc"), None).unwrap();
        assert_eq!((ok.len, ok.sha256), (3, digest(b"abc")));
    }

    #[test]
    fn a_windows_computer_is_never_a_twin_so_its_exe_is_never_sent() {
        if cfg!(windows) {
            assert_eq!(Platform::of_this_machine(), None);
        }
    }

    struct Fake {
        calls: std::cell::RefCell<Vec<String>>,
        tag_missing: bool,
        sums: String,
        archive: Vec<u8>,
    }

    impl Releases for Fake {
        fn latest_tag(&self) -> Result<String, String> {
            self.calls.borrow_mut().push("latest".into());
            Ok("v9.9.9".into())
        }
        fn download(&self, tag: &str, name: &str, dir: &Path) -> Result<PathBuf, String> {
            self.calls.borrow_mut().push(format!("{tag} {name}"));
            if self.tag_missing && tag == format!("v{APP_VERSION}") {
                return Err("not found".into());
            }
            let bytes = if name == "sha256sums.txt" {
                self.sums.clone().into_bytes()
            } else {
                self.archive.clone()
            };
            Ok(file_with(dir, name, &bytes))
        }
    }

    fn other_platform() -> Platform {
        use crate::remote_install::{Arch, Os};
        Platform {
            os: Os::Linux,
            arch: Arch::Aarch64,
        }
    }

    fn sources<'a>(fake: &'a Fake) -> ReleaseSources<'a> {
        ReleaseSources {
            releases: fake,
            local: None,
        }
    }

    #[test]
    fn a_bad_checksum_stops_before_anything_is_unpacked() {
        let fake = Fake {
            calls: Default::default(),
            tag_missing: false,
            sums: format!(
                "{}  {APP_COMMAND}-aarch64-unknown-linux-musl.tar.gz\n",
                digest(b"not it")
            ),
            archive: b"payload".to_vec(),
        };
        let mut said = Vec::new();
        let result = sources(&fake).binary(&other_platform(), &mut |s| said.push(s.to_string()));
        assert!(result.err().unwrap().starts_with("checksum mismatch"));
        assert!(!said.iter().any(|s| s.contains("verified")), "{said:?}");
    }

    #[test]
    fn a_missing_checksum_line_stops_too() {
        let fake = Fake {
            calls: Default::default(),
            tag_missing: false,
            sums: "abc  something-else.tar.gz\n".into(),
            archive: b"payload".to_vec(),
        };
        let error = sources(&fake)
            .binary(&other_platform(), &mut |_| {})
            .err()
            .unwrap();
        assert!(error.contains("no checksum is listed"), "{error}");
    }

    #[test]
    fn this_apps_own_release_is_asked_for_before_the_latest() {
        let fake = Fake {
            calls: Default::default(),
            tag_missing: true,
            sums: "abc  nothing.tar.gz\n".into(),
            archive: b"payload".to_vec(),
        };
        let mut said = Vec::new();
        let _ = sources(&fake).binary(&other_platform(), &mut |s| said.push(s.to_string()));
        let asset = format!("{APP_COMMAND}-aarch64-unknown-linux-musl.tar.gz");
        let calls = fake.calls.borrow().clone();
        assert_eq!(calls[0], format!("v{APP_VERSION} {asset}"));
        assert_eq!(calls[1], "latest");
        assert_eq!(calls[2], format!("v9.9.9 {asset}"));
        assert!(
            said.iter().any(|s| s.contains("using the latest")),
            "{said:?}"
        );
    }

    /// A real tarball through the system `tar`: the whole fetch, check and unpack path.
    #[test]
    fn a_good_release_is_unpacked_to_the_one_binary() {
        let temp = scratch("good");
        let target = "aarch64-unknown-linux-musl";
        let folder = temp.0.join(format!("{APP_COMMAND}-{target}"));
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join(APP_COMMAND), b"#!/bin/sh\necho built\n").unwrap();
        std::fs::write(folder.join("README.md"), b"readme").unwrap();
        let asset = format!("{APP_COMMAND}-{target}.tar.gz");
        let made = system_tool("tar")
            .args(["-czf", asset.as_str(), &format!("{APP_COMMAND}-{target}")])
            .current_dir(&temp.0)
            .status();
        if !made.is_ok_and(|s| s.success()) {
            eprintln!("skipped: no usable tar on this machine");
            return;
        }
        let archive = std::fs::read(temp.0.join(&asset)).unwrap();
        let fake = Fake {
            calls: Default::default(),
            tag_missing: false,
            sums: format!("{}  {asset}\n", digest(&archive)),
            archive,
        };
        let binary = sources(&fake)
            .binary(&other_platform(), &mut |_| {})
            .unwrap();
        assert_eq!(
            std::fs::read(&binary.path).unwrap(),
            b"#!/bin/sh\necho built\n"
        );
        assert_eq!(binary.len, 21);
        let kept = binary.path.clone();
        drop(binary);
        assert!(
            !kept.exists(),
            "the unpacked copy is removed with the binary"
        );
    }
}
