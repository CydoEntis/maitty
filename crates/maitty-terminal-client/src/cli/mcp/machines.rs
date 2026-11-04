//! The `machine` argument and `machine/id` ids of the MCP tools.
//!
//! An MCP call is made by an agent, a model in a pane on this computer, and the text another
//! machine returns can be written to steer that model. So reaching a machine from here is
//! opt-in per machine (`mcp = true`), is checked against the calling pane's own grants on this
//! computer before anything is dialled (the machine's daemon is never told which pane is
//! asking, so it cannot check them), and everything it says comes back cleaned, cut, and led by
//! a `[machine: name]` line so the model can tell whose words they are.

use super::{Server, row_of, text};
use crate::cli::machines_ls;
use crate::cli::target::{PaneAddress, Target, UnknownMachine, resolve_machine};
use crate::ipc;
use crate::protocol::*;
use serde_json::{Value, json};

/// What `machine` means in a list: this computer and every machine open to agents.
const ALL: &str = "all";

/// A pane id no daemon hands out, used to ask this computer's daemon whether the calling pane
/// holds a grant without touching any real pane.
const PROBE_TERM: TermId = TermId::MAX;

/// The most characters of another machine's words returned in one reply.
const REPLY_MAX_CHARS: usize = 32 * 1024;

const START_IS_LOCAL: &str = "starting agents and worktrees is local-only in v1: a new agent's folder and worktree need this computer's files";
const ANSWER_IS_LOCAL: &str = "answering a prompt or interrupting is local-only in v1: tell the user what the other machine's agent is waiting on";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Tool {
    List,
    Read,
    Send,
    Wait,
    Answer,
    Start,
    Interrupt,
}

/// The pane tool `name` is, or `None` for a tool that has no machine to choose (references,
/// secrets), which stays on this computer.
pub(super) fn tool_of(name: &str) -> Option<Tool> {
    Some(match name {
        concat!(env!("APP_COMMAND"), "_list") => Tool::List,
        concat!(env!("APP_COMMAND"), "_read") => Tool::Read,
        concat!(env!("APP_COMMAND"), "_send") => Tool::Send,
        concat!(env!("APP_COMMAND"), "_wait") => Tool::Wait,
        concat!(env!("APP_COMMAND"), "_answer") => Tool::Answer,
        concat!(env!("APP_COMMAND"), "_start") => Tool::Start,
        concat!(env!("APP_COMMAND"), "_interrupt") => Tool::Interrupt,
        _ => return None,
    })
}

/// Why a call that names a machine was refused, as the MCP error the agent reads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum McpError {
    MachineNotEnabled(String),
    UnknownMachine(UnknownMachine),
    PermissionDenied {
        machine: String,
        why: String,
    },
    UnsupportedForRemote(String),
    /// The machine could not be reached: its name and what to do about it, on one line.
    Unreachable(String),
    Invalid(String),
    /// The machine answered with an error of its own.
    Failed(String),
}

impl McpError {
    pub(crate) fn code(&self) -> Option<&'static str> {
        match self {
            McpError::MachineNotEnabled(_) => Some("machine_not_enabled"),
            McpError::UnknownMachine(_) => Some("unknown_machine"),
            McpError::PermissionDenied { .. } => Some("permission_denied"),
            McpError::UnsupportedForRemote(_) => Some("unsupported_for_remote"),
            McpError::Unreachable(_) => Some("unreachable"),
            McpError::Invalid(_) | McpError::Failed(_) => None,
        }
    }

    pub(crate) fn into_value(self) -> Value {
        let message = self.to_string();
        let Some(code) = self.code() else {
            return json!({ "content": [{ "type": "text", "text": message }], "isError": true });
        };
        json!({
            "content": [{ "type": "text", "text": format!("{code}: {message}") }],
            "structuredContent": { "error": { "code": code, "message": message } },
            "isError": true,
        })
    }
}

impl std::fmt::Display for McpError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            McpError::MachineNotEnabled(name) => write!(
                f,
                "machine \"{name}\" is not open to agents; the user can allow it with `mcp = true` in its [[machines]] entry in config.local.toml. Nothing was sent to it"
            ),
            McpError::UnknownMachine(e) => e.fmt(f),
            McpError::PermissionDenied { machine, why } => {
                write!(f, "{why}; {machine} was not contacted")
            }
            McpError::UnsupportedForRemote(what) => f.write_str(what),
            McpError::Unreachable(line) | McpError::Invalid(line) | McpError::Failed(line) => {
                f.write_str(line)
            }
        }
    }
}

/// What the calling pane needs from its own grants on this computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Needs {
    Read,
    Write,
}

/// How a call reaches machines: the program that runs ssh, and how to ask this computer's
/// daemon things as the calling pane.
pub(crate) struct Reach {
    ssh: String,
    ask_local: Box<dyn Fn(ClientMsg) -> Result<Reply, String>>,
}

impl Reach {
    pub(crate) fn live() -> Reach {
        Reach::new(ipc::SSH_PROGRAM, |msg| {
            crate::cli::block_on(crate::cli::request(msg)).map_err(|e| format!("{e:#}"))
        })
    }

    pub(crate) fn new(
        ssh: &str,
        ask_local: impl Fn(ClientMsg) -> Result<Reply, String> + 'static,
    ) -> Reach {
        Reach {
            ssh: ssh.into(),
            ask_local: Box::new(ask_local),
        }
    }

    /// Whether the calling pane holds the grant on this computer, decided by this computer's
    /// daemon with the same table it uses for the same tool used locally: a read of, and a
    /// write to, a pane that does not exist pass the grant check and then find nothing. Any
    /// answer but that one means no, including a daemon that did not answer.
    // shortcut: probing with an absent pane instead of asking for the grants, which would
    // need a new query and a protocol version bump.
    pub(super) fn caller_may(&self, needs: Needs) -> Result<(), String> {
        match needs {
            Needs::Read => {
                match (self.ask_local)(ClientMsg::Query(Query::Read { term: PROBE_TERM })) {
                    Err(e) if e == format!("no pane {PROBE_TERM}") => Ok(()),
                    Err(e) => Err(e),
                    Ok(_) => Ok(()),
                }
            }
            Needs::Write => {
                (self.ask_local)(ClientMsg::Command(Command::ClearQueue { term: PROBE_TERM }))
                    .map(|_| ())
            }
        }
    }
}

/// Where a call goes.
pub(super) enum Place {
    Local,
    Remote(Target),
    Everywhere,
}

/// The pane an `id` argument names: a number, or `machine/number`. An `id` of any other kind
/// is left for the tool to refuse in its own words.
fn address_of(args: &Value) -> Result<Option<PaneAddress>, McpError> {
    match args.get("id") {
        Some(Value::Number(n)) => Ok(n.as_u64().map(|term| PaneAddress {
            machine: None,
            term: term as TermId,
        })),
        Some(Value::String(s)) => s
            .parse()
            .map(Some)
            .map_err(|e: crate::cli::target::BadAddress| McpError::Invalid(e.to_string())),
        _ => Ok(None),
    }
}

/// `args` with a `machine/id` id turned into the bare number the local tools read.
pub(super) fn bare(args: &Value, address: &Option<PaneAddress>) -> Value {
    let mut args = args.clone();
    if let Some(a) = address {
        args["id"] = json!(a.term);
    }
    args
}

/// Another machine's words made safe to hand to a model: no control or hidden characters,
/// cut to a size, and led by whose they are.
pub(super) fn finish(at: &Target, body: &str) -> String {
    if !at.is_remote() {
        return body.to_string();
    }
    marked(&[at.machine().name()], &at.plain(body))
}

fn marked(machines: &[&str], body: &str) -> String {
    let cut = if body.chars().count() > REPLY_MAX_CHARS {
        let head: String = body.chars().take(REPLY_MAX_CHARS).collect();
        format!("{head}\n[cut: more than {REPLY_MAX_CHARS} characters]")
    } else {
        body.to_string()
    };
    let label = if machines.len() == 1 {
        "machine"
    } else {
        "machines"
    };
    format!("[{label}: {}]\n{cut}", machines.join(", "))
}

/// An error a machine's reply or connection raised: unreachable when it is the one-line failure
/// `Target::connect_error` writes, which begins with the machine's name.
fn failure(at: &Target, error: String) -> McpError {
    let line = at.plain(&error);
    if line.starts_with(&format!("{}: ", at.machine().name())) {
        McpError::Unreachable(line)
    } else {
        McpError::Failed(line)
    }
}

/// What asking every open machine for its panes came to.
pub(crate) struct Entries {
    pub rows: Vec<Value>,
    /// The machines that were dialled, whose words are in `rows`.
    pub dialled: Vec<String>,
}

impl Server {
    fn place(&self, tool: Tool, args: &Value) -> Result<(Place, Option<PaneAddress>), McpError> {
        let address = address_of(args)?;
        let given = match args.get("machine") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) if !s.trim().is_empty() => Some(s.trim()),
            Some(_) => {
                return Err(McpError::Invalid(
                    "machine is a machine's name, `local` or `all`".into(),
                ));
            }
        };
        let in_id = address.as_ref().and_then(|a| a.machine.as_deref());
        let name = match (given, in_id) {
            (Some(g), Some(i)) if !g.eq_ignore_ascii_case(i) => {
                return Err(McpError::Invalid(format!(
                    "machine {g} and the id {i}/… name different machines; give one"
                )));
            }
            (Some(name), _) | (None, Some(name)) => name,
            (None, None) => return Ok((Place::Local, address)),
        };
        if tool == Tool::List && name.eq_ignore_ascii_case(ALL) {
            return Ok((Place::Everywhere, address));
        }
        if name.eq_ignore_ascii_case(maitty_settings::config::LOCAL_MACHINE) {
            return Ok((Place::Local, address));
        }
        let machine =
            resolve_machine(name, &self.cfg, &self.reach.ssh).map_err(McpError::UnknownMachine)?;
        if !self.opted_in(name) {
            return Err(McpError::MachineNotEnabled(machine.name().into()));
        }
        Ok((Place::Remote(Target::of(machine)), address))
    }

    fn opted_in(&self, name: &str) -> bool {
        self.cfg
            .machines
            .iter()
            .any(|m| m.mcp && m.name.eq_ignore_ascii_case(name))
    }

    pub(super) fn route(&self, tool: Tool, args: &Value) -> Result<Routed, McpError> {
        let (place, address) = self.place(tool, args)?;
        let args = bare(args, &address);
        Ok(match place {
            Place::Local => Routed::Local(args),
            Place::Remote(at) => Routed::Remote(at, args),
            Place::Everywhere => Routed::Everywhere,
        })
    }

    pub(super) fn call_remote(&self, tool: Tool, at: &Target, args: &Value) -> Value {
        match self.remote_text(tool, at, args) {
            Ok(body) => text(finish(at, &body)),
            Err(McpError::Failed(line)) => McpError::Failed(finish(at, &line)).into_value(),
            Err(e) => e.into_value(),
        }
    }

    fn remote_text(&self, tool: Tool, at: &Target, args: &Value) -> Result<String, McpError> {
        let machine = at.machine().name();
        let fail = |e: String| failure(at, e);
        let (writes, reads) = match tool {
            Tool::Start => return Err(McpError::UnsupportedForRemote(START_IS_LOCAL.into())),
            Tool::Answer | Tool::Interrupt => {
                return Err(McpError::UnsupportedForRemote(ANSWER_IS_LOCAL.into()));
            }
            Tool::Send => (true, args["wait"].as_bool() == Some(true)),
            Tool::List | Tool::Read | Tool::Wait => (false, true),
        };
        if writes {
            self.may(Needs::Write, machine)?;
        }
        if reads {
            self.may(Needs::Read, machine)?;
        }
        match tool {
            Tool::List => self.list(at).map_err(fail),
            Tool::Read => self.read(at, args).map_err(fail),
            Tool::Send => self.send(at, args).map_err(fail),
            Tool::Wait => self
                .session_on(at, args)
                .and_then(|t| {
                    let regex = args.get("regex").and_then(Value::as_str);
                    self.wait(at, t.id, regex, args, false)
                })
                .map_err(fail),
            Tool::Start | Tool::Answer | Tool::Interrupt => Err(McpError::Invalid(
                "that tool does not reach another machine".into(),
            )),
        }
    }

    fn may(&self, needs: Needs, machine: &str) -> Result<(), McpError> {
        self.reach
            .caller_may(needs)
            .map_err(|why| McpError::PermissionDenied {
                machine: machine.into(),
                why,
            })
    }

    /// The machines open to agents, in name order.
    fn open_machines(&self) -> Vec<Target> {
        let mut names: Vec<&str> = self
            .cfg
            .machines
            .iter()
            .filter(|m| m.mcp)
            .map(|m| m.name.as_str())
            .collect();
        names.sort_by_key(|n| n.to_lowercase());
        names
            .into_iter()
            .filter_map(|n| resolve_machine(n, &self.cfg, &self.reach.ssh).ok())
            .map(Target::of)
            .collect()
    }

    /// The panes of every machine open to agents, each machine asked at the same time. One
    /// that cannot be reached, or whose reading the calling pane may not do, is an entry with
    /// the reason, so the others still list.
    pub(crate) fn machine_entries(&self) -> Entries {
        let targets = self.open_machines();
        let name_of = |t: &Target| t.machine().name().to_string();
        if !targets.is_empty()
            && let Err(why) = self.reach.caller_may(Needs::Read)
        {
            let rows = targets
                .iter()
                .map(|t| {
                    let error = McpError::PermissionDenied {
                        machine: name_of(t),
                        why: why.clone(),
                    };
                    entry(t, "permission_denied", &error.to_string())
                })
                .collect();
            return Entries {
                rows,
                dialled: Vec::new(),
            };
        }
        let mut rows = Vec::new();
        for (at, answer) in targets.iter().zip(machines_ls::ask_all(&targets)) {
            match answer {
                Ok(snap) => rows.extend(snap.terms.values().map(|t| self.row_on(at, t))),
                Err(e) => rows.push(entry(at, "unreachable", &at.plain(&format!("{e}")))),
            }
        }
        Entries {
            rows,
            dialled: targets.iter().map(name_of).collect(),
        }
    }

    /// `<app>_list` with `machine: "all"`: this computer's panes, then each open machine's.
    pub(super) fn list_everywhere(&self) -> Value {
        let mut rows = match self.list_rows(&Target::current()) {
            Ok(rows) => rows,
            Err(e) => return McpError::Failed(e).into_value(),
        };
        for row in &mut rows {
            row["machine"] = json!(maitty_settings::config::LOCAL_MACHINE);
        }
        let Entries {
            rows: remote,
            dialled,
        } = self.machine_entries();
        rows.extend(remote);
        let body = serde_json::to_string_pretty(&rows).unwrap_or_default();
        if dialled.is_empty() {
            return text(body);
        }
        let names: Vec<&str> = dialled.iter().map(String::as_str).collect();
        text(marked(&names, &body))
    }
}

/// A machine that gave no panes, as a row the agent reads.
fn entry(at: &Target, code: &str, message: &str) -> Value {
    json!({ "machine": at.machine().name(), "error": code, "message": message })
}

/// Where a call goes, with its arguments ready for the tool.
pub(super) enum Routed {
    Local(Value),
    Remote(Target, Value),
    Everywhere,
}

/// A pane of another machine as a list row: its id carries the machine, so it can be passed
/// back to any tool unchanged.
pub(super) fn remote_row(at: &Target, t: &TermInfo) -> Value {
    let mut row = row_of(t, None);
    row["id"] = json!(at.address(t.id));
    row["machine"] = json!(at.machine().name());
    row
}

#[cfg(test)]
mod tests {
    use super::*;

    fn remote() -> Target {
        Target::of(ipc::Machine::ssh("build", vec!["ssh".into()], "maitty"))
    }

    #[test]
    fn another_machine_s_words_lose_control_characters_and_say_whose_they_are() {
        let said = finish(&remote(), "ok\u{1b}]52;c;QQ==\u{7}\u{202e} done\nnext");
        assert_eq!(said, "[machine: build]\nok]52;c;QQ== done\nnext");
    }

    #[test]
    fn this_computer_s_words_are_left_alone() {
        let here = Target::of(ipc::Machine::local());
        assert_eq!(finish(&here, "a\u{1b}[2Jb"), "a\u{1b}[2Jb");
        assert_eq!(finish(&Target::current(), "plain"), "plain");
    }

    #[test]
    fn a_long_reply_from_another_machine_is_cut() {
        let said = finish(&remote(), &"x".repeat(REPLY_MAX_CHARS + 10));
        assert!(
            said.ends_with("characters]"),
            "{}",
            &said[said.len() - 40..]
        );
        assert!(said.chars().count() < REPLY_MAX_CHARS + 100);
    }

    #[test]
    fn a_failed_connection_is_unreachable_and_another_error_is_the_machines_own() {
        let at = remote();
        assert_eq!(
            failure(&at, "build: needs a login; fix it".into()),
            McpError::Unreachable("build: needs a login; fix it".into())
        );
        assert_eq!(
            failure(&at, "no session build/9".into()),
            McpError::Failed("no session build/9".into())
        );
    }

    #[test]
    fn typed_errors_carry_their_code_in_text_and_structure() {
        let v = McpError::MachineNotEnabled("build".into()).into_value();
        assert_eq!(v["isError"], true);
        assert_eq!(
            v["structuredContent"]["error"]["code"],
            "machine_not_enabled"
        );
        let text = v["content"][0]["text"].as_str().unwrap();
        assert!(text.starts_with("machine_not_enabled: "), "{text}");
        assert!(text.contains("mcp = true"), "{text}");
        let plain = McpError::Failed("nope".into()).into_value();
        assert!(plain.get("structuredContent").is_none());
        assert_eq!(plain["content"][0]["text"], "nope");
    }

    #[test]
    fn an_id_is_a_number_or_machine_slash_number() {
        let id = |v: Value| address_of(&json!({ "id": v }));
        assert_eq!(id(json!(4)).unwrap().unwrap().term, 4);
        assert_eq!(id(json!("4")).unwrap().unwrap().machine, None);
        let a = id(json!("build/4")).unwrap().unwrap();
        assert_eq!((a.machine.as_deref(), a.term), (Some("build"), 4));
        assert!(matches!(id(json!("build/")), Err(McpError::Invalid(_))));
        assert!(matches!(id(json!("a/b/4")), Err(McpError::Invalid(_))));
        assert_eq!(address_of(&json!({})).unwrap(), None);
    }

    #[test]
    fn a_composite_id_is_turned_into_the_number_the_tools_read() {
        let address = address_of(&json!({ "id": "build/4" })).unwrap();
        assert_eq!(
            bare(&json!({ "id": "build/4", "text": "x" }), &address)["id"],
            4
        );
        assert_eq!(bare(&json!({ "text": "x" }), &None), json!({ "text": "x" }));
    }

    fn server_with(machines: &[(&str, bool)]) -> Server {
        let cfg = crate::config::Config {
            machines: machines
                .iter()
                .map(|(name, mcp)| maitty_settings::config::MachineDef {
                    name: (*name).into(),
                    target: format!("{name}.example"),
                    mcp: *mcp,
                    ..Default::default()
                })
                .collect(),
            ..Default::default()
        };
        let none = Reach::new("ssh", |_| Err("nobody asked".into()));
        Server::new(cfg, std::env::temp_dir(), None, none)
    }

    #[test]
    fn every_id_routes_back_to_the_machine_it_names() {
        let server = server_with(&[("build", true), ("gpu", false)]);
        let mixed = [
            (json!(4), None, 4),
            (json!("4"), None, 4),
            (json!("local/4"), None, 4),
            (json!("LOCAL/12"), None, 12),
            (json!("build/4"), Some("build"), 4),
            (json!("BUILD/7"), Some("build"), 7),
        ];
        for (id, machine, term) in mixed {
            let routed = server.route(Tool::Read, &json!({ "id": id })).ok().unwrap();
            match (routed, machine) {
                (Routed::Local(args), None) => assert_eq!(args["id"], term, "{id}"),
                (Routed::Remote(at, args), Some(name)) => {
                    assert_eq!(at.machine().name(), name, "{id}");
                    assert_eq!(args["id"], term, "{id}");
                    assert_eq!(at.address(term as TermId), format!("{name}/{term}"));
                }
                _ => panic!("{id} went the wrong way"),
            }
        }
    }

    #[test]
    fn the_machine_argument_alone_picks_the_machine_and_all_is_for_lists() {
        let server = server_with(&[("build", true)]);
        let by_arg = server.route(Tool::Send, &json!({ "machine": "build", "id": 3 }));
        assert!(matches!(by_arg, Ok(Routed::Remote(..))));
        let here = server.route(Tool::Read, &json!({ "machine": "local", "id": 3 }));
        assert!(matches!(here, Ok(Routed::Local(_))));
        let all = server.route(Tool::List, &json!({ "machine": "all" }));
        assert!(matches!(all, Ok(Routed::Everywhere)));
        let not_all = server.route(Tool::Read, &json!({ "machine": "all", "id": 3 }));
        assert!(matches!(not_all, Err(McpError::UnknownMachine(_))));
        let junk = server.route(Tool::List, &json!({ "machine": 5 }));
        assert!(matches!(junk, Err(McpError::Invalid(_))));
    }

    #[test]
    fn a_machine_must_be_opted_in_by_name_in_any_case() {
        let server = server_with(&[("build", true), ("gpu", false)]);
        for name in ["gpu", "GPU"] {
            let refused = server.route(Tool::List, &json!({ "machine": name }));
            assert_eq!(
                refused.err(),
                Some(McpError::MachineNotEnabled("gpu".into())),
                "{name}"
            );
        }
        assert!(
            server
                .route(Tool::List, &json!({ "machine": "BUILD" }))
                .is_ok()
        );
    }

    #[test]
    fn the_grant_probe_passes_only_when_this_computer_s_daemon_does() {
        let reach = |read: &'static str, write: &'static str| {
            Reach::new("ssh", move |msg| match msg {
                ClientMsg::Query(Query::Read { .. }) => Err(read.to_string()),
                ClientMsg::Command(Command::ClearQueue { .. }) if write.is_empty() => Ok(Reply::Ok),
                _ => Err(write.to_string()),
            })
        };
        let absent = format!("no pane {PROBE_TERM}");
        let leak: &'static str = Box::leak(absent.into_boxed_str());
        let denied = "this pane may not read other panes through maitty";
        assert!(reach(leak, "").caller_may(Needs::Read).is_ok());
        assert!(reach(leak, "").caller_may(Needs::Write).is_ok());
        assert_eq!(
            reach(denied, "").caller_may(Needs::Read),
            Err(denied.into())
        );
        assert_eq!(
            reach(leak, "no write").caller_may(Needs::Write),
            Err("no write".into())
        );
        let down = "couldn't reach the maitty server";
        assert!(
            reach(down, "").caller_may(Needs::Read).is_err(),
            "no answer is no"
        );
    }
}
