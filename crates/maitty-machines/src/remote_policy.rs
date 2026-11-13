//! What a message from another machine's daemon may do on this one. That daemon is someone
//! else's computer: it can forge any message, so each kind is decided here, before it reaches
//! the code that acts on it, and the default for anything unjustified is to ignore it.
//!
//! - Apply: changes only what the window keeps for that machine (its snapshot, its panes'
//!   screens), after cleaning.
//! - Gated: acts on this computer (clipboard, toasts, window focus, sounds), so only when the
//!   machine's `[[machines]]` entry opts in. A machine nothing names has nothing on.
//! - Answer: replies to something this window sent that machine, so only while one is waiting.
//!   What it says is cleaned like any toast or screen text.
//! - Ignore: would touch this computer's state, UI or files, or was never asked for.

use crate::def::MachineDef;
use crate::remote_snapshot::{ITEMS_MAX, PANE_DIM_MAX, sanitize_snapshot, sanitize_terms};
use maitty_shared::hidden::sanitize;
use maitty_shared::github::{clone_address, is_login, is_time};
use maitty_shared::protocol::{GitHubAnswer, GitHubRepoList, Reply, ServerMsg};

/// Most bytes of a remote clipboard write that are honoured.
pub const CLIPBOARD_MAX_BYTES: usize = 64 * 1024;
/// Longest remote toast, in characters.
pub const NOTICE_MAX: usize = 300;
/// Most bytes of a remote reply's text (a check's output) that are kept.
pub const REPLY_TEXT_MAX: usize = 256 * 1024;

/// What a machine is allowed to do here. Raise (window focus) rides on `notices`: it is the
/// same "this machine may get your attention" grant.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MachineOpts {
    pub clipboard: bool,
    pub notices: bool,
    pub sounds: bool,
}

impl MachineOpts {
    /// This computer's own daemon is trusted with all of it.
    pub const LOCAL: MachineOpts = MachineOpts {
        clipboard: true,
        notices: true,
        sounds: true,
    };

    /// The opt-ins of the entry called `name`; nothing on when none is.
    pub fn named(machines: &[MachineDef], name: &str) -> MachineOpts {
        machines
            .iter()
            .find(|m| !name.is_empty() && m.name.eq_ignore_ascii_case(name))
            .map_or(MachineOpts::default(), |m| MachineOpts {
                clipboard: m.clipboard,
                notices: m.notices,
                sounds: m.sounds,
            })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verdict {
    Apply,
    Answer,
    Ignore(&'static str),
}

fn gate(on: bool, off: &'static str) -> Verdict {
    if on {
        Verdict::Apply
    } else {
        Verdict::Ignore(off)
    }
}

/// Whether a message from a machine that is not this one may be acted on. Every variant is
/// named, so a new message cannot be added without deciding here what a stranger may do with it.
pub fn allow(opts: &MachineOpts, msg: &ServerMsg) -> Verdict {
    match msg {
        ServerMsg::Welcome { .. }
        | ServerMsg::State(_)
        | ServerMsg::TermsChanged { .. }
        | ServerMsg::Replay { .. }
        | ServerMsg::Output { .. }
        // Queues that machine's alert; the sound and the toast are gated where they fire.
        | ServerMsg::Attention { .. }
        | ServerMsg::Bye => Verdict::Apply,
        ServerMsg::Clipboard { .. } => gate(opts.clipboard, "clipboard is off for this machine"),
        ServerMsg::Notice(_) => gate(opts.notices, "notices are off for this machine"),
        ServerMsg::Raise => gate(opts.notices, "notices are off for this machine"),
        ServerMsg::Error(_) | ServerMsg::Reply(_) => Verdict::Answer,
        ServerMsg::AskRemoveWorktree { .. } => {
            Verdict::Ignore("a worktree prompt names a folder on this computer")
        }
        ServerMsg::SettingsChanged(_) => {
            Verdict::Ignore("the settings shown are this computer's, not another machine's")
        }
        ServerMsg::AutoWorkspace(_) => {
            Verdict::Ignore("a new workspace here is not another machine's to announce")
        }
        ServerMsg::ConversationChanged { .. } => {
            Verdict::Ignore("the chats read here are this computer's, not another machine's")
        }
    }
}

/// The message to act on, or `None` when `machine`'s daemon may not send it. What stays is
/// cleaned: the snapshot, sizes that allocate a screen, the clipboard text, the toast. `asked`
/// is whether this window has a request out to that machine that is still waiting for its answer.
pub fn vet(opts: &MachineOpts, asked: bool, machine: &str, msg: ServerMsg) -> Option<ServerMsg> {
    match allow(opts, &msg) {
        Verdict::Ignore(why) => {
            tracing::debug!(why, "ignored a message from another machine");
            return None;
        }
        Verdict::Answer if !asked => {
            tracing::debug!("ignored an answer to nothing this window asked");
            return None;
        }
        Verdict::Answer | Verdict::Apply => {}
    }
    Some(match msg {
        ServerMsg::State(snap) => match sanitize_snapshot(snap) {
            Ok(snap) => ServerMsg::State(snap),
            Err(why) => {
                tracing::debug!(why, "refused a snapshot from another machine");
                return None;
            }
        },
        ServerMsg::TermsChanged { changed, removed } => {
            if removed.len() > ITEMS_MAX {
                tracing::debug!("refused a pane delta from another machine: too many removals");
                return None;
            }
            match sanitize_terms(changed) {
                Ok(changed) => ServerMsg::TermsChanged { changed, removed },
                Err(why) => {
                    tracing::debug!(why, "refused a pane delta from another machine");
                    return None;
                }
            }
        }
        ServerMsg::Replay {
            term,
            cols,
            rows,
            data,
        } => ServerMsg::Replay {
            term,
            cols: cols.clamp(1, PANE_DIM_MAX),
            rows: rows.clamp(1, PANE_DIM_MAX),
            data,
        },
        ServerMsg::Clipboard { term, text } => ServerMsg::Clipboard {
            term,
            text: cut_bytes(text, CLIPBOARD_MAX_BYTES),
        },
        ServerMsg::Notice(text) => ServerMsg::Notice(tagged(machine, &text)),
        ServerMsg::Error(text) => ServerMsg::Error(tagged(machine, &text)),
        ServerMsg::Reply(Reply::Text(text)) => ServerMsg::Reply(Reply::Text(sanitize(
            &cut_bytes(text, REPLY_TEXT_MAX),
            true,
        ))),
        ServerMsg::Reply(Reply::Folders { tag, listing }) => {
            ServerMsg::Reply(Reply::Folders {
                tag,
                listing: sanitize_listing(listing),
            })
        }
        ServerMsg::Reply(Reply::Cloned { tag, outcome }) => ServerMsg::Reply(Reply::Cloned {
            tag,
            outcome: match outcome {
                Ok(path) => Ok(sanitize(&cut_bytes(path, PATH_MAX), false)),
                Err(why) => Err(sanitize(&cut_bytes(why, NAME_MAX), false)),
            },
        }),
        ServerMsg::Reply(Reply::GitDone { tag, outcome }) => {
            let line = |text: String| sanitize(&cut_bytes(text, NAME_MAX), false);
            ServerMsg::Reply(Reply::GitDone {
                tag,
                outcome: outcome.map(line).map_err(line),
            })
        }
        ServerMsg::Reply(Reply::GitIdentities { tag, report }) => ServerMsg::Reply(Reply::GitIdentities {
            tag,
            report: sanitize_identities(report),
        }),
        ServerMsg::Reply(Reply::GitHubAccounts { tag, report }) => ServerMsg::Reply(Reply::GitHubAccounts {
            tag,
            report: sanitize_github(report),
        }),
        ServerMsg::Reply(Reply::GitHubRepos { tag, answer }) => ServerMsg::Reply(Reply::GitHubRepos {
            tag,
            answer: sanitize_answer(answer, sanitize_repos),
        }),
        ServerMsg::Reply(Reply::GitHubOwners { tag, answer }) => ServerMsg::Reply(Reply::GitHubOwners {
            tag,
            answer: sanitize_answer(answer, |mut owners: Vec<String>| {
                owners.truncate(maitty_shared::protocol::GITHUB_OWNERS_MAX);
                owners.retain(|owner| is_login(owner));
                owners
            }),
        }),
        other => other,
    })
}

/// The answer to "who commits on that machine" made safe, or why it is refused: only that report
/// passes, cleaned as `vet` cleans it.
pub fn vet_identities_reply(reply: Reply) -> Result<Reply, String> {
    match reply {
        Reply::GitIdentities { tag, report } => Ok(Reply::GitIdentities { tag, report: sanitize_identities(report) }),
        _ => Err("it is not an answer about who commits there".into()),
    }
}

/// What another machine says about who commits there: no more rules than a report carries, every
/// name, email and folder one clean line of a report's length, and a rule's identity kept only
/// with a login GitHub could have given out.
fn sanitize_identities(mut report: maitty_shared::protocol::GitIdentityReport) -> maitty_shared::protocol::GitIdentityReport {
    use maitty_shared::protocol::{GIT_IDENTITY_RULES_MAX, GIT_IDENTITY_TEXT_MAX_CHARS};
    let line = |text: String| sanitize(&text, false).chars().take(GIT_IDENTITY_TEXT_MAX_CHARS).collect::<String>();
    report.name = report.name.map(line);
    report.email = report.email.map(line);
    report.problem = report.problem.map(line);
    report.rules.truncate(GIT_IDENTITY_RULES_MAX);
    for rule in &mut report.rules {
        rule.folder = line(std::mem::take(&mut rule.folder));
        rule.identity = rule.identity.take().filter(|identity| is_login(&identity.login)).map(|mut identity| {
            identity.name = line(std::mem::take(&mut identity.name));
            identity.email = line(std::mem::take(&mut identity.email));
            identity
        });
    }
    report
}

/// A GitHub report from another machine: no more accounts than a report carries, every login one
/// that GitHub could have given out, and the problem one clean line.
fn sanitize_github(mut report: maitty_shared::protocol::GitHubReport) -> maitty_shared::protocol::GitHubReport {
    report.accounts.truncate(maitty_shared::protocol::GITHUB_ACCOUNTS_MAX);
    report.accounts.retain(|account| is_login(&account.login));
    report.problem = report.problem.map(|why| sanitize(&cut_bytes(why, NAME_MAX), false));
    report
}

/// A GitHub answer from another machine: its list cleaned by `clean`, or its reason one clean line.
fn sanitize_answer<T>(answer: GitHubAnswer<T>, clean: impl FnOnce(T) -> T) -> GitHubAnswer<T> {
    match answer {
        GitHubAnswer::Listed(list) => GitHubAnswer::Listed(clean(list)),
        GitHubAnswer::Failed(why) => GitHubAnswer::Failed(sanitize(&cut_bytes(why, NAME_MAX), false)),
        other => other,
    }
}

/// Repositories from another machine: no more than an answer carries, each named as GitHub names
/// one, its clone address built here from that name and never taken from the machine, and its
/// description and time what those can be.
fn sanitize_repos(mut list: GitHubRepoList) -> GitHubRepoList {
    list.repos.truncate(maitty_shared::protocol::GITHUB_REPOS_MAX);
    list.repos.retain_mut(|repo| {
        let Some(address) = clone_address(&repo.name) else {
            return false;
        };
        repo.clone_url = address;
        repo.description = sanitize(&repo.description, false)
            .chars()
            .take(maitty_shared::protocol::GITHUB_DESCRIPTION_MAX_CHARS)
            .collect();
        if !is_time(&repo.pushed_at) {
            repo.pushed_at.clear();
        }
        true
    });
    list
}

/// A folder listing from another machine: every name and branch one clean line, no more entries
/// than the protocol allows, and no name that is a path.
fn sanitize_listing(mut listing: maitty_shared::protocol::FolderListing) -> maitty_shared::protocol::FolderListing {
    listing.path = sanitize(&cut_bytes(listing.path, PATH_MAX), false);
    listing.entries.truncate(maitty_shared::protocol::FOLDERS_MAX);
    listing.entries.retain_mut(|entry| {
        entry.name = sanitize(&cut_bytes(std::mem::take(&mut entry.name), NAME_MAX), false);
        entry.git = entry
            .git
            .take()
            .map(|branch| sanitize(&cut_bytes(branch, NAME_MAX), false));
        !entry.name.is_empty() && !entry.name.contains(['/', '\\'])
    });
    listing
}

/// A remote's words for a toast: one clean line, cut short, led by the machine so the source
/// is never in doubt.
pub fn tagged(machine: &str, text: &str) -> String {
    let head: String = text.chars().take(NOTICE_MAX).collect();
    format!("[{}] {}", sanitize(machine, false), sanitize(&head, false))
}

/// The longest folder path or name a listing from another machine may carry.
const PATH_MAX: usize = 4096;
const NAME_MAX: usize = 255;

fn cut_bytes(mut text: String, max: usize) -> String {
    if text.len() > max {
        let mut end = max;
        while !text.is_char_boundary(end) {
            end -= 1;
        }
        text.truncate(end);
    }
    text
}

#[cfg(test)]
mod tests {
    use super::*;
    use maitty_shared::protocol::{Reply, Snapshot, Status};
    use std::path::PathBuf;

    #[test]
    fn a_folder_listing_from_another_machine_is_cleaned_and_cut() {
        use maitty_shared::protocol::{FOLDERS_MAX, FolderEntry, FolderListing};
        let entry = |name: &str, git: Option<&str>| FolderEntry {
            name: name.into(),
            git: git.map(str::to_string),
            has_children: false,
        };
        let mut entries = vec![
            entry("fine", Some("main")),
            entry("bell\x07and\x1b[31mred", Some("ref\nname")),
            entry("a/b", None),
            entry("", None),
        ];
        entries.extend((0..FOLDERS_MAX + 10).map(|n| entry(&format!("d{n}"), None)));
        let listing = FolderListing {
            path: "/home/me\x1b]0;title\x07".into(),
            exists: true,
            entries,
            truncated: false,
        };

        let Some(ServerMsg::Reply(Reply::Folders { tag, listing })) = vet(
            &nothing_on(),
            true,
            "box",
            ServerMsg::Reply(Reply::Folders { tag: 3, listing }),
        ) else {
            panic!("a listing that was asked for is passed on");
        };

        assert_eq!(tag, 3);
        assert!(listing.entries.len() <= FOLDERS_MAX);
        assert!(!listing.path.contains(['\x1b', '\x07']));
        for entry in &listing.entries {
            assert!(!entry.name.is_empty());
            assert!(!entry.name.contains(['/', '\\', '\x1b', '\x07', '\n']), "{:?}", entry.name);
            assert!(entry.git.as_deref().is_none_or(|branch| !branch.contains('\n')));
        }
        assert_eq!(listing.entries[0].name, "fine");
        assert!(listing.entries.iter().all(|entry| entry.name != "a/b"));
    }

    #[test]
    fn the_question_another_machines_agent_asks_is_cleaned_before_it_is_shown() {
        use maitty_shared::protocol::TermInfo;
        let pane: TermInfo = serde_json::from_value(serde_json::json!({
            "id": 3, "cols": 80, "rows": 24, "title": "", "process": "", "agent": "claude",
            "status": "Blocked", "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
            "ask": {
                "tool": "Ba\u{1b}sh",
                "detail": "rm -rf /\u{1b}]0;title\u{7}\nnext line\u{202e}",
                "answerable": true,
            },
        }))
        .expect("a pane");
        let Some(ServerMsg::TermsChanged { changed, .. }) = vet(
            &nothing_on(),
            false,
            "box",
            ServerMsg::TermsChanged { changed: vec![pane], removed: Vec::new() },
        ) else {
            panic!("a pane delta is applied");
        };
        let ask = changed[0].ask.as_ref().expect("the question is kept");
        assert_eq!(ask.tool, "Bash");
        assert!(!ask.detail.contains(['\u{1b}', '\u{7}', '\n', '\u{202e}']), "{:?}", ask.detail);
        assert!(ask.answerable, "the key is still that machine's daemon's to check");
    }

    #[test]
    fn what_another_machine_says_of_a_clone_is_one_clean_line() {
        for outcome in [Ok("Pushed ma\u{1b}[31min\nx".to_string()), Err("fatal: no\u{7}\nway".to_string())] {
            let Some(ServerMsg::Reply(Reply::GitDone { tag: 6, outcome })) =
                vet(&nothing_on(), true, "box", ServerMsg::Reply(Reply::GitDone { tag: 6, outcome }))
            else {
                panic!("how a fetch, pull or push ended is let through");
            };
            let line = outcome.unwrap_or_else(|why| why);
            assert!(!line.contains(['\u{1b}', '\u{7}', '\n']), "{line:?}");
        }
        {
            use maitty_shared::protocol::{GIT_IDENTITY_RULES_MAX, GitIdentity, GitIdentityReport, GitIdentityRule};
            let rule = |login: &str| GitIdentityRule {
                folder: "/code/a\u{1b}[31m\nb".into(),
                identity: Some(GitIdentity { name: "A\u{7}".into(), email: "a@example.invalid".into(), login: login.into() }),
            };
            let mut rules = vec![rule("login-a"), rule("<b>not a login</b>")];
            rules.extend(std::iter::repeat_with(|| rule("login-a")).take(GIT_IDENTITY_RULES_MAX));
            let report = GitIdentityReport { installed: true, name: Some("Casey\nExample".into()), email: None, rules, problem: None };
            let Some(ServerMsg::Reply(Reply::GitIdentities { tag: 8, report })) =
                vet(&nothing_on(), true, "box", ServerMsg::Reply(Reply::GitIdentities { tag: 8, report }))
            else {
                panic!("who commits on a machine is let through");
            };
            assert_eq!(report.name.as_deref(), Some("Casey Example"));
            assert_eq!(report.rules.len(), GIT_IDENTITY_RULES_MAX);
            assert_eq!(report.rules[0].folder, "/code/a[31m b");
            assert_eq!(report.rules[0].identity.as_ref().map(|identity| identity.name.as_str()), Some("A"));
            assert_eq!(report.rules[1].identity, None, "a login that is not one takes the identity with it");
            let asked = Reply::GitIdentities { tag: 2, report: GitIdentityReport { name: Some("A\u{1b}[0m".into()), ..GitIdentityReport::default() } };
            assert!(matches!(vet_identities_reply(asked), Ok(Reply::GitIdentities { report, .. }) if report.name.as_deref() == Some("A[0m")));
            assert!(vet_identities_reply(Reply::Ok).is_err());
            assert!(vet_identities_reply(Reply::Text("the global config".into())).is_err());
        }
        let cloned = |outcome| ServerMsg::Reply(Reply::Cloned { tag: 4, outcome });
        let Some(ServerMsg::Reply(Reply::Cloned { tag: 4, outcome: Ok(path) })) =
            vet(&nothing_on(), true, "box", cloned(Ok("/home/dev/wid\u{1b}[31mgets\nx".into())))
        else {
            panic!("an answer that was asked for is kept");
        };
        assert!(!path.contains(['\u{1b}', '\n']), "{path:?}");
        let Some(ServerMsg::Reply(Reply::Cloned { outcome: Err(why), .. })) =
            vet(&nothing_on(), true, "box", cloned(Err("fatal: no\u{7}\nsecond line".repeat(100))))
        else {
            panic!("a refusal that was asked for is kept");
        };
        assert!(!why.contains(['\u{7}', '\n']) && why.len() <= NAME_MAX, "{why:?}");
        assert!(vet(&nothing_on(), false, "box", cloned(Ok("/x".into()))).is_none());
    }

    #[test]
    fn a_github_report_from_another_machine_keeps_only_real_logins_and_one_clean_line() {
        use maitty_shared::protocol::{GitHubAccount, GitHubReport};
        let account = |login: &str| GitHubAccount { login: login.into(), active: false };
        let report = GitHubReport {
            installed: true,
            accounts: vec![account("me"), account("<img src=x>"), account(""), account(&"a".repeat(40))],
            problem: Some("gh said\u{1b}[31m no\nsecond".into()),
        };
        let Some(ServerMsg::Reply(Reply::GitHubAccounts { tag: 6, report })) =
            vet(&nothing_on(), true, "box", ServerMsg::Reply(Reply::GitHubAccounts { tag: 6, report }))
        else {
            panic!("an answer that was asked for is kept");
        };
        assert_eq!(report.accounts, vec![account("me")]);
        assert!(report.problem.is_some_and(|why| !why.contains(['\u{1b}', '\n'])));
    }

    #[test]
    fn repositories_from_another_machine_are_named_cleaned_and_given_an_address_built_here() {
        use maitty_shared::protocol::{GITHUB_DESCRIPTION_MAX_CHARS, GitHubAnswer, GitHubRepo, GitHubRepoList};
        let repo = |name: &str| GitHubRepo {
            name: name.into(),
            description: format!("a\u{1b}[31mb\n{}", "d".repeat(400)),
            private: true,
            pushed_at: "<img>".into(),
            clone_url: "ext::sh -c evil".into(),
        };
        let answer = GitHubAnswer::Listed(GitHubRepoList { repos: vec![repo("me/widgets"), repo("me/--upload-pack=x"), repo("../../etc")], more: true });
        let Some(ServerMsg::Reply(Reply::GitHubRepos { tag: 4, answer: GitHubAnswer::Listed(list) })) =
            vet(&nothing_on(), true, "box", ServerMsg::Reply(Reply::GitHubRepos { tag: 4, answer }))
        else {
            panic!("an answer that was asked for is kept");
        };
        assert!(list.more);
        let [only] = &list.repos[..] else { panic!("{:?}", list.repos) };
        assert_eq!((only.name.as_str(), only.clone_url.as_str(), only.pushed_at.as_str()), ("me/widgets", "https://github.com/me/widgets.git", ""));
        assert_eq!(only.description.chars().count(), GITHUB_DESCRIPTION_MAX_CHARS);
        assert!(!only.description.contains(['\u{1b}', '\n']));

        let failed = ServerMsg::Reply(Reply::GitHubRepos { tag: 5, answer: GitHubAnswer::Failed("no\u{1b}[0m\nway".into()) });
        let Some(ServerMsg::Reply(Reply::GitHubRepos { answer: GitHubAnswer::Failed(why), .. })) = vet(&nothing_on(), true, "box", failed) else {
            panic!("a reason is kept");
        };
        assert!(!why.contains(['\u{1b}', '\n']));
    }

    #[test]
    fn owners_from_another_machine_are_only_logins() {
        use maitty_shared::protocol::GitHubAnswer;
        let answer = GitHubAnswer::Listed(vec!["me".to_owned(), "<b>".to_owned(), "my-org".to_owned(), String::new()]);
        let Some(ServerMsg::Reply(Reply::GitHubOwners { answer: GitHubAnswer::Listed(owners), .. })) =
            vet(&nothing_on(), true, "box", ServerMsg::Reply(Reply::GitHubOwners { tag: 3, answer }))
        else {
            panic!("an answer that was asked for is kept");
        };
        assert_eq!(owners, ["me", "my-org"]);
    }

    #[test]
    fn a_folder_listing_nobody_asked_for_is_dropped() {
        use maitty_shared::protocol::FolderListing;
        let reply = ServerMsg::Reply(Reply::Folders {
            tag: 1,
            listing: FolderListing::missing("/x"),
        });
        assert!(vet(&nothing_on(), false, "box", reply).is_none());
    }

    fn nothing_on() -> MachineOpts {
        MachineOpts::default()
    }

    fn every_message() -> Vec<ServerMsg> {
        vec![
            ServerMsg::Welcome {
                version: 1,
                compatible_since: 1,
            },
            ServerMsg::State(Snapshot::default()),
            ServerMsg::TermsChanged {
                changed: Vec::new(),
                removed: Vec::new(),
            },
            ServerMsg::Replay {
                term: 1,
                cols: 80,
                rows: 24,
                data: Vec::new(),
            },
            ServerMsg::Output {
                term: 1,
                data: Vec::new(),
            },
            ServerMsg::Attention {
                term: 1,
                status: Status::Blocked,
            },
            ServerMsg::Reply(Reply::Ok),
            ServerMsg::Reply(Reply::Text("t".into())),
            ServerMsg::Reply(Reply::Worktrees(Vec::new())),
            ServerMsg::Error("e".into()),
            ServerMsg::Notice("n".into()),
            ServerMsg::AskRemoveWorktree {
                path: PathBuf::from("/home/me/repo-wt"),
                name: "wt".into(),
                why: "unmerged".into(),
            },
            ServerMsg::Clipboard {
                term: 1,
                text: "x".into(),
            },
            ServerMsg::AutoWorkspace("api".into()),
            ServerMsg::SettingsChanged(maitty_shared::settings_view::SharedSettings {
                default_agent: "claude".into(),
                new_session_view: Default::default(),
                sidebar_side: Default::default(),
                theme_dark: "night".into(),
                theme_light: "day".into(),
                appearance: Default::default(),
                spinner: Vec::new(),
                start_dir: String::new(),
                reduce_motion: false,
                window_style: Default::default(),
            }),
            ServerMsg::Raise,
            ServerMsg::ConversationChanged {
                id: maitty_shared::conversation::ConversationId(1),
                generation: 1,
            },
            ServerMsg::Bye,
        ]
    }

    fn label(msg: &ServerMsg) -> &'static str {
        match msg {
            ServerMsg::Welcome { .. } => "Welcome",
            ServerMsg::State(_) => "State",
            ServerMsg::TermsChanged { .. } => "TermsChanged",
            ServerMsg::Replay { .. } => "Replay",
            ServerMsg::Output { .. } => "Output",
            ServerMsg::Attention { .. } => "Attention",
            ServerMsg::Reply(_) => "Reply",
            ServerMsg::Error(_) => "Error",
            ServerMsg::Notice(_) => "Notice",
            ServerMsg::AskRemoveWorktree { .. } => "AskRemoveWorktree",
            ServerMsg::Clipboard { .. } => "Clipboard",
            ServerMsg::AutoWorkspace(_) => "AutoWorkspace",
            ServerMsg::SettingsChanged(_) => "SettingsChanged",
            ServerMsg::Raise => "Raise",
            ServerMsg::ConversationChanged { .. } => "ConversationChanged",
            ServerMsg::Bye => "Bye",
        }
    }

    #[test]
    fn what_a_machine_with_nothing_on_may_send() {
        let mut applied = Vec::new();
        let mut answers = Vec::new();
        let mut ignored = Vec::new();
        for msg in every_message() {
            match allow(&nothing_on(), &msg) {
                Verdict::Apply => applied.push(label(&msg)),
                Verdict::Answer => answers.push(label(&msg)),
                Verdict::Ignore(_) => ignored.push(label(&msg)),
            }
        }
        applied.dedup();
        answers.dedup();
        ignored.dedup();
        assert_eq!(
            applied,
            [
                "Welcome",
                "State",
                "TermsChanged",
                "Replay",
                "Output",
                "Attention",
                "Bye"
            ]
        );
        assert_eq!(
            answers,
            ["Reply", "Error"],
            "a reply or error counts only as the answer to a request this window sent"
        );
        assert_eq!(
            ignored,
            [
                "Notice",
                "AskRemoveWorktree",
                "Clipboard",
                "AutoWorkspace",
                "SettingsChanged",
                "Raise",
                "ConversationChanged"
            ]
        );
    }

    #[test]
    fn opt_ins_open_only_their_own_message() {
        let allowed = |opts: MachineOpts| {
            let mut names: Vec<_> = every_message()
                .iter()
                .filter(|m| allow(&opts, m) == Verdict::Apply)
                .map(label)
                .collect();
            names.dedup();
            names
        };
        let base = allowed(nothing_on());
        let with = |names: &[&'static str]| {
            let mut all: Vec<&'static str> =
                base.iter().copied().chain(names.iter().copied()).collect();
            all.sort();
            all
        };
        let sorted = |mut v: Vec<&'static str>| {
            v.sort();
            v
        };
        let clipboard = MachineOpts {
            clipboard: true,
            ..nothing_on()
        };
        let notices = MachineOpts {
            notices: true,
            ..nothing_on()
        };
        let sounds = MachineOpts {
            sounds: true,
            ..nothing_on()
        };
        assert_eq!(sorted(allowed(clipboard)), with(&["Clipboard"]));
        assert_eq!(sorted(allowed(notices)), with(&["Notice", "Raise"]));
        assert_eq!(
            sorted(allowed(sounds)),
            sorted(base.clone()),
            "a sound is gated where it rings, not by a message"
        );
        for always_ignored in ["AskRemoveWorktree", "AutoWorkspace"] {
            assert!(
                !allowed(MachineOpts::LOCAL).contains(&always_ignored),
                "{always_ignored} is ignored even with everything on"
            );
        }
    }

    #[test]
    fn opt_ins_come_from_the_machines_entry_and_default_to_off() {
        let defs = vec![MachineDef {
            name: "build".into(),
            clipboard: true,
            ..MachineDef::default()
        }];
        let build = MachineOpts::named(&defs, "Build");
        assert_eq!(
            build,
            MachineOpts {
                clipboard: true,
                ..nothing_on()
            }
        );
        assert_eq!(MachineOpts::named(&defs, "stranger"), nothing_on());
        assert_eq!(MachineOpts::named(&defs, ""), nothing_on());
    }

    #[test]
    fn a_toast_is_one_clean_line_led_by_its_machine() {
        let hostile = format!("\u{1b}[2Jfake\nline{}", "x".repeat(1000));
        let toast = tagged("build", &hostile);
        assert!(toast.starts_with("[build] [2Jfake line"));
        assert!(toast.chars().count() <= "[build] ".len() + NOTICE_MAX);
        assert!(!toast.chars().any(char::is_control));
    }
}
