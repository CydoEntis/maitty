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

/// Reasoning efforts both providers' CLIs accept, lowest first.
pub const TURN_EFFORTS: [&str; 5] = ["low", "medium", "high", "xhigh", "max"];
/// Longest model name a message may carry.
pub const MAX_MODEL_CHARS: usize = 64;

/// What the user picked in the chat for the next message: a model and a reasoning effort. A field
/// left `None` keeps what the conversation already uses. Permissions are never part of it (ADR 0005).
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TurnChoice {
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub effort: Option<String>,
}

impl TurnChoice {
    /// Refuses anything that is not a plain model name or a known effort. The values reach a
    /// command line, so a name that could read as an option is refused too.
    pub fn validate(&self) -> Result<(), &'static str> {
        if let Some(model) = &self.model {
            let plain = |c: char| c.is_ascii_alphanumeric() || ".-_:/[]".contains(c);
            if model.is_empty()
                || model.len() > MAX_MODEL_CHARS
                || model.starts_with('-')
                || !model.chars().all(plain)
            {
                return Err("The model name is not valid");
            }
        }
        if let Some(effort) = &self.effort
            && !TURN_EFFORTS.contains(&effort.as_str())
        {
            return Err("The reasoning effort is not one the agents accept");
        }
        Ok(())
    }

    /// Nothing picked: the message goes out as it always did.
    pub fn is_empty(&self) -> bool {
        self.model.is_none() && self.effort.is_none()
    }
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

/// How a conversation's agent is set up, as its own file says: for Claude, the permission mode and the model
/// answering. `None` for a field the file has not said (yet); never guessed.
#[derive(Debug, Clone, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct SessionMeta {
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
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
    /// The most recent approvals a person already answered, oldest first, so the answer
    /// survives a restart of the viewing app.
    #[serde(default)]
    pub answered_approvals: Vec<AnsweredApproval>,
    #[serde(default)]
    pub cancelling: bool,
    /// Messages sent while a turn was running, held by the daemon in order and delivered once
    /// each when the turn ends; the window may close and they are still delivered.
    #[serde(default)]
    pub queued: Vec<QueuedMessage>,
    /// The agent's mode and model from its own file, for a conversation that follows an agent in a pane.
    #[serde(default)]
    pub meta: Option<SessionMeta>,
}

/// A message waiting for the running turn to end.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedMessage {
    /// The sender's operation number, which names the message for `ConversationDequeue`.
    pub operation: u64,
    pub text: String,
}

/// The answer to a send that was queued instead of delivered.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QueuedReceipt {
    pub conversation: ConversationId,
    pub generation: u64,
    pub operation: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnsweredApproval {
    pub request: ApprovalRequest,
    pub decision: ApprovalDecision,
    /// Unix milliseconds when the daemon relayed the answer to the provider.
    #[serde(default)]
    pub answered_at_ms: Option<u64>,
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
    /// A command request the person may approve "always in this project". Absent from a peer that
    /// predates it, which then offers no such choice.
    #[serde(default)]
    pub can_remember: bool,
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
    Gemini,
}

impl Provider {
    pub const ALL: [Provider; 3] = [Provider::Codex, Provider::Claude, Provider::Gemini];

    /// The agent's command, which is also its name in a terminal pane.
    pub fn agent_name(self) -> &'static str {
        match self {
            Provider::Codex => "codex",
            Provider::Claude => "claude",
            Provider::Gemini => "gemini",
        }
    }
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
    /// The name you gave it, shared by every client (None: each client words it itself).
    #[serde(default)]
    pub title: Option<String>,
    /// Put away: hidden from the lists, still running and still in the daemon.
    #[serde(default)]
    pub archived: bool,
}

/// Longest name a conversation may carry.
pub const MAX_TITLE_CHARS: usize = 80;

/// A name as it is stored: no control characters, trimmed, at most [`MAX_TITLE_CHARS`] long.
/// Nothing left means "no name".
pub fn clean_title(raw: &str) -> Option<String> {
    let text: String = raw.chars().filter(|c| !c.is_control()).collect();
    let text: String = text.trim().chars().take(MAX_TITLE_CHARS).collect();
    let text = text.trim_end().to_string();
    (!text.is_empty()).then_some(text)
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
    /// Fence a writer that was running when the daemon went away. Returns whether this record
    /// was one: its generation moved and it now needs recovery.
    pub fn recover(&mut self) -> Result<bool, IdentityError> {
        if matches!(
            self.state,
            RecoveryState::Running | RecoveryState::Transferring
        ) {
            self.generation = self
                .generation
                .checked_add(1)
                .ok_or(IdentityError::GenerationExhausted)?;
            self.state = RecoveryState::RecoveryRequired;
            return Ok(true);
        }
        Ok(false)
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
mod turn_choice_tests {
    use super::TurnChoice;

    fn choice(model: Option<&str>, effort: Option<&str>) -> TurnChoice {
        TurnChoice { model: model.map(str::to_owned), effort: effort.map(str::to_owned) }
    }

    #[test]
    fn plain_models_and_known_efforts_pass() {
        assert!(choice(Some("opus"), Some("max")).validate().is_ok());
        assert!(choice(Some("claude-sonnet-4.5[1m]"), None).validate().is_ok());
        assert!(choice(None, None).is_empty());
    }

    #[test]
    fn anything_that_could_read_as_an_option_or_is_unknown_is_refused() {
        for model in ["", "--dangerously-skip-permissions", "a b", "a\nb", &"x".repeat(65)] {
            assert!(choice(Some(model), None).validate().is_err(), "{model:?}");
        }
        assert!(choice(None, Some("ultra-extreme")).validate().is_err());
    }
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
            title: None,
            archived: false,
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
    fn a_read_from_before_answered_approvals_still_decodes() {
        let read: ReadResult = serde_json::from_value(serde_json::json!({
            "conversation": 1,
            "generation": 2,
            "status": "Working",
            "receipt": null,
            "history": { "entries": [], "next": null, "completeness": "Complete" },
            "error": null,
            "checks": [],
            "approvals": []
        }))
        .unwrap();
        assert!(read.answered_approvals.is_empty());
    }

    #[test]
    fn a_peer_that_predates_gemini_still_reads_the_providers_it_knew() {
        for (wire, provider) in [("\"Codex\"", Provider::Codex), ("\"Claude\"", Provider::Claude)] {
            assert_eq!(serde_json::from_str::<Provider>(wire).unwrap(), provider);
        }
        assert_eq!(serde_json::to_string(&Provider::Gemini).unwrap(), "\"Gemini\"");
        assert_eq!(serde_json::from_str::<Provider>("\"Gemini\"").unwrap(), Provider::Gemini);
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

#[cfg(test)]
mod title_tests {
    use super::*;

    #[test]
    fn a_title_is_trimmed_stripped_of_control_characters_and_capped() {
        assert_eq!(clean_title("  Fix login \n").as_deref(), Some("Fix login"));
        assert_eq!(clean_title("a\u{7}b").as_deref(), Some("ab"));
        assert_eq!(clean_title(" \t "), None);
        assert_eq!(clean_title(""), None);
        let long = "x".repeat(MAX_TITLE_CHARS + 20);
        assert_eq!(clean_title(&long).unwrap().chars().count(), MAX_TITLE_CHARS);
    }

    #[test]
    fn an_older_saved_conversation_without_a_title_still_loads() {
        let mut value = serde_json::to_value(super::tests::conversation()).unwrap();
        let map = value.as_object_mut().unwrap();
        map.remove("title");
        map.remove("archived");
        let loaded: Conversation = serde_json::from_value(value).unwrap();
        assert_eq!(loaded.title, None);
        assert!(!loaded.archived);
    }
}

#[cfg(test)]
mod provider_tests {
    use super::*;

    #[test]
    fn each_provider_is_named_as_its_agent_command() {
        let names: Vec<_> = Provider::ALL.iter().map(|p| p.agent_name()).collect();
        assert_eq!(names, ["codex", "claude", "gemini"]);
    }
}
