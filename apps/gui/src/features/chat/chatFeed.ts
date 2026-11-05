import type { ChatPage, HistoryCursor } from '@/api/daemon'
import type { ChatProvider } from './chatEvents'
import { applyAnsweredApprovals, applyApprovals, applyHistoryEntries, createChatProjection, type ChatProjection } from './projection'

/**
 * The chat view's live copy of one conversation: the projection plus where the daemon's history was
 * read up to. Pure and async-free except `syncFeed`, which takes the read as an argument, so the
 * paging, the cursor and the reconnect rules are tested without a daemon.
 *
 * How the daemon's history behaves decides the design. An entry is updated in place (a tool going
 * from Running to Completed, a message growing), so a position in the list is not enough to follow
 * it. Every page therefore carries a `resume` cursor, and a read from it returns only the entries
 * added or changed since: the feed keeps it and a long conversation costs one small read per poll.
 * When the history was rebuilt, or the conversation started over, the page says `reset` and the feed
 * starts again from it. A daemon that predates `resume` is followed the old way: the feed keeps the
 * cursor of the last page it read and asks for just that page again, and when the daemon says that
 * cursor is stale it reads the history from the start. Either way an entry already applied unchanged
 * is skipped, so a re-read adds nothing twice and leaves the events it did not change as the same
 * objects.
 */

export type ChatFeed = {
  readonly conversation: number
  readonly provider: ChatProvider
  /** `null` until the first page; a page of another generation starts the feed again. */
  readonly generation: number | null
  readonly projection: ChatProjection
  /** What was applied for each history entry, by its turn and item, to skip an entry that has not changed. */
  readonly seen: ReadonlyMap<string, string>
  /** Where the next read goes on: the daemon's `resume` cursor (only what changed), or the last page read. */
  readonly resume: HistoryCursor | null
  readonly status: ChatPage['status'] | null
  readonly error: string | null
  readonly cancelling: boolean
  readonly completeness: ChatPage['history']['completeness'] | null
  readonly receipt: ChatPage['receipt']
  /** Messages the daemon holds until the running turn ends, oldest first. */
  readonly queued: ChatPage['queued']
  /** At least one page has been read. */
  readonly loaded: boolean
}

export function createFeed(provider: ChatProvider, conversation: number): ChatFeed {
  return {
    conversation,
    provider,
    generation: null,
    projection: createChatProjection(provider),
    seen: new Map(),
    resume: null,
    status: null,
    error: null,
    cancelling: false,
    completeness: null,
    receipt: null,
    queued: [],
    loaded: false,
  }
}

function keyOf(raw: unknown, signature: string): string {
  if (typeof raw === 'object' && raw !== null && 'source' in raw) {
    const source = raw.source
    if (typeof source === 'object' && source !== null && 'turn' in source && 'item' in source && typeof source.turn === 'string' && typeof source.item === 'string') {
      return `${source.turn}:${source.item}`
    }
  }
  // An entry with no readable address is still kept out of every later read once it was seen.
  return `raw:${signature}`
}

/** Applies one page of history entries. A page of another generation discards what the feed held. */
export function applyPage(feed: ChatFeed, page: ChatPage, nowMs: number): ChatFeed {
  // A page that says `reset` could not follow the cursor (the history was rebuilt): what was held is void.
  const startsOver = (feed.generation !== null && feed.generation !== page.generation) || page.history.reset === true
  const base = startsOver ? createFeed(feed.provider, feed.conversation) : feed
  const seen = new Map(base.seen)
  const fresh: unknown[] = []
  for (const raw of page.history.entries) {
    const signature = JSON.stringify(raw)
    const key = keyOf(raw, signature)
    if (seen.get(key) === signature) continue
    seen.set(key, signature)
    fresh.push(raw)
  }
  return {
    ...base,
    generation: page.generation,
    projection: fresh.length > 0 ? applyHistoryEntries(base.projection, fresh, nowMs) : base.projection,
    seen,
    status: page.status,
    error: page.error,
    cancelling: page.cancelling,
    completeness: page.history.completeness,
    receipt: page.receipt,
    queued: page.queued,
    loaded: true,
  }
}

/** What the feed keeps after a failed read: everything, but the next read starts from the beginning. */
export function afterFailure(feed: ChatFeed): ChatFeed {
  return feed.resume === null ? feed : { ...feed, resume: null }
}

/** A pending approval the person has not answered yet. */
export function hasPendingApproval(feed: ChatFeed): boolean {
  return feed.projection.approvals.some((approval) => approval.resolution.status === 'pending')
}

/** When the running turn started (the daemon's time when it sent one), or `null` when no turn is running. */
export function runningTurnStartedAt(feed: ChatFeed): number | null {
  for (const turnId of [...feed.projection.turnOrder].reverse()) {
    const turn = feed.projection.turns.get(turnId)
    if (turn && (turn.status === null || turn.status === 'running')) return turn.startedAtMs
  }
  return null
}

const STALE_CURSOR = /stale\s*(?:history\s+)?cursor/i

/** The daemon refused a cursor because the history changed, or its generation did. */
export function isStaleCursor(error: unknown): boolean {
  return STALE_CURSOR.test(error instanceof Error ? error.message : String(error))
}

export type ReadPage = (cursor: HistoryCursor | null) => Promise<ChatPage>

/** The most pages one sync reads; a conversation longer than this many pages is read over several syncs. */
const MAX_PAGES_PER_SYNC = 400

/**
 * Reads what is new since the last sync and returns the updated feed. Throws when the daemon cannot
 * be read; the feed it was given is untouched, so the caller keeps showing it and tries again.
 */
export async function syncFeed(start: ChatFeed, read: ReadPage, now: () => number): Promise<ChatFeed> {
  let from = start.resume
  let page: ChatPage
  try {
    page = await read(from)
  } catch (error) {
    if (from === null || !isStaleCursor(error)) throw error
    from = null
    page = await read(null)
  }

  let feed = applyPage(start, page, now())
  // Without the daemon's `resume` (an older daemon) the feed asks again for the last page it read.
  let lastPageStart = from
  let unfinished = false
  const visited = new Set<string>()
  for (let pages = 1; page.history.next; pages += 1) {
    const next = page.history.next
    if (next.conversation !== start.conversation || next.generation !== page.generation) {
      throw new Error('Conversation history cursor belongs to a different conversation')
    }
    const key = JSON.stringify(next)
    if (visited.has(key)) throw new Error('Conversation history cursor repeated')
    visited.add(key)
    if (pages >= MAX_PAGES_PER_SYNC) {
      // The rest is read by the next sync, from where this one stopped.
      unfinished = true
      lastPageStart = next
      break
    }

    lastPageStart = next
    const following = await read(next)
    if (following.generation !== page.generation) throw new Error('Conversation changed while its history was being read')
    page = following
    feed = applyPage(feed, page, now())
  }

  // The daemon's `resume` returns only what changed after this read, however long the history is.
  const resume = unfinished ? lastPageStart : page.history.resume ?? lastPageStart
  // The approvals come with every page and are the daemon's list of those still waiting: the last one is current.
  const projection = applyAnsweredApprovals(applyApprovals(feed.projection, page.approvals), page.answered_approvals)
  return { ...feed, projection, resume }
}
