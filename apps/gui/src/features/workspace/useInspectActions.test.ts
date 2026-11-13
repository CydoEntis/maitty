import { act, renderHook } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { toast } from '@/lib/toast'
import type { SessionView } from './model/session'
import { useInspectActions, useTurnDiff } from './useInspectActions'

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
  it('ties a turn to the session whose row was clicked and focuses its pane, keeping a helper chat open there', () => {
    const { actions, setTurnDiff, showSessionById, setLayout, setInspectView, setInspectOpen } = mount()
    actions.showChanges('pane-2', 'other', 'a.ts', TURN)
    // showSessionById would close an open helper agent's chat; focusing the pane leaves it be.
    expect(showSessionById).not.toHaveBeenCalled()
    expect(setLayout).toHaveBeenCalledTimes(1)
    expect(setTurnDiff).toHaveBeenCalledWith({ sessionId: 'other', turn: TURN })
    expect(setInspectView).toHaveBeenCalledWith('changes')
    expect(setInspectOpen).toHaveBeenCalledWith(true)
  })

  it('shows the whole working tree when no turn is given', () => {
    const { actions, setTurnDiff } = mount()
    actions.showChanges('pane-1', 'here', 'a.ts')
    expect(setTurnDiff).toHaveBeenCalledWith(null)
  })

  it('brings a session from its menu to the front, wherever it is, on the whole working tree', () => {
    const { actions, setTurnDiff, showSessionById } = mount()
    actions.showChangesOf('other')
    expect(showSessionById).toHaveBeenCalledWith('other')
    expect(setTurnDiff).toHaveBeenCalledWith(null)
  })

  it('refuses a session on another machine and keeps no turn', () => {
    const { actions, setTurnDiff, setInspectOpen } = mount()
    actions.showChanges('pane-3', 'far', 'a.ts', TURN)
    expect(toast.message).toHaveBeenCalled()
    expect(setTurnDiff).not.toHaveBeenCalled()
    expect(setInspectOpen).not.toHaveBeenCalled()
  })
})

describe('the turn a diff was opened for', () => {
  it('stays while its session has the focus and goes when another session takes it', () => {
    const { result, rerender } = renderHook(({ focused }: { focused: string | null }) => useTurnDiff(focused), { initialProps: { focused: 'here' } })
    act(() => { result.current[1]({ sessionId: 'other', turn: TURN }) })
    rerender({ focused: 'other' })
    expect(result.current[0]).toEqual({ sessionId: 'other', turn: TURN })
    rerender({ focused: 'here' })
    expect(result.current[0]).toBeNull()
  })
})
