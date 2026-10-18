//! What agents use: context and cost (Claude reports them through its status line, Codex
//! writes them to its session files), plan limits, and saying "continue" once a limit
//! lifts.

use super::*;
use regex::Regex;
use std::path::Path;
use std::sync::LazyLock;

/// How often Codex's session files are read.
const CODEX_EVERY: Duration = Duration::from_secs(5);
/// How much of the end of a Codex session file is read for its latest numbers.
const CODEX_TAIL: u64 = 512 * 1024;
/// A limit with no known reset time: try again after this long (seconds).
const RETRY_LIMIT_AFTER: u64 = 30 * 60;
/// "continue" goes this long after the reset, so the limit has really lifted (seconds).
const AFTER_RESET: u64 = 60;
/// After saying "continue", the old limit line may still be on screen this long.
const CONTINUE_SETTLES: Duration = Duration::from_secs(120);
/// The rows at the bottom of an agent's screen where its limit line shows (above its
/// input box and footer).
const LIMIT_ROWS: u16 = 14;
/// A limit this used up (percent) is the one that stopped it.
const SPENT: f32 = 95.0;
/// How far back "today" reaches for what sessions cost (seconds).
const DAY: u64 = 24 * 60 * 60;

/// The line an agent shows when a plan limit stops it.
static LIMIT_HIT: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?i)(usage limit reached|you'?ve hit your (usage |session |weekly )?limit|you have hit your (usage )?limit|(5-hour|weekly|session|opus) limit reached|out of extra usage)").unwrap()
});

impl Daemon {
    /// Claude's status line said what this pane's session has used.
    pub(super) fn report_usage(&mut self, term: TermId, usage: Usage, limits: Vec<Limit>) {
        let Some(t) = self.terms.get_mut(&term) else { return };
        let agent = t.agent.clone().unwrap_or_else(|| "claude".into());
        let place = t.place();
        if let Some(cost) = usage.cost {
            let before = t.usage.cost.unwrap_or(0.0);
            // Lower than before: a new session (/clear) counting from zero.
            let added = if cost >= before { cost - before } else { cost };
            if added > 0.0 {
                let now = term::unix_now();
                self.spent.retain(|(at, ..)| now.saturating_sub(*at) < DAY);
                self.spent.push((now, added, agent.clone(), place));
                self.dirty = true;
            }
        }
        if t.usage != usage {
            t.usage = usage;
            self.dirty = true;
        }
        if !limits.is_empty() && self.limits.get(&agent) != Some(&limits) {
            self.limits.insert(agent, limits);
            self.dirty = true;
        }
    }

    /// What sessions in seshi have cost over the last day.
    pub(super) fn spent_today(&self) -> f64 {
        let now = term::unix_now();
        self.spent.iter().filter(|(at, ..)| now.saturating_sub(*at) < DAY).map(|(_, c, ..)| c).sum()
    }

    /// A turn ended: note it, and how long the agent worked, for the day's summary.
    pub(super) fn note_turn(&mut self, agent: String, place: String, secs: u64) {
        let now = term::unix_now();
        self.turns.retain(|(at, ..)| now.saturating_sub(*at) < DAY);
        self.turns.push((now, agent, place, secs));
    }

    /// Codex writes its numbers to its session file: read them now and then, off the loop.
    pub(super) fn poll_codex(&mut self) {
        if self.codex_busy || self.last_codex.elapsed() < CODEX_EVERY {
            return;
        }
        let panes: Vec<_> = self.terms.values()
            .filter(|t| t.agent.as_deref() == Some("codex"))
            .map(|t| (t.id, t.session.clone()))
            .collect();
        if panes.is_empty() {
            return;
        }
        self.codex_busy = true;
        self.last_codex = Instant::now();
        let tx = self.tx.clone();
        tokio::task::spawn_blocking(move || {
            let sessions = codex_recent_sessions();
            let mut facts = Vec::new();
            let mut limits = None;
            for (term, session) in &panes {
                let mut pane = CodexPaneFacts { term: *term, session: session.clone(), usage: None, subagents: Vec::new() };
                let owners = panes.iter().filter(|(_, id)| id == session).count();
                if let Some(root) = codex_root(&sessions, pane.session.as_deref()).filter(|_| owners == 1) {
                    let tail = read_tail(&root.file, CODEX_TAIL);
                    let (u, l) = codex_facts(&tail);
                    pane.usage = u;
                    limits = limits.or(l);
                    pane.subagents = codex_helpers(root, &tail, &sessions);
                }
                facts.push(pane);
            }
            let _ = tx.blocking_send(Ev::CodexUsage { facts, limits });
        });
    }

    pub(super) fn codex_usage(&mut self, facts: Vec<CodexPaneFacts>, limits: Option<Vec<Limit>>) {
        self.codex_busy = false;
        for pane in facts {
            let Some(t) = self.terms.get_mut(&pane.term) else { continue };
            // A new thread or another agent may have replaced the one read in the background.
            if !pane.belongs_to(t.agent.as_deref(), t.session.as_deref()) {
                continue;
            }
            if t.subagents != pane.subagents {
                t.subagents = pane.subagents;
                self.dirty = true;
            }
            if let Some(u) = pane.usage.filter(|u| t.usage != *u) {
                t.usage = u;
                self.dirty = true;
            }
        }
        if let Some(l) = limits
            && self.limits.get("codex") != Some(&l)
        {
            self.limits.insert("codex".into(), l);
            self.dirty = true;
        }
    }

    /// An agent stopped by a plan limit gets "continue" once the limit resets.
    pub(super) fn auto_continue(&mut self) {
        let now = term::unix_now();
        let on = self.cfg.auto_continue;
        let mut said = Vec::new();
        for t in self.terms.values_mut() {
            let Some(agent) = t.agent.clone() else { continue };
            if !on || t.status == Status::Working {
                // Off, or it's going again (you continued it yourself).
                if t.resume_at.take().is_some() {
                    self.dirty = true;
                }
                continue;
            }
            if let Some(at) = t.resume_at {
                if now >= at {
                    t.resume_at = None;
                    t.continued = Some(Instant::now());
                    t.pending_input = Some((b"continue\r".to_vec(), Instant::now()));
                    said.push((agent, t.id));
                    self.dirty = true;
                }
                continue;
            }
            if t.continued.is_some_and(|c| c.elapsed() < CONTINUE_SETTLES) || !LIMIT_HIT.is_match(&t.tail_text(LIMIT_ROWS)) {
                continue;
            }
            let reset = self.limits.get(&agent).and_then(|ls| ls.iter().filter(|l| l.used >= SPENT).map(|l| l.resets_at).max()).filter(|r| *r > now);
            t.resume_at = Some(reset.map(|r| r + AFTER_RESET).unwrap_or(now + RETRY_LIMIT_AFTER));
            self.dirty = true;
        }
        for (agent, term) in said {
            self.broadcast(|c| c.attach, ServerMsg::Notice(format!("{agent}'s limit reset: told it to continue (pane {term})")));
        }
    }
}

/// A disk snapshot is tied to the thread that owned the pane when polling began.
#[derive(Debug)]
pub(crate) struct CodexPaneFacts {
    pub term: TermId,
    pub session: Option<String>,
    pub usage: Option<Usage>,
    pub subagents: Vec<(String, String)>,
}

impl CodexPaneFacts {
    fn belongs_to(&self, agent: Option<&str>, session: Option<&str>) -> bool {
        agent == Some("codex") && session == self.session.as_deref()
    }
}

fn codex_root<'a>(sessions: &'a [CodexSession], session: Option<&str>) -> Option<&'a CodexSession> {
    // Folder membership cannot establish ownership when several agents share a checkout.
    let id = session?;
    sessions.iter().find(|s| s.parent.is_none() && s.id == id)
}

/// Codex's sessions folder (`$CODEX_HOME/sessions`, else `~/.codex/sessions`).
fn codex_sessions() -> Option<PathBuf> {
    let home = std::env::var_os("CODEX_HOME").map(PathBuf::from).or_else(|| directories::BaseDirs::new().map(|d| d.home_dir().join(".codex")))?;
    Some(home.join("sessions"))
}

/// A Codex session file: who it is and where it ran, from its first line.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CodexSession {
    pub file: PathBuf,
    pub cwd: Option<PathBuf>,
    pub id: String,
    /// The session that spawned it (a subagent's).
    pub parent: Option<String>,
    /// A subagent's task name (the end of its agent path), else its nickname.
    pub name: String,
    pub modified: std::time::SystemTime,
}

/// A subagent whose file hasn't been written to for this long has stopped without saying so.
const CODEX_SUBAGENT_STALE: Duration = Duration::from_secs(20 * 60);

/// What runs under a Codex session right now, as (id, name): its subagents (and theirs) whose
/// task is still going, and `goal` while a goal is what started its current turn.
pub(super) fn codex_helpers(root: &CodexSession, root_tail: &str, all: &[CodexSession]) -> Vec<(String, String)> {
    let mut out = Vec::new();
    if codex_running(root_tail) == Some(true) && codex_goal_turn(root_tail) {
        out.push(("goal".to_string(), "goal".to_string()));
    }
    let mut parents = vec![root.id.clone()];
    while let Some(parent) = parents.pop() {
        for s in all.iter().filter(|s| s.parent.as_deref() == Some(parent.as_str())) {
            parents.push(s.id.clone());
            let fresh = s.modified.elapsed().is_ok_and(|e| e < CODEX_SUBAGENT_STALE);
            // No task line in the part read: a long task still writing counts as running.
            if fresh && codex_running(&read_tail(&s.file, CODEX_TAIL)).unwrap_or(true) {
                out.push((s.id.clone(), s.name.clone()));
            }
        }
    }
    out
}

/// Whether the last task in a session file's tail is still going (None: no task line there).
pub(super) fn codex_running(tail: &str) -> Option<bool> {
    tail.lines().rev().find_map(|l| {
        if l.contains("\"type\":\"task_started\"") {
            Some(true)
        } else if l.contains("\"type\":\"task_complete\"") || l.contains("\"type\":\"turn_aborted\"") {
            Some(false)
        } else {
            None
        }
    })
}

/// Whether the last task started in the tail was started by a goal (`/goal`).
pub(super) fn codex_goal_turn(tail: &str) -> bool {
    tail.lines().rev().find(|l| l.contains("\"type\":\"task_started\"")).is_some_and(|l| l.contains("\"turn_trigger\":\"goal\""))
}

/// A session's id, parent, folder and name from the start of its first line.
pub(super) fn codex_meta(head: &str) -> Option<(String, Option<String>, Option<PathBuf>, String)> {
    static FIELD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r#""(id|parent_thread_id|cwd|agent_path|agent_nickname|thread_source)":("(?:[^"\\]|\\.)*")"#).unwrap());
    let mut get = std::collections::HashMap::new();
    for c in FIELD.captures_iter(head) {
        // The first of each: the session's own, ahead of anything nested later in the line.
        get.entry(c[1].to_string()).or_insert_with(|| serde_json::from_str::<String>(&c[2]).unwrap_or_default());
    }
    let id = get.get("id").cloned().filter(|s| !s.is_empty())?;
    let sub = get.get("thread_source").is_some_and(|s| s == "subagent");
    let parent = get.get("parent_thread_id").cloned().filter(|p| sub && !p.is_empty());
    let name = get
        .get("agent_path")
        .and_then(|p| p.rsplit('/').next().map(str::to_string))
        .filter(|n| !n.is_empty())
        .or_else(|| get.get("agent_nickname").cloned())
        .unwrap_or_else(|| "subagent".into());
    Some((id, parent, get.get("cwd").map(PathBuf::from), name))
}

/// How many of Codex's days with sessions are looked through: a session lives in the folder
/// of the day it started, and one can run for days.
const CODEX_DAYS: usize = 7;

/// The session files of Codex's last `CODEX_DAYS` days with sessions
/// (`YYYY/MM/DD/rollout-….jsonl`), newest first, each with who it is and the folder it ran in.
/// Who a file is never changes, so it's read once and remembered.
fn codex_recent_sessions() -> Vec<CodexSession> {
    let Some(root) = codex_sessions() else { return Vec::new() };
    let sorted = |dir: &Path| -> Vec<PathBuf> {
        let mut v: Vec<PathBuf> = std::fs::read_dir(dir).map(|r| r.flatten().map(|e| e.path()).collect()).unwrap_or_default();
        v.sort();
        v.reverse();
        v
    };
    let days: Vec<PathBuf> = sorted(&root).iter().flat_map(|y| sorted(y)).flat_map(|m| sorted(&m)).take(CODEX_DAYS).collect();
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = days
        .iter()
        .flat_map(|d| sorted(d))
        .filter(|f| f.extension().is_some_and(|e| e == "jsonl"))
        .filter_map(|f| Some((f.metadata().ok()?.modified().ok()?, f)))
        .collect();
    files.sort_by_key(|f| std::cmp::Reverse(f.0));
    files
        .into_iter()
        .filter_map(|(modified, file)| {
            type Meta = Option<(String, Option<String>, Option<PathBuf>, String)>;
            static SEEN: LazyLock<std::sync::Mutex<std::collections::HashMap<PathBuf, Meta>>> = LazyLock::new(Default::default);
            let meta = SEEN.lock().ok()?.entry(file.clone()).or_insert_with(|| codex_meta(&read_head(&file, CODEX_HEAD))).clone();
            let (id, parent, cwd, name) = meta?;
            Some(CodexSession { file, cwd, id, parent, name, modified })
        })
        .collect()
}

/// How much of the start of a session file holds who it is (its first line's first fields).
const CODEX_HEAD: usize = 16 * 1024;

/// The first `n` bytes of a file, as text.
fn read_head(file: &Path, n: usize) -> String {
    use std::io::Read;
    let mut head = vec![0; n];
    let read = std::fs::File::open(file).and_then(|mut f| f.read(&mut head)).unwrap_or(0);
    String::from_utf8_lossy(&head[..read]).into_owned()
}

/// The last `n` bytes of a file, as text.
fn read_tail(file: &Path, n: u64) -> String {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(file) else { return String::new() };
    let len = f.metadata().map(|m| m.len()).unwrap_or(0);
    let _ = f.seek(SeekFrom::Start(len.saturating_sub(n)));
    let mut buf = Vec::new();
    let _ = f.read_to_end(&mut buf);
    String::from_utf8_lossy(&buf).into_owned()
}

/// A limit window's name from its length in minutes.
fn window_name(minutes: u64) -> String {
    match minutes {
        300 => "5h".into(),
        10080 => "week".into(),
        m if m % 1440 == 0 => format!("{}d", m / 1440),
        m if m % 60 == 0 => format!("{}h", m / 60),
        m => format!("{m}m"),
    }
}

/// Context and plan limits from the end of a Codex session file: its last `token_count`.
pub(super) fn codex_facts(tail: &str) -> (Option<Usage>, Option<Vec<Limit>>) {
    let Some(v) = tail.lines().rev().filter(|l| l.contains("\"token_count\"")).find_map(|l| serde_json::from_str::<serde_json::Value>(l).ok()) else {
        return (None, None);
    };
    let p = &v["payload"];
    let info = &p["info"];
    let window = info["model_context_window"].as_f64().filter(|w| *w > 0.0);
    let used = info["last_token_usage"]["total_tokens"].as_f64();
    let usage = window.zip(used).map(|(w, u)| Usage { context: Some((u / w * 100.0).min(100.0) as f32), cost: None });
    let limits: Vec<Limit> = ["primary", "secondary"]
        .iter()
        .filter_map(|k| {
            let l = &p["rate_limits"][k];
            Some(Limit { name: window_name(l["window_minutes"].as_u64()?), used: l["used_percent"].as_f64()? as f32, resets_at: l["resets_at"].as_u64()? })
        })
        .collect();
    (usage, (!limits.is_empty()).then_some(limits))
}

/// The last day by agent and folder: turns, time working and cost, most work first.
pub(super) fn today(turns: &[(u64, String, String, u64)], spent: &[(u64, f64, String, String)], now: u64) -> Vec<crate::protocol::AgentDay> {
    use crate::protocol::AgentDay;
    let mut rows: Vec<AgentDay> = Vec::new();
    let mut row = |agent: &str, place: &str| -> usize {
        match rows.iter().position(|r| r.agent == agent && r.place == place) {
            Some(i) => i,
            None => {
                rows.push(AgentDay { agent: agent.to_string(), place: place.to_string(), ..Default::default() });
                rows.len() - 1
            }
        }
    };
    let mut turns_by: Vec<(usize, u64)> = Vec::new();
    for (_, agent, place, secs) in turns.iter().filter(|(at, ..)| now.saturating_sub(*at) < DAY) {
        turns_by.push((row(agent, place), *secs));
    }
    let mut cost_by: Vec<(usize, f64)> = Vec::new();
    for (_, cost, agent, place) in spent.iter().filter(|(at, ..)| now.saturating_sub(*at) < DAY) {
        cost_by.push((row(agent, place), *cost));
    }
    for (i, secs) in turns_by {
        rows[i].turns += 1;
        rows[i].working_secs += secs;
    }
    for (i, cost) in cost_by {
        rows[i].cost += cost;
    }
    rows.sort_by(|a, b| b.working_secs.cmp(&a.working_secs).then_with(|| a.agent.cmp(&b.agent)));
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_day_sums_turns_time_and_cost_per_agent_and_folder() {
        let now = 100_000;
        let turns = vec![
            (now - 60, "claude".to_string(), "shop".to_string(), 300),
            (now - 30, "claude".to_string(), "shop".to_string(), 120),
            (now - 10, "codex".to_string(), "api".to_string(), 60),
            (now - DAY - 1, "claude".to_string(), "shop".to_string(), 9_999),
        ];
        let spent = vec![(now - 5, 1.25, "claude".to_string(), "shop".to_string()), (now - 4, 0.5, "gemini".to_string(), "web".to_string())];
        let day = today(&turns, &spent, now);
        assert_eq!((day[0].agent.as_str(), day[0].place.as_str(), day[0].turns, day[0].working_secs), ("claude", "shop", 2, 420), "most work first; older than a day left out");
        assert!((day[0].cost - 1.25).abs() < 1e-9);
        assert_eq!((day[1].agent.as_str(), day[1].turns), ("codex", 1));
        assert_eq!((day[2].agent.as_str(), day[2].turns, day[2].working_secs), ("gemini", 0, 0), "cost without a turn still counts");
    }

    #[test]
    fn codex_helpers_belong_to_the_exact_thread_even_in_the_same_folder() {
        let root = |id: &str| CodexSession { file: PathBuf::new(), cwd: Some(PathBuf::from("same-project")), id: id.into(), parent: None, name: String::new(), modified: std::time::SystemTime::UNIX_EPOCH };
        let sessions = vec![root("newest"), root("older")];
        assert_eq!(codex_root(&sessions, Some("older")).unwrap().id, "older");
        assert_eq!(codex_root(&sessions, Some("newest")).unwrap().id, "newest");
        assert!(codex_root(&sessions, None).is_none());
        assert!(codex_root(&sessions, Some("unknown")).is_none());
        let pane = CodexPaneFacts { term: 1, session: Some("older".into()), usage: None, subagents: Vec::new() };
        assert!(pane.belongs_to(Some("codex"), Some("older")));
        assert!(!pane.belongs_to(Some("codex"), Some("newest")));
        assert!(!pane.belongs_to(Some("pi"), Some("older")));
    }

    #[test]
    fn codex_sessions_say_who_they_are() {
        let root = r#"{"timestamp":"t","ordinal":0,"type":"session_meta","payload":{"creator_user_id":"u","session_id":"aaa","id":"aaa","timestamp":"t","cwd":"C:\\dev\\game","originator":"codex-tui","source":"vscode","thread_source":"user"}}"#;
        assert_eq!(codex_meta(root), Some(("aaa".into(), None, Some(PathBuf::from(r"C:\dev\game")), "subagent".into())));
        let sub = r#"{"type":"session_meta","payload":{"session_id":"aaa","id":"bbb","forked_from_id":"aaa","parent_thread_id":"aaa","cwd":"C:\\dev\\game","source":{"subagent":{"thread_spawn":{"parent_thread_id":"aaa","depth":1,"agent_path":"/root/reconcile_audit","agent_nickname":"Kuhn"}}},"thread_source":"subagent","agent_nickname":"Kuhn"}}"#;
        let (id, parent, _, name) = codex_meta(sub).unwrap();
        assert_eq!((id.as_str(), parent.as_deref(), name.as_str()), ("bbb", Some("aaa"), "reconcile_audit"), "a subagent: its own id, its parent, its task's name");
        assert_eq!(codex_meta("not a session"), None);
    }

    #[test]
    fn codex_subagents_and_goals_are_read_off_its_files() {
        let started = r#"{"type":"event_msg","payload":{"type":"task_started","turn_attribution":{"turn_trigger":"user"}}}"#;
        let by_goal = r#"{"type":"event_msg","payload":{"type":"task_started","turn_attribution":{"turn_trigger":"goal"}}}"#;
        let done = r#"{"type":"event_msg","payload":{"type":"task_complete","turn_id":"x"}}"#;
        assert_eq!(codex_running(&format!("{started}\n{done}")), Some(false));
        assert_eq!(codex_running(&format!("{done}\n{started}\n{{\"type\":\"response_item\"}}")), Some(true));
        assert_eq!(codex_running("{\"type\":\"response_item\"}"), None, "no task line in what was read");
        assert!(codex_goal_turn(&format!("{started}\n{done}\n{by_goal}")) && !codex_goal_turn(&format!("{by_goal}\n{done}\n{started}")), "the last turn's trigger");
        // On disk: a root working toward a goal, one subagent still going, one finished.
        let dir = std::env::temp_dir().join(format!("seshi-codex-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let file = |name: &str, body: &str| {
            let f = dir.join(name);
            std::fs::write(&f, body).unwrap();
            f
        };
        let session = |f: PathBuf, id: &str, parent: Option<&str>, name: &str| CodexSession { file: f, cwd: None, id: id.into(), parent: parent.map(String::from), name: name.into(), modified: std::time::SystemTime::now() };
        let root = session(file("root.jsonl", by_goal), "aaa", None, "subagent");
        let all = vec![root.clone(), session(file("a.jsonl", started), "bbb", Some("aaa"), "reconcile_audit"), session(file("b.jsonl", &format!("{started}\n{done}")), "ccc", Some("aaa"), "docs_pass"), session(file("c.jsonl", started), "ddd", Some("bbb"), "nested_check")];
        let helpers = codex_helpers(&root, by_goal, &all);
        let names: Vec<&str> = helpers.iter().map(|(_, n)| n.as_str()).collect();
        assert_eq!(names, vec!["goal", "reconcile_audit", "nested_check"], "the goal, the running subagent and its own; not the finished one");
        let _ = std::fs::remove_dir_all(&dir);
    }

    /// Against the Codex sessions on this machine: `cargo test codex_live -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn codex_live() {
        let all = codex_recent_sessions();
        let roots: Vec<&CodexSession> = all.iter().filter(|s| s.parent.is_none()).collect();
        println!("{} recent sessions, {} of them yours, {} subagents", all.len(), roots.len(), all.len() - roots.len());
        for r in roots.iter().take(5) {
            let helpers = codex_helpers(r, &read_tail(&r.file, CODEX_TAIL), &all);
            let under = all.iter().filter(|s| s.parent.as_deref() == Some(r.id.as_str())).count();
            println!("  {}: {} subagents ever, running now: {:?}", r.cwd.as_ref().and_then(|c| c.file_name()).map(|n| n.to_string_lossy().into_owned()).unwrap_or_default(), under, helpers.iter().map(|(_, n)| n.as_str()).collect::<Vec<_>>());
        }
    }

    #[test]
    fn codex_numbers_come_from_its_session_file() {
        let tail = r#"{"type":"event_msg","payload":{"type":"agent_message"}}
{"timestamp":"x","type":"event_msg","payload":{"type":"token_count","info":{"last_token_usage":{"total_tokens":64600},"model_context_window":258400},"rate_limits":{"primary":{"used_percent":12.0,"window_minutes":300,"resets_at":1791322893},"secondary":{"used_percent":40.0,"window_minutes":10080,"resets_at":1791900000}}}}
{"type":"response_item","payload":{}}"#;
        let (u, l) = codex_facts(tail);
        assert_eq!(u.and_then(|u| u.context).map(|c| c.round()), Some(25.0));
        let l = l.unwrap();
        assert_eq!((l[0].name.as_str(), l[1].name.as_str(), l[1].used), ("5h", "week", 40.0));
        assert_eq!(codex_facts("nothing here"), (None, None));
    }

    #[test]
    fn a_limit_line_is_told_from_talk_about_limits() {
        for hit in ["Claude usage limit reached. Your limit will reset at 3pm", "You've hit your limit · resets 3pm", "■ You've hit your usage limit. Try again at 4:05 PM."] {
            assert!(LIMIT_HIT.is_match(hit), "{hit}");
        }
        for talk in ["I'll add a usage meter and a limit bar", "rate limits are per minute"] {
            assert!(!LIMIT_HIT.is_match(talk), "{talk}");
        }
    }
}
