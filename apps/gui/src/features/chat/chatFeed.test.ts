import { describe, expect, it } from 'vitest'
import { afterFailure, createFeed, syncFeed } from './chatFeed'
import { FAKE_CONVERSATION, FakeDaemonHistory } from './fakeDaemonHistory'
import { selectChatEvents } from './projection'
import { conversationKey } from '@/api/conversationKey'

const NOW = 1_700_000_000_000
const now = () => NOW
const T1 = 'turn-1'
const T2 = 'turn-2'

const newFeed = () => createFeed('claude', FAKE_CONVERSATION)
const kinds = (feed: ReturnType<typeof newFeed>) => selectChatEvents(feed.projection).map((event) => `${event.kind}:${event.id}`)

function conversationOf(count: number, pageSize: number): FakeDaemonHistory {
  const daemon = new FakeDaemonHistory(pageSize)
  for (let index = 0; index < count; index += 1) daemon.upsert(daemon.message(T1, `m${String(index)}`, index % 2 === 0 ? 'User' : 'Assistant', `line ${String(index)}`))
  return daemon
}

describe('reading a conversation', () => {
  it('pages the whole history on the first read and, from a daemon without resume, remembers where the last page began', async () => {
    const daemon = conversationOf(5, 2)
    daemon.legacy = true
    const feed = await syncFeed(newFeed(), daemon.read, now)

    expect(selectChatEvents(feed.projection)).toHaveLength(5)
    expect(daemon.reads.map((cursor) => cursor?.next_position ?? null)).toEqual([null, 2, 4])
    expect(feed.resume?.next_position).toBe(4)
    expect(feed.loaded).toBe(true)
  })

  it('asks, of a daemon without resume, only for the last page when nothing changed, and keeps the events as they were', async () => {
    const daemon = conversationOf(5, 2)
    daemon.legacy = true
    const first = await syncFeed(newFeed(), daemon.read, now)
    daemon.reads.length = 0

    const second = await syncFeed(first, daemon.read, now)

    expect(daemon.reads).toHaveLength(1)
    expect(daemon.reads[0]?.next_position).toBe(4)
    expect(second.projection.turns).toBe(first.projection.turns)
    expect(selectChatEvents(second.projection)).toHaveLength(5)
  })

  it('applies only the new entries when a daemon without resume grows, with no duplicates', async () => {
    const daemon = conversationOf(3, 2)
    daemon.legacy = true
    const first = await syncFeed(newFeed(), daemon.read, now)
    const before = kinds(first)

    daemon.upsert(daemon.message(T1, 'm3', 'Assistant', 'one more'))
    const second = await syncFeed(first, daemon.read, now)

    // The cursor went stale with the change; the feed read from the start and kept what it knew.
    expect(daemon.reads.at(-1)?.next_position ?? null).not.toBeNull()
    expect(kinds(second)).toEqual([...before, 'message:turn-1:m3'])
  })

  it('updates an item in place when the daemon updates it, keeping its place in the order', async () => {
    const daemon = new FakeDaemonHistory()
    daemon.upsert(daemon.message(T1, 'u', 'User', 'run it'))
    daemon.upsert(daemon.tool(T1, 'c', 'ls', '', 'Running'))
    daemon.upsert(daemon.message(T1, 'a', 'Assistant', 'done'))
    const first = await syncFeed(newFeed(), daemon.read, now)
    expect(selectChatEvents(first.projection).find((event) => event.kind === 'command')).toMatchObject({ status: 'running' })

    daemon.upsert(daemon.tool(T1, 'c', 'ls', 'a.txt', 'Completed'))
    const second = await syncFeed(first, daemon.read, now)

    expect(kinds(second)).toEqual(kinds(first))
    expect(selectChatEvents(second.projection).find((event) => event.kind === 'command')).toMatchObject({ status: 'completed', output: 'a.txt' })
  })

  it('starts over when the conversation has a new generation', async () => {
    const daemon = conversationOf(3, 10)
    const first = await syncFeed(newFeed(), daemon.read, now)
    expect(selectChatEvents(first.projection)).toHaveLength(3)

    daemon.restart()
    daemon.upsert(daemon.message(T2, 'n0', 'User', 'a fresh start'))
    const second = await syncFeed(first, daemon.read, now)

    expect(kinds(second)).toEqual(['message:turn-2:n0'])
    expect(second.generation).toBe(2)
  })

  it('tracks pending approvals and marks one answered when the daemon stops listing it', async () => {
    const daemon = new FakeDaemonHistory()
    daemon.upsert(daemon.message(T1, 'u', 'User', 'go'))
    daemon.approvals = [{ request_id: 7, kind: 'Command', turn_id: T1, command: 'rm -rf build', choices: ['Accept', 'Decline'] }]
    const first = await syncFeed(newFeed(), daemon.read, now)
    expect(first.projection.approvals).toMatchObject([{ requestId: 7, resolution: { status: 'pending' } }])

    daemon.approvals = []
    const second = await syncFeed(first, daemon.read, now)
    expect(second.projection.approvals).toMatchObject([{ requestId: 7, resolution: { status: 'answered', answer: 'unknown' } }])
  })
})

describe('reading a conversation of another machine', () => {
  it('pages the whole history under its key while the daemon numbers it its own way', async () => {
    const daemon = conversationOf(5, 2)
    const key = conversationKey(2, FAKE_CONVERSATION)
    expect(key).not.toBeNull()

    const feed = await syncFeed(createFeed('claude', key ?? 0), daemon.read, now)

    expect(selectChatEvents(feed.projection)).toHaveLength(5)
  })
})

describe('following a long conversation by what changed', () => {
  it('reads the whole history once, then only what changed since, with the daemon\'s resume cursor', async () => {
    const daemon = conversationOf(5, 2)
    const first = await syncFeed(newFeed(), daemon.read, now)
    expect(daemon.reads.map((cursor) => cursor?.next_position ?? null)).toEqual([null, 2, 4])
    expect(first.resume).toMatchObject({ delta: true, next_position: 0 })
    daemon.reads.length = 0

    const quiet = await syncFeed(first, daemon.read, now)

    expect(daemon.reads).toHaveLength(1)
    expect(daemon.reads[0]).toMatchObject({ delta: true })
    expect(quiet.projection.turns).toBe(first.projection.turns)
    expect(selectChatEvents(quiet.projection)).toHaveLength(5)
  })

  it('keeps one small read per poll while a 1,000-entry conversation streams', async () => {
    const daemon = conversationOf(1000, 128)
    const first = await syncFeed(newFeed(), daemon.read, now)
    expect(daemon.reads).toHaveLength(8)
    daemon.reads.length = 0

    daemon.upsert(daemon.message(T1, 'live', 'Assistant', 'typing', 'Running'))
    let feed = await syncFeed(first, daemon.read, now)
    for (let step = 1; step <= 5; step += 1) {
      daemon.upsert(daemon.message(T1, 'live', 'Assistant', `typing${'.'.repeat(step)}`, 'Running'))
      feed = await syncFeed(feed, daemon.read, now)
    }

    expect(daemon.reads).toHaveLength(6)
    const live = selectChatEvents(feed.projection).find((event) => event.kind === 'message' && event.id === 'turn-1:live')
    expect(live).toMatchObject({ text: 'typing.....' })
    expect(selectChatEvents(feed.projection)).toHaveLength(1001)
  })

  it('applies an item changed in place and an added one, and no other event changes', async () => {
    const daemon = new FakeDaemonHistory()
    daemon.upsert(daemon.message(T1, 'u', 'User', 'run it'))
    daemon.upsert(daemon.tool(T1, 'c', 'ls', '', 'Running'))
    daemon.upsert(daemon.message(T1, 'a', 'Assistant', 'done'))
    const first = await syncFeed(newFeed(), daemon.read, now)
    const before = selectChatEvents(first.projection)

    daemon.upsert(daemon.tool(T1, 'c', 'ls', 'a.txt', 'Completed'))
    daemon.upsert(daemon.message(T1, 'b', 'Assistant', 'one more'))
    const second = await syncFeed(first, daemon.read, now)

    const after = selectChatEvents(second.projection)
    expect(after.find((event) => event.kind === 'command')).toMatchObject({ status: 'completed', output: 'a.txt' })
    expect(after.map((event) => event.id)).toEqual([...before.map((event) => event.id), 'turn-1:b'])
    expect(after[0]).toBe(before[0])
  })

  it('discards what it held when the daemon says the history was rebuilt', async () => {
    const daemon = conversationOf(4, 10)
    const first = await syncFeed(newFeed(), daemon.read, now)
    expect(selectChatEvents(first.projection)).toHaveLength(4)

    // A reconnect dropped an unfinished entry: the rows are not the ones the feed holds.
    daemon.entries.pop()
    daemon.changedAt.pop()
    daemon.rebuild()
    const second = await syncFeed(first, daemon.read, now)

    expect(selectChatEvents(second.projection)).toHaveLength(3)
    expect(second.resume).toMatchObject({ delta: true })
  })

  it('starts over on a new generation without an error, and then follows the new history', async () => {
    const daemon = conversationOf(3, 10)
    const first = await syncFeed(newFeed(), daemon.read, now)

    daemon.restart()
    daemon.upsert(daemon.message(T2, 'n0', 'User', 'a fresh start'))
    const second = await syncFeed(first, daemon.read, now)
    daemon.upsert(daemon.message(T2, 'n1', 'Assistant', 'hello'))
    const third = await syncFeed(second, daemon.read, now)

    expect(kinds(second)).toEqual(['message:turn-2:n0'])
    expect(kinds(third)).toEqual(['message:turn-2:n0', 'message:turn-2:n1'])
  })

  it('pages a delta read that is longer than a page and does not miss the last entries', async () => {
    const daemon = conversationOf(6, 2)
    const first = await syncFeed(newFeed(), daemon.read, now)
    for (let index = 6; index < 12; index += 1) daemon.upsert(daemon.message(T1, `m${String(index)}`, 'User', `late ${String(index)}`))

    const second = await syncFeed(first, daemon.read, now)

    expect(selectChatEvents(second.projection)).toHaveLength(12)
  })
})

describe('losing and regaining the daemon', () => {
  it('keeps what it showed when a read fails, and resumes with no duplicates once it works', async () => {
    const daemon = conversationOf(5, 2)
    const first = await syncFeed(newFeed(), daemon.read, now)
    const shown = kinds(first)

    daemon.failure = new Error('maittyd is not running')
    await expect(syncFeed(first, daemon.read, now)).rejects.toThrow('maittyd is not running')
    // The feed given in is untouched by a failed read.
    expect(kinds(first)).toEqual(shown)

    daemon.failure = null
    daemon.upsert(daemon.message(T1, 'm5', 'User', 'while you were away'))
    const back = await syncFeed(afterFailure(first), daemon.read, now)

    expect(daemon.reads.at(-1)?.next_position ?? null).not.toBeNull()
    expect(kinds(back)).toEqual([...shown, 'message:turn-1:m5'])
  })

  it('reads again from the start when it cannot tell the cursor is still good', async () => {
    const daemon = conversationOf(5, 2)
    const first = await syncFeed(newFeed(), daemon.read, now)
    daemon.reads.length = 0

    await syncFeed(afterFailure(first), daemon.read, now)

    expect(daemon.reads[0]).toBeNull()
  })

  it('refuses a cursor that repeats instead of reading forever', async () => {
    const daemon = conversationOf(4, 2)
    const loop = async () => {
      const page = await daemon.read(null)
      return { ...page, history: { ...page.history, next: page.history.next } }
    }
    await expect(syncFeed(newFeed(), () => loop(), now)).rejects.toThrow('cursor repeated')
  })

  it('leaves out an entry it cannot read without hiding the rest', async () => {
    const daemon = new FakeDaemonHistory()
    daemon.upsert(daemon.message(T1, 'u', 'User', 'hello'))
    daemon.upsert({ source: { conversation: FAKE_CONVERSATION, turn: T1, item: 'x' }, content: { Mystery: 1 }, state: 'Completed' })
    daemon.upsert(daemon.message(T1, 'a', 'Assistant', 'hi'))

    const first = await syncFeed(newFeed(), daemon.read, now)
    expect(selectChatEvents(first.projection)).toHaveLength(2)
    expect(first.projection.rejectedInputs).toBe(1)

    // Read again: the unreadable entry is not counted a second time.
    const second = await syncFeed(first, daemon.read, now)
    expect(second.projection.rejectedInputs).toBe(1)
  })
})
