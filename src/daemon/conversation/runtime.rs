use super::transport::{
    Envelope, HANDSHAKE_TIMEOUT, Launch, LaunchIdentity, RequestId, RpcError, StartupFailure,
};
use crate::conversation::{ConversationId, ExecutionKind, RecoveryState};
use crate::daemon::Ev;
use anyhow::{Result, bail};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio::sync::mpsc;

const RUNTIME_COMMAND_QUEUE: usize = 8;

pub(super) enum Rpc {
    Start {
        params: Value,
    },
    Resume {
        params: Value,
    },
    Turns {
        params: Value,
    },
    Send {
        params: Value,
    },
    Interrupt {
        params: Value,
    },
    Answer {
        request_id: RequestId,
        response: Value,
    },
}
impl Rpc {
    fn parts(self) -> Option<(&'static str, Value)> {
        match self {
            Self::Start { params } => Some(("thread/start", params)),
            Self::Resume { params } => Some(("thread/resume", params)),
            Self::Turns { params } => Some(("thread/turns/list", params)),
            Self::Send { params } => Some(("turn/start", params)),
            Self::Interrupt { params } => Some(("turn/interrupt", params)),
            Self::Answer { .. } => None,
        }
    }
}
pub(super) struct Command {
    pub(super) token: u64,
    pub(super) request: Rpc,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::daemon) struct Owner {
    pub(in crate::daemon) conversation: ConversationId,
    pub(in crate::daemon) generation: u64,
    connection: u64,
}

#[cfg(test)]
pub(super) fn test_owner(conversation: ConversationId, generation: u64) -> Owner {
    Owner {
        conversation,
        generation,
        connection: 1,
    }
}

pub(crate) struct Event {
    pub(super) owner: Owner,
    pub(super) payload: Payload,
}

pub(in crate::daemon) enum Payload {
    Response {
        token: u64,
        result: Result<Value, RpcError>,
    },
    Ready,
    Notification {
        method: String,
        params: Value,
    },
    Request {
        id: RequestId,
        method: String,
        params: Value,
    },
    Terminated {
        failure: Option<String>,
        reaped: bool,
    },
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Starting,
    Live,
    Stopping,
    CleanupFailed,
}
struct Runtime {
    owner: Owner,
    phase: Phase,
    stop: mpsc::Sender<()>,
    commands: mpsc::Sender<Command>,
    task: tokio::task::JoinHandle<()>,
    secret: String,
}
impl Drop for Runtime {
    fn drop(&mut self) {
        // Aborting drops the owned connection and its tree guard on shutdown.
        self.task.abort();
    }
}

#[derive(Default)]
pub(in crate::daemon) struct Registry {
    next_connection: u64,
    entries: BTreeMap<ConversationId, Runtime>,
}

#[cfg(test)]
pub(super) enum TestCleanupState {
    Active,
    Requested,
    Finished,
}

impl Registry {
    #[cfg(test)]
    pub(super) fn test_identity(&self, id: ConversationId) -> crate::protocol::StructuredIdentity {
        let runtime = &self.entries[&id];
        crate::protocol::StructuredIdentity {
            conversation: id,
            generation: runtime.owner.generation,
            connection: runtime.owner.connection,
            token: runtime.secret.clone(),
        }
    }
    pub(in crate::daemon) fn authenticate(
        &self,
        claim: &crate::protocol::StructuredIdentity,
    ) -> Option<Owner> {
        let runtime = self.entries.get(&claim.conversation)?;
        (runtime.phase == Phase::Live
            && runtime.owner.generation == claim.generation
            && runtime.owner.connection == claim.connection
            && crate::daemon::term::same_secret(&runtime.secret, &claim.token))
        .then_some(runtime.owner)
    }
    pub(in crate::daemon) fn authenticates(&self, owner: Owner) -> bool {
        self.entries
            .get(&owner.conversation)
            .is_some_and(|runtime| runtime.owner == owner && runtime.phase == Phase::Live)
    }
    pub(in crate::daemon) fn owns(&self, id: ConversationId) -> bool {
        self.entries.contains_key(&id)
    }
    #[cfg(test)]
    pub(super) fn test_cleanup_state(&self, id: ConversationId) -> Option<TestCleanupState> {
        self.entries.get(&id).map(|runtime| {
            if runtime.task.is_finished() {
                TestCleanupState::Finished
            } else if runtime.phase == Phase::Stopping {
                TestCleanupState::Requested
            } else {
                TestCleanupState::Active
            }
        })
    }

    pub(super) fn start(&mut self, launch: Launch, events: mpsc::Sender<Ev>) -> Result<()> {
        if matches!(&launch.identity, LaunchIdentity::Existing(c) if c.execution != ExecutionKind::Structured || c.state != RecoveryState::Stopped)
            || self.entries.contains_key(&launch.id())
        {
            bail!("Conversation already owns a runtime or requires explicit recovery");
        }
        self.next_connection = self
            .next_connection
            .checked_add(1)
            .ok_or_else(|| anyhow::anyhow!("Provider connection identity exhausted"))?;
        let owner = Owner {
            conversation: launch.id(),
            generation: launch.generation(),
            connection: self.next_connection,
        };
        let id = launch.id();
        let secret = crate::daemon::term::new_secret()?;
        let identity = crate::protocol::StructuredIdentity {
            conversation: id,
            generation: owner.generation,
            connection: owner.connection,
            token: secret.clone(),
        };
        let (stop, stopped) = mpsc::channel(RUNTIME_COMMAND_QUEUE);
        let (commands, requests) = mpsc::channel(RUNTIME_COMMAND_QUEUE);
        let task = tokio::spawn(run(launch, owner, identity, events, stopped, requests));
        self.entries.insert(
            id,
            Runtime {
                owner,
                phase: Phase::Starting,
                stop,
                commands,
                task,
                secret,
            },
        );
        Ok(())
    }

    pub(super) fn request(
        &self,
        id: ConversationId,
        generation: u64,
        token: u64,
        request: Rpc,
    ) -> Result<()> {
        let runtime = self
            .entries
            .get(&id)
            .filter(|r| r.owner.generation == generation && r.phase == Phase::Live)
            .ok_or_else(|| anyhow::anyhow!("Conversation runtime is not ready"))?;
        runtime
            .commands
            .try_send(Command { token, request })
            .map_err(|_| anyhow::anyhow!("Conversation request queue is full or closed"))
    }

    pub(super) fn dispose(&mut self, id: ConversationId) -> Result<()> {
        let runtime = self
            .entries
            .get_mut(&id)
            .ok_or_else(|| anyhow::anyhow!("Conversation has no live runtime"))?;
        if runtime.phase == Phase::Stopping {
            bail!("Conversation runtime disposal already requested");
        }
        runtime
            .stop
            .try_send(())
            .map_err(|_| anyhow::anyhow!("Conversation actor exited before disposal"))?;
        runtime.phase = Phase::Stopping;
        Ok(())
    }

    pub(super) fn accept(&mut self, event: &Event, generation: u64) -> bool {
        let Some(runtime) = self.entries.get_mut(&event.owner.conversation) else {
            return false;
        };
        if runtime.owner != event.owner || generation != event.owner.generation {
            return false;
        }
        match event.payload {
            Payload::Terminated { reaped: true, .. } => {
                self.entries.remove(&event.owner.conversation);
            }
            Payload::Terminated { reaped: false, .. } => {
                runtime.phase = Phase::CleanupFailed;
            }
            _ if matches!(runtime.phase, Phase::Stopping | Phase::CleanupFailed) => return false,
            Payload::Ready => runtime.phase = Phase::Live,
            _ if runtime.phase != Phase::Live => return false,
            _ => {}
        }
        true
    }
}

pub(super) async fn emit(events: &mpsc::Sender<Ev>, owner: Owner, payload: Payload) -> Result<()> {
    events
        .send(Ev::Conversation(Event { owner, payload }))
        .await
        .map_err(|_| anyhow::anyhow!("Daemon provider event receiver closed"))
}

async fn run(
    launch: Launch,
    owner: Owner,
    identity: crate::protocol::StructuredIdentity,
    events: mpsc::Sender<Ev>,
    mut stop: mpsc::Receiver<()>,
    mut requests: mpsc::Receiver<Command>,
) {
    let launch = match launch.validate().await {
        Ok(launch) => launch,
        Err(error) => {
            // A closed daemon channel needs no further notification.
            let _ = emit(
                &events,
                owner,
                Payload::Terminated {
                    failure: Some(format!("{error:#}")),
                    reaped: true,
                },
            )
            .await;
            return;
        }
    };
    if launch.provider() == crate::conversation::Provider::Claude {
        super::claude::run(launch, owner, events, stop, requests).await;
        return;
    }
    let connected = launch.connect(&identity).await;
    let mut connection = match connected {
        Ok(connection) => connection,
        Err(failure) => {
            retain_failed_startup(failure, owner, &events, &mut stop).await;
            return;
        }
    };
    #[cfg(test)]
    if launch.failure_point == Some(super::transport::FailurePoint::RuntimeDisposal) {
        connection.process.fail_next_stop();
    }
    let work = async {
        tokio::time::timeout(HANDSHAKE_TIMEOUT, connection.handshake(launch.paths().0))
            .await
            .map_err(|_| anyhow::anyhow!("Codex initialize timed out"))??;
        let (policy, preface) = tokio::time::timeout(
            HANDSHAKE_TIMEOUT,
            connection.runtime_policy(launch.paths().2),
        )
        .await
        .map_err(|_| anyhow::anyhow!("Codex effective configuration read timed out"))??;
        emit(&events, owner, Payload::Ready).await?;
        for envelope in preface {
            if let Envelope::Notification { method, params } = envelope {
                emit(&events, owner, Payload::Notification { method, params }).await?;
            }
        }
        let mut pending = BTreeMap::new();
        let mut next_id: i64 = super::transport::CONFIG_READ_ID;
        let mut deadline = tokio::time::interval(std::time::Duration::from_millis(250));
        loop {
            let envelope = tokio::select! {
                command = requests.recv() => {
                    let command = command.ok_or_else(|| anyhow::anyhow!("Codex command sender closed"))?;
                    if pending.len() >= RUNTIME_COMMAND_QUEUE { bail!("Too many pending Codex requests"); }
                    next_id = next_id.checked_add(1).ok_or_else(|| anyhow::anyhow!("Codex request identity exhausted"))?;
                    let (method, mut params) = match command.request {
                        Rpc::Answer {
                            request_id,
                            response,
                        } => {
                            connection.respond(&request_id, response).await?;
                            continue;
                        }
                        request => request.parts().ok_or_else(|| anyhow::anyhow!("Invalid Codex runtime command"))?,
                    };
                    if matches!(method,"thread/start"|"thread/resume") {
                        let config=params.get_mut("config").and_then(Value::as_object_mut).ok_or_else(||anyhow::anyhow!("Structured launch configuration overrides missing"))?;
                        config.extend(policy.iter().map(|(key,value)|(key.clone(),value.clone())));
                    }
                    pending.insert(next_id, (command.token, tokio::time::Instant::now()));
                    connection.request(next_id, method, params).await?;
                    continue;
                }
                _ = deadline.tick() => {
                    if pending.values().any(|(_, sent)| sent.elapsed() > std::time::Duration::from_secs(30)) { bail!("Codex response timed out; delivery may be uncertain"); }
                    continue;
                }
                envelope = connection.next() => envelope?,
            };
            let payload = match envelope {
                Envelope::Response {
                    id: RequestId::Integer(id),
                    result,
                } => {
                    let (token, _) = pending
                        .remove(&id)
                        .ok_or_else(|| anyhow::anyhow!("Codex response has no pending request"))?;
                    Payload::Response { token, result }
                }
                Envelope::Response { .. } => bail!("Codex response has no pending request"),
                Envelope::Notification { method, params } => {
                    Payload::Notification { method, params }
                }
                Envelope::Request { id, method, params } => Payload::Request { id, method, params },
            };
            emit(&events, owner, payload).await?;
        }
    };
    let result = tokio::select! {
        biased;
        _ = stop.recv() => Ok::<(), anyhow::Error>(()),
        result = work => result,
    };
    let cleanup = connection.stop().await;
    let reaped = cleanup.is_ok();
    let failure = match (result, cleanup) {
        (Err(error), Ok(())) | (Ok(()), Err(error)) => Some(format!("{error:#}")),
        (Err(error), Err(cleanup)) => Some(format!("{error:#}; cleanup failed: {cleanup:#}")),
        (Ok(()), Ok(())) => None,
    };
    if reaped {
        // Release executable and pipe handles before observers can delete a stopped fixture or replace its binary.
        drop(connection);
        let _ = emit(
            &events,
            owner,
            Payload::Terminated {
                failure,
                reaped: true,
            },
        )
        .await;
        return;
    }
    if emit(&events, owner, Payload::Terminated { failure, reaped })
        .await
        .is_err()
        || reaped
    {
        return;
    }
    // Cleanup failures keep the actor and its process guard owned until an
    // explicit disposal retry confirms exit. They never retry a provider mutation.
    while stop.recv().await.is_some() {
        let cleanup = connection.stop().await;
        let reaped = cleanup.is_ok();
        let failure = cleanup.err().map(|error| format!("{error:#}"));
        if reaped {
            drop(connection);
            let _ = emit(
                &events,
                owner,
                Payload::Terminated {
                    failure,
                    reaped: true,
                },
            )
            .await;
            return;
        }
        if emit(&events, owner, Payload::Terminated { failure, reaped })
            .await
            .is_err()
            || reaped
        {
            return;
        }
    }
}

pub(super) async fn retain_failed_startup(
    mut failure: StartupFailure,
    owner: Owner,
    events: &mpsc::Sender<Ev>,
    stop: &mut mpsc::Receiver<()>,
) {
    let reported = emit(
        events,
        owner,
        Payload::Terminated {
            failure: Some(format!("{:#}", failure.error)),
            reaped: failure.reaped,
        },
    )
    .await;
    if reported.is_err() || failure.reaped {
        return;
    }
    while stop.recv().await.is_some() {
        let cleanup = match &mut failure.process {
            Some(process) => process.stop().await.map(|_| ()),
            None => Err(std::io::Error::other(
                "Provider process exit cannot be confirmed after launch task failure",
            )),
        };
        let reaped = cleanup.is_ok();
        let failure = cleanup
            .err()
            .map(|error| format!("Dispose failed Codex startup: {error}"));
        if emit(events, owner, Payload::Terminated { failure, reaped })
            .await
            .is_err()
            || reaped
        {
            return;
        }
    }
}
