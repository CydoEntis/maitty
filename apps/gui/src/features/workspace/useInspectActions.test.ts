import { renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { toast } from '@/lib/toast'
import type { SessionView } from './model/session'
import { useInspectActions } from './useInspectActions'

vi.mock('@/lib/toast', () => ({ toast: { message: vi.fn() } }))

const TURN = { startedAtMs: 1_000, endedAtMs: 9_000 }

function mount() {
  const here = { id: 'here', machine: 0, machineName: null } as unknown as SessionView
  const other = { id: 'other', machine: 0, machineName: null } as unknown as SessionView
  const far = { id: 'far', machine: 2, machineName: 'build' } as unknown as SessionView
  const spies = { setInspectOpen: vi.fn(), setInspectView: vi.fn(), setTurnDiff: vi.fn(), showSessionById: vi.fn(), setLayout: vi.fn() }
  const { result } = renderHook(() => useInspectActions({
    ...spies,
    focusedSession: here,
    sessionMap: new Map([['here', here], ['other', other], ['far', far]]),
    inspectOpen: false,
    inspectView: 'changes',
    focusedId: 'here',
  }))
  return { actions: result.current, ...spies }
}

describe('showing a session’s changes from its chat', () => {
  it('ties a turn to the session whose row was clicked and brings that session to the front', () => {
    const { actions, setTurnDiff, showSessionById, setInspectView, setInspectOpen } = mount()
    actions.showChanges('other', 'a.ts', TURN)
    expect(showSessionById).toHaveBeenCalledWith('other')
    expect(setTurnDiff).toHaveBeenCalledWith({ sessionId: 'other', turn: TURN })
    expect(setInspectView).toHaveBeenCalledWith('changes')
    expect(setInspectOpen).toHaveBeenCalledWith(true)
  })

  it('shows the whole working tree when no turn is given', () => {
    const { actions, setTurnDiff } = mount()
    actions.showChanges('here', 'a.ts')
    expect(setTurnDiff).toHaveBeenCalledWith(null)
  })

  it('refuses a session on another machine and keeps no turn', () => {
    const { actions, setTurnDiff, setInspectOpen } = mount()
    actions.showChanges('far', 'a.ts', TURN)
    expect(toast.message).toHaveBeenCalled()
    expect(setTurnDiff).not.toHaveBeenCalled()
    expect(setInspectOpen).not.toHaveBeenCalled()
  })
})
