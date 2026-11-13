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
    act(() => { result.current.launch(request, { text: 'fix the login', attachments: [picture] }) })
    await waitFor(() => { expect(result.current.failed).toHaveLength(1) })
    expect(revoke).not.toHaveBeenCalled()

    unmount()
    expect(revoke).toHaveBeenCalledWith('blob:shot')
  })

  it('says a start that fails after the page closed in a toast, and lets go of its picture', async () => {
    const revoke = vi.spyOn(URL, 'revokeObjectURL').mockImplementation(() => undefined)
    let fail: (reason: Error) => void = () => undefined
    const { result, unmount } = renderHook(() => useBackgroundStarts(() => new Promise((_, reject) => { fail = reject })))
    act(() => { result.current.launch(request, { text: 'fix the login', attachments: [picture] }) })
    unmount()
    fail(new Error('boom'))

    await waitFor(() => { expect(toast.error).toHaveBeenCalledWith('Could not start “fix the login”: boom') })
    expect(revoke).toHaveBeenCalledWith('blob:shot')
  })
})
