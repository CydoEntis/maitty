import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import { act, cleanup, fireEvent, render, screen, within } from '@testing-library/react'
import { useState, type ReactNode } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { ConversationDiff } from '@/api/daemon'
import { forgetComposers } from '@/features/chat/composerStore'
import { FAKE_CONVERSATION, FakeDaemonHistory } from '@/features/chat/fakeDaemonHistory'
import { forgetChatFeeds } from '@/features/chat/useChatFeed'
import { forgetComments } from './commentStore'
import { SessionPanel } from './SessionPanel'

const fake = vi.hoisted(() => ({
  history: null as null | { read: (cursor: never) => Promise<unknown> },
  file: vi.fn<(path: string) => Promise<string>>(),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  readConversationPage: (_id: number, cursor: unknown) => {
    if (fake.history === null) throw new Error('no fake daemon')
    return fake.history.read(cursor as never)
  },
  loadTargetDiff: () => Promise.resolve(DIFF),
  loadTargetFile: (_target: unknown, path: string) => fake.file(path),
}))

const DIFF: ConversationDiff = {
  files: [{ path: 'src/app.ts', additions: 1, deletions: 1, patch: '@@ -1 +1 @@\n-a\n+b', binary: false }],
  additions: 1,
  deletions: 1,
  total_files: 1,
  truncated: false,
}

type Entry = Record<string, unknown>
const tool = (item: string, label: string, input: unknown, output = ''): Entry => ({
  source: { conversation: FAKE_CONVERSATION, turn: 't1', item },
  content: { Tool: { kind: 'commandExecution', label, output, input } },
  state: 'Completed',
})

const ready = { kind: 'ready', agent: 'claude', link: { conversation: FAKE_CONVERSATION, generation: 1 } } as const
let opened: string[] = []

function Providers({ children }: { children: ReactNode }) {
  const [client] = useState(() => new QueryClient({ defaultOptions: { queries: { retry: false } } }))
  return <QueryClientProvider client={client}>{children}</QueryClientProvider>
}

function open() {
  const history = new FakeDaemonHistory()
  history.upsert(tool('r1', 'Read ', { file_path: '/repo/README.md' }))
  history.upsert(tool('e1', 'Edit ', { file_path: '/repo/src/app.ts', old_string: 'a', new_string: 'b' }))
  history.upsert(tool('w1', 'Write ', { file_path: '/repo/docs/plan.md', content: '# Plan\n\n- **step** one' }))
  history.upsert(tool('w2', 'Write ', { file_path: '/repo/shots/home.png', content: 'binary-ish' }))
  history.upsert(tool('w3', 'Write ', { file_path: '/repo/out/data.json', content: '{"a":1}' }))
  fake.history = history
  render(
    <Providers>
      <SessionPanel mode="docked" title="t" process="claude" availability={ready} root="/repo" folder="/repo" branch={null} online chatOpen width={320} onWide={() => undefined} onOpenInFiles={(file) => { opened.push(file) }} onClose={() => undefined} />
    </Providers>,
  )
}

async function settle(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

async function segment(name: RegExp) {
  await settle()
  fireEvent.mouseDown(screen.getByRole('tab', { name }), { button: 0, ctrlKey: false })
  fireEvent.click(screen.getByRole('tab', { name }))
  await settle()
}

beforeEach(() => {
  vi.useFakeTimers()
  forgetChatFeeds()
  forgetComposers()
  forgetComments()
  fake.file.mockReset()
  opened = []
})

afterEach(() => {
  cleanup()
  vi.useRealTimers()
  fake.history = null
})

describe('the Files segment', () => {
  it('counts the segments and lists the touched files with their kind', async () => {
    open()
    await segment(/^Files/)

    expect(screen.getByRole('tab', { name: 'Files · 5' })).toBeInTheDocument()
    expect(screen.getByRole('tab', { name: 'Artifacts · 3' })).toBeInTheDocument()
    const list = screen.getByRole('list', { name: 'Files this session touched' })
    const rows = within(list).getAllByRole('listitem').map((row) => row.textContent)
    expect(rows).toEqual(expect.arrayContaining([expect.stringContaining('ReadREADME.md'), expect.stringContaining('Editedsrc/app.ts'), expect.stringContaining('Createddocs/plan.md')]))
  })

  it('opens an edited file\'s diff and a read file in the Files drawer', async () => {
    open()
    await segment(/^Files/)

    fireEvent.click(screen.getByRole('button', { name: /^Read README\.md/ }))
    expect(opened).toEqual(['/repo/README.md'])

    fireEvent.click(screen.getByRole('button', { name: /^Edited src\/app\.ts/ }))
    await settle()
    expect(screen.getByRole('region', { name: 'Diff of src/app.ts' })).toBeInTheDocument()
  })

  it('opens a created file that has no diff now in the Files drawer', async () => {
    open()
    await segment(/^Files/)

    fireEvent.click(screen.getByRole('button', { name: /^Created docs\/plan\.md/ }))

    expect(opened).toEqual(['/repo/docs/plan.md'])
  })
})

describe('the Artifacts segment', () => {
  it('shows cards for the documents, data and pictures the agent created', async () => {
    open()
    await segment(/^Artifacts/)

    const list = screen.getByRole('list', { name: 'Artifacts' })
    expect(within(list).getByText('plan.md')).toBeInTheDocument()
    expect(within(list).getByText('Document · docs')).toBeInTheDocument()
    expect(within(list).getByText('data.json')).toBeInTheDocument()
    expect(within(list).getByText('home.png')).toBeInTheDocument()
  })

  it('reads a markdown artifact through the daemon and draws it safely', async () => {
    fake.file.mockResolvedValue('# Plan\n\n- **step** one\n\n<script>window.__artifact = 1</script>')
    open()
    await segment(/^Artifacts/)

    fireEvent.click(screen.getByRole('button', { name: /plan\.md/ }))
    await settle()

    expect(fake.file).toHaveBeenCalledWith('docs/plan.md')
    expect(screen.getByRole('heading', { name: 'Plan' })).toBeInTheDocument()
    expect(screen.getByText('step').tagName).toBe('STRONG')
    expect(document.querySelector('script')).toBeNull()
    expect(screen.getByText(/<script>window\.__artifact = 1<\/script>/)).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Back to artifacts' }))
    expect(screen.getByRole('list', { name: 'Artifacts' })).toBeInTheDocument()
  })

  it('shows other text as text, and says plainly when the daemon cannot read the file', async () => {
    fake.file.mockResolvedValueOnce('{"a":1}')
    open()
    await segment(/^Artifacts/)
    fireEvent.click(screen.getByRole('button', { name: /data\.json/ }))
    await settle()
    expect(screen.getByText('{"a":1}')).toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Back to artifacts' }))
    fake.file.mockRejectedValueOnce(new Error('This file is too large to preview'))
    fireEvent.click(screen.getByRole('button', { name: /plan\.md/ }))
    await settle()
    expect(screen.getByRole('alert')).toHaveTextContent('too large to preview')
  })

  it('does not pretend to show a picture, and offers the Files drawer', async () => {
    open()
    await segment(/^Artifacts/)

    fireEvent.click(screen.getByRole('button', { name: /home\.png/ }))

    expect(screen.getByText(/A picture cannot be shown here/)).toBeInTheDocument()
    expect(fake.file).not.toHaveBeenCalled()
    fireEvent.click(screen.getByRole('button', { name: 'Open in the Files drawer' }))
    expect(opened).toEqual(['/repo/shots/home.png'])
  })

  it('says so when the agent created nothing of the kind', async () => {
    const history = new FakeDaemonHistory()
    history.upsert(tool('r1', 'Read ', { file_path: '/repo/README.md' }))
    fake.history = history
    render(<Providers><SessionPanel mode="docked" title="t" process="claude" availability={ready} root="/repo" folder="/repo" branch={null} online chatOpen width={320} onWide={() => undefined} onOpenInFiles={() => undefined} onClose={() => undefined} /></Providers>)
    await segment(/^Artifacts/)

    expect(screen.getByText(/has not created a document/)).toBeInTheDocument()
  })
})
