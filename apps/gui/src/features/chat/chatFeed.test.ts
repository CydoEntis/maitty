import { describe, expect, it } from 'vitest'
import { afterFailure, createFeed, syncFeed } from './chatFeed'
import { FAKE_CONVERSATION, FakeDaemonHistory } from './fakeDaemonHistory'
import { selectChatEvents } from './projection'

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
  it('pages the whole history on the first read and remembers where the last page began', async () => {
    const daemon = conversationOf(5, 2)
    const feed = await syncFeed(newFeed(), daemon.read, now)

    expect(selectChatEvents(feed.projection)).toHaveLength(5)
    expect(daemon.reads.map((cursor) => cursor?.next_position ?? null)).toEqual([null, 2, 4])
    expect(feed.resume?.next_position).toBe(4)
    expect(feed.loaded).toBe(true)
  })

  it('asks only for the last page when nothing changed, and keeps the events as they were', async () => {
    const daemon = conversationOf(5, 2)
    const first = await syncFeed(newFeed(), daemon.read, now)
    daemon.reads.length = 0

    const second = await syncFeed(first, daemon.read, now)

    expect(daemon.reads).toHaveLength(1)
    expect(daemon.reads[0]?.next_position).toBe(4)
    expect(second.projection.turns).toBe(first.projection.turns)
    expect(selectChatEvents(second.projection)).toHaveLength(5)
  })

  it('applies only the new entries when history grows, with no duplicates', async () => {
    const daemon = conversationOf(3, 2)
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
