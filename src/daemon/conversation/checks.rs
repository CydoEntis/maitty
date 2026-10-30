//! Checkpoints belong to the conversation even when no native pane is attached.

use crate::conversation::ConversationId;
use crate::daemon::{
    Daemon,
    check::{self, CheckOwner, CheckRun, RunPlan, RunRequest},
    check_source::Resolved,
};
use crate::protocol::Status;

impl Daemon {
    pub(in crate::daemon) fn conversation_turn_ended(
        &mut self,
        id: ConversationId,
        generation: u64,
        turn: String,
    ) {
        let Some(record) = self
            .conversations
            .iter()
            .find(|c| c.id == id && c.generation == generation)
        else {
            return;
        };
        let dir = record
            .binding
            .worktree
            .clone()
            .unwrap_or_else(|| record.binding.cwd.clone());
        let state = self.conversation_states.entry(id).or_default();
        if !state.completed_turns.insert(turn) {
            return;
        }
        let Some(turn) = state.checks.turn.checked_add(1) else {
            state.status = Status::Blocked;
            state.error = Some("Conversation checkpoint turn counter exhausted".into());
            self.dirty = true;
            return;
        };
        state.checks.turn = turn;
        if state.checks.running() {
            return;
        }
        if state.checks.run == u64::MAX {
            state.status = Status::Blocked;
            state.error = Some("Conversation checkpoint run counter exhausted".into());
            self.dirty = true;
            return;
        }
        // The repo and its definitions are resolved off the loop; until then done waits.
        let placeholder = crate::config::CheckDef {
            project: dir.clone(),
            name: Some(crate::config::REPO_CHECKS_FILE.into()),
            ..Default::default()
        };
        let plan = RunPlan {
            dir: dir.clone(),
            turn,
            after_turn_ended_at: crate::daemon::term::unix_now(),
        };
        state.checks.begin(&[placeholder], &plan, None);
        state.checks.dirty = true;
        let owner = CheckOwner::Conversation { id, generation };
        let request = RunRequest {
            owner,
            run: state.checks.run,
            user: self.cfg.check.clone(),
            resolver: self.resolver,
            shell: self.cfg.shell_command(),
            project: dir.clone(),
            dir,
            approvals: self.approvals.for_owner(owner),
            only: None,
        };
        self.dirty = true;
        check::spawn_run(request, self.tx.clone());
    }

    pub(in crate::daemon) fn conversation_checks_resolved(
        &mut self,
        id: ConversationId,
        generation: u64,
        run: u64,
        resolved: Vec<Resolved>,
        tree: Option<String>,
    ) {
        if !self
            .conversations
            .iter()
            .any(|c| c.id == id && c.generation == generation)
        {
            return;
        }
        let Some(state) = self
            .conversation_states
            .get_mut(&id)
            .filter(|s| s.checks.run == run)
        else {
            return;
        };
        state.checks.resolved(resolved, tree);
        state.checks.dirty = true;
        self.dirty = true;
    }

    pub(in crate::daemon) fn conversation_check_ran(
        &mut self,
        id: ConversationId,
        generation: u64,
        run: u64,
        name: &str,
        result: CheckRun,
    ) {
        if !self
            .conversations
            .iter()
            .any(|c| c.id == id && c.generation == generation)
        {
            return;
        }
        let Some(state) = self
            .conversation_states
            .get_mut(&id)
            .filter(|s| s.checks.run == run)
        else {
            return;
        };
        if state.checks.record(name, result) {
            state.checks.dirty = true;
            self.dirty = true;
        }
    }
}

#[derive(Debug, Clone)]
pub(in crate::daemon) struct StaleProbe {
    id: ConversationId,
    generation: u64,
    dir: std::path::PathBuf,
    trees: Vec<String>,
}

#[derive(Debug)]
pub(crate) struct StaleFound {
    pub(in crate::daemon) id: ConversationId,
    pub(in crate::daemon) generation: u64,
    tree: String,
    changed: Vec<crate::protocol::ChangedFile>,
}

pub(in crate::daemon) fn report_stale(
    probes: &[StaleProbe],
    tx: &tokio::sync::mpsc::Sender<crate::daemon::Ev>,
) {
    let found: Vec<_> = probes
        .iter()
        .flat_map(|p| {
            crate::daemon::check_stale::changes(&p.dir, &p.trees)
                .into_iter()
                .map(|(tree, changed)| StaleFound {
                    id: p.id,
                    generation: p.generation,
                    tree,
                    changed,
                })
        })
        .collect();
    if !found.is_empty() {
        let _ = tx.blocking_send(crate::daemon::Ev::ConversationChecksStale(found));
    }
}

impl Daemon {
    pub(in crate::daemon) fn conversation_stale_probes(&self) -> Vec<StaleProbe> {
        self.conversations
            .iter()
            .filter_map(|c| {
                let state = self.conversation_states.get(&c.id)?;
                if state.status == Status::Working || state.checks.running() {
                    return None;
                }
                let mut trees: Vec<_> = state
                    .checks
                    .points
                    .iter()
                    .filter(|p| p.latest.info.state == crate::protocol::CheckState::Passed)
                    .filter_map(|p| p.tree.clone())
                    .collect();
                trees.sort();
                trees.dedup();
                (!trees.is_empty()).then(|| StaleProbe {
                    id: c.id,
                    generation: c.generation,
                    dir: c
                        .binding
                        .worktree
                        .clone()
                        .unwrap_or_else(|| c.binding.cwd.clone()),
                    trees,
                })
            })
            .collect()
    }

    pub(in crate::daemon) fn conversation_checks_went_stale(&mut self, found: Vec<StaleFound>) {
        for f in found {
            if !self
                .conversations
                .iter()
                .any(|c| c.id == f.id && c.generation == f.generation)
            {
                continue;
            }
            let Some(state) = self
                .conversation_states
                .get_mut(&f.id)
                .filter(|s| s.status != Status::Working && !s.checks.running())
            else {
                continue;
            };
            for p in state.checks.points.iter_mut().filter(|p| {
                p.latest.info.state == crate::protocol::CheckState::Passed
                    && p.tree.as_deref() == Some(f.tree.as_str())
            }) {
                p.latest.info.state = crate::protocol::CheckState::Stale;
                p.latest.info.stale_since = Some(crate::daemon::term::unix_now());
                p.latest.info.changed_files = f.changed.clone();
                state.checks.dirty = true;
                self.dirty = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::daemon::{Ev, persist};
    use crate::protocol::{CheckSource, CheckState};
    use std::time::Duration;

    fn fixture() -> (
        Daemon,
        tokio::sync::mpsc::Receiver<Ev>,
        persist::ScratchDir,
        ConversationId,
    ) {
        let dir = persist::scratch_dir();
        std::fs::create_dir_all(dir.path()).unwrap();
        crate::proc::git(dir.path(), &["init", "-q"]).unwrap();
        std::fs::write(dir.path().join("source.txt"), "original").unwrap();
        let mut c = crate::test_support::conversation();
        c.binding.cwd = dir.path().to_path_buf();
        c.binding.worktree = None;
        let id = c.id;
        let mut cfg = crate::config::Config::default();
        cfg.check.push(crate::config::CheckDef {
            project: dir.path().to_path_buf(),
            name: Some("verify".into()),
            command: "echo checkpoint".into(),
            ..Default::default()
        });
        let (tx, rx) = tokio::sync::mpsc::channel(32);
        let mut d = Daemon::new(cfg, tx);
        d.conversations.push(c);
        d.conversation_states.entry(id).or_default().status = Status::Done;
        (d, rx, dir, id)
    }

    async fn settle(d: &mut Daemon, rx: &mut tokio::sync::mpsc::Receiver<Ev>, id: ConversationId) {
        tokio::time::timeout(Duration::from_secs(20), async {
            while d.conversation_states[&id].checks.running() {
                d.handle(rx.recv().await.expect("checkpoint worker result"));
            }
        })
        .await
        .expect("checkpoint settled");
    }

    #[tokio::test]
    async fn a_completed_conversation_runs_its_check_once_without_a_native_pane() {
        let (mut d, mut rx, _dir, id) = fixture();
        d.conversation_turn_ended(id, 7, "turn-a".into());
        d.conversation_turn_ended(id, 7, "turn-a".into());
        settle(&mut d, &mut rx, id).await;
        let checks = &d.conversation_states[&id].checks;
        assert!(d.terms.is_empty());
        assert_eq!(checks.turn, 1);
        assert_eq!(checks.infos().len(), 1);
        assert_eq!(checks.infos()[0].state, CheckState::Passed);
        assert_eq!(checks.infos()[0].turn, 1);
        assert!(
            checks
                .named("verify")
                .unwrap()
                .latest
                .output
                .contains("checkpoint")
        );
        assert_eq!(
            d.conversation_summaries()[0].checks[0].state,
            CheckState::Passed
        );
    }

    #[tokio::test]
    async fn a_repo_checkpoint_waits_for_approval_instead_of_trusting_provider_done() {
        let (mut d, mut rx, dir, id) = fixture();
        d.cfg.check.clear();
        std::fs::create_dir_all(dir.path().join(concat!(".", env!("APP_COMMAND")))).unwrap();
        std::fs::write(
            crate::config::repo_checks_path(dir.path()),
            "[[check]]\nname = \"verify\"\ncommand = \"echo must-not-run\"\n",
        )
        .unwrap();
        d.conversation_turn_ended(id, 7, "turn-a".into());
        settle(&mut d, &mut rx, id).await;
        let state = &d.conversation_states[&id];
        assert!(state.checks.holds_done());
        let check = &state.checks.infos()[0];
        assert_eq!(check.state, CheckState::NeedsApproval);
        assert_eq!(check.turn, 1);
        assert_eq!(check.cwd, dir.path());
        assert!(matches!(check.source, CheckSource::Repo { .. }));
        assert!(
            state
                .checks
                .named("verify")
                .unwrap()
                .latest
                .output
                .is_empty()
        );
    }

    #[tokio::test]
    async fn delayed_checkpoint_results_cannot_change_a_new_generation_or_run() {
        let (mut d, mut rx, _dir, id) = fixture();
        d.conversation_turn_ended(id, 7, "turn-a".into());
        settle(&mut d, &mut rx, id).await;
        let run = d.conversation_states[&id].checks.run;
        let result = || CheckRun {
            outcome: check::CheckOutcome::Exited(1),
            output: "wrong owner".into(),
            lines: 1,
            duration: Duration::ZERO,
        };
        let before = d.conversation_states[&id].checks.infos();
        d.conversation_check_ran(id, 6, run, "verify", result());
        d.conversation_checks_resolved(id, 6, run, Vec::new(), None);
        d.conversation_checks_resolved(id, 7, run + 1, Vec::new(), None);
        assert_eq!(d.conversation_states[&id].checks.infos(), before);
        d.conversation_turn_ended(id, 6, "stale-turn".into());
        assert_eq!(d.conversation_states[&id].checks.turn, 1);
        d.conversations[0].generation = 8;
        d.conversation_checks_resolved(id, 7, run, Vec::new(), None);
        assert_eq!(d.conversation_states[&id].checks.infos(), before);
    }

    #[tokio::test]
    async fn a_failed_checkpoint_survives_a_later_native_binding() {
        let (mut d, mut rx, _dir, id) = fixture();
        d.cfg.check[0].command = "exit 1".into();
        d.conversation_turn_ended(id, 7, "turn-a".into());
        settle(&mut d, &mut rx, id).await;
        d.conversations[0].execution = crate::conversation::ExecutionKind::Native;
        let summary = &d.conversation_summaries()[0];
        assert_eq!(summary.checks[0].state, CheckState::Failed);
        assert!(d.conversation_states[&id].checks.holds_done());
        assert!(d.terms.is_empty());
    }

    #[tokio::test]
    async fn changed_files_stale_a_conversation_pass_but_old_generation_probes_do_not() {
        let (mut d, mut rx, dir, id) = fixture();
        d.conversation_turn_ended(id, 7, "turn-a".into());
        settle(&mut d, &mut rx, id).await;
        let probes = d.conversation_stale_probes();
        assert_eq!(probes.len(), 1);
        std::fs::write(dir.path().join("source.txt"), "changed").unwrap();
        let p = &probes[0];
        let found = || {
            crate::daemon::check_stale::changes(&p.dir, &p.trees)
                .into_iter()
                .map(|(tree, changed)| StaleFound {
                    id,
                    generation: 7,
                    tree,
                    changed,
                })
                .collect()
        };
        d.conversations[0].generation = 8;
        d.conversation_checks_went_stale(found());
        assert_eq!(
            d.conversation_states[&id].checks.infos()[0].state,
            CheckState::Passed
        );
        d.conversations[0].generation = 7;
        d.conversation_checks_went_stale(found());
        let check = &d.conversation_states[&id].checks.infos()[0];
        assert_eq!(check.state, CheckState::Stale);
        assert!(check.changed_files.iter().any(|f| f.path == "source.txt"));
    }
}
