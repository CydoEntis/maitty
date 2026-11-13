import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetChatFeeds } from '../useChatFeed'
import { forgetComposers, peekComposer } from '../composerStore'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
  submitMessage: vi.fn<(conversation: { id: string; generation: number }, operation: number, text: string) => Promise<unknown>>(),
  repoSnapshots: vi.fn<(term: number) => Promise<unknown>>(),
  restoreCount: vi.fn<(term: number, commit: string) => Promise<number>>(),
  restoreSnapshot: vi.fn<(term: number, commit: string) => Promise<number>>(),
  toastError: vi.fn<(text: string) => void>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  submitMessage: fake.submitMessage,
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.current === null) throw new Error('no fake daemon')
    return fake.current.read(cursor as never)
  },
}))

vi.mock('@/features/inspect/api', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/features/inspect/api')>()),
  repoSnapshots: fake.repoSnapshots,
  restoreCount: fake.restoreCount,
  restoreSnapshot: fake.restoreSnapshot,
}))

vi.mock('@/lib/toast', async (importOriginal) => {
  const actual = await importOriginal<typeof import('@/lib/toast')>()
  return { toast: Object.assign(actual.toast, { error: fake.toastError }) }
})

const TERM = 7
const SECOND = 1000
const link = { conversation: FAKE_CONVERSATION, generation: 1 }

/** Two turns: "Add a.txt" from 100 s to 130 s, "Add b.txt" from 150 s to 160 s, with a snapshot before and after each. */
function twoTurns(): FakeDaemonHistory {
  const daemon = new FakeDaemonHistory()
  daemon.upsert(daemon.message('t1', 'u1', 'User', 'Add a.txt'))
  daemon.upsert(daemon.message('t1', 'a1', 'Assistant', 'Added it.'))
  daemon.upsert(daemon.turnStatus('t1', 'Completed', { started_at_ms: 100 * SECOND, completed_at_ms: 130 * SECOND }))
  daemon.upsert(daemon.message('t2', 'u2', 'User', 'Add b.txt'))
  daemon.upsert(daemon.message('t2', 'a2', 'Assistant', 'Added that too.'))
  daemon.upsert(daemon.turnStatus('t2', 'Completed', { started_at_ms: 150 * SECOND, completed_at_ms: 160 * SECOND }))
  fake.current = daemon
  return daemon
}

const snapshots = [
  { commit: 'after2', at: 161, why: 'after claude\'s turn' },
  { commit: 'before2', at: 150, why: 'before claude\'s turn' },
  { commit: 'after1', at: 131, why: 'after claude\'s turn' },
  { commit: 'before1', at: 100, why: 'before claude\'s turn' },
]

beforeEach(() => {
  vi.useFakeTimers()
  forgetChatFeeds()
  forgetComposers()
  for (const mock of [fake.submitMessage, fake.repoSnapshots, fake.restoreCount, fake.restoreSnapshot, fake.toastError]) mock.mockReset()
  fake.submitMessage.mockResolvedValue({})
  fake.repoSnapshots.mockResolvedValue(snapshots)
  fake.restoreCount.mockResolvedValue(2)
  fake.restoreSnapshot.mockResolvedValue(2)
  Object.defineProperty(HTMLElement.prototype, 'clientHeight', { configurable: true, get: () => 600 })
  Object.defineProperty(HTMLElement.prototype, 'scrollTop', {
    configurable: true,
    get(this: Element) { return scrolled.get(this) ?? 0 },
    set(this: Element, value: number) { scrolled.set(this, value) },
  })
})

// jsdom lays nothing out: a scroll position that holds what is set.
const scrolled = new WeakMap<Element, number>()

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

async function open(place: { term: number } | { refusal: string } = { term: TERM }) {
  render(<ChatSession link={link} agent="claude" online rewind={place} />)
  await settle()
  await settle()
}

/** Opens a sent message's menu, as the keyboard does: jsdom sends no pointer events Radix reads. */
async function openMenu(message: string) {
  const row = screen.getByText(message).closest('.mx-msg-user')
  if (!(row instanceof HTMLElement)) throw new Error(`no row for ${message}`)
  fireEvent.keyDown(within(row).getByRole('button', { name: 'Message actions' }), { key: 'Enter' })
  await settle()
}

async function click(element: HTMLElement) {
  fireEvent.click(element)
  await settle()
}

async function editFromHere(message: string) {
  await openMenu(message)
  await click(screen.getByRole('menuitem', { name: 'Edit from here' }))
}

const box = () => screen.getByRole('textbox', { name: 'Message claude' })
const confirmButton = (name: string) => within(screen.getByRole('alertdialog')).getByRole('button', { name })

describe('Edit from here on a message the person sent', () => {
  it('asks first, naming how many files change, then restores the snapshot from before that turn and puts the message back unsent', async () => {
    twoTurns()
    await open()

    await editFromHere('Add a.txt')

    expect(fake.restoreCount).toHaveBeenCalledWith(TERM, 'before1')
    expect(fake.restoreSnapshot).not.toHaveBeenCalled()
    const dialog = screen.getByRole('alertdialog')
    expect(dialog).toHaveTextContent('2 files change back')
    expect(dialog).toHaveTextContent('"before a restore" snapshot')

    await click(within(dialog).getByRole('button', { name: 'Restore 2 files' }))

    expect(fake.restoreSnapshot).toHaveBeenCalledWith(TERM, 'before1')
    expect(box()).toHaveValue('Add a.txt')
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  it('starts the next message with one line telling the agent, and only that one', async () => {
    twoTurns()
    await open()
    await editFromHere('Add a.txt')
    await click(confirmButton('Restore 2 files'))

    await click(screen.getByRole('button', { name: 'Send message' }))
    const first = fake.submitMessage.mock.calls[0]?.[2] ?? ''
    expect(first.split('\n')).toHaveLength(2)
    expect(first).toMatch(/^\[Note: I restored the files .*"Add a\.txt".*\]\nAdd a\.txt$/)

    fireEvent.change(box(), { target: { value: 'and now c.txt' } })
    await click(screen.getByRole('button', { name: 'Send message' }))
    expect(fake.submitMessage.mock.calls[1]?.[2]).toBe('and now c.txt')
  })

  it('keeps a draft already in the box, under the message, and drops the note when the box is emptied', async () => {
    twoTurns()
    await open()
    fireEvent.change(box(), { target: { value: 'half a thought' } })
    await editFromHere('Add b.txt')

    expect(screen.getByRole('alertdialog')).toHaveTextContent('above what you have typed')
    await click(confirmButton('Restore 2 files'))
    expect(fake.restoreSnapshot).toHaveBeenCalledWith(TERM, 'before2')
    expect(box()).toHaveValue('Add b.txt\n\nhalf a thought')

    fireEvent.change(box(), { target: { value: '' } })
    expect(peekComposer(FAKE_CONVERSATION).note).toBeNull()
  })

  it('restores nothing when the person cancels', async () => {
    twoTurns()
    await open()
    await editFromHere('Add a.txt')
    await click(confirmButton('Cancel'))

    expect(fake.restoreSnapshot).not.toHaveBeenCalled()
    expect(box()).toHaveValue('')
  })

  it('offers no restore when the files a restore would change cannot be counted', async () => {
    twoTurns()
    fake.restoreCount.mockRejectedValue(new Error('git failed'))
    await open()
    await editFromHere('Add a.txt')

    expect(screen.queryByRole('alertdialog')).toBeNull()
    expect(fake.restoreSnapshot).not.toHaveBeenCalled()
    expect(fake.toastError).toHaveBeenCalledWith(expect.stringContaining('git failed'))
  })

  it('says why when no snapshot from before the message is kept', async () => {
    twoTurns()
    fake.repoSnapshots.mockResolvedValue(snapshots.slice(0, 2))
    await open()
    await editFromHere('Add a.txt')

    expect(screen.queryByRole('alertdialog')).toBeNull()
    expect(fake.restoreCount).not.toHaveBeenCalled()
    expect(fake.toastError).toHaveBeenCalledWith(expect.stringContaining('no snapshot from before this message'))
  })

  it('is greyed out, with the reason, while the agent works', async () => {
    twoTurns().status = 'Working'
    await open()
    await openMenu('Add b.txt')

    await settle()
    expect(screen.getByRole('menuitem', { name: 'Edit from here' })).toHaveAttribute('aria-disabled', 'true')
    expect(screen.getByText('claude is working. Edit from here when it finishes.')).toBeInTheDocument()
  })

  it('is greyed out with the local-only refusal for a session on another machine', async () => {
    twoTurns()
    await open({ refusal: 'Edit from here is local-only in v1: the files are on build, not on this computer.' })
    await openMenu('Add a.txt')

    await settle()
    expect(screen.getByRole('menuitem', { name: 'Edit from here' })).toHaveAttribute('aria-disabled', 'true')
    expect(screen.getByText(/local-only in v1/)).toBeInTheDocument()
    expect(fake.repoSnapshots).not.toHaveBeenCalled()
  })
})
