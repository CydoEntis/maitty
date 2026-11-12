//! Answers to `Query::Repo`: read-only looks at the checkout a pane is in.
//! Everything here blocks; callers run it on a blocking thread.

use super::git::{
    bounded_diff, conversation_diff, conversation_file, conversation_files, git,
    resolve_conversation_path,
};
use crate::repo::{
    ChangeKind, RepoBranch, RepoChange, RepoCommit, RepoReference, RepoReply, RepoRequest,
    RepoSnapshot, RepoStatus,
};
use anyhow::{Context, Result, bail};
use std::path::Path;

const MAX_LOG: usize = 200;
const FIELD: char = '\x1f';
const RECORD: char = '\x1e';

/// `dir` is where the pane is, which may be a folder inside the repository; paths in a request
/// are relative to the repository's top.
pub fn answer(dir: &Path, request: &RepoRequest) -> Result<RepoReply> {
    // Reading files needs no git: a folder that is not a repository is browsed from where the pane is. Everything
    // else about a repository still needs one.
    let top = match git(dir, &["rev-parse", "--show-toplevel"]) {
        Ok(top) => top,
        Err(error) => match request {
            RepoRequest::Tree { .. } | RepoRequest::File { .. } if dir.is_dir() => {
                dir.to_string_lossy().into_owned()
            }
            _ => return Err(error),
        },
    };
    let root = Path::new(&top);
    Ok(match request {
        RepoRequest::Status => RepoReply::Status(status(root)?),
        RepoRequest::Tree { path } => RepoReply::Tree(conversation_files(root, path.as_deref())?),
        RepoRequest::File { path } => RepoReply::File(conversation_file(root, path)?),
        RepoRequest::Locate { path } => RepoReply::Located(locate(dir, root, path)?),
        RepoRequest::Diff { path } => RepoReply::Diff(diff(root, path.as_deref())?),
        RepoRequest::DiffRange { from, to, path } => {
            RepoReply::Diff(diff_range(root, from, to, path.as_deref())?)
        }
        RepoRequest::Branches => RepoReply::Branches(branches(root)?),
        RepoRequest::Log { limit } => RepoReply::Log(log(root, *limit)?),
        RepoRequest::Worktrees => RepoReply::Worktrees(super::git::list_worktrees(root)?),
        RepoRequest::Snapshots => RepoReply::Snapshots(
            crate::snapshot::list(root)
                .into_iter()
                .map(|s| RepoSnapshot {
                    commit: s.commit,
                    at: s.at,
                    why: s.why,
                })
                .collect(),
        ),
        RepoRequest::References => RepoReply::References(references(root)?),
    })
}

/// Put the checkout back as it was in one of its own snapshots (never an arbitrary commit).
pub fn restore(dir: &Path, commit: &str) -> Result<RepoReply> {
    if !crate::snapshot::list(dir)
        .iter()
        .any(|s| s.commit == commit)
    {
        bail!("That is not one of this checkout's snapshots");
    }
    let changed = crate::snapshot::restore(dir, commit).map_err(anyhow::Error::msg)?;
    Ok(RepoReply::Restored(changed))
}

/// Replace the text of an existing file of the checkout `dir` is in. The path is checked as every
/// read of a project file is (relative, no `..`, no links), and `.git` is never written. The new
/// text is put beside the file and moved over it, so a failure leaves the old file whole.
pub fn write_file(dir: &Path, relative: &str, contents: &str) -> Result<RepoReply> {
    let top = git(dir, &["rev-parse", "--show-toplevel"]).unwrap_or_else(|_| dir.to_string_lossy().into_owned());
    super::git::conversation_write(Path::new(&top), relative, contents)?;
    Ok(RepoReply::Written)
}

fn references(root: &Path) -> Result<Vec<RepoReference>> {
    let project = crate::references::project_root(root);
    let found = crate::references::of_project(&project).map_err(anyhow::Error::msg)?;
    Ok(found
        .items
        .into_iter()
        .map(|r| RepoReference {
            name: r.name,
            kind: r.kind.label().to_string(),
            bytes: r.size,
            modified: r.modified,
        })
        .collect())
}

/// `path` as seen in a pane in `dir`, made relative to the repository top `root`.
fn locate(dir: &Path, root: &Path, path: &str) -> Result<String> {
    let wanted = Path::new(path);
    let full = if wanted.is_absolute() {
        wanted.to_path_buf()
    } else {
        dir.join(wanted)
    };
    let full = full.canonicalize().context("That file does not exist")?;
    let root = root.canonicalize().context("opening the repository")?;
    let inside = full
        .strip_prefix(&root)
        .map_err(|_| anyhow::anyhow!("That file is outside this repository"))?;
    if !full.is_file() {
        bail!("That path is not a file");
    }
    Ok(inside
        .components()
        .map(|part| part.as_os_str().to_string_lossy())
        .collect::<Vec<_>>()
        .join("/"))
}

fn status(root: &Path) -> Result<RepoStatus> {
    let out = git(
        root,
        &[
            "status",
            "--porcelain=v1",
            "--branch",
            "-z",
            "--untracked-files=all",
        ],
    )?;
    let mut entries = out.split('\0').filter(|entry| !entry.is_empty());
    let (branch, ahead, behind) = entries
        .next()
        .and_then(|head| head.strip_prefix("## "))
        .map(parse_branch_line)
        .unwrap_or((None, 0, 0));
    let mut files = Vec::new();
    while let Some(entry) = entries.next() {
        let Some((code, path)) = entry.split_at_checked(3) else {
            continue;
        };
        let kind = change_kind(code.get(..2).unwrap_or_default());
        if kind == ChangeKind::Renamed {
            // -z lists the old name next; the new one is what the file is called now.
            entries.next();
        }
        files.push(RepoChange {
            path: path.to_string(),
            kind,
        });
    }
    files.sort_by(|a, b| a.path.cmp(&b.path));
    Ok(RepoStatus {
        branch,
        ahead,
        behind,
        files,
    })
}

/// `main...origin/main [ahead 1, behind 2]`, `No commits yet on main` or `HEAD (no branch)`.
fn parse_branch_line(line: &str) -> (Option<String>, u32, u32) {
    if line.starts_with("HEAD (no branch)") {
        return (None, 0, 0);
    }
    let line = line.strip_prefix("No commits yet on ").unwrap_or(line);
    let (names, counts) = match line.split_once(" [") {
        Some((names, counts)) => (names, counts.trim_end_matches(']')),
        None => (line, ""),
    };
    let branch = names.split("...").next().unwrap_or(names).to_string();
    let count = |label: &str| {
        counts
            .split(", ")
            .find_map(|part| part.strip_prefix(label))
            .and_then(|n| n.parse().ok())
            .unwrap_or(0)
    };
    (Some(branch), count("ahead "), count("behind "))
}

fn change_kind(code: &str) -> ChangeKind {
    match code {
        "??" => ChangeKind::Untracked,
        "DD" | "AU" | "UD" | "UA" | "DU" | "AA" | "UU" => ChangeKind::Conflicted,
        _ if code.contains('R') => ChangeKind::Renamed,
        _ if code.contains('A') => ChangeKind::Added,
        _ if code.contains('D') => ChangeKind::Deleted,
        _ => ChangeKind::Modified,
    }
}

fn diff(root: &Path, path: Option<&str>) -> Result<crate::conversation::ConversationDiff> {
    let mut diff = conversation_diff(root)?;
    if let Some(path) = path {
        diff.files.retain(|file| file.path == path);
        diff.additions = diff.files.iter().map(|file| file.additions).sum();
        diff.deletions = diff.files.iter().map(|file| file.deletions).sum();
        diff.total_files = diff.files.len();
        diff.truncated = false;
    }
    Ok(diff)
}

fn diff_range(
    root: &Path,
    from: &str,
    to: &str,
    path: Option<&str>,
) -> Result<crate::conversation::ConversationDiff> {
    check_revision(from)?;
    check_revision(to)?;
    let mut args = vec![
        "diff",
        "--no-ext-diff",
        "--no-color",
        "--src-prefix=a/",
        "--dst-prefix=b/",
        "--unified=3",
        from,
        to,
        "--",
    ];
    if let Some(path) = path {
        resolve_conversation_path(root, path)?;
        args.push(path);
    }
    Ok(bounded_diff(&git(root, &args)?, 0))
}

/// A branch, tag or commit name. One that starts with `-` would be read as an option.
fn check_revision(revision: &str) -> Result<()> {
    if revision.is_empty()
        || revision.starts_with('-')
        || revision
            .chars()
            .any(|c| c.is_whitespace() || c.is_control())
    {
        bail!("That is not a branch or commit name");
    }
    Ok(())
}

fn branches(root: &Path) -> Result<Vec<RepoBranch>> {
    let out = git(
        root,
        &[
            "for-each-ref",
            "--format=%(HEAD)%(refname:short)",
            "refs/heads",
        ],
    )?;
    Ok(out
        .lines()
        .map(|line| RepoBranch {
            current: line.starts_with('*'),
            name: line.trim_start_matches(['*', ' ']).to_string(),
        })
        .collect())
}

fn log(root: &Path, limit: usize) -> Result<Vec<RepoCommit>> {
    if git(root, &["rev-parse", "--verify", "--quiet", "HEAD"]).is_err() {
        return Ok(Vec::new());
    }
    let count = limit.clamp(1, MAX_LOG).to_string();
    let format = format!("--format=%H{FIELD}%s{FIELD}%an{FIELD}%ct{RECORD}");
    let out = git(root, &["log", "-n", &count, &format])?;
    Ok(out
        .split(RECORD)
        .filter_map(|record| {
            let mut fields = record.trim_start().split(FIELD);
            Some(RepoCommit {
                hash: fields.next().filter(|hash| !hash.is_empty())?.to_string(),
                subject: fields.next()?.to_string(),
                author: fields.next()?.to_string(),
                time: fields.next()?.parse().ok()?,
            })
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    fn run(dir: &Path, args: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(dir)
            .args(args)
            .output()
            .expect("git should run");
        assert!(
            output.status.success(),
            "git {args:?}: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn a_file_is_replaced_whole_and_nothing_else_can_be_written() {
        let dir = crate::daemon::persist::scratch_dir();
        let root = dir.path();
        std::fs::create_dir_all(root.join("src")).expect("folder");
        std::fs::create_dir_all(root.join(".git")).expect("git dir");
        std::fs::write(root.join("src/a.txt"), "old").expect("file");
        std::fs::write(root.join(".git/config"), "keep").expect("file");

        assert_eq!(write_file(root, "src/a.txt", "new text\n").expect("written"), RepoReply::Written);
        assert_eq!(std::fs::read_to_string(root.join("src/a.txt")).unwrap(), "new text\n");
        let stray: Vec<_> = std::fs::read_dir(root.join("src")).unwrap().flatten().map(|e| e.file_name()).collect();
        assert_eq!(stray.len(), 1, "no staging file is left behind: {stray:?}");

        // It makes no file, climbs out of nowhere, and leaves .git alone.
        for bad in ["src/new.txt", "../outside.txt", "/etc/passwd", "src", ".git/config", "src/../.git/config"] {
            assert!(write_file(root, bad, "x").is_err(), "{bad} must be refused");
        }
        assert_eq!(std::fs::read_to_string(root.join(".git/config")).unwrap(), "keep");
        assert!(!root.parent().unwrap().join("outside.txt").exists());
        // A text past the preview limit is refused rather than cut.
        assert!(write_file(root, "src/a.txt", &"x".repeat(300 * 1024)).is_err());
    }

    #[cfg(unix)]
    #[test]
    fn a_link_is_never_followed_to_write_through_it() {
        let dir = crate::daemon::persist::scratch_dir();
        let root = dir.path();
        let outside = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(root).expect("folder");
        std::fs::create_dir_all(outside.path()).expect("folder");
        std::fs::write(outside.path().join("secret.txt"), "untouched").expect("file");
        std::os::unix::fs::symlink(outside.path().join("secret.txt"), root.join("link.txt")).expect("link");
        assert!(write_file(root, "link.txt", "changed").is_err());
        assert_eq!(std::fs::read_to_string(outside.path().join("secret.txt")).unwrap(), "untouched");
    }

    #[test]
    fn a_folder_that_is_not_a_repository_can_still_be_browsed_but_not_asked_about_git() {
        let dir = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).expect("folder");
        std::fs::write(dir.path().join("notes.txt"), "hello").expect("file");

        let RepoReply::Tree(entries) = answer(dir.path(), &RepoRequest::Tree { path: None }).expect("tree") else {
            panic!("a tree");
        };
        assert_eq!(entries.iter().map(|e| e.name.as_str()).collect::<Vec<_>>(), ["notes.txt"]);
        let RepoReply::File(text) = answer(dir.path(), &RepoRequest::File { path: "notes.txt".into() }).expect("file") else {
            panic!("a file");
        };
        assert_eq!(text, "hello");
        assert!(answer(dir.path(), &RepoRequest::Status).is_err());
    }

    /// A repository with one commit on `main` holding `a.txt`.
    fn repository() -> crate::daemon::persist::ScratchDir {
        let repo = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(repo.path()).expect("repository directory");
        run(repo.path(), &["init", "--quiet", "--initial-branch=main"]);
        run(repo.path(), &["config", "user.name", "Test User"]);
        run(
            repo.path(),
            &["config", "user.email", "test@example.invalid"],
        );
        std::fs::write(repo.path().join("a.txt"), "one\n").expect("file");
        run(repo.path(), &["add", "a.txt"]);
        run(repo.path(), &["commit", "--quiet", "-m", "first"]);
        repo
    }

    #[test]
    fn status_names_the_branch_and_each_kind_of_change() {
        let repo = repository();
        std::fs::write(repo.path().join("a.txt"), "two\n").expect("edit");
        std::fs::write(repo.path().join("new.txt"), "x\n").expect("untracked");

        let RepoReply::Status(status) = answer(repo.path(), &RepoRequest::Status).unwrap() else {
            panic!("expected a status");
        };
        assert_eq!(status.branch.as_deref(), Some("main"));
        let kinds: Vec<_> = status
            .files
            .iter()
            .map(|f| (f.path.as_str(), f.kind))
            .collect();
        assert_eq!(
            kinds,
            [
                ("a.txt", ChangeKind::Modified),
                ("new.txt", ChangeKind::Untracked)
            ]
        );
    }

    #[test]
    fn a_renamed_file_is_listed_once_under_its_new_name() {
        let repo = repository();
        run(repo.path(), &["mv", "a.txt", "b.txt"]);

        let RepoReply::Status(status) = answer(repo.path(), &RepoRequest::Status).unwrap() else {
            panic!("expected a status");
        };
        assert_eq!(
            status.files,
            [RepoChange {
                path: "b.txt".into(),
                kind: ChangeKind::Renamed
            }]
        );
    }

    #[test]
    fn a_branch_line_reads_distance_from_the_upstream() {
        assert_eq!(
            parse_branch_line("main...origin/main [ahead 2, behind 1]"),
            (Some("main".into()), 2, 1)
        );
        assert_eq!(
            parse_branch_line("main...origin/main [gone]"),
            (Some("main".into()), 0, 0)
        );
        assert_eq!(
            parse_branch_line("No commits yet on main"),
            (Some("main".into()), 0, 0)
        );
        assert_eq!(parse_branch_line("HEAD (no branch)"), (None, 0, 0));
    }

    #[test]
    fn log_lists_newest_first_and_an_empty_repository_has_none() {
        let repo = repository();
        std::fs::write(repo.path().join("a.txt"), "two\n").expect("edit");
        run(repo.path(), &["commit", "--quiet", "-am", "second"]);

        let RepoReply::Log(log) = answer(repo.path(), &RepoRequest::Log { limit: 10 }).unwrap()
        else {
            panic!("expected a log");
        };
        let subjects: Vec<_> = log.iter().map(|c| c.subject.as_str()).collect();
        assert_eq!(subjects, ["second", "first"]);
        assert_eq!(log[0].author, "Test User");

        let empty = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(empty.path()).expect("directory");
        run(empty.path(), &["init", "--quiet"]);
        let RepoReply::Log(none) = answer(empty.path(), &RepoRequest::Log { limit: 10 }).unwrap()
        else {
            panic!("expected a log");
        };
        assert!(none.is_empty());
    }

    #[test]
    fn branches_mark_the_current_one() {
        let repo = repository();
        run(repo.path(), &["branch", "feature"]);

        let RepoReply::Branches(list) = answer(repo.path(), &RepoRequest::Branches).unwrap() else {
            panic!("expected branches");
        };
        let list: Vec<_> = list.iter().map(|b| (b.name.as_str(), b.current)).collect();
        assert_eq!(list, [("feature", false), ("main", true)]);
    }

    #[test]
    fn a_range_diff_compares_two_commits_and_can_be_narrowed_to_a_file() {
        let repo = repository();
        std::fs::write(repo.path().join("a.txt"), "two\n").expect("edit");
        std::fs::write(repo.path().join("b.txt"), "other\n").expect("second file");
        run(repo.path(), &["add", "a.txt", "b.txt"]);
        run(repo.path(), &["commit", "--quiet", "-m", "second"]);

        let both = diff_range(repo.path(), "HEAD~1", "HEAD", None).unwrap();
        assert_eq!(both.files.len(), 2);
        let one = diff_range(repo.path(), "HEAD~1", "HEAD", Some("b.txt")).unwrap();
        assert_eq!(one.files.len(), 1);
        assert_eq!((one.additions, one.deletions), (1, 0));
    }

    #[test]
    fn a_revision_that_looks_like_an_option_or_a_path_that_climbs_out_is_refused() {
        let repo = repository();
        assert!(diff_range(repo.path(), "--output=x", "HEAD", None).is_err());
        assert!(diff_range(repo.path(), "HEAD", "HEAD", Some("../x")).is_err());
        assert!(
            answer(
                repo.path(),
                &RepoRequest::File {
                    path: "../x".into()
                }
            )
            .is_err()
        );
    }

    #[test]
    fn a_pane_in_a_subfolder_reads_paths_from_the_repository_top() {
        let repo = repository();
        std::fs::create_dir(repo.path().join("sub")).expect("subfolder");

        let reply = answer(
            &repo.path().join("sub"),
            &RepoRequest::File {
                path: "a.txt".into(),
            },
        )
        .unwrap();
        assert_eq!(reply, RepoReply::File("one\n".into()));
    }

    #[test]
    fn a_restore_takes_back_a_turn_but_only_to_a_snapshot_the_daemon_took() {
        let repo = repository();
        let snapshot = crate::snapshot::take(repo.path(), "before claude's turn")
            .unwrap()
            .expect("a repository takes snapshots");
        std::fs::write(
            repo.path().join("a.txt"),
            "the agent's edit
",
        )
        .expect("edit");

        let RepoReply::Snapshots(kept) = answer(repo.path(), &RepoRequest::Snapshots).unwrap()
        else {
            panic!("expected snapshots");
        };
        assert_eq!(kept.len(), 1);
        assert_eq!(kept[0].why, "before claude's turn");

        assert!(restore(repo.path(), "0000000000000000000000000000000000000000").is_err());
        assert_eq!(
            restore(repo.path(), &snapshot.commit).unwrap(),
            RepoReply::Restored(1)
        );
        // git may have written the file with this platform's line ending.
        assert_eq!(
            std::fs::read_to_string(repo.path().join("a.txt"))
                .unwrap()
                .trim_end(),
            "one"
        );
    }

    #[test]
    fn references_list_the_project_folder_and_none_is_an_empty_list() {
        let repo = repository();
        assert_eq!(
            answer(repo.path(), &RepoRequest::References).unwrap(),
            RepoReply::References(Vec::new())
        );
        let folder = crate::references::folder(&crate::references::project_root(repo.path()));
        std::fs::create_dir_all(&folder).expect("references folder");
        std::fs::write(folder.join("home.png"), [0u8; 4]).expect("reference");

        let RepoReply::References(found) = answer(repo.path(), &RepoRequest::References).unwrap()
        else {
            panic!("expected references");
        };
        assert_eq!(found.len(), 1);
        assert_eq!(
            (found[0].name.as_str(), found[0].kind.as_str()),
            ("home.png", "image")
        );
    }

    #[test]
    fn locate_finds_a_file_by_absolute_or_pane_relative_path_and_nothing_outside() {
        let repo = repository();
        std::fs::create_dir(repo.path().join("sub")).expect("subfolder");
        std::fs::write(
            repo.path().join("sub/b.txt"),
            "x
",
        )
        .expect("file");
        let outside = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(outside.path()).expect("outside folder");
        std::fs::write(
            outside.path().join("o.txt"),
            "x
",
        )
        .expect("outside file");

        let locate_in =
            |dir: &Path, path: &str| answer(dir, &RepoRequest::Locate { path: path.into() });
        let absolute = repo.path().join("sub").join("b.txt");
        assert_eq!(
            locate_in(repo.path(), &absolute.to_string_lossy()).unwrap(),
            RepoReply::Located("sub/b.txt".into())
        );
        assert_eq!(
            locate_in(&repo.path().join("sub"), "b.txt").unwrap(),
            RepoReply::Located("sub/b.txt".into())
        );
        assert!(locate_in(repo.path(), &outside.path().join("o.txt").to_string_lossy()).is_err());
        assert!(locate_in(repo.path(), "../../etc/passwd").is_err());
        assert!(locate_in(repo.path(), "sub").is_err());
    }

    #[test]
    fn worktrees_list_the_main_checkout_and_each_linked_one() {
        let repo = repository();
        let linked = crate::daemon::persist::scratch_dir();
        run(
            repo.path(),
            &[
                "worktree",
                "add",
                "-b",
                "feature",
                &linked.path().to_string_lossy(),
            ],
        );

        let RepoReply::Worktrees(list) = answer(repo.path(), &RepoRequest::Worktrees).unwrap()
        else {
            panic!("expected worktrees");
        };
        let summary: Vec<_> = list.iter().map(|w| (w.branch.as_str(), w.main)).collect();
        assert_eq!(summary, [("main", true), ("feature", false)]);
    }

    #[test]
    fn a_working_tree_diff_can_be_narrowed_to_one_file() {
        let repo = repository();
        std::fs::write(repo.path().join("a.txt"), "two\n").expect("edit");
        std::fs::write(repo.path().join("new.txt"), "x\n").expect("untracked");

        let one = diff(repo.path(), Some("a.txt")).unwrap();
        assert_eq!(one.files.len(), 1);
        assert_eq!(one.total_files, 1);
    }
}
