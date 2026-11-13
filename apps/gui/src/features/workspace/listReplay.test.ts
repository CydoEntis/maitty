import { QueryClient, QueryClientProvider, useQuery, useQueryClient } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { act, renderHook } from '@testing-library/react'
import { createElement, type ReactNode, useState } from 'react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { z } from 'zod'
import { loadConversations } from '@/api/daemon'
import { EMPTY_LAYOUT } from './model/layout'
import { LINGER_MS, LINGER_SLACK_MS } from './model/lingering'
import type { RemoteMachine } from './model/machines'
import { useSessionList } from './useSessionList'
import { useSessionSync } from './useSessionSync'
import fixture from './listReplay.fixture.json?raw'

const host = vi.hoisted(() => ({ onSessionsChanged: null as ((event: { payload: unknown }) => void) | null }))
vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => true }))
vi.mock('@tauri-apps/api/event', () => ({
  listen: (event: string, handler: (event: { payload: unknown }) => void) => {
    if (event === 'sessions-changed') host.onSessionsChanged = handler
    return Promise.resolve(() => undefined)
  },
}))

// The fixture is built by scripts/build-list-replay.ts, not recorded from a daemon (docs/TESTING.md). Its first step
// is what the daemon answers the window's first read; every later one is a `sessions-changed` the host sends, null
// when it lost the daemon.
const ListSchema = z.object({
  revision: z.number(),
  terms: z.record(z.string(), z.object({ process: z.string(), remote: z.string().nullable() })),
  removed: z.record(z.string(), z.string()),
})
const StepSchema = z.object({ after: z.number().nonnegative(), event: z.enum(['list_sessions', 'sessions-changed']), note: z.string().optional(), payload: z.unknown() })
const STEPS = z.object({ steps: z.array(StepSchema) }).parse(JSON.parse(fixture)).steps

const NOW = new Date(2026, 0, 5, 9, 30).getTime()
const NO_MACHINES: ReadonlyMap<number, RemoteMachine> = new Map()
const MACHINES_REF = { current: NO_MACHINES }
const idOf = (pane: string) => `t${pane}`
const noop = () => undefined

/** The window's own list code: the conversations query, the sidebar's list, and the sync that hears the host. */
function useWindowList() {
  const sessionsQuery = useQuery({ queryKey: ['conversations'], queryFn: loadConversations })
  const queryClient = useQueryClient()
  const [layout, setLayout] = useState(EMPTY_LAYOUT)
  const { sessions } = useSessionList({ sessionsQuery, online: true, machines: NO_MACHINES, home: '/home/me', layout })
  useSessionSync({ sessionsQuery, retryDelay: 1000, queryClient, setFailures: noop, sessions, setLayout, machinesRef: MACHINES_REF, visibleKey: '', flash: noop })
  return sessions
}

type Fault = { step: number; vanishedWithoutReason: string[]; notListed: string[]; listedThoughGone: string[] }
type Read = { at: number; ids: ReadonlySet<string> }

/**
 * Plays the fixture into the window's hooks on fake timers, with the daemon answering `list_sessions` (the window's
 * reads and the confirming reads alike) with its list as it stands, or failing while it restarts. A row may leave
 * only for a removal reason in the update that drops it, or when a full read asked for at least `LINGER_MS` after a
 * list left it out does not name it; any other leaving is a fault.
 *
 * `api/daemon.ts` keeps the reasons and revision it last heard in module state, so the file replays once: a second
 * replay would start with the first one's.
 */
async function replay() {
  let daemon: unknown = null
  const reads: Read[] = []
  vi.mocked(invoke).mockImplementation((command) => {
    if (command !== 'list_sessions') return Promise.reject(new Error(`unexpected ${command}`))
    if (daemon === null) return Promise.reject(new Error('the daemon is restarting'))
    reads.push({ at: Date.now(), ids: new Set(Object.keys(ListSchema.parse(daemon).terms).map(idOf)) })
    return Promise.resolve({ ...(daemon as object), removed: {} })
  })
  const queryClient = new QueryClient({ defaultOptions: { queries: { retry: false } } })
  const wrapper = ({ children }: { children: ReactNode }) => createElement(QueryClientProvider, { client: queryClient }, children)
  const [first, ...updates] = STEPS
  daemon = first?.payload ?? null
  const { result } = renderHook(useWindowList, { wrapper })
  await act(async () => { await vi.advanceTimersByTimeAsync(0) })

  const rows = () => new Set(result.current.map((session) => session.id))
  const listed = new Set(rows())
  const goneWithReason = new Set<string>()
  const confirmedGone = new Set<string>()
  const unlistedSince = new Map<string, number>()
  const faults: Fault[] = []
  const confirmed = (id: string) => {
    const since = unlistedSince.get(id)
    return since !== undefined && reads.some((read) => read.at >= since + LINGER_MS && !read.ids.has(id))
  }
  const check = (step: number, before: ReadonlySet<string>, reasons: ReadonlySet<string>) => {
    const now = rows()
    const vanishedWithoutReason: string[] = []
    for (const id of [...before].filter((row) => !now.has(row) && !reasons.has(row))) {
      if (!confirmed(id)) { vanishedWithoutReason.push(id); continue }
      listed.delete(id)
      confirmedGone.add(id)
      unlistedSince.delete(id)
    }
    const fault = { step, vanishedWithoutReason, notListed: [...listed].filter((id) => !now.has(id)), listedThoughGone: [...goneWithReason].filter((id) => now.has(id)) }
    if (vanishedWithoutReason.length + fault.notListed.length + fault.listedThoughGone.length > 0) faults.push(fault)
  }

  for (const [index, { after, payload }] of updates.entries()) {
    const step = index + 1
    const waited = rows()
    await act(async () => { await vi.advanceTimersByTimeAsync(after) })
    check(step, waited, new Set())

    const before = rows()
    daemon = payload
    await act(async () => {
      host.onSessionsChanged?.({ payload })
      await vi.advanceTimersByTimeAsync(0)
    })
    const reasons = new Set<string>()
    if (payload !== null) {
      const list = ListSchema.parse(payload)
      const named = new Set(Object.keys(list.terms).map(idOf))
      for (const id of Object.keys(list.removed).map(idOf)) { reasons.add(id); listed.delete(id); goneWithReason.add(id); unlistedSince.delete(id) }
      for (const id of listed) if (!named.has(id) && !unlistedSince.has(id)) unlistedSince.set(id, Date.now())
      for (const id of named) { listed.add(id); goneWithReason.delete(id); confirmedGone.delete(id); unlistedSince.delete(id) }
    }
    check(step, before, reasons)
  }
  const settled = rows()
  await act(async () => { await vi.advanceTimersByTimeAsync(2 * (LINGER_MS + LINGER_SLACK_MS)) })
  check(STEPS.length, settled, new Set())
  return { faults, end: [...rows()].sort(), listed: [...listed].sort(), confirmedGone: [...confirmedGone] }
}

describe('replaying the list updates of a working day', () => {
  beforeEach(() => {
    vi.useFakeTimers({ now: NOW })
    vi.spyOn(console, 'warn').mockImplementation(() => undefined)
  })
  afterEach(() => {
    vi.useRealTimers()
    vi.restoreAllMocks()
  })

  it('holds at least 200 updates with a daemon restart, a lost update, an undone close and an ssh pane', () => {
    const lists = STEPS.flatMap((step) => (step.payload === null ? [] : [ListSchema.parse(step.payload)]))
    const updates = STEPS.filter((step) => step.event === 'sessions-changed' && step.payload !== null)
    expect(updates.length).toBeGreaterThanOrEqual(200)
    const restart = lists.findIndex((list, index) => index > 0 && list.revision === 1 && (lists[index - 1]?.revision ?? 0) > 1)
    const beforeRestart = lists[restart - 1]?.terms ?? {}
    const afterRestart = lists[restart]
    expect(afterRestart?.removed).toEqual({})
    expect(Object.entries(afterRestart?.terms ?? {}).some(([pane, term]) => beforeRestart[pane] !== undefined && beforeRestart[pane].process !== term.process)).toBe(true)
    expect(lists.some((list, index) => index > 0 && list.revision === (lists[index - 1]?.revision ?? 0) + 2)).toBe(true)
    const closedAt = lists.findIndex((list) => list.removed['5'] === 'Closed')
    expect(closedAt).toBeGreaterThan(0)
    expect(lists.slice(closedAt + 1).some((list) => '5' in list.terms)).toBe(true)
    expect(lists.every((list) => Object.values(list.terms).some((term) => term.remote === 'build-box'))).toBe(true)
  })

  it('drops a row the daemon gave no reason for only once a full read confirms it gone, and lists every other session at the end', async () => {
    const { faults, end, listed, confirmedGone } = await replay()
    expect(faults).toEqual([])
    expect(confirmedGone).toEqual(['t6'])
    expect(end).toEqual(listed)
    expect(end).toEqual(['t1', 't2', 't3', 't4', 't5'])
  })
})
