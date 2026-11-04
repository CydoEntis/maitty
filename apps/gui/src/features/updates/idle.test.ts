import { describe, expect, it } from 'vitest'
import { IDLE_INPUT_MS, idleUpdateStep, isIdleForUpdate, type IdleReading } from './idle'

const START = 1_000_000
const away: IdleReading = { now: START + IDLE_INPUT_MS, lastInputAt: START, windowActive: false, agentBusy: false }

describe('idle for an update', () => {
  it('is idle when nothing is working, nobody typed for five minutes and the window is not in front', () => {
    expect(isIdleForUpdate(away)).toBe(true)
  })

  it('is not idle one millisecond before five minutes without typing', () => {
    expect(isIdleForUpdate({ ...away, now: away.now - 1 })).toBe(false)
  })

  it('is not idle while the window is shown and in front, however long since a key', () => {
    expect(isIdleForUpdate({ ...away, now: away.now * 10, windowActive: true })).toBe(false)
  })

  it('is never idle while an agent is working or a request waits, even if the user left long ago', () => {
    expect(isIdleForUpdate({ ...away, now: away.now * 10, agentBusy: true })).toBe(false)
  })

  it('downloads an update that was found and installs one that was fetched, only when idle', () => {
    expect(idleUpdateStep(true, 'available', false)).toBe('download')
    expect(idleUpdateStep(true, 'ready', false)).toBe('install')
    expect(idleUpdateStep(false, 'available', false)).toBeNull()
    expect(idleUpdateStep(false, 'ready', false)).toBeNull()
  })

  it('leaves a skipped release alone and does nothing in other states', () => {
    expect(idleUpdateStep(true, 'available', true)).toBeNull()
    for (const status of ['idle', 'checking', 'current', 'downloading', 'failed']) expect(idleUpdateStep(true, status, false)).toBeNull()
  })
})
