//! What this window may ask another machine's daemon about a pane's repository, and what that
//! daemon may answer. The machine is someone else's computer (ADR 0007): only what changed and the
//! diff of it are asked for, only those two answers are accepted, and every string in them is made
//! safe to draw before anything reads it. A fetch, pull or push there answers one line, cleaned the
//! same way.

use crate::remote_chat::cleaned;
use maitty_shared::hidden::sanitize;
use maitty_shared::protocol::Reply;
use maitty_shared::repo::{RepoReply, RepoRequest};

/// Longest path or branch name kept, in characters.
const NAME_MAX_CHARS: usize = 4096;
/// Longest line kept of how a fetch, pull or push ended, in characters.
const LINE_MAX_CHARS: usize = 512;

/// Whether `request` may be asked of another machine: the changed files and their diff. Reading a
/// file whole, listing folders, branches and the rest stay this computer's.
pub fn may_ask(request: &RepoRequest) -> bool {
    matches!(request, RepoRequest::Status | RepoRequest::Diff { .. })
}

/// `reply` made safe, or why it is refused. Only an answer to `may_ask`'s questions passes: a
/// status or a diff. A path or a branch is one clean line; a patch keeps its lines.
pub fn vet_repo_reply(reply: Reply) -> Result<Reply, String> {
    let Reply::Repo { tag, reply } = reply else {
        return Err("it is not an answer about a repository".into());
    };
    let reply = match cleaned(reply)? {
        RepoReply::Status(mut status) => {
            status.branch = status.branch.map(one_line);
            for file in &mut status.files {
                file.path = one_line(std::mem::take(&mut file.path));
            }
            RepoReply::Status(status)
        }
        RepoReply::Diff(mut diff) => {
            for file in &mut diff.files {
                file.path = one_line(std::mem::take(&mut file.path));
            }
            RepoReply::Diff(diff)
        }
        _ => return Err("it is not what was asked about the repository".into()),
    };
    Ok(Reply::Repo { tag, reply })
}

/// `reply` made safe, or why it is refused. Only how a fetch, pull or push ended passes, as one
/// clean line either way.
pub fn vet_git_reply(reply: Reply) -> Result<Reply, String> {
    let Reply::GitDone { tag, outcome } = reply else {
        return Err("it is not an answer about a fetch, pull or push".into());
    };
    let line = |text: String| one_line(text).chars().take(LINE_MAX_CHARS).collect::<String>();
    Ok(Reply::GitDone { tag, outcome: outcome.map(line).map_err(line) })
}

fn one_line(text: String) -> String {
    sanitize(&text, false).chars().take(NAME_MAX_CHARS).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_shared::conversation::{ConversationDiff, ConversationDiffFile};
    use maitty_shared::repo::{ChangeKind, RepoChange, RepoStatus};

    #[test]
    fn only_the_changes_and_their_diff_are_asked_of_another_machine() {
        assert!(may_ask(&RepoRequest::Status));
        assert!(may_ask(&RepoRequest::Diff { path: Some("src/a.rs".into()) }));
        for local in [
            RepoRequest::Tree { path: None },
            RepoRequest::File { path: "secrets.env".into() },
            RepoRequest::Branches,
            RepoRequest::Log { limit: 5 },
            RepoRequest::Worktrees,
            RepoRequest::References,
        ] {
            assert!(!may_ask(&local), "{local:?}");
        }
    }

    #[test]
    fn a_status_from_another_machine_has_clean_one_line_paths() {
        let status = RepoStatus {
            branch: Some("main\u{1b}[31m\nx".into()),
            ahead: 1,
            behind: 2,
            files: vec![RepoChange { path: "src/a\u{7}\n.rs".into(), kind: ChangeKind::Modified }],
        };
        let Ok(Reply::Repo { tag: 7, reply: RepoReply::Status(status) }) = vet_repo_reply(Reply::Repo { tag: 7, reply: RepoReply::Status(status) }) else {
            panic!("a status is an answer that was asked for");
        };
        assert_eq!(status.branch.as_deref(), Some("main[31m x"));
        assert_eq!(status.files[0].path, "src/a .rs");
        assert_eq!((status.ahead, status.behind), (1, 2));
    }

    #[test]
    fn a_diff_from_another_machine_keeps_its_lines_and_loses_escapes() {
        let diff = ConversationDiff {
            files: vec![ConversationDiffFile {
                path: "a\u{1b}]0;x\u{7}.rs".into(),
                additions: 1,
                deletions: 0,
                patch: "@@ -1 +1 @@\n-old\n+new\u{1b}[2J\n".into(),
                binary: false,
            }],
            additions: 1,
            deletions: 0,
            total_files: 1,
            truncated: false,
        };
        let Ok(Reply::Repo { reply: RepoReply::Diff(diff), .. }) = vet_repo_reply(Reply::Repo { tag: 1, reply: RepoReply::Diff(diff) }) else {
            panic!("a diff is an answer that was asked for");
        };
        assert_eq!(diff.files[0].path, "a]0;x.rs");
        assert_eq!(diff.files[0].patch, "@@ -1 +1 @@\n-old\n+new[2J\n");
    }

    #[test]
    fn how_a_fetch_pull_or_push_ended_is_one_clean_line_and_nothing_else_passes() {
        let long = format!("fatal: \u{1b}[31mno\nway {}", "x".repeat(2000));
        for outcome in [Ok("Pushed ma\u{7}in".to_string()), Err(long)] {
            let Ok(Reply::GitDone { tag: 3, outcome }) = vet_git_reply(Reply::GitDone { tag: 3, outcome }) else {
                panic!("how it ended is an answer that was asked for");
            };
            let line = outcome.unwrap_or_else(|why| why);
            assert!(!line.contains(['\u{1b}', '\u{7}', '\n']) && line.chars().count() <= LINE_MAX_CHARS, "{line:?}");
        }
        assert!(vet_git_reply(Reply::Ok).is_err());
        assert!(vet_git_reply(Reply::Repo { tag: 1, reply: RepoReply::Written }).is_err());
    }

    #[test]
    fn any_other_answer_is_refused() {
        assert!(vet_repo_reply(Reply::Ok).is_err());
        assert!(vet_repo_reply(Reply::Repo { tag: 1, reply: RepoReply::File("the file's text".into()) }).is_err());
        assert!(vet_repo_reply(Reply::Repo { tag: 1, reply: RepoReply::Written }).is_err());
    }
}
