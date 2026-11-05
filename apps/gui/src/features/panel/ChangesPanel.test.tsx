import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { useState, type ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ConversationDiff } from '@/api/daemon'
import { forgetComposers } from '@/features/chat/composerStore'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '@/features/chat/fakeDaemonHistory'
import { reserveOperation } from '@/features/chat/sendOperation'
import { forgetChatFeeds } from '@/features/chat/useChatFeed'
import { forgetComments } from './commentStore'
import { forgetDiffRequest, requestDiff } from './panelCommands'
import { SessionPanel } from './SessionPanel'
import { forgetCommentDelivery } from './useCommentDelivery'

const fake = vi.hoisted(() => ({
  history: null as null | { read: (cursor: never) => Promise<unknown> },
  diff: null as null | (() => Promise<unknown>),
  submitMessage: vi.fn<(conversation: { id: string; generation: number }, operation: number, text: string) => Promise<unknown>>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.history === null) throw new Error('no fake daemon')
    return fake.history.read(cursor as never)
  },
  loadTargetDiff: () => {
    if (fake.diff === null) throw new Error('no fake diff')
    return fake.diff()
  },
  submitMessage: fake.submitMessage,
  cancelTurn: vi.fn().mockResolvedValue(undefined),
}))

const PATCH = [
  'diff --git a/src/app.ts b/src/app.ts',
  '--- a/src/app.ts',
  '+++ b/src/app.ts',
  '@@ -1,3 +1,3 @@',
  ' import a',
  '-const old = 1',
  '+const count = 1',
  ' export {}',
].join('\n')

const DIFF: ConversationDiff = {
  files: [
    { path: 'src/app.ts', additions: 1, deletions: 1, patch: PATCH, binary: false },
    { path: 'docs/readme.md', additions: 3, deletions: 0, patch: '', binary: false },
    { path: 'logo.png', additions: 0, deletions: 0, patch: '', binary: true },
  ],
  additions: 4,
  deletions: 1,
  total_files: 3,
  truncated: false,
}

const link = { conversation: FAKE_CONVERSATION, generation: 1 }
const ready = { kind: 'ready', agent: 'claude', link } as const

let widths: boolean[] = []
let opened: string[] = []

function Providers({ children }: { children: ReactNode }) {
  const [client] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false } } }))
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

function open(options: { readOnly?: boolean; online?: boolean; status?: 'Idle' | 'Working' | 'Blocked'; approvals?: unknown[]; queued?: Array<{ operation: number; text: string }> } = {}) {
  const history = new FakeDaemonHistory()
  history.upsert(history.message('t1', 'u', 'User', 'hello'))
  history.status = options.status ?? 'Idle'
  history.approvals = options.approvals ?? []
  history.queued = options.queued ?? []
  fake.history = history
  fake.diff = () => Promise.resolve(DIFF)
  widths = []
  opened = []
  render(
    <Providers>
      <SessionPanel mode="docked" title="fix tests" process="claude" availability={options.readOnly === true ? { ...ready, link: { ...link, readOnly: true } } : ready} root="/repo" folder="/repo" onOpenInFiles={(file) => { opened.push(file) }} branch="feat/fix" online={options.online ?? true} width={320} onWide={(wide) => { widths.push(wide) }} onClose={() => undefined} />
    </Providers>,
  )
  return history
}

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

async function openFile(name: string) {
  await settle()
  fireEvent.click(screen.getByRole('button', { name: new RegExp(name.replace('.', '\\.')) }))
  await settle()
}

beforeEach(() => {
  vi.useFakeTimers()
  forgetChatFeeds()
  forgetComposers()
  forgetCommentDelivery()
  forgetComments()
  forgetDiffRequest()
  fake.submitMessage.mockReset()
  fake.submitMessage.mockResolvedValue({})
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.history = null
  fake.diff = null
})

describe('the Changes segment', () => {
  it('lists the worktree\'s changed files with the branch, counts and no invented staging', async () => {
    open()
    await settle()

    expect(screen.getByRole('tab', { name: 'Changes · 3' })).toBeInTheDocument()
    expect(screen.getByText('feat/fix')).toBeInTheDocument()
    const list = screen.getByRole('list', { name: 'Changed files' })
    expect(within(list).getAllByRole('listitem')).toHaveLength(3)
    expect(within(list).getByText('+3')).toBeInTheDocument()
    expect(screen.queryByText(/ahead|behind|pushed|Staged|Unstaged/)).toBeNull()
  })

  it('says there are no changes, and says when reading them failed', async () => {
    open()
    fake.diff = () => Promise.resolve({ ...DIFF, files: [], total_files: 0 })
    await settle(5000)
    expect(screen.getByText('No changes in this worktree.')).toBeInTheDocument()

    fake.diff = () => Promise.reject(new Error('the daemon has no working folder for this conversation'))
    await settle(5000)
    expect(screen.getByRole('alert')).toHaveTextContent('no working folder')
  })

  it('widens the panel while a diff is open and narrows it on Back', async () => {
    open()
    await openFile('src/app.ts')

    expect(widths.at(-1)).toBe(true)
    fireEvent.click(screen.getByRole('button', { name: 'Back to changes' }))
    await settle()
    expect(widths.at(-1)).toBe(false)
    expect(screen.getByRole('list', { name: 'Changed files' })).toBeInTheDocument()
  })

  it('says a binary file has no diff', async () => {
    open()
    await openFile('logo.png')

    expect(screen.getByText(/Binary file/)).toBeInTheDocument()
  })
})

describe('line comments', () => {
  async function comment(line: number, text: string, key: string = 'Enter') {
    fireEvent.click(screen.getByRole('button', { name: `Comment on line ${String(line)}` }))
    const box = screen.getByRole('textbox', { name: `Comment on line ${String(line)}` })
    fireEvent.change(box, { target: { value: text } })
    fireEvent.keyDown(box, { key })
    await settle()
  }

  it('numbers the lines of the file as it is now and puts no + on a removed line', async () => {
    open()
    await openFile('src/app.ts')

    expect(screen.getByRole('button', { name: 'Comment on line 1' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Comment on line 2' })).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Comment on line 3' })).toBeInTheDocument()
    expect(screen.getAllByRole('button', { name: /^Comment on line/ })).toHaveLength(3)
  })

  it('sends a comment as one message with its file, line, code and text, and marks the line', async () => {
    open()
    await openFile('src/app.ts')

    await comment(2, 'Why not just use a let?')

    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith({ id: String(FAKE_CONVERSATION), generation: 1 }, 1, 'Comment on src/app.ts:2\n> const count = 1\n\nWhy not just use a let?')
    expect(screen.getByRole('img', { name: /1 comment: Why not just use a let\?/ })).toBeInTheDocument()
    expect(screen.queryByRole('textbox', { name: /Comment on line/ })).toBeNull()
    fireEvent.click(screen.getByRole('button', { name: 'Back to changes' }))
    expect(screen.getByRole('img', { name: '1 comment' })).toBeInTheDocument()
  })

  it('sends on Ctrl+Enter, adds a line on Shift+Enter and ignores an empty comment', async () => {
    open()
    await openFile('src/app.ts')
    fireEvent.click(screen.getByRole('button', { name: 'Comment on line 1' }))
    const box = screen.getByRole('textbox', { name: 'Comment on line 1' })

    fireEvent.keyDown(box, { key: 'Enter' })
    fireEvent.keyDown(box, { key: 'Enter', shiftKey: true })
    expect(screen.getByRole('button', { name: /^Send to claude/ })).toBeDisabled()
    expect(fake.submitMessage).not.toHaveBeenCalled()

    fireEvent.change(box, { target: { value: 'ok' } })
    fireEvent.keyDown(box, { key: 'Enter', ctrlKey: true })
    await settle()
    expect(fake.submitMessage).toHaveBeenCalledOnce()
  })

  it('cancels with Cancel or Esc and sends nothing', async () => {
    open()
    await openFile('src/app.ts')

    fireEvent.click(screen.getByRole('button', { name: 'Comment on line 1' }))
    fireEvent.click(screen.getByRole('button', { name: 'Cancel' }))
    expect(screen.queryByRole('textbox', { name: /Comment on line/ })).toBeNull()

    fireEvent.click(screen.getByRole('button', { name: 'Comment on line 1' }))
    fireEvent.keyDown(screen.getByRole('textbox', { name: 'Comment on line 1' }), { key: 'Escape' })
    expect(screen.queryByRole('textbox', { name: /Comment on line/ })).toBeNull()
    await settle()
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })

  const CONVERSATION = { id: String(FAKE_CONVERSATION), generation: 1 }
  const approval = { request_id: 'r1', kind: 'Command', turn_id: 't1', command: 'rm -rf build', choices: ['Accept', 'Decline'] }

  it('never sends while an approval waits: the comment waits in the panel and goes after the answer', async () => {
    const history = open({ status: 'Blocked', approvals: [approval] })
    await openFile('src/app.ts')
    fireEvent.click(screen.getByRole('button', { name: 'Comment on line 1' }))
    expect(screen.getByText(/waiting for an approval. This comment is queued and sent after you answer it/)).toBeInTheDocument()

    await comment(1, 'Keep the import sorted.')
    await settle(5000)
    expect(fake.submitMessage).not.toHaveBeenCalled()

    history.approvals = []
    history.status = 'Idle'
    history.upsert(history.message('t1', 'u2', 'User', 'ping'))
    await settle(5000)
    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith(CONVERSATION, 1, 'Comment on src/app.ts:1\n> import a\n\nKeep the import sorted.')
  })

  it('sends at once while the agent works, since the daemon holds it until the turn ends', async () => {
    open({ status: 'Working' })
    await openFile('src/app.ts')
    fireEvent.click(screen.getByRole('button', { name: 'Comment on line 3' }))
    expect(screen.getByText(/queued and sent when it finishes this turn/)).toBeInTheDocument()

    await comment(3, 'Add a test.')
    await settle(5000)

    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith(CONVERSATION, 1, 'Comment on src/app.ts:3\n> export {}\n\nAdd a test.')
  })

  it('numbers a comment above what the daemon already holds', async () => {
    open({ status: 'Working', queued: [{ operation: 4, text: 'earlier' }] })
    await openFile('src/app.ts')

    await comment(1, 'Next.')

    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith(CONVERSATION, 5, expect.stringContaining('Next.'))
  })

  it('holds a comment while the daemon is offline and sends it once it is back', async () => {
    open({ online: false })
    await openFile('src/app.ts')
    fireEvent.click(screen.getByRole('button', { name: 'Comment on line 1' }))
    expect(screen.getByText(/The daemon is offline. This comment is queued until it is back/)).toBeInTheDocument()

    await comment(1, 'Later.')
    await settle(5000)
    expect(fake.submitMessage).not.toHaveBeenCalled()
    cleanup()

    open({ online: true })
    await settle(5000)
    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith(CONVERSATION, 1, 'Comment on src/app.ts:1\n> import a\n\nLater.')
  })

  it('sends a comment once through the daemon, whether or not the chat is open', async () => {
    open()
    await openFile('src/app.ts')

    await comment(1, 'Once only.')
    await settle(5000)

    expect(fake.submitMessage).toHaveBeenCalledExactlyOnceWith(CONVERSATION, 1, expect.stringContaining('Once only.'))
  })

  it('tries a failed send again with the same operation number', async () => {
    const history = open()
    fake.submitMessage.mockRejectedValueOnce(new Error('connection lost'))
    await openFile('src/app.ts')

    await comment(1, 'Again.')
    expect(fake.submitMessage).toHaveBeenCalledTimes(1)

    history.upsert(history.message('t1', 'u2', 'User', 'ping'))
    await settle(5000)

    expect(fake.submitMessage).toHaveBeenCalledTimes(2)
    expect(fake.submitMessage.mock.calls[1]).toEqual(fake.submitMessage.mock.calls[0])
  })

  it('takes a new operation number when the daemon calls the old one stale or reused', async () => {
    open()
    fake.submitMessage.mockImplementationOnce(() => {
      // Another sender (the chat's composer) used higher numbers before the daemon answered.
      reserveOperation(FAKE_CONVERSATION, { receipt: null, queued: [{ operation: 6 }] })
      return Promise.reject(new Error('operation is stale or reused'))
    })
    await openFile('src/app.ts')

    await comment(1, 'Renumber.')
    await settle(5000)

    expect(fake.submitMessage).toHaveBeenCalledTimes(2)
    expect(fake.submitMessage.mock.calls[0]?.[1]).toBe(1)
    expect(fake.submitMessage.mock.calls[1]?.[1]).toBe(8)
  })

  it('cannot comment in a read-only chat and says why', async () => {
    open({ readOnly: true })
    await openFile('src/app.ts')

    expect(screen.getByText('Read-only: talk to the agent in the terminal')).toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Comment on line 1' })).toBeDisabled()
    fireEvent.click(screen.getByRole('button', { name: 'Comment on line 1' }))
    expect(screen.queryByRole('textbox', { name: /Comment on line/ })).toBeNull()
    expect(fake.submitMessage).not.toHaveBeenCalled()
  })
})

describe('opening a file from an edit block', () => {
  it('shows that file\'s diff, matching an absolute path against the worktree', async () => {
    open()
    await settle()

    act(() => { requestDiff('/repo/src/app.ts') })
    await settle()

    expect(screen.getByRole('region', { name: 'Diff of src/app.ts' })).toBeInTheDocument()
  })

  it('says so when the file has no changes now', async () => {
    open()
    await settle()

    act(() => { requestDiff('/repo/src/gone.ts') })
    await settle()

    expect(screen.getByRole('status')).toHaveTextContent('src/gone.ts has no changes in the worktree now.')
    expect(screen.queryByRole('region', { name: /^Diff of/ })).toBeNull()
  })
})
