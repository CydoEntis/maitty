import { beforeEach, describe, expect, it, vi } from 'vitest'
import { invoke, isTauri } from '@tauri-apps/api/core'
import type { Conversation } from '@/data/preview'
import { closeChatSession, conversationsFromSnapshot, startChatSession } from './daemon'

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(),
}))

// Chat sessions (ADR 0003): structured conversations the daemon owns, listed beside terminal sessions.
describe('chat sessions', () => {
  beforeEach(() => {
    vi.mocked(isTauri).mockReturnValue(true)
    vi.mocked(invoke).mockReset().mockImplementation((command) => Promise.resolve(command === 'start_chat_session' ? { id: 44, generation: 1 } : null))
  })

  const summary = (extra: Record<string, unknown> = {}) => ({
    id: 44, generation: 1, execution: 'Structured', provider: 'Claude', cwd: '/work/app-trees/agent-x', worktree: null, state: 'Running', status: 'Working',
    native_term: null, checks: [], title: null, archived: false, root: '/work/app', branch: 'agent/x', ...extra,
  })
  const snapshot = (conversations: unknown[]) => ({ terms: {}, workspaces: [], conversations })

  it('starts a chat session that asks first, unless full access is requested', async () => {
    await expect(startChatSession('/work/app', 'claude')).resolves.toEqual({ id: 44, generation: 1 })
    await startChatSession('/work/app', 'codex', 'agent/x')
    await startChatSession('/work/app', 'claude', null, true)
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ['start_chat_session', { cwd: '/work/app', provider: 'Claude', branch: null, fullAccess: false }],
      ['start_chat_session', { cwd: '/work/app', provider: 'Codex', branch: 'agent/x', fullAccess: false }],
      ['start_chat_session', { cwd: '/work/app', provider: 'Claude', branch: null, fullAccess: true }],
    ])
  })

  it("lists the helpers a chat session's provider started as its subagents, and none from an older daemon", () => {
    const helper = { id: 'helper-thread', name: 'probe_helper', parent: null, state: 'Completed', task: 'probe_helper', model: 'gpt-a', effort: 'low', started_at: 1, updated_at: 2, latest_activity: null, result: 'pong' }
    const [chat] = conversationsFromSnapshot(snapshot([summary({ helpers: [helper] })]))
    expect(chat?.details?.subagents).toEqual([{ id: 'helper-thread', name: 'probe_helper', state: 'Completed', task: 'probe_helper', startedAt: 1, updatedAt: 2, latest: null, result: 'pong' }])
    const [older] = conversationsFromSnapshot(snapshot([summary()]))
    expect(older?.details?.subagents).toEqual([])
  })

  it('lists a structured conversation as a chat session in its project, with a chat that can be written to', () => {
    const [chat] = conversationsFromSnapshot(snapshot([summary()]))
    expect(chat).toMatchObject({
      id: '44', execution: 'Structured', agent: 'claude', process: 'claude', status: 'working',
      folder: '/work/app', cwd: '/work/app-trees/agent-x', repo: 'app', branch: 'agent/x',
      chat: { conversation: 44, generation: 1 },
    })
    expect(chat?.kind).toBeUndefined()
    expect(chat?.chat).not.toHaveProperty('readOnly')
  })

  it('does not mistake the chat that follows a terminal pane for a chat session', () => {
    const term = { id: 7, agent: 'claude', process: 'claude', status: 'Working', cwd: '/srv', summary: '', name: 'n', label: '', pinned: false, archived: false, branch: null, root: null, conversation_id: null, popup: false, remote: null }
    const followed = summary({ id: 9, execution: 'Native', native_term: 7 })
    const listed = conversationsFromSnapshot({ terms: { '7': term }, workspaces: [], conversations: [followed] })
    expect(listed.map((item) => item.id)).toEqual(['t7'])
    expect(listed[0]?.chat).toEqual({ conversation: 9, generation: 1, readOnly: true })
  })

  it('reads a summary from before the repository and branch were named', () => {
    const old = Object.fromEntries(Object.entries(summary()).filter(([key]) => key !== 'root' && key !== 'branch'))
    const [chat] = conversationsFromSnapshot(snapshot([old]))
    expect(chat).toMatchObject({ folder: '/work/app-trees/agent-x', repo: null })
  })

  it('closing a chat session stops its agent first, then puts it away; a failed stop puts nothing away', async () => {
    const [chat] = conversationsFromSnapshot(snapshot([summary()]))
    await closeChatSession(chat as Conversation)
    expect(vi.mocked(invoke).mock.calls).toEqual([
      ['end_conversation', { id: 44, generation: 1 }],
      ['archive_conversation', { id: 44, archived: true }],
    ])
    vi.mocked(invoke).mockReset().mockRejectedValue('Conversation has a pending operation')
    await expect(closeChatSession(chat as Conversation)).rejects.toThrow()
    expect(vi.mocked(invoke).mock.calls.map((call) => call[0])).toEqual(['end_conversation'])
  })
})
