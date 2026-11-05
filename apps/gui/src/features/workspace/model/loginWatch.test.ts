import { describe, expect, it } from 'vitest'
import { advanceLoginWatch, LOGIN_WAIT_MS, startLoginWatch } from './loginWatch'

describe('a first login in a session', () => {
  it('waits while the session has not shown up, and while it is open', () => {
    const start = startLoginWatch('box', [1, 2], 1_000)
    const waiting = advanceLoginWatch(start, new Set([1, 2]), 2_000)
    expect(waiting).toEqual({ watch: start, dial: false })
    const open = advanceLoginWatch(start, new Set([1, 2, 7]), 3_000)
    expect(open.watch?.term).toBe(7)
    expect(open.dial).toBe(false)
    if (open.watch === null) throw new Error('the session was seen')
    expect(advanceLoginWatch(open.watch, new Set([1, 2, 7]), 9_000).dial).toBe(false)
  })

  it('dials once, when the session has closed', () => {
    const open = advanceLoginWatch(startLoginWatch('box', [1], 0), new Set([1, 7]), 1).watch
    if (open === null) throw new Error('the session was seen')
    expect(advanceLoginWatch(open, new Set([1]), 5_000)).toEqual({ watch: null, dial: true })
  })

  it('dials anyway when the session never shows up', () => {
    const start = startLoginWatch('box', [1], 0)
    expect(advanceLoginWatch(start, new Set([1]), LOGIN_WAIT_MS - 1).dial).toBe(false)
    expect(advanceLoginWatch(start, new Set([1]), LOGIN_WAIT_MS)).toEqual({ watch: null, dial: true })
  })

  it('does not take a session that was already there for the login', () => {
    const start = startLoginWatch('box', [1, 2], 0)
    expect(advanceLoginWatch(start, new Set([2]), 100).watch?.term).toBeNull()
  })
})
