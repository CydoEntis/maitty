import type { UseQueryResult } from '@tanstack/react-query'
import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { EMPTY_LAYOUT } from './model/layout'
import { LINGER_MS, LINGER_SLACK_MS } from './model/lingering'
import type { RemoteMachine } from './model/machines'
import { useSessionList } from './useSessionList'

/** What the daemon module last read of the lists, the full list a check reads, and the asking of a machine for its list. */
const daemon = vi.hoisted(() => ({ revision: 4, removed: new Set<string>(), load: vi.fn<() => Promise<Conversation[]>>(), askList: vi.fn<(machine: number) => Promise<void>>() }))
vi.mock('@/api/daemon', async (importOriginal) => ({
  ...await importOriginal<typeof import('@/api/daemon')>(),
  checkConversations: daemon.load,
  localListRevision: () => daemon.revision,
  daemonSaidRemoved: (id: string) => daemon.removed.has(id),
}))
vi.mock('@/api/machines', async (importOriginal) => ({
  ...await importOriginal<typeof import('@/api/machines')>(),
  askMachineList: daemon.askList,
}))

const NOW = new Date(2026, 0, 5, 9, 30).getTime()
const WAIT_MS = 5 * 60_000

/** An agent session the daemon says finished, held by its plan limit until `resumeAt` (unix seconds) when one is given. */
const terminal = (id: string, resumeAt: number | null = null): Conversation => ({
  id, generation: 0, kind: 'terminal', title: '', project: '', folder: '/home/me/app', cwd: '/home/me/app', repo: null, branch: 'working tree', provider: 'Claude', status: 'done', updated: null, model: '', permission: '', context: null, nativePane: 1, hasDiff: false, agent: 'claude',
  details: { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, resumeAt },
})
/** One agent that continues after its plan limit in five minutes. */
const LISTED: Conversation[] = [terminal('t1', (NOW + WAIT_MS) / 1000)]
const PROPS = { sessionsQuery: { data: LISTED } as UseQueryResult<Conversation[]>, online: true, machines: new Map(), home: '/home/me', layout: EMPTY_LAYOUT }

describe('a listed session held by its plan limit', () => {
  beforeEach(() => { vi.useFakeTimers({ now: NOW }) })
  afterEach(() => { vi.useRealTimers() })

  it('goes back to the status the daemon gave when its time comes, with no new list', () => {
    const { result } = renderHook(() => useSessionList(PROPS))
    expect(result.current.sessions.map((session) => session.status)).toEqual(['limited'])

    act(() => { vi.advanceTimersByTime(WAIT_MS - 1) })
    expect(result.current.sessions.map((session) => session.status)).toEqual(['limited'])

    act(() => { vi.advanceTimersByTime(1) })
    expect(result.current.sessions.map((session) => session.status)).toEqual(['done'])
  })
})

describe('a session the next list leaves out', () => {
  const listOf = (...ids: string[]) => ({ ...PROPS, sessionsQuery: { data: ids.map((id) => terminal(id)) } as UseQueryResult<Conversation[]> })
  const rows = (sessions: readonly { id: string; status: string }[]) => sessions.map((session) => `${session.id} ${session.status}`)

  beforeEach(() => {
    vi.useFakeTimers({ now: NOW })
    daemon.revision = 4
    daemon.removed.clear()
    daemon.load.mockReset()
  })
  afterEach(() => { vi.useRealTimers() })

  it('stays as unknown, logged once with both revisions, until a full list confirms it is gone', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const { result, rerender } = renderHook((props) => useSessionList(props), { initialProps: listOf('t1', 't2') })
    daemon.revision = 5
    rerender(listOf('t1'))
    expect(rows(result.current.sessions)).toEqual(['t1 done', 't2 unknown'])
    expect(warn).toHaveBeenCalledTimes(1)
    expect(warn.mock.calls[0]?.join(' ')).toMatch(/t2.*4.*5/)

    daemon.revision = 6
    rerender(listOf('t1', 't3'))
    expect(rows(result.current.sessions)).toEqual(['t1 done', 't3 done', 't2 unknown'])
    expect(warn).toHaveBeenCalledTimes(1)

    daemon.load.mockResolvedValue([terminal('t1'), terminal('t3')])
    await act(async () => { await vi.advanceTimersByTimeAsync(LINGER_MS + LINGER_SLACK_MS) })
    expect(daemon.load).toHaveBeenCalledTimes(1)
    expect(rows(result.current.sessions)).toEqual(['t1 done', 't3 done'])
    warn.mockRestore()
  })

  it('stays while the full list still names it or cannot be read', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const { result, rerender } = renderHook((props) => useSessionList(props), { initialProps: listOf('t1', 't2') })
    rerender(listOf('t1'))
    daemon.load.mockResolvedValueOnce([terminal('t1'), terminal('t2')]).mockRejectedValueOnce(new Error('no daemon'))

    await act(async () => { await vi.advanceTimersByTimeAsync(LINGER_MS + LINGER_SLACK_MS) })
    await act(async () => { await vi.advanceTimersByTimeAsync(LINGER_MS + LINGER_SLACK_MS) })
    expect(daemon.load).toHaveBeenCalledTimes(2)
    expect(rows(result.current.sessions)).toEqual(['t1 done', 't2 unknown'])
    warn.mockRestore()
  })

  it('says once that the full list cannot be read, and once that it can again', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const info = vi.spyOn(console, 'info').mockImplementation(() => undefined)
    const { rerender } = renderHook((props) => useSessionList(props), { initialProps: listOf('t1', 't2') })
    rerender(listOf('t1'))
    warn.mockClear()
    daemon.load.mockRejectedValueOnce(new Error('no daemon')).mockRejectedValueOnce(new Error('no daemon')).mockResolvedValueOnce([terminal('t1'), terminal('t2')]).mockResolvedValueOnce([terminal('t1'), terminal('t2')])

    for (let check = 0; check < 4; check += 1) await act(async () => { await vi.advanceTimersByTimeAsync(LINGER_MS + LINGER_SLACK_MS) })
    expect(daemon.load).toHaveBeenCalledTimes(4)
    expect(warn.mock.calls.map((call) => String(call[0]))).toEqual([expect.stringMatching(/could not read/i)])
    expect(info.mock.calls.map((call) => String(call[0]))).toEqual([expect.stringMatching(/read again/i)])
    warn.mockRestore()
    info.mockRestore()
  })

  it('leaves at once, unlogged, when the daemon said why it left', () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const { result, rerender } = renderHook((props) => useSessionList(props), { initialProps: listOf('t1', 't2') })
    daemon.removed.add('t2')
    rerender(listOf('t1'))
    expect(rows(result.current.sessions)).toEqual(['t1 done'])
    expect(warn).not.toHaveBeenCalled()
    warn.mockRestore()
  })
})

describe('a session of another machine the next list leaves out', () => {
  const machineListing = (...ids: string[]): RemoteMachine => ({
    state: { machine: 1, name: 'box', state: 'connected', why: null, attempt: null },
    conversations: ids.map((id) => ({ ...terminal(id), machine: 1 })),
    heard: true,
    listedAt: Date.now(),
  })
  const withBox = (...ids: string[]) => ({ ...PROPS, sessionsQuery: { data: [] as Conversation[] } as UseQueryResult<Conversation[]>, machines: new Map([[1, machineListing(...ids)]]) })
  const rows = (sessions: readonly { id: string; status: string }[]) => sessions.map((session) => `${session.id} ${session.status}`)

  beforeEach(() => {
    vi.useFakeTimers({ now: NOW })
    daemon.removed.clear()
    daemon.load.mockReset()
    daemon.askList.mockReset()
    daemon.askList.mockResolvedValue(undefined)
  })
  afterEach(() => { vi.useRealTimers() })

  it('leaves at once when that machine said why it left', () => {
    const { result, rerender } = renderHook((props) => useSessionList(props), { initialProps: withBox('box/t3', 'box/t4') })
    daemon.removed.add('box/t4')
    rerender(withBox('box/t3'))
    expect(rows(result.current.sessions)).toEqual(['box/t3 done'])
  })

  it('leaves at once with its machine when the machine is taken off the list', () => {
    const { result, rerender } = renderHook((props) => useSessionList(props), { initialProps: withBox('box/t3') })
    rerender({ ...withBox(), machines: new Map() })
    expect(rows(result.current.sessions)).toEqual([])
  })

  it('stays through a read of this computer\'s list, and the machine is asked for its own', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const { result, rerender } = renderHook((props) => useSessionList(props), { initialProps: withBox('box/t3', 'box/t4') })
    rerender(withBox('box/t3'))
    daemon.load.mockResolvedValue([])

    await act(async () => { await vi.advanceTimersByTimeAsync(LINGER_MS + LINGER_SLACK_MS) })
    expect(rows(result.current.sessions)).toEqual(['box/t3 done', 'box/t4 unknown'])
    expect(daemon.askList).toHaveBeenCalledWith(1)
    warn.mockRestore()
  })

  it('goes once a list from that machine, a grace after the miss, does not name it', async () => {
    const warn = vi.spyOn(console, 'warn').mockImplementation(() => undefined)
    const { result, rerender } = renderHook((props) => useSessionList(props), { initialProps: withBox('box/t3', 'box/t4') })
    rerender(withBox('box/t3'))
    daemon.load.mockResolvedValue([])
    await act(async () => { await vi.advanceTimersByTimeAsync(LINGER_MS + LINGER_SLACK_MS) })

    rerender(withBox('box/t3'))
    expect(rows(result.current.sessions)).toEqual(['box/t3 done'])
    warn.mockRestore()
  })
})
