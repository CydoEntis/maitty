import { describe, expect, it, vi } from 'vitest'

vi.mock('@/api/daemon', () => ({
  terminalOf: (id: string) => (id.startsWith('t') ? Number(id.slice(1)) : null),
}))

import { chatAvailability, chatSourceOf } from './chatView'
import { isChatSession, isListedChatSession } from './chatSession'
import { groupSessions, toSessionView } from './session'
import type { Conversation } from '@/data/preview'

const HOME = '/home/casey'

function chat(extra: Partial<Conversation> = {}): Conversation {
  return {
    id: '44', generation: 1, execution: 'Structured', title: 'Claude conversation 44', project: 'app', folder: `${HOME}/app`, cwd: `${HOME}/app-trees/agent-x`, repo: 'app',
    branch: 'agent/x', provider: 'Claude', status: 'working', updated: null, model: '', permission: '', context: null, nativePane: null, hasDiff: false,
    agent: 'claude', process: 'claude', chat: { conversation: 44, generation: 1 }, ...extra,
  }
}

describe('what is a chat session', () => {
  it('is a structured conversation the daemon started, with a chat to write to', () => {
    expect(isChatSession(chat())).toBe(true)
  })

  it('is not a terminal session, nor the read-only chat that follows a pane', () => {
    expect(isChatSession(chat({ kind: 'terminal', execution: 'Native', nativePane: 7 }))).toBe(false)
    expect(isChatSession({ kind: 'terminal', execution: 'Native' })).toBe(false)
    const withoutLink = Object.fromEntries(Object.entries(chat()).filter(([key]) => key !== 'chat')) as Conversation
    expect(isChatSession(withoutLink)).toBe(false)
  })

  it('leaves the list when it is put away', () => {
    expect(isListedChatSession(chat())).toBe(true)
    expect(isListedChatSession(chat({ archived: true }))).toBe(false)
  })
})

describe('a chat session in the window', () => {
  it('sits in its project with its agent and branch, and its chat is ready', () => {
    const view = toSessionView(chat(), HOME)
    expect(view).toMatchObject({ process: 'claude', repoName: 'app', branch: 'agent/x', status: 'working' })
    const groups = groupSessions([view], 'repo', null, HOME)
    expect(groups).toHaveLength(1)
    expect(groups[0]).toMatchObject({ name: 'app', kind: 'repo', workingCount: 1 })
    // Writable: the chat link has no read-only mark, so the composer is on.
    const availability = chatAvailability(chatSourceOf(view))
    expect(availability).toMatchObject({ kind: 'ready', agent: 'claude' })
    expect(availability.kind === 'ready' && availability.link.readOnly).toBeFalsy()
  })

  it('needs you when the agent waits for an approval', () => {
    expect(toSessionView(chat({ status: 'needs-you' }), HOME).status).toBe('needs')
  })
})
