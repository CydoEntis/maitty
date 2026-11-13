import { act, cleanup, render, screen, within } from '@testing-library/react'
import userEvent from '@testing-library/user-event'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '../fakeDaemonHistory'
import { forgetComposers } from '../composerStore'
import { forgetChatFeeds } from '../useChatFeed'
import { ChatSession } from './ChatSession'

const fake = vi.hoisted(() => ({
  current: null as null | { read: (cursor: never) => Promise<unknown> },
  submitMessage: vi.fn<(conversation: { id: string; generation: number }, operation: number, text: string) => Promise<unknown>>(),
  repoFindFiles: vi.fn<(term: number, query: string, limit: number) => Promise<{ paths: string[]; truncated: boolean }>>(),
  indexRemoteFiles: vi.fn<(machine: string, folder: string) => Promise<{ folder: string; files: string[]; truncated: boolean; hidden: number }>>(),
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
  repoFindFiles: fake.repoFindFiles,
}))

vi.mock('@/api/fileGets', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fileGets')>()),
  indexRemoteFiles: fake.indexRemoteFiles,
}))

const TERM = 7
const PLACE = { term: TERM, folder: '/dev/app/crates/core', checkout: '/dev/app' }
const SEARCH_WAIT_MS = 200
const REMOTE = { machine: 'build', folder: '/home/me/app', offline: false }

beforeEach(() => {
  vi.useFakeTimers({ shouldAdvanceTime: true })
  forgetChatFeeds()
  forgetComposers()
  fake.submitMessage.mockReset().mockResolvedValue({})
  fake.indexRemoteFiles.mockReset().mockResolvedValue({ folder: '/home/me/app', files: ['README.md', 'src/lib.rs', 'src/main.rs'], truncated: false, hidden: 0 })
  fake.repoFindFiles.mockReset().mockImplementation((_term, query) => Promise.resolve({
    paths: ['crates/core/src/lib.rs', 'crates/core/src/main.rs', 'docs/my notes.md'].filter((path) => path.includes(query)),
    truncated: false,
  }))
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.current = null
})

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

async function open(fileSearch: typeof PLACE | typeof REMOTE | null = PLACE) {
  const daemon = new FakeDaemonHistory()
  daemon.upsert(daemon.message('t1', 'u', 'User', 'earlier'))
  fake.current = daemon
  const user = userEvent.setup({ advanceTimers: (ms) => vi.advanceTimersByTime(ms) })
  render(<ChatSession link={{ conversation: FAKE_CONVERSATION, generation: 1 }} agent="claude" online {...(fileSearch === null ? {} : { fileSearch })} />)
  await settle()
  return { user, box: screen.getByRole('textbox', { name: 'Message claude' }) }
}

const listed = () => within(screen.getByRole('listbox', { name: 'Files' })).getAllByRole('option').map((option) => option.textContent)

describe('picking a file with @ in the message box', () => {
  it('lists the files the daemon finds for what follows the @, relative to the session folder', async () => {
    const { user, box } = await open()

    await user.type(box, 'look at @src')
    await settle(SEARCH_WAIT_MS)

    expect(fake.repoFindFiles).toHaveBeenLastCalledWith(TERM, 'src', 50)
    expect(listed()).toEqual(['src/lib.rs', 'src/main.rs'])
  })

  it('asks once for a burst of typing, not for every key', async () => {
    const { user, box } = await open()

    await user.type(box, '@main')
    await settle(SEARCH_WAIT_MS)

    expect(fake.repoFindFiles).toHaveBeenCalledOnce()
  })

  it('inserts the picked path and a space on Enter, sending nothing', async () => {
    const { user, box } = await open()

    await user.type(box, 'fix @src')
    await settle(SEARCH_WAIT_MS)
    await user.keyboard('{ArrowDown}{Enter}')

    expect(box).toHaveValue('fix src/main.rs ')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  it('puts the list away on Esc and keeps what was typed', async () => {
    const { user, box } = await open()

    await user.type(box, '@src')
    await settle(SEARCH_WAIT_MS)
    await user.keyboard('{Escape}')

    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(box).toHaveValue('@src')
  })

  it('opens nothing for an @ inside a word', async () => {
    const { user, box } = await open()

    await user.type(box, 'me@src')
    await settle(SEARCH_WAIT_MS)

    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(fake.repoFindFiles).not.toHaveBeenCalled()
  })

  it('opens nothing for a chat whose files cannot be asked for (no pane to ask)', async () => {
    const { user, box } = await open(null)

    await user.type(box, '@src')
    await settle(SEARCH_WAIT_MS)

    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(fake.repoFindFiles).not.toHaveBeenCalled()
  })

  it('never shows an answer for an earlier name over the one for the name typed now', async () => {
    let answerEarlier: (reply: { paths: string[]; truncated: boolean }) => void = () => undefined
    fake.repoFindFiles.mockImplementationOnce(() => new Promise((resolve) => { answerEarlier = resolve }))
    const { user, box } = await open()

    await user.type(box, '@src')
    await settle(SEARCH_WAIT_MS)
    await user.type(box, '/main')
    await settle(SEARCH_WAIT_MS)
    answerEarlier({ paths: ['crates/core/src/lib.rs'], truncated: false })
    await settle()

    expect(listed()).toEqual(['src/main.rs'])
  })

  it('takes no Enter or Tab while the answer for the name typed now has not come, sending and inserting nothing', async () => {
    const { user, box } = await open()

    await user.type(box, 'fix @src')
    await settle(SEARCH_WAIT_MS)
    await user.keyboard('{Enter}')
    expect(box).toHaveValue('fix src/lib.rs ')

    fake.repoFindFiles.mockImplementation(() => new Promise(() => undefined))
    await user.type(box, '@main{Enter}{Tab}')

    expect(box).toHaveValue('fix src/lib.rs @main')
    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(screen.getByText('Looking for files…')).toBeInTheDocument()
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  it('shows no rows of an earlier name while the answer for the name typed now is on its way', async () => {
    const { user, box } = await open()

    await user.type(box, '@src')
    await settle(SEARCH_WAIT_MS)
    fake.repoFindFiles.mockImplementation(() => new Promise(() => undefined))
    await user.type(box, '/m')

    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    expect(screen.getByText('Looking for files…')).toBeInTheDocument()
  })

  it('closes when the caret leaves the end of the text, and gives the keys back to the box', async () => {
    const { user, box } = await open()

    await user.type(box, 'see @src')
    await settle(SEARCH_WAIT_MS)
    expect(listed()).toEqual(['src/lib.rs', 'src/main.rs'])
    await user.keyboard('{ArrowLeft}')

    expect(screen.queryByRole('listbox')).not.toBeInTheDocument()
    await user.keyboard('{Enter}')
    expect(fake.submitMessage).toHaveBeenCalledOnce()
  })

  it('says so in one quiet line when the search fails, as with an older Maitty service', async () => {
    fake.repoFindFiles.mockRejectedValue(new Error('the Maitty service running is older than this app'))
    const { user, box } = await open()

    await user.type(box, '@src')
    await settle(SEARCH_WAIT_MS)

    expect(screen.getByText('Could not search the files: the Maitty service running is older than this app')).toBeInTheDocument()
    expect(screen.queryByRole('alert')).not.toBeInTheDocument()
  })

  it('says when the folder was too large to search whole', async () => {
    fake.repoFindFiles.mockResolvedValue({ paths: ['crates/core/src/lib.rs'], truncated: true })
    const { user, box } = await open()

    await user.type(box, '@lib')
    await settle(SEARCH_WAIT_MS)

    expect(screen.getByText(/too large to search whole/)).toBeInTheDocument()
  })
})

describe('picking a file with @ in a chat on another machine', () => {
  it('lists the paths of that machine’s folder index that match the name', async () => {
    const { user, box } = await open(REMOTE)

    await user.type(box, 'look at @main')
    await settle(SEARCH_WAIT_MS)

    expect(fake.indexRemoteFiles).toHaveBeenCalledWith('build', '/home/me/app')
    expect(fake.repoFindFiles).not.toHaveBeenCalled()
    expect(listed()).toEqual(['src/main.rs'])
  })

  it('indexes the folder once while the list is open, not for every key', async () => {
    const { user, box } = await open(REMOTE)

    await user.type(box, '@s')
    await settle(SEARCH_WAIT_MS)
    await user.type(box, 'rc/l')
    await settle(SEARCH_WAIT_MS)

    expect(fake.indexRemoteFiles).toHaveBeenCalledOnce()
    expect(listed()).toEqual(['src/lib.rs'])
  })

  it('takes no Enter while the index is on its way', async () => {
    fake.indexRemoteFiles.mockImplementation(() => new Promise(() => undefined))
    const { user, box } = await open(REMOTE)

    await user.type(box, 'fix @main{Enter}')

    expect(box).toHaveValue('fix @main')
    expect(screen.getByText('Looking for files…')).toBeInTheDocument()
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  it('says Machine offline, not an error, when the machine is offline', async () => {
    const { user, box } = await open({ ...REMOTE, offline: true })

    await user.type(box, '@main')
    await settle(SEARCH_WAIT_MS)

    expect(screen.getByText('Machine offline')).toBeInTheDocument()
    expect(screen.queryByText(/Could not search/)).not.toBeInTheDocument()
    expect(fake.indexRemoteFiles).not.toHaveBeenCalled()
  })
})
