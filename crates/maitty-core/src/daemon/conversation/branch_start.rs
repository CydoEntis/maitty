//! A chat session on a branch (ADR 0003): the checkout is made with the usual machinery
//! (`worktrees.rs`: naming, trust copy, the repo's setup hook), then the conversation starts in it.

use crate::conversation::{LaunchSettings, StartTarget};
use crate::daemon::{
    ClientId, Daemon, Ev, Grant, Reply, ServerMsg, branch_agent, clean_path,
    create_trusted_worktree,
};
use anyhow::{Result, bail};
use std::path::PathBuf;

impl Daemon {
    /// Check a chat session on `branch` and make its checkout in the background. The reply is the
    /// conversation's own `ConversationStarted`, sent once it starts in the new folder.
    pub(in crate::daemon) fn conversation_start_on_branch(
        &mut self,
        client: ClientId,
        target: StartTarget,
        branch: String,
    ) -> Result<()> {
        self.may(client, Grant::Start, None)?;
        let StartTarget::Create(settings) = target else {
            bail!("only a new conversation can start on a branch");
        };
        if let Some(problem) = branch_agent::branch_problem(&branch) {
            bail!("{problem}");
        }
        // The same limit as a branch agent: a client cannot ask for a checkout (and its setup
        // hook) in a repository the app has never been shown.
        let project = Some(clean_path(settings.cwd.clone()))
            .filter(|dir| dir.is_absolute() && dir.is_dir() && self.is_known_project(dir))
            .ok_or_else(|| anyhow::anyhow!("that is not a project in the app"))?;
        if crate::gitfs::head(&project).is_none() {
            bail!("{} is not in a git repository", project.display());
        }
        let template = self.cfg.worktree.dir.clone();
        let from_default = self.cfg.worktree.base_on_default;
        let tx = self.tx.clone();
        self.pending_ops += 1;
        tokio::task::spawn_blocking(move || {
            let result = create_trusted_worktree(&project, &branch, None, &template, from_default);
            let _ = tx.blocking_send(Ev::ChatWorktreeCreated {
                client,
                settings,
                branch,
                set_up: false,
                result,
            });
        });
        Ok(())
    }

    /// A worktree asked for by folder and branch (`Command::BranchWorktree`) is made: the repo's
    /// setup runs in it first, so the agent the window starts there finds it ready, then the
    /// checkout is listed with the ones the app made and its folder is the reply.
    pub(in crate::daemon) fn branch_worktree_made(
        &mut self,
        client: ClientId,
        set_up: bool,
        result: Result<(PathBuf, String), String>,
    ) {
        if !set_up
            && let Ok((path, _)) = &result
            && let Some(job) = self.hook_job(path, true)
        {
            let tx = self.tx.clone();
            tokio::task::spawn_blocking(move || {
                let _ = tx.blocking_send(Ev::HookRan(job()));
                let _ = tx.blocking_send(Ev::BranchWorktreeMade {
                    client,
                    set_up: true,
                    result,
                });
            });
            return;
        }
        self.pending_ops -= 1;
        match result.map_err(anyhow::Error::msg).and_then(|(path, _)| {
            let folder = clean_path(path.canonicalize()?);
            self.made_worktrees.push(path);
            Ok(folder)
        }) {
            Ok(folder) => {
                self.poll_git_soon();
                self.send(
                    client,
                    ServerMsg::Reply(Reply::Text(folder.to_string_lossy().into_owned())),
                );
            }
            Err(error) => self.send(client, ServerMsg::Error(format!("{error:#}"))),
        }
        self.dirty = true;
    }

    pub(in crate::daemon) fn chat_worktree_created(
        &mut self,
        client: ClientId,
        settings: LaunchSettings,
        branch: String,
        set_up: bool,
        result: Result<(PathBuf, String), String>,
    ) {
        // The repo's own setup comes first (an install, a .env), so the agent starts with it ready.
        if !set_up
            && let Ok((path, _)) = &result
            && let Some(job) = self.hook_job(path, true)
        {
            let tx = self.tx.clone();
            tokio::task::spawn_blocking(move || {
                let _ = tx.blocking_send(Ev::HookRan(job()));
                let _ = tx.blocking_send(Ev::ChatWorktreeCreated {
                    client,
                    settings,
                    branch,
                    set_up: true,
                    result,
                });
            });
            return;
        }
        self.pending_ops -= 1;
        let started = result.map_err(anyhow::Error::msg).and_then(|(path, _)| {
            let folder = clean_path(path.canonicalize()?);
            // Listed with the checkouts the app made, so the usual tidy-up offers it.
            self.made_worktrees.push(path);
            self.conversation_start(
                client,
                StartTarget::Create(LaunchSettings {
                    cwd: folder,
                    ..settings
                }),
            )
        });
        match started {
            Ok(()) => self.poll_git_soon(),
            Err(error) => self.send(client, ServerMsg::Error(format!("{error:#}"))),
        }
        self.dirty = true;
    }
}
