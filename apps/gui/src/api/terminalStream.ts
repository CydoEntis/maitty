import { Channel, invoke } from '@tauri-apps/api/core'
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

/**
 * The host streams every pane's output over one raw channel, opened the first time a view asks.
 * Each frame is handed only to the views of its pane, so a pane never parses another's output.
 */
function ensureWatching(): Promise<void> {
  watching ??= (async () => {
    const onChunk = new Channel<ArrayBuffer>()
    onChunk.onmessage = (frame) => {
      const chunk = decodeFrame(bytesOf(frame))
      countOutput(chunk.bytes.length)
      for (const handler of handlers.get(paneKey(chunk.machine, chunk.term)) ?? []) handler(chunk)
    }
    await invoke('watch_terminal_output', { onChunk })
  })().catch((error: unknown) => {
    // The next view tries again rather than inheriting a channel that never opened.
    watching = null
    throw error
  })
  return watching
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
    if (set.size === 0) handlers.delete(key)
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
