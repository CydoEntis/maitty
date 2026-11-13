use super::history::{History, IncompleteReason, MAX_READ_BYTES, MAX_READ_ENTRIES};
use crate::conversation::{ConversationId, NativeBinding};
use crate::daemon::persist::{create_private, private_dir};
use anyhow::{Context, Result, bail};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex};

/// Why a cache cannot be used, as a category only: a decode error's own text can quote an entry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Unreadable {
    Decode,
    Corrupt,
    Binding,
    Exhausted,
}
impl std::fmt::Display for Unreadable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Decode => "decode",
            Self::Corrupt => "corrupt",
            Self::Binding => "binding",
            Self::Exhausted => "exhausted",
        })
    }
}
impl std::error::Error for Unreadable {}

const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Manifest {
    version: u32,
    binding: String,
    conversation: ConversationId,
    revision: u64,
    completeness: super::history::Completeness,
    pages: Vec<StoredPage>,
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct StoredPage {
    number: u64,
    bytes: usize,
    hash: String,
}
struct State {
    epoch: AtomicU64,
}
pub(in crate::daemon) struct PolicyChange {
    epoch: u64,
    enabled: bool,
}
#[derive(Clone)]
pub(in crate::daemon) struct Cache {
    root: PathBuf,
    state: Arc<Mutex<()>>,
    policy: Arc<State>,
}
pub(super) struct WriteTicket {
    epoch: u64,
    binding: String,
    history: History,
}
fn fingerprint(binding: &NativeBinding) -> Result<String> {
    Ok(format!(
        "{:x}",
        Sha256::digest(serde_json::to_vec(binding)?)
    ))
}
fn remove(root: &Path) -> Result<()> {
    match std::fs::remove_dir_all(root) {
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        result => result.context("deleting private history cache"),
    }
}
impl Cache {
    pub(super) fn for_session(
        session_path: &Path,
        restore_enabled: bool,
        output: bool,
    ) -> Result<Self> {
        Self::open(
            session_path.with_extension("history"),
            restore_enabled,
            output,
        )
    }
    fn open(root: PathBuf, restore_enabled: bool, output: bool) -> Result<Self> {
        let cache = Self {
            root,
            state: Arc::new(Mutex::new(())),
            policy: Arc::new(State {
                epoch: AtomicU64::new(0),
            }),
        };
        cache.configure(restore_enabled, output)?;
        Ok(cache)
    }
    pub(super) fn configure(&self, restore_enabled: bool, output: bool) -> Result<()> {
        let policy = self.fence_policy(restore_enabled, output)?;
        self.apply_policy(policy)
    }
    pub(super) fn fence_policy(&self, restore_enabled: bool, output: bool) -> Result<PolicyChange> {
        // Fence old writes synchronously before background cleanup can be queued.
        let epoch = &self.policy.epoch;
        let mut previous = epoch.load(Ordering::SeqCst);
        loop {
            let next = (previous & !1)
                .checked_add(2)
                .ok_or_else(|| anyhow::anyhow!("History policy generation exhausted"))?;
            match epoch.compare_exchange_weak(previous, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => break,
                Err(actual) => previous = actual,
            }
        }
        let epoch = (previous & !1) + 2;
        Ok(PolicyChange {
            epoch,
            enabled: restore_enabled && output,
        })
    }
    pub(super) fn apply_policy(&self, policy: PolicyChange) -> Result<()> {
        let _guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("history cache lock poisoned"))?;
        if self.policy.epoch.load(Ordering::SeqCst) != policy.epoch {
            bail!("History policy change superseded");
        }
        if !policy.enabled {
            remove(&self.root)?;
        }
        self.policy
            .epoch
            .compare_exchange(
                policy.epoch,
                policy.epoch | u64::from(policy.enabled),
                Ordering::SeqCst,
                Ordering::SeqCst,
            )
            .map_err(|_| anyhow::anyhow!("History policy change superseded"))?;
        Ok(())
    }
    pub(super) fn prepare(
        &self,
        binding: &NativeBinding,
        history: &History,
    ) -> Result<WriteTicket> {
        if history.revision == u64::MAX {
            return Err(IncompleteReason::RevisionExhausted.into());
        }
        let epoch = self.policy.epoch.load(Ordering::SeqCst);
        if epoch & 1 == 0 {
            bail!("history persistence disabled");
        }
        Ok(WriteTicket {
            epoch,
            binding: fingerprint(binding)?,
            history: history.durable(),
        })
    }
    pub(super) fn commit(&self, ticket: WriteTicket) -> Result<bool> {
        self.commit_with(ticket, || Ok(()))
    }
    fn commit_with(
        &self,
        ticket: WriteTicket,
        before_manifest: impl FnOnce() -> Result<()>,
    ) -> Result<bool> {
        // The same lock covers deletion and the final rename; an old queued write cannot recreate disabled content.
        let _guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("history cache lock poisoned"))?;
        if self.policy.epoch.load(Ordering::SeqCst) != ticket.epoch {
            return Ok(false);
        }
        let dir = self.root.join(ticket.history.conversation.0.to_string());
        let current = dir.join("manifest.json");
        match bounded_read(&current, MAX_MANIFEST_BYTES) {
            Ok(bytes) => {
                let old: Manifest =
                    serde_json::from_slice(&bytes).context("invalid history manifest")?;
                if old.binding == ticket.binding && old.revision >= ticket.history.revision {
                    return Ok(false);
                }
            }
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) => {}
            Err(error) => return Err(error),
        }
        private_dir(&self.root).context("creating private history root")?;
        private_dir(&dir).context("creating private conversation history")?;
        let mut manifest = Manifest {
            version: 1,
            binding: ticket.binding,
            conversation: ticket.history.conversation,
            revision: ticket.history.revision,
            completeness: ticket.history.completeness.clone(),
            pages: Vec::new(),
        };
        let mut cursor = None;
        let mut page_number = 0u64;
        loop {
            let page = ticket
                .history
                .read(cursor.as_ref(), MAX_READ_ENTRIES, MAX_READ_BYTES)
                .map_err(|e| anyhow::anyhow!("history page cannot be stored: {e:?}"))?;
            if !page.entries.is_empty() {
                let bytes = serde_json::to_vec(&page.entries)?;
                let (number, mut file) = loop {
                    page_number = page_number
                        .checked_add(1)
                        .context("history page identity exhausted")?;
                    let path = dir.join(format!("{page_number}.json"));
                    match create_new_private(&path) {
                        Ok(file) => break (page_number, file),
                        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => continue,
                        Err(e) => return Err(e).context("creating immutable history page"),
                    }
                };
                file.write_all(&bytes).context("writing history page")?;
                file.sync_all().context("syncing history page")?;
                manifest.pages.push(StoredPage {
                    number,
                    bytes: bytes.len(),
                    hash: format!("{:x}", Sha256::digest(&bytes)),
                });
            }
            cursor = page.next;
            if cursor.is_none() {
                break;
            }
        }
        let bytes = serde_json::to_vec(&manifest)?;
        if bytes.len() as u64 > MAX_MANIFEST_BYTES {
            bail!("history manifest exceeds size limit");
        }
        let temporary = dir.join("manifest.tmp");
        let mut file = create_private(&temporary).context("creating private history manifest")?;
        file.write_all(&bytes).context("writing history manifest")?;
        file.sync_all().context("syncing history manifest")?;
        before_manifest()?;
        if self.policy.epoch.load(Ordering::SeqCst) != ticket.epoch {
            return Ok(false);
        }
        std::fs::rename(&temporary, dir.join("manifest.json"))
            .context("committing history manifest")?;
        #[cfg(unix)]
        std::fs::File::open(&dir)?
            .sync_all()
            .context("syncing history directory")?;
        Ok(true)
    }
    /// Loads a conversation's cached history. A cache this daemon can tell is unusable (an entry
    /// kind it does not know, a corrupt or missing page, a changed binding, an exhausted
    /// revision) is moved aside to `<id>.bad-<n>` (the newest `BAD_COPIES_KEPT` kept) and reported
    /// as absent, so the daemon still starts and the history is read again from the agent; the
    /// old files stay on disk because the agent may no longer hold the history. Nothing is
    /// deleted before the move succeeds, and a move that fails is returned as an error with the
    /// cache intact. Any other failure (a sharing violation, too many open files, an I/O error)
    /// is returned with the files untouched. Only the conversation id and a category are logged,
    /// never the error text, which can quote an entry.
    pub(super) fn load_or_rebuild(
        &self,
        conversation: ConversationId,
        binding: &NativeBinding,
    ) -> Result<Option<History>> {
        // One lock across the check and the move: a commit or a policy removal between them
        // could otherwise have a good cache moved aside, or turn a missing one into an error.
        let _guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("history cache lock poisoned"))?;
        match self.load_locked(conversation, binding) {
            Err(error) => match error.downcast_ref::<Unreadable>() {
                Some(category) => {
                    let dir = self.root.join(conversation.0.to_string());
                    let number = next_bad_number(&self.root, conversation)?;
                    move_aside(&dir, &self.root.join(format!("{}.bad-{number}", conversation.0)))?;
                    prune_bad(&self.root, conversation);
                    tracing::warn!(
                        conversation = conversation.0,
                        category = %category,
                        "Cached conversation history is unreadable; kept as .bad and read again from the agent"
                    );
                    Ok(None)
                }
                None => Err(error),
            },
            loaded => loaded,
        }
    }

    /// A plain load that fails on anything unreadable. The daemon starts through
    /// `load_or_rebuild`; this is the strict form the tests compare it with.
    #[cfg(test)]
    pub(super) fn load(
        &self,
        conversation: ConversationId,
        binding: &NativeBinding,
    ) -> Result<Option<History>> {
        let _guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("history cache lock poisoned"))?;
        self.load_locked(conversation, binding)
    }
    fn load_locked(
        &self,
        conversation: ConversationId,
        binding: &NativeBinding,
    ) -> Result<Option<History>> {
        if self.policy.epoch.load(Ordering::SeqCst) & 1 == 0 {
            return Ok(None);
        }
        let dir = self.root.join(conversation.0.to_string());
        let bytes = match bounded_read(&dir.join("manifest.json"), MAX_MANIFEST_BYTES) {
            Err(error) if is_not_found(&error) => return Ok(None),
            result => result?,
        };
        let manifest: Manifest =
            serde_json::from_slice(&bytes).map_err(|_| Unreadable::Decode)?;
        if manifest.revision == u64::MAX {
            return Err(Unreadable::Exhausted.into());
        }
        if manifest.version != 1
            || manifest.conversation != conversation
            || manifest.binding != fingerprint(binding)?
        {
            return Err(Unreadable::Binding.into());
        }
        let mut history = History::new(conversation);
        for page in manifest.pages {
            let bytes = match bounded_read(
                &dir.join(format!("{}.json", page.number)),
                MAX_READ_BYTES as u64,
            ) {
                Err(error) if is_not_found(&error) => return Err(Unreadable::Corrupt.into()),
                result => result.context("reading a history page")?,
            };
            if bytes.len() != page.bytes || format!("{:x}", Sha256::digest(&bytes)) != page.hash {
                return Err(Unreadable::Corrupt.into());
            }
            let entries: Vec<super::history::Entry> =
                serde_json::from_slice(&bytes).map_err(|_| Unreadable::Decode)?;
            if entries.is_empty() || entries.len() > MAX_READ_ENTRIES {
                return Err(Unreadable::Corrupt.into());
            }
            for entry in entries {
                if entry.source.conversation != conversation
                    || history.entries.iter().any(|e| e.source == entry.source)
                {
                    return Err(Unreadable::Corrupt.into());
                }
                history.entries.push(entry);
            }
        }
        history.revision = manifest.revision;
        history.completeness = manifest.completeness;
        Ok(Some(history))
    }
}
/// How many set-aside copies of one conversation's cache are kept; older ones are dropped.
const BAD_COPIES_KEPT: usize = 3;
/// The numbers of a conversation's set-aside copies (`<id>.bad-<n>`), oldest first.
fn bad_copies(root: &Path, conversation: ConversationId) -> Result<Vec<u64>> {
    let prefix = format!("{}.bad-", conversation.0);
    let mut numbers = Vec::new();
    for item in std::fs::read_dir(root).context("listing set-aside history caches")? {
        let name = item?.file_name();
        if let Some(n) = name
            .to_str()
            .and_then(|name| name.strip_prefix(&prefix))
            .and_then(|n| n.parse::<u64>().ok())
        {
            numbers.push(n);
        }
    }
    numbers.sort_unstable();
    Ok(numbers)
}
fn next_bad_number(root: &Path, conversation: ConversationId) -> Result<u64> {
    Ok(bad_copies(root, conversation)?.last().map_or(1, |n| n + 1))
}
/// Moves `dir` to `aside`. Never deletes: if the move fails, `dir` is as it was.
fn move_aside(dir: &Path, aside: &Path) -> Result<()> {
    std::fs::rename(dir, aside).context("moving an unreadable history cache aside")
}
/// Drops the oldest set-aside copies beyond `BAD_COPIES_KEPT`, only after a new one was made. A
/// copy that cannot be dropped is left and warned about.
fn prune_bad(root: &Path, conversation: ConversationId) {
    let Ok(numbers) = bad_copies(root, conversation) else {
        return;
    };
    let extra = numbers.len().saturating_sub(BAD_COPIES_KEPT);
    for n in &numbers[..extra] {
        if let Err(error) = remove(&root.join(format!("{}.bad-{n}", conversation.0))) {
            tracing::warn!(conversation = conversation.0, %error, "Cannot drop an old set-aside history cache");
        }
    }
}
fn is_not_found(error: &anyhow::Error) -> bool {
    error
        .downcast_ref::<std::io::Error>()
        .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound)
}
fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .context("reading history file")?;
    if bytes.len() as u64 > limit {
        return Err(Unreadable::Corrupt.into());
    }
    Ok(bytes)
}
fn create_new_private(path: &Path) -> std::io::Result<std::fs::File> {
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    std::os::unix::fs::OpenOptionsExt::mode(&mut options, 0o600);
    options.open(path)
}

#[cfg(test)]
mod tests;
