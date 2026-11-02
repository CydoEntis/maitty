//! Durable identity metadata, independent of a terminal or provider connection.

use serde::{Deserialize, Serialize};
use std::path::{Component, Path, PathBuf};

pub mod history;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LaunchSettings {
    #[serde(default)]
    pub provider: Provider,
    pub executable: PathBuf,
    pub config_root: PathBuf,
    pub cwd: PathBuf,
    pub model: Option<String>,
    pub model_provider: Option<String>,
    pub approval_policy: ApprovalPolicy,
    pub sandbox: Sandbox,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalPolicy {
    Untrusted,
    OnRequest,
    Never,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Sandbox {
    ReadOnly,
    WorkspaceWrite,
    /// No sandbox. Only chosen explicitly, together with an approval policy of Never.
    FullAccess,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum StartTarget {
    Create(LaunchSettings),
    Session {
        settings: LaunchSettings,
        thread_id: String,
    },
    Conversation {
        id: ConversationId,
        generation: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingStart {
    pub id: ConversationId,
    pub generation: u64,
    pub settings: LaunchSettings,
    pub thread_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Delivery {
    SubmissionUncertain,
    Acknowledged { turn_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Receipt {
    pub conversation: ConversationId,
    pub generation: u64,
    pub operation: u64,
    pub delivery: Delivery,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Operation {
    pub receipt: Receipt,
    pub digest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReadResult {
    pub conversation: ConversationId,
    pub generation: u64,
    pub status: crate::protocol::Status,
    pub receipt: Option<Receipt>,
    pub history: history::Page,
    pub error: Option<String>,
    pub checks: Vec<crate::protocol::CheckInfo>,
    pub approvals: Vec<ApprovalRequest>,
    #[serde(default)]
    pub cancelling: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationDiff {
    pub files: Vec<ConversationDiffFile>,
    pub additions: usize,
    pub deletions: usize,
    pub total_files: usize,
    pub truncated: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationDiffFile {
    pub path: String,
    pub additions: usize,
    pub deletions: usize,
    pub patch: String,
    pub binary: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ConversationFileEntry {
    pub name: String,
    pub is_directory: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovalRequest {
    pub request_id: ProviderRequestId,
    pub kind: ApprovalRequestKind,
    pub item_id: Option<String>,
    pub turn_id: Option<String>,
    pub command: Option<String>,
    pub reason: Option<String>,
    pub cwd: Option<PathBuf>,
    pub changes: Vec<FileChangePreview>,
    pub questions: Vec<UserInputQuestion>,
    #[serde(default)]
    pub permissions: Option<serde_json::Value>,
    pub choices: Vec<ApprovalDecision>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FileChangePreview {
    pub path: String,
    pub kind: String,
    pub diff: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInputQuestion {
    pub id: String,
    pub header: String,
    pub question: String,
    pub is_other: bool,
    pub is_secret: bool,
    pub options: Vec<UserInputOption>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserInputOption {
    pub label: String,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalRequestKind {
    Command,
    FileChange,
    Permission,
    UserInput,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ApprovalDecision {
    Accept,
    AcceptForSession,
    Decline,
    Cancel,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(untagged)]
pub enum ProviderRequestId {
    Integer(i64),
    Text(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConversationId(pub u64);

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Provider {
    #[default]
    Codex,
    Claude,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct NativeBinding {
    pub provider: Provider,
    pub session_id: String,
    pub account: Option<String>,
    pub profile: Option<String>,
    pub config_root: PathBuf,
    pub executable: PathBuf,
    pub cwd: PathBuf,
    pub worktree: Option<PathBuf>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum ExecutionKind {
    Native,
    #[default]
    Structured,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecoveryState {
    Stopped,
    Running,
    Transferring,
    RecoveryRequired,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conversation {
    pub id: ConversationId,
    pub binding: NativeBinding,
    pub execution: ExecutionKind,
    pub generation: u64,
    pub state: RecoveryState,
    #[serde(default)]
    pub settings: Option<LaunchSettings>,
    #[serde(default)]
    pub awaiting_first_user_message: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IdentityError {
    IncompleteBinding,
    DuplicateId,
    DuplicateNativeSession,
    GenerationExhausted,
    InvalidOperation,
}

impl Conversation {
    pub fn recover(&mut self) -> Result<(), IdentityError> {
        if matches!(
            self.state,
            RecoveryState::Running | RecoveryState::Transferring
        ) {
            self.generation = self
                .generation
                .checked_add(1)
                .ok_or(IdentityError::GenerationExhausted)?;
            self.state = RecoveryState::RecoveryRequired;
        }
        Ok(())
    }
}

fn exact_path(path: &Path) -> bool {
    path.is_absolute()
        && !path
            .components()
            .any(|part| matches!(part, Component::ParentDir | Component::CurDir))
}

fn identity_text(text: &str) -> bool {
    !text.trim().is_empty() && !text.chars().any(char::is_control)
}

pub fn validate_settings(settings: &LaunchSettings) -> Result<(), IdentityError> {
    if ![&settings.config_root, &settings.executable, &settings.cwd]
        .into_iter()
        .all(|p| exact_path(p))
        || settings.model.as_deref().is_some_and(|s| !identity_text(s))
        || settings
            .model_provider
            .as_deref()
            .is_some_and(|s| !identity_text(s))
    {
        return Err(IdentityError::IncompleteBinding);
    }
    Ok(())
}

pub fn validate_metadata(
    records: &[Conversation],
    pending: &[PendingStart],
    operations: &[Operation],
) -> Result<(), IdentityError> {
    validate_registry(records)?;
    for (index, start) in pending.iter().enumerate() {
        validate_settings(&start.settings)?;
        if start.id.0 == 0
            || start.generation == 0
            || start
                .thread_id
                .as_deref()
                .is_some_and(|s| !identity_text(s))
        {
            return Err(IdentityError::IncompleteBinding);
        }
        if records.iter().any(|c| c.id == start.id)
            || pending[..index].iter().any(|p| p.id == start.id)
        {
            return Err(IdentityError::DuplicateId);
        }
    }
    for (index, operation) in operations.iter().enumerate() {
        let r = &operation.receipt;
        if r.operation == 0
            || r.generation == 0
            || !records
                .iter()
                .any(|c| c.id == r.conversation && c.generation >= r.generation)
            || operation.digest.len() != 64
            || !operation.digest.bytes().all(|b| b.is_ascii_hexdigit())
            || matches!(&r.delivery,Delivery::Acknowledged {turn_id} if !identity_text(turn_id))
            || operations[..index]
                .iter()
                .any(|o| o.receipt.conversation == r.conversation)
        {
            return Err(IdentityError::InvalidOperation);
        }
    }
    Ok(())
}

pub fn validate_registry(records: &[Conversation]) -> Result<(), IdentityError> {
    for (index, record) in records.iter().enumerate() {
        let binding = &record.binding;
        if let Some(settings) = &record.settings {
            validate_settings(settings)?;
            if settings.config_root != binding.config_root
                || settings.executable != binding.executable
                || settings.cwd != binding.cwd
                || settings.provider != binding.provider
            {
                return Err(IdentityError::IncompleteBinding);
            }
        }
        if record.id.0 == 0
            || record.generation == 0
            || !identity_text(&binding.session_id)
            || binding
                .account
                .as_deref()
                .is_some_and(|s| !identity_text(s))
            || binding
                .profile
                .as_deref()
                .is_some_and(|s| !identity_text(s))
            || ![&binding.config_root, &binding.executable, &binding.cwd]
                .into_iter()
                .all(|p| exact_path(p))
            || binding.worktree.as_deref().is_some_and(|p| !exact_path(p))
        {
            return Err(IdentityError::IncompleteBinding);
        }
        for other in &records[..index] {
            if record.id == other.id {
                return Err(IdentityError::DuplicateId);
            }
            // A different cwd or executable does not make a second writer safe.
            if binding.provider == other.binding.provider
                && binding.session_id == other.binding.session_id
                && binding.config_root == other.binding.config_root
            {
                return Err(IdentityError::DuplicateNativeSession);
            }
        }
    }
    Ok(())
}

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn conversation() -> Conversation {
        Conversation {
            id: ConversationId(1),
            binding: NativeBinding {
                provider: Provider::Codex,
                session_id: "exact-session".into(),
                account: Some("work".into()),
                profile: Some("native-profile".into()),
                config_root: std::env::temp_dir().join("synthetic-config"),
                executable: std::env::temp_dir().join("synthetic-codex.exe"),
                cwd: std::env::temp_dir().join("synthetic-repo"),
                worktree: Some(std::env::temp_dir().join("synthetic-tree")),
            },
            execution: ExecutionKind::Structured,
            generation: 7,
            state: RecoveryState::Stopped,
            settings: None,
            awaiting_first_user_message: false,
        }
    }

    #[test]
    fn exact_identity_survives_round_trip_without_a_pane() {
        let c = conversation();
        let back: Conversation = serde_json::from_str(&serde_json::to_string(&c).unwrap()).unwrap();
        assert_eq!(back, c);
        assert!(validate_registry(&[back]).is_ok());

        let mut legacy = serde_json::to_value(&c).unwrap();
        legacy
            .as_object_mut()
            .unwrap()
            .remove("awaiting_first_user_message");
        let legacy: Conversation = serde_json::from_value(legacy).unwrap();
        assert!(!legacy.awaiting_first_user_message);
    }

    #[test]
    fn legacy_launch_settings_default_to_codex() {
        let settings = LaunchSettings {
            provider: Provider::Claude,
            executable: std::env::temp_dir().join("synthetic-claude.exe"),
            config_root: std::env::temp_dir().join("synthetic-claude-config"),
            cwd: std::env::temp_dir().join("synthetic-repo"),
            model: None,
            model_provider: None,
            approval_policy: ApprovalPolicy::OnRequest,
            sandbox: Sandbox::ReadOnly,
        };
        let mut legacy = serde_json::to_value(settings).unwrap();
        legacy.as_object_mut().unwrap().remove("provider");
        let decoded: LaunchSettings = serde_json::from_value(legacy).unwrap();
        assert_eq!(decoded.provider, Provider::Codex);
    }

    #[test]
    fn launch_settings_must_match_the_recorded_provider() {
        let mut conversation = conversation();
        conversation.settings = Some(LaunchSettings {
            provider: Provider::Claude,
            executable: conversation.binding.executable.clone(),
            config_root: conversation.binding.config_root.clone(),
            cwd: conversation.binding.cwd.clone(),
            model: None,
            model_provider: None,
            approval_policy: ApprovalPolicy::OnRequest,
            sandbox: Sandbox::ReadOnly,
        });
        assert_eq!(
            validate_registry(&[conversation]),
            Err(IdentityError::IncompleteBinding)
        );
    }

    #[test]
    fn interrupted_writers_are_fenced_on_restart() {
        for state in [RecoveryState::Running, RecoveryState::Transferring] {
            let mut c = conversation();
            c.state = state;
            c.recover().unwrap();
            assert_eq!(
                (c.generation, c.state),
                (8, RecoveryState::RecoveryRequired)
            );
            c.recover().unwrap();
            assert_eq!(c.generation, 8);
        }
    }

    #[test]
    fn ambiguous_or_incomplete_bindings_are_refused() {
        let c = conversation();
        assert_eq!(
            validate_registry(&[c.clone(), c.clone()]),
            Err(IdentityError::DuplicateId)
        );
        let mut duplicate = c.clone();
        duplicate.id = ConversationId(2);
        duplicate.binding.cwd.push("elsewhere");
        assert_eq!(
            validate_registry(&[c.clone(), duplicate]),
            Err(IdentityError::DuplicateNativeSession)
        );
        let mut incomplete = c;
        incomplete.binding.executable = "codex".into();
        assert_eq!(
            validate_registry(&[incomplete]),
            Err(IdentityError::IncompleteBinding)
        );
    }

    #[test]
    fn exhausted_generations_never_wrap() {
        let mut c = conversation();
        c.generation = u64::MAX;
        c.state = RecoveryState::Transferring;
        assert_eq!(c.recover(), Err(IdentityError::GenerationExhausted));
        assert_eq!(c.generation, u64::MAX);
    }
}
