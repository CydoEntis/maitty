use anyhow::{Context, Result};
use interprocess::local_socket::Name;
use interprocess::local_socket::tokio::{Stream, prelude::*};
#[cfg(unix)]
use interprocess::local_socket::{GenericFilePath, ToFsName};
#[cfg(windows)]
use interprocess::local_socket::{GenericNamespaced, ToNsName};
pub use maitty_ipc_transport::{Reader, Writer};
use maitty_ipc_transport::{framed_parts, receive, send};
use maitty_shared::protocol::{self, ClientMsg, ServerMsg};

const APP_COMMAND: &str = env!("MAITTY_APP_COMMAND");
const APP_ENV: &str = env!("MAITTY_APP_ENV");

#[derive(Debug)]
pub struct OtherVersion {
    pub daemon: u32,
}

impl std::fmt::Display for OtherVersion {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "the running {APP_COMMAND} service uses protocol v{}, while this GUI needs v{}",
            self.daemon,
            protocol::PROTOCOL_VERSION
        )
    }
}

impl std::error::Error for OtherVersion {}

pub fn other_version(error: &anyhow::Error) -> Option<u32> {
    error
        .chain()
        .find_map(|cause| cause.downcast_ref::<OtherVersion>())
        .map(|version| version.daemon)
}

pub fn pid_file() -> std::path::PathBuf {
    data_dir().join(format!("{}.pid", socket_id()))
}

pub fn remove_pid_file() {
    let _ = std::fs::remove_file(pid_file());
}

pub fn data_dir() -> std::path::PathBuf {
    if let Some(path) = std::env::var_os(format!("{APP_ENV}_DATA_DIR")) {
        return path.into();
    }
    directories::ProjectDirs::from("", "", APP_COMMAND)
        .map(|dirs| dirs.data_local_dir().to_path_buf())
        .unwrap_or_else(std::env::temp_dir)
}

fn socket_id() -> String {
    let label = std::env::var(format!("{APP_ENV}_SOCKET")).ok();
    let username = std::env::var("USERNAME").ok();
    let user = std::env::var("USER").ok();
    maitty_ipc_transport::socket_id(
        APP_COMMAND,
        label.as_deref(),
        username.as_deref(),
        user.as_deref(),
    )
}

pub async fn is_running() -> bool {
    connect().await.is_ok()
}

fn name() -> Result<Name<'static>> {
    let id = socket_id();
    #[cfg(windows)]
    {
        Ok(id.to_ns_name::<GenericNamespaced>()?.into_owned())
    }
    #[cfg(unix)]
    {
        Ok(socket_dir()?
            .join(id)
            .to_fs_name::<GenericFilePath>()?
            .into_owned())
    }
}

#[cfg(unix)]
fn socket_dir() -> Result<std::path::PathBuf> {
    let runtime_dir = std::env::var_os("XDG_RUNTIME_DIR").map(std::path::PathBuf::from);
    maitty_ipc_transport::private_socket_dir(
        APP_COMMAND,
        &format!("restart {APP_COMMAND}"),
        runtime_dir.as_deref(),
        &std::env::temp_dir(),
    )
}

async fn connect() -> Result<Stream> {
    Ok(Stream::connect(name()?).await?)
}

fn framed(stream: Stream) -> (Reader, Writer) {
    let (read, write) = stream.split();
    framed_parts(read, write)
}

pub async fn receive_server(reader: &mut Reader) -> Result<Option<ServerMsg>> {
    receive(reader).await
}

pub async fn open_for_user() -> Result<(Reader, Writer)> {
    let (mut reader, mut writer) = framed(connect().await?);
    send(
        &mut writer,
        &ClientMsg::Hello {
            version: protocol::PROTOCOL_VERSION,
            attach: false,
            from: None,
            structured: None,
        },
    )
    .await?;
    let version = maitty_ipc_transport::welcome_version(&mut reader).await?;
    if version == protocol::PROTOCOL_VERSION {
        Ok((reader, writer))
    } else {
        Err(OtherVersion { daemon: version }.into())
    }
}

pub async fn open_or_spawn_for_user(
    daemon_executable: &std::path::Path,
) -> Result<(Reader, Writer)> {
    match open_for_user().await {
        Ok(connection) => return Ok(connection),
        Err(error) if other_version(&error).is_some() => return Err(error),
        Err(_) => {}
    }
    spawn_daemon(daemon_executable)?;
    let mut last_error = None;
    for _ in 0..60 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        match open_for_user().await {
            Ok(connection) => return Ok(connection),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| anyhow::anyhow!("daemon did not start")))
}

fn spawn_daemon(executable: &std::path::Path) -> Result<()> {
    daemon_command(executable)
        .spawn()
        .context("starting the daemon")?;
    Ok(())
}

fn daemon_command(executable: &std::path::Path) -> std::process::Command {
    let mut command = std::process::Command::new(executable);
    command
        .arg("daemon")
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x0800_0000);
    }
    for key in [
        format!("{APP_ENV}_CONVERSATION_ID"),
        format!("{APP_ENV}_CONVERSATION_GENERATION"),
        format!("{APP_ENV}_CONVERSATION_CONNECTION"),
        format!("{APP_ENV}_CONVERSATION_AUTH"),
        format!("{APP_ENV}_TERM_ID"),
        format!("{APP_ENV}_PANE_TOKEN"),
        format!("{APP_ENV}_REMOTE"),
        format!("{APP_ENV}_SSH"),
        format!("{APP_ENV}_REMOTE_CMD"),
    ] {
        command.env_remove(key);
    }
    command
}

pub async fn send_message(writer: &mut Writer, message: &ClientMsg) -> Result<()> {
    send(writer, message).await
}

#[cfg(test)]
mod tests {
    use super::daemon_command;

    #[test]
    fn gui_daemon_drops_pane_and_remote_identity_but_keeps_user_configuration() {
        let command = daemon_command(std::path::Path::new("maitty"));
        let removed = command
            .get_envs()
            .filter(|(_, value)| value.is_none())
            .map(|(key, _)| key.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        for key in [
            concat!(env!("MAITTY_APP_ENV"), "_CONVERSATION_ID"),
            concat!(env!("MAITTY_APP_ENV"), "_CONVERSATION_GENERATION"),
            concat!(env!("MAITTY_APP_ENV"), "_CONVERSATION_CONNECTION"),
            concat!(env!("MAITTY_APP_ENV"), "_CONVERSATION_AUTH"),
            concat!(env!("MAITTY_APP_ENV"), "_TERM_ID"),
            concat!(env!("MAITTY_APP_ENV"), "_PANE_TOKEN"),
            concat!(env!("MAITTY_APP_ENV"), "_REMOTE"),
            concat!(env!("MAITTY_APP_ENV"), "_SSH"),
            concat!(env!("MAITTY_APP_ENV"), "_REMOTE_CMD"),
        ] {
            assert!(removed.iter().any(|removed| removed == key), "{key}");
        }
        for key in [
            concat!(env!("MAITTY_APP_ENV"), "_CONFIG"),
            concat!(env!("MAITTY_APP_ENV"), "_DATA_DIR"),
            concat!(env!("MAITTY_APP_ENV"), "_SOCKET"),
        ] {
            assert!(!removed.iter().any(|removed| removed == key), "{key}");
        }
    }
}
