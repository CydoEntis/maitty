import { beforeEach, describe, expect, it, vi } from 'vitest'
import { sendTerminalInput } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import { forgetComposers, peekComposer, updateComposer } from '@/features/chat/composerStore'
import { carryDraftToTerminal } from './draftCarry'
import { toSessionView } from './session'

vi.mock('@/api/daemon', () => ({
  terminalOf: () => 1,
  sendTerminalInput: vi.fn(() => Promise.resolve()),
}))

const CHAT = 7

function session(extra: Partial<Conversation> = {}, terminal = true) {
  return toSessionView(
    { id: 't1', generation: 0, ...(terminal ? { kind: 'terminal' as const } : {}), title: '', project: '', folder: '/home/me/app', cwd: '/home/me/app', repo: null, branch: 'main', provider: 'Claude', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: 1, hasDiff: false, agent: 'claude', chat: { conversation: CHAT, generation: 2, readOnly: true }, ...extra },
    '/home/me',
  )
}

function write(draft: string) {
  updateComposer(CHAT, (state) => ({ ...state, draft }))
}

beforeEach(() => {
  vi.clearAllMocks()
  forgetComposers()
})

describe('the chat box text when the pane switches to its terminal', () => {
  it('is pasted into the agent\'s prompt without Enter and leaves the box', async () => {
    write('rename it\nand test it')
    await carryDraftToTerminal(session())
    expect(sendTerminalInput).toHaveBeenCalledWith({ id: 't1', generation: 0 }, '\x1b[200~rename it\nand test it\x1b[201~')
    expect(peekComposer(CHAT).draft).toBe('')
  })

  it('stays in the box while the agent works or asks something', async () => {
    write('hello')
    await carryDraftToTerminal(session({ status: 'working' }))
    await carryDraftToTerminal(session({ status: 'needs-you' }))
    expect(sendTerminalInput).not.toHaveBeenCalled()
    expect(peekComposer(CHAT).draft).toBe('hello')
  })

  it('stays in the box when files are attached to it', async () => {
    updateComposer(CHAT, (state) => ({ ...state, draft: 'see this', attachments: [{ id: 'a', path: '/tmp/a.png', name: 'a.png', size: 1, kind: 'image', preview: null }] }))
    await carryDraftToTerminal(session())
    expect(sendTerminalInput).not.toHaveBeenCalled()
    expect(peekComposer(CHAT).draft).toBe('see this')
  })

  it('stays in the box when it could not be typed', async () => {
    vi.mocked(sendTerminalInput).mockRejectedValueOnce(new Error('offline'))
    write('hello')
    await expect(carryDraftToTerminal(session())).rejects.toThrow('offline')
    expect(peekComposer(CHAT).draft).toBe('hello')
  })

  it('types nothing for an empty box or a chat session', async () => {
    await carryDraftToTerminal(session())
    write('hello')
    await carryDraftToTerminal(session({ id: '7', execution: 'Structured', chat: { conversation: CHAT, generation: 2 } }, false))
    expect(sendTerminalInput).not.toHaveBeenCalled()
  })
})
