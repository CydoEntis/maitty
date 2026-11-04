import { beforeEach, describe, expect, it, vi } from 'vitest'
import { Channel, invoke } from '@tauri-apps/api/core'
import { listenTerminalChunks, loadTerminalHistory, type TerminalChunk } from './terminalStream'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  Channel: class {
    onmessage: ((message: ArrayBuffer) => void) | undefined
  },
}))

const SEQ_BYTES = 8
const HEADER_BYTES = 2 + 4 + SEQ_BYTES + 1

function historyReply(seq: number, text: string): ArrayBuffer {
  const output = new TextEncoder().encode(text)
  const body = new Uint8Array(SEQ_BYTES + output.length)
  new DataView(body.buffer).setBigUint64(0, BigInt(seq))
  body.set(output, SEQ_BYTES)
  return body.buffer
}

function frame(term: number, seq: number, replay: boolean, text: string, machine = 0): ArrayBuffer {
  const output = new TextEncoder().encode(text)
  const body = new Uint8Array(HEADER_BYTES + output.length)
  const view = new DataView(body.buffer)
  view.setUint16(0, machine)
  view.setUint32(2, term)
  view.setBigUint64(6, BigInt(seq))
  body[HEADER_BYTES - 1] = replay ? 1 : 0
  body.set(output, HEADER_BYTES)
  return body.buffer
}

/** The channel the module handed to the host, so a test can push frames through it. */
let opened: Channel<ArrayBuffer> | undefined

function hostChannel(): Channel<ArrayBuffer> {
  const call = vi.mocked(invoke).mock.calls.find(([command]) => command === 'watch_terminal_output')
  const args = call?.[1] as { onChunk: Channel<ArrayBuffer> } | undefined
  // The module opens the channel once, in the first test that listens; later tests reuse it.
  opened ??= args?.onChunk
  if (!opened) throw new Error('the output channel was not opened')
  return opened
}

describe('terminal stream', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    vi.mocked(invoke).mockResolvedValue(undefined)
  })

  it('reads the history reply as its sequence number followed by the output', async () => {
    vi.mocked(invoke).mockResolvedValue(historyReply(42, 'hello'))

    const history = await loadTerminalHistory(7)

    expect(invoke).toHaveBeenCalledWith('terminal_history', { term: 7 })
    expect(history?.seq).toBe(42)
    expect(new TextDecoder().decode(history?.bytes)).toBe('hello')
  })

  it('reads an empty reply as a dropped history', async () => {
    vi.mocked(invoke).mockResolvedValue(new ArrayBuffer(0))
    await expect(loadTerminalHistory(7)).resolves.toBeNull()
  })

  it('opens one output channel and hands each pane only its own frames, decoded', async () => {
    const mine: TerminalChunk[] = []
    const other: TerminalChunk[] = []
    const stopMine = await listenTerminalChunks(5, (chunk) => { mine.push(chunk) })
    await listenTerminalChunks(6, (chunk) => { other.push(chunk) })
    const channel = hostChannel()

    channel.onmessage(frame(6, 1, false, 'other'))
    channel.onmessage(frame(5, 2, true, 'mine'))
    stopMine()
    channel.onmessage(frame(5, 3, false, 'after'))

    expect(vi.mocked(invoke).mock.calls.filter(([command]) => command === 'watch_terminal_output')).toHaveLength(1)
    expect(mine).toHaveLength(1)
    expect(mine[0]?.seq).toBe(2)
    expect(mine[0]?.replay).toBe(true)
    expect(new TextDecoder().decode(mine[0]?.bytes)).toBe('mine')
    expect(other.map((chunk) => new TextDecoder().decode(chunk.bytes))).toEqual(['other'])
  })

  it('hands a pane of another machine only to the views of that machine, whatever the pane number', async () => {
    const here: TerminalChunk[] = []
    const there: TerminalChunk[] = []
    const stopHere = await listenTerminalChunks(5, (chunk) => { here.push(chunk) })
    const stopThere = await listenTerminalChunks(5, (chunk) => { there.push(chunk) }, 3)
    const channel = hostChannel()

    channel.onmessage(frame(5, 1, true, 'local', 0))
    channel.onmessage(frame(5, 1, true, 'remote', 3))
    channel.onmessage(frame(5, 1, true, 'unwatched', 4))
    stopHere()
    stopThere()

    expect(here.map((chunk) => [chunk.machine, new TextDecoder().decode(chunk.bytes)])).toEqual([[0, 'local']])
    expect(there.map((chunk) => [chunk.machine, new TextDecoder().decode(chunk.bytes)])).toEqual([[3, 'remote']])
  })

  it('decodes the machine, pane, sequence number and replay flag of a frame', async () => {
    const seen: TerminalChunk[] = []
    const stop = await listenTerminalChunks(9, (chunk) => { seen.push(chunk) }, 258)
    hostChannel().onmessage(frame(9, 70000, true, 'x', 258))
    stop()
    expect(seen).toHaveLength(1)
    expect(seen[0]).toMatchObject({ machine: 258, term: 9, seq: 70000, replay: true })
  })

  it('asks for the history of a pane on another machine by naming the machine', async () => {
    vi.mocked(invoke).mockResolvedValue(historyReply(3, 'there'))
    const history = await loadTerminalHistory(7, 2)
    expect(invoke).toHaveBeenCalledWith('terminal_history', { term: 7, machine: 2 })
    expect(history?.seq).toBe(3)
  })
})
