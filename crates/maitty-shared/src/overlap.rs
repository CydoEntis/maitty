//! Two checkouts of one repo changing the same file: agents in different worktrees can't
//! see each other's edits, so it only shows up as a conflict at merge time. Caught here
//! while it's cheap to sort out.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// Runs git in a folder with arguments and answers its output, or why it failed.
pub type Git<'a> = &'a dyn Fn(&Path, &[&str]) -> Result<String, String>;

/// One file changed in more than one checkout.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, serde::Serialize)]
pub struct Overlap {
    /// Repo-relative, with `/`.
    pub file: String,
    /// The checkouts that changed it (their names), sorted.
    pub checkouts: Vec<String>,
}

/// What a checkout has that isn't committed: tracked files it changed, and new files git
/// doesn't ignore.
fn uncommitted(dir: &Path, git: Git) -> Vec<String> {
    let mut files: Vec<String> = Vec::new();
    for args in [
        &["diff", "--name-only", "HEAD"][..],
        &["ls-files", "--others", "--exclude-standard"][..],
    ] {
        files.extend(names(git(dir, args)));
    }
    files
}

fn names(out: Result<String, String>) -> Vec<String> {
    out.map(|o| {
        o.lines()
            .map(str::trim)
            .filter(|l| !l.is_empty())
            .map(String::from)
            .collect()
    })
    .unwrap_or_default()
}

/// The files two or more of these checkouts (name, folder) have each changed since their
/// branches parted, committed or not.
///
/// Each pair is measured from the commit the two last had in common. Measured from one
/// fixed branch instead, everything a branch inherited from the branch it was cut from
/// counts as its own change, and two branches cut from the same place "share" hundreds of
/// files neither of them touched.
pub fn between(checkouts: &[(String, PathBuf)], git: Git) -> Vec<Overlap> {
    let head = |dir: &Path| {
        git(dir, &["rev-parse", "HEAD"])
            .ok()
            .map(|h| h.trim().to_string())
    };
    let at: Vec<(&String, &PathBuf, Option<String>, Vec<String>)> = checkouts
        .iter()
        .map(|(name, dir)| (name, dir, head(dir), uncommitted(dir, git)))
        .collect();
    // What a checkout committed since a commit: asked once per (checkout, commit).
    let mut since: BTreeMap<(usize, String), Vec<String>> = BTreeMap::new();
    let mut changed = |i: usize, base: &str| -> Vec<String> {
        let committed = since.entry((i, base.to_string())).or_insert_with(|| {
            names(git(
                at[i].1,
                &["diff", "--name-only", base, "HEAD"],
            ))
        });
        let mut files: Vec<String> = committed.iter().chain(at[i].3.iter()).cloned().collect();
        files.sort();
        files.dedup();
        files
    };
    let mut by_file: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for a in 0..at.len() {
        for b in a + 1..at.len() {
            let (Some(ha), Some(hb)) = (&at[a].2, &at[b].2) else {
                continue;
            };
            let Ok(base) = git(at[a].1, &["merge-base", ha, hb]) else {
                continue;
            };
            let (fa, fb) = (changed(a, base.trim()), changed(b, base.trim()));
            for file in fa.into_iter().filter(|f| fb.binary_search(f).is_ok()) {
                let who = by_file.entry(file).or_default();
                for name in [at[a].0, at[b].0] {
                    if !who.contains(name) {
                        who.push(name.clone());
                    }
                }
            }
        }
    }
    by_file
        .into_iter()
        .map(|(file, mut checkouts)| {
            checkouts.sort();
            Overlap { file, checkouts }
        })
        .collect()
}

/// "orders and rate-limit both changed checkout.ts" (and how many more files).
pub fn say(overlaps: &[Overlap]) -> Option<String> {
    let first = overlaps.first()?;
    let who = match first.checkouts.as_slice() {
        [a, b] => format!("{a} and {b} both"),
        many => format!("{} checkouts", many.len()),
    };
    let name = first.file.rsplit('/').next().unwrap_or(&first.file);
    let more = match overlaps.len() - 1 {
        0 => String::new(),
        n => format!(" (and {n} more file{})", if n == 1 { "" } else { "s" }),
    };
    Some(format!("{who} changed {name}{more}"))
}
