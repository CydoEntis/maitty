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
    HandoffSaving {
        client: ClientId,
        id: ConversationId,
        generation: u64,
    },
    HandoffStopping {
        client: ClientId,
        id: ConversationId,
        generation: u64,
    },
    HandoffNativeSaving {
        client: ClientId,
        id: ConversationId,
        generation: u64,
    },
    HandoffStarting {
        client: ClientId,
        id: ConversationId,
        generation: u64,
    },
    HandoffToChatSaving {
        client: ClientId,
        id: ConversationId,
        generation: u64,
        term: crate::protocol::TermId,
    },
    HandoffToChatStopping {
        client: ClientId,
        id: ConversationId,
        generation: u64,
        term: crate::protocol::TermId,
        process_reaped: bool,
        kill_tree_finished: bool,
        kill_tree_error: Option<String>,
    },
    HandoffToChatIdentitySaving {
        client: ClientId,
        id: ConversationId,
        generation: u64,
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
            | Self::HandoffSaving { client, .. }
            | Self::HandoffStopping { client, .. }
            | Self::HandoffNativeSaving { client, .. }
            | Self::HandoffStarting { client, .. }
            | Self::HandoffToChatSaving { client, .. }
            | Self::HandoffToChatStopping { client, .. }
            | Self::HandoffToChatIdentitySaving { client, .. }
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
            | Self::HandoffSaving { id, .. }
            | Self::HandoffStopping { id, .. }
            | Self::HandoffNativeSaving { id, .. }
            | Self::HandoffStarting { id, .. }
            | Self::HandoffToChatSaving { id, .. }
            | Self::HandoffToChatStopping { id, .. }
            | Self::HandoffToChatIdentitySaving { id, .. } => *id,
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
    }
}

fn native_resume_command(
    conversation: &Conversation,
) -> Result<crate::daemon::term::DirectCommand> {
    let settings = conversation
        .settings
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("Native conversation has no verified launch settings"))?;
    anyhow::ensure!(
        conversation.execution == ExecutionKind::Native
            && conversation.binding.provider == Provider::Codex,
        "Only a native Codex conversation can resume through the Codex terminal"
    );
    anyhow::ensure!(
        conversation.binding.profile.is_none(),
        "Native profile-v2 handoff is unsupported"
    );
    anyhow::ensure!(
        conversation.binding.account.is_none(),
        "Native account handoff is unsupported"
    );
    anyhow::ensure!(
        settings.executable == conversation.binding.executable
            && settings.config_root == conversation.binding.config_root
            && settings.cwd == conversation.binding.cwd,
        "Native conversation launch settings do not match its exact binding"
    );
    validate_settings(settings)
        .map_err(|_| anyhow::anyhow!("Invalid native Codex launch settings"))?;
    anyhow::ensure!(
        !conversation.binding.session_id.trim().is_empty()
            && !conversation
                .binding
                .session_id
                .chars()
                .any(char::is_control),
        "Native conversation has no exact session identity"
    );

    let mut args = vec!["resume".into(), "--no-daemon".into()];
    args.extend([
        "--cd".into(),
        settings.cwd.as_os_str().to_owned(),
        "--sandbox".into(),
        sandbox(settings).into(),
        "--ask-for-approval".into(),
        approval(settings).into(),
    ]);
    if let Some(model) = &settings.model {
        args.extend(["--model".into(), model.into()]);
    }
    args.push(conversation.binding.session_id.clone().into());
    Ok(crate::daemon::term::DirectCommand {
        executable: settings.executable.clone(),
        args,
        environment: vec![(
            "CODEX_HOME".into(),
            settings.config_root.as_os_str().to_owned(),
        )],
    })
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
    };
    if effective.get("type") != Some(&json!(kind))
        || effective
            .get("networkAccess")
            .is_some_and(|v| v != &json!(false))
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
                LaunchIdentity::Existing(c.clone())
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
                    *record = c.clone();
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
        if self
            .clients
            .get(&client)
            .is_some_and(|owner| owner.from.is_some() || owner.structured.is_some())
        {
            bail!("Only a human client can answer provider approval requests");
        }
        self.may(client, Grant::Respond, None)?;
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

    pub(in crate::daemon) fn conversation_answer_input(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        request_id: ProviderRequestId,
        answers: std::collections::BTreeMap<String, Vec<String>>,
    ) -> Result<()> {
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

    pub(in crate::daemon) fn conversation_handoff_to_native(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
    ) -> Result<()> {
        if self
            .clients
            .get(&client)
            .is_some_and(|client| client.from.is_some() || client.structured.is_some())
        {
            bail!("Only a human client can hand a conversation to native Codex");
        }
        self.may(client, Grant::Write, None)?;
        self.durable_conversation()?;

        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if conversation.generation != generation
            || conversation.state != RecoveryState::Running
            || conversation.execution != ExecutionKind::Structured
        {
            bail!("Conversation is stale or is not owned by structured Codex");
        }
        if conversation.binding.provider != Provider::Codex
            || conversation.binding.profile.is_some()
            || conversation.binding.account.is_some()
            || conversation.settings.is_none()
        {
            bail!("Conversation settings cannot be represented by native Codex");
        }
        if !self.conversation_runtimes.owns(id)
            || self
                .conversation_requests
                .values()
                .any(|pending| pending.conversation() == id)
        {
            bail!("Conversation has a pending operation or no live structured runtime");
        }
        if self.conversation_operations.iter().any(|operation| {
            operation.receipt.conversation == id
                && operation.receipt.delivery == Delivery::SubmissionUncertain
        }) {
            bail!("Conversation has an uncertain turn submission");
        }
        let state = self
            .conversation_states
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation runtime is not ready"))?;
        if !matches!(state.status, Status::Done | Status::Idle)
            || state.blocked
            || !state.history_ready
            || !state.approvals.is_empty()
            || !state.unsupported_requests.is_empty()
            || state.cancelling.is_some()
        {
            bail!("Conversation must be idle, unblocked and fully hydrated before handoff");
        }

        let token = self.conversation_token(Pending::HandoffSaving {
            client,
            id,
            generation,
        })?;
        self.conversations
            .iter_mut()
            .find(|conversation| conversation.id == id)
            .expect("validated conversation")
            .state = RecoveryState::Transferring;
        self.conversation_persist(token)?;
        self.dirty = true;
        Ok(())
    }

    pub(in crate::daemon) fn conversation_handoff_to_chat(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
    ) -> Result<()> {
        if self
            .clients
            .get(&client)
            .is_some_and(|client| client.from.is_some() || client.structured.is_some())
        {
            bail!("Only a human client can resume a conversation in chat");
        }
        self.may(client, Grant::Start, None)?;
        self.durable_conversation()?;

        let conversation = self
            .conversations
            .iter()
            .find(|conversation| conversation.id == id)
            .ok_or_else(|| anyhow::anyhow!("No such conversation"))?;
        if conversation.generation != generation
            || !matches!(
                conversation.state,
                RecoveryState::Running | RecoveryState::RecoveryRequired
            )
            || conversation.execution != ExecutionKind::Native
            || conversation.binding.provider != Provider::Codex
            || conversation.binding.profile.is_some()
            || conversation.binding.account.is_some()
            || conversation.settings.is_none()
        {
            bail!("Conversation is stale or cannot safely return to structured Codex");
        }
        if self.conversation_runtimes.owns(id)
            || self
                .conversation_requests
                .values()
                .any(|pending| pending.conversation() == id)
        {
            bail!("Conversation has a pending operation or structured runtime");
        }
        let term = self
            .terms
            .values()
            .find(|term| {
                term.conversation_id == Some(id)
                    && term.agent.as_deref() == Some("codex")
                    && term.session.as_deref() == Some(conversation.binding.session_id.as_str())
            })
            .ok_or_else(|| anyhow::anyhow!("No native Codex terminal owns this conversation"))?;
        if term.process_exited {
            bail!(
                "The native Codex process has exited; recover it from the TUI before returning to chat"
            );
        }
        if !matches!(term.status, Status::Done | Status::Idle) {
            bail!("Finish the native Codex turn before returning to chat");
        }
        let term_id = term.id;
        let state = self
            .conversation_states
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation runtime is not ready"))?;
        if state.blocked
            || !state.approvals.is_empty()
            || !state.unsupported_requests.is_empty()
            || state.cancelling.is_some()
        {
            bail!("Resolve pending conversation requests before returning to chat");
        }

        let token = self.conversation_token(Pending::HandoffToChatSaving {
            client,
            id,
            generation,
            term: term_id,
        })?;
        self.conversations
            .iter_mut()
            .find(|conversation| conversation.id == id)
            .expect("validated conversation")
            .state = RecoveryState::Transferring;
        self.conversation_persist(token)?;
        self.dirty = true;
        Ok(())
    }

    pub(in crate::daemon) fn conversation_handoff_to_chat_term_event(
        &mut self,
        term: crate::protocol::TermId,
        process_reaped: bool,
        kill_tree_result: Option<std::result::Result<(), String>>,
    ) -> bool {
        let token = self.conversation_requests.iter().find_map(|(token, pending)| {
            matches!(pending, Pending::HandoffToChatStopping { term: owner, .. } if *owner == term)
                .then_some(*token)
        });
        let Some(token) = token else { return false };
        if process_reaped && let Some(pane) = self.terms.get_mut(&term) {
            pane.process_exited = true;
        }
        let (ready, kill_tree_error) = if let Some(Pending::HandoffToChatStopping {
            process_reaped: reaped,
            kill_tree_finished: killed,
            kill_tree_error,
            ..
        }) = self.conversation_requests.get_mut(&token)
        {
            *reaped |= process_reaped;
            if let Some(result) = kill_tree_result {
                match result {
                    Ok(()) => *killed = true,
                    Err(error) => *kill_tree_error = Some(error),
                }
            }
            let failed = kill_tree_error.clone();
            ((*reaped && *killed) || failed.is_some(), failed)
        } else {
            (false, None)
        };
        if ready {
            let Some(Pending::HandoffToChatStopping {
                client,
                id,
                generation,
                term,
                process_reaped,
                ..
            }) = self.conversation_requests.remove(&token)
            else {
                return true;
            };
            if let Some(error) = kill_tree_error {
                if let Some(conversation) = self.conversations.iter_mut().find(|conversation| {
                    conversation.id == id
                        && conversation.generation == generation
                        && conversation.execution == ExecutionKind::Native
                        && conversation.state == RecoveryState::Transferring
                }) {
                    conversation.state = RecoveryState::RecoveryRequired;
                }
                self.conversation_fail(
                    Pending::HandoffToChatStopping {
                        client,
                        id,
                        generation,
                        term,
                        process_reaped,
                        kill_tree_finished: false,
                        kill_tree_error: Some(error.clone()),
                    },
                    format!("Could not confirm native Codex process cleanup: {error}"),
                );
                return true;
            }
            self.remove_term(term);
            let conversation = self.conversations.iter_mut().find(|conversation| {
                conversation.id == id
                    && conversation.generation == generation
                    && conversation.execution == ExecutionKind::Native
                    && conversation.state == RecoveryState::Transferring
            });
            if let Some(conversation) = conversation {
                let Some(next_generation) = conversation.generation.checked_add(1) else {
                    self.conversation_fail(
                        Pending::HandoffToChatIdentitySaving {
                            client,
                            id,
                            generation,
                        },
                        "Conversation generation exhausted".into(),
                    );
                    return true;
                };
                conversation.generation = next_generation;
                conversation.execution = ExecutionKind::Structured;
                conversation.state = RecoveryState::Stopped;
            } else {
                self.conversation_fail(
                    Pending::HandoffToChatIdentitySaving {
                        client,
                        id,
                        generation,
                    },
                    "Conversation ownership changed during handoff".into(),
                );
                return true;
            }
            let next_generation = generation + 1;
            self.conversation_states
                .entry(id)
                .or_default()
                .history_ready = false;
            self.conversation_requests.insert(
                token,
                Pending::HandoffToChatIdentitySaving {
                    client,
                    id,
                    generation: next_generation,
                },
            );
            if let Err(error) = self.persist_barrier(token)
                && let Some(pending) = self.conversation_requests.remove(&token)
            {
                self.conversation_fail(pending, error.to_string());
            }
            self.dirty = true;
        }
        true
    }

    fn spawn_native_conversation(&mut self, conversation: &Conversation) -> Result<()> {
        let settings = conversation.settings.as_ref().ok_or_else(|| {
            anyhow::anyhow!("Native conversation has no verified launch settings")
        })?;
        anyhow::ensure!(
            conversation.state == RecoveryState::RecoveryRequired,
            "Conversation settings cannot be represented by native Codex"
        );
        let direct = native_resume_command(conversation)?;

        let id = self.next();
        let mut term = crate::daemon::term::Term::spawn(
            &self.cfg,
            crate::daemon::term::SpawnSpec {
                id,
                cmd: None,
                direct: Some(direct),
                cwd: &settings.cwd,
                cols: self.guess_size().0,
                rows: self.guess_size().1,
                once: true,
            },
            self.tx.clone(),
        )?;
        term.agent = Some("codex".into());
        term.session = Some(conversation.binding.session_id.clone());
        term.conversation_id = Some(conversation.id);
        self.terms.insert(id, term);

        let workspace = self.next();
        let tab = self.next();
        self.workspaces.push(crate::daemon::WorkspaceInfo {
            id: workspace,
            name: String::new(),
            cwd: settings.cwd.clone(),
            tabs: vec![crate::daemon::TabInfo {
                id: tab,
                name: String::new(),
                layout: crate::layout::Node::Leaf(id),
                focus: id,
            }],
            active_tab: tab,
            git: None,
            worktree: false,
            color: self.free_color(),
            is_new: false,
            group: None,
        });
        self.active_ws = Some(workspace);
        self.had_terms = true;
        self.poll_git_soon();
        Ok(())
    }

    pub(in crate::daemon) fn conversation_send(
        &mut self,
        client: ClientId,
        id: ConversationId,
        generation: u64,
        operation: u64,
        text: String,
    ) -> Result<()> {
        self.may(client, Grant::Write, None)?;
        self.durable_conversation()?;
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
        let state = self
            .conversation_states
            .get(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation runtime is not ready"))?;
        if c.state != RecoveryState::Running
            || state.blocked
            || state.status == Status::Working
            || self
                .conversation_requests
                .values()
                .any(|p| p.conversation() == id)
        {
            bail!("Conversation is busy, blocked or not ready");
        }
        if !state.history_ready {
            bail!("Native history is incomplete; read it before submitting");
        }
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
        })?;
        self.conversation_persist(token)?;
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
                }
            })
            .collect();
        summaries.extend(self.pending_starts.iter().map(|p| {
            ConversationSummary {
                id: p.id,
                generation: p.generation,
                provider: Provider::Codex,
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
                        self.conversations.iter().any(|record| record == c)
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
            Pending::HandoffSaving {
                client,
                id,
                generation,
            } => {
                let valid = self.conversations.iter().any(|conversation| {
                    conversation.id == id
                        && conversation.generation == generation
                        && conversation.execution == ExecutionKind::Structured
                        && conversation.state == RecoveryState::Transferring
                }) && self.conversation_runtimes.owns(id)
                    && !self
                        .terms
                        .values()
                        .any(|term| term.conversation_id == Some(id));
                if !valid {
                    Err((
                        token,
                        "Conversation ownership changed before native handoff".into(),
                    ))
                } else {
                    self.conversation_requests.insert(
                        token,
                        Pending::HandoffStopping {
                            client,
                            id,
                            generation,
                        },
                    );
                    self.conversation_runtimes
                        .dispose(id)
                        .map_err(|error| (token, error.to_string()))
                }
            }
            Pending::HandoffStopping { .. } => Err((
                token,
                "Conversation handoff is waiting for runtime cleanup".into(),
            )),
            Pending::HandoffNativeSaving {
                client,
                id,
                generation,
            } => {
                let conversation = self
                    .conversations
                    .iter()
                    .find(|conversation| {
                        conversation.id == id
                            && conversation.generation == generation
                            && conversation.execution == ExecutionKind::Native
                            && conversation.state == RecoveryState::RecoveryRequired
                    })
                    .cloned();
                match conversation {
                    Some(conversation) => match self.spawn_native_conversation(&conversation) {
                        Ok(()) => {
                            self.conversations
                                .iter_mut()
                                .find(|conversation| conversation.id == id)
                                .expect("validated native conversation")
                                .state = RecoveryState::Running;
                            self.conversation_requests.insert(
                                token,
                                Pending::HandoffStarting {
                                    client,
                                    id,
                                    generation,
                                },
                            );
                            self.conversation_persist(token)
                                .map_err(|error| (token, error.to_string()))
                        }
                        Err(error) => {
                            self.conversation_requests.insert(
                                token,
                                Pending::HandoffNativeSaving {
                                    client,
                                    id,
                                    generation,
                                },
                            );
                            Err((token, error.to_string()))
                        }
                    },
                    None => {
                        self.conversation_requests.insert(
                            token,
                            Pending::HandoffNativeSaving {
                                client,
                                id,
                                generation,
                            },
                        );
                        Err((
                            token,
                            "Native conversation intent changed before launch".into(),
                        ))
                    }
                }
            }
            Pending::HandoffStarting {
                client,
                id,
                generation,
            } => {
                let valid = self.conversations.iter().any(|conversation| {
                    conversation.id == id
                        && conversation.generation == generation
                        && conversation.execution == ExecutionKind::Native
                        && conversation.state == RecoveryState::Running
                }) && self
                    .terms
                    .values()
                    .any(|term| term.conversation_id == Some(id));
                if valid {
                    self.send(client, ServerMsg::Reply(Reply::Ok));
                    Ok(())
                } else {
                    self.conversation_requests.insert(
                        token,
                        Pending::HandoffStarting {
                            client,
                            id,
                            generation,
                        },
                    );
                    Err((token, "Native conversation pane was not saved".into()))
                }
            }
            Pending::HandoffToChatSaving {
                client,
                id,
                generation,
                term,
            } => {
                let valid = self.conversations.iter().any(|conversation| {
                    conversation.id == id
                        && conversation.generation == generation
                        && conversation.execution == ExecutionKind::Native
                        && conversation.state == RecoveryState::Transferring
                }) && self.terms.get(&term).is_some_and(|pane| {
                    pane.conversation_id == Some(id)
                        && pane.agent.as_deref() == Some("codex")
                        && matches!(pane.status, Status::Done | Status::Idle)
                }) && !self.conversation_runtimes.owns(id);
                if !valid {
                    Err((
                        token,
                        "Native conversation changed before chat handoff".into(),
                    ))
                } else {
                    self.conversation_requests.insert(
                        token,
                        Pending::HandoffToChatStopping {
                            client,
                            id,
                            generation,
                            term,
                            process_reaped: false,
                            kill_tree_finished: false,
                            kill_tree_error: None,
                        },
                    );
                    if let Some(pane) = self.terms.get_mut(&term) {
                        pane.kill_tree_for_handoff(term, self.tx.clone());
                        Ok(())
                    } else {
                        self.conversation_requests.remove(&token);
                        Err((
                            token,
                            "Native conversation terminal exited before handoff".into(),
                        ))
                    }
                }
            }
            Pending::HandoffToChatStopping { .. } => Err((
                token,
                "Conversation handoff is waiting for native process cleanup".into(),
            )),
            Pending::HandoffToChatIdentitySaving {
                client,
                id,
                generation,
            } => {
                let launch = self
                    .conversations
                    .iter()
                    .find(|conversation| {
                        conversation.id == id
                            && conversation.generation == generation
                            && conversation.execution == ExecutionKind::Structured
                            && conversation.state == RecoveryState::Stopped
                    })
                    .cloned();
                match launch {
                    Some(conversation)
                        if !self.conversation_runtimes.owns(id)
                            && !self
                                .terms
                                .values()
                                .any(|pane| pane.conversation_id == Some(id)) =>
                    {
                        let launch = Launch {
                            identity: LaunchIdentity::Existing(conversation),
                            environment: std::env::vars_os()
                                .filter(|(key, value)| {
                                    valid_provider_environment_entry(key, value)
                                        && !key.to_string_lossy().eq_ignore_ascii_case("CODEX_HOME")
                                })
                                .collect(),
                            #[cfg(test)]
                            failure_point: None,
                        };
                        self.conversation_requests
                            .insert(token, Pending::Validating { client, id });
                        let tx = self.tx.clone();
                        tokio::spawn(async move {
                            let result = launch.validate().await.map_err(|error| error.to_string());
                            let _ = tx
                                .send(crate::daemon::Ev::ConversationValidated { token, result })
                                .await;
                        });
                        Ok(())
                    }
                    _ => {
                        self.conversation_requests.insert(
                            token,
                            Pending::HandoffToChatIdentitySaving {
                                client,
                                id,
                                generation,
                            },
                        );
                        Err((
                            token,
                            "Structured conversation identity was not saved".into(),
                        ))
                    }
                }
            }
            Pending::Submit {
                client,
                receipt,
                text,
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
                self.conversation_runtimes.request(id, generation, token, Rpc::Send { params: json!({"threadId":thread,"input":[{"type":"text","text":text,"text_elements":[]}],"clientUserMessageId":format!("{}-{generation}-{operation}", id.0)}) }).map_err(|e| (token, e.to_string()))
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
                                snapshot.conversations.iter().any(|saved| saved == c)
                            }
                        },
                        Pending::Identity { id, generation, .. } => {
                            snapshot.conversations.iter().any(|saved| {
                                saved.id == *id
                                    && saved.generation == *generation
                                    && self.conversations.iter().any(|current| current == saved)
                            })
                        }
                        Pending::HandoffSaving { id, generation, .. } => {
                            snapshot.conversations.iter().any(|saved| {
                                saved.id == *id
                                    && saved.generation == *generation
                                    && saved.state == RecoveryState::Transferring
                                    && self.conversations.iter().any(|current| current == saved)
                            })
                        }
                        Pending::HandoffNativeSaving { id, generation, .. } => {
                            snapshot.conversations.iter().any(|saved| {
                                saved.id == *id
                                    && saved.generation == *generation
                                    && saved.execution == ExecutionKind::Native
                                    && saved.state == RecoveryState::RecoveryRequired
                                    && self.conversations.iter().any(|current| current == saved)
                            })
                        }
                        Pending::HandoffStarting { id, generation, .. } => {
                            snapshot.conversations.iter().any(|saved| {
                                saved.id == *id
                                    && saved.generation == *generation
                                    && saved.execution == ExecutionKind::Native
                                    && saved.state == RecoveryState::Running
                                    && self.conversations.iter().any(|current| current == saved)
                            }) && snapshot.workspaces.iter().any(|workspace| {
                                workspace.tabs.iter().any(|tab| {
                                    tab.panes
                                        .values()
                                        .any(|pane| pane.conversation_id == Some(*id))
                                })
                            })
                        }
                        Pending::HandoffToChatSaving {
                            id,
                            generation,
                            term,
                            ..
                        } => {
                            snapshot.conversations.iter().any(|saved| {
                                saved.id == *id
                                    && saved.generation == *generation
                                    && saved.execution == ExecutionKind::Native
                                    && saved.state == RecoveryState::Transferring
                                    && self.conversations.iter().any(|current| current == saved)
                            }) && snapshot.workspaces.iter().any(|workspace| {
                                workspace.tabs.iter().any(|tab| {
                                    tab.panes
                                        .get(term)
                                        .is_some_and(|pane| pane.conversation_id == Some(*id))
                                })
                            })
                        }
                        Pending::HandoffToChatIdentitySaving { id, generation, .. } => {
                            snapshot.conversations.iter().any(|saved| {
                                saved.id == *id
                                    && saved.generation == *generation
                                    && saved.execution == ExecutionKind::Structured
                                    && saved.state == RecoveryState::Stopped
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
        if cursor
            .as_ref()
            .is_some_and(|cursor| cursor.generation != generation)
        {
            bail!("Stale history cursor generation");
        }
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
        let mut page = history
            .read(read.cursor.as_ref(), read.limit, history::MAX_READ_BYTES)
            .map_err(|e| anyhow::anyhow!("History read refused: {e:?}"))?;
        if let Some(next) = page.next.as_mut() {
            next.generation = c.generation;
        }
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
                approvals: state.map_or_else(Vec::new, |s| s.approvals.values().cloned().collect()),
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
        if let Pending::HandoffSaving { id, generation, .. } = &pending
            && let Some(conversation) = self.conversations.iter_mut().find(|conversation| {
                conversation.id == *id && conversation.generation == *generation
            })
            && conversation.state == RecoveryState::Transferring
        {
            conversation.state = RecoveryState::Running;
        }
        if let Pending::HandoffToChatSaving { id, generation, .. } = &pending
            && let Some(conversation) = self.conversations.iter_mut().find(|conversation| {
                conversation.id == *id && conversation.generation == *generation
            })
            && conversation.state == RecoveryState::Transferring
        {
            conversation.state = RecoveryState::Running;
        }
        if let Pending::HandoffStarting { id, generation, .. } = &pending
            && let Some(conversation) = self.conversations.iter_mut().find(|conversation| {
                conversation.id == *id && conversation.generation == *generation
            })
            && conversation.state == RecoveryState::Running
        {
            conversation.state = RecoveryState::RecoveryRequired;
        }
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
            | Pending::HandoffSaving { client, .. }
            | Pending::HandoffStopping { client, .. }
            | Pending::HandoffNativeSaving { client, .. }
            | Pending::HandoffStarting { client, .. }
            | Pending::HandoffToChatSaving { client, .. }
            | Pending::HandoffToChatStopping { client, .. }
            | Pending::HandoffToChatIdentitySaving { client, .. }
            | Pending::Receipt { client, .. } => Some(client),
            Pending::Hydration { read, started, .. } => read.map(|r| r.client).or(started),
        };
        let state = self.conversation_states.entry(id).or_default();
        state.status = Status::Blocked;
        state.error = Some(error.clone());
        state.history_ready = false;
        if let Some(client) = client {
            self.send(client, ServerMsg::Error(error));
        }
        self.dirty = true;
    }
}

#[cfg(test)]
mod native_resume_tests {
    use super::*;
    use std::ffi::OsString;
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

    fn native_conversation() -> Conversation {
        let mut conversation = crate::conversation::tests::conversation();
        conversation.execution = ExecutionKind::Native;
        conversation.state = RecoveryState::RecoveryRequired;
        conversation.binding.session_id = "exact session; & whoami".into();
        conversation.settings = Some(LaunchSettings {
            provider: Provider::Codex,
            executable: conversation.binding.executable.clone(),
            config_root: conversation.binding.config_root.clone(),
            cwd: conversation.binding.cwd.clone(),
            model: Some("codex-model".into()),
            model_provider: None,
            approval_policy: ApprovalPolicy::OnRequest,
            sandbox: Sandbox::WorkspaceWrite,
        });
        conversation.binding.profile = None;
        conversation.binding.account = None;
        conversation
    }

    #[test]
    fn native_resume_uses_exact_identity_as_one_argument() {
        let conversation = native_conversation();
        let command = native_resume_command(&conversation).unwrap();
        assert_eq!(
            command.args,
            [
                OsString::from("resume"),
                OsString::from("--no-daemon"),
                OsString::from("--cd"),
                conversation.binding.cwd.as_os_str().to_owned(),
                OsString::from("--sandbox"),
                OsString::from("workspace-write"),
                OsString::from("--ask-for-approval"),
                OsString::from("on-request"),
                OsString::from("--model"),
                OsString::from("codex-model"),
                OsString::from("exact session; & whoami"),
            ]
        );
        assert_eq!(command.environment[0].0, "CODEX_HOME");
        assert_eq!(
            command.environment[0].1,
            conversation.binding.config_root.as_os_str()
        );
    }

    #[test]
    fn native_resume_refuses_unrepresented_profiles_and_stale_bindings() {
        let mut conversation = native_conversation();
        conversation.binding.profile = Some("unsupported-profile".into());
        assert!(native_resume_command(&conversation).is_err());

        conversation.binding.profile = None;
        conversation.binding.cwd.push("different");
        assert!(native_resume_command(&conversation).is_err());
    }
}
