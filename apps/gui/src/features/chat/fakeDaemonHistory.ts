import type { ChatPage, HistoryCursor } from '@/api/daemon'

/**
 * A stand-in for the daemon's conversation history, for tests. It pages the way the daemon does:
 * `next` is set only while entries remain, a plain cursor from before the last change is refused as
 * stale, and every page carries a `resume` cursor that returns only the entries changed since.
 * With `legacy` set it behaves like a daemon that predates `resume`: no `resume`, and a cursor of
 * another generation is refused instead of answered with a reset.
 */

type Entry = Readonly<Record<string, unknown>>

export const FAKE_CONVERSATION = 5

export class FakeDaemonHistory {
  entries: Entry[] = []
  /** The revision at which each entry last changed. */
  changedAt: number[] = []
  /** Which run of the history this is; a rebuild starts a new one. */
  epoch = 1
  legacy = false
  revision = 1
  generation = 1
  status: ChatPage['status'] = 'Idle'
  approvals: unknown[] = []
  /** Messages held until the turn ends. */
  queued: Array<{ operation: number; text: string }> = []
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
    this.revision += 1
    if (at === -1) {
      this.entries.push(entry)
      this.changedAt.push(this.revision)
    } else {
      this.entries[at] = entry
      this.changedAt[at] = this.revision
    }
  }

  /** The history was rebuilt (a reconnect that dropped drafts): cursors from before it cannot be followed. */
  rebuild(): void {
    this.epoch += 1
    this.revision += 1
    this.changedAt = this.changedAt.map(() => this.revision)
  }

  /** The daemon restarted the conversation: a new generation, a new history. */
  restart(): void {
    this.generation += 1
    this.revision = 1
    this.entries = []
    this.changedAt = []
  }

  read = (requested: HistoryCursor | null): Promise<ChatPage> => {
    this.reads.push(requested)
    if (this.failure) return Promise.reject(this.failure)
    let cursor = requested
    let reset = false
    if (cursor !== null && cursor.generation !== this.generation) {
      if (this.legacy) return Promise.reject(new Error('Stale history cursor generation'))
      cursor = null
      reset = true
    }
    // The first page of a delta read fixes the revision its later pages report from.
    let since: { revision: number; snapshot: number } | null = null
    if (cursor?.delta === true) {
      if (cursor.epoch !== this.epoch || cursor.revision > this.revision) {
        cursor = null
        reset = true
      } else {
        since = { revision: cursor.revision, snapshot: cursor.snapshot ? cursor.snapshot : this.revision }
      }
    } else if (cursor !== null && (cursor.revision !== this.revision || cursor.next_position > this.entries.length)) {
      return Promise.reject(new Error('History read refused: StaleCursor'))
    }
    const wanted: number[] = []
    for (let index = cursor?.next_position ?? 0; index < this.entries.length; index += 1) {
      if (since === null || (this.changedAt[index] ?? 0) > since.revision) wanted.push(index)
    }
    const taken = wanted.slice(0, this.pageSize)
    const following = wanted[this.pageSize]
    const next: HistoryCursor | null = following === undefined
      ? null
      : {
          generation: this.generation,
          conversation: FAKE_CONVERSATION,
          revision: since?.revision ?? this.revision,
          next_position: following,
          epoch: this.epoch,
          delta: since !== null,
          snapshot: since?.snapshot ?? 0,
        }
    const resume: HistoryCursor = {
      generation: this.generation,
      conversation: FAKE_CONVERSATION,
      revision: since?.snapshot ?? this.revision,
      next_position: 0,
      epoch: this.epoch,
      delta: true,
      snapshot: 0,
    }
    return Promise.resolve({
      conversation: FAKE_CONVERSATION,
      generation: this.generation,
      status: this.status,
      receipt: null,
      error: null,
      cancelling: false,
      history: {
        entries: taken.map((index) => this.entries[index] as Entry),
        next,
        completeness: 'Complete',
        ...(this.legacy ? {} : { resume, reset }),
      },
      approvals: this.approvals,
      queued: this.queued.map((message) => ({ ...message })),
    })
  }
}
