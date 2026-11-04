//! The window's connections to daemons, one per machine. Each has a reader task and a writer
//! task of its own; the one event loop takes what the readers collected and routes what the
//! window sends by machine. A connection that ends is marked lost and stays in the registry:
//! it never ends the window.

pub use crate::ids::{MachineId, Outgoing};
use maitty_ipc_transport as ipc;
use maitty_shared::protocol::{ClientMsg, ServerMsg};
use std::task::Poll;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

/// Server messages taken in one go before the next frame.
pub const SERVER_BURST: usize = 512;

/// How many messages a reader holds ahead of the loop. Past it the reader stops reading its
/// socket, so a daemon sees a window that falls behind and lets go of it, as it always has.
const INBOX: usize = SERVER_BURST;

/// How long a frame may sit part-arrived, with no more of it coming, before the line is
/// judged dead. Two quiet windows in a row decide it, so a dead line is dropped within twice
/// this. It is under ssh's own `ServerAlive` (3 x 15 s), which is what ends an idle line: a line
/// with nothing to say has no frame half sent, so silence alone proves nothing.
const STALL: Duration = Duration::from_secs(15);

/// A message for a machine that is lost or was never connected.
#[derive(Debug, PartialEq, Eq)]
pub struct NotConnected(pub MachineId);

/// What one turn of the loop takes from one machine.
#[derive(Debug)]
pub struct Burst {
    pub machine: MachineId,
    pub msgs: Vec<ServerMsg>,
    /// The connection ended after `msgs`.
    pub lost: bool,
    /// It ended because a frame stopped arriving, not because the other side closed it.
    pub stalled: bool,
}

/// A message, or how the connection ended.
type Inbound = Result<ServerMsg, End>;

enum End {
    Closed,
    /// A frame began and no more of it came (see `STALL`).
    Stalled,
}

struct Entry {
    id: MachineId,
    inbox: mpsc::Receiver<Inbound>,
    outbox: mpsc::UnboundedSender<ClientMsg>,
    tasks: [JoinHandle<()>; 2],
    lost: bool,
}

impl Entry {
    /// Stop reading and writing now. Dropping the writer is what ends an ssh child.
    fn close(&mut self) {
        self.tasks.iter().for_each(JoinHandle::abort);
    }
}

impl Drop for Entry {
    fn drop(&mut self) {
        self.close();
    }
}

#[derive(Default)]
pub struct Connections {
    entries: Vec<Entry>,
    /// Where the next look for a ready machine starts: the one after the last served, so a
    /// chatty machine cannot keep the others waiting.
    after: usize,
}

impl Connections {
    /// Start reading and writing a connection for `id`, replacing the one it had.
    pub fn add(&mut self, id: MachineId, halves: (ipc::Reader, ipc::Writer)) {
        self.add_with_stall(id, halves, STALL);
    }

    fn add_with_stall(
        &mut self,
        id: MachineId,
        (reader, writer): (ipc::Reader, ipc::Writer),
        stall: Duration,
    ) {
        let (inbox_tx, inbox) = mpsc::channel(INBOX);
        let (outbox, outbox_rx) = mpsc::unbounded_channel();
        let entry = Entry {
            id,
            inbox,
            outbox,
            tasks: [
                tokio::spawn(read_until_lost(reader, inbox_tx, stall)),
                tokio::spawn(write_until_lost(writer, outbox_rx)),
            ],
            lost: false,
        };
        match self.entries.iter_mut().find(|e| e.id == id) {
            Some(slot) => *slot = entry,
            None => self.entries.push(entry),
        }
    }

    /// Let go of a machine's connection for good, as when it is no longer one of the machines.
    pub fn remove(&mut self, id: MachineId) {
        self.entries.retain(|e| e.id != id);
    }

    pub fn is_connected(&self, id: MachineId) -> bool {
        self.entries.iter().any(|e| e.id == id && !e.lost)
    }

    /// End a machine's connection from this side, as if it had been lost.
    pub fn mark_lost(&mut self, id: MachineId) {
        if let Some(entry) = self.entries.iter_mut().find(|e| e.id == id) {
            entry.lost = true;
            entry.close();
        }
    }

    /// Hand a message to its machine's writer. Never waits.
    pub fn send(&self, Outgoing { machine, msg }: Outgoing) -> Result<(), NotConnected> {
        let entry = self
            .entries
            .iter()
            .find(|e| e.id == machine && !e.lost)
            .ok_or(NotConnected(machine))?;
        entry.outbox.send(msg).map_err(|_| NotConnected(machine))
    }

    /// Wait for a machine with something to say, then take up to `max` messages from that
    /// machine alone. Waits for ever with no connected machine.
    pub async fn recv_burst(&mut self, max: usize) -> Burst {
        let (at, first) = std::future::poll_fn(|cx| self.poll_first(cx)).await;
        let entry = &mut self.entries[at];
        let mut msgs = Vec::new();
        let mut end = None;
        let mut next = Some(first);
        while let Some(item) = next.take() {
            match item {
                Ok(m) => msgs.push(m),
                Err(how) => {
                    end = Some(how);
                    break;
                }
            }
            if msgs.len() < max {
                next = match entry.inbox.try_recv() {
                    Ok(item) => Some(item),
                    Err(mpsc::error::TryRecvError::Empty) => None,
                    Err(mpsc::error::TryRecvError::Disconnected) => Some(Err(End::Closed)),
                };
            }
        }
        let lost = end.is_some();
        if lost {
            entry.lost = true;
            entry.close();
        }
        Burst {
            machine: entry.id,
            msgs,
            lost,
            stalled: matches!(end, Some(End::Stalled)),
        }
    }

    fn poll_first(&mut self, cx: &mut std::task::Context<'_>) -> Poll<(usize, Inbound)> {
        let len = self.entries.len();
        for step in 0..len {
            let at = (self.after + step) % len;
            let entry = &mut self.entries[at];
            if entry.lost {
                continue;
            }
            match entry.inbox.poll_recv(cx) {
                Poll::Ready(item) => {
                    self.after = at + 1;
                    return Poll::Ready((at, item.unwrap_or(Err(End::Closed))));
                }
                Poll::Pending => {}
            }
        }
        Poll::Pending
    }
}

/// Whether a frame has stopped arriving, judged from how much of one is buffered at the end
/// of each quiet window. A frame that trickles in grows the buffer every window, however
/// slowly, so a big frame on a slow link is never judged dead; only one that does not grow is.
#[derive(Default)]
struct Stall {
    buffered: Option<usize>,
}

impl Stall {
    fn progressed(&mut self) {
        self.buffered = None;
    }

    /// A window passed with no whole frame: `buffered` is what is held of a partial one.
    fn stalled(&mut self, buffered: usize) -> bool {
        let stuck = buffered > 0 && self.buffered == Some(buffered);
        self.buffered = Some(buffered);
        stuck
    }
}

async fn read_until_lost(mut reader: ipc::Reader, inbox: mpsc::Sender<Inbound>, stall: Duration) {
    let mut watch = Stall::default();
    let end = loop {
        match tokio::time::timeout(stall, ipc::receive::<ServerMsg>(&mut reader)).await {
            Ok(Ok(Some(m))) => {
                watch.progressed();
                if inbox.send(Ok(m)).await.is_err() {
                    return;
                }
            }
            // One message that didn't decode: skip it rather than drop the connection.
            Ok(Err(e)) if ipc::is_decode_error(&e) => {
                watch.progressed();
                tracing::warn!("skipped a message from the server: {e:#}");
            }
            Ok(_) => break End::Closed,
            Err(_) if watch.stalled(reader.read_buffer().len()) => break End::Stalled,
            Err(_) => {}
        }
    };
    let _ = inbox.send(Err(end)).await;
}

async fn write_until_lost(mut writer: ipc::Writer, mut outbox: mpsc::UnboundedReceiver<ClientMsg>) {
    while let Some(msg) = outbox.recv().await {
        if ipc::send(&mut writer, &msg).await.is_err() {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::DuplexStream;
    use tokio_util::codec::{FramedRead, FramedWrite, LengthDelimitedCodec};

    type Half = (ipc::Reader, ipc::Writer);

    fn half(stream: DuplexStream) -> Half {
        let (read, write) = tokio::io::split(stream);
        (
            FramedRead::new(Box::new(read), LengthDelimitedCodec::new()),
            FramedWrite::new(Box::new(write), LengthDelimitedCodec::new()),
        )
    }

    /// A connection for the registry and the daemon end of it.
    fn link() -> (Half, Half) {
        let (window, daemon) = tokio::io::duplex(64 * 1024);
        (half(window), half(daemon))
    }

    fn notice(text: &str) -> ServerMsg {
        ServerMsg::Notice(text.into())
    }

    fn said(burst: &Burst) -> Vec<&str> {
        burst
            .msgs
            .iter()
            .map(|m| match m {
                ServerMsg::Notice(text) => text.as_str(),
                other => panic!("unexpected {other:?}"),
            })
            .collect()
    }

    /// Lets every task that can run, run: on the test's single thread nothing else moves.
    async fn settle() {
        for _ in 0..64 {
            tokio::task::yield_now().await;
        }
    }

    async fn within<T>(what: impl std::future::Future<Output = T>) -> T {
        tokio::time::timeout(std::time::Duration::from_secs(30), what)
            .await
            .expect("timed out")
    }

    #[tokio::test]
    async fn a_chatty_machine_does_not_starve_another() {
        const MAX: usize = 8;
        let (here, mut here_daemon) = link();
        let (there, mut there_daemon) = link();
        let mut connections = Connections::default();
        connections.add(MachineId(1), there);
        connections.add(MachineId::LOCAL, here);
        tokio::spawn(async move {
            for i in 0..10 * INBOX {
                ipc::send(&mut there_daemon.1, &notice(&i.to_string()))
                    .await
                    .unwrap();
            }
        });
        ipc::send(&mut here_daemon.1, &notice("mine"))
            .await
            .unwrap();
        settle().await;

        let first = within(connections.recv_burst(MAX)).await;
        let second = within(connections.recv_burst(MAX)).await;

        assert!(first.msgs.len() <= MAX && second.msgs.len() <= MAX);
        let local = [&first, &second]
            .into_iter()
            .find(|b| b.machine == MachineId::LOCAL)
            .expect("the local machine is served within two turns");
        assert_eq!(said(local), ["mine"]);
    }

    #[tokio::test]
    async fn a_machine_that_ends_is_lost_and_the_others_carry_on() {
        let (here, mut here_daemon) = link();
        let (there, there_daemon) = link();
        let mut connections = Connections::default();
        connections.add(MachineId::LOCAL, here);
        connections.add(MachineId(1), there);
        drop(there_daemon);

        let lost = within(connections.recv_burst(8)).await;
        assert_eq!((lost.machine, lost.lost), (MachineId(1), true));
        assert!(!connections.is_connected(MachineId(1)));
        assert!(connections.is_connected(MachineId::LOCAL));

        let dropped = Outgoing {
            machine: MachineId(1),
            msg: ClientMsg::Resize {
                term: 1,
                cols: 80,
                rows: 24,
            },
        };
        assert_eq!(connections.send(dropped), Err(NotConnected(MachineId(1))));
        let unknown = Outgoing {
            machine: MachineId(7),
            msg: ClientMsg::Resize {
                term: 1,
                cols: 80,
                rows: 24,
            },
        };
        assert_eq!(connections.send(unknown), Err(NotConnected(MachineId(7))));

        ipc::send(&mut here_daemon.1, &notice("still here"))
            .await
            .unwrap();
        let next = within(connections.recv_burst(8)).await;
        assert_eq!(
            (next.machine, said(&next)),
            (MachineId::LOCAL, vec!["still here"])
        );

        let kept = Outgoing {
            machine: MachineId::LOCAL,
            msg: ClientMsg::Resize {
                term: 3,
                cols: 100,
                rows: 30,
            },
        };
        connections.send(kept).unwrap();
        let heard: ClientMsg = within(ipc::receive::<ClientMsg>(&mut here_daemon.0))
            .await
            .unwrap()
            .unwrap();
        assert!(matches!(
            heard,
            ClientMsg::Resize {
                term: 3,
                cols: 100,
                rows: 30
            }
        ));
    }

    #[tokio::test]
    async fn a_machine_marked_lost_is_no_longer_connected() {
        let (there, _there_daemon) = link();
        let mut connections = Connections::default();
        connections.add(MachineId(1), there);
        connections.mark_lost(MachineId(1));
        assert!(!connections.is_connected(MachineId(1)));
    }

    const TINY: Duration = Duration::from_millis(40);

    /// The bytes of one `Notice` frame, as a daemon would write them.
    fn frame_of(text: &str) -> Vec<u8> {
        let body = maitty_shared::protocol::encode(&notice(text)).unwrap();
        let mut bytes = (body.len() as u32).to_be_bytes().to_vec();
        bytes.extend_from_slice(&body);
        bytes
    }

    async fn raw_link() -> (Half, tokio::io::DuplexStream) {
        let (window, daemon) = tokio::io::duplex(64 * 1024);
        let (read, write) = tokio::io::split(window);
        let halves = (
            FramedRead::new(Box::new(read) as Box<_>, LengthDelimitedCodec::new()),
            FramedWrite::new(Box::new(write) as Box<_>, LengthDelimitedCodec::new()),
        );
        (halves, daemon)
    }

    #[tokio::test]
    async fn a_big_frame_on_a_slow_link_is_not_judged_dead() {
        use tokio::io::AsyncWriteExt;
        let (halves, mut daemon) = raw_link().await;
        let mut connections = Connections::default();
        connections.add_with_stall(MachineId(1), halves, TINY);
        let long = "x".repeat(2000);
        let bytes = frame_of(&long);
        let trickle = tokio::spawn(async move {
            for piece in bytes.chunks(100) {
                daemon.write_all(piece).await.unwrap();
                tokio::time::sleep(TINY / 2).await;
            }
            daemon
        });
        let burst = within(connections.recv_burst(8)).await;
        assert!(!burst.lost, "bytes kept coming: {burst:?}");
        assert_eq!(said(&burst), [long.as_str()]);
        drop(trickle.await.unwrap());
    }

    #[tokio::test]
    async fn a_frame_that_stops_arriving_is_judged_dead() {
        use tokio::io::AsyncWriteExt;
        let (halves, mut daemon) = raw_link().await;
        let mut connections = Connections::default();
        connections.add_with_stall(MachineId(1), halves, TINY);
        let bytes = frame_of(&"x".repeat(2000));
        daemon.write_all(&bytes[..500]).await.unwrap();

        let burst = within(connections.recv_burst(8)).await;
        assert!(burst.lost && burst.stalled, "{burst:?}");
        assert!(!connections.is_connected(MachineId(1)));
        drop(daemon);
    }

    #[tokio::test]
    async fn an_idle_line_with_nothing_to_say_is_not_judged_dead() {
        let (halves, mut daemon) = raw_link().await;
        let mut connections = Connections::default();
        connections.add_with_stall(MachineId(1), halves, TINY);
        let quiet = tokio::time::timeout(TINY * 10, connections.recv_burst(8)).await;
        assert!(quiet.is_err(), "an idle line must stay open");
        assert!(connections.is_connected(MachineId(1)));

        tokio::io::AsyncWriteExt::write_all(&mut daemon, &frame_of("still here"))
            .await
            .unwrap();
        let burst = within(connections.recv_burst(8)).await;
        assert_eq!(said(&burst), ["still here"]);
    }

    #[test]
    fn a_stall_needs_a_partial_frame_that_did_not_grow() {
        let mut watch = Stall::default();
        assert!(!watch.stalled(0), "idle");
        assert!(!watch.stalled(0), "idle again");
        assert!(!watch.stalled(100), "a frame began");
        assert!(!watch.stalled(200), "and grew");
        assert!(watch.stalled(200), "then stopped");
        watch.progressed();
        assert!(!watch.stalled(200), "a whole frame arrived in between");
    }

    #[tokio::test]
    async fn adding_a_machine_again_replaces_its_lost_connection() {
        let (first, first_daemon) = link();
        let mut connections = Connections::default();
        connections.add(MachineId::LOCAL, first);
        drop(first_daemon);
        assert!(within(connections.recv_burst(8)).await.lost);

        let (second, mut second_daemon) = link();
        connections.add(MachineId::LOCAL, second);
        assert!(connections.is_connected(MachineId::LOCAL));
        ipc::send(&mut second_daemon.1, &notice("back"))
            .await
            .unwrap();
        let burst = within(connections.recv_burst(8)).await;
        assert_eq!(said(&burst), ["back"]);
    }
}
