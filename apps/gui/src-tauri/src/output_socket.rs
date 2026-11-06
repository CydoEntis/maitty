//! Terminal output on a socket the web view reads itself (ADR 0057).
//!
//! A Tauri channel is delivered through the host's main loop, and on Linux the web view did not
//! run a message the host had sent until the window next got an event: every pane ran one step
//! behind (ticket 35). A WebSocket on the loopback address is read by the web process's own
//! network stack, so it does not wait for either. The frames are the ones the channel carries.
//!
//! Only this computer can connect (loopback), only with a token the window asked the host for
//! (one use, short-lived), and only from the app's own origin. Until a window has connected and
//! proved the token, frames go down the channel as before, so a socket that cannot start costs
//! speed and nothing else.

use serde::Serialize;
use std::collections::VecDeque;
use std::net::{Ipv4Addr, SocketAddr, TcpListener, TcpStream};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{Receiver, SyncSender, TrySendError, sync_channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tungstenite::handshake::server::{Callback, ErrorResponse, Request, Response};
use tungstenite::http::StatusCode;
use tungstenite::{Message, accept_hdr};

/// How long a token stays good after the window is given it.
const TOKEN_LIFETIME: Duration = Duration::from_secs(60);
/// Tokens handed out and not yet used; the oldest goes first.
const MAX_TOKENS: usize = 8;
/// A client has this long to finish the handshake and send its token.
const AUTH_TIMEOUT: Duration = Duration::from_secs(5);
/// Connections that have not yet proved a token. More are refused: a flood of silent
/// connections must not hold a thread each.
const MAX_UNPROVEN: usize = 4;
/// Frames waiting for the window. A window that stops reading for this long loses the socket
/// and the pane falls back to the channel, then asks for a replay.
const QUEUE_FRAMES: usize = 4096;
/// The word the host sends once the token is proved; the window then trusts the socket.
pub const READY: &str = "ready";
/// The token's length in bytes before it is written as hex.
const TOKEN_BYTES: usize = 32;
/// Origins the app's own web view loads from (Tauri's schemes, and the dev server).
const ALLOWED_ORIGINS: &[&str] = &[
    "tauri://localhost",
    "http://tauri.localhost",
    "https://tauri.localhost",
];

/// What the window needs to connect.
#[derive(Serialize, Debug, PartialEq)]
pub struct SocketGrant {
    pub port: u16,
    pub token: String,
}

struct Inner {
    port: u16,
    tokens: Mutex<VecDeque<(String, Instant)>>,
    /// The connected window's queue of frames; none until one has proved a token.
    sink: Mutex<Option<SyncSender<Vec<u8>>>>,
    unproven: AtomicUsize,
    /// How many frames may wait for the window.
    queue_frames: usize,
    /// An extra origin to accept, for a development server.
    dev_origin: Option<String>,
}

/// The listener, or nothing when it could not start.
pub struct OutputSocket(Option<Arc<Inner>>);

impl OutputSocket {
    /// Listen on the loopback address on a port the system picks.
    pub fn start(dev_origin: Option<String>) -> OutputSocket {
        Self::start_sized(dev_origin, QUEUE_FRAMES)
    }

    fn start_sized(dev_origin: Option<String>, queue_frames: usize) -> OutputSocket {
        let listener = match TcpListener::bind(SocketAddr::from((Ipv4Addr::LOCALHOST, 0))) {
            Ok(listener) => listener,
            Err(error) => {
                tracing::warn!("the terminal output socket could not start: {error}");
                return OutputSocket(None);
            }
        };
        let Ok(address) = listener.local_addr() else {
            return OutputSocket(None);
        };
        let inner = Arc::new(Inner {
            port: address.port(),
            tokens: Mutex::new(VecDeque::new()),
            sink: Mutex::new(None),
            unproven: AtomicUsize::new(0),
            queue_frames,
            dev_origin,
        });
        let shared = Arc::clone(&inner);
        std::thread::Builder::new()
            .name("terminal-output-socket".into())
            .spawn(move || accept_loop(&listener, &shared))
            .map_or_else(
                |error| {
                    tracing::warn!("the terminal output socket could not start: {error}");
                    OutputSocket(None)
                },
                |_| OutputSocket(Some(inner)),
            )
    }

    /// A token for one connection, or None when the socket is not running.
    pub fn grant(&self) -> Option<SocketGrant> {
        let inner = self.0.as_ref()?;
        let mut bytes = [0u8; TOKEN_BYTES];
        getrandom::fill(&mut bytes).ok()?;
        let token = bytes.iter().map(|byte| format!("{byte:02x}")).collect::<String>();
        let mut tokens = inner.tokens.lock().ok()?;
        while tokens.len() >= MAX_TOKENS {
            tokens.pop_front();
        }
        tokens.push_back((token.clone(), Instant::now()));
        Some(SocketGrant {
            port: inner.port,
            token,
        })
    }

    /// Hand a frame to the connected window. Gives the frame back when there is none, or when
    /// the one there has stopped reading, so the caller can use the channel.
    pub fn send(&self, frame: Vec<u8>) -> Result<(), Vec<u8>> {
        let Some(inner) = &self.0 else {
            return Err(frame);
        };
        let Ok(mut sink) = inner.sink.lock() else {
            return Err(frame);
        };
        let Some(queue) = sink.as_ref() else {
            return Err(frame);
        };
        match queue.try_send(frame) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(frame) | TrySendError::Disconnected(frame)) => {
                *sink = None;
                Err(frame)
            }
        }
    }
}

/// The window asks for a way onto the socket. It connects, sends the token, and waits for
/// `ready` before it trusts the socket; until then frames keep coming down the channel.
#[tauri::command]
pub fn open_terminal_socket(
    socket: tauri::State<'_, OutputSocket>,
) -> Result<SocketGrant, String> {
    socket
        .grant()
        .ok_or_else(|| "The terminal output socket is not running".to_string())
}

fn accept_loop(listener: &TcpListener, inner: &Arc<Inner>) {
    for stream in listener.incoming() {
        let Ok(stream) = stream else { continue };
        if inner.unproven.fetch_add(1, Ordering::SeqCst) >= MAX_UNPROVEN {
            inner.unproven.fetch_sub(1, Ordering::SeqCst);
            continue;
        }
        let inner = Arc::clone(inner);
        let spawned = std::thread::Builder::new()
            .name("terminal-output-client".into())
            .spawn({
                let inner = Arc::clone(&inner);
                move || serve(stream, &inner)
            });
        if spawned.is_err() {
            inner.unproven.fetch_sub(1, Ordering::SeqCst);
        }
    }
}

/// One connection: prove the token, then write frames until the window goes.
fn serve(stream: TcpStream, inner: &Arc<Inner>) {
    let outcome = handshake(stream, inner);
    inner.unproven.fetch_sub(1, Ordering::SeqCst);
    let Some((mut socket, frames)) = outcome else {
        return;
    };
    while let Ok(frame) = frames.recv() {
        if socket.send(Message::Binary(frame.into())).is_err() {
            break;
        }
    }
    // Whoever replaced this connection has its own queue; this one only ends.
    let _ = socket.close(None);
}

type Client = tungstenite::WebSocket<TcpStream>;

/// Accept the WebSocket upgrade from an allowed origin, read the token, and on a good one make
/// this connection the window's.
fn handshake(stream: TcpStream, inner: &Arc<Inner>) -> Option<(Client, Receiver<Vec<u8>>)> {
    stream.set_read_timeout(Some(AUTH_TIMEOUT)).ok()?;
    stream.set_write_timeout(Some(AUTH_TIMEOUT)).ok()?;
    stream.set_nodelay(true).ok()?;
    let mut socket = accept_hdr(
        stream,
        OriginCheck {
            dev_origin: inner.dev_origin.as_deref(),
        },
    )
    .ok()?;
    let Ok(Message::Text(token)) = socket.read() else {
        return None;
    };
    if !take_token(&inner.tokens, token.as_str(), Instant::now()) {
        return None;
    }
    // From here the window decides when it stops reading; writes may take as long as it does.
    socket.get_ref().set_write_timeout(None).ok()?;
    socket.get_ref().set_read_timeout(None).ok()?;
    let (queue, frames) = sync_channel(inner.queue_frames);
    socket.send(Message::text(READY)).ok()?;
    socket.flush().ok()?;
    // A window that reloads takes the place of the one before it.
    *inner.sink.lock().ok()? = Some(queue);
    Some((socket, frames))
}

/// Lets the upgrade through only from the app's own origin.
struct OriginCheck<'a> {
    dev_origin: Option<&'a str>,
}

impl Callback for OriginCheck<'_> {
    fn on_request(self, request: &Request, response: Response) -> Result<Response, ErrorResponse> {
        let origin = request
            .headers()
            .get("origin")
            .and_then(|value| value.to_str().ok());
        match origin {
            Some(origin) if origin_allowed(origin, self.dev_origin) => Ok(response),
            _ => {
                let mut refused = ErrorResponse::new(None);
                *refused.status_mut() = StatusCode::FORBIDDEN;
                Err(refused)
            }
        }
    }
}

fn origin_allowed(origin: &str, dev_origin: Option<&str>) -> bool {
    ALLOWED_ORIGINS.contains(&origin) || dev_origin == Some(origin)
}

/// Whether `offered` is a token that was issued, unused and not stale; it is spent either way.
fn take_token(tokens: &Mutex<VecDeque<(String, Instant)>>, offered: &str, now: Instant) -> bool {
    let Ok(mut tokens) = tokens.lock() else {
        return false;
    };
    tokens.retain(|(_, issued)| now.duration_since(*issued) < TOKEN_LIFETIME);
    // Every stored token is compared, so how long this takes does not say how much matched.
    let mut found = None;
    for (index, (token, _)) in tokens.iter().enumerate() {
        if same_bytes(token.as_bytes(), offered.as_bytes()) {
            found = Some(index);
        }
    }
    found.is_some_and(|index| tokens.remove(index).is_some())
}

fn same_bytes(left: &[u8], right: &[u8]) -> bool {
    left.len() == right.len()
        && left
            .iter()
            .zip(right)
            .fold(0u8, |difference, (a, b)| difference | (a ^ b))
            == 0
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::TcpStream as Tcp;
    use tungstenite::client::IntoClientRequest;

    fn connect(
        port: u16,
        origin: Option<&str>,
    ) -> Result<tungstenite::WebSocket<tungstenite::stream::MaybeTlsStream<Tcp>>, Box<tungstenite::Error>>
    {
        let mut request = format!("ws://127.0.0.1:{port}/").into_client_request().unwrap();
        if let Some(origin) = origin {
            request
                .headers_mut()
                .insert("origin", origin.parse().unwrap());
        }
        tungstenite::connect(request)
            .map(|(socket, _)| socket)
            .map_err(Box::new)
    }

    const APP: &str = "tauri://localhost";

    fn started() -> (OutputSocket, u16) {
        let socket = OutputSocket::start(None);
        let port = socket.0.as_ref().expect("the listener starts").port;
        (socket, port)
    }

    #[test]
    fn a_window_with_a_token_gets_the_frames_in_order() {
        let (socket, port) = started();
        let grant = socket.grant().unwrap();
        let mut window = connect(port, Some(APP)).unwrap();
        window.send(Message::text(grant.token)).unwrap();
        assert_eq!(window.read().unwrap(), Message::text(READY));

        for number in 0..50u8 {
            assert!(socket.send(vec![number; 3]).is_ok());
        }
        for number in 0..50u8 {
            assert_eq!(window.read().unwrap(), Message::Binary(vec![number; 3].into()));
        }
    }

    #[test]
    fn before_a_window_has_proved_a_token_frames_are_given_back_for_the_channel() {
        let (socket, _) = started();
        assert_eq!(socket.send(vec![1, 2, 3]), Err(vec![1, 2, 3]));
    }

    #[test]
    fn a_token_works_once() {
        let (socket, port) = started();
        let grant = socket.grant().unwrap();
        let mut first = connect(port, Some(APP)).unwrap();
        first.send(Message::text(grant.token.clone())).unwrap();
        assert_eq!(first.read().unwrap(), Message::text(READY));

        let mut second = connect(port, Some(APP)).unwrap();
        second.send(Message::text(grant.token)).unwrap();
        assert!(!matches!(second.read(), Ok(Message::Text(_))));
    }

    #[test]
    fn a_wrong_token_is_refused_and_frames_keep_using_the_channel() {
        let (socket, port) = started();
        socket.grant().unwrap();
        let mut window = connect(port, Some(APP)).unwrap();
        window.send(Message::text("0".repeat(TOKEN_BYTES * 2))).unwrap();
        assert!(!matches!(window.read(), Ok(Message::Text(_))));
        assert_eq!(socket.send(vec![9]), Err(vec![9]));
    }

    #[test]
    fn a_page_from_another_origin_cannot_connect_even_with_a_token() {
        let (socket, port) = started();
        let grant = socket.grant().unwrap();
        for origin in [Some("https://example.com"), Some("http://localhost:8080"), None] {
            let attempt = connect(port, origin);
            assert!(attempt.is_err(), "{origin:?} was let in");
        }
        // The token was never spent, so the app's own window still gets in.
        let mut window = connect(port, Some(APP)).unwrap();
        window.send(Message::text(grant.token)).unwrap();
        assert_eq!(window.read().unwrap(), Message::text(READY));
    }

    #[test]
    fn a_development_origin_is_accepted_only_when_asked_for() {
        assert!(origin_allowed("http://localhost:1420", Some("http://localhost:1420")));
        assert!(!origin_allowed("http://localhost:1420", None));
        assert!(!origin_allowed("http://localhost:1421", Some("http://localhost:1420")));
    }

    #[test]
    fn a_stale_token_is_not_good() {
        let tokens = Mutex::new(VecDeque::from([(
            "abc".to_string(),
            Instant::now() - TOKEN_LIFETIME - Duration::from_secs(1),
        )]));
        assert!(!take_token(&tokens, "abc", Instant::now()));
        let tokens = Mutex::new(VecDeque::from([("abc".to_string(), Instant::now())]));
        assert!(take_token(&tokens, "abc", Instant::now()));
        assert!(!take_token(&tokens, "abc", Instant::now()));
    }

    #[test]
    fn only_a_few_tokens_are_kept() {
        let (socket, _) = started();
        let grants = (0..MAX_TOKENS + 3)
            .map(|_| socket.grant().unwrap().token)
            .collect::<Vec<_>>();
        let kept = socket.0.as_ref().unwrap().tokens.lock().unwrap().len();
        assert_eq!(kept, MAX_TOKENS);
        let unique = grants.iter().collect::<std::collections::HashSet<_>>();
        assert_eq!(unique.len(), grants.len());
        assert!(grants.iter().all(|token| token.len() == TOKEN_BYTES * 2));
    }

    #[test]
    fn a_window_that_stops_reading_loses_the_socket_and_gets_the_channel_back() {
        let socket = OutputSocket::start_sized(None, 8);
        let port = socket.0.as_ref().unwrap().port;
        let grant = socket.grant().unwrap();
        let mut window = connect(port, Some(APP)).unwrap();
        window.send(Message::text(grant.token)).unwrap();
        assert_eq!(window.read().unwrap(), Message::text(READY));
        // Nothing reads from `window`, and each frame is large enough to fill the socket's buffers.
        let big = vec![0u8; 1 << 20];
        let mut given_back = false;
        for _ in 0..4096 {
            if socket.send(big.clone()).is_err() {
                given_back = true;
                break;
            }
        }
        assert!(given_back, "the queue never filled");
        assert_eq!(socket.send(vec![1]), Err(vec![1]));
    }
}
