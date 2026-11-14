//! `<app> workflow report`: a workflow's progress, sent from a process inside a pane (ADR 0010).
//! The workflow runs itself; this only tells the daemon where it is, for display.

use super::block_on;
use crate::brand::{APP_COMMAND, APP_NAME};
use crate::ipc;
use crate::protocol::{ClientMsg, TermId, WORKFLOW_REPORT_SINCE};
use crate::workflow::{WorkflowRun, checked};
use anyhow::{Context, Result, anyhow, bail};
use std::io::Read;
use std::time::Duration;

/// How long a report may take to reach the daemon before the workflow is let go.
const SEND_TIMEOUT: Duration = Duration::from_secs(3);

/// Reads one run as JSON from stdin and sends it for this pane. Outside a pane it says so and
/// succeeds, since a workflow also runs where there is nobody to tell.
pub(crate) fn report() -> Result<()> {
    let mut input = String::new();
    std::io::stdin()
        .read_to_string(&mut input)
        .context("could not read the report from stdin")?;
    let run = parse(&input)?;
    let term = std::env::var(concat!(env!("APP_ENV"), "_TERM_ID"))
        .ok()
        .and_then(|s| s.parse::<TermId>().ok());
    let token = std::env::var(concat!(env!("APP_ENV"), "_PANE_TOKEN")).ok();
    let (Some(term), Some(token)) = (term, token) else {
        eprintln!("{APP_COMMAND} workflow report: not inside a {APP_NAME} pane, so nothing was reported");
        return Ok(());
    };
    let msg = ClientMsg::WorkflowReport { term, token, run };
    block_on(async {
        tokio::time::timeout(SEND_TIMEOUT, async {
            let ((_r, mut w), version) = ipc::open_knowing_version(false).await?;
            if let Some(version) = version.filter(|v| *v < WORKFLOW_REPORT_SINCE) {
                bail!(
                    "the running {APP_NAME} service (protocol {version}) is older than workflow reports; restart it to show this run"
                );
            }
            ipc::send(&mut w, &msg).await
        })
        .await
        .unwrap_or_else(|_| Err(anyhow!("the {APP_NAME} service did not take the report in time")))
    })
}

fn parse(input: &str) -> Result<WorkflowRun> {
    let run: WorkflowRun =
        serde_json::from_str(input).context("the report is not a workflow run in JSON")?;
    checked(run).map_err(|reason| anyhow!("the report was refused: {reason}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflow::StepState;

    #[test]
    fn a_report_is_read_from_json_and_checked() {
        let run = parse(
            r#"{"workflow":"kitchen","run":"007","ticket":{"key":"MAITTY-42","title":"Add export"},
                "steps":[{"name":"intake","state":"passed"},{"name":"build","state":"working","helper":"MAITTY-42 builder"}]}"#,
        )
        .unwrap();
        assert_eq!(run.steps[1].state, StepState::Working);
        assert_eq!(run.steps[1].helper.as_deref(), Some("MAITTY-42 builder"));
        assert_eq!(run.ticket.unwrap().key, "MAITTY-42");
    }

    #[test]
    fn a_report_that_is_not_a_run_or_not_allowed_says_why() {
        let not_json = parse("steps: build").unwrap_err();
        assert!(format!("{not_json:#}").contains("not a workflow run"));
        let empty = parse(r#"{"workflow":"kitchen","run":"1","steps":[]}"#).unwrap_err();
        assert!(format!("{empty:#}").contains("at least one step"));
    }
}
