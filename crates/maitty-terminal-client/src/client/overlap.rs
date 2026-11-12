//! Two checkouts of one repo changing the same file: agents in different worktrees can't
//! see each other's edits, so it only shows up as a conflict at merge time. Caught here
//! while it's cheap to sort out. The comparison is `maitty_shared::overlap`, shared with the
//! desktop app; this runs it with this client's git.

use std::path::PathBuf;

pub use maitty_shared::overlap::{Overlap, say};

/// The files two or more of these checkouts (name, folder) have each changed since their
/// branches parted, committed or not.
pub fn between(checkouts: &[(String, PathBuf)]) -> Vec<Overlap> {
    maitty_shared::overlap::between(checkouts, &|dir, args| crate::proc::git(dir, args))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::Path;
    use crate::brand::APP_COMMAND;

    fn o(file: &str, checkouts: &[&str]) -> Overlap {
        Overlap {
            file: file.into(),
            checkouts: checkouts.iter().map(|c| c.to_string()).collect(),
        }
    }

    #[test]
    fn overlaps_are_said_in_a_line() {
        assert_eq!(
            say(&[o("src/checkout.ts", &["orders", "rate-limit"])]).as_deref(),
            Some("orders and rate-limit both changed checkout.ts")
        );
        assert_eq!(
            say(&[o("x.rs", &["a", "b", "c"]), o("y.rs", &["a", "b"])]).as_deref(),
            Some("3 checkouts changed x.rs (and 1 more file)")
        );
        assert_eq!(say(&[]), None, "nothing shared: nothing to say");
    }

    /// A repo on `main` with one commit (checkout.ts, shared.ts), and a way to run git in it.
    fn repo(tag: &str) -> (PathBuf, PathBuf, impl Fn(&Path, &[&str]) -> String) {
        let root = std::env::temp_dir().join(format!(
            "{APP_COMMAND}-overlap-{tag}-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&root);
        let repo = root.join("repo");
        std::fs::create_dir_all(&repo).unwrap();
        let git = |dir: &Path, args: &[&str]| crate::proc::git(dir, args).unwrap();
        git(&repo, &["init", "-q", "-b", "main"]);
        git(&repo, &["config", "user.email", "t@t"]);
        git(&repo, &["config", "user.name", "t"]);
        std::fs::write(repo.join("checkout.ts"), "a").unwrap();
        std::fs::write(repo.join("shared.ts"), "a").unwrap();
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-qm", "one"]);
        (root, repo, git)
    }

    #[test]
    fn a_file_two_checkouts_each_changed_is_an_overlap() {
        let (root, repo, git) = repo("each");
        let wt = root.join("orders");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "orders",
                wt.to_str().unwrap(),
            ],
        );
        // The worktree commits a change and adds a new file; the main folder edits the same
        // file without committing, and another one only it touches.
        std::fs::write(wt.join("checkout.ts"), "b").unwrap();
        git(&wt, &["commit", "-qam", "two"]);
        std::fs::write(wt.join("new.ts"), "c").unwrap();
        std::fs::write(repo.join("checkout.ts"), "d").unwrap();
        std::fs::write(repo.join("shared.ts"), "d").unwrap();
        let found = between(&[
            ("main folder".into(), repo.clone()),
            ("orders".into(), wt.clone()),
        ]);
        assert_eq!(
            found,
            vec![o("checkout.ts", &["main folder", "orders"])],
            "only the file both changed"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn what_two_branches_inherited_together_is_not_shared() {
        let (root, repo, git) = repo("inherited");
        // A long-lived branch, dev, far ahead of main: forty files main has never seen.
        git(&repo, &["switch", "-qc", "dev"]);
        for i in 0..40 {
            std::fs::write(repo.join(format!("dev{i}.ts")), "x").unwrap();
        }
        git(&repo, &["add", "."]);
        git(&repo, &["commit", "-qm", "dev work"]);
        // Two agents each cut a branch from dev and change one file of their own, plus one
        // they both touch. The main folder goes back to main, where the app used to measure from.
        let (a, b) = (root.join("a"), root.join("b"));
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat-a",
                a.to_str().unwrap(),
                "dev",
            ],
        );
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat-b",
                b.to_str().unwrap(),
                "dev",
            ],
        );
        git(&repo, &["switch", "-q", "main"]);
        std::fs::write(a.join("only-a.ts"), "a").unwrap();
        std::fs::write(b.join("only-b.ts"), "b").unwrap();
        for dir in [&a, &b] {
            std::fs::write(dir.join("dev0.ts"), dir.to_string_lossy().as_bytes()).unwrap();
            git(dir, &["add", "."]);
            git(dir, &["commit", "-qm", "mine"]);
        }
        let found = between(&[
            ("a".into(), a.clone()),
            ("b".into(), b.clone()),
            ("main folder".into(), repo.clone()),
        ]);
        assert_eq!(
            found,
            vec![o("dev0.ts", &["a", "b"])],
            "the one file both changed, not the forty they both inherited"
        );
        // A branch that's simply behind another shares nothing with it.
        let c = root.join("c");
        git(
            &repo,
            &[
                "worktree",
                "add",
                "-q",
                "-b",
                "feat-c",
                c.to_str().unwrap(),
                "dev",
            ],
        );
        assert!(between(&[("a".into(), a), ("c".into(), c)]).is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
