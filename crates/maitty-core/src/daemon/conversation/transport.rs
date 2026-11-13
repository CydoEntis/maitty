use super::process::OwnedChild;
use crate::conversation::{
    Conversation, LaunchSettings, PendingStart, Provider, validate_registry,
};
use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};
use tokio_util::codec::{FramedRead, FramedWrite, LinesCodec};

pub(super) const MAX_FRAME_BYTES: usize = 1024 * 1024;
const MAX_PREFLIGHT_BYTES: usize = 8 * 1024 * 1024;
const STDERR_CHUNK_BYTES: usize = 8192;
const PREFLIGHT_TIMEOUT: Duration = Duration::from_secs(20);
pub(super) const HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const INITIALIZE_ID: i64 = 1;
pub(super) const CONFIG_READ_ID: i64 = 2;
const MAX_STARTUP_NOTIFICATIONS: usize = 8;

// The caller supplies the effective environment; values can contain credentials.
// Deliberately no Debug/Serialize implementation for launch configuration.
pub(crate) struct Launch {
    pub(super) identity: LaunchIdentity,
    pub(crate) environment: Vec<(OsString, OsString)>,
    #[cfg(test)]
    pub(super) failure_point: Option<FailurePoint>,
}

pub(super) enum LaunchIdentity {
    Existing(Box<Conversation>),
    Pending(PendingStart),
}

impl Launch {
    pub(super) fn id(&self) -> crate::conversation::ConversationId {
        match &self.identity {
            LaunchIdentity::Existing(c) => c.id,
            LaunchIdentity::Pending(p) => p.id,
        }
    }
    pub(super) fn generation(&self) -> u64 {
        match &self.identity {
            LaunchIdentity::Existing(c) => c.generation,
            LaunchIdentity::Pending(p) => p.generation,
        }
    }
    pub(super) fn settings(&self) -> Option<&LaunchSettings> {
        match &self.identity {
            LaunchIdentity::Existing(c) => c.settings.as_ref(),
            LaunchIdentity::Pending(p) => Some(&p.settings),
        }
    }
    pub(super) fn provider(&self) -> Provider {
        self.settings().map_or_else(
            || match &self.identity {
                LaunchIdentity::Existing(conversation) => conversation.binding.provider,
                LaunchIdentity::Pending(_) => Provider::Codex,
            },
            |settings| settings.provider,
        )
    }
    pub(super) fn paths(&self) -> (&Path, &Path, &Path) {
        match &self.identity {
            LaunchIdentity::Existing(c) => (
                &c.binding.config_root,
                &c.binding.executable,
                &c.binding.cwd,
            ),
            LaunchIdentity::Pending(p) => (
                &p.settings.config_root,
                &p.settings.executable,
                &p.settings.cwd,
            ),
        }
    }
}

pub(super) struct StartupFailure {
    pub(super) error: anyhow::Error,
    pub(super) process: Option<Box<OwnedChild>>,
    pub(super) reaped: bool,
}

impl From<anyhow::Error> for StartupFailure {
    fn from(error: anyhow::Error) -> Self {
        Self {
            error,
            process: None,
            reaped: true,
        }
    }
}

impl StartupFailure {
    pub(super) fn spawn(error: super::process::SpawnFailure, context: &'static str) -> Self {
        Self {
            error: anyhow::Error::new(error.error).context(context),
            process: error.process,
            reaped: error.reaped,
        }
    }

    pub(super) async fn dispose(mut process: OwnedChild, error: anyhow::Error) -> Self {
        match process.stop().await {
            Ok(_) => error.into(),
            Err(cleanup) => Self {
                error: error.context(format!("Provider cleanup failed: {cleanup}")),
                process: Some(Box::new(process)),
                reaped: false,
            },
        }
    }
}

#[cfg(test)]
#[derive(PartialEq, Eq)]
pub(super) enum FailurePoint {
    PreflightDisposal,
    RuntimeDisposal,
}

impl Launch {
    pub(super) async fn validate(mut self) -> Result<Self> {
        tokio::task::spawn_blocking(move || {
            let provider = self.provider();
            // Gemini's chat is read from its terminal's transcript (follow.rs); it has no structured runtime.
            if provider == Provider::Gemini { bail!("Gemini has no structured chat runtime"); }
            if let LaunchIdentity::Existing(c) = &self.identity {
                validate_registry(std::slice::from_ref(c)).map_err(|_| anyhow::anyhow!("Invalid structured conversation binding"))?;
                if c.binding.provider != provider || c.binding.profile.is_some() {
                    bail!("Unsupported provider binding: profile-v2 and native profile configuration have no verified structured-chat mapping");
                }
            }
            // A chat in a worktree whose folder was deleted gets it back from its branch.
            if let LaunchIdentity::Existing(c) = &self.identity
                && let Some(origin) = &c.worktree_origin
                && !c.binding.cwd.is_dir()
                && c.binding.cwd.starts_with(&origin.folder)
            {
                crate::daemon::worktrees::recreate_worktree(&origin.main, &origin.folder, &origin.branch)?;
            }
            let (config_root, executable, cwd) = self.paths();
            for path in [config_root, cwd] {
                if !path.is_absolute() { bail!("Provider binding requires absolute paths"); }
                if !path.is_dir() { bail!("Provider binding requires existing configuration and working directories"); }
            }
            if !executable.is_absolute() || !executable.is_file() { bail!("Selected provider executable does not exist at an absolute path"); }
            let home = std::fs::canonicalize(config_root).context("Resolve selected provider configuration root")?;
            let executable = crate::daemon::clean_path(
                crate::proc::resolve_executable_path(executable)
                    .context("Resolve selected provider executable")?,
            );
            let cwd=crate::daemon::clean_path(std::fs::canonicalize(cwd).context("Resolve selected provider working directory")?);
            for (key, value) in &self.environment {
                if key.is_empty() || key.to_string_lossy().contains(['=', '\0']) || value.to_string_lossy().contains('\0') {
                    bail!("Invalid effective provider environment");
                }
                let config_variable = match provider {
                    Provider::Codex => "CODEX_HOME",
                    Provider::Claude => "CLAUDE_CONFIG_DIR",
                    Provider::Gemini => unreachable!("refused above"),
                };
                if key.to_string_lossy().eq_ignore_ascii_case(config_variable)
                    && std::fs::canonicalize(Path::new(value)).ok().as_ref() != Some(&home) {
                    bail!("Effective environment disagrees with selected provider configuration root");
                }
            }
            let home=crate::daemon::clean_path(home);
            match &mut self.identity {
                LaunchIdentity::Pending(p)=>{p.settings.config_root=home;p.settings.executable=executable;p.settings.cwd=cwd;}
                LaunchIdentity::Existing(c)=>{
                    c.binding.config_root=home.clone();c.binding.executable=executable.clone();c.binding.cwd=cwd.clone();
                    if let Some(settings)=c.settings.as_mut() {settings.config_root=home;settings.executable=executable;settings.cwd=cwd;}
                }
            }
            Ok(self)
        }).await.context("Validate Codex launch binding in background")?
    }

    fn command(&self, home: &Path) -> Command {
        let (_, executable, cwd) = self.paths();
        let mut command = Command::new(executable);
        command
            .env_clear()
            .envs(
                self.environment
                    .iter()
                    .filter(|(key, _)| {
                        !crate::ipc::STRUCTURED_ENV
                            .iter()
                            .chain(
                                [
                                    concat!(env!("APP_ENV"), "_TERM_ID"),
                                    concat!(env!("APP_ENV"), "_PANE_TOKEN"),
                                ]
                                .iter(),
                            )
                            .any(|excluded| key.to_string_lossy().eq_ignore_ascii_case(excluded))
                    })
                    .cloned(),
            )
            .env("CODEX_HOME", home)
            .current_dir(cwd)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        crate::proc::quiet(&mut command);
        command
    }

    pub(super) async fn connect(
        &self,
        identity: &crate::protocol::StructuredIdentity,
    ) -> Result<Connection, StartupFailure> {
        self.preflight().await?;
        let mut command = self.command(self.paths().0);
        command
            .env(
                crate::ipc::STRUCTURED_ENV[0],
                identity.conversation.0.to_string(),
            )
            .env(
                crate::ipc::STRUCTURED_ENV[1],
                identity.generation.to_string(),
            )
            .env(
                crate::ipc::STRUCTURED_ENV[2],
                identity.connection.to_string(),
            )
            .env(crate::ipc::STRUCTURED_ENV[3], &identity.token);
        command.arg("app-server").args(["--listen", "stdio://"]);
        let mut process = OwnedChild::spawn(command)
            .await
            .map_err(|error| StartupFailure::spawn(error, "Launch selected Codex app-server"))?;
        let (Some(stdin), Some(stdout), Some(stderr)) = (
            process.child.stdin.take(),
            process.child.stdout.take(),
            process.child.stderr.take(),
        ) else {
            return Err(StartupFailure::dispose(
                process,
                anyhow::anyhow!("Codex app-server pipe missing"),
            )
            .await);
        };
        let stderr_task = tokio::spawn(discard_stderr(stderr));
        Ok(Connection {
            process,
            input: FramedWrite::new(stdin, LinesCodec::new_with_max_length(MAX_FRAME_BYTES)),
            output: FramedRead::new(stdout, LinesCodec::new_with_max_length(MAX_FRAME_BYTES)),
            stderr_task: Some(stderr_task),
        })
    }

    async fn preflight(&self) -> Result<(), StartupFailure> {
        let directory = PreflightDirectory::create().await?;
        let home = directory.0.join("home");
        let schema = directory.0.join("schema");
        let mut version = self.command(&home);
        version.arg("--version");
        let version = self.run_preflight(version).await?;
        let version = std::str::from_utf8(&version)
            .context("Codex version is not UTF-8")?
            .trim();
        let version = version
            .strip_prefix("codex-cli ")
            .context("Unrecognized Codex executable version")?;
        // Refuse an unverified binary before asking it to generate a schema.
        if !maitty_provider_codex::is_verified_version(version) {
            return Err(anyhow::anyhow!(
                "Unsupported Codex executable version; select a verified Codex {}",
                maitty_provider_codex::VERIFIED_VERSIONS.join(" or ")
            )
            .into());
        }
        // A passed check holds for this path and version, so the slow schema
        // generation is not repeated for every conversation. A changed version
        // is a different key and is checked in full.
        let key = (self.paths().1.to_path_buf(), version.to_owned());
        if preflight_already_passed(&key) {
            directory.remove().await?;
            return Ok(());
        }
        let mut generate = self.command(&home);
        generate
            .args(["app-server", "generate-json-schema", "--out"])
            .arg(&schema);
        self.run_preflight(generate).await?;
        let inventory = tokio::task::spawn_blocking(move || -> Result<String> {
            let read = |name: &str| -> Result<Value> {
                let path = schema.join(name);
                if std::fs::metadata(&path).context("Inspect generated Codex schema")?.len() > MAX_PREFLIGHT_BYTES as u64 {
                    bail!("Generated Codex schema exceeds size limit");
                }
                let bytes = std::fs::read(path).context("Read generated Codex schema")?;
                serde_json::from_slice(&bytes).map_err(|_| anyhow::anyhow!("Invalid generated Codex schema"))
            };
            Ok(json!({"clientRequests": read("ClientRequest.json")?, "serverRequests": read("ServerRequest.json")?}).to_string())
        }).await.context("Read Codex schema in background")??;
        maitty_provider_codex::check_compatibility(version, &inventory)
            .map_err(anyhow::Error::new)?;
        directory.remove().await?;
        remember_preflight_passed(key);
        Ok(())
    }
}

type PreflightKey = (PathBuf, String);

fn passed_preflights() -> &'static std::sync::Mutex<std::collections::HashSet<PreflightKey>> {
    static PASSED: std::sync::OnceLock<std::sync::Mutex<std::collections::HashSet<PreflightKey>>> =
        std::sync::OnceLock::new();
    PASSED.get_or_init(Default::default)
}

fn preflight_already_passed(key: &PreflightKey) -> bool {
    passed_preflights()
        .lock()
        .is_ok_and(|passed| passed.contains(key))
}

fn remember_preflight_passed(key: PreflightKey) {
    // A poisoned lock only means the next start repeats the full check.
    if let Ok(mut passed) = passed_preflights().lock() {
        passed.insert(key);
    }
}

#[cfg(test)]
mod preflight_cache_tests {
    use super::*;

    #[test]
    fn a_passed_binary_and_version_is_remembered_but_a_new_version_is_not() {
        let path = PathBuf::from("codex-for-cache-test");
        assert!(!preflight_already_passed(&(path.clone(), "9.9.9".into())));
        remember_preflight_passed((path.clone(), "9.9.9".into()));
        assert!(preflight_already_passed(&(path.clone(), "9.9.9".into())));
        assert!(!preflight_already_passed(&(path, "9.9.10".into())));
    }
}

struct PreflightDirectory(PathBuf);
impl PreflightDirectory {
    async fn create() -> Result<Self> {
        tokio::task::spawn_blocking(|| -> Result<Self> {
            let mut entropy = [0_u8; 16];
            getrandom::fill(&mut entropy)
                .map_err(|error| anyhow::anyhow!("Allocate Codex preflight identity: {error}"))?;
            let suffix: String = entropy.iter().map(|n| format!("{n:02x}")).collect();
            let path = std::env::temp_dir().join(format!(
                "{}-codex-preflight-{suffix}",
                crate::brand::APP_COMMAND
            ));
            std::fs::create_dir(&path).context("Create isolated Codex preflight folder")?;
            let directory = Self(path);
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&directory.0, std::fs::Permissions::from_mode(0o700))?;
            }
            std::fs::create_dir(directory.0.join("home"))?;
            Ok(directory)
        })
        .await
        .context("Prepare Codex preflight in background")?
    }

    async fn remove(self) -> Result<()> {
        let path = self.0.clone();
        tokio::task::spawn_blocking(move || std::fs::remove_dir_all(path))
            .await
            .context("Clean up Codex preflight in background")?
            .context("Clean up isolated Codex preflight")
    }
}
impl Drop for PreflightDirectory {
    fn drop(&mut self) {
        let path = self.0.clone();
        // Cancellation must not leak isolated generated files or block a daemon loop.
        tokio::task::spawn_blocking(move || {
            if let Err(error) = std::fs::remove_dir_all(path)
                && error.kind() != std::io::ErrorKind::NotFound
            {
                tracing::warn!(%error, "Cannot clean Codex preflight folder");
            }
        });
    }
}

async fn discard_stderr(mut stderr: impl AsyncRead + Unpin) -> std::io::Result<()> {
    let mut chunk = [0_u8; STDERR_CHUNK_BYTES];
    while stderr.read(&mut chunk).await? != 0 {}
    Ok(())
}

impl Launch {
    async fn run_preflight(&self, command: Command) -> Result<Vec<u8>, StartupFailure> {
        let mut process = OwnedChild::spawn(command)
            .await
            .map_err(|error| StartupFailure::spawn(error, "Start Codex compatibility preflight"))?;
        #[cfg(test)]
        if self.failure_point == Some(FailurePoint::PreflightDisposal) {
            process.fail_next_stop();
        }
        process.child.stdin.take();
        let (Some(stdout), Some(stderr)) =
            (process.child.stdout.take(), process.child.stderr.take())
        else {
            return Err(StartupFailure::dispose(
                process,
                anyhow::anyhow!("Codex preflight pipe missing"),
            )
            .await);
        };
        let run = async {
            let mut bytes = Vec::new();
            let read = async {
                stdout
                    .take((MAX_PREFLIGHT_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
                    .await?;
                if bytes.len() > MAX_PREFLIGHT_BYTES {
                    bail!("Codex preflight output exceeds size limit");
                }
                Ok::<_, anyhow::Error>(())
            };
            let ((), (), status) = tokio::try_join!(
                read,
                async {
                    discard_stderr(stderr)
                        .await
                        .context("Drain preflight stderr")
                },
                async {
                    process
                        .child
                        .wait()
                        .await
                        .context("Wait for Codex preflight")
                }
            )?;
            if !status.success() {
                bail!("Codex compatibility preflight failed");
            }
            Ok(bytes)
        };
        let result = tokio::time::timeout(PREFLIGHT_TIMEOUT, run)
            .await
            .context("Codex compatibility preflight timed out")
            .and_then(|r| r);
        match process.stop().await {
            Ok(_) => result.map_err(StartupFailure::from),
            Err(cleanup) => Err(StartupFailure {
                error: result
                    .err()
                    .unwrap_or_else(|| anyhow::anyhow!("Codex compatibility preflight completed"))
                    .context(format!("Dispose Codex preflight process tree: {cleanup}")),
                process: Some(Box::new(process)),
                reaped: false,
            }),
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug, serde::Serialize, serde::Deserialize)]
#[serde(untagged)]
pub(super) enum RequestId {
    Integer(i64),
    Text(String),
}

const THREAD_NOT_MATERIALIZED_MESSAGE: &str =
    "thread is not materialized yet; thread/turns/list is unavailable before first user message";

#[derive(Debug)]
pub(in crate::daemon) struct RpcError {
    pub(super) code: i64,
    thread_not_materialized: bool,
}

impl RpcError {
    pub(super) fn adapter_failure(code: i64) -> Self {
        Self {
            code,
            thread_not_materialized: false,
        }
    }

    pub(in crate::daemon) fn is_thread_not_materialized(&self) -> bool {
        self.thread_not_materialized
    }
}

impl From<RequestId> for crate::conversation::ProviderRequestId {
    fn from(id: RequestId) -> Self {
        match id {
            RequestId::Integer(value) => Self::Integer(value),
            RequestId::Text(value) => Self::Text(value),
        }
    }
}

impl From<crate::conversation::ProviderRequestId> for RequestId {
    fn from(id: crate::conversation::ProviderRequestId) -> Self {
        match id {
            crate::conversation::ProviderRequestId::Integer(value) => Self::Integer(value),
            crate::conversation::ProviderRequestId::Text(value) => Self::Text(value),
        }
    }
}

pub(super) enum Envelope {
    Response {
        id: RequestId,
        result: Result<Value, RpcError>,
    },
    Notification {
        method: String,
        params: Value,
    },
    Request {
        id: RequestId,
        method: String,
        params: Value,
    },
}

struct EnvelopeObject(serde_json::Map<String, Value>);

impl<'de> serde::Deserialize<'de> for EnvelopeObject {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct ObjectVisitor;
        impl<'de> serde::de::Visitor<'de> for ObjectVisitor {
            type Value = EnvelopeObject;
            fn expecting(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                f.write_str("an unambiguous provider envelope")
            }
            fn visit_map<M: serde::de::MapAccess<'de>>(
                self,
                mut fields: M,
            ) -> Result<Self::Value, M::Error> {
                let mut object = serde_json::Map::new();
                while let Some((key, value)) = fields.next_entry::<String, Value>()? {
                    if object.insert(key, value).is_some() {
                        return Err(serde::de::Error::custom("duplicate envelope field"));
                    }
                }
                Ok(EnvelopeObject(object))
            }
        }
        deserializer.deserialize_map(ObjectVisitor)
    }
}

pub(super) fn decode(line: &str) -> Result<Envelope> {
    // Parse errors deliberately omit source text and serde's input-derived details.
    let EnvelopeObject(object) =
        serde_json::from_str(line).map_err(|_| anyhow::anyhow!("Malformed Codex JSON frame"))?;
    if object.get("jsonrpc").is_some_and(|v| v != "2.0") {
        bail!("Invalid Codex JSON-RPC version");
    }
    let id = object
        .get("id")
        .map(|value| {
            value
                .as_i64()
                .map(RequestId::Integer)
                .or_else(|| value.as_str().map(|s| RequestId::Text(s.into())))
                .context("Codex request ID must be an integer or string")
        })
        .transpose()?;
    let method = object
        .get("method")
        .map(|m| {
            m.as_str()
                .filter(|s| !s.is_empty() && !s.chars().any(char::is_control))
                .map(str::to_owned)
                .context("Invalid Codex method")
        })
        .transpose()?;
    if let Some(method) = method {
        if object.keys().any(|k| {
            !["jsonrpc", "id", "method", "params"].contains(&k.as_str())
                && !(k == "emittedAtMs" && id.is_none())
        }) {
            let keys: Vec<_> = object
                .keys()
                .take(8)
                .map(|key| {
                    key.chars()
                        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_')
                        .take(48)
                        .collect::<String>()
                })
                .collect();
            bail!(
                "Ambiguous Codex request envelope (fields: {})",
                keys.join(", ")
            );
        }
        if object
            .get("emittedAtMs")
            .is_some_and(|value| !value.is_null() && value.as_i64().is_none())
        {
            bail!("Invalid Codex notification timestamp");
        }
        let params = object.get("params").cloned().unwrap_or(Value::Null);
        if !params.is_null() && !params.is_object() && !params.is_array() {
            bail!("Invalid Codex params");
        }
        return Ok(match id {
            Some(id) => Envelope::Request { id, method, params },
            None => Envelope::Notification { method, params },
        });
    }
    let id = id.context("Codex response has no request ID")?;
    if object
        .keys()
        .any(|k| !["jsonrpc", "id", "result", "error"].contains(&k.as_str()))
        || object.contains_key("result") == object.contains_key("error")
    {
        bail!("Ambiguous Codex response envelope");
    }
    let result = if let Some(result) = object.get("result") {
        Ok(result.clone())
    } else {
        let error = object["error"]
            .as_object()
            .context("Invalid Codex error response")?;
        if !error.get("code").is_some_and(|c| c.as_i64().is_some())
            || !error.get("message").is_some_and(Value::is_string)
            || error
                .keys()
                .any(|k| !["code", "message", "data"].contains(&k.as_str()))
        {
            bail!("Invalid Codex error response");
        }
        let message = error["message"]
            .as_str()
            .context("Invalid Codex error message")?;
        Err(RpcError {
            code: error["code"].as_i64().context("Invalid Codex error code")?,
            thread_not_materialized: message == THREAD_NOT_MATERIALIZED_MESSAGE,
        })
    };
    Ok(Envelope::Response { id, result })
}

pub(super) struct Connection {
    pub(super) process: OwnedChild,
    input: FramedWrite<tokio::process::ChildStdin, LinesCodec>,
    output: FramedRead<tokio::process::ChildStdout, LinesCodec>,
    stderr_task: Option<tokio::task::JoinHandle<std::io::Result<()>>>,
}

impl Connection {
    pub(super) async fn runtime_policy(
        &mut self,
        cwd: &Path,
    ) -> Result<(serde_json::Map<String, Value>, Vec<Envelope>)> {
        self.request(
            CONFIG_READ_ID,
            "config/read",
            json!({"cwd":cwd,"includeLayers":false}),
        )
        .await?;
        let mut notifications = Vec::new();
        loop {
            match self.next().await? {
                Envelope::Response {
                    id: RequestId::Integer(CONFIG_READ_ID),
                    result: Ok(response),
                } => {
                    return Ok((
                        maitty_provider_codex::runtime_policy_overrides(
                            &response,
                            &crate::ipc::STRUCTURED_ENV,
                            &crate::ipc::RUNTIME_ROUTE_ENV,
                        )?,
                        notifications,
                    ));
                }
                Envelope::Notification { method, params }
                    if notifications.len() < MAX_STARTUP_NOTIFICATIONS =>
                {
                    notifications.push(Envelope::Notification { method, params })
                }
                _ => bail!(
                    "Codex effective configuration response was rejected, mismatched or interrupted by an unsupported startup request"
                ),
            }
        }
    }
    pub(super) async fn request(&mut self, id: i64, method: &str, params: Value) -> Result<()> {
        tokio::time::timeout(
            HANDSHAKE_TIMEOUT,
            self.input
                .send(json!({"id": id, "method": method, "params": params}).to_string()),
        )
        .await
        .context("Codex request write timed out")?
        .context("Write Codex request")
    }

    pub(super) async fn respond(&mut self, id: &RequestId, response: Value) -> Result<()> {
        tokio::time::timeout(
            HANDSHAKE_TIMEOUT,
            self.input
                .send(json!({"id": id, "result": response}).to_string()),
        )
        .await
        .context("Codex response write timed out")?
        .context("Write Codex response")
    }
    pub(super) async fn handshake(&mut self, config_root: &Path) -> Result<()> {
        let request = json!({"id": INITIALIZE_ID, "method": "initialize", "params": {
            "clientInfo": {"name": crate::brand::APP_COMMAND, "version": env!("APP_VERSION")},
            "capabilities": {"experimentalApi": true}
        }});
        self.input
            .send(request.to_string())
            .await
            .context("Write Codex initialize")?;
        let result = match self.next().await? {
            Envelope::Response {
                id: RequestId::Integer(INITIALIZE_ID),
                result: Ok(value),
            } => value,
            Envelope::Response {
                id: RequestId::Integer(INITIALIZE_ID),
                result: Err(_),
            } => bail!("Codex rejected initialize"),
            _ => bail!("Codex initialize response does not match pending request"),
        };
        let home = result
            .get("codexHome")
            .and_then(Value::as_str)
            .context("Codex initialize response lacks codexHome")?;
        for field in ["platformFamily", "platformOs", "userAgent"] {
            if !result
                .get(field)
                .and_then(Value::as_str)
                .is_some_and(|s| !s.is_empty())
            {
                bail!("Codex initialize response lacks required platform identity");
            }
        }
        let actual = PathBuf::from(home);
        if !actual.is_absolute() {
            bail!("Codex initialized with a relative configuration root");
        }
        let expected = config_root.to_path_buf();
        let matches = tokio::task::spawn_blocking(move || -> Result<bool> {
            Ok(
                std::fs::canonicalize(actual).context("Resolve initialized Codex home")?
                    == std::fs::canonicalize(expected).context("Resolve expected Codex home")?,
            )
        })
        .await
        .context("Validate Codex home in background")??;
        if !matches {
            bail!("Codex initialized with a different configuration root");
        }
        self.input
            .send(json!({"method": "initialized"}).to_string())
            .await
            .context("Write Codex initialized")?;
        Ok(())
    }

    pub(super) async fn next(&mut self) -> Result<Envelope> {
        // A descendant can retain a pipe after its owning provider exits.
        let frame = tokio::select! {
            frame = self.output.next() => frame,
            status = self.process.child.wait() => {
                status.context("Wait for Codex app-server exit")?;
                bail!("Codex stdout closed: provider process exited");
            }
        };
        let line = frame
            .context("Codex stdout closed")?
            .map_err(|_| anyhow::anyhow!("Codex stdout frame invalid or exceeds size limit"))?;
        decode(&line)
    }

    pub(super) async fn stop(&mut self) -> Result<()> {
        self.process
            .stop()
            .await
            .context("Dispose Codex app-server process tree")?;
        if let Some(stderr_task) = self.stderr_task.take() {
            stderr_task
                .await
                .context("Join Codex stderr drain")?
                .context("Drain Codex stderr")?;
        }
        Ok(())
    }
}

impl Drop for Connection {
    fn drop(&mut self) {
        if let Some(stderr_task) = &self.stderr_task {
            stderr_task.abort();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn claude_launch_validates_existing_paths() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

        struct TestDirectory(PathBuf);

        impl Drop for TestDirectory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let directory = loop {
            let candidate = std::env::temp_dir().join(format!(
                "{}-claude-launch-{}-{}",
                crate::brand::APP_COMMAND,
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&candidate) {
                Ok(()) => break TestDirectory(candidate),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create test directory: {error}"),
            }
        };
        let config_root = directory.0.join("claude-config");
        let cwd = directory.0.join("project");
        std::fs::create_dir(&config_root).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        let executable = directory.0.join(if cfg!(windows) {
            "claude.exe"
        } else {
            "claude"
        });
        std::fs::write(&executable, b"fake claude executable").unwrap();
        let launch = Launch {
            identity: LaunchIdentity::Pending(PendingStart {
                id: crate::conversation::ConversationId(1),
                generation: 1,
                settings: LaunchSettings {
                    provider: crate::conversation::Provider::Claude,
                    executable,
                    config_root,
                    cwd,
                    model: None,
                    model_provider: None,
                    approval_policy: crate::conversation::ApprovalPolicy::OnRequest,
                    sandbox: crate::conversation::Sandbox::ReadOnly,
                },
                thread_id: None,
            }),
            environment: Vec::new(),
            failure_point: None,
        };

        let validated = launch.validate().await.unwrap();
        assert_eq!(validated.provider(), crate::conversation::Provider::Claude);
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn launch_validation_preserves_a_mise_shim_invocation_path() {
        use crate::conversation::{ApprovalPolicy, ConversationId, Sandbox};
        use std::os::unix::fs::symlink;
        use std::sync::atomic::{AtomicUsize, Ordering};

        static NEXT_DIRECTORY: AtomicUsize = AtomicUsize::new(0);

        struct TestDirectory(PathBuf);

        impl Drop for TestDirectory {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.0);
            }
        }

        let directory = loop {
            let candidate = std::env::temp_dir().join(format!(
                "{}-codex-mise-launch-{}-{}",
                crate::brand::APP_COMMAND,
                std::process::id(),
                NEXT_DIRECTORY.fetch_add(1, Ordering::Relaxed)
            ));
            match std::fs::create_dir(&candidate) {
                Ok(()) => break TestDirectory(candidate),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => panic!("create test directory: {error}"),
            }
        };
        let config_root = directory.0.join("codex-home");
        let cwd = directory.0.join("project");
        std::fs::create_dir(&config_root).unwrap();
        std::fs::create_dir(&cwd).unwrap();
        let codex = directory.0.join("codex");
        symlink(std::env::current_exe().unwrap(), &codex).unwrap();

        let launch = Launch {
            identity: LaunchIdentity::Pending(PendingStart {
                id: ConversationId(1),
                generation: 1,
                settings: LaunchSettings {
                    provider: crate::conversation::Provider::Codex,
                    executable: codex.clone(),
                    config_root,
                    cwd,
                    model: None,
                    model_provider: None,
                    approval_policy: ApprovalPolicy::OnRequest,
                    sandbox: Sandbox::ReadOnly,
                },
                thread_id: None,
            }),
            environment: Vec::new(),
            failure_point: None,
        };

        let validated = launch.validate().await.unwrap();

        assert_eq!(validated.paths().1, std::path::absolute(&codex).unwrap());
    }

    #[test]
    fn identifies_only_codexs_unmaterialized_thread_error() {
        let response = decode(&format!(
            r#"{{"id":2,"error":{{"code":-32600,"message":"{THREAD_NOT_MATERIALIZED_MESSAGE}"}}}}"#
        ))
        .unwrap();
        assert!(matches!(
            response,
            Envelope::Response {
                result: Err(error),
                ..
            } if error.code == -32600 && error.is_thread_not_materialized()
        ));

        let response =
            decode(r#"{"id":2,"error":{"code":-32600,"message":"invalid request"}}"#).unwrap();
        assert!(matches!(
            response,
            Envelope::Response {
                result: Err(error),
                ..
            } if error.code == -32600 && !error.is_thread_not_materialized()
        ));
    }
}
