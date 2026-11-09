import { describe, expect, it } from 'vitest'
import { chatAvailability, effectiveView, noChatReason, type ChatAvailability, type ChatViewDefaults } from './chatView'

const LINK = { conversation: 7, generation: 2 }
const TERMINAL_FIRST: ChatViewDefaults = { claude: 'terminal', codex: 'terminal', gemini: 'terminal' }

describe('chatAvailability', () => {
  it('is ready for a Claude or Codex session the daemon holds a conversation for', () => {
    expect(chatAvailability({ agent: 'claude', machineName: null, chat: LINK })).toEqual({ kind: 'ready', agent: 'claude', link: LINK })
    expect(chatAvailability({ agent: 'Codex', machineName: null, chat: LINK })).toEqual({ kind: 'ready', agent: 'codex', link: LINK })
  })

  it('says there are no events for a supported agent without a conversation', () => {
    expect(chatAvailability({ agent: 'claude', machineName: null, chat: null })).toEqual({ kind: 'no-events', agent: 'claude' })
  })

  it('leaves a plain shell and other agents terminal-only', () => {
    expect(chatAvailability({ agent: null, machineName: null, chat: null })).toEqual({ kind: 'shell' })
    expect(chatAvailability({ agent: 'opencode', machineName: null, chat: LINK })).toEqual({ kind: 'unsupported', agent: 'opencode' })
  })

  it('gives Gemini CLI the same chat as Claude and Codex, read from its transcript', () => {
    expect(chatAvailability({ agent: 'gemini', machineName: null, chat: LINK })).toEqual({ kind: 'ready', agent: 'gemini', link: LINK })
    expect(chatAvailability({ agent: 'Gemini', machineName: null, chat: null })).toEqual({ kind: 'no-events', agent: 'gemini' })
  })

  it('keeps another machine terminal-only, whatever it runs', () => {
    expect(chatAvailability({ agent: 'claude', machineName: 'build', chat: LINK })).toEqual({ kind: 'remote', agent: 'claude', machineName: 'build' })
  })
})

describe('effectiveView', () => {
  const ready: ChatAvailability = { kind: 'ready', agent: 'claude', link: LINK }

  it('opens as the terminal unless the agent default says chat', () => {
    expect(effectiveView(undefined, ready, TERMINAL_FIRST)).toBe('terminal')
    expect(effectiveView(undefined, ready, { claude: 'chat', codex: 'terminal', gemini: 'terminal' })).toBe('chat')
    expect(effectiveView(undefined, { kind: 'ready', agent: 'codex', link: LINK }, { claude: 'chat', codex: 'terminal', gemini: 'terminal' })).toBe('terminal')
  })

  it('never opens a session without a chat in the chat view by default', () => {
    expect(effectiveView(undefined, { kind: 'shell' }, { claude: 'chat', codex: 'chat', gemini: 'chat' })).toBe('terminal')
    expect(effectiveView(undefined, { kind: 'no-events', agent: 'claude' }, { claude: 'chat', codex: 'chat', gemini: 'chat' })).toBe('terminal')
  })

  it('lets an explicit choice win, even on a shell tab (it shows the empty chat card)', () => {
    expect(effectiveView('chat', { kind: 'shell' }, TERMINAL_FIRST)).toBe('chat')
    expect(effectiveView('terminal', ready, { claude: 'chat', codex: 'chat', gemini: 'chat' })).toBe('terminal')
  })
})

describe('noChatReason', () => {
  it('names the shell case the way the design does', () => {
    expect(noChatReason({ kind: 'shell' })).toContain('Chat view is for agent sessions')
  })

  it('says why a remote session shows the terminal only', () => {
    expect(noChatReason({ kind: 'remote', agent: 'claude', machineName: 'build' })).toContain('local-only in v1')
  })
})
