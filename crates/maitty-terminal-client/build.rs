#[path = "../../build.rs"]
#[allow(dead_code)]
mod app_brand;

/// Runs git in the package folder; `None` when git is missing or the command fails.
fn git(args: &[&str]) -> Option<String> {
    let output = std::process::Command::new("git").args(args).output().ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_string())
}

/// `APP_BUILD`: the commit this was built from, `*` when the tree had changes, and nothing at all
/// for a clean build of a release tag. `APP_BUILT_UNIX`: when. A build that is not a release says
/// so on screen, so a stale one is easy to tell from a fresh one.
fn stamp() {
    let hash = git(&["rev-parse", "--short", "HEAD"]).unwrap_or_else(|| "unknown".into());
    let dirty = git(&["status", "--porcelain"]).is_some_and(|text| !text.is_empty());
    let release = !dirty && git(&["describe", "--exact-match", "--tags", "HEAD"]).is_some();
    let build = if release { String::new() } else { format!("{hash}{}", if dirty { "*" } else { "" }) };
    let built = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    println!("cargo:rustc-env=APP_BUILD={build}");
    println!("cargo:rustc-env=APP_BUILT_UNIX={built}");
    // Watching the sources makes this rerun whenever they change, so the stamp is never older than the code.
    println!("cargo:rerun-if-changed=src");
}

fn main() {
    stamp();
    app_brand::configure(
        &std::path::PathBuf::from(
            std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo sets CARGO_MANIFEST_DIR"),
        )
        .join("../../apps/tui/Cargo.toml"),
    );
}
