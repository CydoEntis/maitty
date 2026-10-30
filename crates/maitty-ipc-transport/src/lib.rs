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
    use super::{framed_parts, receive, send, socket_id, welcome_version};
    use maitty_shared::protocol::{ClientMsg, PROTOCOL_VERSION, ServerMsg};

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
}
