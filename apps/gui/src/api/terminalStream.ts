import { invoke } from '@tauri-apps/api/core'
import { listen } from '@tauri-apps/api/event'
import { z } from 'zod'

/** One piece of a pane's output, in order. A replay replaces what the view shows. */
export type TerminalChunk = { term: number; seq: number; replay: boolean; bytes: Uint8Array }

export type TerminalHistory = { seq: number; bytes: Uint8Array }

const ChunkSchema = z.object({ term: z.number(), seq: z.number(), replay: z.boolean(), data: z.string() })
const HistorySchema = z.object({ seq: z.number(), data: z.string() }).nullable()

function decode(data: string): Uint8Array {
  const binary = atob(data)
  const bytes = new Uint8Array(binary.length)
  for (let index = 0; index < binary.length; index += 1) bytes[index] = binary.charCodeAt(index)
  return bytes
}

/** Calls back with each chunk of one pane's output as the daemon pushes it. */
export async function listenTerminalChunks(term: number, onChunk: (chunk: TerminalChunk) => void): Promise<() => void> {
  return listen<unknown>('terminal-chunk', (event) => {
    const parsed = ChunkSchema.safeParse(event.payload)
    if (!parsed.success || parsed.data.term !== term) return
    onChunk({ term, seq: parsed.data.seq, replay: parsed.data.replay, bytes: decode(parsed.data.data) })
  })
}

/**
 * Everything a pane has shown since its last replay, and the sequence number it ends at. Chunks
 * with a sequence number at or below it are already included. Null when the history was dropped.
 */
export async function loadTerminalHistory(term: number): Promise<TerminalHistory | null> {
  const history = HistorySchema.parse(await invoke<unknown>('terminal_history', { term }))
  return history === null ? null : { seq: history.seq, bytes: decode(history.data) }
}
