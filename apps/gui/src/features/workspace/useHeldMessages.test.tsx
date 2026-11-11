import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { sendTerminalInput } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import { claimHeld, forgetComposers, peekComposer, updateComposer } from '@/features/chat/composerStore'
import { HELD_GAP_MS } from '@/features/chat/useChatComposer'
import { toSessionView, type SessionView } from './model/session'
import { useHeldMessages } from './useHeldMessages'

vi.mock('@/api/daemon', () => ({
  terminalOf: () => 1,
  sendTerminalInput: vi.fn(() => Promise.resolve()),
}))

const CHAT = 7

function session(status: Conversation['status']): SessionView {
  return toSessionView(
    { id: 't1', generation: 0, kind: 'terminal' as const, title: '', project: '', folder: '/home/me/app', cwd: '/home/me/app', repo: null, branch: 'main', provider: 'Claude', status, updated: null, model: '', permission: '', context: null, nativePane: 1, hasDiff: false, agent: 'claude', chat: { conversation: CHAT, generation: 2, readOnly: true } },
    '/home/me',
  )
}

const hold = (...texts: string[]) => {
  updateComposer(CHAT, (state) => ({ ...state, held: texts.map((text, index) => ({ id: -1 - index, text })) }))
}

async function pass(ms = 0) {
  await act(async () => { await vi.advanceTimersByTimeAsync(ms) })
}

beforeEach(() => {
  vi.useFakeTimers()
  vi.clearAllMocks()
  forgetComposers()
})

afterEach(() => { vi.useRealTimers() })

describe('messages queued for a terminal agent whose chat is not on screen', () => {
  it('go in one at a time as the agent finishes, without the chat being opened', async () => {
    hold('first', 'second')
    const view = renderHook(({ status }: { status: Conversation['status'] }) => { useHeldMessages({ sessions: [session(status)], online: true }) }, { initialProps: { status: 'working' as Conversation['status'] } })
    await pass(HELD_GAP_MS)
    expect(sendTerminalInput).not.toHaveBeenCalled()

    // The turn ends: the first goes in.
    view.rerender({ status: 'done' })
    await pass()
    expect(sendTerminalInput).toHaveBeenCalledExactlyOnceWith({ id: 't1', generation: 0 }, '\x1b[200~first\x1b[201~\r')
    expect(peekComposer(CHAT).held.map((message) => message.text)).toEqual(['second'])

    // The agent has not said it is working yet: the next does not follow at once.
    await pass(HELD_GAP_MS - 1)
    expect(sendTerminalInput).toHaveBeenCalledOnce()

    // It works on the first, then finishes: the second goes in.
    view.rerender({ status: 'working' })
    await pass(HELD_GAP_MS)
    view.rerender({ status: 'done' })
    await pass()
    expect(vi.mocked(sendTerminalInput).mock.calls.map(([, text]) => text)).toEqual(['\x1b[200~first\x1b[201~\r', '\x1b[200~second\x1b[201~\r'])
    expect(peekComposer(CHAT).held).toEqual([])
  })

  it('are left to the message box while it shows the chat, and are not typed into a question', async () => {
    hold('first')
    const release = claimHeld(CHAT)
    const view = renderHook(({ status }: { status: Conversation['status'] }) => { useHeldMessages({ sessions: [session(status)], online: true }) }, { initialProps: { status: 'idle' as Conversation['status'] } })
    await pass(HELD_GAP_MS)
    expect(sendTerminalInput).not.toHaveBeenCalled()

    release()
    view.rerender({ status: 'needs-you' })
    await pass(HELD_GAP_MS)
    expect(sendTerminalInput).not.toHaveBeenCalled()
  })

  it('keeps a message that could not be typed, to try again', async () => {
    vi.mocked(sendTerminalInput).mockRejectedValueOnce(new Error('offline'))
    hold('first')
    renderHook(() => { useHeldMessages({ sessions: [session('idle')], online: true }) })

    await pass()

    expect(peekComposer(CHAT).held.map((message) => message.text)).toEqual(['first'])
  })
})
