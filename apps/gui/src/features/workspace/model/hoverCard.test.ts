import { describe, expect, it } from 'vitest'
import { HOVER_CARD_WIDTH_PX, placeHoverCard, statusWhyLine } from './hoverCard'

describe('where the hover card goes', () => {
  it('sits to the right of the row, level with it', () => {
    expect(placeHoverCard({ left: 20, right: 280, top: 200 }, 180, 1400, 900)).toEqual({ left: 288, top: 200 })
  })

  it('moves up so it never runs past the bottom of the window', () => {
    expect(placeHoverCard({ left: 20, right: 280, top: 800 }, 180, 1400, 900).top).toBe(712)
  })

  it('goes to the left of a sidebar that is on the right', () => {
    expect(placeHoverCard({ left: 1120, right: 1380, top: 200 }, 180, 1400, 900).left).toBe(1120 - 8 - HOVER_CARD_WIDTH_PX)
  })
})

describe('why a session has its status', () => {
  const NOW = 1_700_000_000_000
  const set = (statusWhy: string, agoMs: number | null) => ({ status: 'working' as const, helperAsk: [], statusWhy, statusSince: agoMs === null ? null : NOW - agoMs })

  it('names the hook that set it and how long ago', () => {
    expect(statusWhyLine(set('hook: Stop', 3_000), NOW)).toBe('hook: Stop, 3 s ago')
  })

  it('names what was matched on the screen', () => {
    expect(statusWhyLine(set('screen: matched `esc to interrupt`', 60_000), NOW)).toBe('screen: matched `esc to interrupt`, 1 min ago')
  })

  it('names the process observation', () => {
    expect(statusWhyLine(set('process: an agent started', 2 * 60 * 60_000), NOW)).toBe('process: an agent started, 2 h ago')
  })

  it('counts days once it is that old', () => {
    expect(statusWhyLine(set('hook: Stop', 3 * 24 * 60 * 60_000), NOW)).toBe('hook: Stop, 3 d ago')
  })

  it('says nothing when the daemon gave no reason', () => {
    expect(statusWhyLine(set('', 3_000), NOW)).toBeNull()
  })

  it('leaves the age out when the daemon did not say when', () => {
    expect(statusWhyLine(set('hook: Stop', null), NOW)).toBe('hook: Stop')
  })

  it('says nothing for a session whose status is not known', () => {
    expect(statusWhyLine({ ...set('hook: Stop', 3_000), status: 'unknown' }, NOW)).toBeNull()
  })
})

describe('when the reason is not about the status shown', () => {
  const NOW = 1_700_000_000_000
  const set = { status: 'needs' as const, helperAsk: [] as string[], statusWhy: 'hook: PreToolUse', statusSince: NOW - 3_000 }

  it('says nothing while the session waits on you to start its helpers', () => {
    expect(statusWhyLine({ ...set, helperAsk: ['review the diff'] }, NOW)).toBeNull()
  })

  it('says nothing for a stopped session, whose reason explains idle and not the stop', () => {
    expect(statusWhyLine({ ...set, status: 'error', statusWhy: 'screen: quiet' }, NOW)).toBeNull()
  })

  it('cuts a long screen pattern short and keeps the age', () => {
    const statusWhy = `screen: matched \`${'a|'.repeat(200)}\``
    expect(statusWhyLine({ ...set, statusWhy }, NOW)).toBe(`${statusWhy.slice(0, 139)}…, 3 s ago`)
  })
})
