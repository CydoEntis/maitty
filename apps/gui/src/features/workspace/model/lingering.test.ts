import { describe, expect, it } from 'vitest'
import { confirmGone, confirmGoneOnMachines, heldView, LINGER_MS, nextCheckAt, trackMissing } from './lingering'
import type { SessionView } from './session'

const session = (id: string): SessionView => ({ id, status: 'working' }) as SessionView
const never = () => false
const missAt = (at: number) => ({ at, listedIn: 4, missedIn: 5 })

describe('sessions a list misses', () => {
  it('holds a session the list stopped naming, with when and in which revisions it was missed', () => {
    const held = trackMissing(new Map(), [session('a'), session('b')], [session('a')], missAt(100), never)
    expect([...held.keys()]).toEqual(['b'])
    expect(held.get('b')?.miss).toEqual({ at: 100, listedIn: 4, missedIn: 5 })
  })

  it('keeps holding it through later lists however long it takes, with its first miss', () => {
    const first = trackMissing(new Map(), [session('a')], [], missAt(100), never)
    const later = trackMissing(first, [], [], missAt(100 + 10 * LINGER_MS), never)
    expect(later.get('a')).toBe(first.get('a'))
  })

  it('stops holding a session the list names again', () => {
    const first = trackMissing(new Map(), [session('a')], [], missAt(100), never)
    expect(trackMissing(first, [], [session('a')], missAt(200), never).size).toBe(0)
  })

  it('does not hold a session that left with a reason', () => {
    expect(trackMissing(new Map(), [session('a')], [], missAt(100), (id) => id === 'a').size).toBe(0)
  })

  it('lets go of a held session once its reason is heard', () => {
    const first = trackMissing(new Map(), [session('a')], [], missAt(100), never)
    expect(trackMissing(first, [], [], missAt(200), (id) => id === 'a').size).toBe(0)
  })

  it('shows a held session as unknown', () => {
    expect(heldView({ session: session('a'), miss: missAt(1) }).status).toBe('unknown')
  })
})

describe('confirming a held session is gone', () => {
  const held = trackMissing(new Map(), [session('a'), session('b')], [], missAt(100), never)

  it('drops a held session that a full list asked for after the miss does not name', () => {
    expect([...confirmGone(held, new Set(['b']), 150).keys()]).toEqual(['b'])
  })

  it('keeps a session missed after the full list was asked for', () => {
    expect(confirmGone(held, new Set(), 50)).toBe(held)
  })

  it('never takes this computer\'s list as word on another machine\'s session', () => {
    const remote = trackMissing(new Map(), [session('box/t3')], [], missAt(100), never)
    expect(confirmGone(remote, new Set(), 150)).toBe(remote)
  })

  it('drops another machine\'s held session when a list of that machine, a grace after the miss, does not name it', () => {
    const remote = trackMissing(new Map(), [session('box/t3'), session('box/t4'), session('a')], [], missAt(100), never)
    const listed = (listedAt: number) => new Map([['box', { listedAt, ids: new Set(['box/t4']) }]])
    expect(confirmGoneOnMachines(remote, listed(100 + LINGER_MS - 1))).toBe(remote)
    expect([...confirmGoneOnMachines(remote, listed(100 + LINGER_MS)).keys()]).toEqual(['box/t4', 'a'])
    expect(confirmGoneOnMachines(remote, new Map())).toBe(remote)
  })

  it('checks a grace after the first miss, and again a grace after the last check', () => {
    expect(nextCheckAt(new Map(), 0)).toBeNull()
    expect(nextCheckAt(held, 0)).toBe(100 + LINGER_MS)
    expect(nextCheckAt(held, 100 + LINGER_MS)).toBe(100 + 2 * LINGER_MS)
  })
})
