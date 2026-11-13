import { renderHook } from '@testing-library/react'
import { QueryClient } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'

const daemon = vi.hoisted(() => ({
  startAgentSession: vi.fn<(folder: string, agent: string, words: readonly string[]) => Promise<void>>(() => Promise.resolve()),
  addWorktree: vi.fn((folder: string) => Promise.resolve(`${folder}-tree`)),
  switchBranch: vi.fn(() => Promise.resolve()),
}))
vi.mock('@/api/daemon', async (importOriginal) => ({ ...(await importOriginal<typeof import('@/api/daemon')>()), ...daemon, loadConversations: () => Promise.resolve([]) }))
vi.mock('./model/agentHooks', () => ({ connectAgentOnce: () => Promise.resolve() }))

import { useSessionStarts } from './useSessionStarts'
import type { NewChatRequest } from './components/NewChatPage'

const ID = '0d0f5c1e-1111-4222-8333-444455556666'

function starts() {
  const setter = vi.fn()
  const props = {
    queryClient: new QueryClient(), sessions: [], setWorktreeFrom: setter, setLayout: setter, flash: setter, focusedSession: undefined,
    setNewChatOpen: setter, online: true, starting: false, setStarting: setter, setStartedFrom: setter, sessionMap: new Map(),
    preferences: { yoloClaude: false, yoloCodex: false, yoloGemini: false }, setExpectedAgents: setter, paneViews: { choose: setter },
    setSettingsSection: setter, setNewChatMachine: setter, setNewChatSeed: setter, home: '/home/me', machinesRef: { current: new Map() },
  } as unknown as Parameters<typeof useSessionStarts>[0]
  return renderHook(() => useSessionStarts(props)).result.current
}

const request = (patch: Partial<NewChatRequest>): NewChatRequest => ({
  folder: '/dev/app', agent: 'claude', model: 'default', reasoning: 'medium', mode: 'default', message: '', branch: null, machine: 0, machineName: 'This computer', resume: null, ...patch,
})

beforeEach(() => { for (const call of Object.values(daemon)) call.mockClear() })

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
