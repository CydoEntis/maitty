import { describe, expect, it } from 'vitest'
import { heldView, LINGER_MS, pruneMissing, trackMissing } from './lingering'
import type { SessionView } from './session'

const session = (id: string): SessionView => ({ id, status: 'working' }) as SessionView
const never = () => false

describe('sessions a list misses', () => {
  it('holds a session the list stopped naming, from the moment it was missed', () => {
    const held = trackMissing(new Map(), [session('a'), session('b')], [session('a')], 100, never)
    expect([...held.keys()]).toEqual(['b'])
    expect(held.get('b')?.since).toBe(100)
  })

  it('keeps the first time it was missed across later lists, and lets go after the grace', () => {
    const first = trackMissing(new Map(), [session('a')], [], 100, never)
    const again = trackMissing(first, [], [], 100 + LINGER_MS - 1, never)
    expect(again.get('a')?.since).toBe(100)
    expect(trackMissing(again, [], [], 100 + LINGER_MS, never).size).toBe(0)
    expect(pruneMissing(first, 100 + LINGER_MS).size).toBe(0)
  })

  it('stops holding a session the list names again', () => {
    const first = trackMissing(new Map(), [session('a')], [], 100, never)
    expect(trackMissing(first, [], [session('a')], 200, never).size).toBe(0)
  })

  it('does not hold a session this window closed', () => {
    expect(trackMissing(new Map(), [session('a')], [], 100, (id) => id === 'a').size).toBe(0)
  })

  it('shows a held session as unknown', () => {
    expect(heldView({ session: session('a'), since: 1 }).status).toBe('unknown')
  })
})
