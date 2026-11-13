//! Reopening a closed session (`Command::ReopenClosedSession`): its conversation resumed in a new
//! session, in the folder it worked in. A linked worktree whose folder is gone is made again first,
//! only at the recorded path, from the recorded branch, in the recorded repository. Whatever is
//! already at that path is left alone.

use super::worktrees::trust_like_repo;
use super::*;
use std::path::Path;

/// The longest conversation id typed into a resume command.
const SESSION_ID_MAX_CHARS: usize = 128;

/// Where a closed session starts again.
#[derive(Debug)]
pub(crate) struct Ready {
    pub folder: PathBuf,
    /// The worktree made again for it, when its folder was gone.
    pub remade: Option<PathBuf>,
}

/// Whether `id` can be typed into a shell as a conversation id: a letter or digit, then letters,
/// digits, `_` and `-`, at most `SESSION_ID_MAX_CHARS` in all.
fn plain_session_id(id: &str) -> bool {
    let mut chars = id.chars();
    id.len() <= SESSION_ID_MAX_CHARS
        && chars.next().is_some_and(|c| c.is_ascii_alphanumeric())
        && chars.all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-')
}

/// The folder to start `closed` in, its worktree made again when that is what is gone. Blocks: runs git.
pub(super) fn ready_folder(closed: &ClosedSession) -> Result<Ready, String> {
    if closed.cwd.is_dir() {
        return Ok(Ready { folder: closed.cwd.clone(), remade: None });
    }
    let (Some(top), Some(repo), Some(branch)) = (&closed.worktree, &closed.repo, &closed.branch) else {
        return Err(format!(
            "{} is gone, and it was not a worktree that can be made again",
            closed.cwd.display()
        ));
    };
    recreate_worktree(repo, top, branch)?;
    // The subfolder it worked in may not be on the branch any more.
    let folder = Some(closed.cwd.clone())
        .filter(|cwd| cwd.starts_with(top) && cwd.is_dir())
        .unwrap_or_else(|| top.clone());
    Ok(Ready { folder, remade: Some(top.clone()) })
}

/// Make the worktree of `repo` at `top` on the existing `branch` again. Already there as that
/// worktree is fine; anything else at `top` is refused and left as it is.
fn recreate_worktree(repo: &Path, top: &Path, branch: &str) -> Result<(), String> {
    let main = crate::gitfs::head(repo).filter(|h| !h.linked && same_path(&h.top, repo));
    if main.is_none() {
        return Err(format!("{} is no longer a repository's main checkout; nothing was made", repo.display()));
    }
    if !top.is_absolute() || same_path(top, repo) {
        return Err(format!("{} is not a worktree folder; nothing was made", top.display()));
    }
    if top.symlink_metadata().is_ok() {
        return if is_that_worktree(repo, top, branch) { Ok(()) } else { Err(left_alone(top)) };
    }
    let has_branch = !branch.starts_with('-')
        && git::git(repo, &["show-ref", "--verify", "--quiet", &format!("refs/heads/{branch}")]).is_ok();
    if !has_branch {
        return Err(format!(
            "the branch {branch} no longer exists in {}, so its worktree was not made again",
            repo.display()
        ));
    }
    clear_stale_registrations(repo, top, branch)?;
    add_worktree_in_new_folder(repo, top, branch)?;
    trust_like_repo(repo, top);
    Ok(())
}

fn left_alone(top: &Path) -> String {
    format!("{} is already there and is not that worktree; it was left alone", top.display())
}

fn is_that_worktree(repo: &Path, top: &Path, branch: &str) -> bool {
    crate::gitfs::head(top).is_some_and(|h| {
        h.linked && same_path(&h.top, top) && same_path(&h.main_root, repo) && h.branch == branch
    })
}

/// A folder deleted by hand stays registered with git, holding its branch and path. Only the
/// registration in the way is removed: one whose folder is missing for another reason (an unplugged
/// drive) keeps its own. A branch checked out in a worktree that is still there is refused.
fn clear_stale_registrations(repo: &Path, top: &Path, branch: &str) -> Result<(), String> {
    let registered = git::list_worktrees(repo).map_err(|e| format!("{e:#}"))?;
    let in_the_way = registered.iter().filter(|e| !e.main && (e.branch == branch || same_path(&e.path, top)));
    for entry in in_the_way {
        if entry.path.symlink_metadata().is_ok() {
            if entry.branch == branch {
                return Err(format!(
                    "the branch {branch} is checked out in {}; nothing was made",
                    entry.path.display()
                ));
            }
            continue;
        }
        let path = entry.path.to_string_lossy().into_owned();
        // No --force: should the folder come back meanwhile (a drive plugged in), git refuses a tree
        // with changes or untracked files. A clean one coming back in that moment can still go, with
        // its ignored files.
        git::git(repo, &["worktree", "remove", &path]).map_err(|e| format!("{e:#}"))?;
    }
    Ok(())
}

/// The folder is made here first, so one that appeared since it was looked at is never written into:
/// making it fails instead. git fills the empty folder.
fn add_worktree_in_new_folder(repo: &Path, top: &Path, branch: &str) -> Result<(), String> {
    let parents = make_parents(top).map_err(|e| format!("couldn't make the folders above {}: {e}", top.display()))?;
    let made = match std::fs::create_dir(top) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Err(left_alone(top)),
        Err(e) => Err(format!("couldn't make {}: {e}", top.display())),
    };
    let path = top.to_string_lossy().into_owned();
    let added = made.and_then(|()| {
        git::git(repo, &["worktree", "add", &path, branch]).map(drop).map_err(|e| {
            remove_made(&[top.to_path_buf()]);
            format!("{e:#}")
        })
    });
    if added.is_err() {
        remove_made(&parents);
    }
    added
}

/// Make the missing folders above `top`; returns them deepest first.
fn make_parents(top: &Path) -> std::io::Result<Vec<PathBuf>> {
    let mut missing: Vec<PathBuf> = top
        .ancestors()
        .skip(1)
        .take_while(|dir| dir.symlink_metadata().is_err())
        .map(Path::to_path_buf)
        .collect();
    if let Some(parent) = top.parent() {
        std::fs::create_dir_all(parent)?;
    }
    missing.retain(|dir| dir.is_dir());
    Ok(missing)
}

/// Remove folders this module made, deepest first. `remove_dir` refuses one with anything in it.
fn remove_made(dirs: &[PathBuf]) {
    for dir in dirs {
        if let Err(e) = std::fs::remove_dir(dir) {
            tracing::warn!("couldn't remove the empty folder {}: {e}", dir.display());
        }
    }
}

impl Daemon {
    /// Reopen the closed session kept for `agent` and `session`: ready its folder off the loop, then
    /// start it (`closed_session_ready`).
    pub(super) fn reopen_closed_session(&mut self, client: ClientId, agent: &str, session: &str) -> Result<()> {
        let closed = self
            .closed_sessions
            .list()
            .iter()
            .find(|c| c.agent == agent && c.session == session)
            .cloned()
            .ok_or_else(|| anyhow::anyhow!("that session is not in the closed sessions list"))?;
        let cmd = self.resume_closed(&closed)?;
        let tx = self.tx.clone();
        self.pending_ops += 1;
        tokio::task::spawn_blocking(move || {
            let result = ready_folder(&closed);
            let _ = tx.blocking_send(Ev::ClosedSessionReady { client, cmd, result });
        });
        Ok(())
    }

    /// The agent's own resume command for a closed session's conversation.
    fn resume_closed(&self, closed: &ClosedSession) -> Result<String> {
        anyhow::ensure!(
            plain_session_id(&closed.session),
            "that session's conversation id can't be resumed"
        );
        let template = self
            .agents
            .iter()
            .find(|a| a.name == closed.agent)
            .and_then(|a| a.resume.as_deref())
            .ok_or_else(|| anyhow::anyhow!("{} can't resume a conversation by its id", closed.agent))?;
        Ok(template.replace("{session}", &closed.session))
    }

    /// A closed session's folder is ready (or not): start its agent there in a new session.
    pub(super) fn closed_session_ready(&mut self, client: ClientId, cmd: String, result: Result<Ready, String>) {
        self.pending_ops -= 1;
        let ready = match result {
            Ok(ready) => ready,
            Err(e) => {
                self.send(client, ServerMsg::Error(e));
                return;
            }
        };
        if let Some(top) = ready.remade {
            self.worktree_hook(&top, true);
            self.made_worktrees.push(top);
        }
        let start = Command::NewWorkspace { cwd: Some(ready.folder), name: None, cmd: Some(cmd) };
        match self.command(client, start) {
            Ok(_) => self.send(client, ServerMsg::Reply(Reply::Ok)),
            Err(e) => self.send(client, ServerMsg::Error(format!("{e:#}"))),
        }
        self.dirty = true;
    }
}

#[cfg(test)]
#[path = "reopen_tests.rs"]
mod tests;
