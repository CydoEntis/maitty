import { act, render, screen, waitFor } from '@testing-library/react'
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
  label: '', pinned: false, archived: false, model: '', said: '', branch: null, linked: false, subagents: [],
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
  const row = (await screen.findAllByRole('button', { name: /build · / })).find((button) => !button.classList.contains('mx-group-head'))
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

  it('says Inspect is local-only, from the shortcut and the disabled Repository button', async () => {
    const user = await openRemotePane()
    const button = screen.getByRole('button', { name: 'Repository' })
    expect(button).toHaveAttribute('aria-disabled', 'true')
    expect(button).toHaveAttribute('title', REFUSAL('Inspect'))
    await user.click(button)
    expect(await screen.findByText(REFUSAL('Inspect'))).toBeInTheDocument()
    expect(screen.queryByRole('complementary', { name: /repository|inspect/i })).toBeNull()
  })

  it('says Worktrees is local-only and opens no dialog', async () => {
    const user = await openRemotePane()
    await user.keyboard('{Control>}{Shift>}n{/Shift}{/Control}')
    expect(await screen.findByText(REFUSAL('Worktrees'))).toBeInTheDocument()
    expect(screen.queryByRole('dialog', { name: /worktree/i })).toBeNull()
  })

  it('does not type a dropped file into it and says why', async () => {
    await openRemotePane()
    document.elementFromPoint = vi.fn(() => null)
    act(() => { terminal.drop.current?.(['/home/casey/a.png'], { x: 1, y: 1 }) })
    expect(await screen.findByText(REFUSAL('Dropping files'))).toBeInTheDocument()
    await waitFor(() => { expect(terminal.pasted).toEqual([]) })
  })
})
