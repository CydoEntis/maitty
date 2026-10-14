//! Workflow execution belongs to the daemon; preparation and handoff I/O run off-loop.

use super::*;
use crate::workflows::{self as wf, Run, Stage, State};
use anyhow::{Context, ensure};

const MAX_RUNS: usize = 32;

pub(crate) struct Prepared {
    pub dir: PathBuf,
    pub root: PathBuf,
    pub profiles: [crate::profiles::Profile; 3],
    pub artifacts: PathBuf,
    pub setup_error: Option<String>,
}

fn artifacts(id: u64) -> PathBuf {
    use std::hash::{Hash, Hasher};
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    ipc::socket_id().hash(&mut hash);
    crate::config::data_dir()
        .join("workflows")
        .join(format!("{:x}", hash.finish()))
        .join(id.to_string())
}

fn prepare(cfg: &Config, run: &Run, expected_root: Option<PathBuf>) -> Result<Prepared> {
    let source = run
        .source
        .canonicalize()
        .context("finding the workflow checkout")?;
    let head = crate::gitfs::head(&source).context("workflows need a git checkout")?;
    if let Some(expected) = expected_root {
        ensure!(
            same_path(&head.main_root, &expected),
            "an agent may only start workflows in its own project"
        );
    }
    let available = crate::profiles::load(cfg, &source)?;
    let workflows = wf::load(cfg, &source)?;
    let workflow = workflows
        .iter()
        .find(|w| w.workflow.name == run.name)
        .with_context(|| format!("no workflow named {}; run seshi workflow list", run.name))?;
    let profiles = workflow.workflow.profiles(&available)?;
    for profile in &profiles {
        profile.validate()?;
    }
    let hook = crate::project::load(&source).hooks.on_create;
    ensure!(
        hook.is_empty() || crate::project::allowed(&source, &hook),
        "this repo's on_create hook needs approval; run seshi allow in the repo before starting a workflow"
    );
    let branch = format!("workflow-{}", run.id);
    let base = crate::proc::git(&source, &["rev-parse", "HEAD"]).map_err(anyhow::Error::msg)?;
    let (dir, _) = create_trusted_worktree(&source, &branch, Some(base.trim()), &cfg.worktree.dir)
        .map_err(anyhow::Error::msg)?;
    let actual_hook = crate::project::load(&dir).hooks.on_create;
    let setup_error = if actual_hook.is_empty() {
        None
    } else if !crate::project::allowed(&dir, &actual_hook) {
        Some(
            "The new worktree's on_create hook needs approval. Set up the checkout, then retry."
                .into(),
        )
    } else {
        let vars = [
            ("SESHI_WORKTREE", dir.display().to_string()),
            ("SESHI_BRANCH", branch),
            ("SESHI_REPO", head.main_root.display().to_string()),
        ];
        crate::project::run_hook(&cfg.shell_command(), &dir, &actual_hook, &vars)
            .err()
            .map(|e| format!("on_create failed: {e}. Set up the checkout, then retry."))
    };
    Ok(Prepared {
        dir,
        root: head.main_root,
        profiles,
        artifacts: artifacts(run.id),
        setup_error,
    })
}

fn write_launch(cfg: &Config, run: &Run) -> Result<String> {
    std::fs::create_dir_all(&run.artifacts).context("creating workflow handoffs")?;
    for (name, text) in [
        ("goal.md", &run.goal),
        ("plan.md", &run.plan),
        ("worker.md", &run.work),
        ("review.md", &run.review),
    ] {
        crate::config::write_atomic(&run.artifacts.join(name), text.as_bytes())
            .context("writing workflow handoff")?;
    }
    let launch = wf::Launch {
        profile: run.profiles[run.stage.index()].clone(),
        task: run.task(),
    };
    launch.profile.args(&launch.task)?;
    let path = run
        .artifacts
        .join(format!("launch-{}-{}.json", run.stage.label(), run.round));
    crate::config::write_atomic(&path, &serde_json::to_vec(&launch)?)
        .context("writing workflow launch")?;
    let exe = std::env::current_exe().context("finding Seshi")?;
    let shell = cfg.shell_command();
    let powershell = shell.first().is_some_and(|s| {
        std::path::Path::new(s)
            .file_stem()
            .is_some_and(|s| s.eq_ignore_ascii_case("pwsh") || s.eq_ignore_ascii_case("powershell"))
    });
    Ok(format!(
        "{}{} workflow stage {}",
        if powershell { "& " } else { "" },
        cfg.quote_for_shell(&exe.display().to_string()),
        cfg.quote_for_shell(&path.display().to_string())
    ))
}

impl Daemon {
    pub(super) fn start_workflow(
        &mut self,
        client: ClientId,
        name: String,
        cwd: PathBuf,
        goal: String,
    ) -> Result<u64> {
        let last = self.workflows.iter().map(|r| r.id).max().unwrap_or(0);
        wf::Workflow {
            name: name.clone(),
            planner: "x".into(),
            worker: "x".into(),
            reviewer: "x".into(),
        }
        .validate()?;
        ensure!(
            !goal.trim().is_empty() && goal.len() <= wf::MAX_GOAL && !goal.contains('\0'),
            "goal must contain 1..{} bytes and no NUL",
            wf::MAX_GOAL
        );
        let from = self.clients.get(&client).and_then(|c| c.from);
        ensure!(
            !self
                .workflows
                .iter()
                .any(|r| r.term == from && from.is_some()),
            "you are an assigned workflow stage; finish that stage instead of starting another workflow"
        );
        let expected = from
            .map(|id| {
                self.terms
                    .get(&id)
                    .context("unverified workflow caller")
                    .map(|t| {
                        t.head
                            .as_ref()
                            .map(|h| h.main_root.clone())
                            .unwrap_or_else(|| t.cwd.clone())
                    })
            })
            .transpose()?;
        let parent = self.starting(client)?;
        if self.workflows.len() >= MAX_RUNS
            && let Some(old) = self.workflows.iter().position(|r| r.state.terminal())
        {
            self.workflows.remove(old);
        }
        ensure!(
            self.workflows.len() < MAX_RUNS,
            "{MAX_RUNS} workflows are still active; finish or cancel one first"
        );
        let id = (term::unix_now() * 1_000).max(last.saturating_add(1));
        let run = Run {
            id,
            name,
            goal,
            source: cwd,
            dir: PathBuf::new(),
            artifacts: PathBuf::new(),
            parent,
            term: None,
            finished: Vec::new(),
            profiles: std::array::from_fn(|_| Default::default()),
            stage: Stage::Plan,
            state: State::Preparing,
            round: 0,
            plan: String::new(),
            work: String::new(),
            review: String::new(),
            pending: None,
            note: "Preparing the task worktree.".into(),
            exhausted: false,
        };
        self.workflows.push(run.clone());
        self.dirty = true;
        self.pending_ops += 1;
        let tx = self.tx.clone();
        tokio::task::spawn_blocking(move || {
            let result = Config::load()
                .and_then(|cfg| prepare(&cfg, &run, expected))
                .map_err(|e| format!("{e:#}"));
            let _ = tx.blocking_send(Ev::WorkflowPrepared { run: id, result });
        });
        Ok(id)
    }

    pub(super) fn workflow_prepared(&mut self, id: u64, result: Result<Prepared, String>) {
        self.pending_ops = self.pending_ops.saturating_sub(1);
        let Some(run) = self.workflows.iter_mut().find(|r| r.id == id) else {
            return;
        };
        match result {
            Ok(prepared) => {
                // A cancel during setup still keeps the checkout so it can be inspected.
                run.dir = prepared.dir;
                run.source = prepared.root;
                run.profiles = prepared.profiles;
                run.artifacts = prepared.artifacts;
                if run.state == State::Preparing {
                    run.state = if prepared.setup_error.is_some() {
                        State::Failed
                    } else {
                        State::Ready
                    };
                    run.note = prepared
                        .setup_error
                        .unwrap_or_else(|| "Planner queued.".into());
                }
            }
            Err(error) => {
                if run.state != State::Cancelled {
                    run.state = State::Failed;
                    run.note = error;
                }
            }
        }
        self.dirty = true;
    }

    pub(super) fn finish_workflow(
        &mut self,
        client: ClientId,
        id: u64,
        report: wf::Report,
    ) -> Result<()> {
        let caller = self
            .clients
            .get(&client)
            .and_then(|c| c.from)
            .context("stage reports must come from their assigned pane")?;
        ensure!(self.terms.contains_key(&caller), "unverified stage caller");
        let run = self
            .workflows
            .iter_mut()
            .find(|r| r.id == id)
            .context("no such workflow run")?;
        ensure!(
            run.term == Some(caller),
            "only the active stage's pane may report for this workflow"
        );
        run.submit(report)?;
        self.dirty = true;
        Ok(())
    }

    pub(super) fn control_workflow(&mut self, id: u64, action: WorkflowAction) -> Result<()> {
        let run = self
            .workflows
            .iter_mut()
            .find(|r| r.id == id)
            .context("no such workflow run")?;
        match action {
            WorkflowAction::Approve => run.approve()?,
            WorkflowAction::Cancel => {
                ensure!(!run.state.terminal(), "this run has already ended");
                run.state = State::Cancelled;
                run.pending = None;
                run.note = "Cancelled; no more stages will start. Existing panes and files are kept; interrupt a running stage in its pane if needed.".into();
            }
            WorkflowAction::Retry => {
                ensure!(
                    matches!(run.state, State::Failed | State::Interrupted),
                    "only a failed or interrupted stage can be retried"
                );
                ensure!(
                    !run.dir.as_os_str().is_empty(),
                    "setup did not create a checkout; correct the error and start a new run"
                );
                ensure!(
                    run.term.is_none_or(|id| !self.terms.contains_key(&id)),
                    "close the previous stage's pane before retrying; files will be kept"
                );
                ensure!(
                    !run.exhausted,
                    "three review rounds are exhausted; inspect the findings and start a new task explicitly"
                );
                run.term = None;
                run.pending = None;
                run.state = State::Ready;
                run.note = "Stage retry queued; existing files are kept.".into();
            }
        }
        self.dirty = true;
        Ok(())
    }

    pub(super) fn update_workflows(&mut self) {
        let mut launch = Vec::new();
        let completed: Vec<_> = self
            .workflows
            .iter()
            .flat_map(|r| r.finished.iter().copied())
            .collect();
        let busy = self
            .terms
            .values()
            .filter(|t| {
                (t.started_by.is_some() || self.workflows.iter().any(|r| r.term == Some(t.id)))
                    && (t.status == Status::Working
                        || (!completed.contains(&t.id) && t.born.elapsed() < HELPER_SETTLES))
            })
            .count()
            + self
                .workflows
                .iter()
                .filter(|r| r.state == State::Preparing)
                .count();
        let mut slots = if self.cfg.mcp.helpers_at_once == 0 {
            usize::MAX
        } else {
            self.cfg.mcp.helpers_at_once.saturating_sub(busy)
        };
        for run in &mut self.workflows {
            if run.state == State::Running {
                match run.term.and_then(|id| self.terms.get(&id)) {
                    Some(t)
                        if run.pending.is_some()
                            && matches!(t.status, Status::Done | Status::Idle) =>
                    {
                        run.finish_turn();
                        self.dirty = true;
                    }
                    Some(t) if t.agent.is_none() && t.born.elapsed() >= AGENT_SHOWS_WITHIN => {
                        run.state = State::Failed;
                        run.note = "The agent exited without completing this stage. Inspect its pane, close it, then retry.".into();
                        self.dirty = true;
                    }
                    None => {
                        run.state = State::Failed;
                        run.note = "The stage pane closed before its report and finished turn were received. Files are kept.".into();
                        self.dirty = true;
                    }
                    Some(t) => {
                        let note = if t.status == Status::Blocked {
                            "This stage needs you. Open its pane to answer the question or permission prompt."
                        } else if matches!(t.status, Status::Done | Status::Idle) {
                            "The turn ended without a stage report. Open its pane and ask it to submit the report."
                        } else if run.pending.is_some() {
                            "Report received; waiting for the stage's turn to finish."
                        } else {
                            "The assigned profile is working; prompts stay in its pane."
                        };
                        if run.note != note {
                            run.note = note.into();
                            self.dirty = true;
                        }
                    }
                }
            }
            if run.state == State::Ready && slots > 0 {
                // Re-check the parent's start grant before every automatically started helper.
                let allowed = run
                    .parent
                    .and_then(|id| self.terms.get(&id))
                    .is_none_or(|t| {
                        t.grants
                            .as_ref()
                            .unwrap_or(&self.cfg.mcp.grants)
                            .iter()
                            .any(|g| g == "start")
                    });
                if !allowed {
                    run.state = State::Failed;
                    run.note =
                        "The coordinating pane no longer has permission to start helpers.".into();
                    self.dirty = true;
                    continue;
                }
                run.state = State::Preparing;
                run.note = "Preparing the stage handoff.".into();
                launch.push(run.clone());
                slots -= 1;
                self.dirty = true;
            }
        }
        for run in launch {
            let tx = self.tx.clone();
            let cfg = self.cfg.clone();
            self.pending_ops += 1;
            tokio::task::spawn_blocking(move || {
                let result = write_launch(&cfg, &run).map_err(|e| format!("{e:#}"));
                let _ = tx.blocking_send(Ev::WorkflowLaunch {
                    run: run.id,
                    result,
                });
            });
        }
    }

    pub(super) fn workflow_launched(&mut self, id: u64, result: Result<String, String>) {
        self.pending_ops = self.pending_ops.saturating_sub(1);
        let Some(index) = self
            .workflows
            .iter()
            .position(|r| r.id == id && r.state == State::Preparing)
        else {
            return;
        };
        let run = self.workflows[index].clone();
        let result = result.map_err(anyhow::Error::msg).and_then(|cmd| {
            let (cols, rows) = self.guess_size();
            let term = self.spawn(Some(&cmd), &run.dir, cols, rows)?;
            self.started(term, run.parent);
            if let Some(t) = self.terms.get_mut(&term) {
                t.agent = Some(run.profiles[run.stage.index()].agent.command().into());
                t.label = format!("{} · {} · round {}", run.name, run.stage.label(), run.round);
                t.status = Status::Working;
            }
            let ws = self.next();
            let tab = self.next();
            let color = self.free_color();
            self.workspaces.push(WorkspaceInfo {
                id: ws,
                name: String::new(),
                cwd: run.dir.clone(),
                tabs: vec![TabInfo {
                    id: tab,
                    name: String::new(),
                    layout: Node::Leaf(term),
                    focus: term,
                }],
                active_tab: tab,
                git: None,
                worktree: true,
                color,
                is_new: false,
                group: None,
            });
            // Running an agent's workflow must not steal its user's focused pane.
            if self.active_ws.is_none() {
                self.active_ws = Some(ws);
            }
            self.poll_git_soon();
            Ok(term)
        });
        let run = &mut self.workflows[index];
        match result {
            Ok(term) => {
                run.term = Some(term);
                run.state = State::Running;
                run.note = "The assigned profile is working; prompts stay in its pane.".into();
            }
            Err(error) => {
                run.state = State::Failed;
                run.note = format!("{error:#}");
            }
        }
        self.dirty = true;
    }
}
