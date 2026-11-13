import { act, render, screen, waitFor, within } from '@testing-library/react'
import { QueryClient, QueryClientProvider } from '@tanstack/react-query'
import userEvent from '@testing-library/user-event'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import App from './App'

const daemon = vi.hoisted(() => ({
  loadConversations: vi.fn<() => Promise<Conversation[]>>(),
  listenSessionsChanged: vi.fn(() => Promise.resolve(() => {})),
  defaultSessionFolder: vi.fn<() => Promise<string>>(),
  loadDaemonRestartStatus: vi.fn(),
  suggestProjectFolders: vi.fn<(typed: string) => Promise<string[]>>(),
  loadMachines: vi.fn(() => Promise.resolve({ names: ['build'], error: null })),
}))

vi.mock('@/api/daemon', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/daemon')>()),
  ...daemon,
  isDesktop: () => true,
}))

const terminal = vi.hoisted(() => ({
  pasted: [] as Array<[string, string]>,
  drop: { current: null as null | ((paths: string[], at: { x: number; y: number }) => void) },
}))

const remoteTerm = {
  id: 7, cols: 80, rows: 24, title: 't', process: 'bash', agent: null, status: 'Idle', cwd: '/srv/app', summary: '', name: 'job',
  label: '', pinned: false, archived: false, model: '', said: '', branch: null, linked: false, helperAsk: [], subagents: [],
  conversation_id: null, popup: false, remote: null, root: '/srv/app', top: null, since: 0, asleep: false, win32_input: false,
}

// One other machine, build, that is up and has one session.
vi.mock('@/api/machines', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/machines')>()),
  loadMachinesOverview: () => Promise.resolve([{
    machine: 1, name: 'build', state: 'connected', why: null, attempt: null,
    snapshot: { terms: { '7': remoteTerm }, workspaces: [], conversations: [] },
  }]),
  listenMachineState: () => Promise.resolve(() => undefined),
  listenMachineSessions: () => Promise.resolve(() => undefined),
  listenMachineError: () => Promise.resolve(() => undefined),
  listenMachineRemoved: () => Promise.resolve(() => undefined),
  listenMachineNotice: () => Promise.resolve(() => undefined),
  listenAttention: () => Promise.resolve(() => undefined),
  listenMachineClipboard: () => Promise.resolve(() => undefined),
}))

const sends = vi.hoisted(() => ({
  send: vi.fn<(machine: string, paths: readonly string[], transfer: string) => Promise<void>>(),
  emit: { current: null as null | ((event: unknown) => void) },
}))

vi.mock('@/api/fileSends', async (importOriginal) => ({
  ...(await importOriginal<typeof import('@/api/fileSends')>()),
  sendFilesToMachine: sends.send,
  listenFileSends: (onEvent: (event: unknown) => void) => {
    sends.emit.current = onEvent
    return Promise.resolve(() => undefined)
  },
}))

vi.mock('@/features/terminal/useFileDrop', () => ({
  useFileDrop: (onDrop: (paths: string[], at: { x: number; y: number }) => void) => { terminal.drop.current = onDrop },
}))

vi.mock('./TerminalView', () => ({
  TerminalView: ({ conversation, label, onHandle }: { conversation: Conversation; label: string; onHandle?: (handle: { find: () => boolean; clearFind: () => void; focus: () => void; paste: (text: string) => void } | null) => void }) => (
    <div ref={() => { onHandle?.({ find: () => false, clearFind: () => {}, focus: () => {}, paste: (text) => { terminal.pasted.push([conversation.id, text]) } }) }} role="region" aria-label={label} tabIndex={0}>
      Pane {conversation.nativePane}
    </div>
  ),
}))

const REFUSAL = (feature: string) => `${feature} is local-only in v1: the files are on build, not on this computer.`

async function openRemotePane() {
  window.localStorage.setItem('maitty.gui.welcomed.v1', '1')
  const client = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  render(<QueryClientProvider client={client}><App /></QueryClientProvider>)
  const user = userEvent.setup()
  // The only session there is runs on build, so the first row of the list is its row.
  const item = (await within(await screen.findByRole('navigation', { name: 'Sessions' })).findAllByRole('listitem'))[0]
  const row = item === undefined ? undefined : within(item).getAllByRole('button')[0]
  if (row === undefined) throw new Error('the session row of build was not shown')
  await user.click(row)
  await screen.findByRole('region', { name: /Terminal 1/ })
  return user
}

describe('a pane of another machine and the features that need its files', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    window.localStorage.clear()
    terminal.pasted.length = 0
    daemon.defaultSessionFolder.mockResolvedValue('/home/casey')
    daemon.loadConversations.mockResolvedValue([])
    daemon.suggestProjectFolders.mockResolvedValue([])
  })

  it('opens the side panel on the machine\'s changes, and its views that read files say they are local-only', async () => {
    const user = await openRemotePane()
    await user.click(screen.getByRole('button', { name: 'More pane actions' }))
    await user.click(await screen.findByRole('menuitem', { name: /^Changes/ }))
    const panel = within(await screen.findByRole('complementary', { name: 'Side panel' }))
    expect(panel.getByRole('tab', { name: /^Changes/ })).toHaveAttribute('aria-selected', 'true')
    expect(screen.queryByText(REFUSAL('Inspect'))).toBeNull()
    for (const view of ['Files', 'Branches', 'History', 'References']) {
      await user.click(panel.getByRole('tab', { name: view }))
      expect(await panel.findByText(REFUSAL(view))).toBeInTheDocument()
    }
  })

  it('says Worktrees is local-only and opens no dialog', async () => {
    const user = await openRemotePane()
    await user.keyboard('{Control>}{Shift>}n{/Shift}{/Control}')
    expect(await screen.findByText(REFUSAL('Worktrees'))).toBeInTheDocument()
    expect(screen.queryByRole('dialog', { name: /worktree/i })).toBeNull()
  })

  it('sends a dropped file to the machine and types the path it was kept at, not the path here', async () => {
    await openRemotePane()
    sends.send.mockResolvedValue(undefined)
    document.elementFromPoint = vi.fn(() => null)
    act(() => { terminal.drop.current?.(['/home/casey/a.png'], { x: 1, y: 1 }) })
    await waitFor(() => { expect(sends.send).toHaveBeenCalledTimes(1) })
    const [machine, paths, transfer] = sends.send.mock.calls[0] ?? []
    expect([machine, paths]).toEqual(['build', ['/home/casey/a.png']])
    expect(terminal.pasted, 'nothing is typed until the machine has the file').toEqual([])
    act(() => { sends.emit.current?.({ kind: 'sent', transfer, file: 'a.png', index: 0, count: 1, remotePath: '/home/dev/.cache/maitty/pastes/p-a.png', bytes: 1 }) })
    expect(terminal.pasted).toEqual([['build/t7', '/home/dev/.cache/maitty/pastes/p-a.png ']])
    act(() => { sends.emit.current?.({ kind: 'done', transfer, sent: 1, count: 1 }) })
    expect(await screen.findByText('Sent 1 file to build')).toBeInTheDocument()
  })

  it('refuses more than ten files for a machine and sends none', async () => {
    await openRemotePane()
    document.elementFromPoint = vi.fn(() => null)
    const eleven = Array.from({ length: 11 }, (_, n) => `/home/casey/f${String(n)}`)
    act(() => { terminal.drop.current?.(eleven, { x: 1, y: 1 }) })
    expect(await screen.findByText('At most 10 files can be sent at a time; 11 were dropped.')).toBeInTheDocument()
    expect(sends.send).not.toHaveBeenCalled()
  })
})
