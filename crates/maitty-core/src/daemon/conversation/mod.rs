pub(super) mod checks;
mod claude;
pub(super) mod commands;
mod daemon;
pub(super) mod history;
mod history_provider;
pub(super) mod history_storage;
mod process;
mod runtime;
pub(super) mod transport;

pub(in crate::daemon) use runtime::Owner;
use runtime::Payload;
pub(super) use runtime::{Event, Registry};
#[cfg(test)]
pub(super) use transport::Launch;

#[derive(Default)]
pub(super) struct State {
    pub(super) status: crate::protocol::Status,
    pub(super) checks: crate::daemon::check::Checks,
    pub(super) completed_turns: std::collections::BTreeSet<String>,
    pub(super) error: Option<String>,
    pub(super) blocked: bool,
    pub(super) history_ready: bool,
    pub(super) history_revision: u64,
    pub(super) started_by: Option<crate::protocol::TermId>,
    pub(super) started_at: Option<std::time::Instant>,
    pub(super) approvals: std::collections::BTreeMap<
        crate::conversation::ProviderRequestId,
        crate::conversation::ApprovalRequest,
    >,
    pub(super) file_change_previews:
        std::collections::BTreeMap<(String, String), Vec<crate::conversation::FileChangePreview>>,
    pub(super) unsupported_requests:
        std::collections::BTreeMap<crate::conversation::ProviderRequestId, String>,
    pub(super) cancelling: Option<String>,
}

impl State {
    pub(super) fn effective_status(&self) -> crate::protocol::Status {
        use crate::protocol::Status;
        if self.error.is_some()
            || (matches!(self.status, Status::Done | Status::Idle) && self.checks.holds_done())
        {
            Status::Blocked
        } else {
            self.status
        }
    }
}

#[cfg(test)]
mod tests;
