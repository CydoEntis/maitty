mod branch_start;
pub(super) mod checks;
mod claude;
pub(super) mod commands;
mod daemon;
pub(super) mod follow;
mod follow_transcript;
pub(super) mod history;
mod history_provider;
pub(super) mod news;
mod tool_input;
pub(super) mod history_storage;
mod process;
mod redact;
mod runtime;
pub(super) mod transport;

/// `text` with token- and key-shaped strings masked, for text that leaves the daemon outside a
/// conversation's history (a permission prompt's subject).
pub(in crate::daemon) fn mask_secrets(text: &str) -> String {
    redact::redact_text(text).into_owned()
}

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
    /// Approvals a person already answered, oldest first and capped, so a viewer that restarts
    /// can still show what was decided. Held in memory only: they follow the daemon's life.
    pub(super) answered_approvals: std::collections::VecDeque<crate::conversation::AnsweredApproval>,
}

/// How many answered approvals a conversation keeps; the oldest are dropped first.
const MAX_ANSWERED_APPROVALS: usize = 64;

impl State {
    /// Remembers that `request` was answered with `decision`.
    pub(super) fn record_answer(
        &mut self,
        request: crate::conversation::ApprovalRequest,
        decision: crate::conversation::ApprovalDecision,
    ) {
        let answered_at_ms = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .ok()
            .and_then(|elapsed| u64::try_from(elapsed.as_millis()).ok());
        if self.answered_approvals.len() >= MAX_ANSWERED_APPROVALS {
            self.answered_approvals.pop_front();
        }
        self.answered_approvals.push_back(crate::conversation::AnsweredApproval {
            request,
            decision,
            answered_at_ms,
        });
    }

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
