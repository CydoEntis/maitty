import { renderHook, waitFor } from '@testing-library/react'
import { QueryClient } from '@tanstack/react-query'
import { beforeEach, describe, expect, it, vi } from 'vitest'
import { archiveSessionOnDaemon, snoozeSessionOnDaemon } from '@/api/daemon'
import { toast } from '@/lib/toast'
import { toSessionView, type SessionView } from './model/session'
import { usePutAway } from './usePutAway'

vi.mock('@/api/daemon', () => ({ archiveSessionOnDaemon: vi.fn(() => Promise.resolve()), snoozeSessionOnDaemon: vi.fn(() => Promise.resolve()), terminalOf: (id: string) => Number(id.slice(1)) }))
vi.mock('@/lib/toast', () => ({ toast: Object.assign(vi.fn(), { error: vi.fn() }) }))

const finished = toSessionView({ id: 't1', kind: 'terminal', title: 'Parser fix', named: true, project: '', folder: '/home/me/app', cwd: '/home/me/app', repo: null, branch: '', provider: 'Claude', status: 'done', updated: null, model: '', permission: '', context: null, nativePane: 1, hasDiff: false, agent: 'claude' }, '/home/me')

const UNTIL = 4_102_444_800
const details = { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null } }
/** The finished session as the list reports it while snoozed until `until` (unix seconds). */
const snoozedUntil = (until: number | null) => toSessionView({ ...finished.conversation, details: { ...details, snoozedUntil: until } }, '/home/me', 0)

function render(initial: SessionView = finished) {
  const queryClient = new QueryClient()
  const invalidate = vi.spyOn(queryClient, 'invalidateQueries').mockResolvedValue()
  const hook = renderHook(({ session }) => usePutAway({ queryClient, sessionMap: new Map([[session.id, session]]) }), { initialProps: { session: initial } })
  return { hook, invalidate }
}

describe('bringing a session back', () => {
  beforeEach(() => { vi.clearAllMocks() })

  it('also wakes it when it is snoozed, so it cannot stay under Put away', async () => {
    const { hook, invalidate } = render(snoozedUntil(UNTIL))
    hook.result.current.bringBack('t1')
    await waitFor(() => { expect(invalidate).toHaveBeenCalled() })
    expect(archiveSessionOnDaemon).toHaveBeenCalledWith({ id: 't1' }, false)
    expect(snoozeSessionOnDaemon).toHaveBeenCalledWith({ id: 't1' }, null)
  })

  it('does not touch the snooze of a session that has none', async () => {
    const { hook, invalidate } = render()
    hook.result.current.bringBack('t1')
    await waitFor(() => { expect(invalidate).toHaveBeenCalled() })
    expect(snoozeSessionOnDaemon).not.toHaveBeenCalled()
  })
})

describe('putting a session away', () => {
  beforeEach(() => { vi.clearAllMocks() })

  it('puts it away on the daemon, without closing it, and offers Undo for six seconds', async () => {
    const { hook, invalidate } = render()
    hook.result.current.putAway('t1')
    await waitFor(() => { expect(toast).toHaveBeenCalled() })
    expect(archiveSessionOnDaemon).toHaveBeenCalledWith({ id: 't1' }, true)
    const [title, options] = vi.mocked(toast).mock.calls[0] ?? []
    expect([title, options?.duration, options?.action?.label]).toEqual(['Put away Parser fix', 6000, 'Undo'])
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ['conversations'] })
  })

  it('brings it back when Undo is clicked', async () => {
    const { hook } = render()
    hook.result.current.putAway('t1')
    await waitFor(() => { expect(toast).toHaveBeenCalled() })
    const options = vi.mocked(toast).mock.calls[0]?.[1]
    options?.action?.onClick({ preventDefault: () => undefined })
    expect(archiveSessionOnDaemon).toHaveBeenLastCalledWith({ id: 't1' }, false)
  })

  it('says so when the daemon could not put it away', async () => {
    vi.mocked(archiveSessionOnDaemon).mockRejectedValueOnce(new Error('pipe closed'))
    const { hook } = render()
    hook.result.current.putAway('t1')
    await waitFor(() => { expect(toast.error).toHaveBeenCalledWith('pipe closed') })
    expect(toast).not.toHaveBeenCalled()
  })
})

describe('snoozing a session', () => {
  beforeEach(() => { vi.clearAllMocks() })

  it('snoozes it on the daemon until the time, and offers Undo for six seconds', async () => {
    const { hook, invalidate } = render()
    hook.result.current.snooze('t1', UNTIL)
    await waitFor(() => { expect(toast).toHaveBeenCalled() })
    expect(snoozeSessionOnDaemon).toHaveBeenCalledWith({ id: 't1' }, UNTIL)
    // The list now reports the snooze, as it does once the daemon has taken it.
    hook.rerender({ session: snoozedUntil(UNTIL) })
    const [title, options] = vi.mocked(toast).mock.calls[0] ?? []
    expect([title, options?.duration, options?.action?.label]).toEqual([expect.stringMatching(/^Snoozed Parser fix until /), 6000, 'Undo'])
    expect(invalidate).toHaveBeenCalledWith({ queryKey: ['conversations'] })
    options?.action?.onClick({ preventDefault: () => undefined })
    expect(snoozeSessionOnDaemon).toHaveBeenLastCalledWith({ id: 't1' }, null)
  })

  it('leaves a snooze alone on Undo when it is no longer the one just set', async () => {
    const { hook } = render()
    hook.result.current.snooze('t1', UNTIL)
    await waitFor(() => { expect(toast).toHaveBeenCalled() })
    const action = vi.mocked(toast).mock.calls[0]?.[1]?.action
    // Another window woke it, or snoozed it to another time, or it is awake.
    for (const other of [snoozedUntil(UNTIL + 3600), snoozedUntil(null)]) {
      hook.rerender({ session: other })
      action?.onClick({ preventDefault: () => undefined })
    }
    expect(snoozeSessionOnDaemon).toHaveBeenCalledTimes(1)
  })

  it('says it asked, not that it is done, for a session on another machine', async () => {
    const there = toSessionView({ ...finished.conversation, machine: 3, machineName: 'devbox' }, '/home/me')
    const { hook } = render(there)
    hook.result.current.snooze('t1', UNTIL)
    await waitFor(() => { expect(toast).toHaveBeenCalled() })
    expect(vi.mocked(toast).mock.calls[0]?.[0]).toMatch(/^Asked to snooze devbox · Parser fix until /)
  })

  it('wakes it with Unsnooze', async () => {
    const { hook, invalidate } = render()
    hook.result.current.unsnooze('t1')
    await waitFor(() => { expect(invalidate).toHaveBeenCalled() })
    expect(snoozeSessionOnDaemon).toHaveBeenCalledWith({ id: 't1' }, null)
  })

  it('says why when the daemon refuses, as it does for a session that needs you', async () => {
    vi.mocked(snoozeSessionOnDaemon).mockRejectedValueOnce(new Error('it needs you now: answer it before you snooze it'))
    const { hook } = render()
    hook.result.current.snooze('t1', 1_800_000_000)
    await waitFor(() => { expect(toast.error).toHaveBeenCalledWith('it needs you now: answer it before you snooze it') })
    expect(toast).not.toHaveBeenCalled()
  })
})
