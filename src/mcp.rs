//! `<app> mcp`: the app as an MCP server (stdio), so an agent can see and steer the others —
//! list them, read their screens, message them, answer their prompts (only as allowed), start
//! new ones in their own worktrees, interrupt them. There is no merge, push or delete.
//!
//! Guardrails come from `[mcp]` in config: `approve` (never / safe / always) decides whether
//! prompts may be answered "yes"; `scope` (project / all) limits it to the caller's own repo.
//! Starting a session is starting a helper: `[sessions] helpers` (ask / always / never) and
//! `helpers_at_once` decide whether it may, and the daemon asks the user on the caller's row
//! the first time (see `daemon/helpers.rs`). A refused start comes back as an error to read.

use crate::cli;
use crate::brand::{APP_COMMAND};
use crate::protocol::*;
use anyhow::Result;
use serde_json::{Value, json};
use std::io::{BufRead, Write};
use std::path::{Path, PathBuf};

fn norm(p: &Path) -> String {
    p.to_string_lossy().replace('/', "\\").trim_end_matches('\\').to_lowercase()
}

/// The repo (or folder) the calling agent works in.
pub(crate) fn caller_root() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    crate::gitfs::head(&cwd).map(|h| h.main_root).unwrap_or(cwd)
}

fn row_of(t: &TermInfo, me: Option<TermId>) -> Value {
    let mut row = json!({
        "id": t.id,
        "agent": t.agent.clone().unwrap_or_else(|| if t.is_shell() { "shell".into() } else { t.display_name() }),
        "status": match t.status { Status::Blocked => "needs-you", s => s.label() },
        "branch": t.branch,
        "folder": t.top.clone().unwrap_or_else(|| t.cwd.clone()),
        "on": t.summary,
        "asleep": t.asleep,
    });
    if let Some(at) = t.quiet_since.filter(|_| t.status == Status::Working) {
        row["quiet_since"] = json!(at);
    }
    if Some(t.id) == me {
        row["you"] = json!(true);
    }
    if let Some(note) = cli::checks_note(t) {
        row["check"] = json!(note);
    }
    row
}

fn root_of(t: &TermInfo) -> PathBuf {
    t.root.clone().unwrap_or_else(|| t.cwd.clone())
}

struct Server {
    cfg: crate::config::Config,
    root: PathBuf,
    me: Option<TermId>,
}

fn text(s: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": s.into() }] })
}

fn fail(s: impl Into<String>) -> Value {
    json!({ "content": [{ "type": "text", "text": s.into() }], "isError": true })
}

fn tools() -> Value {
    let id = json!({ "type": "integer", "description": "The session id (from the session list)." });
    json!([
        {
            "name": concat!(env!("APP_COMMAND"), "_list"),
            "description": "List sessions in the caller's project, with their status and current task.",
            "inputSchema": { "type": "object", "properties": {} }
        },
        {
            "name": concat!(env!("APP_COMMAND"), "_read"),
            "description": "Read the end of a session's screen (what the agent shows right now).",
            "inputSchema": { "type": "object", "properties": { "id": id, "lines": { "type": "integer", "description": "How many lines from the bottom (default 40)." } }, "required": ["id"] }
        },
        {
            "name": concat!(env!("APP_COMMAND"), "_send"),
            "description": "Type a message into a session's prompt and press Enter, like a person would. With wait, returns once it has finished that turn, with its reply.",
            "inputSchema": { "type": "object", "properties": {
                "id": id,
                "text": { "type": "string" },
                "wait": { "type": "boolean", "description": "Wait for its turn to end and return its reply (default false)." },
                "timeout": { "type": "integer", "description": "Seconds to wait at most (default 600)." }
            }, "required": ["id", "text"] }
        },
        {
            "name": concat!(env!("APP_COMMAND"), "_wait"),
            "description": "Wait until a session finishes its turn (or needs an answer), or until text matching a regex shows on its screen. Returns its reply or the matching line.",
            "inputSchema": { "type": "object", "properties": {
                "id": id,
                "regex": { "type": "string", "description": "Wait for this on the screen instead of the turn ending." },
                "timeout": { "type": "integer", "description": "Seconds to wait at most (default 600)." }
            }, "required": ["id"] }
        },
        {
            "name": concat!(env!("APP_COMMAND"), "_answer"),
            "description": concat!("Answer a session's numbered prompt (e.g. 1 = Yes, 3 = No). Pass the prompt id ", env!("APP_COMMAND"), "_list or ", env!("APP_COMMAND"), "_wait gave you; if the prompt on screen has changed since, nothing is sent and you get the current one. Saying yes is only allowed if the user has turned that on in ", env!("APP_COMMAND"), "'s settings; otherwise ask the user."),
            "inputSchema": { "type": "object", "properties": {
                "id": id,
                "choice": { "type": "integer", "minimum": 1, "maximum": 9 },
                "prompt": { "type": "string", "description": concat!("The prompt id from ", env!("APP_COMMAND"), "_list or ", env!("APP_COMMAND"), "_wait.") }
            }, "required": ["id", "choice", "prompt"] }
        },
        {
            "name": concat!(env!("APP_COMMAND"), "_start"),
            "description": "Start a helper: a new agent on a task. In a git repo it gets its own worktree and branch, so it never edits the same files as anyone else. Returns its session id. The first time, the user is asked to allow helpers; the error says so. Call it for every helper you want, then try again once they have answered. If the user said no, do the work yourself.",
            "inputSchema": { "type": "object", "properties": {
                "prompt": { "type": "string", "description": "The task." },
                "agent": { "type": "string", "description": "claude, codex, gemini, … (default: the user's first agent)." },
                "worktree": { "type": "boolean", "description": "Own worktree (default true)." }
            }, "required": ["prompt"] }
        },
        {
            "name": concat!(env!("APP_COMMAND"), "_references"),
            "description": concat!("List the project's reference images and design files (screenshots, mockups, exports) the user keeps for UI and design work: each file's absolute path, kind, size and when it changed. Open the images it lists before building or changing a screen. The files sit in .", env!("APP_COMMAND"), "/references/ at the repo's main checkout and are plain files any agent can read."),
            "inputSchema": { "type": "object", "properties": {
                "path": { "type": "string", "description": "A folder in the project, if not the one you run in." }
            } }
        },
        {
            "name": concat!(env!("APP_COMMAND"), "_interrupt"),
            "description": "Interrupt what a session is doing (Ctrl+C).",
            "inputSchema": { "type": "object", "properties": { "id": id }, "required": ["id"] }
        }
    ])
}

/// The numbered choices on a screen and the question above them.
fn prompt_on(screen: &str) -> (String, Vec<(u32, String)>) {
    let lines: Vec<&str> = screen.lines().collect();
    let mut opts = Vec::new();
    let mut first = lines.len();
    for (i, l) in lines.iter().enumerate().rev().take(20) {
        let t = l.trim().trim_matches(|c: char| "│┃ ".contains(c)).trim_start_matches(['❯', '>', ' ']).trim();
        let mut ch = t.chars();
        if let (Some(d), Some('.')) = (ch.next(), ch.next())
            && let Some(n) = d.to_digit(10)
        {
            opts.push((n, ch.as_str().trim().to_string()));
            first = i;
        }
    }
    opts.reverse();
    let question = lines[first.saturating_sub(6)..first].join(" ");
    (question, opts)
}

/// A short id for the prompt on a screen: its question and choices, whitespace aside. An
/// answer names the id it saw, so a prompt that changed meanwhile is never answered.
fn prompt_id(question: &str, opts: &[(u32, String)]) -> String {
    use std::hash::{Hash, Hasher};
    let mut h = std::hash::DefaultHasher::new();
    question.split_whitespace().collect::<Vec<_>>().hash(&mut h);
    opts.hash(&mut h);
    format!("{:08x}", h.finish() as u32)
}

/// The prompt on `screen` if it is the one with id `seen`; else what it reads now.
fn prompt_seen(screen: &str, seen: &str) -> Result<(String, Vec<(u32, String)>), String> {
    let (question, opts) = prompt_on(screen);
    if prompt_id(&question, &opts) == seen { Ok((question, opts)) } else { Err(prompt_lines(&question, &opts)) }
}

/// The prompt's lines for a reply: the question, its choices and its id.
fn prompt_lines(question: &str, opts: &[(u32, String)]) -> String {
    let choices: Vec<String> = opts.iter().map(|(n, l)| format!("{n}. {l}")).collect();
    format!("{}\n{}\nprompt {}", question.trim(), choices.join("\n"), prompt_id(question, opts))
}

impl Server {
    fn snapshot(&self) -> Result<Snapshot, String> {
        match cli::block_on(cli::request(ClientMsg::Query(Query::List))) {
            Ok(Reply::List(s)) => Ok(s),
            Ok(_) => Err("unexpected reply".into()),
            Err(e) => Err(format!("{e:#}")),
        }
    }

    fn in_scope(&self, t: &TermInfo) -> bool {
        self.cfg.mcp.scope == "all" || norm(&root_of(t)) == norm(&self.root)
    }

    /// A session the caller may touch.
    fn session(&self, args: &Value) -> Result<TermInfo, String> {
        let id = args.get("id").and_then(Value::as_u64).ok_or("give the session id")? as TermId;
        let snap = self.snapshot()?;
        let t = snap.terms.get(&id).cloned().ok_or_else(|| format!("no session {id}"))?;
        if !self.in_scope(&t) {
            return Err(format!("session {id} is in another project; {APP_COMMAND} only lets you reach sessions in {}", self.root.display()));
        }
        Ok(t)
    }

    fn screen(&self, id: TermId) -> Result<String, String> {
        match cli::block_on(cli::request(ClientMsg::Query(Query::Read { term: id }))) {
            Ok(Reply::Text(t)) => Ok(t),
            Ok(_) => Err("unexpected reply".into()),
            Err(e) => Err(format!("{e:#}")),
        }
    }

    fn call(&self, name: &str, args: &Value) -> Value {
        let r = match name {
            concat!(env!("APP_COMMAND"), "_list") => self.list(),
            concat!(env!("APP_COMMAND"), "_read") => self.read(args),
            concat!(env!("APP_COMMAND"), "_send") => self.send(args),
            concat!(env!("APP_COMMAND"), "_wait") => self.session(args).and_then(|t| {
                let regex = args.get("regex").and_then(Value::as_str).map(str::to_string);
                self.wait(t.id, regex.as_deref(), args, false)
            }),
            concat!(env!("APP_COMMAND"), "_answer") => self.answer(args),
            concat!(env!("APP_COMMAND"), "_start") => self.start(args),
            concat!(env!("APP_COMMAND"), "_references") => self.references(args),
            concat!(env!("APP_COMMAND"), "_interrupt") => self.session(args).and_then(|t| cli::send(Some(t.id), "\x03".into(), false).map(|_| format!("interrupted {}", t.id)).map_err(|e| format!("{e:#}"))),
            other => Err(format!("no tool {other}")),
        };
        match r {
            Ok(s) => text(s),
            Err(e) => fail(e),
        }
    }

    fn list(&self) -> Result<String, String> {
        let snap = self.snapshot()?;
        let mut out = Vec::new();
        for t in snap.terms.values().filter(|t| self.in_scope(t)) {
            let mut row = row_of(t, self.me);
            if t.status == Status::Blocked
                && let Ok(screen) = self.screen(t.id)
            {
                let (q, opts) = prompt_on(&screen);
                row["question"] = json!(q.trim());
                row["choices"] = json!(opts.iter().map(|(n, l)| format!("{n}. {l}")).collect::<Vec<_>>());
                row["prompt"] = json!(prompt_id(&q, &opts));
            }
            out.push(row);
        }
        Ok(serde_json::to_string_pretty(&out).unwrap_or_default())
    }

    fn read(&self, args: &Value) -> Result<String, String> {
        let t = self.session(args)?;
        let n = args.get("lines").and_then(Value::as_u64).unwrap_or(40) as usize;
        let screen = self.screen(t.id)?;
        let lines: Vec<&str> = screen.lines().collect();
        Ok(lines[lines.len().saturating_sub(n)..].join("\n").trim_end().to_string())
    }

    fn send(&self, args: &Value) -> Result<String, String> {
        let t = self.session(args)?;
        if Some(t.id) == self.me {
            return Err("that's you".into());
        }
        let msg = args.get("text").and_then(Value::as_str).ok_or("give the text")?;
        // Text sent to a session that's waiting on a question would answer it; that goes
        // through the app_answer and the user's approval rules instead.
        if t.status == Status::Blocked && self.cfg.mcp.approve != "always" {
            return Err(format!(concat!("session {} is waiting on a question; use ", env!("APP_COMMAND"), "_answer (the user's approval rules apply) or ask the user"), t.id));
        }
        cli::send(Some(t.id), msg.to_string(), true).map_err(|e| format!("{e:#}"))?;
        if args.get("wait").and_then(Value::as_bool) == Some(true) {
            return self.wait(t.id, None, args, true);
        }
        Ok(format!("sent to {} ({})", t.id, t.agent.unwrap_or_default()))
    }

    fn wait(&self, term: TermId, regex: Option<&str>, args: &Value, just_sent: bool) -> Result<String, String> {
        if Some(term) == self.me {
            return Err("that's you".into());
        }
        let secs = args.get("timeout").and_then(Value::as_u64).unwrap_or(600);
        match cli::wait_on(term, regex, std::time::Duration::from_secs(secs), just_sent).map_err(|e| format!("{e:#}"))? {
            cli::Waited::Matched(l) => Ok(format!("matched: {l}")),
            cli::Waited::TimedOut { quiet: Some(at) } => Err(format!("still working, but quiet for {} (waited {secs}s)", cli::quiet_for(at))),
            cli::Waited::TimedOut { quiet: None } => Err(format!("still going after {secs}s")),
            cli::Waited::Turn(s, said, check) => {
                let reply = if said.trim().is_empty() {
                    let screen = self.screen(term)?;
                    let lines: Vec<&str> = screen.trim_end().lines().collect();
                    lines[lines.len().saturating_sub(30)..].join("\n")
                } else {
                    said
                };
                let state = match s {
                    Status::Blocked => concat!("needs an answer (see ", env!("APP_COMMAND"), "_read / ", env!("APP_COMMAND"), "_answer)"),
                    _ => "finished",
                };
                let checked = check.map(|c| format!("\n{}", cli::check_line(&c))).unwrap_or_default();
                let prompt = if s == Status::Blocked {
                    let (q, opts) = prompt_on(&self.screen(term)?);
                    format!("\nprompt {}", prompt_id(&q, &opts))
                } else {
                    String::new()
                };
                Ok(format!("{term} {state}:\n{reply}{checked}{prompt}"))
            }
        }
    }

    fn answer(&self, args: &Value) -> Result<String, String> {
        let t = self.session(args)?;
        let choice = args.get("choice").and_then(Value::as_u64).filter(|c| (1..=9).contains(c)).ok_or("choice must be 1-9")? as u32;
        let seen = args.get("prompt").and_then(Value::as_str).ok_or(concat!("call ", env!("APP_COMMAND"), "_list first and pass its prompt id"))?;
        let (question, opts) = prompt_seen(&self.screen(t.id)?, seen).map_err(|now| format!("the prompt on session {} has changed since you read it; nothing was sent. It now reads:\n{now}", t.id))?;
        let label = opts.iter().find(|(n, _)| *n == choice).map(|(_, l)| l.clone()).unwrap_or_default();
        let saying_no = label.to_lowercase().starts_with("no");
        if !saying_no {
            match self.cfg.mcp.approve.as_str() {
                "always" => {}
                "safe" => {
                    let q = question.to_lowercase();
                    if !self.cfg.mcp.safe.iter().any(|s| !s.is_empty() && q.contains(&s.to_lowercase())) {
                        return Err(format!(concat!("\"{}\" isn't on the user's safe list, so ", env!("APP_COMMAND"), " won't approve it. Ask the user."), question.trim()));
                    }
                }
                _ => return Err(concat!("The user hasn't let agents approve prompts (", env!("APP_COMMAND"), " Settings → Agents). Ask the user to answer it.").into()),
            }
        }
        cli::send(Some(t.id), choice.to_string(), false).map_err(|e| format!("{e:#}"))?;
        Ok(format!("answered {choice}{} on session {}", if label.is_empty() { String::new() } else { format!(" ({label})") }, t.id))
    }

    /// The references of the caller's project, or of the project `path` is in (within scope).
    fn references(&self, args: &Value) -> Result<String, String> {
        let root = match args.get("path") {
            None | Some(Value::Null) => self.root.clone(),
            Some(Value::String(p)) => self.project_at(p)?,
            Some(_) => return Err("path must be a string".into()),
        };
        let folder = crate::references::folder(&root);
        let found = crate::references::of_project(&root)?;
        if found.is_empty() {
            return Ok(format!("No references in {} yet. The user adds them from the References view in {APP_COMMAND} (paste a screenshot or add a file).", folder.display()));
        }
        Ok(serde_json::to_string_pretty(&references_json(&folder, &found)).unwrap_or_default())
    }

    /// The project root of a folder an agent named: an existing folder, and the caller's own
    /// project unless `[mcp] scope` is `all`.
    fn project_at(&self, raw: &str) -> Result<PathBuf, String> {
        if raw.trim().is_empty() || raw.contains('\0') {
            return Err("give a folder path".into());
        }
        let p = PathBuf::from(raw.trim());
        let p = if p.is_absolute() { p } else { std::env::current_dir().unwrap_or_default().join(p) };
        let dir = p.canonicalize().map_err(|e| format!("no folder {raw}: {e}"))?;
        if !dir.is_dir() {
            return Err(format!("{raw} is not a folder"));
        }
        let mine = self.root.canonicalize().unwrap_or_else(|_| self.root.clone());
        // A folder inside the caller's own project, git or not, is that project.
        if dir.starts_with(&mine) {
            return Ok(self.root.clone());
        }
        let root = crate::references::project_root(&dir);
        let theirs = root.canonicalize().unwrap_or_else(|_| root.clone());
        if self.cfg.mcp.scope != "all" && norm(&theirs) != norm(&mine) {
            return Err(format!("{raw} is in another project; {APP_COMMAND} only lists references in {}", self.root.display()));
        }
        Ok(root)
    }

    fn start(&self, args: &Value) -> Result<String, String> {
        let prompt = args.get("prompt").and_then(Value::as_str).filter(|p| !p.trim().is_empty()).ok_or("give the task")?;
        crate::hidden::check(prompt)?;
        let (agent, cmd) = start_command(&self.cfg, args, prompt)?;
        let before = self.snapshot()?;
        let had: Vec<TermId> = before.terms.keys().copied().collect();
        let focus = before.active().and_then(|w| w.tab()).map(|t| t.focus);
        let worktree = args.get("worktree").and_then(Value::as_bool).unwrap_or(true) && crate::gitfs::head(&self.root).is_some();
        let branch = slug(prompt);
        let c = if worktree {
            let ws = before.active_ws.or_else(|| before.workspaces.first().map(|w| w.id)).ok_or(concat!(env!("APP_COMMAND"), " has nothing open"))?;
            let branch = if before.terms.values().any(|t| t.branch.as_deref() == Some(branch.as_str())) { format!("{branch}-{}", had.len()) } else { branch.clone() };
            Command::NewWorktree { ws, branch, base: None, cmd: Some(cmd), split: None, from: Some(self.root.clone()) }
        } else {
            Command::NewWorkspace { cwd: Some(self.root.clone()), name: None, cmd: Some(cmd) }
        };
        cli::block_on(cli::request(ClientMsg::Command(c))).map_err(|e| format!("{e:#}"))?;
        // Wait for it to appear, then give the user back the session they were on.
        for _ in 0..60 {
            std::thread::sleep(std::time::Duration::from_millis(250));
            let Ok(now) = self.snapshot() else { continue };
            if let Some(new) = cli::handoff::new_pane(&had, now.terms.keys().copied()) {
                if let Some(f) = focus.filter(|f| now.terms.contains_key(f)) {
                    let _ = cli::block_on(cli::request(ClientMsg::Command(Command::FocusPane { term: f })));
                }
                let place = if worktree { format!("its own worktree ({branch})") } else { self.root.display().to_string() };
                return Ok(format!("started {agent} as session {new} in {place}"));
            }
        }
        Ok(format!("asked {APP_COMMAND} to start {agent}; it hasn't appeared yet"))
    }
}

fn references_json(folder: &Path, found: &[crate::references::Reference]) -> Value {
    let rows: Vec<Value> = found
        .iter()
        .map(|r| json!({ "path": r.path, "name": r.name, "kind": r.kind.label(), "bytes": r.size, "modified": crate::references::utc_iso(r.modified) }))
        .collect();
    json!({ "folder": folder, "references": rows })
}

/// The agent `<app>_start` runs and its command line: the named `agent`, or the user's first.
fn start_command(cfg: &crate::config::Config, args: &Value, prompt: &str) -> Result<(String, String), String> {
    let agent = args.get("agent").and_then(Value::as_str).map_or_else(|| cfg.default_agent(), String::from);
    let cmd = cfg.agent_command(Some(&agent), None, prompt)?;
    Ok((agent, cmd))
}

fn slug(s: &str) -> String {
    let words: Vec<String> = s.split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).take(4).map(|w| w.to_ascii_lowercase()).collect();
    let s = words.join("-");
    let s: String = s.chars().take(40).collect();
    if s.is_empty() { "agent-task".into() } else { s.trim_end_matches('-').to_string() }
}

/// Run the server on stdin / stdout until stdin closes.
pub fn run() -> Result<()> {
    let (cfg, _) = crate::config::Config::load_or_default();
    let server = Server { cfg, root: caller_root(), me: std::env::var(concat!(env!("APP_ENV"), "_TERM_ID")).ok().and_then(|s| s.parse().ok()) };
    let stdin = std::io::stdin();
    let mut out = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        let Ok(msg) = serde_json::from_str::<Value>(&line) else { continue };
        let Some(id) = msg.get("id").cloned() else { continue }; // notifications need no reply
        let method = msg.get("method").and_then(Value::as_str).unwrap_or("");
        let params = msg.get("params").cloned().unwrap_or(Value::Null);
        let reply = match method {
            "initialize" => Ok(json!({
                "protocolVersion": params.get("protocolVersion").cloned().unwrap_or(json!("2025-06-18")),
                "capabilities": { "tools": {} },
                "serverInfo": { "name": env!("APP_COMMAND"), "version": env!("CARGO_PKG_VERSION") },
                "instructions": concat!(env!("APP_COMMAND"), " runs your user's coding agents side by side. Use ", env!("APP_COMMAND"), "_list/read/send/start for helpers. Before UI, front-end or design work, call ", env!("APP_COMMAND"), "_references and open the images it lists: they are the user's reference screenshots and designs for this project. Don't answer prompts unless the user allows it; when you do, pass ", env!("APP_COMMAND"), "_answer the prompt id ", env!("APP_COMMAND"), "_list gave you.")
            })),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(json!({ "tools": tools() })),
            "tools/call" => {
                let name = params.get("name").and_then(Value::as_str).unwrap_or("");
                let args = params.get("arguments").cloned().unwrap_or(json!({}));
                Ok(server.call(name, &args))
            }
            _ => Err(json!({ "code": -32601, "message": format!("no method {method}") })),
        };
        let resp = match reply {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err(error) => json!({ "jsonrpc": "2.0", "id": id, "error": error }),
        };
        writeln!(out, "{resp}")?;
        out.flush()?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_a_prompt() {
        let screen = "• Read test/checkout.spec.ts\n\nRun npm test -- checkout?\n❯ 1. Yes\n  2. Yes, and always allow npm test\n  3. No, tell it what to do instead\n";
        let (q, opts) = prompt_on(screen);
        assert!(q.contains("Run npm test -- checkout?"));
        assert_eq!(opts.len(), 3);
        assert_eq!(opts[2], (3, "No, tell it what to do instead".into()));
        assert_eq!(slug("Fix the flaky checkout test, please"), "fix-the-flaky-checkout");
    }

    #[test]
    fn the_list_says_what_the_check_found() {
        let mut t: TermInfo = serde_json::from_value(json!({
            "id": 4, "cols": 80, "rows": 24, "title": "", "process": "node", "agent": "claude",
            "status": "Done", "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
        }))
        .unwrap();
        let plain = row_of(&t, None);
        assert!(plain.get("check").is_none(), "no check, no key");
        t.checks = vec![CheckInfo { command: "npm test".into(), state: CheckState::Failed, exit_code: Some(1), ..Default::default() }];
        assert_eq!(row_of(&t, None)["check"], "check failed (exit 1)");
        assert_eq!(row_of(&t, None)["status"], "done");
        t.status = Status::Working;
        assert!(row_of(&t, None).get("check").is_none(), "an older turn's result is not reported");
        t.status = Status::Done;
        let stale = CheckInfo { name: "test".into(), state: CheckState::Stale, changed_files: vec![crate::protocol::ChangedFile { kind: 'M', path: "a".into() }], ..Default::default() };
        t.checks = vec![CheckInfo { name: "build".into(), state: CheckState::Passed, ..Default::default() }, stale];
        assert_eq!(row_of(&t, None)["check"], "stale pass: test · 1 file changed · 2 checkpoints");
    }

    const RM_PROMPT: &str = "Bash command\n  rm -rf build\nDo you want to proceed?\n❯ 1. Yes\n  2. No";
    const LS_PROMPT: &str = "Bash command\n  ls\nDo you want to proceed?\n❯ 1. Yes\n  2. No";

    #[test]
    fn a_helper_task_with_hidden_characters_is_never_started() {
        let server = Server { cfg: crate::config::Config::default(), root: std::env::temp_dir(), me: None };
        let err = server.start(&json!({ "prompt": "fix the bug\u{e0049}\u{e0067}" })).unwrap_err();
        assert!(err.starts_with("refused: hidden characters \\u{e0049} at 11"), "{err}");
    }

    #[test]
    fn an_answer_goes_only_to_the_prompt_it_saw() {
        let (q, opts) = prompt_on(LS_PROMPT);
        let seen = prompt_id(&q, &opts);
        assert!(prompt_seen(LS_PROMPT, &seen).is_ok(), "the same prompt is answered");
        assert!(prompt_seen(&LS_PROMPT.replace("  ls", "ls    "), &seen).is_ok(), "spacing alone is the same prompt");
        let now = prompt_seen(RM_PROMPT, &seen).expect_err("a different command is a different prompt");
        let (rq, ropts) = prompt_on(RM_PROMPT);
        assert!(now.contains("rm -rf build") && now.contains("1. Yes") && now.ends_with(&format!("prompt {}", prompt_id(&rq, &ropts))), "{now}");
        assert!(prompt_seen("$ ", &seen).is_err(), "no prompt left on screen");
    }

    #[test]
    fn app_answer_requires_the_prompt_id() {
        let tools = tools();
        let answer = tools.as_array().unwrap().iter().find(|t| t["name"] == concat!(env!("APP_COMMAND"), "_answer")).unwrap();
        assert_eq!(answer["inputSchema"]["required"], json!(["id", "choice", "prompt"]));
    }

    #[test]
    fn the_list_gives_a_quiet_working_agent_its_quiet_since() {
        let mut t: TermInfo = serde_json::from_value(json!({
            "id": 4, "cols": 80, "rows": 24, "title": "", "process": "node", "agent": "claude",
            "status": "Working", "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
        }))
        .unwrap();
        assert!(row_of(&t, None).get("quiet_since").is_none(), "not quiet, no key");
        t.quiet_since = Some(1_000);
        assert_eq!(row_of(&t, None)["quiet_since"], 1_000);
    }

    /// Two plain folders side by side: `mine` (with one reference) and `other`.
    fn two_projects(tag: &str) -> (PathBuf, PathBuf, PathBuf) {
        let base = std::env::temp_dir().join(format!("{APP_COMMAND}-mcp-refs-{tag}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&base);
        let (mine, other) = (base.join("mine"), base.join("other"));
        std::fs::create_dir_all(crate::references::folder(&mine)).unwrap();
        std::fs::create_dir_all(crate::references::folder(&other)).unwrap();
        std::fs::write(crate::references::folder(&mine).join("home.png"), "png").unwrap();
        std::fs::write(crate::references::folder(&other).join("theirs.png"), "png").unwrap();
        (base, mine, other)
    }

    fn refs_server(root: &Path, scope: &str) -> Server {
        let mut cfg = crate::config::Config::default();
        cfg.mcp.scope = scope.into();
        Server { cfg, root: root.to_path_buf(), me: None }
    }

    #[test]
    fn references_list_the_callers_project() {
        let (base, mine, _) = two_projects("mine");
        // Skip where the temp folder sits inside a repo: then both are one project.
        if crate::gitfs::head(&mine).is_none() {
            let out = refs_server(&mine, "project").references(&json!({})).unwrap();
            let v: Value = serde_json::from_str(&out).unwrap();
            let rows = v["references"].as_array().unwrap();
            assert_eq!(rows.len(), 1);
            assert_eq!(rows[0]["name"], "home.png");
            assert_eq!(rows[0]["kind"], "image");
            assert!(Path::new(rows[0]["path"].as_str().unwrap()).is_absolute(), "an absolute path to open");
            let empty = refs_server(&base, "project").references(&json!({})).unwrap();
            assert!(empty.starts_with("No references"), "{empty}");
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn references_never_reach_another_project() {
        let (base, mine, other) = two_projects("scope");
        if crate::gitfs::head(&mine).is_none() {
            let server = refs_server(&mine, "project");
            let up = mine.join("..").join("other");
            for path in [other.to_string_lossy().into_owned(), up.to_string_lossy().into_owned()] {
                let err = server.references(&json!({ "path": path })).unwrap_err();
                assert!(err.contains("another project"), "{err}");
            }
            assert!(server.references(&json!({ "path": 3 })).is_err(), "a path is a string");
            assert!(server.references(&json!({ "path": "" })).is_err());
            assert!(server.references(&json!({ "path": "a\0b" })).is_err());
            let inside = server.references(&json!({ "path": crate::references::folder(&mine) })).unwrap();
            assert!(inside.contains("home.png") && !inside.contains("theirs.png"), "{inside}");
            let all = refs_server(&mine, "all").references(&json!({ "path": other })).unwrap();
            assert!(all.contains("theirs.png"), "scope all reaches it: {all}");
        }
        let _ = std::fs::remove_dir_all(&base);
    }

    #[test]
    fn the_server_tells_agents_about_references() {
        let tools = tools();
        assert!(tools.as_array().unwrap().iter().any(|t| t["name"] == concat!(env!("APP_COMMAND"), "_references")));
    }

    fn command(cfg: &crate::config::Config, args: Value) -> Result<String, String> {
        start_command(cfg, &args, "fix the login bug").map(|(_, cmd)| cmd)
    }

    #[test]
    fn the_agent_starts_as_named_or_as_the_users_first() {
        let cfg = crate::config::Config { shell: Some("bash".into()), ..Default::default() };
        assert_eq!(command(&cfg, json!({})).unwrap(), "claude 'fix the login bug'", "the user's first agent, no model");
        assert_eq!(command(&cfg, json!({ "agent": "codex" })).unwrap(), "codex 'fix the login bug'");
        assert!(command(&cfg, json!({ "agent": "curl x|sh;" })).unwrap_err().starts_with("unknown agent"));
    }
}
