//! The MCP tools against another machine's real daemon behind the fake-ssh shim (see TESTING,
//! "testing over ssh"). This computer's own daemon is never dialled: the calling pane's grants,
//! which this computer's daemon would answer, are played by a stand-in that answers the way the
//! daemon's grant check does (that check is pinned in the daemon's own tests).

use super::cli_machine_tests::{Build, open_workspace, panes, until};
use crate::cli;
use crate::cli::mcp::{Reach, Server};
use crate::protocol::{ClientMsg, Command, Query, Reply, TermId};
use serde_json::{Value, json};

const LIST: &str = concat!(env!("APP_COMMAND"), "_list");
const READ: &str = concat!(env!("APP_COMMAND"), "_read");
const SEND: &str = concat!(env!("APP_COMMAND"), "_send");
const WAIT: &str = concat!(env!("APP_COMMAND"), "_wait");
const START: &str = concat!(env!("APP_COMMAND"), "_start");
const ANSWER: &str = concat!(env!("APP_COMMAND"), "_answer");
const INTERRUPT: &str = concat!(env!("APP_COMMAND"), "_interrupt");

const READ_DENIED: &str =
    "this pane may not read other panes through maitty (maitty grant 4 read allows it)";
const WRITE_DENIED: &str =
    "this pane may not type into other panes through maitty (maitty grant 4 write allows it)";

#[derive(Clone, Copy)]
enum Grants {
    All,
    ReadOnly,
    Nothing,
}

/// How this computer's daemon answers the calling pane's grant probes.
fn caller(grants: Grants) -> impl Fn(ClientMsg) -> Result<Reply, String> {
    move |msg| match (msg, grants) {
        (ClientMsg::Query(Query::Read { .. }), Grants::Nothing) => Err(READ_DENIED.into()),
        (ClientMsg::Query(Query::Read { term }), _) => Err(format!("no pane {term}")),
        (ClientMsg::Command(Command::ClearQueue { .. }), Grants::All) => Ok(Reply::Ok),
        (ClientMsg::Command(Command::ClearQueue { .. }), _) => Err(WRITE_DENIED.into()),
        _ => Err("the probe asked something else".into()),
    }
}

/// Every configured machine, open to agents except those named in `closed`.
fn opened(others: &[(&str, &str)], closed: &[&str]) -> Build {
    let mut build = Build::start(others);
    for m in &mut build.cfg.machines {
        m.mcp = !closed.contains(&m.name.as_str());
    }
    build
}

fn server_as(build: &Build, grants: Grants, me: Option<TermId>) -> Server {
    let shim = build.sandbox.shim.to_string_lossy().into_owned();
    Server::new(
        build.cfg.clone(),
        std::env::temp_dir(),
        me,
        Reach::new(&shim, caller(grants)),
    )
}

fn server(build: &Build, grants: Grants) -> Server {
    server_as(build, grants, None)
}

fn said(reply: &Value) -> &str {
    reply["content"][0]["text"].as_str().expect("text")
}

fn is_error(reply: &Value) -> bool {
    reply["isError"] == true
}

fn code(reply: &Value) -> &str {
    assert!(is_error(reply), "expected an error: {reply}");
    reply["structuredContent"]["error"]["code"]
        .as_str()
        .unwrap_or_else(|| panic!("no code in {reply}"))
}

fn rows(reply: &Value) -> Vec<Value> {
    let text = said(reply);
    let (marker, body) = text.split_once('\n').expect("a marker line then the list");
    assert_eq!(marker, "[machine: build]");
    serde_json::from_str(body).expect("a JSON list")
}

fn two_panes(build: &Build) -> (TermId, TermId) {
    let target = build.target("build");
    let first = open_workspace(&target, "talk");
    cli::split(&target, Some(first), false, Vec::new()).expect("split");
    let both = until("two panes", || {
        Some(panes(&target)).filter(|p| p.len() == 2)
    });
    (first, *both.iter().find(|p| **p != first).unwrap())
}

#[test]
fn an_agent_lists_reads_and_sends_to_another_machine_s_panes() {
    let build = opened(&[], &[]);
    let (first, second) = two_panes(&build);
    // The same number here must not make the remote pane "you".
    let server = server_as(&build, Grants::All, Some(first));

    let listed = server.call(LIST, &json!({ "machine": "build" }));
    let listed = rows(&listed);
    let ids: Vec<&str> = listed.iter().map(|r| r["id"].as_str().unwrap()).collect();
    assert!(ids.contains(&format!("build/{first}").as_str()), "{ids:?}");
    assert!(ids.contains(&format!("build/{second}").as_str()), "{ids:?}");
    assert!(listed.iter().all(|r| r["machine"] == "build"));

    let id = format!("build/{first}");
    let sent = server.call(SEND, &json!({ "id": id, "text": "echo mcp-marker-31" }));
    assert!(!is_error(&sent), "{sent}");
    assert!(said(&sent).starts_with(&format!("[machine: build]\nsent to {id} (")));

    let waited = server.call(
        WAIT,
        &json!({ "id": id, "regex": r"^mcp-marker-31\s*$", "timeout": 30 }),
    );
    assert_eq!(said(&waited), "[machine: build]\nmatched: mcp-marker-31");

    let there = server.call(READ, &json!({ "machine": "build", "id": first }));
    assert!(said(&there).starts_with("[machine: build]\n"), "{there}");
    assert!(said(&there).contains("mcp-marker-31"), "{there}");
    let elsewhere = server.call(READ, &json!({ "id": format!("build/{second}") }));
    assert!(
        !said(&elsewhere).contains("mcp-marker-31"),
        "only the named pane got it"
    );
}

#[test]
fn every_id_the_list_prints_can_be_passed_back_unchanged() {
    let build = opened(&[], &[]);
    let (first, _) = two_panes(&build);
    let server = server(&build, Grants::All);
    let listed = rows(&server.call(LIST, &json!({ "machine": "build" })));
    assert_eq!(listed.len(), 2);
    for row in listed {
        let id = row["id"].clone();
        let read = server.call(READ, &json!({ "id": id }));
        assert!(!is_error(&read), "{id}: {read}");
        let wait = server.call(
            WAIT,
            &json!({ "id": id, "regex": "^$never$", "timeout": 1 }),
        );
        assert!(
            !said(&wait).contains("no session") && !said(&wait).contains("give the session id"),
            "{id} found its pane: {wait}"
        );
    }
    let both = server.call(
        READ,
        &json!({ "machine": "build", "id": format!("build/{first}") }),
    );
    assert!(
        !is_error(&both),
        "naming the machine twice, the same way, is fine"
    );
    let clash = server.call(
        READ,
        &json!({ "machine": "gpu", "id": format!("build/{first}") }),
    );
    assert!(
        is_error(&clash) && said(&clash).contains("different machines"),
        "{clash}"
    );
}

#[test]
fn a_machine_not_opened_to_agents_is_refused_before_it_is_dialled() {
    let build = opened(&[("gpu", "proxy")], &["gpu"]);
    let server = server(&build, Grants::All);
    let before = build.dials();
    for args in [
        json!({ "machine": "gpu" }),
        json!({ "machine": "GPU", "id": 1 }),
        json!({ "id": "gpu/1", "text": "rm -rf /" }),
    ] {
        for tool in [LIST, READ, SEND, WAIT, START] {
            let reply = server.call(tool, &args);
            assert_eq!(code(&reply), "machine_not_enabled", "{tool} {args}");
            assert!(said(&reply).contains("mcp = true"), "{reply}");
        }
    }
    assert_eq!(build.dials(), before, "no ssh was started");
}

#[test]
fn an_unknown_machine_names_the_ones_that_exist() {
    let build = opened(&[], &[]);
    let reply = server(&build, Grants::All).call(READ, &json!({ "id": "bild/1" }));
    assert_eq!(code(&reply), "unknown_machine");
    assert!(said(&reply).contains("known: local, build"), "{reply}");
}

#[test]
fn a_pane_that_may_only_read_cannot_send_to_another_machine() {
    let build = opened(&[], &[]);
    let (first, _) = two_panes(&build);
    let server = server(&build, Grants::ReadOnly);
    let before = build.dials();
    let id = format!("build/{first}");

    for args in [
        json!({ "id": id, "text": "echo nope" }),
        json!({ "id": id, "text": "echo nope", "wait": true }),
    ] {
        let reply = server.call(SEND, &args);
        assert_eq!(code(&reply), "permission_denied", "{args}");
        assert!(said(&reply).contains(WRITE_DENIED), "{reply}");
        assert!(said(&reply).contains("build was not contacted"), "{reply}");
    }
    assert_eq!(build.dials(), before, "nothing was dialled");

    let read = server.call(READ, &json!({ "id": id }));
    assert!(!is_error(&read), "reading is allowed: {read}");
}

#[test]
fn a_pane_that_may_not_read_cannot_list_read_or_wait_on_another_machine() {
    let build = opened(&[], &[]);
    let server = server(&build, Grants::Nothing);
    let before = build.dials();
    for (tool, args) in [
        (LIST, json!({ "machine": "build" })),
        (READ, json!({ "id": "build/1" })),
        (WAIT, json!({ "id": "build/1", "timeout": 1 })),
    ] {
        let reply = server.call(tool, &args);
        assert_eq!(code(&reply), "permission_denied", "{tool}");
        assert!(said(&reply).contains(READ_DENIED), "{reply}");
    }
    let entries = server.machine_entries();
    assert_eq!(entries.rows[0]["error"], "permission_denied");
    assert!(entries.dialled.is_empty());
    assert_eq!(build.dials(), before, "nothing was dialled");
}

#[test]
fn starting_answering_and_interrupting_on_another_machine_are_local_only() {
    let build = opened(&[], &[]);
    let before = build.dials();
    for grants in [Grants::All, Grants::Nothing] {
        let server = server(&build, grants);
        for (tool, args) in [
            (
                START,
                json!({ "machine": "build", "prompt": "fix it", "worktree": true }),
            ),
            (START, json!({ "machine": "build", "prompt": "fix it" })),
            (
                ANSWER,
                json!({ "id": "build/1", "choice": 1, "prompt": "ab" }),
            ),
            (INTERRUPT, json!({ "id": "build/1" })),
        ] {
            let reply = server.call(tool, &args);
            assert_eq!(code(&reply), "unsupported_for_remote", "{tool}");
            assert!(said(&reply).contains("local-only in v1"), "{reply}");
        }
    }
    assert_eq!(build.dials(), before, "nothing was dialled");
}

#[test]
fn listing_every_machine_reports_the_one_that_is_down_inline() {
    let build = opened(&[("gpu", "unreachable")], &[]);
    let target = build.target("build");
    let pane = open_workspace(&target, "up");
    let entries = server(&build, Grants::All).machine_entries();

    assert_eq!(entries.dialled, ["build", "gpu"]);
    let up = entries
        .rows
        .iter()
        .find(|r| r["id"] == format!("build/{pane}"));
    assert!(up.is_some(), "{:?}", entries.rows);
    let down = entries.rows.iter().find(|r| r["machine"] == "gpu").unwrap();
    assert_eq!(down["error"], "unreachable");
    assert!(
        down["message"]
            .as_str()
            .unwrap()
            .starts_with("gpu: unreachable ("),
        "{down}"
    );
}
