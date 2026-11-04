//! `ls --all`: this computer and every configured machine, each asked on its own, so one that
//! is down costs a line on stderr and not the whole listing.

use super::target::{Target, resolve_machine};
use super::{block_on, request_to, workspace_lines, workspaces_json};
use crate::config::Config;
use crate::ipc;
use crate::protocol::{ClientMsg, Query, Reply, Snapshot};
use anyhow::{Result, anyhow, bail};
use serde_json::Value;
use std::time::Duration;

/// How long one machine has to answer, beyond the connection's own budget.
const ANSWER_SLACK: Duration = Duration::from_secs(10);

/// What `ls --all` prints: the listing, and a line for each machine that did not answer.
pub(crate) struct Listing {
    pub out: String,
    pub warnings: Vec<String>,
}

/// The machines `--all` asks: this computer, then the configured ones in name order.
fn every_target(cfg: &Config, ssh: &str) -> Vec<Target> {
    let mut names: Vec<&str> = cfg.machines.iter().map(|m| m.name.as_str()).collect();
    names.sort_by_key(|name| name.to_lowercase());
    std::iter::once(maitty_settings::config::LOCAL_MACHINE)
        .chain(names)
        .filter_map(|name| resolve_machine(name, cfg, ssh).ok())
        .map(Target::of)
        .collect()
}

fn ask(target: &Target) -> Result<Snapshot> {
    let limit = ipc::CONNECT_BUDGET + ANSWER_SLACK;
    let answer = block_on(async {
        let asked = request_to(target, ClientMsg::Query(Query::List));
        match tokio::time::timeout(limit, asked).await {
            Ok(reply) => reply,
            Err(_) => Err(target.connect_error(anyhow!("no answer in {}s", limit.as_secs()))),
        }
    })?;
    match answer {
        Reply::List(snap) => Ok(snap),
        _ => bail!("unexpected reply"),
    }
}

/// Every machine's answer in order, asked at the same time.
pub(crate) fn ask_all(targets: &[Target]) -> Vec<Result<Snapshot>> {
    std::thread::scope(|scope| {
        let asking: Vec<_> = targets
            .iter()
            .map(|target| scope.spawn(|| ask(target)))
            .collect();
        asking
            .into_iter()
            .map(|asked| {
                asked
                    .join()
                    .unwrap_or_else(|_| Err(anyhow!("the query stopped")))
            })
            .collect()
    })
}

/// A failed machine as one line: a remote's error already starts with its name.
fn warning(target: &Target, error: &anyhow::Error) -> String {
    if target.is_remote() {
        format!("{error}")
    } else {
        format!("{}: {error:#}", target.machine().name())
    }
}

/// The listing of every machine in `targets` that answered (workspaces tagged with their
/// machine). `Err` when none did.
pub(crate) fn list(targets: &[Target], as_json: bool) -> Result<Listing> {
    let mut answered = Vec::new();
    let mut warnings = Vec::new();
    for (target, result) in targets.iter().zip(ask_all(targets)) {
        match result {
            Ok(snap) => answered.push((target, snap)),
            Err(error) => warnings.push(warning(target, &error)),
        }
    }
    if answered.is_empty() {
        bail!("{}\nno machine answered", warnings.join("\n"));
    }
    let out = if as_json {
        let items: Vec<Value> = answered
            .iter()
            .flat_map(|(target, snap)| workspaces_json(snap, target.name()))
            .collect();
        serde_json::to_string_pretty(&items)?
    } else {
        let sections: Vec<String> = answered
            .iter()
            .map(|(target, snap)| workspace_lines(snap, target).join("\n"))
            .collect();
        sections.join("\n\n")
    };
    Ok(Listing { out, warnings })
}

/// `ls --all`: prints the listing, and each machine that did not answer on stderr. Succeeds
/// when at least one machine answered.
pub(crate) fn all(cfg: &Config, ssh: &str, as_json: bool) -> Result<()> {
    let listing = list(&every_target(cfg, ssh), as_json)?;
    for line in &listing.warnings {
        eprintln!("{line}");
    }
    println!("{}", listing.out);
    Ok(())
}
