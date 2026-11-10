import { beforeEach, describe, expect, it, vi } from 'vitest'
import { sendTerminalInput, submitMessage } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import { agentName, sendAgentMessage } from './agentMessage'
import { toSessionView } from './session'

vi.mock('@/api/daemon', () => ({
  terminalOf: () => 1,
  sendTerminalInput: vi.fn(() => Promise.resolve()),
  submitMessage: vi.fn(() => Promise.resolve({})),
}))

function session(extra: Partial<Conversation> = {}, terminal = true) {
  return toSessionView(
    { id: 't1', generation: 0, ...(terminal ? { kind: 'terminal' as const } : {}), title: '', project: '', folder: '/home/me/app', cwd: '/home/me/app', repo: null, branch: 'main', provider: 'Claude', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: 1, hasDiff: false, agent: 'claude', ...extra },
    '/home/me',
  )
}

beforeEach(() => { vi.clearAllMocks() })

describe('a message to an agent from outside its message box', () => {
  it('is pasted into a terminal agent as one message and entered', async () => {
    await sendAgentMessage(session(), 'About a.ts, line 3:\n> x\n\nrename it')
    expect(sendTerminalInput).toHaveBeenCalledWith({ id: 't1', generation: 0 }, '\x1b[200~About a.ts, line 3:\n> x\n\nrename it\x1b[201~\r')
    expect(submitMessage).not.toHaveBeenCalled()
  })

  it('is not typed into an agent that is asking something', async () => {
    await expect(sendAgentMessage(session({ status: 'needs-you' }), 'hello')).rejects.toThrow('Claude is asking something')
    expect(sendTerminalInput).not.toHaveBeenCalled()
  })

  it('asks for the chat to be opened first when a chat session has not been read yet', async () => {
    const chat = session({ id: '7', execution: 'Structured', chat: { conversation: 7, generation: 2 } }, false)
    await expect(sendAgentMessage(chat, 'hello')).rejects.toThrow('Open the chat once')
    expect(submitMessage).not.toHaveBeenCalled()
  })
})

describe('an agent’s name', () => {
  it('starts with a capital', () => {
    expect(agentName('claude')).toBe('Claude')
    expect(agentName('codex')).toBe('Codex')
  })
})
