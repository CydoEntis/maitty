//! Answers to `Query::Repo`: read-only looks at the checkout a pane is in.
//! Everything here blocks; callers run it on a blocking thread.

use super::git::{
    bounded_diff, conversation_diff, conversation_file, conversation_files, git,
    resolve_conversation_path,
};
use crate::repo::{
    ChangeKind, RepoBranch, RepoChange, RepoCommit, RepoReply, RepoRequest, RepoStatus,
};
use anyhow::{Result, bail};
use std::path::Path;

const MAX_LOG: usize = 200;
const FIELD: char = '\x1f';
const RECORD: char = '\x1e';

/// `dir` is where the pane is, which may be a folder inside the repository; paths in a request
/// are relative to the repository's top.
pub fn answer(dir: &Path, request: &RepoRequest) -> Result<RepoReply> {
    let top = git(dir, &["rev-parse", "--show-toplevel"])?;
    let root = Path::new(&top);
    Ok(match request {
        RepoRequest::Status => RepoReply::Status(status(root)?),
        RepoRequest::Tree { path } => RepoReply::Tree(conversation_files(root, path.as_deref())?),
        RepoRequest::File { path } => RepoReply::File(conversation_file(root, path)?),
        RepoRequest::Diff { path } => RepoReply::Diff(diff(root, path.as_deref())?),
        RepoRequest::DiffRange { from, to, path } => {
            RepoReply::Diff(diff_range(root, from, to, path.as_deref())?)
        }
        RepoRequest::Branches => RepoReply::Branches(branches(root)?),
        RepoRequest::Log { limit } => RepoReply::Log(log(root, *limit)?),
    })
}

fn status(root: &Path) -> Result<RepoStatus> {
    let out = git(
        root,
        &["status", "--porcelain=v1", "--branch", "-z", "--untracked-files=all"],
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
        || revision.chars().any(|c| c.is_whitespace() || c.is_control())
    {
        bail!("That is not a branch or commit name");
    }
    Ok(())
}

fn branches(root: &Path) -> Result<Vec<RepoBranch>> {
    let out = git(
        root,
        &["for-each-ref", "--format=%(HEAD)%(refname:short)", "refs/heads"],
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

    /// A repository with one commit on `main` holding `a.txt`.
    fn repository() -> crate::daemon::persist::ScratchDir {
        let repo = crate::daemon::persist::scratch_dir();
        std::fs::create_dir_all(repo.path()).expect("repository directory");
        run(repo.path(), &["init", "--quiet", "--initial-branch=main"]);
        run(repo.path(), &["config", "user.name", "Test User"]);
        run(repo.path(), &["config", "user.email", "test@example.invalid"]);
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
        let kinds: Vec<_> = status.files.iter().map(|f| (f.path.as_str(), f.kind)).collect();
        assert_eq!(
            kinds,
            [("a.txt", ChangeKind::Modified), ("new.txt", ChangeKind::Untracked)]
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
        assert_eq!(parse_branch_line("main...origin/main [gone]"), (Some("main".into()), 0, 0));
        assert_eq!(parse_branch_line("No commits yet on main"), (Some("main".into()), 0, 0));
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

        let RepoReply::Branches(list) = answer(repo.path(), &RepoRequest::Branches).unwrap()
        else {
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
        assert!(answer(repo.path(), &RepoRequest::File { path: "../x".into() }).is_err());
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
        assert_eq!(reply, RepoReply::File("one
".into()));
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
