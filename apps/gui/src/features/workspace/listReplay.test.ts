import type { UseQueryResult } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { z } from 'zod'
import { conversationsFromSnapshot } from '@/api/daemon'
import type { Conversation } from '@/data/preview'
import { EMPTY_LAYOUT } from './model/layout'
import { LINGER_MS, LINGER_SLACK_MS } from './model/lingering'
import { useSessionList } from './useSessionList'
import fixture from './listReplay.fixture.json?raw'

vi.mock('@tauri-apps/api/core', () => ({ invoke: vi.fn(), isTauri: () => true }))

// The fixture is built by scripts/build-list-replay.ts, not recorded from a daemon (docs/TESTING.md).
const ListSchema = z.object({ revision: z.number(), terms: z.record(z.string(), z.object({ remote: z.string().nullable() })), removed: z.record(z.string(), z.string()) })
const StepSchema = z.object({ after: z.number().nonnegative(), event: z.enum(['list_sessions', 'sessions-changed']), note: z.string().optional(), payload: z.unknown() })
const STEPS = z.object({ steps: z.array(StepSchema) }).parse(JSON.parse(fixture)).steps

const NOW = new Date(2026, 0, 5, 9, 30).getTime()
const idOf = (pane: string) => `t${pane}`

type Fault = { step: number; vanishedWithoutReason: string[]; notListed: string[]; listedThoughGone: string[] }

/**
 * Feeds each list to the window as the app does: through `conversationsFromSnapshot` into the sessions query that
 * `useSessionList` reads, with time passing between lists. A null `sessions-changed` (the host lost the daemon) only
 * sends the query to ask again, so the window keeps the list it had until the answer comes as `list_sessions`.
 */
async function replay() {
  let daemonList: unknown = null
  vi.mocked(invoke).mockImplementation((command) => (command === 'list_sessions' ? Promise.resolve(daemonList) : Promise.reject(new Error(`unexpected ${command}`))))
  let data: Conversation[] = []
  const props = () => ({ sessionsQuery: { data } as UseQueryResult<Conversation[]>, online: true, machines: new Map(), home: '/home/me', layout: EMPTY_LAYOUT })
  const { result, rerender } = renderHook((current) => useSessionList(current), { initialProps: props() })
  const rows = () => new Set(result.current.sessions.map((session) => session.id))
  const listed = new Set<string>()
  const goneWithReason = new Set<string>()
  const faults: Fault[] = []
  for (const [step, { after, payload }] of STEPS.entries()) {
    const before = rows()
    await act(async () => { await vi.advanceTimersByTimeAsync(after) })
    const reasons = new Set<string>()
    if (payload !== null) {
      daemonList = payload
      const list = ListSchema.parse(payload)
      for (const pane of Object.keys(list.terms)) { listed.add(idOf(pane)); goneWithReason.delete(idOf(pane)) }
      for (const pane of Object.keys(list.removed)) { reasons.add(idOf(pane)); listed.delete(idOf(pane)); goneWithReason.add(idOf(pane)) }
      data = conversationsFromSnapshot(payload)
      rerender(props())
    }
    const now = rows()
    const fault = {
      step,
      vanishedWithoutReason: [...before].filter((id) => !now.has(id) && !reasons.has(id)),
      notListed: [...listed].filter((id) => !now.has(id)),
      listedThoughGone: [...goneWithReason].filter((id) => now.has(id)),
    }
    if (fault.vanishedWithoutReason.length + fault.notListed.length + fault.listedThoughGone.length > 0) faults.push(fault)
  }
  await act(async () => { await vi.advanceTimersByTimeAsync(2 * (LINGER_MS + LINGER_SLACK_MS)) })
  return { faults, end: [...rows()].sort(), listed: [...listed].sort() }
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
    expect(lists.some((list, index) => index > 0 && list.revision === 1 && (lists[index - 1]?.revision ?? 0) > 1)).toBe(true)
    expect(lists.some((list, index) => index > 0 && list.revision === (lists[index - 1]?.revision ?? 0) + 2)).toBe(true)
    const closedAt = lists.findIndex((list) => list.removed['5'] === 'Closed')
    expect(closedAt).toBeGreaterThan(0)
    expect(lists.slice(closedAt + 1).some((list) => '5' in list.terms)).toBe(true)
    expect(lists.every((list) => list.terms['3']?.remote === 'build-box')).toBe(true)
  })

  it('never drops a row the daemon gave no reason for, and lists every session not removed with a reason at the end', async () => {
    const { faults, end, listed } = await replay()
    expect(faults).toEqual([])
    expect(end).toEqual(listed)
    expect(end).toEqual(['t1', 't2', 't3', 't5', 't6'])
  })
})
