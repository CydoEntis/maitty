import { describe, expect, it } from 'vitest'
import { RECONNECT_TRIES, serviceStateOf, tryNumber } from './serviceState'

describe('the service state', () => {
  it('reconnects for four failed tries and is down at the fifth', () => {
    expect(serviceStateOf({ online: true, failures: 0, mismatch: false })).toBe('ok')
    expect(serviceStateOf({ online: false, failures: 1, mismatch: false })).toBe('reconnecting')
    expect(serviceStateOf({ online: false, failures: RECONNECT_TRIES - 1, mismatch: false })).toBe('reconnecting')
    expect(serviceStateOf({ online: false, failures: RECONNECT_TRIES, mismatch: false })).toBe('down')
  })
  it('asks for an update, however many tries failed, when the service is another version', () => {
    expect(serviceStateOf({ online: false, failures: 9, mismatch: true })).toBe('update')
  })
  it('numbers a try from 1 and never past the last', () => {
    expect([0, 1, 4, 5, 8].map(tryNumber)).toEqual([1, 1, 4, 5, 5])
  })
})
