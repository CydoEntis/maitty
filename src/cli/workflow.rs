//! Workflow commands used by people, coordinating agents and assigned stages.

use crate::{config::Config, profiles::Runner, protocol::*, workflows as wf};
use anyhow::{Context, Result, bail, ensure};
use clap::Subcommand;
use std::io::Read;
use std::path::PathBuf;

#[derive(Subcommand)]
pub(crate) enum WorkflowCmd {
    /// List the effective saved workflows for this checkout.
    List,
    /// Print the selected profiles and instructions for a coordinating session.
    Show { name: String },
    /// Save a feature workflow and any missing starter profiles.
    Init {
        #[arg(long, default_value = "claude", value_parser = ["claude", "codex"])]
        agent: String,
        #[arg(long)]
        project: bool,
    },
    /// Start a daemon-owned workflow in its own task worktree; print the run id.
    Run {
        name: String,
        #[arg(last = true)]
        task: Vec<String>,
    },
    /// Show saved run progress and reports (all runs if no id is given).
    Status {
        run: Option<u64>,
        #[arg(long)]
        json: bool,
    },
    /// Submit this stage's Markdown report, then finish the agent turn.
    Finish {
        run: u64,
        #[arg(long)]
        report: PathBuf,
        #[arg(long, value_enum)]
        verdict: wf::Verdict,
    },
    /// Human only: approve the saved plan and queue the build.
    Approve { run: u64 },
    /// Human only: explicitly retry an interrupted/failed stage, keeping files.
    Retry { run: u64 },
    /// Human only: stop sequencing; preserve existing panes and files.
    Cancel { run: u64 },
    #[command(hide = true)]
    Stage { path: PathBuf },
}

pub(crate) fn definitions(dir: &std::path::Path) -> Result<Vec<wf::ResolvedWorkflow>> {
    wf::load(&Config::load()?, dir)
}

pub(crate) fn start(name: String, goal: String, cwd: PathBuf) -> Result<String> {
    match super::block_on(super::request(ClientMsg::Command(Command::WorkflowStart {
        name,
        cwd,
        goal,
    })))? {
        Reply::Text(id) => Ok(id),
        _ => bail!("unexpected workflow start reply"),
    }
}

pub(crate) fn status(id: Option<u64>) -> Result<String> {
    let runs = super::snapshot()?.workflows;
    if let Some(id) = id {
        let run = runs
            .iter()
            .find(|r| r.id == id)
            .context("no such workflow run")?;
        Ok(describe(run))
    } else {
        Ok(runs
            .iter()
            .map(|r| {
                format!(
                    "{} · {} · {} · {} · round {}",
                    r.id,
                    r.name,
                    r.state.label(),
                    r.stage.label(),
                    r.round
                )
            })
            .collect::<Vec<_>>()
            .join("\n"))
    }
}

pub(crate) fn describe(run: &wf::Run) -> String {
    let pending = run
        .pending
        .as_ref()
        .map(|r| {
            format!(
                "\nReceived stage report (turn not yet finished)\n{}\n",
                r.text
            )
        })
        .unwrap_or_default();
    format!(
        "Run {} · {}\nState: {}\nStage: {} · round {} of {}\nCheckout: {}\nPane: {}\n{}\n\nGoal\n{}\n\nPlan\n{}\n\nWorker report\n{}\n\nReview\n{}\n{pending}",
        run.id,
        run.name,
        run.state.label(),
        run.stage.label(),
        run.round,
        wf::MAX_ROUNDS,
        run.dir.display(),
        run.term
            .map(|id| id.to_string())
            .unwrap_or_else(|| "none".into()),
        run.note,
        run.goal,
        run.plan,
        run.work,
        run.review
    )
}

pub(crate) fn command(cmd: WorkflowCmd) -> Result<()> {
    ensure!(
        crate::ipc::remote().is_none(),
        "workflow setup runs on this machine; run the command on the remote host"
    );
    let dir = std::env::current_dir().context("finding workflow checkout")?;
    match cmd {
        WorkflowCmd::List => {
            for entry in definitions(&dir)? {
                let w = entry.workflow;
                println!(
                    "{} · {} → approval → {} → {} · {}",
                    w.name,
                    w.planner,
                    w.worker,
                    w.reviewer,
                    if entry.project { "project" } else { "personal" }
                );
            }
        }
        WorkflowCmd::Show { name } => {
            let list = definitions(&dir)?;
            let w = list
                .iter()
                .find(|w| w.workflow.name == name)
                .context("no workflow with this name; run seshi workflow list")?;
            print!("{}", wf::show(&w.workflow));
        }
        WorkflowCmd::Init { agent, project } => {
            wf::init(
                &Config::load()?,
                &dir,
                if agent == "codex" {
                    Runner::Codex
                } else {
                    Runner::Claude
                },
                project,
            )?;
            println!("Saved feature workflow. Use seshi workflow show feature to inspect it.");
        }
        WorkflowCmd::Run { name, task } => println!("{}", start(name, task.join(" "), dir)?),
        WorkflowCmd::Status { run, json } => {
            if json {
                let mut runs = super::snapshot()?.workflows;
                if let Some(id) = run {
                    runs.retain(|r| r.id == id);
                    ensure!(!runs.is_empty(), "no such workflow run");
                }
                println!("{}", serde_json::to_string_pretty(&runs)?);
            } else {
                println!("{}", status(run)?);
            }
        }
        WorkflowCmd::Finish {
            run,
            report,
            verdict,
        } => {
            let mut bytes = Vec::new();
            std::fs::File::open(report)
                .context("opening stage report")?
                .take((wf::MAX_REPORT + 1) as u64)
                .read_to_end(&mut bytes)
                .context("reading stage report")?;
            ensure!(bytes.len() <= wf::MAX_REPORT, "report is too large");
            let text = String::from_utf8(bytes).context("report must be UTF-8")?;
            super::command(Command::WorkflowFinish {
                run,
                report: wf::Report { text, verdict },
            })?;
            println!("Report accepted. Finish your turn; Seshi will advance after it ends.");
        }
        WorkflowCmd::Approve { run } => super::command(Command::WorkflowControl {
            run,
            action: WorkflowAction::Approve,
        })?,
        WorkflowCmd::Retry { run } => super::command(Command::WorkflowControl {
            run,
            action: WorkflowAction::Retry,
        })?,
        WorkflowCmd::Cancel { run } => super::command(Command::WorkflowControl {
            run,
            action: WorkflowAction::Cancel,
        })?,
        WorkflowCmd::Stage { path } => {
            const MAX_LAUNCH: u64 = 131_072;
            let mut bytes = Vec::new();
            std::fs::File::open(path)
                .context("opening workflow launch")?
                .take(MAX_LAUNCH + 1)
                .read_to_end(&mut bytes)
                .context("reading workflow launch")?;
            ensure!(
                bytes.len() as u64 <= MAX_LAUNCH,
                "workflow launch is too large"
            );
            let launch: wf::Launch =
                serde_json::from_slice(&bytes).context("parsing workflow launch")?;
            let args = launch.profile.args(&launch.task)?;
            let status = crate::proc::agent(launch.profile.agent.command(), &args)?
                .status()
                .context("starting workflow profile")?;
            ensure!(status.success(), "workflow agent exited with {status}");
        }
    }
    Ok(())
}
