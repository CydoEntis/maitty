import { act, renderHook, waitFor } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { DraftAttachment } from '@/features/chat/composerStore'
import type { NewChatRequest } from './components/NewChatPage'
import { useBackgroundStarts } from './useBackgroundStarts'

const toast = vi.hoisted(() => ({ error: vi.fn() }))
vi.mock('@/lib/toast', () => ({ toast }))

const picture: DraftAttachment = { id: '1', path: '/data/attach-1.png', name: 'shot.png', size: 10, kind: 'image', preview: 'blob:shot' }
const request = { message: 'fix the login' } as NewChatRequest

afterEach(() => { vi.restoreAllMocks() })

describe('starts in the background', () => {
  it('lets go of a failed start\'s picture when the page closes without putting it back', async () => {
    const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined)
    const { result, unmount } = renderHook(() => useBackgroundStarts(() => Promise.reject(new Error('boom'))))
    act(() => { result.current.launch([request], { text: 'fix the login', attachments: [picture] }) })
    await waitFor(() => { expect(result.current.failed).toHaveLength(1) })
    expect(revoke).not.toHaveBeenCalled()

    unmount()
    expect(revoke).toHaveBeenCalledWith('blob:shot')
  })

  it('says a start that fails after the page closed in a toast, and lets go of its picture', async () => {
    const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined)
    let fail: (reason: Error) => void = () => undefined
    const { result, unmount } = renderHook(() => useBackgroundStarts(() => new Promise((_, reject) => { fail = reject })))
    act(() => { result.current.launch([request], { text: 'fix the login', attachments: [picture] }) })
    unmount()
    fail(new Error('boom'))

    await waitFor(() => { expect(toast.error).toHaveBeenCalledWith('Could not start “fix the login”: boom') })
    expect(revoke).toHaveBeenCalledWith('blob:shot')
  })

  it('keeps a prompt\'s picture while one of its starts failed, and never lets go of it once put back', async () => {
    const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined)
    const settle: Record<string, { pass: () => void; fail: (reason: Error) => void }> = {}
    const start = (asked: NewChatRequest) => new Promise<void>((pass, fail) => { settle[asked.agent] = { pass: () => { pass() }, fail } })
    const { result } = renderHook(() => useBackgroundStarts(start))
    const several = ['claude', 'codex', 'gemini'].map((agent) => ({ ...request, agent }) as NewChatRequest)
    act(() => { result.current.launch(several, { text: 'fix the login', attachments: [picture] }) })

    settle.codex?.fail(new Error('boom'))
    settle.claude?.pass()
    await waitFor(() => { expect(result.current.failed).toHaveLength(1) })
    expect(revoke).not.toHaveBeenCalled()

    let back: unknown = null
    act(() => { back = result.current.takeBack(result.current.failed[0]?.key ?? -1) })
    expect(back).toEqual({ text: 'fix the login', attachments: [picture] })
    settle.gemini?.pass()
    await waitFor(() => { expect(result.current.pending).toBe(0) })
    expect(revoke).not.toHaveBeenCalled()
  })

  it('gives a start that fails after its prompt was put back nothing to put back, and keeps the box\'s picture', async () => {
    const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined)
    const settle: Record<string, (reason: Error) => void> = {}
    const start = (asked: NewChatRequest) => new Promise<void>((_, fail) => { settle[asked.agent] = fail })
    const { result, unmount } = renderHook(() => useBackgroundStarts(start))
    const two = ['claude', 'codex'].map((agent) => ({ ...request, agent }) as NewChatRequest)
    act(() => { result.current.launch(two, { text: 'fix the login', attachments: [picture] }) })

    settle.claude?.(new Error('boom'))
    await waitFor(() => { expect(result.current.failed).toHaveLength(1) })
    act(() => { result.current.takeBack(result.current.failed[0]?.key ?? -1) })
    settle.codex?.(new Error('boom'))
    await waitFor(() => { expect(result.current.failed).toHaveLength(1) })

    expect(result.current.failed[0]?.draft).toBeNull()
    unmount()
    expect(revoke).not.toHaveBeenCalled()
  })
})
