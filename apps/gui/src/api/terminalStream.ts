import { Channel, invoke } from '@tauri-apps/api/core'
import { z } from 'zod'
import { bytesOf } from './ipcBytes'
import { countOutput } from '@/lib/perf-meter'

/** One piece of a pane's output, in order. A replay replaces what the view shows. */
/** `machine` is 0 for this computer, else the id the host gave the machine. */
export type TerminalChunk = { machine: number; term: number; seq: number; replay: boolean; bytes: Uint8Array }

export type TerminalHistory = { seq: number; bytes: Uint8Array }

/** The history reply starts with the sequence number it ends at, as big-endian bytes. */
const SEQ_BYTES = 8
/**
 * A chunk frame: the machine as 2 big-endian bytes, the pane as 4, the sequence number as 8, one
 * byte for replay, then the output. Each machine numbers its own panes.
 */
const FRAME_MACHINE_BYTES = 2
const FRAME_TERM_BYTES = 4
const FRAME_SEQ_OFFSET = FRAME_MACHINE_BYTES + FRAME_TERM_BYTES
const FRAME_REPLAY_OFFSET = FRAME_SEQ_OFFSET + SEQ_BYTES
const FRAME_HEADER_BYTES = FRAME_REPLAY_OFFSET + 1
/** This computer. */
const LOCAL_MACHINE = 0

type ChunkHandler = (chunk: TerminalChunk) => void

const handlers = new Map<string, Set<ChunkHandler>>()

function paneKey(machine: number, term: number): string {
  return `${String(machine)}/${String(term)}`
}
let watching: Promise<void> | null = null

function decodeFrame(frame: Uint8Array): TerminalChunk {
  if (frame.length < FRAME_HEADER_BYTES) throw new Error('The terminal output frame is too short to carry its header')
  const view = new DataView(frame.buffer, frame.byteOffset, FRAME_HEADER_BYTES)
  return {
    machine: view.getUint16(0),
    term: view.getUint32(FRAME_MACHINE_BYTES),
    seq: Number(view.getBigUint64(FRAME_SEQ_OFFSET)),
    replay: frame[FRAME_REPLAY_OFFSET] === 1,
    bytes: frame.subarray(FRAME_HEADER_BYTES),
  }
}

/** The last sequence number handed out for each pane, to drop a frame that arrives twice or late. */
const lastSeq = new Map<string, number>()

/** Sends a frame to the views of its pane. A gap in the numbers means a frame was lost on the way. */
function deliver(frame: Uint8Array): void {
  const chunk = decodeFrame(frame)
  countOutput(chunk.bytes.length)
  const key = paneKey(chunk.machine, chunk.term)
  const views = handlers.get(key)
  // Nobody is watching this pane: a view that opens later starts from the host's history.
  if (views === undefined) return
  const last = lastSeq.get(key)
  if (last !== undefined && chunk.seq <= last) return
  lastSeq.set(key, chunk.seq)
  for (const handler of views) handler(chunk)
  if (last !== undefined && !chunk.replay && chunk.seq > last + 1) void resyncPane(key)
}

/**
 * Brings a pane's views up to what the host holds. A view that already has all of it ignores the
 * reply (its numbers are not behind); one that missed a frame replaces its screen.
 */
async function resyncPane(key: string): Promise<void> {
  const [machine, term] = key.split('/').map(Number)
  if (machine === undefined || term === undefined) return
  const history = await loadTerminalHistory(term, machine).catch(() => null)
  if (history === null) return
  const chunk: TerminalChunk = { machine, term, seq: history.seq, replay: true, bytes: history.bytes }
  lastSeq.set(key, Math.max(lastSeq.get(key) ?? 0, history.seq))
  for (const handler of handlers.get(key) ?? []) handler(chunk)
}

function resyncAll(): void {
  for (const key of handlers.keys()) void resyncPane(key)
}

/**
 * The host streams every pane's output over one raw channel, opened the first time a view asks.
 * Each frame is handed only to the views of its pane, so a pane never parses another's output.
 * The channel is also how output arrives until the local socket below is up.
 */
function ensureWatching(): Promise<void> {
  watching ??= (async () => {
    const onChunk = new Channel<ArrayBuffer>()
    onChunk.onmessage = (frame) => { deliver(bytesOf(frame)) }
    await invoke('watch_terminal_output', { onChunk })
    void connectSocket()
  })().catch((error: unknown) => {
    // The next view tries again rather than inheriting a channel that never opened.
    watching = null
    throw error
  })
  return watching
}

const MAX_PORT = 65_535
const SocketGrant = z.object({ port: z.number().int().min(1).max(MAX_PORT), token: z.string().regex(/^[0-9a-f]{64}$/) })
/** The word the host sends once it has accepted the token. */
const SOCKET_READY = 'ready'
const SOCKET_RETRY_FIRST_MS = 1000
const SOCKET_RETRY_LAST_MS = 30_000
const SOCKET_BACKOFF = 2
/** Refused this many times in a row without ever being ready: leave it to the channel. */
const SOCKET_GIVE_UP_AFTER = 5

/**
 * Output also arrives on a WebSocket to the host, read by the web view's own network stack. The
 * channel waits for the host's main loop and the next window event, which held a pane a step
 * behind (ticket 35). The host sends nothing down the socket until it says `ready`, so a socket
 * that never opens leaves the channel doing what it always did.
 */
async function connectSocket(failures = 0, wait = SOCKET_RETRY_FIRST_MS): Promise<void> {
  let grant: z.infer<typeof SocketGrant>
  try {
    grant = SocketGrant.parse(await invoke('open_terminal_socket'))
  } catch {
    return
  }
  let ready = false
  const retry = () => {
    if (ready) resyncAll()
    const next = ready ? 0 : failures + 1
    if (next >= SOCKET_GIVE_UP_AFTER) return
    window.setTimeout(() => { void connectSocket(next, ready ? SOCKET_RETRY_FIRST_MS : Math.min(wait * SOCKET_BACKOFF, SOCKET_RETRY_LAST_MS)) }, wait)
  }
  let socket: WebSocket
  try {
    socket = new WebSocket(`ws://127.0.0.1:${String(grant.port)}/`)
  } catch {
    retry()
    return
  }
  socket.binaryType = 'arraybuffer'
  socket.onopen = () => { socket.send(grant.token) }
  socket.onmessage = (event: MessageEvent<unknown>) => {
    if (typeof event.data === 'string') {
      if (event.data !== SOCKET_READY) return
      ready = true
      // Frames sent down the channel before the switch can still be on their way, and arrive after
      // newer ones from the socket; the host's history has all of them.
      resyncAll()
      return
    }
    if (event.data instanceof ArrayBuffer) deliver(new Uint8Array(event.data))
  }
  let closed = false
  const closedOnce = () => {
    if (closed) return
    closed = true
    retry()
  }
  socket.onclose = closedOnce
  socket.onerror = closedOnce
}

/**
 * Calls back with each chunk of one pane's output as its daemon pushes it. The pane is on this
 * computer unless a machine is named.
 */
export async function listenTerminalChunks(
  term: number,
  onChunk: ChunkHandler,
  machine: number = LOCAL_MACHINE,
): Promise<() => void> {
  await ensureWatching()
  const key = paneKey(machine, term)
  const set = handlers.get(key) ?? new Set<ChunkHandler>()
  set.add(onChunk)
  handlers.set(key, set)
  return () => {
    set.delete(onChunk)
    if (set.size === 0) {
      handlers.delete(key)
      lastSeq.delete(key)
    }
  }
}

/**
 * Everything a pane has shown since its last replay, and the sequence number it ends at. Chunks
 * with a sequence number at or below it are already included. Null when the history was dropped.
 */
export async function loadTerminalHistory(
  term: number,
  machine: number = LOCAL_MACHINE,
): Promise<TerminalHistory | null> {
  const args = machine === LOCAL_MACHINE ? { term } : { term, machine }
  const reply = bytesOf(await invoke<unknown>('terminal_history', args))
  if (reply.length === 0) return null
  if (reply.length < SEQ_BYTES) throw new Error('The terminal history reply is too short to carry its sequence number')
  const seq = Number(new DataView(reply.buffer, reply.byteOffset, SEQ_BYTES).getBigUint64(0))
  return { seq, bytes: reply.subarray(SEQ_BYTES) }
}
