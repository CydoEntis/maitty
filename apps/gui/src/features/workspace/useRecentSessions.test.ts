import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import type { ClosedSession } from '@/api/daemon'
import { toast } from '@/lib/toast'
import { useRecentSessions } from './useRecentSessions'

vi.mock('@/lib/toast', () => ({ toast: { error: vi.fn(), message: vi.fn() } }))

const parser: ClosedSession = { agent: 'claude', session: 'abc-123', cwd: '/dev/app', branch: null, title: 'Fix the parser', at: 1, folder_present: true }

function listOpen(starting: boolean) {
  const startNewChat = vi.fn(() => Promise.resolve())
  const hook = renderHook(() => useRecentSessions(startNewChat, starting))
  act(() => { hook.result.current.setRecentOpen(true) })
  return { hook, startNewChat }
}

describe('reopening from Recent sessions', () => {
  it('closes the list and starts the session', () => {
    const { hook, startNewChat } = listOpen(false)
    act(() => { hook.result.current.reopenSession(parser) })
    expect(startNewChat).toHaveBeenCalledWith(expect.objectContaining({ folder: '/dev/app', resume: 'abc-123' }))
    expect(hook.result.current.recentOpen).toBe(false)
  })

  it('says so, and keeps the list open, while another session is starting', () => {
    const { hook, startNewChat } = listOpen(true)
    act(() => { hook.result.current.reopenSession(parser) })
    expect(startNewChat).not.toHaveBeenCalled()
    expect(toast.message).toHaveBeenCalledWith('A session is already starting')
    expect(hook.result.current.recentOpen).toBe(true)
  })
})
