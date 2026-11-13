import { renderHook } from '@testing-library/react'
import { QueryClient } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'

// The daemon's list: each agent started is a new terminal session in it, numbered in the order it was made.
const listed = vi.hoisted(() => ({ sessions: [] as { id: string; kind: 'terminal'; nativePane: number }[] }))
const daemon = vi.hoisted(() => ({
  startAgentSession: vi.fn<(folder: string, agent: string, words: readonly string[]) => Promise<void>>(() => {
    const pane = listed.sessions.length + 1
    listed.sessions.push({ id: String(pane), kind: 'terminal', nativePane: pane })
    return Promise.resolve()
  }),
  addWorktree: vi.fn<(folder: string, branch: string) => Promise<string>>((folder: string, branch: string) => Promise.resolve(`${folder}-${branch}`)),
  switchBranch: vi.fn(() => Promise.resolve()),
  renameConversationOnDaemon: vi.fn<(session: { id: string }, name: string) => Promise<void>>(() => Promise.resolve()),
  sendWhenAgentReady: vi.fn(() => Promise.resolve()),
}))
vi.mock('@/api/daemon', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/daemon')>()), ...daemon, loadConversations: () => Promise.resolve([...listed.sessions]) }))
const toast = vi.hoisted(() => ({ error: vi.fn(), message: vi.fn() }))
vi.mock('@/lib/toast', () => ({ toast }))
vi.mock('./model/agentHooks', () => ({ connectAgentOnce: () => Promise.resolve() }))

import { useSessionStarts } from './useSessionStarts'
import type { NewChatRequest } from './components/NewChatPage'

const ID = '0d0f5c1e-1111-4222-8333-444455556666'

type Watch = { setLayout?: () => void; setNewChatOpen?: (open: boolean) => void; setStartedFrom?: (change: unknown) => void }
function starts(watch: Watch = {}) {
  const setter = vi.fn()
  const props = {
    queryClient: new QueryClient(), sessions: [], setWorktreeFrom: setter, setLayout: watch.setLayout ?? setter, flash: setter, focusedSession: undefined,
    setNewChatOpen: watch.setNewChatOpen ?? setter, online: true, starting: false, setStarting: setter, setStartedFrom: watch.setStartedFrom ?? setter, sessionMap: new Map([['old', {}]]),
    preferences: { yoloClaude: false, yoloCodex: false, yoloGemini: false }, setExpectedAgents: setter, paneViews: { choose: setter },
    setSettingsSection: setter, setNewChatMachine: setter, setNewChatSeed: setter, home: '/home/me', machinesRef: { current: new Map() },
  } as unknown as Parameters<typeof useSessionStarts>[0]
  return renderHook(() => useSessionStarts(props)).result.current
}

const request = (patch: Partial<NewChatRequest>): NewChatRequest => ({
  folder: '/dev/app', agent: 'claude', model: 'default', reasoning: 'medium', mode: 'default', message: '', branch: null, machine: 0, machineName: 'This computer', resume: null, ...patch,
})

beforeEach(() => {
  listed.sessions = []
  for (const call of Object.values(daemon)) call.mockClear()
  toast.error.mockClear()
})

describe('starting a new chat that resumes a saved conversation', () => {
  it('puts Claude\'s resume option first, then the picked flags', async () => {
    await starts().startNewChat(request({ resume: ID }))
    expect(daemon.startAgentSession).toHaveBeenCalledWith('/dev/app', 'claude', ['--resume', ID, '--effort', 'medium'])
  })

  it('puts Codex\'s resume subcommand first, then the picked flags', async () => {
    await starts().startNewChat(request({ agent: 'codex', resume: ID }))
    expect(daemon.startAgentSession).toHaveBeenCalledWith('/dev/app', 'codex', ['resume', ID, '-c', 'model_reasoning_effort=medium'])
  })

  it('refuses to resume a Gemini chat', async () => {
    await expect(starts().startNewChat(request({ agent: 'gemini', resume: ID }))).rejects.toThrow('cannot be resumed')
    expect(daemon.startAgentSession).not.toHaveBeenCalled()
  })

  it('refuses an id that would be read as a flag before any worktree or branch is made', async () => {
    const start = starts()
    await expect(start.startNewChat(request({ resume: '--dangerously-skip-permissions', branch: { name: 'x', create: true, worktree: true } }))).rejects.toThrow('cannot be resumed')
    await expect(start.startNewChat(request({ resume: '-x', branch: { name: 'x', create: false, worktree: false } }))).rejects.toThrow('cannot be resumed')
    expect(daemon.addWorktree).not.toHaveBeenCalled()
    expect(daemon.switchBranch).not.toHaveBeenCalled()
    expect(daemon.startAgentSession).not.toHaveBeenCalled()
  })
})

describe('starting new chats in the background', () => {
  const inWorktree = (name: string) => ({ name, create: true, worktree: true })

  it('starts a second chat while the first is still making its worktree, and gives each its own session', async () => {
    let finishFirstTree: () => void = () => undefined
    daemon.addWorktree.mockImplementationOnce((folder, branch) => new Promise((resolve) => { finishFirstTree = () => { resolve(`${folder}-${branch}`) } }))
    const start = starts()
    const first = start.startNewChat(request({ message: 'first task', branch: inWorktree('bold-otter') }), { background: true })
    await start.startNewChat(request({ message: 'second task', branch: inWorktree('calm-heron') }), { background: true })
    finishFirstTree()
    await first

    expect(daemon.addWorktree.mock.calls).toEqual([['/dev/app', 'bold-otter'], ['/dev/app', 'calm-heron']])
    expect(daemon.startAgentSession.mock.calls.map(([folder]) => folder)).toEqual(['/dev/app-calm-heron', '/dev/app-bold-otter'])
    expect(daemon.renameConversationOnDaemon.mock.calls).toEqual([[{ id: '1' }, 'second task'], [{ id: '2' }, 'first task']])
  })

  it('matches each launch to its own session when two start at once', async () => {
    const start = starts()
    await Promise.all([
      start.startNewChat(request({ message: 'one' }), { background: true }),
      start.startNewChat(request({ message: 'two' }), { background: true }),
    ])
    expect(daemon.renameConversationOnDaemon.mock.calls).toEqual([[{ id: '1' }, 'one'], [{ id: '2' }, 'two']])
  })

  it('leaves the window and the page as they are', async () => {
    const setLayout = vi.fn()
    const setNewChatOpen = vi.fn()
    await starts({ setLayout, setNewChatOpen }).startNewChat(request({ message: 'quietly' }), { background: true })
    expect(daemon.startAgentSession).toHaveBeenCalledTimes(1)
    expect(setLayout).not.toHaveBeenCalled()
    expect(setNewChatOpen).not.toHaveBeenCalled()
  })

  it('opens a chat started the plain way and closes the page, as before', async () => {
    const setLayout = vi.fn()
    const setNewChatOpen = vi.fn()
    await starts({ setLayout, setNewChatOpen }).startNewChat(request({ message: 'open it' }))
    expect(setLayout).toHaveBeenCalled()
    expect(setNewChatOpen).toHaveBeenCalledWith(false)
  })

  it('refuses a start from elsewhere while a chat is starting', async () => {
    let finishLaunch: () => void = () => undefined
    daemon.startAgentSession.mockImplementationOnce(() => new Promise((resolve) => {
      finishLaunch = () => { listed.sessions.push({ id: '1', kind: 'terminal', nativePane: 1 }); resolve() }
    }))
    const start = starts()
    const chat = start.startNewChat(request({ message: 'busy' }), { background: true })
    await vi.waitFor(() => { expect(daemon.startAgentSession).toHaveBeenCalled() })
    await expect(start.runStart(() => Promise.resolve('/dev/app'), () => undefined)).resolves.toBe(false)
    finishLaunch()
    await chat
  })
})

describe('what a start in the background says, and what it hides', () => {
  /** Follows the sidebar's "sessions there were when a start began" as the hook sets it, keeping every value it took. */
  function startedFromState() {
    let now: ReadonlySet<string> | null = null
    const seen: (ReadonlySet<string> | null)[] = []
    const setStartedFrom = (change: unknown) => {
      now = typeof change === 'function' ? (change as (was: typeof now) => typeof now)(now) : (change as typeof now)
      seen.push(now)
    }
    return { setStartedFrom, seen }
  }

  it('gives the page the real reason a start failed, with no toast', async () => {
    daemon.startAgentSession.mockImplementationOnce(() => Promise.reject(new Error('claude was not found')))
    await expect(starts().startNewChat(request({ message: 'x' }), { background: true })).rejects.toThrow('claude was not found')
    expect(toast.error).not.toHaveBeenCalled()
  })

  it('says another start is running when one that runs alone is under way', async () => {
    let finishShell: () => void = () => undefined
    const start = starts()
    const shell = start.runStart(() => Promise.resolve('/dev/app'), () => undefined, () => new Promise((resolve) => {
      finishShell = () => { listed.sessions.push({ id: '1', kind: 'terminal', nativePane: 1 }); resolve() }
    }))
    await expect(start.startNewChat(request({ message: 'x' }), { background: true })).rejects.toThrow('Another start is running. Try again in a moment.')
    finishShell()
    await expect(shell).resolves.toBe(true)
  })

  it('hides no session from the sidebar while it starts', async () => {
    const { setStartedFrom, seen } = startedFromState()
    await starts({ setStartedFrom }).startNewChat(request({ message: 'quietly' }), { background: true })
    expect(seen.every((value) => value === null)).toBe(true)
  })

  it('still holds new sessions off the sidebar while a chat that opens its pane starts', async () => {
    const { setStartedFrom, seen } = startedFromState()
    await starts({ setStartedFrom }).startNewChat(request({ message: 'open it' }))
    expect(seen[0]).toEqual(new Set(['old']))
    expect(seen.at(-1)).toBeNull()
  })
})
