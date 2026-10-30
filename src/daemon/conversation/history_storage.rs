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
        let previous = self
            .policy
            .epoch
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |epoch| {
                (epoch & !1).checked_add(2)
            })
            .map_err(|_| anyhow::anyhow!("History policy generation exhausted"))?;
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
    pub(super) fn load(
        &self,
        conversation: ConversationId,
        binding: &NativeBinding,
    ) -> Result<Option<History>> {
        let _guard = self
            .state
            .lock()
            .map_err(|_| anyhow::anyhow!("history cache lock poisoned"))?;
        if self.policy.epoch.load(Ordering::SeqCst) & 1 == 0 {
            return Ok(None);
        }
        let dir = self.root.join(conversation.0.to_string());
        let bytes = match bounded_read(&dir.join("manifest.json"), MAX_MANIFEST_BYTES) {
            Err(error)
                if error
                    .downcast_ref::<std::io::Error>()
                    .is_some_and(|e| e.kind() == std::io::ErrorKind::NotFound) =>
            {
                return Ok(None);
            }
            result => result?,
        };
        let manifest: Manifest =
            serde_json::from_slice(&bytes).context("invalid history manifest")?;
        if manifest.revision == u64::MAX {
            return Err(IncompleteReason::RevisionExhausted.into());
        }
        if manifest.version != 1
            || manifest.conversation != conversation
            || manifest.binding != fingerprint(binding)?
        {
            bail!("history binding or format mismatch; native hydration required");
        }
        let mut history = History::new(conversation);
        for page in manifest.pages {
            let bytes = bounded_read(
                &dir.join(format!("{}.json", page.number)),
                MAX_READ_BYTES as u64,
            )
            .context("history incomplete: missing or unreadable page")?;
            if bytes.len() != page.bytes || format!("{:x}", Sha256::digest(&bytes)) != page.hash {
                bail!("history incomplete: corrupt page");
            }
            let entries: Vec<super::history::Entry> =
                serde_json::from_slice(&bytes).context("history incomplete: invalid page")?;
            if entries.is_empty() || entries.len() > MAX_READ_ENTRIES {
                bail!("history incomplete: invalid page count");
            }
            for entry in entries {
                if entry.source.conversation != conversation
                    || history.entries.iter().any(|e| e.source == entry.source)
                {
                    bail!("history incomplete: invalid source identity");
                }
                history.entries.push(entry);
            }
        }
        history.revision = manifest.revision;
        history.completeness = manifest.completeness;
        Ok(Some(history))
    }
}
fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>> {
    use std::io::Read;
    let file = std::fs::File::open(path)?;
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .context("reading history file")?;
    if bytes.len() as u64 > limit {
        bail!("history file exceeds size limit");
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
