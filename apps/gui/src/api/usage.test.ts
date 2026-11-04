import { beforeEach, describe, expect, it, vi } from 'vitest'
import { loadUsageReport } from './usage'

const shell = vi.hoisted(() => ({ isTauri: true, invoke: vi.fn<(command: string, args?: unknown) => Promise<unknown>>() }))
vi.mock('@tauri-apps/api/core', () => ({ isTauri: () => shell.isTauri, invoke: shell.invoke }))

/** The shape the daemon sends (a `UsageReport` serialized by serde in camelCase). */
function report(overrides: Record<string, unknown> = {}) {
  const none = { tokensIn: null, tokensOut: null, cost: null, turns: 0, workingSecs: 0 }
  return {
    period: 'Week',
    from: '2026-10-29',
    to: '2026-11-04',
    retentionDays: 90,
    totals: { tokensIn: 1200, tokensOut: 300, cost: 0.75, turns: 4, workingSecs: 600 },
    sessionCount: 2,
    sessionsWithoutCost: 1,
    sessions: [
      { session: 'a', term: 3, agent: 'claude', project: 'shop', firstDay: '2026-11-03', lastDay: '2026-11-04', tokensIn: 1000, tokensOut: 200, cost: 0.75, turns: 3, workingSecs: 500 },
      { session: 'b', term: null, agent: 'codex', project: 'api', firstDay: '2026-11-04', lastDay: '2026-11-04', ...none, tokensIn: 200, tokensOut: 100, turns: 1, workingSecs: 100 },
    ],
    days: [{ day: '2026-11-04', ...none, turns: 4 }],
    ...overrides,
  }
}

beforeEach(() => {
  shell.isTauri = true
  shell.invoke.mockReset()
})

describe('usage report', () => {
  it('asks the host for the period and keeps unreported figures as null', async () => {
    shell.invoke.mockResolvedValue(report())
    const result = await loadUsageReport('week')
    expect(shell.invoke).toHaveBeenCalledWith('usage_report', { period: 'week' })
    expect(result.sessions[1]?.cost).toBeNull()
    expect(result.sessions[1]?.term).toBeNull()
    expect(result.sessionsWithoutCost).toBe(1)
  })

  it('refuses an answer the daemon should never give', async () => {
    shell.invoke.mockResolvedValue(report({ totals: { tokensIn: 1, tokensOut: 1, cost: -2, turns: 0, workingSecs: 0 } }))
    await expect(loadUsageReport('day')).rejects.toThrow()
    shell.invoke.mockResolvedValue(report({ from: 'yesterday' }))
    await expect(loadUsageReport('day')).rejects.toThrow()
  })

  it('says so outside the desktop app', async () => {
    shell.isTauri = false
    await expect(loadUsageReport('month')).rejects.toThrow('desktop app')
    expect(shell.invoke).not.toHaveBeenCalled()
  })
})
