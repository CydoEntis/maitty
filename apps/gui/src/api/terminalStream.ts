import { Channel, invoke } from '@tauri-apps/api/core'
import { bytesOf } from './ipcBytes'
import { countOutput } from '@/lib/perf-meter'

/** One piece of a pane's output, in order. A replay replaces what the view shows. */
export type TerminalChunk = { term: number; seq: number; replay: boolean; bytes: Uint8Array }

export type TerminalHistory = { seq: number; bytes: Uint8Array }

/** The history reply starts with the sequence number it ends at, as big-endian bytes. */
const SEQ_BYTES = 8
/** A chunk frame: the pane as 4 big-endian bytes, the sequence number as 8, one byte for replay, then the output. */
const FRAME_TERM_BYTES = 4
const FRAME_REPLAY_OFFSET = FRAME_TERM_BYTES + SEQ_BYTES
const FRAME_HEADER_BYTES = FRAME_REPLAY_OFFSET + 1

type ChunkHandler = (chunk: TerminalChunk) => void

const handlers = new Map<number, Set<ChunkHandler>>()
let watching: Promise<void> | null = null

function decodeFrame(frame: Uint8Array): TerminalChunk {
  if (frame.length < FRAME_HEADER_BYTES) throw new Error('The terminal output frame is too short to carry its header')
  const view = new DataView(frame.buffer, frame.byteOffset, FRAME_HEADER_BYTES)
  return {
    term: view.getUint32(0),
    seq: Number(view.getBigUint64(FRAME_TERM_BYTES)),
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
      for (const handler of handlers.get(chunk.term) ?? []) handler(chunk)
    }
    await invoke('watch_terminal_output', { onChunk })
  })().catch((error: unknown) => {
    // The next view tries again rather than inheriting a channel that never opened.
    watching = null
    throw error
  })
  return watching
}

/** Calls back with each chunk of one pane's output as the daemon pushes it. */
export async function listenTerminalChunks(term: number, onChunk: ChunkHandler): Promise<() => void> {
  await ensureWatching()
  const set = handlers.get(term) ?? new Set<ChunkHandler>()
  set.add(onChunk)
  handlers.set(term, set)
  return () => {
    set.delete(onChunk)
    if (set.size === 0) handlers.delete(term)
  }
}

/**
 * Everything a pane has shown since its last replay, and the sequence number it ends at. Chunks
 * with a sequence number at or below it are already included. Null when the history was dropped.
 */
export async function loadTerminalHistory(term: number): Promise<TerminalHistory | null> {
  const reply = bytesOf(await invoke<unknown>('terminal_history', { term }))
  if (reply.length === 0) return null
  if (reply.length < SEQ_BYTES) throw new Error('The terminal history reply is too short to carry its sequence number')
  const seq = Number(new DataView(reply.buffer, reply.byteOffset, SEQ_BYTES).getBigUint64(0))
  return { seq, bytes: reply.subarray(SEQ_BYTES) }
}
