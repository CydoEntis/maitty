//! Updating the app from its GitHub releases: `<app> update`, and a quiet check (when the
//! window opens, then every few hours) that says when a newer version is out. Downloads go through the GitHub CLI while the repo is
//! private (your sign-in), and straight from GitHub once it's public.

use crate::brand::{APP_COMMAND, APP_NAME};
use crate::installed_executable::installed_executable;
use crate::version::{newer_than, parse as version};
use anyhow::{Context, Result, bail};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

/// The steps of an install, in order. The download tools report no byte count, so progress is
/// which step it is on, not a percentage.
pub const STEPS: [&str; 4] = [
    "Downloading",
    "Checking the download",
    "Unpacking",
    "Installing",
];

/// The step an install is on, counting from 1; 0 when none runs. Read by the Updates box
/// (UI thread) while the install runs on a background thread.
static STEP: AtomicUsize = AtomicUsize::new(0);

/// The step the running install is on (1-based), or 0 when none is running.
pub fn step() -> usize {
    STEP.load(Ordering::Relaxed)
}

fn at_step(n: usize) {
    STEP.store(n, Ordering::Relaxed);
}

const REPO: &str = concat!("CydoEntis/", env!("APP_COMMAND"));

/// The release build that runs on this machine.
pub fn target() -> Option<&'static str> {
    target_for(std::env::consts::OS, std::env::consts::ARCH)
}

/// The release build for a machine, named as `std::env::consts` names its OS and CPU.
pub fn target_for(os: &str, arch: &str) -> Option<&'static str> {
    match (os, arch) {
        // Windows on ARM runs the x64 build.
        ("windows", "x86_64" | "aarch64") => Some("x86_64-pc-windows-msvc"),
        ("macos", "aarch64") => Some("aarch64-apple-darwin"),
        ("macos", "x86_64") => Some("x86_64-apple-darwin"),
        ("linux", "x86_64") => Some("x86_64-unknown-linux-musl"),
        ("linux", "aarch64") => Some("aarch64-unknown-linux-musl"),
        _ => None,
    }
}

/// The archive that holds `target`'s build: Windows releases are zips, the rest tarballs.
pub(crate) fn asset_for(os: &str, target: &str) -> String {
    format!(
        "{APP_COMMAND}-{target}.{}",
        if os == "windows" { "zip" } else { "tar.gz" }
    )
}

/// Whether the release `tag` is newer than this build.
pub fn is_newer(tag: &str) -> bool {
    newer_than(tag, env!("APP_VERSION"))
}

pub fn installed_version() -> Result<String, String> {
    let exe = installed_executable().map_err(|e| format!("finding installed {APP_NAME}: {e}"))?;
    let text = crate::proc::run(Command::new(exe).arg("--version"))?;
    let value = text
        .trim()
        .strip_prefix(concat!(env!("APP_COMMAND"), " "))
        .ok_or(concat!(
            "installed program did not report a ",
            env!("APP_NAME"),
            " version"
        ))?;
    if version(value).is_none()
        || !value
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || ".-+".contains(c))
    {
        return Err(concat!(
            "installed ",
            env!("APP_NAME"),
            " reported an invalid version"
        )
        .into());
    }
    Ok(value.to_string())
}

fn gh_signed_in() -> bool {
    crate::proc::run(Command::new("gh").args(["auth", "status"])).is_ok()
}

use crate::proc::system_tool;

/// The newest release's tag, e.g. `v0.3.0`.
pub fn latest_tag() -> Result<String, String> {
    if gh_signed_in() {
        return crate::proc::run(Command::new("gh").args([
            "release", "view", "-R", REPO, "--json", "tagName", "-q", ".tagName",
        ]))
        .map(|s| s.trim().to_string());
    }
    // The releases/latest page redirects to the newest tag's page. Unlike GitHub's API it
    // has no per-address limit, which shared networks (offices, CI) run out of.
    let url = format!("https://github.com/{REPO}/releases/latest");
    let landed = crate::proc::run(system_tool("curl").args([
        "-fsSL",
        "--max-time",
        "20",
        "-o",
        NULL_DEVICE,
        "-w",
        "%{url_effective}",
        &url,
    ]))
    .map_err(|e| format!("couldn't reach GitHub: {e}"))?;
    tag_from_url(&landed).ok_or_else(|| "no release found".into())
}

const NULL_DEVICE: &str = if cfg!(windows) { "NUL" } else { "/dev/null" };

/// `v1.2.3` from `https://github.com/owner/repo/releases/tag/v1.2.3`.
fn tag_from_url(url: &str) -> Option<String> {
    let tag = url
        .trim()
        .rsplit_once("/releases/tag/")?
        .1
        .trim_end_matches('/');
    version(tag).map(|_| tag.to_string())
}

pub(crate) fn download(tag: &str, name: &str, dir: &Path) -> Result<PathBuf> {
    let to = dir.join(name);
    let got = if gh_signed_in() {
        crate::proc::run(
            Command::new("gh")
                .args([
                    "release",
                    "download",
                    tag,
                    "-R",
                    REPO,
                    "-p",
                    name,
                    "--clobber",
                    "-D",
                ])
                .arg(dir),
        )
    } else {
        let url = format!("https://github.com/{REPO}/releases/download/{tag}/{name}");
        crate::proc::run(
            system_tool("curl")
                .args(["-fsSL", "--max-time", "300", "-o"])
                .arg(&to)
                .arg(&url),
        )
    };
    got.map_err(|e| anyhow::anyhow!("downloading {name}: {e}"))?;
    Ok(to)
}

pub(crate) fn sha256_hex(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(Sha256::digest(&bytes)
        .iter()
        .map(|b| format!("{b:02x}"))
        .collect())
}

/// `<app> update`: install the newest release over this one. `check` only says whether
/// there is one; `force` reinstalls it even when it's this version.
pub fn run(check: bool, force: bool) -> Result<()> {
    let current = env!("APP_VERSION");
    let tag = latest_tag().map_err(|e| anyhow::anyhow!(e))?;
    if !is_newer(&tag) && !force {
        println!("{APP_COMMAND} {current} is the latest version.");
        return Ok(());
    }
    if check {
        println!(
            "{APP_COMMAND} {} is out (you have {current}). Run: {APP_COMMAND} update",
            tag.trim_start_matches('v')
        );
        return Ok(());
    }
    let Some(target) = target() else {
        bail!("there's no release build for this machine; build from source")
    };
    let asset = asset_for(std::env::consts::OS, target);
    let tmp = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-update-{}"),
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp)?;
    let result = install(&tag, target, &asset, &tmp, true);
    let _ = std::fs::remove_dir_all(&tmp);
    let exe = result?;
    println!(
        "Updated {APP_COMMAND} {current} -> {} ({}).",
        tag.trim_start_matches('v'),
        exe.display()
    );
    if !integrate_opted_out() {
        match Command::new(&exe).arg("integrate").status() {
            Ok(status) if status.success() => {}
            Ok(_) => println!(concat!(
                "Warning: some agents weren't connected; run `",
                env!("APP_COMMAND"),
                " integrate` to retry."
            )),
            Err(e) => println!(
                concat!(
                    "Warning: couldn't connect your agents ({}); run `",
                    env!("APP_COMMAND"),
                    " integrate` to retry."
                ),
                e
            ),
        }
    }
    if crate::cli::server_running() {
        offer_restart()?;
    } else {
        println!(concat!("Run ", env!("APP_COMMAND"), " to start it."));
    }
    Ok(())
}

/// The server still runs the old version: say what a restart costs and offer it. Without a
/// terminal to answer, running the app does it.
fn offer_restart() -> Result<()> {
    use std::io::{IsTerminal, Write};
    let busy = crate::restart::read().map_or(0, |a| a.busy());
    println!("Your running sessions use the old version until the server restarts.");
    if busy > 0 {
        println!(
            "{busy} {} working or waiting for you; they stop and resume with their conversation.",
            if busy == 1 {
                "session is"
            } else {
                "sessions are"
            }
        );
    }
    if !(std::io::stdin().is_terminal() && std::io::stdout().is_terminal()) {
        println!(concat!(
            "Run ",
            env!("APP_COMMAND"),
            ": it will restart the server itself."
        ));
        return Ok(());
    }
    print!(
        "Restart the server now? [{}] ",
        if busy == 0 { "Y/n" } else { "y/N" }
    );
    std::io::stdout().flush()?;
    let mut answer = String::new();
    std::io::stdin().read_line(&mut answer)?;
    if wants_restart(&answer, busy == 0) {
        crate::cli::kill_server(&crate::cli::target::Target::current(), false)?;
        println!(concat!(
            "Run ",
            env!("APP_COMMAND"),
            ": your sessions resume where they left off."
        ));
    } else {
        println!(concat!(
            "Run ",
            env!("APP_COMMAND"),
            " when you are ready: it will restart the server itself."
        ));
    }
    Ok(())
}

/// `y`/`yes` says yes, `n`/`no` says no, and an empty answer takes the default.
fn wants_restart(answer: &str, default: bool) -> bool {
    match answer.trim().to_lowercase().as_str() {
        "" => default,
        a => a == "y" || a == "yes",
    }
}

/// Install the newest release over this one without printing (from inside the app, whose
/// screen printing would garble). Returns the version installed and where it went.
pub fn install_latest() -> Result<(String, PathBuf)> {
    let tag = latest_tag().map_err(|e| anyhow::anyhow!(e))?;
    let Some(target) = target() else {
        bail!("there's no release build for this machine; build from source")
    };
    let asset = asset_for(std::env::consts::OS, target);
    let tmp = std::env::temp_dir().join(format!(
        concat!(env!("APP_COMMAND"), "-update-{}"),
        std::process::id()
    ));
    std::fs::create_dir_all(&tmp)?;
    let result = install(&tag, target, &asset, &tmp, false);
    at_step(0);
    let _ = std::fs::remove_dir_all(&tmp);
    Ok((tag.trim_start_matches('v').to_string(), result?))
}

/// The newly installed program connects every installed agent (status hooks and MCP), so the
/// hooks are its own version's. `<APP>_NO_INTEGRATE=1` leaves agents' config alone.
pub fn integrate_agents(exe: &Path) -> Result<(), String> {
    if integrate_opted_out() {
        return Ok(());
    }
    crate::proc::run(Command::new(exe).arg("integrate")).map(drop)
}

fn integrate_opted_out() -> bool {
    std::env::var(concat!(env!("APP_ENV"), "_NO_INTEGRATE")).is_ok_and(|v| v == "1")
}

/// The sha256 `sha256sums.txt` lists for `asset`, lowercase.
pub(crate) fn expected_checksum(sums: &str, asset: &str) -> Option<String> {
    sums.lines().find_map(|l| {
        l.split_once("  ")
            .filter(|(_, n)| n.trim() == asset)
            .map(|(h, _)| h.trim().to_lowercase())
    })
}

fn install(tag: &str, target: &str, asset: &str, tmp: &Path, say: bool) -> Result<PathBuf> {
    if say {
        println!(
            "Downloading {APP_COMMAND} {} for {target}...",
            tag.trim_start_matches('v')
        );
    }
    at_step(1);
    let archive = download(tag, asset, tmp)?;
    let sums = std::fs::read_to_string(download(tag, "sha256sums.txt", tmp)?)?;
    at_step(2);
    let expected =
        expected_checksum(&sums, asset).with_context(|| format!("no checksum for {asset}"))?;
    if sha256_hex(&archive)? != expected {
        bail!("checksum mismatch for {asset}; nothing was changed");
    }
    at_step(3);
    crate::proc::run(
        system_tool("tar")
            .arg("-xf")
            .arg(&archive)
            .arg("-C")
            .arg(tmp),
    )
    .map_err(|e| anyhow::anyhow!("unpacking: {e}"))?;
    let name = if cfg!(windows) {
        concat!(env!("APP_COMMAND"), ".exe")
    } else {
        env!("APP_COMMAND")
    };
    let new = tmp.join(format!("{APP_COMMAND}-{target}")).join(name);
    if !new.is_file() {
        bail!("the download had no {name}");
    }
    let exe = std::env::current_exe().context(concat!("finding this ", env!("APP_COMMAND")))?;
    at_step(4);
    replace_exe(&new, &exe)?;
    Ok(exe)
}

/// Put `new` where `exe` is. Windows won't overwrite a running program but lets it be
/// renamed, so the old one steps aside (and is cleaned up on a later start).
fn replace_exe(new: &Path, exe: &Path) -> Result<()> {
    if cfg!(windows) {
        tidy_beside(exe);
        let old = free_old_path(exe);
        std::fs::rename(exe, &old)
            .with_context(|| format!("moving the old {} aside", exe.display()))?;
        if let Err(e) = std::fs::copy(new, exe) {
            let _ = std::fs::rename(&old, exe);
            return Err(e).context(concat!(
                "putting the new ",
                env!("APP_COMMAND"),
                " in place"
            ));
        }
    } else {
        let staged = exe.with_file_name(concat!(".", env!("APP_COMMAND"), ".new"));
        std::fs::copy(new, &staged).context(concat!("staging the new ", env!("APP_COMMAND")))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&staged, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&staged, exe).context(concat!(
            "putting the new ",
            env!("APP_COMMAND"),
            " in place"
        ))?;
    }
    Ok(())
}

/// A name for the old program that's free. An older one moved aside can still be running
/// (an agent's `<app> mcp` keeps going until its session ends), so it can't always be
/// removed; the next one gets a number.
fn free_old_path(exe: &Path) -> PathBuf {
    std::iter::once(exe.with_file_name(concat!(env!("APP_COMMAND"), ".old.exe")))
        .chain((2..).map(|n| exe.with_file_name(format!("{APP_COMMAND}.old-{n}.exe"))))
        .find(|p| !p.exists())
        .unwrap_or_else(|| exe.with_file_name(concat!(env!("APP_COMMAND"), ".old.exe")))
}

/// Remove the old programs updates moved aside beside `exe`, the ones nothing runs any more.
fn tidy_beside(exe: &Path) {
    let Some(dir) = exe.parent() else { return };
    for e in std::fs::read_dir(dir).into_iter().flatten().flatten() {
        let name = e.file_name().to_string_lossy().to_lowercase();
        if name.starts_with(concat!(env!("APP_COMMAND"), ".old")) && name.ends_with(".exe") {
            let _ = std::fs::remove_file(e.path());
        }
    }
}

/// Remove the old programs an update moved aside (Windows), once nothing runs them.
pub fn tidy() {
    if cfg!(windows)
        && let Ok(exe) = std::env::current_exe()
    {
        tidy_beside(&exe);
    }
}

// ---- the check ------------------------------------------------------------------------------

/// What changed since this build: the `- ` lines of every newer release's notes, newest
/// release first. Empty when GitHub can't be reached. Slow (network): off the UI thread.
pub fn changes_since_this() -> Vec<String> {
    let path = format!("repos/{REPO}/releases?per_page={RELEASES_LOOKED_AT}");
    let json = if gh_signed_in() {
        crate::proc::run(Command::new("gh").args(["api", &path]))
    } else {
        let url = format!("https://api.github.com/{path}");
        crate::proc::run(system_tool("curl").args([
            "-fsSL",
            "--max-time",
            "20",
            "-H",
            "Accept: application/vnd.github+json",
            &url,
        ]))
    };
    json.map(|j| changes_in(&j, env!("APP_VERSION")))
        .unwrap_or_default()
}

/// Enough releases to cover anyone a few versions behind.
const RELEASES_LOOKED_AT: usize = 20;

/// The notes' `- ` lines from the releases (GitHub's JSON) newer than `current`.
fn changes_in(json: &str, current: &str) -> Vec<String> {
    #[derive(serde::Deserialize)]
    struct Release {
        tag_name: String,
        #[serde(default)]
        body: Option<String>,
        #[serde(default)]
        draft: bool,
    }
    let Ok(releases) = serde_json::from_str::<Vec<Release>>(json) else {
        return Vec::new();
    };
    let Some(mine) = version(current) else {
        return Vec::new();
    };
    releases
        .iter()
        .filter(|r| !r.draft && version(&r.tag_name).is_some_and(|v| v > mine))
        .flat_map(|r| {
            r.body
                .as_deref()
                .unwrap_or("")
                .lines()
                .filter_map(|l| l.trim().strip_prefix("- "))
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::installed_executable::installed_path;

    #[test]
    fn changes_are_the_note_lines_of_newer_releases() {
        let json = r#"[
            {"tag_name": "v0.12.9", "draft": false, "body": "- Fixed: copying\r\n- New: an Update now button\r\n\r\n**Full Changelog**: x"},
            {"tag_name": "v0.12.8", "draft": false, "body": "- New: sidebar hints"},
            {"tag_name": "v0.13.0", "draft": true, "body": "- not out yet"},
            {"tag_name": "v0.12.7", "draft": false, "body": "- already have it"}
        ]"#;
        assert_eq!(
            changes_in(json, "0.12.7"),
            vec![
                "Fixed: copying",
                "New: an Update now button",
                "New: sidebar hints"
            ]
        );
        assert!(changes_in("not json", "0.12.7").is_empty());
    }

    #[test]
    fn versions_compare_as_numbers() {
        assert_eq!(version("v0.10.2"), Some((0, 10, 2)));
        assert_eq!(version("1.2.3-beta"), Some((1, 2, 3)));
        assert_eq!(version("nonsense"), None);
        assert!(is_newer("v999.0.0"));
        assert!(
            !is_newer(&format!("v{}", env!("APP_VERSION"))),
            "the same version isn't newer"
        );
        assert!(!is_newer("v0.0.1"));
    }

    #[test]
    fn the_current_app_release_is_not_reported_as_an_update() {
        let manifest = include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../apps/tui/Cargo.toml"
        ));
        let version = manifest
            .lines()
            .skip_while(|line| line.trim() != "[package]")
            .skip(1)
            .take_while(|line| !line.trim_start().starts_with('['))
            .find_map(|line| {
                let (key, value) = line.split_once('=')?;
                (key.trim() == "version").then(|| value.trim().trim_matches('"'))
            })
            .expect("the TUI package declares its release version");

        assert!(
            !is_newer(&format!("v{version}")),
            "the running app must recognize its own release as current"
        );
    }

    #[test]
    fn installed_copy_and_available_release_are_distinct_from_the_running_image() {
        let folder = Path::new("bin");
        assert_eq!(
            installed_path(&folder.join(concat!(env!("APP_COMMAND"), ".old.exe"))),
            folder.join(concat!(env!("APP_COMMAND"), ".exe"))
        );
        assert_eq!(
            installed_path(&folder.join(concat!(env!("APP_COMMAND"), ".old-2.exe"))),
            folder.join(concat!(env!("APP_COMMAND"), ".exe"))
        );
        assert_eq!(
            installed_path(&folder.join(concat!(env!("APP_COMMAND"), ".exe"))),
            folder.join(concat!(env!("APP_COMMAND"), ".exe"))
        );
        assert_eq!(
            installed_path(Path::new(concat!(
                "bin/",
                env!("APP_COMMAND"),
                " (deleted)"
            ))),
            PathBuf::from(concat!("bin/", env!("APP_COMMAND")))
        );
        assert!(newer_than("v0.30.0", "0.29.1"));
        assert!(
            !newer_than("v0.30.0", "0.30.0"),
            "an installed release is not another available update"
        );
    }

    #[test]
    fn the_latest_tag_comes_from_where_github_redirects() {
        assert_eq!(
            tag_from_url(concat!(
                "https://github.com/CydoEntis/",
                env!("APP_COMMAND"),
                "/releases/tag/v0.3.1\n"
            ))
            .as_deref(),
            Some("v0.3.1")
        );
        assert_eq!(
            tag_from_url(concat!(
                "https://github.com/CydoEntis/",
                env!("APP_COMMAND"),
                "/releases"
            )),
            None,
            "no release yet"
        );
        assert_eq!(
            tag_from_url(concat!(
                "https://github.com/CydoEntis/",
                env!("APP_COMMAND"),
                "/releases/tag/nightly"
            )),
            None
        );
    }

    #[test]
    fn the_cli_restart_offer_defaults_to_yes_only_when_nothing_is_working() {
        assert!(wants_restart("", true) && !wants_restart("", false));
        assert!(wants_restart("Y", false) && wants_restart(" yes\n", false));
        assert!(!wants_restart("n", true) && !wants_restart("later", true));
    }

    #[test]
    fn this_machine_has_a_build() {
        let t = target().expect("a release target for the platforms we build");
        assert!(asset_for(std::env::consts::OS, t).starts_with(concat!(env!("APP_COMMAND"), "-")));
    }

    #[test]
    fn each_os_and_cpu_has_its_own_release_asset() {
        let rows = [
            ("windows", "x86_64", "x86_64-pc-windows-msvc", "zip"),
            ("windows", "aarch64", "x86_64-pc-windows-msvc", "zip"),
            ("macos", "aarch64", "aarch64-apple-darwin", "tar.gz"),
            ("macos", "x86_64", "x86_64-apple-darwin", "tar.gz"),
            ("linux", "x86_64", "x86_64-unknown-linux-musl", "tar.gz"),
            ("linux", "aarch64", "aarch64-unknown-linux-musl", "tar.gz"),
        ];
        for (os, arch, target, ext) in rows {
            assert_eq!(target_for(os, arch), Some(target), "{os} {arch}");
            assert_eq!(
                asset_for(os, target),
                format!("{}-{target}.{ext}", env!("APP_COMMAND"))
            );
        }
        assert_eq!(target_for("linux", "riscv64"), None);
        assert_eq!(target_for("freebsd", "x86_64"), None);
    }

    #[test]
    fn the_self_update_still_targets_this_machine() {
        assert_eq!(
            target(),
            target_for(std::env::consts::OS, std::env::consts::ARCH)
        );
    }

    #[test]
    fn a_checksum_is_the_listed_one_for_exactly_that_asset() {
        let sums = "AAA1  maitty-a.tar.gz\nbbb2  maitty-b.tar.gz\n";
        assert_eq!(
            expected_checksum(sums, "maitty-a.tar.gz").as_deref(),
            Some("aaa1")
        );
        assert_eq!(
            expected_checksum(sums, "maitty-b.tar.gz").as_deref(),
            Some("bbb2")
        );
        assert_eq!(expected_checksum(sums, "maitty-c.tar.gz"), None);
        assert_eq!(expected_checksum(sums, "tar.gz"), None);
    }

    #[test]
    fn swapping_the_program_keeps_a_way_back() {
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-swap-{}"),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join(if cfg!(windows) {
            concat!(env!("APP_COMMAND"), ".exe")
        } else {
            env!("APP_COMMAND")
        });
        let new = dir.join("new-build");
        std::fs::write(&exe, "old").unwrap();
        std::fs::write(&new, "new").unwrap();
        replace_exe(&new, &exe).unwrap();
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "new");
        if cfg!(windows) {
            assert_eq!(
                std::fs::read_to_string(
                    exe.with_file_name(concat!(env!("APP_COMMAND"), ".old.exe"))
                )
                .unwrap(),
                "old",
                "the old one is moved aside"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// An older program moved aside is still running (an agent's `<app> mcp`): the update
    /// steps past it instead of failing.
    #[cfg(windows)]
    #[test]
    fn an_old_program_still_running_doesnt_stop_an_update() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = std::env::temp_dir().join(format!(
            concat!(env!("APP_COMMAND"), "-swap-busy-{}"),
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let exe = dir.join(concat!(env!("APP_COMMAND"), ".exe"));
        let new = dir.join("new-build");
        std::fs::write(&exe, "current").unwrap();
        std::fs::write(&new, "new").unwrap();
        std::fs::write(
            dir.join(concat!(env!("APP_COMMAND"), ".old.exe")),
            "running",
        )
        .unwrap();
        // Open with no delete sharing, the way a running program holds its file.
        const FILE_SHARE_READ: u32 = 1;
        let held = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(FILE_SHARE_READ)
            .open(dir.join(concat!(env!("APP_COMMAND"), ".old.exe")))
            .unwrap();
        replace_exe(&new, &exe).unwrap();
        assert_eq!(std::fs::read_to_string(&exe).unwrap(), "new");
        assert_eq!(
            std::fs::read_to_string(dir.join(concat!(env!("APP_COMMAND"), ".old-2.exe"))).unwrap(),
            "current",
            "the next free name"
        );
        drop(held);
        tidy_beside(&exe);
        assert!(
            !dir.join(concat!(env!("APP_COMMAND"), ".old.exe")).exists()
                && !dir
                    .join(concat!(env!("APP_COMMAND"), ".old-2.exe"))
                    .exists(),
            "both tidied once nothing runs them"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }
}
