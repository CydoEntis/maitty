import { describe, expect, it } from 'vitest'
import type { SessionUsage } from '@/api/usage'
import { ariaSortOf, budgetRing, dayBars, DASH, DEFAULT_SORT, formatCost, formatLastActive, formatTokenCount, formatWorking, nextSort, parseBudget, sortSessions, totalTokens, windowLabel } from './model'

function row(session: string, overrides: Partial<SessionUsage> = {}): SessionUsage {
  return { session, term: null, agent: 'claude', project: 'shop', firstDay: '2026-11-04', lastDay: '2026-11-04', tokensIn: 100, tokensOut: 50, cost: 1, turns: 1, workingSecs: 60, ...overrides }
}
const name = (r: SessionUsage) => r.session

describe('formatting', () => {
  it('shows a dash for every figure nobody reported, and a real zero as zero', () => {
    expect(formatCost(null)).toBe(DASH)
    expect(formatTokenCount(null)).toBe(DASH)
    expect(formatCost(0)).toBe('$0.00')
    expect(formatTokenCount(0)).toBe('0')
  })

  it('writes cost in dollars and never rounds a real cost down to free', () => {
    expect(formatCost(0.754)).toBe('$0.75')
    expect(formatCost(12)).toBe('$12.00')
    expect(formatCost(0.002)).toBe('<$0.01')
  })

  it('writes tokens compactly', () => {
    expect(formatTokenCount(812)).toBe('812')
    expect(formatTokenCount(12_400)).toBe('12k')
    expect(formatTokenCount(1_300_000)).toBe('1.3M')
  })

  it('writes working time in the unit that fits', () => {
    expect(formatWorking(42)).toBe('42s')
    expect(formatWorking(720)).toBe('12m')
    expect(formatWorking(3900)).toBe('1h 05m')
  })

  it('adds input and output only when one of them was reported', () => {
    expect(totalTokens({ tokensIn: null, tokensOut: null })).toBeNull()
    expect(totalTokens({ tokensIn: 10, tokensOut: null })).toBe(10)
    expect(totalTokens({ tokensIn: 10, tokensOut: 5 })).toBe(15)
  })

  it('says when a session was last active, counted in UTC days from the last day of the report', () => {
    expect(formatLastActive('2026-11-04', '2026-11-04')).toBe('Today')
    expect(formatLastActive('2026-11-03', '2026-11-04')).toBe('Yesterday')
    expect(formatLastActive('2026-10-30', '2026-11-04')).toBe('Oct 30')
  })

  it('names the periods as the rolling windows they are', () => {
    expect(windowLabel('day')).toBe('Today (UTC)')
    expect(windowLabel('week')).toBe('Last 7 days')
    expect(windowLabel('month')).toBe('Last 30 days')
  })
})

describe('sorting', () => {
  it('starts on cost, biggest first', () => {
    expect(DEFAULT_SORT).toEqual({ key: 'cost', direction: 'descending' })
  })

  it('starts text columns A to Z and number columns biggest first, then flips on a second click', () => {
    expect(nextSort(DEFAULT_SORT, 'project')).toEqual({ key: 'project', direction: 'ascending' })
    expect(nextSort(DEFAULT_SORT, 'turns')).toEqual({ key: 'turns', direction: 'descending' })
    expect(nextSort({ key: 'turns', direction: 'descending' }, 'turns')).toEqual({ key: 'turns', direction: 'ascending' })
  })

  it('reports aria-sort only on the sorted column', () => {
    expect(ariaSortOf(DEFAULT_SORT, 'cost')).toBe('descending')
    expect(ariaSortOf(DEFAULT_SORT, 'turns')).toBe('none')
  })

  it('puts unreported figures last in either direction', () => {
    const rows = [row('a', { cost: null }), row('b', { cost: 2 }), row('c', { cost: 1 })]
    expect(sortSessions(rows, { key: 'cost', direction: 'descending' }, name).map((r) => r.session)).toEqual(['b', 'c', 'a'])
    expect(sortSessions(rows, { key: 'cost', direction: 'ascending' }, name).map((r) => r.session)).toEqual(['c', 'b', 'a'])
  })

  it('sorts text without regard to case and dates by day', () => {
    const rows = [row('a', { project: 'Zed', lastDay: '2026-11-01' }), row('b', { project: 'api', lastDay: '2026-11-04' })]
    expect(sortSessions(rows, { key: 'project', direction: 'ascending' }, name).map((r) => r.session)).toEqual(['b', 'a'])
    expect(sortSessions(rows, { key: 'lastDay', direction: 'descending' }, name).map((r) => r.session)).toEqual(['b', 'a'])
  })

  it('does not change the rows it was given', () => {
    const rows = [row('a', { cost: 1 }), row('b', { cost: 2 })]
    sortSessions(rows, DEFAULT_SORT, name)
    expect(rows.map((r) => r.session)).toEqual(['a', 'b'])
  })
})

describe('budget ring', () => {
  it('is drawn only with a budget, a known cost and the month window', () => {
    expect(budgetRing('month', 10, null)).toBeNull()
    expect(budgetRing('month', null, 100)).toBeNull()
    expect(budgetRing('week', 10, 100)).toBeNull()
    expect(budgetRing('month', 10, 100)).toEqual({ fraction: 0.1, percent: 10, tone: 'accent' })
  })

  it('turns warn from 70 percent and danger from 90, and stops filling at the budget', () => {
    expect(budgetRing('month', 70, 100)?.tone).toBe('warn')
    expect(budgetRing('month', 90, 100)?.tone).toBe('danger')
    expect(budgetRing('month', 150, 100)).toMatchObject({ fraction: 1, percent: 150, tone: 'danger' })
  })
})

describe('budget input', () => {
  it('reads an amount, with or without a dollar sign, and empty as no budget', () => {
    expect(parseBudget('50')).toEqual({ ok: true, value: 50 })
    expect(parseBudget(' $12.50 ')).toEqual({ ok: true, value: 12.5 })
    expect(parseBudget('')).toEqual({ ok: true, value: null })
  })

  it('refuses anything else', () => {
    for (const text of ['abc', '-5', '0', '1e3', '1.234', '5000000', '1,000']) expect(parseBudget(text)).toEqual({ ok: false })
  })
})

describe('day bars', () => {
  it('gives every day of the window a slot and scales bars to the tallest day', () => {
    const none = { cost: null, turns: 0, workingSecs: 0 }
    const bars = dayBars('2026-11-02', '2026-11-04', [
      { day: '2026-11-02', tokensIn: 100, tokensOut: 100, ...none },
      { day: '2026-11-04', tokensIn: 50, tokensOut: null, ...none },
    ])
    expect(bars.map((bar) => bar.day)).toEqual(['2026-11-02', '2026-11-03', '2026-11-04'])
    expect(bars.map((bar) => bar.fraction)).toEqual([1, 0, 0.25])
    expect(bars[1]?.tokens).toBeNull()
  })
})
