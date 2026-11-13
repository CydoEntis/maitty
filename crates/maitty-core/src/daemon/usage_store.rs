//! The durable record of what agents reported using, per session per day (ADR 0009).
//!
//! Only what Claude Code and Codex themselves report goes in: tokens, cost, and the turns and
//! working time the daemon itself observed. Nothing is estimated, and a subscription session
//! stores no cost. Providers report running totals for a session, so the store keeps the last
//! total it saw per session (a cursor) and adds only the difference to today's row; the cursor
//! is saved too, or a restart would count a whole session again.
//!
//! The file is `usage-<socket>.json` in the data folder, private to the user, replaced
//! atomically, and read tolerantly: a file that can't be read is kept as `.bak` and the store
//! starts empty, never stopping the daemon.

use super::persist::{create_private, private_dir};
use super::*;
use crate::usage_report::{DayUsage, SessionUsage, UsageFigures, UsagePeriod, UsageReport};
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

/// The file format this build writes. A file with a higher number was written by a newer build
/// and is kept untouched as `.bak` rather than read wrongly.
pub(super) const FORMAT_VERSION: u32 = 1;
/// How many days of usage are kept (today included); older days and sessions are pruned.
pub(super) const RETENTION_DAYS: u64 = 90;
/// Changes reach the disk at most this often while the daemon runs.
const SAVE_EVERY: Duration = Duration::from_secs(30);
const SECONDS_PER_DAY: u64 = 24 * 60 * 60;

/// One session's usage on one day.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Row {
    day: String,
    session: String,
    agent: String,
    project: String,
    #[serde(default)]
    tokens_in: Option<u64>,
    #[serde(default)]
    tokens_out: Option<u64>,
    #[serde(default)]
    cost: Option<f64>,
    #[serde(default)]
    turns: u64,
    #[serde(default)]
    working_secs: u64,
}

/// The running totals last seen for a session, so the next report adds only what is new.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
struct Cursor {
    session: String,
    #[serde(default)]
    tokens_in: Option<u64>,
    #[serde(default)]
    tokens_out: Option<u64>,
    #[serde(default)]
    cost: Option<f64>,
    /// The provider showed this is a subscription (plan limits reported), so no cost is kept.
    #[serde(default)]
    subscription: bool,
    #[serde(default)]
    last_day: String,
}

/// The file as stored. Unknown fields are ignored and missing ones default, so a newer build
/// can add fields without a version bump; a change that can't be read that way raises
/// `FORMAT_VERSION`.
#[derive(Debug, Default, Serialize, Deserialize)]
struct Stored {
    #[serde(default)]
    version: u32,
    #[serde(default)]
    rows: Vec<Row>,
    #[serde(default)]
    cursors: Vec<Cursor>,
}

/// What a provider said a session has used in total, as of now.
pub(super) struct Reported<'a> {
    pub session: &'a str,
    pub agent: &'a str,
    pub project: &'a str,
    pub usage: &'a Usage,
    /// The report came with plan limits: a subscription, whose cost is never kept.
    pub subscription: bool,
}

pub(super) struct UsageStore {
    path: Option<PathBuf>,
    /// When this daemon started: names panes whose agent never reported a session id, since
    /// pane numbers start over with every daemon.
    pub boot: u64,
    rows: BTreeMap<(String, String), Row>,
    cursors: HashMap<String, Cursor>,
    dirty: bool,
    last_save: Instant,
    writing: Arc<AtomicBool>,
    write_failed: Arc<AtomicBool>,
}

impl UsageStore {
    /// A store that lives in memory only (tests, and daemons with no data folder).
    pub fn in_memory() -> UsageStore {
        UsageStore {
            path: None,
            boot: term::unix_now(),
            rows: BTreeMap::new(),
            cursors: HashMap::new(),
            dirty: false,
            last_save: Instant::now(),
            writing: Arc::default(),
            write_failed: Arc::default(),
        }
    }

    /// Where this daemon keeps its usage, named by socket so a nested daemon never shares it.
    pub fn default_path() -> PathBuf {
        let label =
            std::env::var(concat!(env!("APP_ENV"), "_SOCKET")).unwrap_or_else(|_| "default".into());
        crate::config::data_dir().join(format!("usage-{label}.json"))
    }

    /// Read the file at `path`, keeping it as the place to save. A missing file starts empty;
    /// one that can't be read is set aside as `.bak` and the store starts empty, saying why.
    pub fn open(path: PathBuf, now: u64) -> UsageStore {
        let mut store = UsageStore::in_memory();
        match std::fs::read_to_string(&path) {
            Ok(text) => match serde_json::from_str::<Stored>(&text) {
                Ok(stored) if stored.version <= FORMAT_VERSION => store.fill(stored),
                Ok(stored) => {
                    tracing::warn!(
                        "usage file {} is format {}, newer than this build reads ({FORMAT_VERSION}); \
                         keeping it as .bak and starting empty",
                        path.display(),
                        stored.version
                    );
                    set_aside(&path);
                }
                Err(error) => {
                    tracing::warn!(
                        "usage file {} can't be read ({error}); keeping it as .bak and starting empty",
                        path.display()
                    );
                    set_aside(&path);
                }
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => {
                tracing::warn!(
                    "usage file {} can't be opened ({error}); starting empty",
                    path.display()
                );
            }
        }
        store.path = Some(path);
        store.prune(now);
        store
    }

    fn fill(&mut self, stored: Stored) {
        for row in stored.rows {
            self.rows
                .insert((row.day.clone(), row.session.clone()), row);
        }
        for cursor in stored.cursors {
            self.cursors.insert(cursor.session.clone(), cursor);
        }
    }

    /// Add what a session newly reported to today's row.
    pub fn observe(&mut self, now: u64, reported: Reported) {
        let day = day_string(now);
        let session = reported.session.to_string();
        let usage = reported.usage;
        let cursor = self
            .cursors
            .entry(session.clone())
            .or_insert_with(|| Cursor {
                session: session.clone(),
                ..Default::default()
            });
        let before = cursor.clone();
        cursor.subscription |= reported.subscription;
        let cost = usage.cost.filter(|_| !cursor.subscription);
        cursor.last_day = day.clone();
        // A session that only has a context bar to show gets a cursor but no row.
        if usage.tokens_in.is_none() && usage.tokens_out.is_none() && cost.is_none() {
            self.dirty |= *cursor != before;
            return;
        }
        let row = self
            .rows
            .entry((day.clone(), session.clone()))
            .or_insert_with(|| Row {
                day,
                session,
                ..Default::default()
            });
        row.agent = reported.agent.to_string();
        row.project = reported.project.to_string();
        if let Some(total) = usage.tokens_in {
            row.tokens_in = Some(row.tokens_in.unwrap_or(0) + added(cursor.tokens_in, total));
            cursor.tokens_in = Some(total);
        }
        if let Some(total) = usage.tokens_out {
            row.tokens_out = Some(row.tokens_out.unwrap_or(0) + added(cursor.tokens_out, total));
            cursor.tokens_out = Some(total);
        }
        if let Some(total) = cost {
            row.cost = Some(row.cost.unwrap_or(0.0) + added_cost(cursor.cost, total));
            cursor.cost = Some(total);
        }
        self.dirty = true;
    }

    /// An agent finished a turn that took `secs`.
    pub fn turn(&mut self, now: u64, session: &str, agent: &str, project: &str, secs: u64) {
        let day = day_string(now);
        let row = self
            .rows
            .entry((day.clone(), session.to_string()))
            .or_insert_with(|| Row {
                day,
                session: session.to_string(),
                ..Default::default()
            });
        row.agent = agent.to_string();
        row.project = project.to_string();
        row.turns += 1;
        row.working_secs += secs;
        self.dirty = true;
    }

    /// Forget days past retention, and sessions not seen since.
    pub fn prune(&mut self, now: u64) {
        let oldest = day_string(now.saturating_sub((RETENTION_DAYS - 1) * SECONDS_PER_DAY));
        let rows = self.rows.len();
        let cursors = self.cursors.len();
        self.rows.retain(|(day, _), _| *day >= oldest);
        self.cursors.retain(|_, c| c.last_day >= oldest);
        if self.rows.len() != rows || self.cursors.len() != cursors {
            self.dirty = true;
        }
    }

    /// The window's usage, in total, by session and by day. `open` names the panes showing
    /// sessions right now.
    pub fn report(
        &self,
        period: UsagePeriod,
        now: u64,
        open: &HashMap<String, TermId>,
    ) -> UsageReport {
        let to = day_string(now);
        let from = day_string(now.saturating_sub((period.days() - 1) * SECONDS_PER_DAY));
        let mut totals = Accumulator::default();
        let mut days: BTreeMap<&str, Accumulator> = BTreeMap::new();
        let mut sessions: BTreeMap<&str, (SessionUsage, Accumulator)> = BTreeMap::new();
        for ((day, session), row) in &self.rows {
            if *day < from || *day > to {
                continue;
            }
            totals.add(row);
            days.entry(day).or_default().add(row);
            let (info, sum) = sessions.entry(session).or_insert_with(|| {
                (
                    SessionUsage {
                        session: session.clone(),
                        term: open.get(session).copied(),
                        agent: row.agent.clone(),
                        project: row.project.clone(),
                        figures: UsageFigures::default(),
                        first_day: day.clone(),
                        last_day: day.clone(),
                    },
                    Accumulator::default(),
                )
            });
            sum.add(row);
            // Rows come oldest first per session, so the last one seen names the session now.
            info.agent = row.agent.clone();
            info.project = row.project.clone();
            info.last_day = day.clone();
            if *day < info.first_day {
                info.first_day = day.clone();
            }
        }
        let mut sessions: Vec<SessionUsage> = sessions
            .into_values()
            .map(|(mut info, sum)| {
                info.figures = sum.figures();
                info
            })
            .collect();
        sessions.sort_by(|a, b| {
            let cost = |s: &SessionUsage| s.figures.cost.unwrap_or(-1.0);
            cost(b)
                .total_cmp(&cost(a))
                .then(b.figures.working_secs.cmp(&a.figures.working_secs))
                .then_with(|| a.session.cmp(&b.session))
        });
        UsageReport {
            period,
            from,
            to,
            retention_days: RETENTION_DAYS as u32,
            totals: totals.figures(),
            session_count: sessions.len() as u32,
            sessions_without_cost: sessions.iter().filter(|s| s.figures.cost.is_none()).count()
                as u32,
            sessions,
            days: days
                .into_iter()
                .map(|(day, sum)| DayUsage {
                    day: day.to_string(),
                    figures: sum.figures(),
                })
                .collect(),
        }
    }

    fn to_bytes(&self) -> Result<Vec<u8>> {
        let stored = Stored {
            version: FORMAT_VERSION,
            rows: self.rows.values().cloned().collect(),
            cursors: {
                let mut cursors: Vec<Cursor> = self.cursors.values().cloned().collect();
                cursors.sort_by(|a, b| a.session.cmp(&b.session));
                cursors
            },
        };
        Ok(serde_json::to_vec(&stored)?)
    }

    /// Save in the background when something changed and the last save was a while ago.
    /// Serializing is quick; the file work runs off the loop.
    pub fn save_if_due(&mut self, now: u64) {
        if self.write_failed.swap(false, Ordering::Relaxed) {
            self.dirty = true;
        }
        if !self.dirty
            || self.path.is_none()
            || self.last_save.elapsed() < SAVE_EVERY
            || self.writing.load(Ordering::Relaxed)
        {
            return;
        }
        self.prune(now);
        let (Some(path), Ok(bytes)) = (self.path.clone(), self.to_bytes()) else {
            return;
        };
        self.dirty = false;
        self.last_save = Instant::now();
        self.writing.store(true, Ordering::Relaxed);
        let (writing, failed) = (self.writing.clone(), self.write_failed.clone());
        tokio::task::spawn_blocking(move || {
            if let Err(error) = write_file(&path, &bytes) {
                tracing::warn!("couldn't save usage: {error:#}");
                failed.store(true, Ordering::Relaxed);
            }
            writing.store(false, Ordering::Relaxed);
        });
    }

    /// Save now, on the caller's thread, for when the daemon is about to exit.
    pub fn save_now(&mut self, now: u64) {
        let Some(path) = self.path.clone() else {
            return;
        };
        if !self.dirty {
            return;
        }
        self.prune(now);
        match self.to_bytes().map(|bytes| write_file(&path, &bytes)) {
            Ok(Ok(())) => self.dirty = false,
            Ok(Err(error)) | Err(error) => tracing::warn!("couldn't save usage: {error:#}"),
        }
    }
}

/// Sums rows, keeping a figure `None` until some row reports it.
#[derive(Default)]
struct Accumulator {
    tokens_in: Option<u64>,
    tokens_out: Option<u64>,
    cost: Option<f64>,
    turns: u64,
    working_secs: u64,
}

impl Accumulator {
    fn add(&mut self, row: &Row) {
        self.tokens_in = sum(self.tokens_in, row.tokens_in);
        self.tokens_out = sum(self.tokens_out, row.tokens_out);
        self.cost = match (self.cost, row.cost) {
            (None, None) => None,
            (a, b) => Some(a.unwrap_or(0.0) + b.unwrap_or(0.0)),
        };
        self.turns += row.turns;
        self.working_secs += row.working_secs;
    }

    fn figures(&self) -> UsageFigures {
        UsageFigures {
            tokens_in: self.tokens_in,
            tokens_out: self.tokens_out,
            cost: self.cost,
            turns: self.turns,
            working_secs: self.working_secs,
        }
    }
}

fn sum(a: Option<u64>, b: Option<u64>) -> Option<u64> {
    match (a, b) {
        (None, None) => None,
        (a, b) => Some(a.unwrap_or(0) + b.unwrap_or(0)),
    }
}

/// What a running total grew by. A total lower than the last one is a new session counting
/// from zero (`/clear`), so all of it is new.
fn added(last: Option<u64>, total: u64) -> u64 {
    match last {
        Some(last) if total >= last => total - last,
        _ => total,
    }
}

fn added_cost(last: Option<f64>, total: f64) -> f64 {
    match last {
        Some(last) if total >= last => total - last,
        _ => total,
    }
}

/// Keep a file that can't be used as `<name>.bak`, so a bad file is never silently lost.
pub(super) fn set_aside(path: &Path) {
    let mut name = path.as_os_str().to_owned();
    name.push(".bak");
    if let Err(error) = std::fs::rename(path, &name) {
        tracing::warn!("couldn't keep {} as .bak: {error}", path.display());
    }
}

/// Write `bytes` whole and synced to a private temporary file, then rename it over `path`, so
/// a crash leaves the old file or the new one. Blocks: call it off the loop.
pub(super) fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    if let Some(dir) = path.parent() {
        private_dir(dir).with_context(|| format!("making {}", dir.display()))?;
    }
    let tmp = path.with_extension("json.tmp");
    let mut file = create_private(&tmp).with_context(|| format!("opening {}", tmp.display()))?;
    file.write_all(bytes)
        .with_context(|| format!("writing {}", tmp.display()))?;
    file.sync_all()
        .with_context(|| format!("syncing {}", tmp.display()))?;
    drop(file);
    std::fs::rename(&tmp, path).with_context(|| format!("replacing {}", path.display()))
}

/// `YYYY-MM-DD` (UTC) for a Unix time. Days are UTC so one session's usage never splits
/// differently on two machines; the civil-date arithmetic is Howard Hinnant's.
pub(super) fn day_string(secs: u64) -> String {
    let days = (secs / SECONDS_PER_DAY) as i64 + 719_468;
    let era = days.div_euclid(146_097);
    let doe = days.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

#[cfg(test)]
#[path = "usage_store_tests.rs"]
mod tests;
