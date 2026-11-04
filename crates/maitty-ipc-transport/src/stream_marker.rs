//! The marker a remote `proxy` writes to stdout before its first frame, and the reader that
//! skips whatever a login shell printed ahead of it. The marker sits outside the msgpack
//! frames, so the wire protocol is unchanged.

use std::io::Cursor;
use std::time::Duration;
use tokio::io::{AsyncRead, AsyncReadExt};

use crate::MAX_FRAME_BYTES;

/// A NUL and a record separator can't open a text banner, and no frame starts with them: a
/// frame's length begins with a zero byte, never `0x1e`, in its second byte.
pub const STREAM_MARKER: &[u8] = b"\0\x1emaitty-stream\x1e\n";

/// How much login text to skip, and how long to wait for the marker.
#[derive(Debug, Clone, Copy)]
pub struct Limits {
    pub cap: usize,
    pub wait: Duration,
}

/// The wait outlasts ssh's `ConnectTimeout`, so a slow login is not cut off; the cap is far
/// above any banner, but keeps a runaway profile script from filling memory.
pub const LIMITS: Limits = Limits {
    cap: 64 * 1024,
    wait: Duration::from_secs(30),
};

/// What `scan` makes of the bytes read so far.
#[derive(Debug, PartialEq, Eq)]
pub enum Scan {
    /// The marker ends at this offset; everything before it is login text.
    Found(usize),
    /// A frame begins at offset 0: a remote from before the marker answering the Hello.
    Frame,
    More,
    TooMuch,
}

pub fn scan(buf: &[u8], cap: usize) -> Scan {
    if let Some(at) = buf
        .windows(STREAM_MARKER.len())
        .position(|window| window == STREAM_MARKER)
    {
        return if at > cap {
            Scan::TooMuch
        } else {
            Scan::Found(at + STREAM_MARKER.len())
        };
    }
    if STREAM_MARKER.starts_with(buf) {
        return Scan::More;
    }
    if starts_a_frame(buf) {
        return Scan::Frame;
    }
    if buf.len() >= cap + STREAM_MARKER.len() {
        Scan::TooMuch
    } else {
        Scan::More
    }
}

/// Text never passes: its first four bytes read as a length above `MAX_FRAME_BYTES`.
fn starts_a_frame(buf: &[u8]) -> bool {
    let Some(head) = buf.first_chunk::<4>() else {
        return false;
    };
    (1..=MAX_FRAME_BYTES).contains(&(u32::from_be_bytes(*head) as usize))
}

/// Why no frame stream was reached.
#[derive(Debug)]
pub enum NoMarker {
    /// The stream ended, or the wait did, with nothing read.
    Silent,
    /// Text arrived and the marker never did.
    Text,
    Io(std::io::Error),
}

impl std::fmt::Display for NoMarker {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            NoMarker::Silent => f.write_str("nothing came from the remote"),
            NoMarker::Text => f.write_str("no stream marker arrived"),
            NoMarker::Io(e) => write!(f, "reading from the remote: {e}"),
        }
    }
}

impl std::error::Error for NoMarker {}

/// What follows the marker (bytes already read, then the rest of the stream).
pub type Resynced<R> = tokio::io::Chain<Cursor<Vec<u8>>, R>;

/// Reads `read` until the marker, dropping the login text before it, and returns the stream
/// from the first byte after. A stream that opens with a frame instead is returned whole.
pub async fn skip_login_noise<R: AsyncRead + Unpin>(
    mut read: R,
    limits: Limits,
) -> Result<Resynced<R>, NoMarker> {
    let deadline = tokio::time::Instant::now() + limits.wait;
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    loop {
        match scan(&buf, limits.cap) {
            Scan::Found(end) => {
                buf.drain(..end);
                break;
            }
            Scan::Frame => break,
            Scan::TooMuch => return Err(NoMarker::Text),
            Scan::More => {}
        }
        let n = match tokio::time::timeout_at(deadline, read.read(&mut chunk)).await {
            Ok(Ok(n)) => n,
            Ok(Err(e)) => return Err(NoMarker::Io(e)),
            Err(_) => 0,
        };
        if n == 0 {
            return Err(if buf.is_empty() {
                NoMarker::Silent
            } else {
                NoMarker::Text
            });
        }
        buf.extend_from_slice(&chunk[..n]);
    }
    Ok(Cursor::new(buf).chain(read))
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncWriteExt, duplex};

    fn frame(body: &[u8]) -> Vec<u8> {
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(body);
        bytes
    }

    fn with_marker(before: &[u8], after: &[u8]) -> Vec<u8> {
        [before, STREAM_MARKER, after].concat()
    }

    #[test]
    fn scan_classifies_what_has_arrived() {
        let marker = STREAM_MARKER.len();
        let half = &STREAM_MARKER[..marker / 2];
        let table: Vec<(&str, Vec<u8>, Scan)> = vec![
            ("nothing yet", vec![], Scan::More),
            (
                "marker at offset 0",
                with_marker(b"", b""),
                Scan::Found(marker),
            ),
            (
                "marker after a banner",
                with_marker(b"Last login: x\n", b"abc"),
                Scan::Found(14 + marker),
            ),
            ("first bytes of the marker", half.to_vec(), Scan::More),
            (
                "marker cut in two reads",
                [b"hi\n".as_slice(), half].concat(),
                Scan::More,
            ),
            (
                "banner holding marker prefixes",
                with_marker(b"\0\x1e\0\x1emaittyx", b""),
                Scan::Found(2 + 8 + 1 + marker),
            ),
            ("banner too short to judge", b"Wel".to_vec(), Scan::More),
            ("banner", b"Welcome to the box\n".to_vec(), Scan::More),
            (
                "an old remote's Welcome",
                frame(b"\x81\xa7Welcome"),
                Scan::Frame,
            ),
            ("a frame's first bytes", vec![0, 0, 0], Scan::More),
        ];
        for (name, bytes, want) in table {
            assert_eq!(scan(&bytes, 64), want, "{name}");
        }
    }

    #[test]
    fn scan_stops_when_the_banner_outgrows_the_cap() {
        let marker = STREAM_MARKER.len();
        let text = |n: usize| vec![b'x'; n];
        assert_eq!(
            scan(&with_marker(&text(8), b""), 8),
            Scan::Found(8 + marker),
            "exactly at the cap"
        );
        assert_eq!(
            scan(&with_marker(&text(9), b""), 8),
            Scan::TooMuch,
            "one past the cap"
        );
        assert_eq!(
            scan(&text(8 + marker - 1), 8),
            Scan::More,
            "the marker could still start at the cap"
        );
        assert_eq!(
            scan(&text(8 + marker), 8),
            Scan::TooMuch,
            "no marker can fit any more"
        );
    }

    #[tokio::test]
    async fn banner_before_the_marker_is_dropped_and_the_frame_after_it_kept() {
        let (mut far, near) = duplex(64);
        let sent = frame(b"first");
        let task = tokio::spawn(async move {
            // Split inside the marker and between the marker and the frame.
            let bytes = with_marker(b"Last login: today\r\n", &sent);
            let cut = 19 + 3;
            far.write_all(&bytes[..cut]).await.unwrap();
            far.write_all(&bytes[cut..]).await.unwrap();
            far.shutdown().await.unwrap();
        });
        let mut resynced = skip_login_noise(near, LIMITS).await.unwrap();
        let mut got = Vec::new();
        resynced.read_to_end(&mut got).await.unwrap();
        task.await.unwrap();
        assert_eq!(got, frame(b"first"));
    }

    #[tokio::test]
    async fn a_remote_without_the_marker_keeps_its_first_frame() {
        let (mut far, near) = duplex(64);
        far.write_all(&frame(b"welcome")).await.unwrap();
        drop(far);
        let mut resynced = skip_login_noise(near, LIMITS).await.unwrap();
        let mut got = Vec::new();
        resynced.read_to_end(&mut got).await.unwrap();
        assert_eq!(got, frame(b"welcome"));
    }

    fn quick(cap: usize) -> Limits {
        Limits {
            cap,
            wait: Duration::from_millis(100),
        }
    }

    #[tokio::test]
    async fn text_without_a_marker_fails_at_the_end_of_the_stream_the_cap_and_the_wait() {
        let (mut ended, near) = duplex(64);
        ended.write_all(b"Welcome\n").await.unwrap();
        drop(ended);
        assert!(matches!(
            skip_login_noise(near, quick(64)).await,
            Err(NoMarker::Text)
        ));

        let (mut flood, near) = duplex(1024);
        flood.write_all(&[b'x'; 200]).await.unwrap();
        assert!(matches!(
            skip_login_noise(near, quick(64)).await,
            Err(NoMarker::Text)
        ));

        let (mut stalled, near) = duplex(64);
        stalled.write_all(b"Welcome\n").await.unwrap();
        assert!(matches!(
            skip_login_noise(near, quick(64)).await,
            Err(NoMarker::Text)
        ));
    }

    #[tokio::test]
    async fn silence_is_told_from_text() {
        let (ended, near) = duplex(64);
        drop(ended);
        assert!(matches!(
            skip_login_noise(near, quick(64)).await,
            Err(NoMarker::Silent)
        ));

        let (_open, near) = duplex(64);
        assert!(matches!(
            skip_login_noise(near, quick(64)).await,
            Err(NoMarker::Silent)
        ));
    }
}
