//! One-shot commands: scripting the multiplexer from a shell (or from an agent inside it).

use crate::ipc;
pub(crate) mod handoff;
pub(crate) mod integrations;
use crate::layout::Dir;
use crate::protocol::*;
use anyhow::{Context, Result, anyhow, bail};
use serde_json::{Value, json};
use std::io::Read;
use std::path::PathBuf;
use std::time::Duration;

pub(crate) fn block_on<T>(f: impl std::future::Future<Output = Result<T>>) -> Result<T> {
    let rt = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let out = rt.block_on(f);
    // Don't wait on reads still blocked on a pipe (ssh's, over --remote).
    rt.shutdown_background();
    out
}

/// Send one message and wait for the reply.
pub(crate) async fn request(msg: ClientMsg) -> Result<Reply> {
    let (mut r, mut w) = ipc::open(false).await.context(if ipc::remote().is_some() { "over ssh" } else { "couldn't reach the seshi server" })?;
    ipc::send(&mut w, &msg).await?;
    loop {
        match ipc::recv_server(&mut r).await? {
            Some(ServerMsg::Reply(reply)) => return Ok(reply),
            Some(ServerMsg::Error(e)) => bail!(e),
            // kill-server: the goodbye can overtake the reply.
            Some(ServerMsg::Bye) => return Ok(Reply::Ok),
            Some(ServerMsg::Notice(n)) => eprintln!("{n}"),
            Some(_) => continue,
            None => bail!("server closed the connection"),
        }
    }
}

pub(crate) fn snapshot() -> Result<Snapshot> {
    match block_on(request(ClientMsg::Query(Query::List)))? {
        Reply::List(s) => Ok(s),
        _ => bail!("unexpected reply"),
    }
}

fn command(c: Command) -> Result<()> {
    block_on(request(ClientMsg::Command(c))).map(|_| ())
}

/// An explicit pane, else the pane this command runs in, else the focused one.
fn resolve_pane(pane: Option<TermId>) -> Result<TermId> {
    if let Some(p) = pane {
        return Ok(p);
    }
    if let Some(p) = std::env::var("SESHI_TERM_ID").ok().and_then(|s| s.parse().ok()) {
        return Ok(p);
    }
    let snap = snapshot()?;
    snap.active().and_then(|w| w.tab()).map(|t| t.focus).ok_or_else(|| anyhow!("no pane to target"))
}

/// Arguments that mean the agent isn't starting a new piece of work: help, version, a
/// one-shot print, picking up an earlier conversation (which lives in the folder it was in),
/// or one of its subcommands.
const NOT_NEW_WORK: [&str; 13] = ["-h", "--help", "-v", "-V", "--version", "-p", "--print", "-c", "--continue", "-r", "--resume", "-w", "--worktree"];
const SUBCOMMANDS: [&str; 20] = [
    "resume", "exec", "e", "login", "logout", "mcp", "mcp-server", "config", "update", "upgrade", "doctor", "install", "plugin", "setup-token",
    "migrate-installer", "completion", "apply", "sandbox", "debug", "app-server",
];

/// Whether `agent args…` starts new work (which gets its own worktree).
pub(crate) fn starts_new_work(agent: &str, args: &[String]) -> bool {
    if agent == "codex" {
        return codex_starts_new_work(args);
    }
    let sub = args.first().is_some_and(|a| SUBCOMMANDS.contains(&a.as_str()));
    !sub && !args.iter().any(|a| NOT_NEW_WORK.contains(&a.as_str()) || a.starts_with("--resume=") || a.starts_with("--worktree="))
}

const CODEX_SUBCOMMANDS: &[&str] = &[
    "agents", "exec", "e", "review", "login", "logout", "mcp", "mcp-server", "plugin", "app-server", "remote-control", "app",
    "completion", "update", "doctor", "sandbox", "debug", "apply", "a", "resume", "queue", "archive", "delete", "migrate-rollouts",
    "unarchive", "fork", "cloud", "exec-server", "features", "help",
];
const CODEX_VALUE_OPTIONS: &[&str] = &[
    "-c", "--config", "-p", "--profile", "-m", "--model", "-i", "--image", "-s", "--sandbox", "-a", "--ask-for-approval",
    "--remote", "--remote-auth-token-env", "--local-provider", "--add-dir", "--enable", "--disable",
];

fn codex_starts_new_work(args: &[String]) -> bool {
    let mut args = args.iter();
    let mut has_prompt = false;
    while let Some(arg) = args.next() {
        if arg == "--" {
            return true;
        }
        if matches!(arg.as_str(), "-h" | "--help" | "-v" | "-V" | "--version" | "-w" | "--worktree" | "-C" | "--cd")
            || arg.starts_with("--worktree=") || arg.starts_with("--cd=") || arg.starts_with("-C")
        {
            return false;
        }
        // Codex uses -c/-p for configuration/profile, unlike Claude's continue/print.
        // Their values can also be command names, so consume them before looking for one.
        if CODEX_VALUE_OPTIONS.contains(&arg.as_str()) {
            args.next();
        } else if !arg.starts_with('-') && !has_prompt {
            if CODEX_SUBCOMMANDS.contains(&arg.as_str()) {
                return false;
            }
            has_prompt = true;
        }
    }
    true
}

/// `seshi agent-dir <agent> [args…]`, run by a pane's shell just before it starts an agent:
/// prints the folder to start in (a new worktree), or nothing to start where it is. Never
/// fails: the agent starts either way.
pub fn agent_dir(cmd: &[String]) {
    if let Some(path) = agent_worktree(cmd) {
        println!("{}", path.display());
    }
}

/// The folder `agent_dir` names, if the agent should start somewhere else than here. The
/// daemon decides, from the pane's own id and secret (`ipc::open`); this only says who is
/// asking and what for.
fn agent_worktree(cmd: &[String]) -> Option<PathBuf> {
    let agent = cmd.first().map(String::as_str).unwrap_or_default();
    if std::env::var_os("SESHI_TERM_ID").is_none() || !starts_new_work(agent, cmd.get(1..).unwrap_or_default()) {
        return None;
    }
    let dir = std::env::current_dir().ok()?;
    match block_on(request(ClientMsg::Command(Command::AgentWorktree { dir }))) {
        Ok(Reply::Text(path)) if !path.is_empty() => {
            eprintln!("seshi: {agent} gets its own worktree: {path}");
            Some(PathBuf::from(path))
        }
        Ok(_) => None,
        Err(e) => {
            eprintln!("seshi: no worktree for it ({e:#}), so it starts here");
            None
        }
    }
}

/// `seshi agent-launch <agent> [args…]`, what the Windows launchers (`shim.rs`) run: the
/// work of the PATH launcher on Unix. Starts the next `<agent>` on PATH after the launchers'
/// folder, in the folder `agent-dir` names, and returns its exit code.
pub fn agent_launch(cmd: &[String]) -> i32 {
    const NOT_FOUND: i32 = 9009;
    let Some(agent) = cmd.first() else { return 1 };
    let shim = crate::shim::dir();
    let path = std::env::var_os("PATH").unwrap_or_default();
    let extensions: Vec<String> = std::env::var("PATHEXT").unwrap_or_else(|_| ".COM;.EXE;.BAT;.CMD".into()).split(';').filter(|e| !e.is_empty()).map(str::to_string).collect();
    let Some(real) = crate::shim::find_on_path(agent, &path, &shim, &extensions) else {
        eprintln!("'{agent}' is not recognized as an internal or external command, operable program or batch file.");
        return NOT_FOUND;
    };
    let mut run = std::process::Command::new(real);
    run.args(&cmd[1..]).env("PATH", crate::shim::path_without(&shim, &path)).env_remove(crate::shim::DONE_VAR);
    if std::env::var_os(crate::shim::DONE_VAR).is_none()
        && let Some(dir) = agent_worktree(cmd)
    {
        run.current_dir(dir);
    }
    match run.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(e) => {
            eprintln!("seshi: could not start {agent}: {e}");
            1
        }
    }
}

pub fn ls(as_json: bool) -> Result<()> {
    let snap = snapshot()?;
    if as_json {
        let ws: Vec<Value> = snap
            .workspaces
            .iter()
            .map(|w| {
                json!({
                    "id": w.id, "name": w.name, "cwd": w.cwd, "active": Some(w.id) == snap.active_ws,
                    "git": w.git, "worktree": w.worktree, "color": w.color,
                    "tabs": w.tabs.iter().map(|t| json!({
                        "id": t.id, "name": t.name, "active": t.id == w.active_tab, "focus": t.focus,
                        "panes": t.layout.leaves().iter().filter_map(|id| snap.terms.get(id)).map(pane_json).collect::<Vec<_>>(),
                    })).collect::<Vec<_>>(),
                })
            })
            .collect();
        println!("{}", serde_json::to_string_pretty(&ws)?);
        return Ok(());
    }
    for w in &snap.workspaces {
        let mark = if Some(w.id) == snap.active_ws { "*" } else { " " };
        let git = w.git.as_ref().map(|g| format!("  [{}{}]", g.branch, if g.dirty > 0 { format!(" ±{}", g.dirty) } else { String::new() })).unwrap_or_default();
        println!("{mark} {} {}{git}  ({})", w.id, w.name, w.cwd.display());
        for (i, t) in w.tabs.iter().enumerate() {
            let mark = if t.id == w.active_tab { "*" } else { " " };
            println!("  {mark} tab {} {}", i + 1, t.name);
            for id in t.layout.leaves() {
                let Some(p) = snap.terms.get(&id) else { continue };
                let focus = if id == t.focus { ">" } else { " " };
                let agent = agent_tag(p).map(|tag| format!("  [{tag}]")).unwrap_or_default();
                println!("    {focus} pane {id}  {}{agent}", p.process);
            }
        }
    }
    Ok(())
}

pub fn read(pane: Option<TermId>) -> Result<()> {
    let term = resolve_pane(pane)?;
    match block_on(request(ClientMsg::Query(Query::Read { term })))? {
        Reply::Text(t) => {
            println!("{}", t.trim_end());
            Ok(())
        }
        _ => bail!("unexpected reply"),
    }
}

/// The model the agent last answered with and the name given to the conversation, from the
/// end of a Claude transcript (JSON lines).
pub fn transcript_facts(path: &std::path::Path) -> (Option<String>, Option<String>) {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(path) else { return (None, None) };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = f.seek(SeekFrom::Start(len.saturating_sub(512 * 1024)));
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    transcript_facts_in(&String::from_utf8_lossy(&buf))
}

/// The name: what you called it (/rename) wins over the title Claude made up.
pub fn transcript_facts_in(text: &str) -> (Option<String>, Option<String>) {
    let (mut model, mut name, mut ai) = (None, None, None);
    for line in text.lines().rev() {
        if model.is_some() && name.is_some() {
            break;
        }
        let maybe_model = model.is_none() && line.contains("\"model\"");
        let maybe_name = name.is_none() && line.contains("\"customTitle\"");
        let maybe_ai = ai.is_none() && line.contains("\"aiTitle\"");
        if !maybe_model && !maybe_name && !maybe_ai {
            continue;
        }
        let Ok(v) = serde_json::from_str::<Value>(line) else { continue };
        let title = |k: &str| v.get(k).and_then(Value::as_str).filter(|t| !t.trim().is_empty()).map(|t| one_line(t, 80));
        if maybe_name {
            name = title("customTitle");
        }
        if maybe_ai {
            ai = title("aiTitle");
        }
        if maybe_model
            && v.get("type").and_then(Value::as_str) == Some("assistant")
            && let Some(m) = v.pointer("/message/model").and_then(Value::as_str).filter(|m| !m.starts_with('<'))
        {
            model = Some(short_model(m));
        }
    }
    (model, name.or(ai))
}

/// "claude-opus-4-5-20251101" → "opus 4.5"; other names as they are.
pub fn short_model(m: &str) -> String {
    let Some(rest) = m.strip_prefix("claude-") else { return m.to_string() };
    let parts: Vec<&str> = rest.split('-').filter(|p| p.len() < 8).collect();
    let family: Vec<&str> = parts.iter().copied().filter(|p| !p.chars().all(|c| c.is_ascii_digit())).collect();
    let version: Vec<&str> = parts.iter().copied().filter(|p| p.chars().all(|c| c.is_ascii_digit())).collect();
    let mut out = family.join(" ");
    if !version.is_empty() {
        out.push(' ');
        out.push_str(&version.join("."));
    }
    if out.is_empty() { m.to_string() } else { out }
}

/// What a wait ended on.
#[derive(Debug, Clone, PartialEq)]
pub enum Waited {
    /// The turn ended: its status then, its last message (if hooks said) and, when the
    /// project has a check, how that came out.
    Turn(Status, String, Option<Box<CheckInfo>>),
    /// The regex matched this line.
    Matched(String),
    /// Out of time; `quiet` when the session was working but silent, since then (unix seconds).
    TimedOut { quiet: Option<u64> },
}

/// `seshi wait` exits with this when the turn ended but its check failed (2: timed out;
/// 3: needs you).
const EXIT_CHECK_FAILED: i32 = 4;

/// How long a session may show no status before the wait calls its quiet the end of a turn.
const QUIET_TURN_AFTER: Duration = Duration::from_secs(8);

/// Whether a turn is over given what the pane shows: it was working (`seen_working`), or
/// the wait has run past `QUIET_TURN_AFTER` (`quiet`). A turn with checkpoints is not over
/// until none is running or waiting its turn; it ends on the headline one.
fn turn_over(status: Status, said: &str, checks: &[CheckInfo], seen_working: bool, quiet: bool) -> Option<Waited> {
    let ended = match status {
        Status::Blocked | Status::Done | Status::Idle => seen_working || quiet,
        Status::None => quiet,
        Status::Working => false,
    };
    if !ended || checks.iter().any(|c| matches!(c.state, CheckState::Checking | CheckState::Pending)) {
        return None;
    }
    let said = if status == Status::None { String::new() } else { said.to_string() };
    Some(Waited::Turn(status, said, headline(checks).cloned().map(Box::new)))
}

/// How `seshi wait` exits once the turn is over, when not 0: 3 when the agent needs you, 4
/// when a checkpoint failed (`headline` is the worst). Every other result, a stale pass or a
/// check that did not run included, says nothing against the work.
fn turn_exit(status: Status, headline: Option<&CheckInfo>) -> Option<i32> {
    if status == Status::Blocked {
        return Some(3);
    }
    headline.is_some_and(|c| c.state == CheckState::Failed).then_some(EXIT_CHECK_FAILED)
}

/// The check's result in one line, for a waiter to read beside the reply.
pub fn check_line(c: &CheckInfo) -> String {
    let exit = c.exit_code.map(|code| format!(" (exit {code})")).unwrap_or_default();
    let line = if c.first_failing_line.is_empty() { String::new() } else { format!(": {}", c.first_failing_line) };
    match c.state {
        CheckState::Checking => format!("checking: {}", c.command),
        CheckState::Passed => format!("check passed: {}", c.command),
        CheckState::Failed => format!("check failed: {}{exit}{line}", c.command),
        CheckState::Overridden => format!("check failed, accepted: {}{exit}{line}", c.command),
        CheckState::CouldNotRun => format!("check could not run: {}", c.command),
        CheckState::Pending => format!("pending: {}", c.command),
        CheckState::NeedsApproval | CheckState::Stale | CheckState::Never => check_note(c),
    }
}

/// What a check adds beside the status in `seshi ls` and `seshi_list`: `check failed (exit 1)`.
pub fn check_note(c: &CheckInfo) -> String {
    match (c.state, c.exit_code) {
        (CheckState::Checking, _) => "checking".into(),
        (CheckState::Passed, _) => "check passed".into(),
        (CheckState::Failed, Some(code)) => format!("check failed (exit {code})"),
        (CheckState::Failed, None) => "check failed".into(),
        (CheckState::Overridden, _) => "check overridden".into(),
        (CheckState::CouldNotRun, _) => "check could not run".into(),
        (CheckState::Pending, _) => "pending".into(),
        (CheckState::NeedsApproval, _) => format!("approve check: {}", c.command),
        (CheckState::Stale, _) => format!("stale pass: {} · {}", checkpoint_name(c), files_changed(c.changed_files.len())),
        (CheckState::Never, _) => "not checked: never for this repo".into(),
    }
}

/// A checkpoint's name, or `check` for one that has none.
fn checkpoint_name(c: &CheckInfo) -> &str {
    if c.name.is_empty() { "check" } else { &c.name }
}

fn files_changed(n: usize) -> String {
    if n == 1 { "1 file changed".into() } else { format!("{n} files changed") }
}

/// The headline checkpoint's note, and how many there are when there are several:
/// `check failed (exit 1) · 2 checkpoints`.
pub fn checks_note(p: &TermInfo) -> Option<String> {
    let note = check_note(p.reported_check()?);
    Some(if p.checks.len() > 1 { format!("{note} · {} checkpoints", p.checks.len()) } else { note })
}

/// `claude: done · check failed (exit 1)`, for an agent pane.
fn agent_tag(p: &TermInfo) -> Option<String> {
    let agent = p.agent.as_deref()?;
    let quiet = quiet_note(p).map(|n| format!(" · {n}")).unwrap_or_default();
    let note = checks_note(p).map(|n| format!(" · {n}")).unwrap_or_default();
    Some(format!("{agent}: {}{quiet}{note}", p.status.label()))
}

/// `quiet 12m` for a working agent that has gone silent: it may be stuck.
pub fn quiet_note(p: &TermInfo) -> Option<String> {
    p.quiet_since.filter(|_| p.status == Status::Working).map(|at| format!("quiet {}", quiet_for(at)))
}

/// How long since `at` (unix seconds), shortest unit: `45s`, `12m`, `3h`, `2d`.
pub fn quiet_for(at: u64) -> String {
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let s = now.saturating_sub(at);
    match s {
        0..=59 => format!("{s}s"),
        60..=3599 => format!("{}m", s / 60),
        3600..=86_399 => format!("{}h", s / 3600),
        _ => format!("{}d", s / 86_400),
    }
}

/// One pane in `seshi ls --json`.
fn pane_json(p: &TermInfo) -> Value {
    let mut v = json!({
        "id": p.id, "process": p.process, "title": p.title, "agent": p.agent, "asleep": p.asleep, "status_why": p.status_why, "win32_input": p.win32_input, "model": p.model, "name": p.name, "mem_mb": p.mem >> 20, "bell": p.bell,
        "status": p.status.label(), "cols": p.cols, "rows": p.rows, "cwd": p.cwd,
    });
    if let Some(at) = p.quiet_since.filter(|_| p.status == Status::Working) {
        v["quiet_since"] = json!(at);
    }
    if let Some(c) = p.reported_check() {
        v["check"] = json!({
            "state": c.state.label(), "command": c.command, "exit_code": c.exit_code, "duration_ms": c.duration_ms,
            "first_failing_line": c.first_failing_line, "rerun_queued": c.rerun_queued, "overridden_at": c.overridden_at,
        });
        v["checks"] = p.checks.iter().map(checkpoint_json).collect();
    }
    v
}

/// One checkpoint in `seshi ls --json`'s `checks`.
fn checkpoint_json(c: &CheckInfo) -> Value {
    json!({
        "name": c.name, "state": c.state.label(), "command": c.command, "exit_code": c.exit_code, "duration_ms": c.duration_ms,
        "first_failing_line": c.first_failing_line, "line_count": c.line_count, "turn": c.turn, "rerun_queued": c.rerun_queued,
        "overridden_at": c.overridden_at, "overridden_by": c.overridden_by, "stale_since": c.stale_since,
        "changed_files": c.changed_files.iter().map(|f| json!({ "kind": f.kind.to_string(), "path": f.path })).collect::<Vec<_>>(),
    })
}

/// Wait on pane `term`: for its turn to end (it was working, now it isn't; a prompt that
/// never starts a turn counts as ended after a few seconds), or for `regex` on its screen.
/// `just_sent`: the turn hasn't started yet, so wait for it to start first. A session with a
/// check is waited on until the check has answered, within `timeout`.
pub fn wait_on(term: TermId, regex: Option<&str>, timeout: Duration, just_sent: bool) -> Result<Waited> {
    let re = regex.map(|r| regex::RegexBuilder::new(r).case_insensitive(true).build()).transpose().context("bad --regex")?;
    let start = std::time::Instant::now();
    let mut seen_working = !just_sent;
    let mut quiet = None;
    loop {
        if let Some(re) = &re {
            if let Reply::Text(screen) = block_on(request(ClientMsg::Query(Query::Read { term })))?
                && let Some(l) = screen.lines().find(|l| re.is_match(l))
            {
                return Ok(Waited::Matched(l.trim().to_string()));
            }
        } else {
            let snap = snapshot()?;
            let t = snap.terms.get(&term).ok_or_else(|| anyhow!("pane {term} is gone"))?;
            seen_working |= t.status == Status::Working;
            quiet = t.quiet_since.filter(|_| t.status == Status::Working);
            if let Some(done) = turn_over(t.status, &t.said, &t.checks, seen_working, start.elapsed() > QUIET_TURN_AFTER) {
                return Ok(done);
            }
        }
        if start.elapsed() >= timeout {
            return Ok(Waited::TimedOut { quiet });
        }
        std::thread::sleep(Duration::from_millis(400));
    }
}

pub fn send_wait(pane: Option<TermId>, text: String, enter: bool, timeout: u64) -> Result<()> {
    let term = resolve_pane(pane)?;
    send(Some(term), text, enter)?;
    wait_print(term, None, timeout, true)
}

pub fn wait(pane: Option<TermId>, regex: Option<String>, timeout: u64) -> Result<()> {
    wait_print(resolve_pane(pane)?, regex, timeout, false)
}

/// `seshi wait` / `seshi send --wait`: wait, then print the reply (or the screen's end).
fn wait_print(term: TermId, regex: Option<String>, timeout: u64, just_sent: bool) -> Result<()> {
    match wait_on(term, regex.as_deref(), Duration::from_secs(timeout), just_sent)? {
        Waited::Matched(l) => println!("{l}"),
        Waited::Turn(s, said, check) => {
            if s == Status::Blocked {
                eprintln!("(pane {term} needs you)");
            }
            if let Some(c) = &check {
                eprintln!("({})", check_line(c));
            }
            if said.trim().is_empty() {
                if let Reply::Text(screen) = block_on(request(ClientMsg::Query(Query::Read { term })))? {
                    let lines: Vec<&str> = screen.trim_end().lines().collect();
                    println!("{}", lines[lines.len().saturating_sub(30)..].join("\n"));
                }
            } else {
                println!("{}", said.trim_end());
            }
            if let Some(code) = turn_exit(s, check.as_deref()) {
                std::process::exit(code);
            }
        }
        Waited::TimedOut { quiet } => {
            match quiet {
                Some(at) => eprintln!("timed out waiting on pane {term}: still working, but quiet for {}", quiet_for(at)),
                None => eprintln!("timed out waiting on pane {term}"),
            }
            std::process::exit(2);
        }
    }
    Ok(())
}

pub fn send(pane: Option<TermId>, text: String, enter: bool) -> Result<()> {
    let term = resolve_pane(pane)?;
    block_on(async move {
        let (_r, mut w) = ipc::open(false).await.context(if ipc::remote().is_some() { "over ssh" } else { "couldn't reach the seshi server" })?;
        let data = if text.contains('\n') { format!("\x1b[200~{text}\x1b[201~") } else { text };
        ipc::send(&mut w, &ClientMsg::Input { term, data: data.into_bytes() }).await?;
        if enter {
            // A separate write so the program reads the text before the Enter.
            tokio::time::sleep(Duration::from_millis(30)).await;
            ipc::send(&mut w, &ClientMsg::Input { term, data: b"\r".to_vec() }).await?;
        }
        tokio::time::sleep(Duration::from_millis(30)).await;
        Ok(())
    })
}

/// Send key presses (`ctrl+space`, `%`, `enter`), encoded the way a terminal would.
pub fn send_keys(pane: Option<TermId>, keys: Vec<String>) -> Result<()> {
    let term = resolve_pane(pane)?;
    let mut chunks = Vec::new();
    for k in &keys {
        let spec: crate::keys::KeySpec = k.parse()?;
        let ev = ratatui::crossterm::event::KeyEvent::new(spec.code, spec.mods);
        chunks.push(crate::keys::encode(&ev, false));
    }
    block_on(async move {
        let (_r, mut w) = ipc::open(false).await.context(if ipc::remote().is_some() { "over ssh" } else { "couldn't reach the seshi server" })?;
        for data in chunks {
            ipc::send(&mut w, &ClientMsg::Input { term, data }).await?;
            // One key per beat, like a person typing; lets modes change between keys.
            tokio::time::sleep(Duration::from_millis(80)).await;
        }
        Ok(())
    })
}

fn join_command(parts: Vec<String>) -> Option<String> {
    (!parts.is_empty()).then(|| parts.join(" "))
}

pub fn split(pane: Option<TermId>, down: bool, command: Vec<String>) -> Result<()> {
    let term = resolve_pane(pane)?;
    let dir = if down { Dir::Down } else { Dir::Right };
    self::command(Command::Split { term, dir, cmd: join_command(command), cwd: None })
}

/// `seshi handoff <pane> --to <agent>`: start `to` beside `pane`, in its checkout, with a
/// text brief of what the pane was doing as its first prompt. The old pane is left alone.
pub fn handoff(pane: TermId, to: String) -> Result<()> {
    let (cfg, _) = crate::config::Config::load_or_default();
    let before = snapshot()?;
    let from = handoff::check(&cfg, pane, before.terms.get(&pane), &to).map_err(|e| anyhow!(e))?;
    let t = &before.terms[&pane];
    let screen = match block_on(request(ClientMsg::Query(Query::Read { term: pane })))? {
        Reply::Text(s) => s,
        _ => bail!("unexpected reply"),
    };
    let checkout = t.top.clone().unwrap_or_else(|| t.cwd.clone());
    let changed = match ipc::remote() {
        Some(_) => Err("git isn't run on the remote machine".into()),
        None => handoff::changed_files(&checkout, t.root.as_deref()),
    };
    let facts = handoff::Facts { pane, agent: from, task: t.name.clone(), last_prompt: t.summary.clone(), branch: t.branch.clone(), checkout: checkout.clone(), changed, screen };
    let cmd = cfg.agent_command(Some(&to), None, &handoff::brief(&facts)).map_err(|e| anyhow!(e))?;
    let had: Vec<TermId> = before.terms.keys().copied().collect();
    command(Command::Split { term: pane, dir: Dir::Right, cmd: Some(cmd), cwd: Some(checkout) })?;
    for _ in 0..60 {
        std::thread::sleep(Duration::from_millis(250));
        let Ok(now) = snapshot() else { continue };
        if let Some(new) = handoff::new_pane(&had, now.terms.keys().copied()) {
            let t = &now.terms[&new];
            println!("handed pane {pane} to {to}: pane {new} in {}", t.top.as_ref().unwrap_or(&t.cwd).display());
            return Ok(());
        }
    }
    println!("asked seshi to start {to}; it hasn't appeared yet");
    Ok(())
}

pub fn new_workspace(path: Option<PathBuf>, name: Option<String>, cmd: Vec<String>) -> Result<()> {
    let cwd = path.or_else(|| std::env::current_dir().ok()).map(|p| p.canonicalize().unwrap_or(p));
    command(Command::NewWorkspace { cwd, name, cmd: join_command(cmd) })
}

pub fn focus(pane: TermId) -> Result<()> {
    command(Command::FocusPane { term: pane })
}

/// The session you're on, if a server is running.
pub fn focused_pane() -> Option<TermId> {
    let s = snapshot().ok()?;
    s.workspaces.iter().find(|w| Some(w.id) == s.active_ws)?.tab().map(|t| t.focus)
}

/// `seshi popup -- <command>`: a floating pane in the folder you're in.
pub fn popup(cmd: Vec<String>) -> Result<()> {
    let cmd = join_command(cmd).ok_or_else(|| anyhow!("what to run: seshi popup -- fzf"))?;
    command(Command::Popup { cmd, cwd: std::env::current_dir().ok() })
}

/// `seshi grant <pane> read,write,…|default`.
pub fn grant(term: TermId, grants: &str) -> Result<()> {
    let list = (grants.trim() != "default").then(|| grants.split(',').map(|g| g.trim().to_lowercase()).filter(|g| !g.is_empty()).collect::<Vec<_>>());
    command(Command::Grant { term, grants: list.clone() })?;
    match list {
        Some(l) if l.is_empty() => println!("pane {term} may do nothing through seshi but its own work"),
        Some(l) => println!("pane {term} may: {}", l.join(", ")),
        None => println!("pane {term} is back to the default grants ([mcp] grants in config)"),
    }
    Ok(())
}

/// `seshi ask-human "…?" -o Yes -o No`: ask the person, wait, print the answer.
pub fn ask_human(text: String, options: Vec<String>) -> Result<()> {
    let term = resolve_pane(None).context("run it in a seshi pane (the question shows there)")?;
    match block_on(request(ClientMsg::Command(Command::AskHuman { term, text, options })))? {
        Reply::Text(answer) => {
            println!("{answer}");
            Ok(())
        }
        _ => bail!("no answer"),
    }
}

/// `seshi teach [pane]`: the program running there is an agent.
pub fn teach(pane: Option<TermId>) -> Result<()> {
    let term = resolve_pane(pane)?;
    command(Command::TeachAgent { term })?;
    println!("seshi knows it as an agent now (see [[agents]] in {})", crate::config::config_path().display());
    Ok(())
}

/// Focus a pane and bring seshi's window forward (what clicking a notification does).
pub fn reveal(pane: TermId) -> Result<()> {
    command(Command::Reveal { term: pane })
}

pub fn close(pane: Option<TermId>) -> Result<()> {
    command(Command::ClosePane { term: resolve_pane(pane)? })
}

/// Whether a seshi server is running (for this socket).
pub(crate) fn server_running() -> bool {
    block_on(async { ipc::connect().await.map(|_| ()) }).is_ok()
}

pub fn kill_server(forget: bool) -> Result<()> {
    if ipc::remote().is_none() && !server_running() {
        if forget {
            crate::daemon::forget_session();
        }
        println!("no seshi server is running; nothing to stop");
        return Ok(());
    }
    match command(Command::KillServer { forget }) {
        // Another version can't be asked to stop: stop its process instead.
        Err(e) if ipc::other_version(&e).is_some() => {
            let pid = stop_server_process()?;
            if forget {
                crate::daemon::forget_session();
            }
            println!("stopped the server (another seshi version, process {pid})");
            Ok(())
        }
        // It answers, then lets go of its socket a moment later: a server started right
        // after would find it.
        Ok(()) if ipc::remote().is_none() => crate::restart::wait_until_server_gone(),
        r => r,
    }
}

/// Stop this socket's server by its process, for when it's a version that can't be talked
/// to. Its saved session stays, as with `kill-server`.
fn stop_server_process() -> Result<u32> {
    let pid = server_pid().filter(|pid| is_seshi_daemon(*pid)).ok_or_else(|| {
        anyhow!("couldn't find the server's process; stop it yourself (on Linux/macOS: pkill -f \"seshi daemon\"; on Windows: end seshi.exe in Task Manager)")
    })?;
    let killed = if cfg!(windows) {
        crate::proc::run(std::process::Command::new("taskkill").args(["/PID", &pid.to_string(), "/F"]))
    } else {
        crate::proc::run(std::process::Command::new("kill").arg(pid.to_string()))
    };
    killed.map_err(|e| anyhow!("couldn't stop the server (process {pid}): {e}"))?;
    // Wait for it to let go of its socket, so the next start doesn't find it.
    for _ in 0..40 {
        if block_on(ipc::connect()).is_err() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
    let _ = std::fs::remove_file(ipc::pid_file());
    Ok(pid)
}

/// Whether `pid` is a `seshi daemon` process, so a stale pid file can't point a stop at
/// something else that took the number. Where the process list can't be read, the pid
/// file's word stands.
fn is_seshi_daemon(pid: u32) -> bool {
    #[cfg(target_os = "linux")]
    {
        let Ok(cmdline) = std::fs::read(format!("/proc/{pid}/cmdline")) else { return false };
        let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
        args.first().is_some_and(|a| a.ends_with(b"seshi")) && args.get(1) == Some(&b"daemon".as_slice())
    }
    #[cfg(not(target_os = "linux"))]
    {
        let _ = pid;
        true
    }
}

/// This socket's server process: from the id it noted, or (servers too old to note it) the
/// one running `seshi daemon` for this socket.
fn server_pid() -> Option<u32> {
    if let Some(pid) = std::fs::read_to_string(ipc::pid_file()).ok().and_then(|s| s.trim().parse().ok()) {
        return Some(pid);
    }
    #[cfg(target_os = "linux")]
    {
        let label = std::env::var("SESHI_SOCKET").unwrap_or_else(|_| "default".into());
        let me = std::process::id();
        for entry in std::fs::read_dir("/proc").ok()?.flatten() {
            let Some(pid) = entry.file_name().to_str().and_then(|s| s.parse::<u32>().ok()) else { continue };
            if pid == me {
                continue;
            }
            let Ok(cmdline) = std::fs::read(entry.path().join("cmdline")) else { continue };
            let args: Vec<&[u8]> = cmdline.split(|b| *b == 0).collect();
            let is_daemon = args.first().is_some_and(|a| a.ends_with(b"seshi")) && args.get(1) == Some(&b"daemon".as_slice());
            if !is_daemon {
                continue;
            }
            // Its socket: SESHI_SOCKET in its environment, else the default one.
            let env = std::fs::read(entry.path().join("environ")).unwrap_or_default();
            let theirs = env
                .split(|b| *b == 0)
                .find_map(|kv| kv.strip_prefix(b"SESHI_SOCKET="))
                .map(|v| String::from_utf8_lossy(v).into_owned())
                .unwrap_or_else(|| "default".into());
            if theirs == label {
                return Some(pid);
            }
        }
    }
    // Elsewhere a server's socket can't be read off its process: only when there's exactly
    // one, and it's the default server you mean.
    #[cfg(not(target_os = "linux"))]
    if std::env::var("SESHI_SOCKET").is_err() {
        let list = if cfg!(windows) {
            crate::proc::run(std::process::Command::new("powershell").args([
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                "Get-CimInstance Win32_Process -Filter \"Name='seshi.exe'\" | Where-Object { $_.CommandLine -match '\\sdaemon\\s*$' } | ForEach-Object { $_.ProcessId }",
            ]))
        } else {
            crate::proc::run(std::process::Command::new("pgrep").args(["-f", "seshi daemon$"]))
        };
        let pids: Vec<u32> = list.unwrap_or_default().split_whitespace().filter_map(|p| p.parse().ok()).filter(|p| *p != std::process::id()).collect();
        if let [pid] = pids[..] {
            return Some(pid);
        }
    }
    None
}

/// The workspace this command runs in (via its pane), else the active one.
fn resolve_ws(ws: Option<WsId>) -> Result<WsId> {
    if let Some(ws) = ws {
        return Ok(ws);
    }
    let snap = snapshot()?;
    let here = std::env::var("SESHI_TERM_ID").ok().and_then(|s| s.parse().ok());
    here.and_then(|t| snap.locate(t).map(|(w, _)| w.id))
        .or(snap.active_ws)
        .ok_or_else(|| anyhow!("no workspace to target"))
}

pub fn worktree(branch: String, base: Option<String>, ws: Option<WsId>, cmd: Vec<String>) -> Result<()> {
    let ws = resolve_ws(ws)?;
    command(Command::NewWorktree { ws, branch, base, cmd: join_command(cmd), split: None, from: None })
}

fn codex_status_hooks_installed(text: &str, windows: bool) -> bool {
    let Ok(config) = text.parse::<toml_edit::DocumentMut>() else {
        return false;
    };
    [("SessionStart", true), ("SubagentStart", false), ("SubagentStop", false)].into_iter().all(|(event, same_status)| {
        config
            .get("hooks")
            .and_then(toml_edit::Item::as_table)
            .and_then(|hooks| hooks.get(event))
            .and_then(toml_edit::Item::as_array_of_tables)
            .is_some_and(|entries| {
                entries.iter().any(|entry| {
                    entry
                        .get("hooks")
                        .and_then(toml_edit::Item::as_array_of_tables)
                        .is_some_and(|handlers| {
                            handlers.iter().any(|handler| {
                                if handler.get("type").and_then(toml_edit::Item::as_str) != Some("command") {
                                    return false;
                                }
                                let command_ok = handler
                                    .get("command")
                                    .and_then(toml_edit::Item::as_str)
                                    .is_some_and(|command| {
                                        if same_status {
                                            command.ends_with(" hook codex --status same")
                                        } else {
                                            command.ends_with(" hook codex")
                                        }
                                    });
                                let windows_ok = !windows || handler
                                    .get("commandWindows")
                                    .and_then(toml_edit::Item::as_str)
                                    .is_some_and(|command| {
                                        command.starts_with("& ")
                                            && if same_status { command.ends_with(" hook codex --status same") } else { command.ends_with(" hook codex") }
                                    });
                                command_ok && windows_ok
                            })
                        })
                })
            })
    })
}


/// `seshi doctor`: each thing seshi relies on, ✓ or what to do about it.
pub fn doctor() -> Result<()> {
    let mut bad = 0;
    let mut line = |ok: Option<bool>, what: &str, detail: String| {
        let mark = match ok {
            Some(true) => "\x1b[32m✓\x1b[0m",
            Some(false) => {
                bad += 1;
                "\x1b[31m✕\x1b[0m"
            }
            None => "\x1b[33m·\x1b[0m",
        };
        println!(" {mark} {what:<22} {detail}");
    };
    let run = |cmd: &str, args: &[&str]| -> Option<String> {
        let mut c = std::process::Command::new(cmd);
        c.args(args);
        let out = c.output().ok().filter(|o| o.status.success())?;
        Some(String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").trim().to_string())
    };
    let on_path = |name: &str| -> Option<String> {
        let names = if cfg!(windows) { vec![format!("{name}.exe"), format!("{name}.cmd"), format!("{name}.ps1"), name.to_string()] } else { vec![name.to_string()] };
        std::env::var_os("PATH").and_then(|p| {
            std::env::split_paths(&p).find_map(|d| names.iter().map(|n| d.join(n)).find(|f| f.is_file()).map(|f| f.display().to_string()))
        })
    };

    println!("seshi {}  (protocol {})\n", env!("CARGO_PKG_VERSION"), PROTOCOL_VERSION);

    // Config
    let path = crate::config::config_path();
    match crate::config::Config::load() {
        Ok(_) if path.exists() => line(Some(true), "config", path.display().to_string()),
        Ok(_) => line(None, "config", format!("none yet (defaults); `seshi config init` writes one at {}", path.display())),
        Err(e) => line(Some(false), "config", format!("{e:#}  ({})", path.display())),
    }
    let (cfg, _) = crate::config::Config::load_or_default();

    // Server
    let server = block_on(async { ipc::open(false).await });
    match server {
        Ok(_) => line(Some(true), "server", format!("running ({})", ipc::socket_id())),
        Err(e) if format!("{e:#}").contains("protocol") => {
            line(Some(false), "server", format!("{e:#}"));
            println!("{:27}fix: run seshi; it restarts the server itself (or `seshi kill-server`)", "");
        }
        Err(_) => line(None, "server", "not running (it starts with `seshi`)".into()),
    }

    // Tools
    match run("git", &["--version"]) {
        Some(v) => line(Some(true), "git", v),
        None => line(Some(false), "git", "not found; worktrees, Changes and branches need it".into()),
    }
    match run("gh", &["--version"]) {
        Some(v) => line(Some(true), "gh (GitHub CLI)", v),
        None => line(None, "gh (GitHub CLI)", "not found; PRs and GitHub issues use it (optional)".into()),
    }
    let shell = cfg.shell_command();
    let shell_ok = on_path(&shell[0]).is_some() || std::path::Path::new(&shell[0]).is_file();
    line(Some(shell_ok), "shell", shell.join(" "));

    // Agents
    let mut found = Vec::new();
    for a in [
        "claude", "codex", "gemini", "opencode", "cursor-agent", "copilot", "amp", "qwen",
        "aider", "grok", "auggie", "kimi", "pi", "omp", "kilo", "mastracode",
    ] {
        if on_path(a).is_some() {
            found.push(a);
        }
    }
    line(Some(!found.is_empty()), "agents on PATH", if found.is_empty() { "none found (claude, codex, gemini, …)".into() } else { found.join(", ") });

    // Codex child sessions are only attached to their parent pane after these machine-local hooks run.
    if found.contains(&"codex") {
        let path = codex_config();
        match std::fs::read_to_string(&path) {
            Ok(text) if codex_status_hooks_installed(&text, cfg!(windows)) => {
                line(Some(true), "codex subagent hooks", "installed".into());
            }
            Ok(text) if text.parse::<toml_edit::DocumentMut>().is_err() => {
                line(Some(false), "codex subagent hooks", format!("{} is invalid TOML; repair it before `seshi integrate codex`", path.display()));
            }
            Ok(_) => line(Some(false), "codex subagent hooks", "missing: `seshi integrate codex` (restart Codex to load them)".into()),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                line(Some(false), "codex subagent hooks", "missing: `seshi integrate codex` (restart Codex to load them)".into());
            }
            Err(error) => line(Some(false), "codex subagent hooks", format!("couldn't read {}: {error}", path.display())),
        }
    }

    // File presence means setup is installed, not that a running host has reloaded it.
    // Until a native event arrives, the normal process and screen detectors remain active.
    for agent in ["pi", "opencode", "omp", "kilo", "mastracode", "kimi"] {
        if !found.contains(&agent) {
            continue;
        }
        let path = match integrations::status_adapter_path(agent) {
            Ok(path) => path,
            Err(error) => {
                line(
                    None,
                    &format!("{agent} status adapter"),
                    format!("couldn't locate config: {error:#}"),
                );
                continue;
            }
        };
        if integrations::status_adapter_installed(agent, &path) {
            line(
                None,
                &format!("{agent} status adapter"),
                format!(
                    "configured at {}; restart/reload {agent} to load; native status is confirmed when its first event arrives",
                    path.display()
                ),
            );
        } else {
            line(
                None,
                &format!("{agent} status adapter"),
                format!("not installed; fallback detection remains active; run `seshi integrate {agent}`"),
            );
        }
    }

    // Claude: hooks and MCP
    if found.contains(&"claude") {
        let settings = std::fs::read_to_string(claude_settings()).unwrap_or_default();
        let hooked = settings.contains("hook claude");
        let stale = stale_claude_hooks();
        match (hooked, stale.first()) {
            (false, _) => line(Some(false), "claude status hooks", "missing: `seshi integrate claude` (exact working / needs-you / done)".into()),
            // Another seshi can't talk to this one's server: every agent would look idle.
            (true, Some(other)) => line(Some(false), "claude status hooks", format!("they run another seshi ({other}); `seshi integrate claude` (or restart the server) fixes it")),
            (true, None) => line(Some(true), "claude status hooks", "installed".into()),
        }
        // Any hook whose program is gone (often an old hydra's): Claude shows "hook error" on
        // every turn it runs in, and only removing or repointing it stops that.
        let mut broken = Vec::new();
        for file in claude_settings_files() {
            let Some(root) = std::fs::read_to_string(&file).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()) else { continue };
            for cmd in broken_hooks_in(&root, |p| p.exists(), |n| on_path(n).is_some()) {
                broken.push(format!("{} in {}", program_of(&cmd), file.display()));
            }
        }
        match broken.first() {
            None => line(Some(true), "claude hooks run", "every hook's program is there".into()),
            Some(first) => line(
                Some(false),
                "claude hooks run",
                format!("{} missing ({first}{}); `seshi integrate claude` fixes seshi's own, remove the rest", broken.len(), if broken.len() > 1 { ", …" } else { "" }),
            ),
        }
        let claude_json = std::fs::read_to_string(claude_json()).unwrap_or_default();
        let mcp = claude_json.contains("\"seshi\"") && claude_json.contains("\"mcp\"");
        let old = serde_json::from_str::<Value>(&claude_json).map(|root| old_mcp_names(&root)).unwrap_or_default();
        match old.first() {
            // Claude names the server in every call it shows ("Called hydra"), and may run an old program.
            Some(name) => line(Some(false), "seshi MCP for claude", format!("still registered as `{name}`: `seshi integrate mcp` renames it")),
            None => line(if mcp { Some(true) } else { None }, "seshi MCP for claude", if mcp { "registered".into() } else { "not set up: `seshi integrate mcp` lets agents see each other (optional)".into() }),
        }
    }

    // Terminal
    let term = std::env::var("TERM_PROGRAM").or_else(|_| std::env::var("TERM")).unwrap_or_else(|_| if std::env::var("WT_SESSION").is_ok() { "Windows Terminal".into() } else { "unknown".into() });
    let truecolor = std::env::var("COLORTERM").is_ok_and(|c| c.contains("truecolor") || c.contains("24bit")) || std::env::var("WT_SESSION").is_ok();
    line(if truecolor { Some(true) } else { None }, "terminal", format!("{term}{}", if truecolor { ", true colour" } else { " (colours may look off without true colour)" }));
    if let Ok((w, h)) = crossterm::terminal::size() {
        line(Some(w >= 100 && h >= 30), "window size", format!("{w}×{h}{}", if w < 100 || h < 30 { " (seshi wants at least 100×30)" } else { "" }));
    }

    // Data and sync
    let data = crate::config::data_dir();
    let writable = std::fs::create_dir_all(&data).is_ok() && std::fs::write(data.join(".doctor"), b"ok").is_ok();
    let _ = std::fs::remove_file(data.join(".doctor"));
    line(Some(writable), "data folder", data.display().to_string());
    line(None, "sync", if crate::sync::enabled() { format!("on ({})", crate::sync::dir().display()) } else { "off (`seshi sync setup` shares config and ideas)".into() });
    if let Some(ed) = Some(cfg.editor.clone()).filter(|e| !e.is_empty()).or_else(|| std::env::var("VISUAL").ok()).or_else(|| std::env::var("EDITOR").ok()) {
        line(Some(true), "editor", ed);
    } else {
        line(None, "editor", "none set (editor = \"code\" in config, or $EDITOR)".into());
    }

    println!();
    if bad == 0 {
        println!("All good.");
    } else {
        println!("{bad} thing(s) to fix above.");
    }
    Ok(())
}


/// Run by an agent inside a pane: make a worktree and move this agent into it.
pub fn move_to_worktree(branch: String) -> Result<()> {
    let term: TermId = std::env::var("SESHI_TERM_ID")
        .ok()
        .and_then(|s| s.parse().ok())
        .ok_or_else(|| anyhow!("run this from inside a seshi pane (an agent running in seshi)"))?;
    let branch = Some(branch.trim().to_string()).filter(|b| !b.is_empty());
    block_on(async move {
        let (mut r, mut w) = ipc::open(false).await.context(if ipc::remote().is_some() { "over ssh" } else { "couldn't reach the seshi server" })?;
        ipc::send(&mut w, &ClientMsg::Command(Command::MoveToWorktree { term, branch })).await?;
        loop {
            match ipc::recv_server(&mut r).await? {
                Some(ServerMsg::Notice(n)) => println!("{n}"),
                Some(ServerMsg::Reply(_)) => return Ok(()),
                Some(ServerMsg::Error(e)) => bail!(e),
                Some(_) => continue,
                None => bail!("server closed the connection"),
            }
        }
    })
}

pub fn worktree_remove(ws: Option<WsId>, force: bool) -> Result<()> {
    command(Command::RemoveWorktree { ws: resolve_ws(ws)?, force, delete_branch: false })
}

pub fn config_init() -> Result<()> {
    let path = crate::config::config_path();
    if path.exists() {
        bail!("{} already exists", path.display());
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    std::fs::write(&path, crate::config::EXAMPLE)?;
    println!("wrote {}", path.display());
    Ok(())
}

// ---- hooks -------------------------------------------------------------------------

/// Map a Claude Code (or Codex, same dialect) hook payload to a status.
pub fn status_from_hook(payload: &Value) -> Option<HookStatus> {
    let event = payload.get("hook_event_name").and_then(Value::as_str).unwrap_or("");
    let tool = payload.get("tool_name").and_then(Value::as_str).unwrap_or("");
    Some(match event {
        "PreToolUse" if tool == "AskUserQuestion" => HookStatus::Blocked,
        "UserPromptSubmit" | "PreToolUse" | "PostToolUse" | "SubagentStart" => HookStatus::Working,
        // Gemini CLI.
        "BeforeAgent" | "BeforeTool" | "AfterTool" => HookStatus::Working,
        "AfterAgent" => HookStatus::Done,
        "SubagentStop" | "SubagentEnd" => HookStatus::Same,
        "PermissionRequest" => HookStatus::Blocked,
        "Notification" => {
            let kind = payload.get("notification_type").and_then(Value::as_str).unwrap_or("");
            let msg = payload.get("message").and_then(Value::as_str).unwrap_or("").to_lowercase();
            match kind {
                "permission_prompt" | "elicitation_dialog" | "ToolPermission" => HookStatus::Blocked,
                "idle_prompt" => HookStatus::Done,
                _ if msg.contains("permission") => HookStatus::Blocked,
                _ => return None,
            }
        }
        "Stop" => HookStatus::Done,
        "SessionStart" => HookStatus::Idle,
        "SessionEnd" => HookStatus::Gone,
        "pi.agent_start" | "pi.ui_prompt_end" => HookStatus::Working,
        "pi.ui_prompt_start" => HookStatus::Blocked,
        "pi.agent_settled" => HookStatus::Done,
        "pi.session_start" => HookStatus::Idle,
        "pi.session_shutdown" => HookStatus::Gone,
        // opencode (its plugin passes the event).
        "" if payload.get("type").and_then(Value::as_str).is_some_and(|t| t.contains('.')) => {
            let ty = payload.get("type").and_then(Value::as_str).unwrap_or("");
            let state = payload.pointer("/properties/status/type").and_then(Value::as_str).unwrap_or("");
            match ty {
                "session.status" if state == "busy" || state == "retry" => HookStatus::Working,
                "session.status" if state == "idle" => HookStatus::Done,
                "session.idle" | "session.error" => HookStatus::Done,
                "permission.asked" | "permission.updated" | "question.asked" => HookStatus::Blocked,
                "permission.replied" | "question.replied" | "question.rejected" => HookStatus::Working,
                "session.created" => HookStatus::Idle,
                "session.deleted" => HookStatus::Gone,
                _ => return None,
            }
        }
        // Codex `notify`.
        _ if payload.get("type").and_then(Value::as_str) == Some("agent-turn-complete") => HookStatus::Done,
        _ => return None,
    })
}

/// The last thing the assistant wrote, from a Claude Code transcript (JSON lines).
fn last_assistant_text(path: &std::path::Path) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut f = std::fs::File::open(path).ok()?;
    let len = f.metadata().ok()?.len();
    // Only the tail matters; transcripts can be large.
    let start = len.saturating_sub(512 * 1024);
    f.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = String::new();
    f.read_to_string(&mut buf).ok()?;
    buf.lines().rev().find_map(|line| {
        let v: Value = serde_json::from_str(line).ok()?;
        if v.get("type").and_then(Value::as_str) != Some("assistant") {
            return None;
        }
        let content = v.pointer("/message/content")?.as_array()?;
        let text: Vec<&str> = content
            .iter()
            .filter(|c| c.get("type").and_then(Value::as_str) == Some("text"))
            .filter_map(|c| c.get("text").and_then(Value::as_str))
            .collect();
        (!text.is_empty()).then(|| text.join("\n"))
    })
}

/// Collapse whitespace and cut to `max` characters.
fn one_line(s: &str, max: usize) -> String {
    let flat = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if flat.chars().count() <= max {
        return flat;
    }
    let mut cut: String = flat.chars().take(max.saturating_sub(1)).collect();
    cut.push('…');
    cut
}

fn parse_status(s: &str) -> Option<HookStatus> {
    Some(match s {
        "working" | "busy" | "running" => HookStatus::Working,
        "blocked" | "waiting" | "input" => HookStatus::Blocked,
        "done" | "finished" => HookStatus::Done,
        "idle" => HookStatus::Idle,
        "same" => HookStatus::Same,
        "gone" | "exit" | "end" => HookStatus::Gone,
        _ => return None,
    })
}

pub fn hook(agent: &str, status: Option<&str>, payload: Option<&str>) -> Result<()> {
    // Inert outside seshi, so global hooks don't bother other terminals.
    let Some(term) = std::env::var("SESHI_TERM_ID").ok().and_then(|s| s.parse::<TermId>().ok()) else {
        return Ok(());
    };
    let mut payload_json = payload.and_then(|raw| serde_json::from_str(raw).ok()).unwrap_or(Value::Null);
    let status = match status {
        Some(s) => parse_status(s).ok_or_else(|| anyhow!("unknown status {s}"))?,
        None => {
            let raw = match payload {
                Some(p) => p.to_string(),
                None => {
                    let mut s = String::new();
                    std::io::stdin().take(1 << 20).read_to_string(&mut s)?;
                    s
                }
            };
            payload_json = serde_json::from_str(&raw).unwrap_or(Value::Null);
            match status_from_hook(&payload_json) {
                Some(s) => s,
                None => return Ok(()),
            }
        }
    };
    // The agent that ran this hook (still alive, unlike this short process) vouches for it.
    let pid = {
        use sysinfo::{Pid, ProcessRefreshKind, ProcessesToUpdate, System};
        let me = Pid::from_u32(std::process::id());
        let mut sys = System::new();
        sys.refresh_processes_specifics(ProcessesToUpdate::Some(&[me]), true, ProcessRefreshKind::nothing());
        sys.process(me).and_then(|p| p.parent()).map(|p| p.as_u32()).unwrap_or(0)
    };
    let token = std::env::var("SESHI_PANE_TOKEN").unwrap_or_default();
    let message = hook_message(HookOrigin { term, agent: agent.to_string(), pid, token }, &payload_json, status);
    block_on(async move {
        tokio::time::timeout(Duration::from_secs(2), async {
            let (_r, mut w) = ipc::open(false).await?;
            ipc::send(&mut w, &message).await?;
            Ok::<_, anyhow::Error>(())
        })
        .await?
    })
}

struct HookOrigin {
    term: TermId,
    agent: String,
    pid: u32,
    token: String,
}

/// Claude's hook payloads carry no task for a subagent, but it writes the description the parent
/// gave it next to the subagent's transcript. It may not exist yet at SubagentStart; a later
/// hook from inside the subagent finds it.
fn subagent_description(agent: &str, transcript: Option<&std::path::Path>, agent_id: Option<&str>) -> Option<String> {
    if agent != "claude" {
        return None;
    }
    let transcript = transcript?;
    let meta = if transcript.file_stem()?.to_string_lossy().starts_with("agent-") {
        transcript.with_extension("meta.json")
    } else {
        let id = agent_id?.trim_start_matches("agent-");
        transcript.with_extension("").join("subagents").join(format!("agent-{id}.meta.json"))
    };
    let meta: Value = serde_json::from_str(&std::fs::read_to_string(meta).ok()?).ok()?;
    meta.get("description")?.as_str().map(str::trim).filter(|d| !d.is_empty()).map(str::to_string)
}

fn hook_message(origin: HookOrigin, payload_json: &Value, status: HookStatus) -> ClientMsg {
    let field = |names: &[&str]| names.iter().find_map(|n| payload_json.get(*n)?.as_str().map(str::to_string));
    let session = field(&["session_id", "thread-id", "thread_id", "conversation_id"]);
    let cwd = field(&["cwd"]).map(PathBuf::from);
    let mut event = field(&["hook_event_name", "type"]).unwrap_or_default();
    let transcript = field(&["transcript_path"]).map(PathBuf::from);
    // A hook fired inside a subagent (it has an agent id, or its own agent-… transcript) is
    // news about that subagent, not about the session: it keeps the subagent listed as
    // running, and only a question it asks changes the session's state. Its transcript's
    // name, model and last words aren't the session's.
    let tracks_helpers = matches!(origin.agent.as_str(), "claude" | "codex" | "qwen" | "kimi" | "mastracode" | "opencode");
    let lifecycle = tracks_helpers && matches!(event.as_str(), "SubagentStart" | "SubagentStop" | "SubagentEnd");
    let inner_id = tracks_helpers.then(|| field(&["agent_id"]).or_else(|| {
        transcript.as_deref().and_then(|p| p.file_stem()).map(|s| s.to_string_lossy().into_owned()).filter(|s| s.starts_with("agent-"))
    })).flatten();
    let inner = !lifecycle && inner_id.is_some();
    let status = if inner && status != HookStatus::Blocked { HookStatus::Same } else { status };
    let subagent = (lifecycle || inner).then(|| Subagent {
        id: field(&["agent_id", "subagent_id", "tool_use_id"]).or(inner_id).unwrap_or_default(),
        kind: field(&["agent_type", "subagent_type", "agent_name"]).unwrap_or_else(|| "subagent".into()),
        start: !matches!(event.as_str(), "SubagentStop" | "SubagentEnd"),
        reported_at: std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_secs(),
        task: field(&["task", "description", "prompt"]).or_else(|| subagent_description(&origin.agent, transcript.as_deref(), field(&["agent_id"]).as_deref())).map(|task| one_line(&task, 240)),
        model: field(&["model", "model_id"]),
        effort: field(&["effort", "reasoning_effort"]),
        activity: field(&["tool_name"]).map(|tool| format!("{event}: {tool}")).or_else(|| Some(event.clone())),
    });
    let parent_facts = !inner;
    let turn_facts = parent_facts && !lifecycle;
    let prompt = field(&["prompt"]).filter(|_| turn_facts).map(|p| one_line(&p, 160));
    let transcript = transcript.filter(|_| turn_facts);
    let (model, name) = transcript.as_deref().map(transcript_facts).unwrap_or_default();
    if event == "Notification"
        && let Some(kind) = field(&["notification_type"])
    {
        event = format!("Notification:{kind}");
    }
    let said = field(&["last_assistant_message", "last-assistant-message"])
        .or_else(|| transcript.as_deref().and_then(last_assistant_text))
        .map(|s| s.trim().chars().take(2000).collect::<String>())
        .filter(|s| !s.is_empty() && turn_facts);
    let session = session.filter(|_| parent_facts);
    let cwd = cwd.filter(|_| parent_facts);
    let HookOrigin { term, agent, pid, token } = origin;
    ClientMsg::Hook { term, agent, status, session, cwd, prompt, said, subagent, event, pid, token, transcript, model, name }
}

/// Gemini CLI's hook events (its settings.json takes Claude's shape, with its own names).
const GEMINI_EVENTS: &[(&str, Option<&str>)] = &[
    ("BeforeAgent", None),
    ("BeforeTool", None),
    ("AfterTool", None),
    ("Notification", None),
    ("AfterAgent", None),
    ("SessionStart", None),
    ("SessionEnd", None),
];

/// A tool's settings.json under the home folder (`.gemini`, `.qwen`), or `$<env>/…`.
fn home_settings(env: &str, dir: &str) -> PathBuf {
    std::env::var_os(env)
        .map(|h| PathBuf::from(h).join(dir))
        .unwrap_or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(dir)).unwrap_or_default())
        .join("settings.json")
}

/// opencode has no command hooks: a plugin hands each event to `seshi hook opencode`.
fn opencode_plugin(uninstall: bool) -> Result<PathBuf> {
    let exe = this_exe()?;
    integrations::install_opencode(&exe, uninstall)
}

const CLAUDE_EVENTS: &[(&str, Option<&str>)] = &[
    ("UserPromptSubmit", None),
    ("PreToolUse", None),
    ("PostToolUse", None),
    ("PermissionRequest", None),
    ("Notification", None),
    ("Stop", None),
    ("SubagentStart", None),
    ("SubagentStop", None),
    ("SessionStart", None),
    ("SessionEnd", None),
];

/// A hook group seshi put there: tagged, or (from installs before the tag) one that runs
/// `seshi hook …` under any of the app's names.
fn is_ours(group: &Value) -> bool {
    ["_seshi", "_hydra", "_drover"].iter().any(|tag| group.get(*tag).and_then(Value::as_bool) == Some(true))
        || group.get("hooks").and_then(Value::as_array).into_iter().flatten().filter_map(|h| h.get("command").and_then(Value::as_str)).any(runs_our_hook)
}

/// A hook command split into the program it starts (its first word, or the quoted path)
/// and its arguments.
fn split_command(cmd: &str) -> (&str, &str) {
    let c = cmd.trim_start();
    let (prog, args) = match c.strip_prefix('"') {
        Some(rest) => rest.split_once('"').unwrap_or((rest, "")),
        None => c.split_once(char::is_whitespace).unwrap_or((c, "")),
    };
    (prog, args.trim_start())
}

fn program_of(cmd: &str) -> &str {
    split_command(cmd).0
}

/// `"<…/seshi>" hook claude`, or the same from a hydra or drover install.
fn runs_our_hook(cmd: &str) -> bool {
    let (prog, args) = split_command(cmd);
    let stem = std::path::Path::new(prog).file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
    matches!(stem.as_str(), "seshi" | "hydra" | "drover") && args.starts_with("hook ")
}

/// Hook commands in Claude settings `root` whose program isn't there (a removed install, a
/// moved file): Claude reports "hook error" on every turn they run in. Paths are checked
/// with `exists`; a bare name only when it's one of the app's old names (`on_path`).
fn broken_hooks_in(root: &Value, exists: impl Fn(&std::path::Path) -> bool, on_path: impl Fn(&str) -> bool) -> Vec<String> {
    let mut out: Vec<String> = root
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|h| h.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter_map(|g| g.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|h| h.get("command").and_then(Value::as_str))
        .filter(|cmd| {
            let prog = program_of(cmd);
            if prog.contains('$') || prog.starts_with('~') {
                return false;
            }
            if prog.contains('/') || prog.contains('\\') {
                !exists(std::path::Path::new(prog))
            } else {
                matches!(prog, "seshi" | "hydra" | "drover") && !on_path(prog)
            }
        })
        .map(String::from)
        .collect();
    out.sort();
    out.dedup();
    out
}

/// Where Claude Code keeps its MCP servers (yours, for every project).
fn claude_json() -> PathBuf {
    directories::BaseDirs::new().map(|d| d.home_dir().join(".claude.json")).unwrap_or_default()
}

/// The MCP servers in Claude's `root` that are this app under an old name: Claude shows the
/// name on every call, and the program behind it is an install that no longer updates.
fn old_mcp_names(root: &Value) -> Vec<String> {
    let Some(servers) = root.get("mcpServers").and_then(Value::as_object) else { return Vec::new() };
    servers
        .iter()
        .filter(|(_, server)| {
            let prog = server.get("command").and_then(Value::as_str).unwrap_or_default();
            let stem = std::path::Path::new(prog).file_stem().map(|s| s.to_string_lossy().to_ascii_lowercase()).unwrap_or_default();
            let serves = server.get("args").and_then(Value::as_array).is_some_and(|a| a.iter().any(|x| x.as_str() == Some("mcp")));
            matches!(stem.as_str(), "hydra" | "drover") && serves
        })
        .map(|(name, _)| name.clone())
        .collect()
}

/// Runs Claude Code's own CLI (a `.cmd` shim on Windows).
fn run_claude(args: &[&str]) -> std::io::Result<std::process::ExitStatus> {
    std::process::Command::new(if cfg!(windows) { "claude.cmd" } else { "claude" }).args(args).status().or_else(|_| std::process::Command::new("claude").args(args).status())
}

/// The Claude settings files whose hooks run here: yours, and this folder's project ones.
fn claude_settings_files() -> Vec<PathBuf> {
    let user = claude_settings();
    let mut files = vec![user.with_file_name("settings.local.json"), user];
    if let Ok(here) = std::env::current_dir() {
        files.push(here.join(".claude").join("settings.json"));
        files.push(here.join(".claude").join("settings.local.json"));
    }
    files
}

/// Claude Code's settings file, where its hooks live.
fn claude_settings() -> PathBuf {
    std::env::var_os("CLAUDE_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".claude")).unwrap_or_default())
        .join("settings.json")
}

/// This seshi, as hook commands name it.
fn this_exe() -> Result<String> {
    Ok(std::env::current_exe()?.to_string_lossy().replace('\\', "/"))
}

/// Context, cost and plan limits from the JSON Claude Code hands its status line.
pub(crate) fn statusline_facts(json: &str) -> (Usage, Vec<Limit>) {
    let Ok(v) = serde_json::from_str::<serde_json::Value>(json) else { return (Usage::default(), Vec::new()) };
    let usage = Usage { context: v["context_window"]["used_percentage"].as_f64().map(|p| p as f32), cost: v["cost"]["total_cost_usd"].as_f64() };
    let limits = [("five_hour", "5h"), ("seven_day", "week")]
        .iter()
        .filter_map(|(k, name)| {
            let l = &v["rate_limits"][k];
            Some(Limit { name: name.to_string(), used: l["used_percentage"].as_f64()? as f32, resets_at: l["resets_at"].as_u64()? })
        })
        .collect();
    (usage, limits)
}

/// Where the status line you had before seshi's is kept (seshi's runs it after its own).
fn statusline_before() -> PathBuf {
    crate::config::data_dir().join("claude-statusline.txt")
}

/// Whether a status line command is seshi's (or the hydra's it was renamed from).
fn is_our_statusline(cmd: &str) -> bool {
    (cmd.contains("seshi") || cmd.contains("hydra")) && cmd.trim_end().ends_with(" statusline")
}

/// Make Claude's status line seshi's (or with `uninstall`, put yours back). Yours keeps
/// showing: seshi's runs it with the same input and prints what it prints.
fn statusline_setting(root: &mut Value, exe: &str, uninstall: bool, kept: &std::path::Path) -> Result<()> {
    let cur = root.get("statusLine").and_then(|s| s.get("command")).and_then(Value::as_str).map(String::from);
    let ours = cur.as_deref().is_some_and(is_our_statusline);
    let before = std::fs::read_to_string(kept).unwrap_or_default();
    let obj = root.as_object_mut().ok_or_else(|| anyhow!("settings.json is not an object"))?;
    if uninstall {
        if ours {
            match before.trim() {
                "" => {
                    obj.remove("statusLine");
                }
                b => obj["statusLine"]["command"] = json!(b),
            }
        }
        let _ = std::fs::remove_file(kept);
        return Ok(());
    }
    if !ours {
        if let Some(dir) = kept.parent() {
            std::fs::create_dir_all(dir)?;
        }
        std::fs::write(kept, cur.unwrap_or_default())?;
    }
    let line = obj.entry("statusLine").or_insert_with(|| json!({ "type": "command" }));
    if !line.is_object() {
        *line = json!({ "type": "command" });
    }
    line["command"] = json!(format!("\"{exe}\" statusline"));
    Ok(())
}

/// `seshi statusline`, Claude Code's status line command: hands what the session has used
/// to seshi (inside a seshi pane), then shows the status line you had before, if any.
pub fn statusline() {
    use std::io::Write;
    let mut input = String::new();
    let _ = std::io::stdin().read_to_string(&mut input);
    let term = std::env::var("SESHI_TERM_ID").ok().and_then(|s| s.parse::<TermId>().ok());
    if let (Some(term), Ok(token)) = (term, std::env::var("SESHI_PANE_TOKEN")) {
        let (usage, limits) = statusline_facts(&input);
        let msg = ClientMsg::Usage { term, token, usage, limits };
        // Never hold up Claude's screen for it; a report that doesn't get there is skipped.
        let sent = block_on(async {
            tokio::time::timeout(Duration::from_millis(800), async {
                let (_r, mut w) = ipc::open(false).await?;
                ipc::send(&mut w, &msg).await
            })
            .await
            .unwrap_or_else(|_| Err(anyhow!("timed out")))
        });
        if let Err(e) = sent {
            tracing::debug!("status line report: {e:#}");
        }
    }
    let before = std::fs::read_to_string(statusline_before()).unwrap_or_default();
    let before = before.trim();
    if before.is_empty() || is_our_statusline(before) {
        return;
    }
    let mut cmd = shell_line(before);
    cmd.stdin(std::process::Stdio::piped()).stdout(std::process::Stdio::inherit()).stderr(std::process::Stdio::null());
    if let Ok(mut child) = crate::proc::quiet(&mut cmd).spawn() {
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(input.as_bytes());
        }
        let _ = child.wait();
    }
}

/// A command line run the way Claude Code runs its own: by bash (Git Bash on Windows, where
/// cmd stands in when there's none).
fn shell_line(line: &str) -> std::process::Command {
    if cfg!(windows) {
        let bash = std::env::var_os("CLAUDE_CODE_GIT_BASH_PATH")
            .map(PathBuf::from)
            .or_else(|| ["C:/Program Files/Git/bin/bash.exe", "C:/Program Files (x86)/Git/bin/bash.exe"].iter().map(PathBuf::from).find(|p| p.is_file()));
        match bash {
            Some(b) => {
                let mut c = std::process::Command::new(b);
                c.args(["-c", line]);
                c
            }
            None => {
                let mut c = std::process::Command::new("cmd");
                c.args(["/C", line]);
                c
            }
        }
    } else {
        let mut c = std::process::Command::new("sh");
        c.args(["-c", line]);
        c
    }
}

/// Add (or with `uninstall`, remove) seshi's hooks in Claude Code's settings.
fn claude_hooks(uninstall: bool) -> Result<PathBuf> {
    json_hooks(&claude_settings(), "claude", CLAUDE_EVENTS, 5, uninstall)
}

/// Add (or remove) seshi's hooks in a Claude-style settings.json (Claude Code, Gemini CLI,
/// Qwen Code): one tagged group per event running `seshi hook <agent>`, `timeout` in the
/// tool's own unit. The rest of the file is kept, with a backup beside it.
fn json_hooks(path: &std::path::Path, agent: &str, events: &[(&str, Option<&str>)], timeout: u64, uninstall: bool) -> Result<PathBuf> {
    let exe = this_exe()?;
    let path = path.to_path_buf();
    let mut root: Value = match std::fs::read_to_string(&path) {
        Ok(s) if s.trim().is_empty() => json!({}),
        Ok(s) => serde_json::from_str(&s).with_context(|| format!("{} isn't valid JSON; not touching it", path.display()))?,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => json!({}),
        Err(e) => return Err(e.into()),
    };
    if path.exists() {
        std::fs::copy(&path, path.with_extension("json.seshi-bak"))?;
    }
    let hooks = root
        .as_object_mut()
        .ok_or_else(|| anyhow!("settings.json is not an object"))?
        .entry("hooks")
        .or_insert_with(|| json!({}));
    let hooks = hooks.as_object_mut().ok_or_else(|| anyhow!("`hooks` is not an object"))?;
    for (event, matcher) in events {
        let groups = hooks.entry(*event).or_insert_with(|| json!([]));
        let Some(arr) = groups.as_array_mut() else { continue };
        arr.retain(|g| !is_ours(g));
        if !uninstall {
            let mut g = json!({
                "_seshi": true,
                "hooks": [{ "type": "command", "command": format!("\"{exe}\" hook {agent}"), "timeout": timeout }],
            });
            if let Some(m) = matcher {
                g["matcher"] = json!(m);
            }
            arr.push(g);
        }
    }
    hooks.retain(|_, v| v.as_array().is_none_or(|a| !a.is_empty()));
    if agent == "claude" {
        statusline_setting(&mut root, &exe, uninstall, &statusline_before())?;
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::config::write_atomic(&path, serde_json::to_string_pretty(&root)? + "\n")?;
    Ok(path)
}

/// The commands seshi's Claude hooks run that aren't this seshi (an older install, a copy
/// that moved). Empty when they're right, or when there are none.
pub fn stale_claude_hooks() -> Vec<String> {
    let (Ok(exe), Ok(text)) = (this_exe(), std::fs::read_to_string(claude_settings())) else { return Vec::new() };
    let Ok(root) = serde_json::from_str::<Value>(&text) else { return Vec::new() };
    stale_in(&root, &format!("\"{exe}\" hook claude"))
}

/// Seshi's Claude hooks are in and run this seshi (the check `seshi doctor` makes). Reads a
/// file: call it off the UI thread.
pub fn claude_status_hooks_installed() -> bool {
    std::fs::read_to_string(claude_settings()).is_ok_and(|s| s.contains("hook claude")) && stale_claude_hooks().is_empty()
}

/// Seshi's hook commands in Claude settings `root` other than `want`.
fn stale_in(root: &Value, want: &str) -> Vec<String> {
    let mut stale: Vec<String> = root
        .get("hooks")
        .and_then(Value::as_object)
        .into_iter()
        .flat_map(|h| h.values())
        .filter_map(Value::as_array)
        .flatten()
        .filter(|g| is_ours(g))
        .filter_map(|g| g.get("hooks").and_then(Value::as_array))
        .flatten()
        .filter_map(|h| h.get("command").and_then(Value::as_str))
        .filter(|c| *c != want)
        .map(String::from)
        .collect();
    stale.sort();
    stale.dedup();
    stale
}

/// Seshi's Claude hooks are in, but not its status line (installed before there was one).
fn needs_statusline() -> bool {
    let Ok(text) = std::fs::read_to_string(claude_settings()) else { return false };
    let Ok(root) = serde_json::from_str::<Value>(&text) else { return false };
    let hooked = root.get("hooks").and_then(Value::as_object).is_some_and(|h| h.values().filter_map(Value::as_array).flatten().any(is_ours));
    let cmd = root.get("statusLine").and_then(|s| s.get("command")).and_then(Value::as_str).unwrap_or_default();
    hooked && !is_our_statusline(cmd)
}

/// Point seshi's Claude hooks at this seshi if they name another one. A hook from a
/// different version can't talk to this server, and hooks fail silently by design, so
/// every agent would look idle. Run when the server starts.
pub fn refresh_claude_hooks() {
    // A side server (SESHI_SOCKET) or a build in a source checkout mustn't take your
    // hooks from the seshi you use.
    let side = std::env::var("SESHI_SOCKET").is_ok_and(|s| s != "default");
    let dev_build = this_exe().is_ok_and(|e| e.contains("/target/debug/") || e.contains("/target/release/"));
    if side || dev_build {
        return;
    }
    if !stale_claude_hooks().is_empty() || needs_statusline() {
        match claude_hooks(false) {
            Ok(p) => tracing::info!("pointed seshi's Claude hooks in {} at this seshi", p.display()),
            Err(e) => tracing::warn!("couldn't update seshi's Claude hooks: {e:#}"),
        }
    }
}

/// Codex's config file (`$CODEX_HOME/config.toml`, else `~/.codex/config.toml`).
fn codex_config() -> PathBuf {
    std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".codex")).unwrap_or_default())
        .join("config.toml")
}

enum CodexNotify {
    Set,
    Removed,
    /// Someone else's notify is there (shown as written).
    Theirs(String),
}

/// Point Codex's top-level `notify` at seshi (or take seshi's out), keeping the rest of the
/// file. Another program's notify is left alone.
fn codex_notify(path: &std::path::Path, exe: &str, uninstall: bool) -> Result<CodexNotify> {
    let text = match std::fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => String::new(),
        Err(e) => return Err(e.into()),
    };
    let mut doc: toml_edit::DocumentMut = text.parse().with_context(|| format!("{} has a syntax error; not touching it", path.display()))?;
    let ours = |v: &toml_edit::Item| v.as_array().is_some_and(|a| a.iter().any(|x| x.as_str() == Some("hook")) && a.iter().any(|x| x.as_str().is_some_and(|s| s.contains("seshi") || s.contains("hydra"))));
    if let Some(cur) = doc.get("notify")
        && !ours(cur)
    {
        return Ok(CodexNotify::Theirs(cur.to_string().trim().to_string()));
    }
    if uninstall {
        doc.remove("notify");
    } else {
        let mut a = toml_edit::Array::new();
        for part in [exe, "hook", "codex"] {
            a.push(part);
        }
        // Top-level keys go before the tables.
        doc.insert("notify", toml_edit::value(a));
    }
    if path.exists() {
        std::fs::copy(path, path.with_extension("toml.seshi-bak"))?;
    }
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    crate::config::write_atomic(path, doc.to_string())?;
    Ok(if uninstall { CodexNotify::Removed } else { CodexNotify::Set })
}

pub(super) fn install_codex_status(exe: &str, uninstall: bool) -> Result<PathBuf> {
    let path = codex_config();
    integrations::codex_hooks_at(&path, exe, uninstall)?;
    match codex_notify(&path, exe, uninstall)? {
        CodexNotify::Theirs(other) => tracing::info!("left Codex's existing notify command untouched: {other}"),
        CodexNotify::Set | CodexNotify::Removed => {}
    }
    Ok(path)
}

pub fn integrate(agent: &str, uninstall: bool) -> Result<()> {
    let exe = this_exe()?;
    match agent {
        "claude" => {
            let path = claude_hooks(uninstall)?;
            if uninstall {
                println!("removed seshi hooks from {}", path.display());
            } else {
                println!("installed seshi hooks into {}", path.display());
                println!("they only act inside seshi panes (SESHI_TERM_ID), so other terminals are unaffected.");
            }
            Ok(())
        }
        "mcp" => {
            // Claude Code: register for every project (user scope).
            let old = std::fs::read_to_string(claude_json()).ok().and_then(|s| serde_json::from_str::<Value>(&s).ok()).map(|root| old_mcp_names(&root)).unwrap_or_default();
            for name in old {
                match run_claude(&["mcp", "remove", "--scope", "user", &name]) {
                    Ok(s) if s.success() => println!("removed the old `{name}` MCP server from Claude Code"),
                    _ => println!("Claude Code: run  claude mcp remove --scope user {name}"),
                }
            }
            match run_claude(&["mcp", "add", "--scope", "user", "seshi", "--", exe.as_str(), "mcp"]) {
                Ok(s) if s.success() => println!("added the seshi MCP server to Claude Code (all projects)"),
                _ => println!("Claude Code: run  claude mcp add --scope user seshi -- \"{exe}\" mcp"),
            }
            println!("\nCodex: add to ~/.codex/config.toml\n\n[mcp_servers.seshi]\ncommand = \"{exe}\"\nargs = [\"mcp\"]\n");
            println!("Agents can then list, read, message and start sessions. Whether they may approve");
            println!("prompts is up to you: seshi Settings → Agents (never, by default).");
            Ok(())
        }
        "gemini" | "qwen" => {
            let (path, events, timeout) = if agent == "gemini" {
                // Gemini counts its timeout in milliseconds.
                (home_settings("GEMINI_CLI_HOME", ".gemini"), GEMINI_EVENTS, 5000)
            } else {
                // Qwen Code copies Claude's events (and seconds).
                (home_settings("QWEN_HOME", ".qwen"), CLAUDE_EVENTS, 10)
            };
            let path = json_hooks(&path, agent, events, timeout, uninstall)?;
            println!("{} seshi hooks {} {}", if uninstall { "removed" } else { "installed" }, if uninstall { "from" } else { "into" }, path.display());
            Ok(())
        }
        "opencode" => {
            let path = opencode_plugin(uninstall)?;
            println!("{} seshi's opencode plugin: {}", if uninstall { "removed" } else { "wrote" }, path.display());
            Ok(())
        }
        "pi" | "omp" | "mastracode" | "kilo" | "kimi" => {
            let path = match agent {
                "pi" => integrations::install_pi(&exe, uninstall)?,
                "omp" => integrations::install_omp(&exe, uninstall)?,
                "mastracode" => integrations::install_mastracode(&exe, uninstall)?,
                "kilo" => integrations::install_kilo(&exe, uninstall)?,
                "kimi" => integrations::install_kimi(&exe, uninstall)?,
                _ => unreachable!(),
            };
            println!("{} seshi's {agent} status integration: {}", if uninstall { "removed" } else { "wrote" }, path.display());
            println!("Restart {agent} to load it. Status follows the session, including z.ai and DeepSeek models.");
            Ok(())
        }
        "codex" => {
            let path = install_codex_status(&exe, uninstall)?;
            println!("{} Seshi's Codex status hooks in {}", if uninstall { "removed" } else { "installed" }, path.display());
            println!("Live subagent events are tracked; any other Codex notify command is preserved.");
            Ok(())
        }
        other => bail!("no integration for `{other}`; any agent can call `seshi hook {other} --status <working|blocked|done|idle>`"),
    }
}

/// Debugging: the colours a pane's program is drawing (rows with a background colour).
pub fn debug_colors(pane: TermId) -> Result<()> {
    block_on(async move {
        let (mut r, _w) = ipc::open(true).await.context("couldn't reach the seshi server")?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(3);
        while let Ok(Ok(Some(msg))) = tokio::time::timeout_at(deadline, ipc::recv_server(&mut r)).await {
            if let ServerMsg::Replay { term, cols, rows, data } = msg
                && term == pane
            {
                let mut p = vt100::Parser::new(rows, cols, 0);
                p.process(&data);
                let s = p.screen();
                println!("alternate screen: {}  mouse: {:?} / {:?}", s.alternate_screen(), s.mouse_protocol_mode(), s.mouse_protocol_encoding());
                let mut h = vt100::Parser::new(rows, cols, 100_000);
                h.process(&data);
                h.screen_mut().set_scrollback(usize::MAX);
                println!("history lines: {} (replay {} bytes)", h.screen().scrollback(), data.len());
                for row in 0..rows {
                    let mut seen: Vec<String> = Vec::new();
                    for col in 0..cols {
                        let Some(c) = s.cell(row, col) else { continue };
                        if c.bgcolor() == vt100::Color::Default && !c.inverse() {
                            continue;
                        }
                        let d = format!("fg={:?} bg={:?} inv={} dim={}", c.fgcolor(), c.bgcolor(), c.inverse(), c.dim());
                        if !seen.contains(&d) {
                            seen.push(d);
                        }
                    }
                    if !seen.is_empty() {
                        let text: String = s.rows(0, cols).nth(row as usize).unwrap_or_default().chars().take(40).collect();
                        println!("row {row:>3} {text:<40} {}", seen.join(" | "));
                    }
                }
                return Ok(());
            }
        }
        anyhow::bail!("no pane {pane}")
    })
}

/// `seshi allow`: show the repo's hook commands and let them run from now on.
/// A `~` at the start of a value is your home folder.
fn home_in(value: &str) -> String {
    match (value.strip_prefix('~'), directories::BaseDirs::new()) {
        (Some(rest), Some(d)) => d.home_dir().join(rest.trim_start_matches(['/', '\\'])).to_string_lossy().into_owned(),
        _ => value.to_string(),
    }
}

/// The command line `seshi account` runs for `command`: the program as typed, each argument
/// quoted again for the shell that will run it.
pub fn account_line(cfg: &crate::config::Config, command: &[String]) -> Option<String> {
    let (program, args) = command.split_first()?;
    Some(std::iter::once(program.clone()).chain(args.iter().map(|a| cfg.quote_for_shell(a))).collect::<Vec<_>>().join(" "))
}

/// `seshi account <name> -- <command>`: run it with that account's environment, in this
/// terminal, and leave with its exit code.
pub fn account(name: &str, command: &[String]) -> Result<()> {
    let (cfg, _) = crate::config::Config::load_or_default();
    let Some(a) = cfg.account(name) else {
        let known: Vec<&str> = cfg.quick.agents.iter().filter(|a| !a.env.is_empty()).map(|a| a.name.as_str()).collect();
        let known = if known.is_empty() { "none yet: give a [[quick.agents]] entry an env".to_string() } else { known.join(", ") };
        anyhow::bail!("no account named \"{name}\" (accounts: {known})");
    };
    let Some(line) = account_line(&cfg, command) else {
        anyhow::bail!("give the command to run: seshi account \"{name}\" -- claude");
    };
    let status = crate::proc::shell(&cfg.shell_command(), &line, true).envs(a.env.iter().map(|(k, v)| (k.clone(), home_in(v)))).status()?;
    std::process::exit(status.code().unwrap_or(1));
}

pub fn allow(dir: Option<std::path::PathBuf>) -> Result<()> {
    let dir = match dir {
        Some(d) => d,
        None => std::env::current_dir()?,
    };
    let p = crate::project::load(&dir);
    let cmds = [p.hooks.on_create.as_str(), p.hooks.on_remove.as_str()];
    if cmds.iter().all(|c| c.trim().is_empty()) {
        println!("no hooks in {} here; nothing to allow", crate::project::FILE);
        return Ok(());
    }
    crate::project::allow(&dir, &cmds)?;
    for (what, c) in [("on_create", cmds[0]), ("on_remove", cmds[1])] {
        if !c.trim().is_empty() {
            println!("allowed {what}: {c}");
        }
    }
    println!("(if the commands change, they won't run until you allow them again)");
    Ok(())
}

#[cfg(test)]
mod tests {
    fn check(state: CheckState) -> CheckInfo {
        CheckInfo { command: "npm test".into(), state, exit_code: Some(1), first_failing_line: "FAIL footer.test.ts".into(), ..Default::default() }
    }

    #[test]
    fn a_waiter_stays_blocked_while_the_check_runs() {
        let running = check(CheckState::Checking);
        assert_eq!(turn_over(Status::Done, "all done", std::slice::from_ref(&running), true, false), None);
        assert_eq!(turn_over(Status::Idle, "", std::slice::from_ref(&running), false, true), None, "not even after the quiet-turn grace");
        assert_eq!(turn_over(Status::Done, "", &[], true, false), Some(Waited::Turn(Status::Done, String::new(), None)));
    }

    #[test]
    fn a_waiter_gets_the_check_result_with_the_reply() {
        for state in [CheckState::Passed, CheckState::Failed, CheckState::CouldNotRun, CheckState::Overridden] {
            let info = check(state);
            assert_eq!(turn_over(Status::Done, "done", std::slice::from_ref(&info), true, false), Some(Waited::Turn(Status::Done, "done".into(), Some(Box::new(info)))));
        }
    }

    #[test]
    fn a_session_with_no_check_waits_as_before() {
        assert_eq!(turn_over(Status::Working, "", &[], false, true), None);
        assert_eq!(turn_over(Status::Done, "x", &[], false, false), None, "the turn never started");
        assert_eq!(turn_over(Status::Blocked, "q", &[], true, false), Some(Waited::Turn(Status::Blocked, "q".into(), None)));
        assert_eq!(turn_over(Status::None, "", &[], false, true), Some(Waited::Turn(Status::None, String::new(), None)));
    }

    fn named(name: &str, state: CheckState) -> CheckInfo {
        CheckInfo { name: name.into(), ..check(state) }
    }

    #[test]
    fn a_waiter_waits_until_no_checkpoint_is_running_or_waiting_its_turn() {
        let failed_first = [named("build", CheckState::Failed), named("test", CheckState::Checking)];
        assert_eq!(turn_over(Status::Done, "", &failed_first, true, false), None, "a failure does not end the wait while another runs");
        let pending = [named("build", CheckState::Passed), named("test", CheckState::Pending)];
        assert_eq!(turn_over(Status::Done, "", &pending, true, false), None);
        let settled = [named("build", CheckState::Failed), named("test", CheckState::Passed)];
        let Some(Waited::Turn(_, _, Some(headline))) = turn_over(Status::Done, "", &settled, true, false) else { panic!("the turn is over") };
        assert_eq!(headline.name, "build", "it ends on the worst checkpoint");
    }

    #[test]
    fn wait_exits_four_on_any_failed_checkpoint_and_zero_on_the_rest() {
        let exit = |states: &[CheckState]| {
            let checks: Vec<_> = states.iter().enumerate().map(|(i, &s)| named(&format!("c{i}"), s)).collect();
            let Some(Waited::Turn(s, _, headline)) = turn_over(Status::Done, "", &checks, true, false) else { panic!("over") };
            turn_exit(s, headline.as_deref())
        };
        assert_eq!(exit(&[CheckState::Passed, CheckState::Failed]), Some(EXIT_CHECK_FAILED));
        assert_eq!(exit(&[CheckState::Failed, CheckState::NeedsApproval]), Some(EXIT_CHECK_FAILED));
        for state in [CheckState::NeedsApproval, CheckState::Never, CheckState::CouldNotRun, CheckState::Stale, CheckState::Overridden, CheckState::Passed] {
            assert_eq!(exit(&[CheckState::Passed, state]), None, "{state:?}");
        }
        assert_eq!(turn_exit(Status::Blocked, None), Some(3));
    }

    #[test]
    fn the_new_states_read_in_the_v7_words() {
        let stale = CheckInfo {
            name: "test".into(),
            state: CheckState::Stale,
            changed_files: ["a", "b", "c"].map(|p| ChangedFile { kind: 'M', path: p.into() }).to_vec(),
            ..check(CheckState::Stale)
        };
        let words = |c: &CheckInfo| (check_note(c), check_line(c));
        assert_eq!(words(&stale), ("stale pass: test · 3 files changed".into(), "stale pass: test · 3 files changed".into()));
        let one = CheckInfo { changed_files: vec![ChangedFile { kind: 'A', path: "x".into() }], ..stale.clone() };
        assert_eq!(check_note(&one), "stale pass: test · 1 file changed");
        assert_eq!(words(&check(CheckState::Never)), ("not checked: never for this repo".into(), "not checked: never for this repo".into()));
        assert_eq!(words(&check(CheckState::NeedsApproval)), ("approve check: npm test".into(), "approve check: npm test".into()));
        assert_eq!(words(&check(CheckState::Pending)), ("pending".into(), "pending: npm test".into()));
    }

    #[test]
    fn ls_shows_the_headline_and_how_many_checkpoints() {
        let mut p = pane("Done", Some("claude"), None);
        p.checks = vec![named("build", CheckState::Passed), named("test", CheckState::Failed)];
        assert_eq!(agent_tag(&p).as_deref(), Some("claude: done · check failed (exit 1) · 2 checkpoints"));
        let v = pane_json(&p);
        assert_eq!(v["check"]["state"], "failed", "check stays the headline for scripts");
        let listed: Vec<_> = v["checks"].as_array().unwrap().iter().map(|c| (c["name"].as_str().unwrap(), c["state"].as_str().unwrap())).collect();
        assert_eq!(listed, [("build", "passed"), ("test", "failed")]);
    }

    #[test]
    fn the_result_reads_as_one_line() {
        assert_eq!(check_line(&check(CheckState::Passed)), "check passed: npm test");
        assert_eq!(check_line(&check(CheckState::Failed)), "check failed: npm test (exit 1): FAIL footer.test.ts");
        assert_eq!(check_line(&check(CheckState::CouldNotRun)), "check could not run: npm test");
        assert_eq!(check_line(&check(CheckState::Overridden)), "check failed, accepted: npm test (exit 1): FAIL footer.test.ts");
    }

    fn pane(status: &str, agent: Option<&str>, check: Option<CheckInfo>) -> TermInfo {
        let mut p: TermInfo = serde_json::from_value(json!({
            "id": 3, "cols": 80, "rows": 24, "title": "", "process": "node", "agent": agent,
            "status": status, "cwd": "/code", "summary": "", "name": "", "model": "", "said": "",
            "branch": null, "linked": false, "subagents": [], "root": null, "top": null,
            "since": 0, "asleep": false, "win32_input": false,
        }))
        .unwrap();
        p.checks = check.into_iter().collect();
        p
    }

    #[test]
    fn a_check_reads_beside_the_status() {
        let note = |state, exit| check_note(&CheckInfo { state, exit_code: exit, ..check(state) });
        assert_eq!(note(CheckState::Failed, Some(1)), "check failed (exit 1)");
        assert_eq!(note(CheckState::Failed, None), "check failed");
        assert_eq!(note(CheckState::Checking, None), "checking");
        assert_eq!(note(CheckState::Passed, Some(0)), "check passed");
        assert_eq!(note(CheckState::CouldNotRun, Some(127)), "check could not run");
        assert_eq!(note(CheckState::Overridden, Some(1)), "check overridden");
    }

    #[test]
    fn ls_shows_the_check_in_the_agent_tag_only_when_there_is_one() {
        let failed = pane("Done", Some("claude"), Some(check(CheckState::Failed)));
        assert_eq!(agent_tag(&failed).as_deref(), Some("claude: done · check failed (exit 1)"));
        assert_eq!(agent_tag(&pane("Done", Some("claude"), None)).as_deref(), Some("claude: done"));
        assert_eq!(agent_tag(&pane("Done", None, Some(check(CheckState::Failed)))), None, "a shell has no tag");
        let working = pane("Working", Some("claude"), Some(check(CheckState::Failed)));
        assert_eq!(agent_tag(&working).as_deref(), Some("claude: working"), "the last result is about an older turn");
    }

    #[test]
    fn ls_says_how_long_a_working_agent_has_been_quiet() {
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_secs();
        let mut p = pane("Working", Some("claude"), None);
        p.quiet_since = Some(now - 12 * 60);
        assert_eq!(agent_tag(&p).as_deref(), Some("claude: working · quiet 12m"));
        assert_eq!(pane_json(&p)["quiet_since"], json!(now - 12 * 60));
        p.status = Status::Done;
        assert_eq!(agent_tag(&p).as_deref(), Some("claude: done"), "only a working agent is quiet");
        assert!(pane_json(&p).get("quiet_since").is_none());
    }

    #[test]
    fn ls_json_gives_a_pane_with_a_check_a_check_object() {
        let mut info = check(CheckState::Overridden);
        (info.duration_ms, info.rerun_queued, info.overridden_at) = (14_200, true, Some(130));
        let v = pane_json(&pane("Done", Some("claude"), Some(info)));
        assert_eq!(
            v["check"],
            json!({
                "state": "overridden", "command": "npm test", "exit_code": 1, "duration_ms": 14_200,
                "first_failing_line": "FAIL footer.test.ts", "rerun_queued": true, "overridden_at": 130,
            })
        );
        assert_eq!(v["status"], "done", "the status field itself is unchanged");
    }

    #[test]
    fn ls_json_for_a_pane_with_no_check_is_what_it_was() {
        let v = pane_json(&pane("Done", Some("claude"), None));
        assert!(v.get("check").is_none());
        assert_eq!(
            v.to_string(),
            r#"{"agent":"claude","asleep":false,"bell":false,"cols":80,"cwd":"/code","id":3,"mem_mb":0,"model":"","name":"","process":"node","rows":24,"status":"done","status_why":"","title":"","win32_input":false}"#
        );
    }

    #[test]
    fn codex_status_setup_requires_root_thread_and_subagent_hooks_for_each_platform() {
        let linux = r#"
            [[hooks.SessionStart]]
            [[hooks.SessionStart.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex --status same'
            [[hooks.SubagentStart]]
            [[hooks.SubagentStart.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex'
            [[hooks.SubagentStop]]
            [[hooks.SubagentStop.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex'
        "#;
        let windows = r#"
            [[hooks.SessionStart]]
            [[hooks.SessionStart.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex --status same'
            commandWindows = '& "C:\\\\seshi.exe" hook codex --status same'
            [[hooks.SubagentStart]]
            [[hooks.SubagentStart.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex'
            commandWindows = '& "C:\\\\seshi.exe" hook codex'
            [[hooks.SubagentStop]]
            [[hooks.SubagentStop.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex'
            commandWindows = '& "C:\\\\seshi.exe" hook codex'
        "#;
        let missing_stop = r#"
            [[hooks.SessionStart]]
            [[hooks.SessionStart.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex --status same'
            [[hooks.SubagentStart]]
            [[hooks.SubagentStart.hooks]]
            type = "command"
            command = '"/usr/local/bin/seshi" hook codex'
        "#;

        assert!(super::codex_status_hooks_installed(linux, false));
        assert!(!super::codex_status_hooks_installed(linux, true));
        assert!(super::codex_status_hooks_installed(windows, true));
        assert!(!super::codex_status_hooks_installed(missing_stop, false));
        assert!(!super::codex_status_hooks_installed("[hooks\n", false));
    }

    #[test]
    fn hydras_status_line_keeps_yours() {
        let kept = std::env::temp_dir().join(format!("seshi-statusline-{}.txt", std::process::id()));
        let mut root = serde_json::json!({ "statusLine": { "type": "command", "command": "chm statusline", "padding": 1 } });
        super::statusline_setting(&mut root, "C:/x/seshi.exe", false, &kept).unwrap();
        assert_eq!(root["statusLine"]["command"], "\"C:/x/seshi.exe\" statusline");
        assert_eq!(root["statusLine"]["padding"], 1, "your settings for it stay");
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), "chm statusline", "and yours still runs after seshi's");
        super::statusline_setting(&mut root, "C:/y/seshi.exe", false, &kept).unwrap();
        assert_eq!(std::fs::read_to_string(&kept).unwrap(), "chm statusline", "installing again doesn't lose yours");
        super::statusline_setting(&mut root, "C:/y/seshi.exe", true, &kept).unwrap();
        assert_eq!(root["statusLine"]["command"], "chm statusline", "uninstalling puts yours back");
        let mut none = serde_json::json!({});
        super::statusline_setting(&mut none, "C:/x/seshi.exe", false, &kept).unwrap();
        super::statusline_setting(&mut none, "C:/x/seshi.exe", true, &kept).unwrap();
        assert!(none.get("statusLine").is_none(), "none before: none after");
    }

    #[test]
    fn claude_says_what_it_used_through_its_status_line() {
        let json = r#"{"cost":{"total_cost_usd":1.25},"context_window":{"context_window_size":200000,"used_percentage":42},
            "rate_limits":{"five_hour":{"used_percentage":23.5,"resets_at":1738425600},"seven_day":{"used_percentage":41.2,"resets_at":1738857600}}}"#;
        let (u, l) = super::statusline_facts(json);
        assert_eq!(u, crate::protocol::Usage { context: Some(42.0), cost: Some(1.25) });
        assert_eq!(l.iter().map(|x| (x.name.as_str(), x.used, x.resets_at)).collect::<Vec<_>>(), [("5h", 23.5, 1738425600), ("week", 41.2, 1738857600)]);
        let (u, l) = super::statusline_facts(r#"{"context_window":{"used_percentage":null}}"#);
        assert_eq!((u, l.len()), (crate::protocol::Usage::default(), 0), "early in a session, or not on a plan: nothing yet");
    }

    #[test]
    fn only_new_work_gets_a_worktree() {
        let w = |a: &[&str]| super::starts_new_work("claude", &a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        assert!(w(&[]) && w(&["fix the login page"]) && w(&["--model", "opus"]) && w(&["update the readme"]), "a new session or a task");
        assert!(!w(&["--continue"]) && !w(&["-r"]) && !w(&["resume"]), "an earlier conversation stays in its folder");
        assert!(!w(&["--version"]) && !w(&["-p", "what's 2+2"]) && !w(&["mcp", "list"]) && !w(&["update"]), "not work: help, one-shots, subcommands");
    }

    #[test]
    fn codex_config_and_profile_options_still_start_new_work() {
        let w = |a: &[&str]| super::starts_new_work("codex", &a.iter().map(|s| s.to_string()).collect::<Vec<_>>());
        for args in [vec![], vec!["-c", "model=\"example\""], vec!["-p", "feature"], vec!["--profile", "resume"],
            vec!["--config=model=\"example\"", "build login"], vec!["-pfeature"], vec!["--", "--resume"]] {
            assert!(w(&args), "new Codex work: {args:?}");
        }
        for args in [vec!["resume", "--last"], vec!["-m", "example", "resume", "--last"], vec!["fork", "--last"],
            vec!["--profile", "feature", "review"], vec!["--version"], vec!["mcp", "list"], vec!["--worktree"],
            vec!["-C", "somewhere"], vec!["--cd=somewhere"], vec!["-Csomewhere"], vec!["fix login", "--cd", "somewhere"]] {
            assert!(!w(&args), "Codex stays in the requested folder: {args:?}");
        }
    }

    #[test]
    fn a_finished_turn_is_done_even_when_its_transcript_has_background_work() {
        let file = std::env::temp_dir().join(format!("seshi-background-turn-{}.jsonl", std::process::id()));
        let transcript = [
            json!({"type":"user","message":{"content":"start the game server"}}),
            json!({"type":"user","message":{"content":[{"type":"tool_result","content":"Command running in background with ID: server1. Output is being written to: x"}]}}),
            json!({"type":"assistant","message":{"content":[{"type":"text","text":"The game server is ready."}]}}),
        ];
        std::fs::write(&file, transcript.iter().map(Value::to_string).collect::<Vec<_>>().join("\n")).unwrap();
        for event in ["Stop", "Notification"] {
            let payload = json!({"hook_event_name":event,"notification_type":"idle_prompt","transcript_path":file});
            let status = super::status_from_hook(&payload).unwrap();
            let source = super::HookOrigin { term: 1, agent: "claude".into(), pid: 0, token: String::new() };
            let ClientMsg::Hook { status, said, transcript, .. } = super::hook_message(source, &payload, status) else { panic!("expected hook report") };
            assert_eq!(status, HookStatus::Done, "{event} ends the turn while the server remains alive");
            assert_eq!(said.as_deref(), Some("The game server is ready."));
            assert_eq!(transcript.as_ref(), Some(&file));
        }
        std::fs::remove_file(file).unwrap();
    }

    #[test]
    fn claude_subagent_task_comes_from_its_meta_file() {
        let dir = std::env::temp_dir().join(format!("seshi-meta-{}", std::process::id()));
        let subagents = dir.join("sess").join("subagents");
        std::fs::create_dir_all(&subagents).unwrap();
        std::fs::write(subagents.join("agent-abc.meta.json"), r#"{"agentType":"general-purpose","description":"Debugging Go back test failure"}"#).unwrap();
        let parent = dir.join("sess.jsonl");
        let inner = subagents.join("agent-abc.jsonl");
        let want = Some("Debugging Go back test failure".to_string());
        assert_eq!(super::subagent_description("claude", Some(&parent), Some("abc")), want, "from SubagentStart");
        assert_eq!(super::subagent_description("claude", Some(&inner), None), want, "from a hook inside the subagent");
        assert_eq!(super::subagent_description("codex", Some(&inner), None), None);
        assert_eq!(super::subagent_description("claude", Some(&parent), Some("missing")), None);
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn gemini_qwen_and_opencode_events_mean_states() {
        use crate::protocol::HookStatus as H;
        let st = |v: serde_json::Value| super::status_from_hook(&v);
        assert_eq!(st(serde_json::json!({"hook_event_name": "BeforeAgent"})), Some(H::Working), "gemini: a turn starts");
        assert_eq!(st(serde_json::json!({"hook_event_name": "AfterAgent"})), Some(H::Done));
        assert_eq!(st(serde_json::json!({"hook_event_name": "Notification", "notification_type": "ToolPermission"})), Some(H::Blocked));
        assert_eq!(st(serde_json::json!({"hook_event_name": "Stop"})), Some(H::Done), "qwen speaks claude's events");
        assert_eq!(st(serde_json::json!({"type": "session.status", "properties": {"status": {"type": "busy"}}})), Some(H::Working), "opencode");
        assert_eq!(st(serde_json::json!({"type": "permission.asked", "properties": {}})), Some(H::Blocked));
        assert_eq!(st(serde_json::json!({"type": "session.idle", "properties": {}})), Some(H::Done));
        assert_eq!(st(serde_json::json!({"type": "question.replied"})), Some(H::Working));
        assert_eq!(st(serde_json::json!({"type": "question.rejected"})), Some(H::Working));
        assert_eq!(st(serde_json::json!({"type": "session.error"})), Some(H::Done));
        assert_eq!(st(serde_json::json!({"type": "agent-turn-complete"})), Some(H::Done), "codex unchanged");
    }

    #[test]
    fn native_helper_events_from_kimi_and_mastracode_reach_the_parent() {
        use crate::protocol::HookStatus as H;
        let start = json!({"hook_event_name":"SubagentStart","session_id":"root","agent_name":"Explore","task":"Audit account flow","model":"opus","reasoning_effort":"high"});
        let origin = super::HookOrigin { term: 9, agent: "kimi".into(), pid: 10, token: String::new() };
        let ClientMsg::Hook { subagent, status, .. } = super::hook_message(origin, &start, H::Working) else { panic!("expected hook") };
        assert_eq!(status, H::Working);
        assert!(subagent.as_ref().is_some_and(|event| event.id.is_empty() && event.kind == "Explore" && event.start && event.reported_at > 0));
        assert!(subagent.as_ref().is_some_and(|event| event.task.as_deref() == Some("Audit account flow") && event.model.as_deref() == Some("opus") && event.effort.as_deref() == Some("high")));

        let end = json!({"hook_event_name":"SubagentEnd","session_id":"root","agent_id":"child","agent_type":"review"});
        let origin = super::HookOrigin { term: 9, agent: "mastracode".into(), pid: 10, token: String::new() };
        let ClientMsg::Hook { subagent, status, .. } = super::hook_message(origin, &end, H::Same) else { panic!("expected hook") };
        assert_eq!(status, H::Same);
        assert!(subagent.as_ref().is_some_and(|event| event.id == "child" && event.kind == "review" && !event.start && event.reported_at > 0));
    }

    #[test]
    fn codex_native_helper_events_reach_the_owning_parent_session() {
        use crate::protocol::HookStatus as H;
        let payload = json!({
            "hook_event_name": "SubagentStart",
            "session_id": "parent-thread",
            "agent_id": "child-thread",
            "agent_type": "reviewer",
            "cwd": "/project",
            "last_assistant_message": "child result must not become the parent summary",
        });
        assert_eq!(super::status_from_hook(&payload), Some(H::Working));
        let origin = super::HookOrigin { term: 4, agent: "codex".into(), pid: 12, token: "pane-token".into() };
        let ClientMsg::Hook { session, cwd, subagent, status, said, .. } = super::hook_message(origin, &payload, H::Working) else { panic!("expected hook") };
        assert_eq!(status, H::Working);
        assert_eq!(session.as_deref(), Some("parent-thread"));
        assert_eq!(cwd.as_deref(), Some(std::path::Path::new("/project")));
        assert!(subagent.as_ref().is_some_and(|event| event.id == "child-thread" && event.kind == "reviewer" && event.start));
        assert!(said.is_none(), "helper output is not attributed to the parent conversation");

        let stop = json!({"hook_event_name":"SubagentStop","session_id":"parent-thread","agent_id":"child-thread","agent_type":"reviewer"});
        assert_eq!(super::status_from_hook(&stop), Some(H::Same));
        let origin = super::HookOrigin { term: 4, agent: "codex".into(), pid: 12, token: "pane-token".into() };
        let ClientMsg::Hook { session, subagent, .. } = super::hook_message(origin, &stop, H::Same) else { panic!("expected hook") };
        assert_eq!(session.as_deref(), Some("parent-thread"));
        assert!(subagent.as_ref().is_some_and(|event| event.id == "child-thread" && event.kind == "reviewer" && !event.start));
    }

    #[test]
    fn codex_session_start_records_thread_without_changing_agent_status() {
        use crate::protocol::HookStatus as H;
        let payload = json!({"hook_event_name":"SessionStart","session_id":"root-thread","cwd":"/project"});
        let origin = super::HookOrigin { term: 4, agent: "codex".into(), pid: 12, token: "pane-token".into() };
        let ClientMsg::Hook { session, cwd, status, event, .. } = super::hook_message(origin, &payload, H::Same) else { panic!("expected hook") };
        assert_eq!(session.as_deref(), Some("root-thread"));
        assert_eq!(cwd.as_deref(), Some(std::path::Path::new("/project")));
        assert_eq!(status, H::Same, "capturing the root thread must not mark a running Codex turn idle");
        assert_eq!(event, "SessionStart");
    }

    #[test]
    fn pi_native_events_map_to_status_with_evidence_and_no_conversation_payload() {
        use crate::protocol::HookStatus as H;
        for (event, expected) in [("pi.session_start", H::Idle), ("pi.agent_start", H::Working), ("pi.ui_prompt_start", H::Blocked), ("pi.ui_prompt_end", H::Working), ("pi.agent_settled", H::Done), ("pi.session_shutdown", H::Gone)] {
            let payload = json!({"hook_event_name":event,"cwd":"/project"});
            assert_eq!(super::status_from_hook(&payload), Some(expected));
            let origin = super::HookOrigin { term: 7, agent: "pi".into(), pid: 10, token: String::new() };
            let ClientMsg::Hook { event: evidence, prompt, said, .. } = super::hook_message(origin, &payload, expected) else { panic!("expected hook") };
            assert_eq!(evidence, event);
            assert!(prompt.is_none() && said.is_none());
        }
        let payload = json!({"type":"question.asked"});
        let origin = super::HookOrigin { term: 7, agent: "opencode".into(), pid: 10, token: String::new() };
        let ClientMsg::Hook { event, .. } = super::hook_message(origin, &payload, H::Blocked) else { panic!("expected hook") };
        assert_eq!(event, "question.asked");
    }

    #[test]
    fn codex_notify_is_set_and_others_kept() {
        let file = std::env::temp_dir().join(format!("seshi-codex-{}.toml", std::process::id()));
        std::fs::write(&file, "model = \"gpt-5\"\n\n[tui]\nnotifications = true\n").unwrap();
        assert!(matches!(super::codex_notify(&file, "/bin/seshi", false).unwrap(), super::CodexNotify::Set));
        let text = std::fs::read_to_string(&file).unwrap();
        assert!(text.contains(r#"notify = ["/bin/seshi", "hook", "codex"]"#) && text.contains("[tui]") && text.contains("gpt-5"), "{text}");
        assert!(text.find("notify") < text.find("[tui]"), "a top-level key, before the tables: {text}");
        // Run again with seshi elsewhere: replaced (it's ours).
        super::codex_notify(&file, "/usr/local/bin/seshi", false).unwrap();
        assert!(std::fs::read_to_string(&file).unwrap().contains("/usr/local/bin/seshi"));
        // Someone else's notify stays.
        std::fs::write(&file, "notify = [\"my-notifier\"]\n").unwrap();
        assert!(matches!(super::codex_notify(&file, "/bin/seshi", false).unwrap(), super::CodexNotify::Theirs(_)));
        assert!(std::fs::read_to_string(&file).unwrap().contains("my-notifier"));
        let _ = std::fs::remove_file(&file);
        let _ = std::fs::remove_file(file.with_extension("toml.seshi-bak"));
    }

    #[test]
    fn an_mcp_server_left_under_an_old_name_is_found() {
        let root = serde_json::json!({ "mcpServers": {
            "hydra": { "command": "C:/Programs/hydra/hydra.exe", "args": ["mcp"] },
            "seshi": { "command": "C:/Programs/seshi/seshi.exe", "args": ["mcp"] },
            "plane": { "command": "npx", "args": ["plane-mcp"] },
        }});
        assert_eq!(super::old_mcp_names(&root), vec!["hydra".to_string()], "only the old install's");
        assert!(super::old_mcp_names(&serde_json::json!({})).is_empty(), "no servers at all");
    }

    #[test]
    fn hooks_from_another_hydra_are_found() {
        let want = "\"/home/me/.local/bin/seshi\" hook claude";
        let settings = |cmd: &str| {
            serde_json::json!({ "hooks": {
                "Stop": [
                    { "_hydra": true, "hooks": [{ "type": "command", "command": cmd }] },
                    { "hooks": [{ "type": "command", "command": "someone-elses-hook" }] },
                ],
            }})
        };
        assert!(super::stale_in(&settings(want), want).is_empty(), "this seshi's hooks are fine; others' aren't ours to judge");
        let old = "\"/home/me/.cargo/bin/seshi\" hook claude";
        assert_eq!(super::stale_in(&settings(old), want), vec![old.to_string()]);
        // An old install's hook without the tag is still ours (so it gets repointed).
        let untagged = serde_json::json!({ "hooks": { "Stop": [{ "hooks": [{ "type": "command", "command": "/home/me/.local/bin/hydra hook claude" }] }] } });
        assert_eq!(super::stale_in(&untagged, want), vec!["/home/me/.local/bin/hydra hook claude".to_string()]);
        assert!(!super::runs_our_hook("hydra-lint check"), "only the app's own hook");
    }

    #[test]
    fn hooks_whose_program_is_gone_are_found() {
        let root = serde_json::json!({ "hooks": {
            "Stop": [{ "hooks": [
                { "type": "command", "command": "\"/home/me/.local/bin/hydra\" hook claude" },
                { "type": "command", "command": "/usr/bin/notify-send done" },
                { "type": "command", "command": "hydra hook claude" },
                { "type": "command", "command": "npx something" },
                { "type": "command", "command": "$HOME/bin/x" },
            ] }],
        }});
        let exists = |p: &std::path::Path| p == std::path::Path::new("/usr/bin/notify-send");
        let broken = super::broken_hooks_in(&root, exists, |_| false);
        assert_eq!(broken, vec!["\"/home/me/.local/bin/hydra\" hook claude".to_string(), "hydra hook claude".to_string()]);
    }

    #[test]
    fn model_and_name_from_a_transcript() {
        assert_eq!(super::short_model("claude-opus-4-5-20251101"), "opus 4.5");
        assert_eq!(super::short_model("claude-fable-5-1"), "fable 5.1");
        assert_eq!(super::short_model("gpt-5.5"), "gpt-5.5");
        let t = [
            r#"{"type":"assistant","message":{"model":"claude-sonnet-5-5","content":[]}}"#,
            r#"{"type":"ai-title","aiTitle":"Fix the login flow","sessionId":"s"}"#,
            r#"{"type":"user","message":{"content":"say \"model\" and \"customTitle\""}}"#,
        ]
        .join("\n");
        assert_eq!(super::transcript_facts_in(&t), (Some("sonnet 5.5".into()), Some("Fix the login flow".into())));
        let t = format!("{t}\n{}", r#"{"type":"custom-title","customTitle":"auth rewrite","sessionId":"s"}"#);
        assert_eq!(super::transcript_facts_in(&t).1.as_deref(), Some("auth rewrite"), "your /rename wins");
    }

    use super::*;


    #[test]
    fn claude_hook_mapping() {
        let s = |v: Value| status_from_hook(&v);
        assert_eq!(s(json!({"hook_event_name": "UserPromptSubmit"})), Some(HookStatus::Working));
        assert_eq!(s(json!({"hook_event_name": "Stop"})), Some(HookStatus::Done));
        assert_eq!(
            s(json!({"hook_event_name": "Notification", "notification_type": "permission_prompt"})),
            Some(HookStatus::Blocked)
        );
        assert_eq!(s(json!({"hook_event_name": "PreToolUse", "tool_name": "AskUserQuestion"})), Some(HookStatus::Blocked));
        assert_eq!(s(json!({"type": "agent-turn-complete"})), Some(HookStatus::Done));
        assert_eq!(s(json!({"hook_event_name": "Notification", "notification_type": "auth_success"})), None);
    }
}
