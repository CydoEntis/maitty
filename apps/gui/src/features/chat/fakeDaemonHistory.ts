import type { ChatPage, HistoryCursor } from '@/api/daemon'

/**
 * A stand-in for the daemon's conversation history, for tests. It pages the way the daemon does:
 * `next` is set only while entries remain, and a cursor from before the last change is refused as
 * stale because every change moves the revision.
 */

type Entry = Readonly<Record<string, unknown>>

export const FAKE_CONVERSATION = 5

export class FakeDaemonHistory {
  entries: Entry[] = []
  revision = 1
  generation = 1
  status: ChatPage['status'] = 'Idle'
  approvals: unknown[] = []
  answeredApprovals: unknown[] = []
  /** Every read, as the cursor it asked from (`null`: the start). */
  reads: Array<HistoryCursor | null> = []
  /** While set, every read fails with it. */
  failure: Error | null = null

  private readonly pageSize: number

  constructor(pageSize = 128) {
    this.pageSize = pageSize
  }

  message(turn: string, item: string, role: 'User' | 'Assistant', text: string, state = 'Completed'): Entry {
    return { source: { conversation: FAKE_CONVERSATION, turn, item }, content: { Message: { role, text } }, state }
  }

  tool(turn: string, item: string, label: string, output: string, state = 'Completed'): Entry {
    return { source: { conversation: FAKE_CONVERSATION, turn, item }, content: { Tool: { kind: 'commandExecution', label, output } }, state }
  }

  turnStatus(turn: string, state: string, times: { started_at_ms?: number; completed_at_ms?: number } = {}): Entry {
    return { source: { conversation: FAKE_CONVERSATION, turn, item: '' }, content: 'TurnStatus', state, ...times }
  }

  /** Adds an entry, or replaces the one with the same address in place, as the daemon does. */
  upsert(entry: Entry): void {
    const source = entry.source as { turn: string; item: string }
    const at = this.entries.findIndex((held) => {
      const heldSource = held.source as { turn: string; item: string }
      return heldSource.turn === source.turn && heldSource.item === source.item
    })
    if (at === -1) this.entries.push(entry)
    else this.entries[at] = entry
    this.revision += 1
  }

  /** The daemon restarted the conversation: a new generation, a new history. */
  restart(): void {
    this.generation += 1
    this.revision = 1
    this.entries = []
  }

  read = (cursor: HistoryCursor | null): Promise<ChatPage> => {
    this.reads.push(cursor)
    if (this.failure) return Promise.reject(this.failure)
    if (cursor !== null && cursor.generation !== this.generation) return Promise.reject(new Error('Stale history cursor generation'))
    if (cursor !== null && (cursor.revision !== this.revision || cursor.next_position > this.entries.length)) {
      return Promise.reject(new Error('History read refused: StaleCursor'))
    }
    const start = cursor?.next_position ?? 0
    const entries = this.entries.slice(start, start + this.pageSize)
    const nextPosition = start + entries.length
    const next: HistoryCursor | null = nextPosition < this.entries.length
      ? { generation: this.generation, conversation: FAKE_CONVERSATION, revision: this.revision, next_position: nextPosition }
      : null
    return Promise.resolve({
      conversation: FAKE_CONVERSATION,
      generation: this.generation,
      status: this.status,
      receipt: null,
      error: null,
      cancelling: false,
      history: { entries, next, completeness: 'Complete' },
      approvals: this.approvals,
      answered_approvals: this.answeredApprovals,
    })
  }
}
