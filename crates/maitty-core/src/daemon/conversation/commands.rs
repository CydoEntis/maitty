use super::{
    history::{self, Completeness, History},
    runtime::Rpc,
    transport::{Launch, LaunchIdentity},
};
use crate::conversation::*;
use crate::daemon::{ClientId, Daemon, Grant, ServerMsg, persist};
use crate::protocol::{ConversationSummary, Reply, Status};
use anyhow::{Result, bail};

const MAX_NATIVE_TERMINAL_INPUT_BYTES: usize = 16 * 1024;
const MIN_NATIVE_TERMINAL_DIMENSION: u16 = 2;
const MAX_NATIVE_TERMINAL_COLS: u16 = 500;
const MAX_NATIVE_TERMINAL_ROWS: u16 = 300;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::ffi::OsStr;

const MAX_PROMPT_BYTES: usize = 256 * 1024;
/// Messages the daemon holds for one busy conversation, and their total text.
const MAX_QUEUED_MESSAGES: usize = 32;
const MAX_QUEUED_BYTES: usize = 1024 * 1024;
/// A queued message was sent by a request that is long over; its replies have nowhere to go.
const NO_CLIENT: ClientId = ClientId::MAX;

/// What a message carries on its way to the runtime: its text, the digest that tells a retry from a
/// new message, and the model and effort picked for it.
struct Outgoing {
    text: String,
    digest: String,
    choice: TurnChoice,
}

/// A message waiting for the running turn to end.
pub(in crate::daemon) struct QueuedSend {
    generation: u64,
    operation: u64,
    text: String,
    digest: String,
    choice: TurnChoice,
}

/// The provider's `turn/start` parameters. Only what the user picked is added; the providers
/// keep a model and effort for the turns after, so a choice made once holds until changed.
fn send_params(
    thread: &str,
    text: &str,
    client_message_id: &str,
    choice: &TurnChoice,
) -> Value {
    let mut params = json!({"threadId":thread,"input":[{"type":"text","text":text,"text_elements":[]}],"clientUserMessageId":client_message_id});
    if let Some(model) = &choice.model {
        params["model"] = json!(model);
    }
    if let Some(effort) = &choice.effort {
        params["effort"] = json!(effort);
    }
    params
}
const MAX_HYDRATION_ENTRIES: usize = 100_000;
const MAX_HYDRATION_BYTES: usize = 32 * 1024 * 1024;
const MAX_USER_INPUT_ANSWER_CHARS: usize = 10_000;

fn valid_provider_environment_entry(key: &OsStr, value: &OsStr) -> bool {
    let key = key.to_string_lossy();
    !key.is_empty() && !key.contains(['=', '\0']) && !value.to_string_lossy().contains('\0')
}

fn cached_claude_history(
    id: ConversationId,
    cached: Option<&History>,
    is_currently_reconciled: bool,
) -> Result<(History, bool)> {
    let mut history = cached.cloned().unwrap_or_else(|| History::new(id));
    if !is_currently_reconciled || cached.is_none() {
        history.reconnect().map_err(|reason| {
            anyhow::anyhow!("Cannot mark Claude history incomplete: {reason:?}")
        })?;
    }
    let is_complete = is_currently_reconciled && cached.is_some();
    Ok((history, is_complete))
}

/// The requester of a start the daemon makes itself. No client has this id, so a reply to it is
/// dropped and a failure is only recorded on the conversation and logged.
pub(in crate::daemon) const DAEMON_REQUESTER: ClientId = ClientId::MAX;

pub(in crate::daemon) enum Pending {
    Validating {
        client: ClientId,
        id: ConversationId,
    },
    Launch {
        client: ClientId,
        launch: Box<Launch>,
    },
    Opening {
        client: ClientId,
        id: ConversationId,
        generation: u64,
        settings: LaunchSettings,
        expected: Option<String>,
    },
    Identity {
        client: ClientId,
        id: ConversationId,
        generation: u64,
        created_thread: bool,
    },
    Hydration {
        id: ConversationId,
        generation: u64,
        history: History,
        cursors: BTreeSet<String>,
        read: Option<ReadRequest>,
        started: Option<ClientId>,
        changed: bool,
        bytes: usize,
    },
    Submit {
        client: ClientId,
        receipt: Receipt,
        text: String,
        choice: TurnChoice,
    },
    Sending {
        client: ClientId,
        receipt: Receipt,
    },
    Cancelling {
        client: ClientId,
        id: ConversationId,
        generation: u64,
        turn_id: String,
    },
    Receipt {
        client: ClientId,
        receipt: Receipt,
    },
}

pub(in crate::daemon) struct ReadRequest {
    client: ClientId,
    cursor: Option<history::Cursor>,
    limit: usize,
}

impl Pending {
    pub(super) fn client(&self) -> Option<ClientId> {
        match self {
            Self::Validating { client, .. }
            | Self::Launch { client, .. }
            | Self::Opening { client, .. }
            | Self::Identity { client, .. }
            | Self::Submit { client, .. }
            | Self::Sending { client, .. }
            | Self::Cancelling { client, .. }
            | Self::Receipt { client, .. } => Some(*client),
            Self::Hydration { read, started, .. } => {
                read.as_ref().map(|read| read.client).or(*started)
            }
        }
    }
    pub(in crate::daemon) fn conversation(&self) -> ConversationId {
        match self {
            Self::Launch { launch, .. } => launch.id(),
            Self::Validating { id, .. }
            | Self::Opening { id, .. }
            | Self::Identity { id, .. }
            | Self::Hydration { id, .. } => *id,
            Self::Cancelling { id, .. }
            => *id,
            Self::Submit { receipt, .. }
            | Self::Sending { receipt, .. }
            | Self::Receipt { receipt, .. } => receipt.conversation,
        }
    }
}

fn approval(settings: &LaunchSettings) -> &'static str {
    match settings.approval_policy {
        ApprovalPolicy::Untrusted => "untrusted",
        ApprovalPolicy::OnRequest => "on-request",
        ApprovalPolicy::Never => "never",
    }
}
fn sandbox(settings: &LaunchSettings) -> &'static str {
    match settings.sandbox {
        Sandbox::ReadOnly => "read-only",
        Sandbox::WorkspaceWrite => "workspace-write",
        Sandbox::FullAccess => "danger-full-access",
    }
}

fn params(settings: &LaunchSettings) -> Value {
    json!({"cwd": settings.cwd, "model": settings.model, "modelProvider": settings.model_provider,
        "approvalPolicy": approval(settings), "approvalsReviewer":"user", "sandbox":sandbox(settings),
        "config": {"sandbox_workspace_write": {"network_access":false,"writable_roots":[],"exclude_tmpdir_env_var":true,"exclude_slash_tmp":true}}})
}
fn identifier(value: &Value, key: &str) -> Result<String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|s| !s.trim().is_empty() && !s.chars().any(char::is_control))
        .map(str::to_owned)
        .ok_or_else(|| anyhow::anyhow!("Codex response lacks a valid {key}"))
}
fn validate_response(
    value: &Value,
    settings: &mut LaunchSettings,
    expected: Option<&str>,
) -> Result<String> {
    let thread = value
        .get("thread")
        .ok_or_else(|| anyhow::anyhow!("Codex response lacks thread"))?;
    let id = identifier(thread, "id")?;
    if expected.is_some_and(|expected| expected != id) {
        bail!("Provider resumed a different exact session");
    }
    if settings.provider == Provider::Claude {
        return Ok(id);
    }
    let cwd = identifier(value, "cwd")?;
    if std::path::Path::new(&cwd) != settings.cwd
        || value.get("approvalPolicy") != Some(&json!(approval(settings)))
        || value.get("approvalsReviewer") != Some(&json!("user"))
    {
        bail!("Codex effective working directory or approval posture differs");
    }
    let effective = value
        .get("sandbox")
        .ok_or_else(|| anyhow::anyhow!("Codex response lacks sandbox"))?;
    let kind = match settings.sandbox {
        Sandbox::ReadOnly => "readOnly",
        Sandbox::WorkspaceWrite => "workspaceWrite",
        Sandbox::FullAccess => "dangerFullAccess",
    };
    // Full access has no sandbox, so there is no network restriction to verify.
    if effective.get("type") != Some(&json!(kind))
        || (settings.sandbox != Sandbox::FullAccess
            && effective
                .get("networkAccess")
                .is_some_and(|v| v != &json!(false)))
    {
        bail!("Codex effective sandbox differs");
    }
    if settings.sandbox == Sandbox::WorkspaceWrite {
        let roots = effective.get("writableRoots").and_then(Value::as_array);
        if roots.is_some_and(|roots| {
            roots
                .iter()
                .any(|root| root.as_str().map(std::path::Path::new) != Some(settings.cwd.as_path()))
        }) || effective.get("excludeTmpdirEnvVar") != Some(&json!(true))
            || effective.get("excludeSlashTmp") != Some(&json!(true))
        {
            bail!("Codex effective writable roots differ");
        }
    }
    let model = identifier(value, "model")?;
    let provider = identifier(value, "modelProvider")?;
    if settings.model.as_ref().is_some_and(|m| m != &model)
        || settings
            .model_provider
            .as_ref()
            .is_some_and(|p| p != &provider)
    {
        bail!("Codex effective model selection differs");
    }
    settings.model = Some(model);
    settings.model_provider = Some(provider);
    Ok(id)
}

impl Daemon {
    pub(in crate::daemon) fn conversation_terminal_screen(
        &self,
        id: ConversationId,
        generation: u64,
    ) -> Result<String> {
        let term = self.conversation_terminal_term(id, generation)?;
        let pane = self
            .terms
            .get(&term)
            .ok_or_else(|| anyhow::anyhow!("Native conversation terminal is not running"))?;
        let mut screen = b"\x1b[0m\x1b[2J\x1b[H".to_vec();
        screen.extend(pane.parser.screen().state_formatted());
        Ok(String::from_utf8_lossy(&screen).into_owned())
    }

    pub(in crate::daemon) fn conversation_terminal_replay(
        &self,
        id: ConversationId,
        generation: u64,
    ) -> Result<Vec<u8>> {
        let term = self.conversation_terminal_term(id, generation)?;
        let pane = self
            .terms
            .get(&term)
            .ok_or_else(|| anyhow::anyhow!("Native conversation terminal is not running"))?;
        Ok(pane.replay())
    }

    pub(in crate::daemon) fn conversation_terminal_input(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        data: Vec<u8>,
    ) -> Result<()> {
        anyhow::ensure!(
            !data.is_empty() && data.len() <= MAX_NATIVE_TERMINAL_INPUT_BYTES,
            "Native terminal input must contain between 1 and {MAX_NATIVE_TERMINAL_INPUT_BYTES} bytes"
        );
        let term = self.conversation_terminal_term(id, generation)?;
        let asking = self
            .terms
            .get(&term)
            .is_some_and(|pane| pane.status == crate::protocol::Status::Blocked);
        self.may(client, Grant::Write, Some(term))?;
        if asking {
            self.may(client, Grant::Respond, Some(term))?;
        }
        self.terms
            .get_mut(&term)
            .ok_or_else(|| anyhow::anyhow!("Native conversation terminal is not running"))?
            .input(&data);
        Ok(())
    }

    pub(in crate::daemon) fn conversation_terminal_resize(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        cols: u16,
        rows: u16,
    ) -> Result<()> {
        anyhow::ensure!(
            (MIN_NATIVE_TERMINAL_DIMENSION..=MAX_NATIVE_TERMINAL_COLS).contains(&cols)
                && (MIN_NATIVE_TERMINAL_DIMENSION..=MAX_NATIVE_TERMINAL_ROWS).contains(&rows),
            "Native terminal size must be between 2x2 and 500x300"
        );
        let term = self.conversation_terminal_term(id, generation)?;
        self.may(client, Grant::Write, Some(term))?;
        self.terms
            .get_mut(&term)
            .ok_or_else(|| anyhow::anyhow!("Native conversation terminal is not running"))?
            .resize(cols, rows);
        self.dirty = true;
        Ok(())
    }

    fn conversation_terminal_term(
        &self,
        id: ConversationId,
        generation: u64,
    ) -> Result<crate::protocol::TermId> {
        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        anyhow::ensure!(
            conversation.generation == generation,
            "Native terminal request is stale"
        );
        anyhow::ensure!(
            conversation.execution == crate::conversation::ExecutionKind::Native,
            "Conversation is not owned by a native terminal"
        );
        let pane = self
            .terms
            .values()
            .find(|pane| pane.conversation_id == Some(id) && !pane.asleep)
            .ok_or_else(|| anyhow::anyhow!("Native conversation terminal is not running"))?;
        Ok(pane.id)
    }

    fn visible_receipt(&self, id: ConversationId) -> Option<Receipt> {
        let mut receipt = self
            .conversation_operations
            .iter()
            .find(|operation| operation.receipt.conversation == id)?
            .receipt
            .clone();
        if self.conversation_requests.values().any(
            |pending| matches!(pending,Pending::Receipt {receipt:staged,..} if *staged==receipt),
        ) {
            receipt.delivery = Delivery::SubmissionUncertain;
        }
        Some(receipt)
    }
    fn conversation_token(&mut self, pending: Pending) -> Result<u64> {
        self.conversation_request_id = self
            .conversation_request_id
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Conversation operation identity exhausted"))?;
        let token = self.conversation_request_id;
        self.conversation_requests.insert(token, pending);
        Ok(token)
    }
    fn durable_conversation(&self) -> Result<()> {
        if !self.cfg.restore.enabled {
            bail!("Structured mutations require restore.enabled for durable delivery metadata");
        }
        if !self.session_path.is_absolute() {
            bail!("Structured persistence requires an explicit absolute session path");
        }
        Ok(())
    }
    pub(in crate::daemon) fn conversation_start(
        &mut self,
        client: ClientId,
        target: StartTarget,
    ) -> Result<()> {
        if client == DAEMON_REQUESTER && !matches!(target, StartTarget::Conversation { .. }) {
            bail!("The daemon may only resume an existing conversation");
        }
        self.may(client, Grant::Start, None)?;
        self.durable_conversation()?;
        let by = self.starting(client, "a structured Codex conversation")?;
        let identity = match target {
            StartTarget::Create(settings)
            | StartTarget::Session {
                settings,
                thread_id: _,
            } if !settings.cwd.is_absolute()
                || !settings.config_root.is_absolute()
                || !settings.executable.is_absolute() =>
            {
                bail!("Conversation paths must be absolute")
            }
            StartTarget::Create(settings) => {
                validate_settings(&settings)
                    .map_err(|_| anyhow::anyhow!("Invalid structured launch settings"))?;
                let id = self.next_conversation_id()?;
                let pending = PendingStart {
                    id,
                    generation: 1,
                    settings,
                    thread_id: None,
                };
                self.pending_starts.push(pending.clone());
                LaunchIdentity::Pending(pending)
            }
            StartTarget::Session {
                settings,
                thread_id,
            } => {
                validate_settings(&settings)
                    .map_err(|_| anyhow::anyhow!("Invalid structured launch settings"))?;
                if thread_id.trim().is_empty() || thread_id.chars().any(char::is_control) {
                    bail!("An exact nonempty thread ID is required");
                }
                if self.conversations.iter().any(|c| {
                    c.binding.provider == settings.provider
                        && c.binding.session_id == thread_id
                        && c.binding.config_root == settings.config_root
                }) || self.pending_starts.iter().any(|p| {
                    p.settings.provider == settings.provider
                        && p.thread_id.as_ref() == Some(&thread_id)
                        && p.settings.config_root == settings.config_root
                }) {
                    bail!(concat!(
                        "Exact thread is already owned by ",
                        env!("APP_COMMAND"),
                        "; use --conversation"
                    ));
                }
                let pending = PendingStart {
                    id: self.next_conversation_id()?,
                    generation: 1,
                    settings,
                    thread_id: Some(thread_id),
                };
                self.pending_starts.push(pending.clone());
                LaunchIdentity::Pending(pending)
            }
            StartTarget::Conversation { id, generation } => {
                if self.conversation_runtimes.owns(id)
                    || self
                        .conversation_requests
                        .values()
                        .any(|p| p.conversation() == id)
                {
                    bail!("Conversation already owns a runtime or pending operation");
                }
                if self.terms.values().any(|t| t.conversation_id == Some(id)) {
                    bail!("A native terminal still owns this conversation");
                }
                let c = self.conversations.iter_mut().find(|c| c.id == id).ok_or_else(|| anyhow::anyhow!("No complete conversation with that ID; unresolved creation requires investigation"))?;
                if c.generation != generation {
                    bail!("Stale conversation generation");
                }
                if c.execution != ExecutionKind::Structured {
                    bail!("Native conversation history must be reconciled before chat takeover");
                }
                if c.settings.is_none() {
                    bail!("Conversation has no verified structured launch settings");
                }
                c.generation = c
                    .generation
                    .checked_add(1)
                    .ok_or_else(|| anyhow::anyhow!("Conversation generation exhausted"))?;
                c.state = RecoveryState::Stopped;
                LaunchIdentity::Existing(Box::new(c.clone()))
            }
        };
        let launch = Launch {
            identity,
            environment: std::env::vars_os()
                .filter(|(key, value)| {
                    let valid_entry = valid_provider_environment_entry(key, value);
                    let key = key.to_string_lossy();
                    valid_entry
                        && !crate::ipc::STRUCTURED_ENV
                            .iter()
                            .any(|excluded| key.eq_ignore_ascii_case(excluded))
                        && ![
                            concat!(env!("APP_ENV"), "_TERM_ID"),
                            concat!(env!("APP_ENV"), "_PANE_TOKEN"),
                            "CODEX_HOME",
                            "CLAUDE_CONFIG_DIR",
                        ]
                        .iter()
                        .any(|excluded| key.eq_ignore_ascii_case(excluded))
                })
                .map(|(key, value)| match crate::login_path::resolved() {
                    Some(path) if key.eq_ignore_ascii_case("PATH") => (key, path.to_os_string()),
                    _ => (key, value),
                })
                .collect(),
            #[cfg(test)]
            failure_point: None,
        };
        let id = launch.id();
        let state = self.conversation_states.entry(id).or_default();
        state.status = Status::Working;
        state.started_by = by.or(state.started_by);
        state.started_at = Some(std::time::Instant::now());
        let token = self.conversation_token(Pending::Validating { client, id })?;
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let result = launch.validate().await.map_err(|e| e.to_string());
            // A closed daemon has no consumer for the completed admission.
            let _ = tx
                .send(crate::daemon::Ev::ConversationValidated { token, result })
                .await;
        });
        self.dirty = true;
        Ok(())
    }
    fn next_conversation_id(&self) -> Result<ConversationId> {
        let last = self
            .conversations
            .iter()
            .map(|c| c.id.0)
            .chain(self.pending_starts.iter().map(|p| p.id.0))
            .chain(self.followed.iter().map(|f| f.id.0))
            .chain(std::iter::once(self.followed_high_water))
            .max()
            .unwrap_or(0);
        last.checked_add(1)
            .map(ConversationId)
            .ok_or_else(|| anyhow::anyhow!("Conversation identity exhausted"))
    }
    pub(in crate::daemon) fn conversation_validated(
        &mut self,
        token: u64,
        result: std::result::Result<Launch, String>,
    ) {
        let Some(pending) = self.conversation_requests.remove(&token) else {
            return;
        };
        let Pending::Validating { client, id } = &pending else {
            self.conversation_fail(pending, "Unexpected admission stage".into());
            return;
        };
        let (client, id) = (*client, *id);
        let launch = match result {
            Ok(launch) => launch,
            Err(error) => {
                self.conversation_fail(pending, error);
                return;
            }
        };
        let valid = match &launch.identity {
            LaunchIdentity::Pending(p) => {
                let duplicate = p.thread_id.as_ref().is_some_and(|thread| {
                    self.conversations.iter().any(|c| {
                        c.binding.provider == p.settings.provider
                            && c.binding.session_id == *thread
                            && c.binding.config_root == p.settings.config_root
                    }) || self.pending_starts.iter().any(|other| {
                        other.id != id
                            && other.settings.provider == p.settings.provider
                            && other.thread_id.as_ref() == Some(thread)
                            && other.settings.config_root == p.settings.config_root
                    })
                });
                if let Some(record) = self
                    .pending_starts
                    .iter_mut()
                    .find(|record| record.id == id)
                {
                    *record = p.clone();
                }
                !duplicate
            }
            LaunchIdentity::Existing(c) => {
                if let Some(record) = self
                    .conversations
                    .iter_mut()
                    .find(|record| record.id == id && record.generation == c.generation)
                {
                    *record = (**c).clone();
                    true
                } else {
                    false
                }
            }
        };
        if !valid {
            self.conversation_fail(
                pending,
                "Conversation identity is duplicate or stale after path resolution".into(),
            );
            return;
        }
        self.conversation_requests.insert(
            token,
            Pending::Launch {
                client,
                launch: Box::new(launch),
            },
        );
        if let Err(error) = self.conversation_persist(token) {
            self.send(client, ServerMsg::Error(error.to_string()));
        }
    }

    fn conversation_persist(&mut self, token: u64) -> Result<()> {
        if let Err(error) = self.persist_barrier(token) {
            if let Some(pending) = self.conversation_requests.remove(&token) {
                self.conversation_fail(pending, error.to_string());
            }
            return Err(error);
        }
        Ok(())
    }
    pub(in crate::daemon) fn conversation_approve(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        request_id: ProviderRequestId,
        decision: ApprovalDecision,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can answer provider approval requests");
        }
        self.may(client, Grant::Respond, None)?;
        self.answer_approval(id, generation, request_id, decision)?;
        self.send(client, ServerMsg::Reply(Reply::Ok));
        Ok(())
    }

    /// Answers a provider approval request with `decision`, after whoever asks has been checked. The
    /// request must belong to the acknowledged active turn of a running conversation.
    pub(in crate::daemon) fn answer_approval(
        &mut self,
        id: ConversationId,
        generation: u64,
        request_id: ProviderRequestId,
        decision: ApprovalDecision,
    ) -> Result<()> {
        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if conversation.generation != generation || conversation.state != RecoveryState::Running {
            bail!("Approval request belongs to a stale or stopped conversation");
        }
        let active_turn = self
            .conversation_operations
            .iter()
            .find(|operation| operation.receipt.conversation == id)
            .and_then(|operation| match &operation.receipt.delivery {
                crate::conversation::Delivery::Acknowledged { turn_id } => Some(turn_id.as_str()),
                crate::conversation::Delivery::SubmissionUncertain => None,
            });
        let state = self
            .conversation_states
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation has no active approval request"))?;
        let approval = state
            .approvals
            .get(&request_id)
            .ok_or_else(|| anyhow::anyhow!("Approval request is stale or already answered"))?;
        if approval.turn_id.as_deref() != active_turn {
            bail!("Approval request does not belong to the acknowledged active turn");
        }
        if !matches!(
            approval.kind,
            crate::conversation::ApprovalRequestKind::Command
                | crate::conversation::ApprovalRequestKind::FileChange
                | crate::conversation::ApprovalRequestKind::Permission
        ) {
            bail!("This provider request kind does not support approval decisions");
        }
        if !approval.choices.contains(&decision) {
            bail!("That decision is not available for this provider request");
        }
        let response = if approval.kind == crate::conversation::ApprovalRequestKind::Permission {
            let permissions = match decision {
                ApprovalDecision::Accept => approval.permissions.clone().ok_or_else(|| {
                    anyhow::anyhow!("Permission request has no requested profile")
                })?,
                ApprovalDecision::Decline => json!({}),
                ApprovalDecision::AcceptForSession | ApprovalDecision::Cancel => {
                    bail!("This permission request supports only a turn-scoped approval")
                }
            };
            json!({ "permissions": permissions, "scope": "turn" })
        } else {
            let result = match decision {
                ApprovalDecision::Accept => "accept",
                ApprovalDecision::AcceptForSession => "acceptForSession",
                ApprovalDecision::Decline => "decline",
                ApprovalDecision::Cancel => "cancel",
            };
            json!({ "decision": result })
        };
        self.conversation_runtimes.request(
            id,
            generation,
            0,
            Rpc::Answer {
                request_id: request_id.clone().into(),
                response,
            },
        )?;
        if let Some(state) = self.conversation_states.get_mut(&id) {
            state.command_hashes.remove(&request_id);
            if let Some(answered) = state.approvals.remove(&request_id) {
                state.record_answer(answered, decision);
            }
            state.blocked = !state.approvals.is_empty() || !state.unsupported_requests.is_empty();
            state.status = if state.blocked {
                Status::Blocked
            } else {
                Status::Working
            };
            state.error = state.unsupported_requests.values().next().cloned();
        }
        self.dirty = true;
        Ok(())
    }

    /// A person approves a command request and has it remembered for the conversation's project
    /// (ADR 0005): the same command is then approved without asking. Only a command request can be
    /// remembered, and only by a human.
    pub(in crate::daemon) fn conversation_allow_always(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        request_id: ProviderRequestId,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can answer provider approval requests");
        }
        self.may(client, Grant::Respond, None)?;
        let state = self.conversation_states.get(&id);
        let approval = state
            .and_then(|state| state.approvals.get(&request_id))
            .ok_or_else(|| anyhow::anyhow!("Approval request is stale or already answered"))?;
        let (Some(hash), true) = (
            state.and_then(|state| state.command_hashes.get(&request_id)).cloned(),
            approval.kind == crate::conversation::ApprovalRequestKind::Command
                && approval.can_remember,
        ) else {
            bail!("Only a command request can be approved for the project");
        };
        let shown = approval.command.clone().unwrap_or_default();
        let project = self.conversation_project(id)?;
        // Answer first: a request that cannot be answered now is never remembered.
        self.answer_approval(id, generation, request_id, ApprovalDecision::Accept)?;
        let rule = super::allowed::Rule {
            project,
            command_sha256: hash,
            command: shown,
            at: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_secs()),
        };
        self.chat_allowed.add(rule.clone());
        let file = self.chat_allowed_file.clone();
        tokio::task::spawn_blocking(move || {
            if let Err(e) = super::allowed::remember(&file, rule) {
                tracing::warn!("saving the commands approved for a project: {e:#}");
            }
        });
        self.send(client, ServerMsg::Reply(Reply::Ok));
        Ok(())
    }

    /// The project a conversation belongs to: its repository's main checkout, else its folder.
    pub(in crate::daemon) fn conversation_project(&self, id: ConversationId) -> Result<std::path::PathBuf> {
        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        let folder = conversation
            .binding
            .worktree
            .as_deref()
            .unwrap_or(&conversation.binding.cwd);
        Ok(crate::gitfs::head(folder).map_or_else(|| folder.to_path_buf(), |head| head.main_root))
    }

    /// Answers a command request at once when its project approved exactly that command before.
    /// Called as a request arrives; a request that is not remembered is left for the person.
    pub(in crate::daemon) fn auto_approve_remembered(
        &mut self,
        id: ConversationId,
        request_id: &ProviderRequestId,
    ) {
        let Some(state) = self.conversation_states.get(&id) else {
            return;
        };
        let Some(approval) = state.approvals.get(request_id) else {
            return;
        };
        let (Some(hash), true) = (
            state.command_hashes.get(request_id).cloned(),
            approval.kind == crate::conversation::ApprovalRequestKind::Command
                && approval.choices.contains(&ApprovalDecision::Accept),
        ) else {
            return;
        };
        let (Ok(project), Some(generation)) = (
            self.conversation_project(id),
            self.conversations
                .iter()
                .find(|conversation| conversation.id == id)
                .map(|conversation| conversation.generation),
        ) else {
            return;
        };
        if !self.chat_allowed.allows_hash(&project, &hash) {
            return;
        }
        if let Err(error) = self.answer_approval(id, generation, request_id.clone(), ApprovalDecision::Accept) {
            // Left for the person to answer by hand.
            tracing::warn!("a remembered command approval could not be answered: {error:#}");
        }
    }

    pub(in crate::daemon) fn conversation_answer_input(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        request_id: ProviderRequestId,
        answers: std::collections::BTreeMap<String, Vec<String>>,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can answer provider input requests");
        }
        self.may(client, Grant::Respond, None)?;
        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if conversation.generation != generation || conversation.state != RecoveryState::Running {
            bail!("Input request belongs to a stale or stopped conversation");
        }
        let active_turn = self
            .conversation_operations
            .iter()
            .find(|operation| operation.receipt.conversation == id)
            .and_then(|operation| match &operation.receipt.delivery {
                crate::conversation::Delivery::Acknowledged { turn_id } => Some(turn_id.as_str()),
                crate::conversation::Delivery::SubmissionUncertain => None,
            });
        let state = self
            .conversation_states
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation has no active input request"))?;
        let request = state
            .approvals
            .get(&request_id)
            .ok_or_else(|| anyhow::anyhow!("Input request is stale or already answered"))?;
        if request.kind != crate::conversation::ApprovalRequestKind::UserInput
            || request.turn_id.as_deref() != active_turn
        {
            bail!("Input request does not belong to the acknowledged active turn");
        }
        anyhow::ensure!(
            answers.len() == request.questions.len()
                && request
                    .questions
                    .iter()
                    .all(|question| answers.contains_key(&question.id)),
            "Answer every requested question exactly once"
        );
        for question in &request.questions {
            let values = &answers[&question.id];
            anyhow::ensure!(!values.is_empty(), "Each question needs an answer");
            let mut seen = std::collections::BTreeSet::new();
            for value in values {
                anyhow::ensure!(
                    !value.is_empty() && value.chars().count() <= MAX_USER_INPUT_ANSWER_CHARS,
                    "Input answers must be nonempty and no longer than 10,000 characters"
                );
                crate::hidden::check(value).map_err(anyhow::Error::msg)?;
                anyhow::ensure!(seen.insert(value), "An answer was selected more than once");
                if !question.options.is_empty()
                    && !question.options.iter().any(|option| option.label == *value)
                    && !(question.is_other
                        && value
                            .strip_prefix("user_note: ")
                            .is_some_and(|note| !note.trim().is_empty()))
                {
                    bail!(
                        "Answer for '{}' is not one of the available choices",
                        question.header
                    );
                }
            }
        }
        let answers = answers
            .into_iter()
            .map(|(question, answers)| (question, json!({ "answers": answers })))
            .collect::<serde_json::Map<_, _>>();
        self.conversation_runtimes.request(
            id,
            generation,
            0,
            Rpc::Answer {
                request_id: request_id.clone().into(),
                response: json!({ "answers": answers }),
            },
        )?;
        if let Some(state) = self.conversation_states.get_mut(&id) {
            state.approvals.remove(&request_id);
            state.blocked = !state.approvals.is_empty() || !state.unsupported_requests.is_empty();
            state.status = if state.blocked {
                Status::Blocked
            } else {
                Status::Working
            };
            state.error = state.unsupported_requests.values().next().cloned();
        }
        self.send(client, ServerMsg::Reply(Reply::Ok));
        self.dirty = true;
        Ok(())
    }

    pub(in crate::daemon) fn conversation_cancel(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can cancel a structured turn");
        }
        self.may(client, Grant::Write, None)?;
        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if conversation.generation != generation || conversation.state != RecoveryState::Running {
            bail!("Turn belongs to a stale or stopped conversation");
        }
        let thread_id = conversation.binding.session_id.clone();
        let state = self
            .conversation_states
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation runtime is not ready"))?;
        if !matches!(state.status, Status::Working | Status::Blocked) {
            bail!("Conversation has no active turn to cancel");
        }
        let operation = self
            .conversation_operations
            .iter()
            .find(|operation| operation.receipt.conversation == id)
            .ok_or_else(|| anyhow::anyhow!("Conversation has no acknowledged turn"))?;
        if operation.receipt.generation != generation {
            bail!("Active turn belongs to a stale conversation generation");
        }
        let Delivery::Acknowledged { turn_id } = &operation.receipt.delivery else {
            bail!(
                "Turn delivery is uncertain; {} cannot safely identify it to cancel",
                crate::brand::APP_NAME
            );
        };
        let turn_id = turn_id.clone();
        let cancellation_pending = state.cancelling.as_deref() == Some(turn_id.as_str())
            || self.conversation_requests.values().any(|pending| {
                matches!(pending, Pending::Cancelling { id: pending_id, generation: pending_generation, turn_id: pending_turn, .. }
                    if *pending_id == id && *pending_generation == generation && pending_turn == &turn_id)
            });
        if cancellation_pending {
            self.send(client, ServerMsg::Reply(Reply::Ok));
            return Ok(());
        }
        let token = self.conversation_token(Pending::Cancelling {
            client,
            id,
            generation,
            turn_id: turn_id.clone(),
        })?;
        let request = self.conversation_runtimes.request(
            id,
            generation,
            token,
            Rpc::Interrupt {
                params: json!({"threadId":thread_id,"turnId":turn_id}),
            },
        );
        if let Err(error) = request {
            self.conversation_requests.remove(&token);
            return Err(error);
        }
        self.conversation_states
            .get_mut(&id)
            .expect("validated conversation state")
            .cancelling = Some(turn_id);
        self.send(client, ServerMsg::Reply(Reply::Ok));
        self.dirty = true;
        Ok(())
    }

    /// Stop a chat conversation's agent on purpose. The record stays, stopped and
    /// idle, so its history is kept; the client then hides it.
    pub(in crate::daemon) fn conversation_end(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can end a conversation");
        }
        self.may(client, Grant::Write, None)?;
        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if conversation.generation != generation {
            bail!("Conversation generation is stale");
        }
        if conversation.execution != ExecutionKind::Structured {
            bail!("A native terminal conversation ends with its terminal");
        }
        if self
            .conversation_requests
            .values()
            .any(|pending| pending.conversation() == id)
        {
            bail!("Conversation has a pending operation; try again in a moment");
        }
        self.stop_structured(id)?;
        self.send(client, ServerMsg::Reply(Reply::Ok));
        Ok(())
    }

    /// Stops a chat's agent for good and drops what was waiting for it. Its record stays, stopped.
    fn stop_structured(&mut self, id: ConversationId) -> Result<()> {
        if self.conversation_runtimes.owns(id) {
            self.conversation_ending.insert(id);
            if let Err(error) = self.conversation_runtimes.dispose(id) {
                self.conversation_ending.remove(&id);
                return Err(error);
            }
        }
        // Ending the agent is the person's answer to what was waiting for it.
        self.conversation_queues.remove(&id);
        self.dirty = true;
        Ok(())
    }

    /// Closes a chat with an undo window: it is archived (hidden) at once and its agent keeps running
    /// until `UNDO_WINDOW` passes, when `finish_chat_closes` ends it. A second close keeps the first clock.
    pub(in crate::daemon) fn conversation_close(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can close a conversation");
        }
        self.may(client, Grant::Write, None)?;
        let conversation = self
            .conversations
            .iter_mut()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if conversation.generation != generation {
            bail!("Conversation generation is stale");
        }
        if conversation.execution != ExecutionKind::Structured {
            bail!("A native terminal conversation closes with its terminal");
        }
        conversation.archived = true;
        self.conversation_closing
            .entry(id)
            .or_insert_with(std::time::Instant::now);
        self.send(client, ServerMsg::Reply(Reply::Ok));
        self.dirty = true;
        Ok(())
    }

    /// Brings back a chat closed with `conversation_close` while its window is open.
    pub(in crate::daemon) fn conversation_undo_close(
        &mut self,
        client: ClientId,
        id: ConversationId,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can bring back a conversation");
        }
        self.may(client, Grant::Write, None)?;
        if self.conversation_closing.remove(&id).is_none() {
            bail!("The undo window has passed");
        }
        if let Some(conversation) = self.conversations.iter_mut().find(|c| c.id == id) {
            conversation.archived = false;
        }
        self.send(client, ServerMsg::Reply(Reply::Ok));
        self.dirty = true;
        Ok(())
    }

    /// Ends the chats whose undo window has passed. One with an operation still pending is tried
    /// again on the next tick.
    pub(in crate::daemon) fn finish_chat_closes(&mut self) {
        let due: Vec<ConversationId> = self
            .conversation_closing
            .iter()
            .filter(|(_, at)| at.elapsed() >= crate::daemon::UNDO_WINDOW)
            .map(|(id, _)| *id)
            .collect();
        for id in due {
            if self
                .conversation_requests
                .values()
                .any(|pending| pending.conversation() == id)
            {
                continue;
            }
            self.conversation_closing.remove(&id);
            self.remember_closed_chat(id);
            let tops = self.worktrees_of_chat(id);
            match self.stop_structured(id) {
                // No client is waiting on a close whose window passed, so an unmerged
                // worktree stays and only the daemon log says why.
                Ok(()) => self.cleanup_worktrees(NO_CLIENT, tops),
                Err(error) => tracing::warn!("ending a closed chat: {error:#}"),
            }
        }
    }

    pub(in crate::daemon) fn conversation_send(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        operation: u64,
        text: String,
        choice: TurnChoice,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        self.may(client, Grant::Write, None)?;
        self.durable_conversation()?;
        choice.validate().map_err(anyhow::Error::msg)?;
        crate::hidden::check(&text).map_err(anyhow::Error::msg)?;
        if text.trim().is_empty() || text.len() > MAX_PROMPT_BYTES || operation == 0 {
            bail!("A bounded nonempty prompt and nonzero monotonic operation ID are required");
        }
        let c = self
            .conversations
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if c.generation != generation {
            bail!("Stale conversation generation");
        }
        let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
        if let Some(old) = self
            .conversation_operations
            .iter()
            .find(|o| o.receipt.conversation == id)
        {
            if old.receipt.operation == operation
                && old.receipt.generation == generation
                && old.digest == digest
            {
                if matches!(old.receipt.delivery, Delivery::Acknowledged { .. })
                    && !self
                        .conversation_requests
                        .values()
                        .any(|p| matches!(p,Pending::Receipt {receipt,..} if receipt==&old.receipt))
                {
                    self.send(
                        client,
                        ServerMsg::Reply(Reply::ConversationSent(old.receipt.clone())),
                    );
                    return Ok(());
                }
                bail!("Submission delivery is uncertain; automatic resend is refused");
            }
            if operation <= old.receipt.operation {
                bail!("Operation ID is stale or reused with changed content");
            }
            if old.receipt.delivery == Delivery::SubmissionUncertain {
                bail!("Previous submission delivery remains uncertain");
            }
        }
        // A message already waiting is the same send again (a retry), or a reused number.
        if let Some(waiting) = self
            .conversation_queues
            .get(&id)
            .and_then(|queue| queue.iter().find(|m| m.operation == operation))
        {
            if waiting.generation == generation && waiting.digest == digest {
                self.send(
                    client,
                    ServerMsg::Reply(Reply::ConversationQueued(QueuedReceipt {
                        conversation: id,
                        generation,
                        operation,
                    })),
                );
                return Ok(());
            }
            bail!("Operation ID is stale or reused with changed content");
        }
        if self
            .conversation_queues
            .get(&id)
            .and_then(|queue| queue.last())
            .is_some_and(|last| operation <= last.operation)
        {
            bail!("Operation ID is stale or reused with changed content");
        }
        let state = self
            .conversation_states
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation runtime is not ready"))?;
        if c.state != RecoveryState::Running || state.blocked {
            bail!("Conversation is busy, blocked or not ready");
        }
        if !state.history_ready {
            bail!("Native history is incomplete; read it before submitting");
        }
        // The agent is mid-turn (or a send is in flight, or earlier messages are waiting): the
        // daemon holds the message, so it survives the window and goes out in order.
        let busy = state.status == Status::Working
            || self
                .conversation_requests
                .values()
                .any(|p| p.conversation() == id)
            || self
                .conversation_queues
                .get(&id)
                .is_some_and(|queue| !queue.is_empty());
        let message = Outgoing {
            text,
            digest,
            choice,
        };
        if busy {
            return self.conversation_enqueue(client, id, generation, operation, message);
        }
        self.conversation_submit(client, id, generation, operation, message)
    }

    /// Hands a message to the runtime now: records its receipt, then submits once that is durable.
    fn conversation_submit(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        operation: u64,
        message: Outgoing,
    ) -> Result<()> {
        let Outgoing {
            text,
            digest,
            choice,
        } = message;
        let receipt = Receipt {
            conversation: id,
            generation,
            operation,
            delivery: Delivery::SubmissionUncertain,
        };
        self.conversation_operations
            .retain(|o| o.receipt.conversation != id);
        self.conversation_operations.push(Operation {
            receipt: receipt.clone(),
            digest,
        });
        let token = self.conversation_token(Pending::Submit {
            client,
            receipt,
            text,
            choice,
        })?;
        self.conversation_persist(token)?;
        Ok(())
    }

    /// Keep the checkout's files as they are `when` ("before", "after") a turn. Before: once
    /// a message was handed to the provider, which happens once a turn. After: once the
    /// provider said the turn completed, failed or was interrupted.
    pub(super) fn snapshot_conversation_turn(&self, id: ConversationId, when: &str) {
        if !self.cfg.snapshots.enabled {
            return;
        }
        let Some(c) = self.conversations.iter().find(|c| c.id == id) else {
            return;
        };
        // Named as a terminal agent's snapshots are, so the list reads the same for both.
        let agent = c.binding.provider.agent_name();
        let dir = c.binding.worktree.as_ref().unwrap_or(&c.binding.cwd);
        self.snapshot_off_loop(format!("{when} {agent}'s turn"), dir.clone());
    }

    fn conversation_enqueue(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        operation: u64,
        message: Outgoing,
    ) -> Result<()> {
        let Outgoing {
            text,
            digest,
            choice,
        } = message;
        let queue = self.conversation_queues.entry(id).or_default();
        if queue.len() >= MAX_QUEUED_MESSAGES
            || queue.iter().map(|m| m.text.len()).sum::<usize>() + text.len() > MAX_QUEUED_BYTES
        {
            bail!("Too many messages are waiting; wait for the agent to catch up");
        }
        queue.push(QueuedSend {
            generation,
            operation,
            text,
            digest,
            choice,
        });
        self.send(
            client,
            ServerMsg::Reply(Reply::ConversationQueued(QueuedReceipt {
                conversation: id,
                generation,
                operation,
            })),
        );
        self.dirty = true;
        Ok(())
    }

    /// Takes a waiting message back out of the queue.
    pub(in crate::daemon) fn conversation_dequeue(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        operation: u64,
    ) -> Result<()> {
        self.refuse_if_followed(id)?;
        self.may(client, Grant::Write, None)?;
        let found = self.conversation_queues.get_mut(&id).and_then(|queue| {
            let position = queue
                .iter()
                .position(|m| m.operation == operation && m.generation == generation)?;
            queue.remove(position);
            Some(())
        });
        if found.is_none() {
            bail!("That message is no longer waiting; it may already have been sent");
        }
        self.send(client, ServerMsg::Reply(Reply::Ok));
        self.dirty = true;
        Ok(())
    }

    /// Delivers the oldest waiting message of every conversation whose turn has ended: once, in
    /// order, and never into a turn that is running, an unanswered request or a send in flight.
    /// Called on the daemon's tick, so the window need not be open.
    pub(in crate::daemon) fn deliver_queued(&mut self) {
        let waiting: Vec<ConversationId> = self
            .conversation_queues
            .iter()
            .filter(|(_, queue)| !queue.is_empty())
            .map(|(id, _)| *id)
            .collect();
        for id in waiting {
            let current = self
                .conversations
                .iter()
                .find(|c| c.id == id)
                .map(|c| (c.generation, c.state));
            let Some((generation, record_state)) = current else {
                self.conversation_queues.remove(&id);
                continue;
            };
            // A message belongs to the runtime it was sent to; after a restart of the
            // conversation it is not delivered into the new one.
            if let Some(queue) = self.conversation_queues.get_mut(&id) {
                let before = queue.len();
                queue.retain(|m| m.generation == generation);
                if queue.len() != before {
                    tracing::info!("dropped queued messages of a conversation that started over");
                    self.dirty = true;
                }
            }
            let Some(state) = self.conversation_states.get(&id) else {
                continue;
            };
            let ready = record_state == RecoveryState::Running
                && !state.blocked
                && state.history_ready
                && state.status != Status::Working
                && state.approvals.is_empty()
                && state.unsupported_requests.is_empty()
                && state.cancelling.is_none()
                && !self
                    .conversation_requests
                    .values()
                    .any(|p| p.conversation() == id)
                && !self.conversation_operations.iter().any(|o| {
                    o.receipt.conversation == id
                        && o.receipt.delivery == Delivery::SubmissionUncertain
                });
            if !ready {
                continue;
            }
            let Some(queue) = self.conversation_queues.get_mut(&id) else {
                continue;
            };
            if queue.is_empty() {
                continue;
            }
            // Out of the queue first: a message is handed over once, never twice.
            let next = queue.remove(0);
            self.dirty = true;
            if let Err(error) = self.conversation_submit(
                NO_CLIENT,
                id,
                next.generation,
                next.operation,
                Outgoing {
                    text: next.text,
                    digest: next.digest,
                    choice: next.choice,
                },
            ) {
                let state = self.conversation_states.entry(id).or_default();
                state.error = Some(format!("A queued message could not be sent: {error}"));
            }
        }
    }
    /// Name a conversation (empty: back to automatic), for every client at once.
    pub(in crate::daemon) fn conversation_rename(
        &mut self,
        id: ConversationId,
        title: &str,
    ) -> Result<()> {
        let conversation = self
            .conversations
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        conversation.title = clean_title(title);
        self.dirty = true;
        Ok(())
    }

    /// Put a conversation away, or bring it back. It keeps running either way.
    pub(in crate::daemon) fn conversation_archive(
        &mut self,
        id: ConversationId,
        archived: bool,
    ) -> Result<()> {
        let conversation = self
            .conversations
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        conversation.archived = archived;
        self.dirty = true;
        Ok(())
    }

    pub(in crate::daemon) fn conversation_summaries(&self) -> Vec<ConversationSummary> {
        let mut summaries: Vec<_> = self
            .conversations
            .iter()
            .map(|c| {
                let state = self.conversation_states.get(&c.id);
                let native_term = self
                    .terms
                    .values()
                    .find(|term| term.conversation_id == Some(c.id));
                // Read for a chat session only: a native record has its pane, which carries this.
                let checkout = (c.execution == ExecutionKind::Structured)
                    .then(|| {
                        crate::gitfs::head(
                            c.binding.worktree.as_deref().unwrap_or(&c.binding.cwd),
                        )
                    })
                    .flatten();
                ConversationSummary {
                    id: c.id,
                    generation: c.generation,
                    provider: c.binding.provider,
                    execution: c.execution,
                    cwd: c.binding.cwd.clone(),
                    worktree: c.binding.worktree.clone(),
                    state: c.state,
                    status: if c.execution == ExecutionKind::Native {
                        native_term.map_or_else(
                            || {
                                if c.state == RecoveryState::RecoveryRequired {
                                    Status::Blocked
                                } else {
                                    Status::Idle
                                }
                            },
                            |term| term.status,
                        )
                    } else {
                        state.map_or(
                            if c.state == RecoveryState::RecoveryRequired {
                                Status::Blocked
                            } else {
                                Status::Idle
                            },
                            |s| s.effective_status(),
                        )
                    },
                    native_term: native_term
                        .filter(|term| !term.process_exited)
                        .map(|term| term.id),
                    checks: state.map_or_else(Vec::new, |s| s.checks.infos()),
                    title: c.title.clone(),
                    archived: c.archived,
                    root: checkout.as_ref().map(|head| head.main_root.clone()),
                    branch: checkout.map(|head| head.branch),
                }
            })
            .collect();
        summaries.extend(self.followed.iter().filter_map(|f| {
            let term = self.terms.get(&f.term)?;
            Some(ConversationSummary {
                id: f.id,
                generation: f.generation,
                provider: f.provider,
                execution: ExecutionKind::Native,
                cwd: term.cwd.clone(),
                worktree: None,
                state: RecoveryState::Running,
                // The pane's own status; reading its transcript never changes it.
                status: term.status,
                native_term: Some(f.term),
                checks: Vec::new(),
                title: None,
                archived: false,
                root: None,
                branch: None,
            })
        }));
        summaries.extend(self.pending_starts.iter().map(|p| {
            ConversationSummary {
                id: p.id,
                generation: p.generation,
                provider: p.settings.provider,
                execution: ExecutionKind::Structured,
                cwd: p.settings.cwd.clone(),
                worktree: None,
                state: RecoveryState::RecoveryRequired,
                status: self
                    .conversation_states
                    .get(&p.id)
                    .map_or(Status::Blocked, |s| s.status),
                native_term: None,
                checks: Vec::new(),
                title: None,
                archived: false,
                root: None,
                branch: None,
            }
        }));
        summaries
    }

    pub(in crate::daemon) fn conversation_barrier(
        &mut self,
        token: u64,
        result: std::result::Result<persist::SessionCommit, String>,
    ) {
        self.conversation_barriers.remove(&token);
        let Some(pending) = self.conversation_requests.remove(&token) else {
            return;
        };
        let id = pending.conversation();
        let result = result.and_then(|commit| match commit {
            persist::SessionCommit::Committed => Ok(()),
            persist::SessionCommit::Superseded { .. } => Err("Session persistence was superseded; mutation refused before delivery acknowledgement".into()),
        });
        if let Err(error) = result {
            self.conversation_fail(pending, error);
            return;
        }
        if !self.cfg.restore.enabled {
            self.conversation_fail(
                pending,
                "Restore was disabled while the operation was queued".into(),
            );
            return;
        }
        let applied = match pending {
            Pending::Launch { client, launch } => {
                if let Some(by) = self
                    .conversation_states
                    .get(&id)
                    .and_then(|state| state.started_by)
                {
                    let allowed = self
                        .may_start_helper(by, "a structured Codex conversation")
                        .and_then(|()| {
                            let busy = self.busy_helpers(by, Some(id));
                            if busy >= self.cfg.helpers_at_once() {
                                bail!("Structured helper limit reached before launch");
                            }
                            Ok(())
                        });
                    if let Err(error) = allowed {
                        self.conversation_fail(
                            Pending::Launch { client, launch },
                            error.to_string(),
                        );
                        return;
                    }
                }
                let matches = match &launch.identity {
                    LaunchIdentity::Pending(p) => {
                        self.pending_starts.iter().any(|record| record == p)
                    }
                    LaunchIdentity::Existing(c) => {
                        self.conversations.iter().any(|record| record == &**c)
                    }
                };
                if !matches
                    || self.conversation_runtimes.owns(id)
                    || self.terms.values().any(|t| t.conversation_id == Some(id))
                {
                    self.conversation_fail(
                        Pending::Launch { client, launch },
                        "Conversation ownership changed before provider launch".into(),
                    );
                    return;
                }
                let generation = launch.generation();
                let settings = launch.settings().cloned();
                let expected = match &launch.identity {
                    LaunchIdentity::Pending(p) => p.thread_id.clone(),
                    LaunchIdentity::Existing(c) => Some(c.binding.session_id.clone()),
                };
                match settings {
                    Some(settings) => {
                        let opening = Pending::Opening {
                            client,
                            id,
                            generation,
                            settings,
                            expected,
                        };
                        self.conversation_requests.insert(token, opening);
                        self.conversation_runtimes
                            .start(*launch, self.tx.clone())
                            .map_err(|e| (token, e.to_string()))
                    }
                    None => Err((token, "Structured launch settings unavailable".into())),
                }
            }
            Pending::Identity {
                client,
                id,
                generation,
                created_thread,
            } => {
                if !self
                    .conversations
                    .iter()
                    .any(|c| c.id == id && c.generation == generation)
                {
                    self.send(
                        client,
                        ServerMsg::Error("Conversation identity changed during persistence".into()),
                    );
                    return;
                }
                if created_thread {
                    match self.complete_empty_thread(id) {
                        Ok(()) => {
                            self.send(
                                client,
                                ServerMsg::Reply(Reply::ConversationStarted { id, generation }),
                            );
                            Ok(())
                        }
                        Err(error) => Err((token, error.to_string())),
                    }
                } else {
                    self.begin_hydration(id, generation, None, Some(client))
                        .map_err(|error| (token, error.to_string()))
                }
            }
            Pending::Submit {
                client,
                receipt,
                text,
                choice,
            } => {
                let valid = self.conversations.iter().any(|c| {
                    c.id == id
                        && c.generation == receipt.generation
                        && c.state == RecoveryState::Running
                }) && self
                    .conversation_operations
                    .iter()
                    .any(|o| o.receipt == receipt)
                    && self.conversation_states.get(&id).is_some_and(|s| {
                        !s.blocked && s.status != Status::Working && s.history_ready
                    });
                if !valid {
                    self.send(
                        client,
                        ServerMsg::Error(
                            "Conversation became stale, busy or blocked before submission".into(),
                        ),
                    );
                    return;
                }
                let thread = self
                    .conversations
                    .iter()
                    .find(|c| c.id == id)
                    .map(|c| c.binding.session_id.clone())
                    .unwrap_or_default();
                let generation = receipt.generation;
                let operation = receipt.operation;
                self.conversation_requests
                    .insert(token, Pending::Sending { client, receipt });
                let state = self.conversation_states.entry(id).or_default();
                state.status = Status::Working;
                state.error = None;
                let sent = self.conversation_runtimes.request(id, generation, token, Rpc::Send { params: send_params(&thread, &text, &format!("{}-{generation}-{operation}", id.0), &choice) }).map_err(|e| (token, e.to_string()));
                if sent.is_ok() {
                    self.snapshot_conversation_turn(id, "before");
                }
                sent
            }
            Pending::Receipt { client, receipt } => {
                if self
                    .conversation_operations
                    .iter()
                    .any(|o| o.receipt == receipt)
                    && self
                        .conversations
                        .iter()
                        .any(|c| c.id == id && c.generation == receipt.generation)
                {
                    self.send(client, ServerMsg::Reply(Reply::ConversationSent(receipt)));
                } else {
                    self.send(
                        client,
                        ServerMsg::Error("Submission acknowledgement became stale".into()),
                    );
                }
                Ok(())
            }
            other => {
                self.conversation_fail(other, "Unexpected persistence stage".into());
                return;
            }
        };
        if let Err((token, error)) = applied
            && let Some(pending) = self.conversation_requests.remove(&token)
        {
            self.conversation_fail(pending, error);
        }
        self.dirty = true;
    }

    pub(in crate::daemon) fn conversation_save_completed(
        &mut self,
        barrier: Option<u64>,
        result: std::result::Result<persist::SessionCommit, String>,
    ) {
        if let Err(error) = result {
            if let Some(token) = barrier {
                self.conversation_barrier(token, Err(error));
            }
            return;
        }
        let snapshot = match serde_json::from_str::<persist::Saved>(&self.last_saved) {
            Ok(snapshot) => snapshot,
            Err(_) => return,
        };
        let ready: Vec<_> = self
            .conversation_barriers
            .iter()
            .filter(|(_, ticket)| **ticket <= self.session_saved_ticket)
            .map(|(token, _)| *token)
            .collect();
        for token in ready {
            let matches =
                self.conversation_requests
                    .get(&token)
                    .is_some_and(|pending| match pending {
                        Pending::Launch { launch, .. } => match &launch.identity {
                            LaunchIdentity::Pending(p) => {
                                snapshot.pending_starts.iter().any(|saved| saved == p)
                            }
                            LaunchIdentity::Existing(c) => {
                                snapshot.conversations.iter().any(|saved| saved == &**c)
                            }
                        },
                        Pending::Identity { id, generation, .. } => {
                            snapshot.conversations.iter().any(|saved| {
                                saved.id == *id
                                    && saved.generation == *generation
                                    && self.conversations.iter().any(|current| current == saved)
                            })
                        }
                        Pending::Submit { receipt, .. } | Pending::Receipt { receipt, .. } => {
                            snapshot.conversation_operations.iter().any(|saved| {
                                saved.receipt == *receipt
                                    && self
                                        .conversation_operations
                                        .iter()
                                        .any(|current| current == saved)
                            })
                        }
                        _ => false,
                    });
            self.conversation_barrier(
                token,
                if matches {
                    Ok(persist::SessionCommit::Committed)
                } else {
                    Err(
                        "Newer durable snapshot does not contain the matching operation intent"
                            .into(),
                    )
                },
            );
        }
    }

    pub(in crate::daemon) fn conversation_ready(&mut self, id: ConversationId) -> Result<()> {
        let token = self.conversation_requests.iter().find_map(|(token, p)| {
            matches!(p, Pending::Opening { id: candidate, .. } if *candidate == id)
                .then_some(*token)
        });
        let Some(token) = token else { return Ok(()) };
        let Some(Pending::Opening {
            generation,
            settings,
            expected,
            ..
        }) = self.conversation_requests.get(&token)
        else {
            return Ok(());
        };
        let mut params = params(settings);
        let rpc = if let Some(thread) = expected {
            params["threadId"] = json!(thread);
            params["excludeTurns"] = json!(true);
            Rpc::Resume { params }
        } else {
            Rpc::Start { params }
        };
        self.conversation_runtimes
            .request(id, *generation, token, rpc)
    }

    pub(in crate::daemon) fn conversation_response(
        &mut self,
        token: u64,
        result: std::result::Result<Value, super::transport::RpcError>,
    ) -> Result<()> {
        let Some(pending) = self.conversation_requests.remove(&token) else {
            bail!("Provider response has no daemon operation");
        };
        let result = match result {
            Err(error)
                if error.is_thread_not_materialized()
                    && matches!(&pending, Pending::Hydration { .. })
                    && self.conversations.iter().any(|conversation| {
                        conversation.id == pending.conversation()
                            && conversation.awaiting_first_user_message
                    })
                    && !self.conversation_operations.iter().any(|operation| {
                        operation.receipt.conversation == pending.conversation()
                    }) =>
            {
                Ok(json!({"data":[],"nextCursor":null}))
            }
            result => result,
        };
        let value = match result {
            Ok(value) => value,
            Err(error) => {
                let stage = match &pending {
                    Pending::Opening { expected: None, .. } => "thread/start",
                    Pending::Opening { .. } => "thread/resume",
                    Pending::Hydration { .. } => "thread/turns/list",
                    Pending::Sending { .. } => "turn/start",
                    Pending::Cancelling { .. } => "turn/interrupt",
                    _ => "unknown",
                };
                self.conversation_fail(
                    pending,
                    format!("Codex rejected {stage} (RPC error {}); no fallback or automatic resend was attempted", error.code),
                );
                return Ok(());
            }
        };
        let id = pending.conversation();
        match pending {
            Pending::Opening {
                client,
                id,
                generation,
                mut settings,
                expected,
            } => {
                let created_thread = expected.is_none();
                let thread = match validate_response(&value, &mut settings, expected.as_deref()) {
                    Ok(thread) => thread,
                    Err(error) => {
                        self.conversation_fail(
                            Pending::Opening {
                                client,
                                id,
                                generation,
                                settings,
                                expected,
                            },
                            error.to_string(),
                        );
                        return Ok(());
                    }
                };
                let origin = worktree_origin_of(&settings.cwd);
                let c = Conversation {
                    id,
                    generation,
                    binding: NativeBinding {
                        provider: settings.provider,
                        session_id: thread,
                        account: None,
                        profile: None,
                        config_root: settings.config_root.clone(),
                        executable: settings.executable.clone(),
                        cwd: settings.cwd.clone(),
                        worktree: None,
                    },
                    execution: ExecutionKind::Structured,
                    state: RecoveryState::Running,
                    settings: Some(settings),
                    awaiting_first_user_message: created_thread
                        || self
                            .conversations
                            .iter()
                            .find(|conversation| conversation.id == id)
                            .is_some_and(|conversation| {
                                conversation.awaiting_first_user_message
                            }),
                    title: self
                        .conversations
                        .iter()
                        .find(|conversation| conversation.id == id)
                        .and_then(|conversation| conversation.title.clone()),
                    archived: self
                        .conversations
                        .iter()
                        .find(|conversation| conversation.id == id)
                        .is_some_and(|conversation| conversation.archived),
                    // What the folder says now wins (the branch may have been switched in it);
                    // the saved one stays when the folder says nothing (detached, or gone).
                    worktree_origin: origin.or_else(|| {
                        self.conversations
                            .iter()
                            .find(|conversation| conversation.id == id)
                            .and_then(|conversation| conversation.worktree_origin.clone())
                    }),
                };
                let mut proposed = self.conversations.clone();
                proposed.retain(|c| c.id != id);
                proposed.push(c);
                validate_registry(&proposed).map_err(|e| {
                    anyhow::anyhow!("Provider identity conflicts with registry: {e:?}")
                })?;
                self.conversations = proposed;
                self.pending_starts.retain(|p| p.id != id);
                self.conversation_requests.insert(
                    token,
                    Pending::Identity {
                        client,
                        id,
                        generation,
                        created_thread,
                    },
                );
                self.conversation_persist(token)?;
            }
            Pending::Sending {
                client,
                mut receipt,
            } => {
                let turn = value
                    .get("turn")
                    .ok_or_else(|| anyhow::anyhow!("Codex turn response lacks turn"))?;
                let turn_id = identifier(turn, "id")?;
                if !turn.get("items").is_some_and(Value::is_array)
                    || !turn.get("status").is_some_and(Value::is_string)
                {
                    bail!("Codex turn response is incomplete; delivery uncertain");
                }
                if let Some(conversation) = self
                    .conversations
                    .iter_mut()
                    .find(|conversation| conversation.id == receipt.conversation)
                {
                    conversation.awaiting_first_user_message = false;
                }
                receipt.delivery = Delivery::Acknowledged { turn_id };
                let operation = self
                    .conversation_operations
                    .iter_mut()
                    .find(|o| {
                        o.receipt.conversation == id
                            && o.receipt.generation == receipt.generation
                            && o.receipt.operation == receipt.operation
                    })
                    .ok_or_else(|| {
                        anyhow::anyhow!("Submission generation changed before acknowledgement")
                    })?;
                operation.receipt = receipt.clone();
                self.conversation_requests
                    .insert(token, Pending::Receipt { client, receipt });
                self.conversation_persist(token)?;
            }
            Pending::Cancelling {
                id,
                generation,
                turn_id,
                ..
            } => {
                if !value.is_object()
                    || !self.conversations.iter().any(|conversation| {
                        conversation.id == id
                            && conversation.generation == generation
                            && conversation.state == RecoveryState::Running
                    })
                    || !self.conversation_operations.iter().any(|operation| {
                        operation.receipt.conversation == id
                            && operation.receipt.generation == generation
                            && matches!(&operation.receipt.delivery, Delivery::Acknowledged { turn_id: acknowledged } if acknowledged == &turn_id)
                    })
                {
                    bail!("Codex interrupt acknowledgement is stale or invalid");
                }
            }
            Pending::Hydration {
                id,
                generation,
                mut history,
                mut cursors,
                read,
                started,
                changed,
                bytes,
            } => {
                let bytes = bytes
                    .checked_add(serde_json::to_vec(&value)?.len())
                    .filter(|bytes| *bytes <= MAX_HYDRATION_BYTES)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Codex history exceeds bounded hydration size")
                    })?;
                let thread = self
                    .conversations
                    .iter()
                    .find(|c| c.id == id && c.generation == generation)
                    .ok_or_else(|| anyhow::anyhow!("History generation changed"))?
                    .binding
                    .session_id
                    .clone();
                let turns = value
                    .get("data")
                    .and_then(Value::as_array)
                    .ok_or_else(|| anyhow::anyhow!("Codex history page lacks turns"))?;
                if !turns.is_empty()
                    && let Some(conversation) = self
                        .conversations
                        .iter_mut()
                        .find(|conversation| conversation.id == id)
                {
                    conversation.awaiting_first_user_message = false;
                }
                for turn in turns {
                    if turn.get("itemsView").is_some_and(|v| v != "full") {
                        bail!("Codex history returned a partial item view");
                    }
                    let turn_id = identifier(turn, "id")?;
                    if let Some(update) = super::history_provider::notification(
                        id,
                        &thread,
                        "turn/completed",
                        &json!({"threadId":thread,"turn":turn}),
                    )? {
                        history.apply(update)?;
                    }
                    for item in turn
                        .get("items")
                        .and_then(Value::as_array)
                        .ok_or_else(|| anyhow::anyhow!("Codex history turn lacks items"))?
                    {
                        if let Some(update) = super::history_provider::notification(
                            id,
                            &thread,
                            "item/completed",
                            &json!({"threadId":thread,"turnId":turn_id,"item":item}),
                        )? {
                            history.apply(update)?;
                        }
                    }
                    if history.entries.len() > MAX_HYDRATION_ENTRIES {
                        bail!("Codex history exceeds bounded hydration limit");
                    }
                }
                match value.get("nextCursor") {
                    Some(Value::String(cursor))
                        if !cursor.is_empty() && cursors.insert(cursor.clone()) =>
                    {
                        let params = json!({"threadId":thread,"cursor":cursor,"limit":history::MAX_READ_ENTRIES,"sortDirection":"asc","itemsView":"full"});
                        self.conversation_requests.insert(
                            token,
                            Pending::Hydration {
                                id,
                                generation,
                                history,
                                cursors,
                                read,
                                started,
                                changed,
                                bytes,
                            },
                        );
                        self.conversation_runtimes.request(
                            id,
                            generation,
                            token,
                            Rpc::Turns { params },
                        )?;
                    }
                    None | Some(Value::Null) => {
                        if matches!(
                            history.completeness,
                            Completeness::Unknown | Completeness::Loading
                        ) {
                            history.set_completeness(if changed {
                                Completeness::Incomplete(history::IncompleteReason::Disconnected)
                            } else {
                                Completeness::Complete
                            })?;
                        }
                        let history_ready = {
                            let state = self.conversation_states.entry(id).or_default();
                            state.history_revision = state
                                .history_revision
                                .max(history.revision)
                                .checked_add(1)
                                .ok_or_else(|| anyhow::anyhow!("History revision exhausted"))?;
                            history.revision = state.history_revision;
                            state.history_ready = history.completeness == Completeness::Complete;
                            if !state.blocked
                                && state.status == Status::Working
                                && !history.entries.iter().any(|e| {
                                    e.content == history::Content::TurnStatus
                                        && e.state == history::ItemState::Running
                                })
                            {
                                state.status = Status::Idle;
                            }
                            if started.is_some() && !state.history_ready {
                                state.status = Status::Blocked;
                                state.error = Some(
                                    "Codex history is incomplete; chat takeover is disabled".into(),
                                );
                            }
                            state.history_ready
                        };
                        if self.cfg.restore.output && self.cfg.restore.enabled {
                            self.conversation_histories.insert(id, history.clone());
                            self.cache_conversation(id);
                        }
                        if let Some(read) = read {
                            self.conversation_history
                                .hydrate_native(read.client, history.clone())?;
                            self.reply_history(id, read)?;
                        }
                        if let Some(client) = started {
                            if history_ready {
                                self.send(
                                    client,
                                    ServerMsg::Reply(Reply::ConversationStarted {
                                        id,
                                        generation,
                                    }),
                                );
                                self.continue_cut_turn(client, id, generation);
                            } else {
                                let message = "Codex history is incomplete; chat takeover is disabled";
                                self.send(client, ServerMsg::Error(message.into()));
                            }
                        }
                    }
                    _ => bail!("Codex history cursor is invalid or repeated"),
                }
            }
            other => {
                self.conversation_fail(other, "Unexpected provider response stage".into());
            }
        }
        self.dirty = true;
        Ok(())
    }

    pub(in crate::daemon) fn begin_hydration(
        &mut self,
        id: ConversationId,
        generation: u64,
        read: Option<ReadRequest>,
        started: Option<ClientId>,
    ) -> Result<()> {
        let c = self
            .conversations
            .iter()
            .find(|c| c.id == id && c.generation == generation)
            .ok_or_else(|| anyhow::anyhow!("No current conversation"))?;
        if c.binding.provider == Provider::Claude {
            return self.begin_claude_history_hydration(id, generation, read, started);
        }
        let params = json!({"threadId":c.binding.session_id,"limit":history::MAX_READ_ENTRIES,"sortDirection":"asc","itemsView":"full"});
        let mut history = History::new(id);
        history.revision = self
            .conversation_states
            .get(&id)
            .map_or(1, |s| s.history_revision.max(1));
        history.set_completeness(Completeness::Loading)?;
        let token = self.conversation_token(Pending::Hydration {
            id,
            generation,
            history,
            cursors: BTreeSet::new(),
            read,
            started,
            changed: false,
            bytes: 0,
        })?;
        self.conversation_runtimes
            .request(id, generation, token, Rpc::Turns { params })
    }

    fn begin_claude_history_hydration(
        &mut self,
        id: ConversationId,
        generation: u64,
        read: Option<ReadRequest>,
        started: Option<ClientId>,
    ) -> Result<()> {
        let Some(conversation) = self.conversations.iter().find(|conversation| {
            conversation.id == id
                && conversation.generation == generation
                && conversation.binding.provider == Provider::Claude
        }) else {
            bail!("Claude conversation identity changed during history hydration");
        };
        if conversation.awaiting_first_user_message {
            self.complete_empty_thread(id)?;
            if let Some(client) = started {
                self.send(
                    client,
                    ServerMsg::Reply(Reply::ConversationStarted { id, generation }),
                );
            }
            if let Some(read) = read {
                if self.conversation_history.get(read.client).is_none() {
                    self.conversation_history.attach(read.client, id);
                    let history = self
                        .conversation_histories
                        .get(&id)
                        .cloned()
                        .unwrap_or_else(|| History::new(id));
                    self.conversation_history
                        .hydrate_native(read.client, history)
                        .map_err(|reason| {
                            anyhow::anyhow!("Claude empty history hydration failed: {reason:?}")
                        })?;
                }
                self.reply_history(id, read)?;
            }
            return Ok(());
        }

        let is_currently_reconciled = self
            .conversation_states
            .get(&id)
            .is_some_and(|state| state.history_ready)
            && self.conversation_runtimes.owns(id);
        let (history, is_complete) = cached_claude_history(
            id,
            self.conversation_histories.get(&id),
            is_currently_reconciled,
        )?;
        let attachment = read.as_ref().map(|read| read.client).or(started);
        if let Some(client) = attachment {
            if self.conversation_history.get(client).is_none() {
                self.conversation_history.attach(client, id);
            }
            self.conversation_history
                .hydrate_native(client, history.clone())
                .map_err(|reason| {
                    anyhow::anyhow!("Claude history attachment failed: {reason:?}")
                })?;
        }
        let state = self.conversation_states.entry(id).or_default();
        state.history_ready = is_complete && history.completeness == Completeness::Complete;
        if !state.history_ready {
            state.status = Status::Blocked;
            state.error = Some(
                "Claude history could not be reconciled with the provider transcript; chat resume is disabled".into(),
            );
        }
        if let Some(read) = read {
            self.reply_history(id, read)?;
        }
        if let Some(client) = started {
            if self
                .conversation_states
                .get(&id)
                .is_some_and(|state| state.history_ready)
            {
                self.send(
                    client,
                    ServerMsg::Reply(Reply::ConversationStarted { id, generation }),
                );
                self.continue_cut_turn(client, id, generation);
            } else {
                self.send(
                    client,
                    ServerMsg::Error(
                        "Claude history is incomplete; chat resume is disabled".into(),
                    ),
                );
            }
        }
        Ok(())
    }

    fn complete_empty_thread(&mut self, id: ConversationId) -> Result<()> {
        let mut history = History::new(id);
        history.set_completeness(Completeness::Complete)?;
        let state = self.conversation_states.entry(id).or_default();
        state.history_revision = state
            .history_revision
            .max(history.revision)
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("History revision exhausted"))?;
        history.revision = state.history_revision;
        state.history_ready = true;
        state.blocked = false;
        state.error = None;
        if state.status == Status::Working {
            state.status = Status::Idle;
        }
        if self.cfg.restore.enabled && self.cfg.restore.output {
            self.conversation_histories.insert(id, history);
            self.cache_conversation(id);
        }
        self.dirty = true;
        Ok(())
    }

    pub(in crate::daemon) fn conversation_read(
        &mut self,
        client: ClientId,
        id: ConversationId,
        cursor: Option<history::Cursor>,
        limit: usize,
    ) -> Result<()> {
        self.may(client, Grant::Read, None)?;
        if limit == 0 || limit > history::MAX_READ_ENTRIES {
            bail!(
                "History limit must be between 1 and {}",
                history::MAX_READ_ENTRIES
            );
        }
        if self.is_followed(id) {
            return self.followed_read(client, id, cursor, limit);
        }
        let c = self
            .conversations
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| {
                anyhow::anyhow!("No complete conversation; creation may be uncertain")
            })?;
        let generation = c.generation;
        let known_empty = c.execution == ExecutionKind::Structured
            && c.awaiting_first_user_message
            && self
                .conversation_states
                .get(&id)
                .is_some_and(|state| state.history_ready)
            && !self
                .conversation_operations
                .iter()
                .any(|operation| operation.receipt.conversation == id);
        let read = ReadRequest {
            client,
            cursor,
            limit,
        };
        if self
            .conversation_history
            .get(client)
            .is_some_and(|h| h.conversation == id)
        {
            return self.reply_history(id, read);
        }
        self.conversation_history.attach(client, id);
        if known_empty {
            let mut history = History::new(id);
            history.set_completeness(Completeness::Complete)?;
            self.conversation_history.hydrate_native(client, history)?;
            return self.reply_history(id, read);
        }
        if let Some(history) = self.conversation_histories.get(&id).filter(|h| {
            h.completeness == Completeness::Complete || !self.conversation_runtimes.owns(id)
        }) {
            if self
                .conversation_states
                .get(&id)
                .is_some_and(|s| s.history_ready)
            {
                self.conversation_history
                    .hydrate_native(client, history.clone())?;
            } else {
                self.conversation_history
                    .hydrate_cached(client, history.clone())?;
            }
            return self.reply_history(id, read);
        }
        self.begin_hydration(id, generation, Some(read), None)
    }

    fn reply_history(&self, id: ConversationId, read: ReadRequest) -> Result<()> {
        let c = self
            .conversations
            .iter()
            .find(|c| c.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        let history = self
            .conversation_history
            .get(read.client)
            .ok_or_else(|| anyhow::anyhow!("History attachment closed"))?;
        let page = history
            .read_generation(
                c.generation,
                read.cursor.as_ref(),
                read.limit,
                history::MAX_READ_BYTES,
            )
            .map_err(|e| anyhow::anyhow!("History read refused: {e:?}"))?;
        let state = self.conversation_states.get(&id);
        self.send(
            read.client,
            ServerMsg::Reply(Reply::ConversationRead(ReadResult {
                conversation: id,
                generation: c.generation,
                status: state.map_or(Status::Blocked, |s| s.effective_status()),
                receipt: self.visible_receipt(id),
                history: page,
                error: state.and_then(|s| s.error.clone()),
                checks: state.map_or_else(Vec::new, |s| s.checks.infos()),
                // A structured conversation chose its own mode and model; only a followed pane reads them from a file.
                meta: None,
                approvals: state.map_or_else(Vec::new, |s| s.approvals.values().cloned().collect()),
                answered_approvals: state.map_or_else(Vec::new, |s| {
                    s.answered_approvals.iter().cloned().collect()
                }),
                queued: self.conversation_queues.get(&id).map_or_else(Vec::new, |queue| {
                    queue
                        .iter()
                        .map(|m| QueuedMessage {
                            operation: m.operation,
                            text: m.text.clone(),
                        })
                        .collect()
                }),
                cancelling: state.is_some_and(|s| s.cancelling.is_some())
                    || self.conversation_requests.values().any(
                        |pending| matches!(pending, Pending::Cancelling { id: pending_id, .. } if *pending_id == id),
                    ),
            })),
        );
        Ok(())
    }

    pub(in crate::daemon) fn conversation_fail(&mut self, pending: Pending, error: String) {
        let id = pending.conversation();
        if let Pending::Cancelling { turn_id, .. } = &pending
            && let Some(state) = self.conversation_states.get_mut(&id)
            && state.cancelling.as_deref() == Some(turn_id)
        {
            state.cancelling = None;
        }
        if let Pending::Receipt { receipt, .. } = &pending
            && let Some(operation) = self
                .conversation_operations
                .iter_mut()
                .find(|o| o.receipt == *receipt)
        {
            operation.receipt.delivery = Delivery::SubmissionUncertain;
        }
        let client = match pending {
            Pending::Validating { client, .. }
            | Pending::Launch { client, .. }
            | Pending::Opening { client, .. }
            | Pending::Identity { client, .. }
            | Pending::Submit { client, .. }
            | Pending::Sending { client, .. }
            | Pending::Cancelling { client, .. }
            | Pending::Receipt { client, .. } => Some(client),
            Pending::Hydration { read, started, .. } => read.map(|r| r.client).or(started),
        };
        let state = self.conversation_states.entry(id).or_default();
        state.status = Status::Blocked;
        state.error = Some(error.clone());
        state.history_ready = false;
        if client == Some(DAEMON_REQUESTER) {
            tracing::warn!("conversation {} was not resumed: {error}", id.0);
        }
        if let Some(client) = client {
            self.send(client, ServerMsg::Error(error));
        }
        self.dirty = true;
    }
}

/// Where the linked worktree holding `cwd` came from, if it is in one and on a branch (a
/// detached HEAD has no branch to make the folder again from). Reads the checkout's `.git`
/// files only; no git process runs.
fn worktree_origin_of(cwd: &std::path::Path) -> Option<crate::conversation::WorktreeOrigin> {
    let head = crate::gitfs::head(cwd).filter(|h| h.linked && !h.detached)?;
    Some(crate::conversation::WorktreeOrigin {
        main: head.main_root,
        folder: head.top,
        branch: head.branch,
    })
}

#[cfg(test)]
mod worktree_origin_tests {
    use super::worktree_origin_of;
    use std::path::Path;
    use std::process::Command;

    fn git(dir: &Path, args: &[&str]) {
        let mut command = Command::new("git");
        command.arg("-C").arg(dir).args(args);
        let output = crate::proc::quiet(&mut command).output().unwrap();
        assert!(output.status.success(), "git {args:?}: {}", String::from_utf8_lossy(&output.stderr));
    }

    #[test]
    fn only_a_linked_worktree_on_a_branch_has_an_origin() {
        let root = std::env::temp_dir().join(format!("maitty-origin-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        let main = root.join("main");
        std::fs::create_dir_all(&main).unwrap();
        git(&main, &["init", "-q", "-b", "main"]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        git(&main, &["config", "commit.gpgsign", "false"]);
        std::fs::write(main.join("a.txt"), "one\n").unwrap();
        git(&main, &["add", "a.txt"]);
        git(&main, &["commit", "-q", "-m", "first"]);
        let task = root.join("task");
        git(&main, &["worktree", "add", "-q", "-b", "task", task.to_str().unwrap()]);
        let loose = root.join("loose");
        git(&main, &["worktree", "add", "-q", "--detach", loose.to_str().unwrap()]);

        assert_eq!(worktree_origin_of(&main), None, "the main checkout is not a worktree");
        assert_eq!(worktree_origin_of(&loose), None, "a detached HEAD has no branch");
        let origin = worktree_origin_of(&task)
            .expect("a linked worktree on a branch");
        assert_eq!(origin.branch, "task");
        // The temp dir may be spelled with 8.3 short names; compare what each names on disk.
        let named = |p: &Path| crate::daemon::clean_path(std::fs::canonicalize(p).unwrap());
        assert_eq!(named(&origin.folder), named(&task));
        assert_eq!(named(&origin.main), named(&main));
        let _ = std::fs::remove_dir_all(&root);
    }
}

#[cfg(test)]
mod native_resume_tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn inherited_provider_environment_skips_windows_drive_pseudo_variables() {
        assert!(!valid_provider_environment_entry(
            OsStr::new("=C:"),
            OsStr::new("C:\\project")
        ));
        assert!(valid_provider_environment_entry(
            OsStr::new("PATH"),
            OsStr::new("C:\\tools")
        ));
    }

    #[test]
    fn claude_identity_accepts_only_the_exact_provider_session() {
        let mut settings = LaunchSettings {
            provider: Provider::Claude,
            executable: PathBuf::from("claude"),
            config_root: PathBuf::from("claude-config"),
            cwd: PathBuf::from("project"),
            model: None,
            model_provider: None,
            approval_policy: ApprovalPolicy::OnRequest,
            sandbox: Sandbox::ReadOnly,
        };
        assert_eq!(
            validate_response(
                &json!({"thread":{"id":"exact-claude-session"}}),
                &mut settings,
                Some("exact-claude-session")
            )
            .unwrap(),
            "exact-claude-session"
        );
        assert!(
            validate_response(
                &json!({"thread":{"id":"different-session"}}),
                &mut settings,
                Some("exact-claude-session")
            )
            .is_err()
        );
    }

    #[test]
    fn missing_claude_cache_is_explicitly_incomplete() {
        let (history, is_complete) = cached_claude_history(ConversationId(9), None, false).unwrap();
        assert!(!is_complete);
        assert_eq!(history.conversation, ConversationId(9));
        assert!(history.entries.is_empty());
        assert_eq!(
            history.completeness,
            Completeness::Incomplete(history::IncompleteReason::Disconnected)
        );
    }

    #[test]
    fn disconnected_claude_cache_keeps_saved_messages_visible_but_incomplete() {
        let id = ConversationId(10);
        let mut cached = History::new(id);
        cached.set_completeness(Completeness::Complete).unwrap();
        cached.entries.push(history::Entry {
            source: history::SourceId {
                conversation: id,
                turn: "turn-1".into(),
                item: "item-1".into(),
            },
            content: history::Content::Message {
                role: history::Role::User,
                text: "keep this message".into(),
            },
            state: history::ItemState::Completed,
            started_at_ms: None,
            completed_at_ms: None,
        });

        let (history, is_complete) = cached_claude_history(id, Some(&cached), false).unwrap();

        assert!(!is_complete);
        assert_eq!(history.entries, cached.entries);
        assert_eq!(
            history.completeness,
            Completeness::Incomplete(history::IncompleteReason::Disconnected)
        );
    }

    #[test]
    fn live_claude_history_stays_complete_without_rewriting_entries() {
        let id = ConversationId(11);
        let mut cached = History::new(id);
        cached.set_completeness(Completeness::Complete).unwrap();
        let (history, is_complete) = cached_claude_history(id, Some(&cached), true).unwrap();
        assert!(is_complete);
        assert_eq!(history.completeness, Completeness::Complete);
        assert_eq!(history.entries, cached.entries);
    }

    #[test]
    fn a_picked_model_and_effort_ride_on_the_codex_turn_start_and_nothing_else_changes() {
        let plain = send_params("thread", "hi", "1-1-1", &TurnChoice::default());
        assert!(plain.get("model").is_none() && plain.get("effort").is_none());
        let picked = send_params(
            "thread",
            "hi",
            "1-1-1",
            &TurnChoice { model: Some("gpt-x".into()), effort: Some("high".into()) },
        );
        assert_eq!(picked["model"], "gpt-x");
        assert_eq!(picked["effort"], "high");
        // Permissions are set when the thread starts and are not part of a turn's parameters.
        for key in ["approvalPolicy", "sandbox", "sandboxPolicy", "cwd"] {
            assert!(picked.get(key).is_none(), "{key}");
        }
        let mut stripped = picked.clone();
        stripped.as_object_mut().unwrap().retain(|key, _| key != "model" && key != "effort");
        assert_eq!(stripped, plain);
    }
}
