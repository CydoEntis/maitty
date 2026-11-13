import type { UseQueryResult } from '@tanstack/react-query'
import { act, renderHook } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { EMPTY_LAYOUT } from './model/layout'
import { useSessionList } from './useSessionList'

const NOW = new Date(2026, 0, 5, 9, 30).getTime()
const WAIT_MS = 5 * 60_000

/** One agent the daemon says finished and continues after its plan limit in five minutes. */
const LISTED: Conversation[] = [{
  id: 't1', generation: 0, kind: 'terminal', title: '', project: '', folder: '/home/me/app', cwd: '/home/me/app', repo: null, branch: 'working tree', provider: 'Claude', status: 'done', updated: null, model: '', permission: '', context: null, nativePane: 1, hasDiff: false, agent: 'claude',
  details: { asking: '', interrupted: null, timeline: [], usage: { context: null, cost: null, tokens: null }, resumeAt: (NOW + WAIT_MS) / 1000 },
}]
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
