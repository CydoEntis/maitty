#[cfg(unix)]
use anyhow::Context;
use anyhow::Result;
use futures_util::{SinkExt, StreamExt};
use serde::{Serialize, de::DeserializeOwned};
use tokio::io::{AsyncRead, AsyncWrite};
use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

pub const MAX_FRAME_BYTES: usize = 64 * 1024 * 1024;

pub fn socket_id(
    app_command: &str,
    socket_label: Option<&str>,
    username: Option<&str>,
    user: Option<&str>,
) -> String {
    let label = socket_label.unwrap_or("default");
    let user = username.or(user).unwrap_or("user");
    format!("{app_command}-{user}-{label}.sock")
}

pub fn socket_name(
    id: &str,
    unix_directory: Option<&std::path::Path>,
) -> Result<interprocess::local_socket::Name<'static>> {
    #[cfg(windows)]
    {
        use interprocess::local_socket::{GenericNamespaced, ToNsName};
        let _ = unix_directory;
        Ok(id.to_ns_name::<GenericNamespaced>()?.into_owned())
    }
    #[cfg(unix)]
    {
        use interprocess::local_socket::{GenericFilePath, ToFsName};
        let directory = unix_directory.context("Unix socket directory is required")?;
        Ok(directory
            .join(id)
            .to_fs_name::<GenericFilePath>()?
            .into_owned())
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (id, unix_directory);
        anyhow::bail!("local sockets are unsupported on this platform")
    }
}

#[cfg(unix)]
pub fn private_socket_dir(
    app_command: &str,
    recovery_instruction: &str,
    runtime_dir: Option<&std::path::Path>,
    temp_dir: &std::path::Path,
) -> anyhow::Result<std::path::PathBuf> {
    use std::os::unix::fs::{DirBuilderExt, MetadataExt};

    // SAFETY: getuid has no preconditions and can't fail.
    let uid = unsafe { libc::getuid() };
    let base = runtime_dir.filter(|path| path.is_dir()).unwrap_or(temp_dir);
    let directory = base.join(format!("{app_command}-{uid}"));
    match std::fs::DirBuilder::new().mode(0o700).create(&directory) {
        Ok(()) => {}
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(error) => {
            return Err(error).with_context(|| format!("creating {}", directory.display()));
        }
    }
    let metadata = std::fs::symlink_metadata(&directory)
        .with_context(|| format!("checking {}", directory.display()))?;
    if !metadata.is_dir() || metadata.uid() != uid || metadata.mode() & 0o077 != 0 {
        anyhow::bail!(
            "{} isn't a private folder of yours; remove it and {recovery_instruction}",
            directory.display()
        );
    }
    Ok(directory)
}

pub type Reader = FramedRead<Box<dyn AsyncRead + Send + Unpin>, LengthDelimitedCodec>;
pub type Writer = FramedWrite<Box<dyn AsyncWrite + Send + Unpin>, LengthDelimitedCodec>;

pub async fn welcome_version(reader: &mut Reader) -> Result<u32> {
    use maitty_shared::protocol::ServerMsg;

    match receive(reader).await? {
        Some(ServerMsg::Welcome { version }) => Ok(version),
        Some(ServerMsg::Error(error)) => anyhow::bail!(error),
        _ => anyhow::bail!("unexpected handshake from daemon"),
    }
}

pub fn codec() -> LengthDelimitedCodec {
    LengthDelimitedCodec::builder()
        .max_frame_length(MAX_FRAME_BYTES)
        .new_codec()
}

pub fn reader(stream: impl AsyncRead + Send + Unpin + 'static) -> Reader {
    FramedRead::new(Box::new(stream), codec())
}

pub fn writer(stream: impl AsyncWrite + Send + Unpin + 'static) -> Writer {
    FramedWrite::new(Box::new(stream), codec())
}

pub fn framed_parts(
    read: impl AsyncRead + Send + Unpin + 'static,
    write: impl AsyncWrite + Send + Unpin + 'static,
) -> (Reader, Writer) {
    (reader(read), writer(write))
}

pub async fn send<T: Serialize>(writer: &mut Writer, message: &T) -> Result<()> {
    writer
        .send(maitty_shared::protocol::encode(message)?)
        .await?;
    Ok(())
}

pub async fn receive<T: DeserializeOwned>(reader: &mut Reader) -> Result<Option<T>> {
    match reader.next().await {
        Some(frame) => Ok(Some(maitty_shared::protocol::decode(&frame?)?)),
        None => Ok(None),
    }
}

pub fn is_decode_error(error: &anyhow::Error) -> bool {
    error.downcast_ref::<rmp_serde::decode::Error>().is_some()
}

#[cfg(test)]
mod tests {
    #[cfg(unix)]
    use super::private_socket_dir;
    use super::{framed_parts, receive, send, socket_id, welcome_version};
    use maitty_shared::protocol::{ClientMsg, PROTOCOL_VERSION, ServerMsg, Surface};

    #[tokio::test]
    async fn local_clients_exchange_protocol_messages_in_length_delimited_frames() {
        let (left, right) = tokio::io::duplex(1024);
        let (left_read, left_write) = tokio::io::split(left);
        let (right_read, right_write) = tokio::io::split(right);
        let (_left_reader, mut left_writer) = framed_parts(left_read, left_write);
        let (mut right_reader, _right_writer) = framed_parts(right_read, right_write);
        let message = ClientMsg::Hello {
            version: PROTOCOL_VERSION,
            attach: false,
            from: Some((7, "pane-secret".into())),
            structured: None,
            surface: Surface::Terminal,
        };

        send(&mut left_writer, &message).await.unwrap();
        let received: Option<ClientMsg> = receive(&mut right_reader).await.unwrap();
        assert!(matches!(
            received,
            Some(ClientMsg::Hello {
                version: PROTOCOL_VERSION,
                attach: false,
                from: Some((7, token)),
                structured: None,
                surface: Surface::Terminal,
            }) if token == "pane-secret"
        ));
    }

    #[tokio::test]
    async fn handshake_reads_welcome_version_and_preserves_mismatch_for_the_caller() {
        let (client, server) = tokio::io::duplex(1024);
        let (client_read, client_write) = tokio::io::split(client);
        let (server_read, server_write) = tokio::io::split(server);
        let (mut client_reader, _client_writer) = framed_parts(client_read, client_write);
        let (_server_reader, mut server_writer) = framed_parts(server_read, server_write);

        send(
            &mut server_writer,
            &ServerMsg::Welcome {
                version: PROTOCOL_VERSION + 1,
            },
        )
        .await
        .unwrap();

        assert_eq!(
            welcome_version(&mut client_reader).await.unwrap(),
            PROTOCOL_VERSION + 1
        );
    }

    #[tokio::test]
    async fn handshake_returns_daemon_errors_and_rejects_unexpected_messages() {
        for (message, expected_error) in [
            (
                ServerMsg::Error("upgrade required".into()),
                "upgrade required",
            ),
            (
                ServerMsg::Notice("not a handshake".into()),
                "unexpected handshake from daemon",
            ),
        ] {
            let (client, server) = tokio::io::duplex(1024);
            let (client_read, client_write) = tokio::io::split(client);
            let (server_read, server_write) = tokio::io::split(server);
            let (mut client_reader, _client_writer) = framed_parts(client_read, client_write);
            let (_server_reader, mut server_writer) = framed_parts(server_read, server_write);

            send(&mut server_writer, &message).await.unwrap();

            let error = welcome_version(&mut client_reader).await.unwrap_err();
            assert!(error.to_string().contains(expected_error));
        }
    }

    #[test]
    fn socket_identity_uses_the_windows_user_first_and_default_values() {
        assert_eq!(
            socket_id("maitty", Some("work"), Some("cst"), Some("fallback")),
            "maitty-cst-work.sock"
        );
        assert_eq!(
            socket_id("maitty", None, None, Some("linux-user")),
            "maitty-linux-user-default.sock"
        );
        assert_eq!(
            socket_id("maitty", None, None, None),
            "maitty-user-default.sock"
        );
    }

    #[test]
    fn socket_name_uses_the_platform_namespace() {
        #[cfg(unix)]
        {
            assert!(super::socket_name("maitty-test.sock", None).is_err());
            let directory = std::env::temp_dir();
            assert!(super::socket_name("maitty-test.sock", Some(&directory)).is_ok());
        }
        #[cfg(windows)]
        assert!(super::socket_name("maitty-test", None).is_ok());
    }

    #[cfg(unix)]
    #[test]
    fn private_socket_directory_is_created_with_owner_only_permissions() {
        use std::os::unix::fs::PermissionsExt;

        let root = std::env::temp_dir().join(format!(
            "maitty-ipc-transport-test-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir(&root).unwrap();
        let result =
            private_socket_dir("maitty-test", "start maitty-test again", Some(&root), &root)
                .unwrap();
        let permissions = std::fs::metadata(&result).unwrap().permissions();
        assert_eq!(permissions.mode() & 0o777, 0o700);
        std::fs::remove_dir(&result).unwrap();
        std::fs::remove_dir(root).unwrap();
    }
}
