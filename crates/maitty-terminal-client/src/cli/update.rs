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

/// `with_gh` when the GitHub CLI is signed in, and the public address when it is not or when
/// its call fails: a sign-in that expired or cannot see this repo must not stop an update the
/// public address can serve. When both fail, both reasons are told.
fn gh_then_public<T>(
    signed_in: bool,
    with_gh: impl FnOnce() -> Result<T, String>,
    public: impl FnOnce() -> Result<T, String>,
) -> Result<T, String> {
    if !signed_in {
        return public();
    }
    with_gh().or_else(|gh_said| {
        public().map_err(|public_said| format!("{public_said} (and with gh: {gh_said})"))
    })
}

/// The newest release's tag, e.g. `v0.3.0`.
pub fn latest_tag() -> Result<String, String> {
    gh_then_public(
        gh_signed_in(),
        || {
            crate::proc::run(Command::new("gh").args([
                "release", "view", "-R", REPO, "--json", "tagName", "-q", ".tagName",
            ]))
            .map(|s| s.trim().to_string())
        },
        latest_tag_from_the_public_page,
    )
}

fn latest_tag_from_the_public_page() -> Result<String, String> {
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
    gh_then_public(
        gh_signed_in(),
        || {
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
        },
        || {
            let url = format!("https://github.com/{REPO}/releases/download/{tag}/{name}");
            crate::proc::run(
                system_tool("curl")
                    .args(["-fsSL", "--max-time", "300", "-o"])
                    .arg(&to)
                    .arg(&url),
            )
        },
    )
    .map_err(|e| anyhow::anyhow!("downloading {name}: {e}"))?;
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

/// The tag a person named with `--to`: `0.70.0` and `v0.70.0` are the same release. Anything
/// that is not a version is refused here, before it becomes part of a download address.
fn tag_named(to: &str) -> Result<String> {
    let bare = to.trim().trim_start_matches('v');
    if version(bare).is_none() {
        bail!("`{to}` is not a version; name a release such as 0.70.0");
    }
    Ok(format!("v{bare}"))
}

/// `<app> update`: install the newest release over this one. `check` only says whether
/// there is one; `force` reinstalls it even when it's this version; `to` installs that release
/// instead, older ones included, which is the way back from an update that went wrong.
pub fn run(check: bool, force: bool, to: Option<&str>) -> Result<()> {
    let current = env!("APP_VERSION");
    let tag = match to {
        Some(to) => tag_named(to)?,
        None => latest_tag().map_err(|e| anyhow::anyhow!(e))?,
    };
    if to.is_some() && tag.trim_start_matches('v') == current && !force {
        println!("{APP_COMMAND} {current} is already installed.");
        return Ok(());
    }
    if to.is_none() && !is_newer(&tag) && !force {
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
    let (cfg, _) = crate::config::Config::load_or_default();
    let machines: Vec<&str> = cfg.machines.iter().map(|m| m.name.as_str()).collect();
    if let Some(note) = machines_note(&machines) {
        println!("{note}");
    }
    Ok(())
}

/// What is left to update after this computer: each configured machine runs its own copy, and
/// one on another version cannot be talked to until it has this one.
fn machines_note(names: &[&str]) -> Option<String> {
    let mut names = names.to_vec();
    names.sort_by_key(|name| name.to_lowercase());
    let commands: Vec<String> = names
        .iter()
        .map(|name| format!("  {APP_COMMAND} machine update {name}"))
        .collect();
    (!commands.is_empty()).then(|| {
        format!(
            "Your other machines run their own copy and need this version too:
{}",
            commands.join("
")
        )
    })
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

pub(crate) fn integrate_opted_out() -> bool {
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

/// The public key releases are signed with: the desktop updater's (`plugins.updater.pubkey` in
/// the desktop app's `tauri.conf.json`, which a test holds this to), so one key vouches for
/// both apps. A public key is not a secret.
const RELEASE_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDMzNzI5RUU4NTdGMUIxNUMKUldSY3NmRlg2SjV5TTF5VTJWUTQ1VnlHSlBWTU9uYnl5ZW50UzQzSFVXcU5zRlhqOGZIZXpucUsK";

/// The first release whose `sha256sums.txt` is signed. An older one, which only `--to` reaches,
/// is checked by its checksum alone.
const SIGNED_SINCE: &str = "0.71.0";

/// The file beside `sha256sums.txt` that holds its signature.
const SUMS_SIGNATURE: &str = "sha256sums.txt.sig";

/// Whether the release at `tag` must come with a signature.
fn is_signed(tag: &str) -> bool {
    !newer_than(SIGNED_SINCE, tag)
}

/// Check that `signature` (as the release workflow writes it: a minisign signature, base64) is
/// `key`'s over exactly these bytes. The checksums vouch for every download, so this is what
/// ties a download to the people who release the app and not only to the server it came from.
fn verify_signed(data: &[u8], signature: &str, key: &str) -> Result<()> {
    use base64::Engine;
    let text = |encoded: &str| -> Result<String> {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(encoded.trim())
            .context("not base64")?;
        String::from_utf8(bytes).context("not text")
    };
    let key = minisign_verify::PublicKey::decode(&text(key)?)
        .map_err(|e| anyhow::anyhow!("reading the release key: {e}"))?;
    let signature = minisign_verify::Signature::decode(&text(signature)?)
        .map_err(|e| anyhow::anyhow!("reading the signature: {e}"))?;
    key.verify(data, &signature, true)
        .map_err(|e| anyhow::anyhow!("the signature does not match: {e}"))
}

/// `<app> verify-release <sha256sums.txt>`: check the signature beside a checksum file with the
/// key built into this program. The release workflow runs it on what it is about to publish, so
/// a release this app would refuse to update to is never published.
pub fn verify_release(sums: &Path) -> Result<()> {
    let signature = sums.with_file_name(SUMS_SIGNATURE);
    let data = std::fs::read(sums).with_context(|| format!("reading {}", sums.display()))?;
    let signature = std::fs::read_to_string(&signature)
        .with_context(|| format!("reading {}", signature.display()))?;
    verify_signed(&data, &signature, RELEASE_KEY)?;
    println!("{} is signed with {APP_NAME}'s release key.", sums.display());
    Ok(())
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
    if is_signed(tag) {
        let signature = std::fs::read_to_string(download(tag, SUMS_SIGNATURE, tmp)?)?;
        verify_signed(sums.as_bytes(), &signature, RELEASE_KEY).with_context(|| {
            format!("this release's checksums are not signed with {APP_NAME}'s key; nothing was changed")
        })?;
    }
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
    runs_as(&new, tag)?;
    let exe = std::env::current_exe().context(concat!("finding this ", env!("APP_COMMAND")))?;
    at_step(4);
    replace_exe(&new, &exe)?;
    Ok(exe)
}

/// Run the downloaded program before it replaces this one: a build that does not start here (the
/// wrong one for this computer, or one a virus scanner broke) must not become the installed app.
fn runs_as(new: &Path, tag: &str) -> Result<()> {
    let said = crate::proc::run(crate::proc::quiet(Command::new(new).arg("--version")))
        .map_err(|why| {
            anyhow::anyhow!("the download does not run on this computer ({why}); nothing was changed")
        })?;
    says_version(&said, tag)
}

/// Whether `--version` output names the release that was asked for.
fn says_version(said: &str, tag: &str) -> Result<()> {
    let wanted = format!("{APP_COMMAND} {}", tag.trim_start_matches('v'));
    if said.trim() != wanted {
        bail!(
            "the download says it is `{}`, not `{wanted}`; nothing was changed",
            said.trim()
        );
    }
    Ok(())
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
pub(crate) fn free_old_path(exe: &Path) -> PathBuf {
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
    let json = gh_then_public(
        gh_signed_in(),
        || crate::proc::run(Command::new("gh").args(["api", &path])),
        || {
            let url = format!("https://api.github.com/{path}");
            crate::proc::run(system_tool("curl").args([
                "-fsSL",
                "--max-time",
                "20",
                "-H",
                "Accept: application/vnd.github+json",
                &url,
            ]))
        },
    );
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

    /// A key and a signature made for this test with the same tool the release uses; the private
    /// half was thrown away.
    const TEST_KEY: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IG1pbmlzaWduIHB1YmxpYyBrZXk6IDY0OTA3MTExNDdERjgzQTEKUldTaGc5OUhFWEdRWkIzRFZ0WEx4amt1TStGUk1qVWJ2Ky9EMm5ubmZHN2c5NjlYNTc2aWZmbDQK";
    const TEST_SIGNATURE: &str = "dW50cnVzdGVkIGNvbW1lbnQ6IHNpZ25hdHVyZSBmcm9tIHRhdXJpIHNlY3JldCBrZXkKUlVTaGc5OUhFWEdRWkx6VGdxS2t1VnhJUGRPZ2ZPemc0WEU0S2YyaU5TenloWlpTOVRqWnVXZHd0SXlySjl4ZTZFT0JtWGt0L1gwem5obFY0cXJtU2hkRmZBU0Q2ZjFYcHcwPQp0cnVzdGVkIGNvbW1lbnQ6IHRpbWVzdGFtcDoxNzk0MzE1OTE0CWZpbGU6c2hhMjU2c3Vtcy50eHQKU0t6OU5ZZ1h2MnFEVDJhZFh1OXEyV0gxMW9OVXFpZDFsanlISlM2RmZGU2tFbno3cm9RZS9KY2FtUEtPeVFXQXJxdFJmWmpNYzdsZGMrbFhwcjJRQkE9PQo=";
    const TEST_SUMS: &[u8] = b"abc123  maitty-x86_64-pc-windows-msvc.zip\n";

    #[test]
    fn signed_checksums_verify_and_changed_ones_do_not() {
        verify_signed(TEST_SUMS, TEST_SIGNATURE, TEST_KEY).unwrap();
        let changed = b"abc124  maitty-x86_64-pc-windows-msvc.zip\n";
        let error = verify_signed(changed, TEST_SIGNATURE, TEST_KEY).unwrap_err();
        assert!(error.to_string().contains("does not match"), "{error}");
    }

    #[test]
    fn a_signature_from_another_key_is_refused() {
        assert!(verify_signed(TEST_SUMS, TEST_SIGNATURE, RELEASE_KEY).is_err());
        assert!(verify_signed(TEST_SUMS, "not a signature", TEST_KEY).is_err());
        assert!(verify_signed(TEST_SUMS, "", TEST_KEY).is_err());
    }

    #[test]
    fn the_release_key_is_the_desktop_updaters() {
        let conf = concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/gui/src-tauri/tauri.conf.json");
        let conf = std::fs::read_to_string(conf).unwrap();
        assert!(conf.contains(&format!("\"pubkey\": \"{RELEASE_KEY}\"")));
    }

    #[test]
    fn releases_are_signed_from_the_first_signed_one_on() {
        assert!(!is_signed("v0.70.0"));
        assert!(is_signed(&format!("v{SIGNED_SINCE}")));
        assert!(is_signed("v0.71.1"));
        assert!(is_signed("v1.0.0"));
    }

    #[test]
    fn a_named_release_is_a_version_with_or_without_its_v() {
        assert_eq!(tag_named("0.70.0").unwrap(), "v0.70.0");
        assert_eq!(tag_named(" v0.70.0 ").unwrap(), "v0.70.0");
        assert!(tag_named("latest").is_err());
        assert!(tag_named("../../other").is_err());
        assert!(tag_named("").is_err());
    }

    #[test]
    fn a_download_that_is_another_version_is_not_installed() {
        let ours = format!("{APP_COMMAND} 1.2.3\n");
        assert!(says_version(&ours, "v1.2.3").is_ok());
        let error = says_version(&ours, "v1.2.4").unwrap_err().to_string();
        assert!(error.contains("nothing was changed"), "{error}");
        assert!(says_version("something else entirely", "v1.2.3").is_err());
    }

    #[test]
    fn a_download_that_does_not_start_is_not_installed() {
        let dir = std::env::temp_dir().join(format!(
            "{APP_COMMAND}-update-test-{}-broken",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        let broken = dir.join(if cfg!(windows) { "broken.exe" } else { "broken" });
        std::fs::write(&broken, b"not a program").unwrap();
        let error = runs_as(&broken, "v1.2.3").unwrap_err().to_string();
        assert!(error.contains("does not run on this computer"), "{error}");
        assert!(error.contains("nothing was changed"), "{error}");
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn an_update_names_the_command_for_each_machine_and_says_nothing_with_none() {
        assert_eq!(machines_note(&[]), None);
        let note = machines_note(&["kingdom", "Build"]).unwrap();
        assert!(
            note.ends_with(&format!(
                "  {APP_COMMAND} machine update Build
  {APP_COMMAND} machine update kingdom"
            )),
            "{note}"
        );
    }

    #[test]
    fn a_failed_gh_call_falls_back_to_the_public_address() {
        let ok = |s: &str| Ok::<String, String>(s.to_string());
        let no = |s: &str| Err::<String, String>(s.to_string());
        assert_eq!(gh_then_public(true, || ok("gh"), || ok("public")), ok("gh"));
        assert_eq!(gh_then_public(true, || no("expired"), || ok("public")), ok("public"));
        assert_eq!(
            gh_then_public(false, || -> Result<String, String> { panic!("gh is not signed in") }, || ok("public")),
            ok("public")
        );
        assert_eq!(
            gh_then_public(true, || no("expired"), || no("offline")),
            no("offline (and with gh: expired)")
        );
    }

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
