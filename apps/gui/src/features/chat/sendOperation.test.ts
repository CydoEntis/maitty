import { beforeEach, describe, expect, it } from 'vitest'
import { forgetOperations, nextOperation, reserveOperation, STALE_OPERATION } from './sendOperation'

beforeEach(forgetOperations)

describe('nextOperation', () => {
  it('goes one past the receipt, the queue and the last number used', () => {
    expect(nextOperation({ receiptOperation: undefined, queuedOperations: [], lastUsed: 0 })).toBe(1)
    expect(nextOperation({ receiptOperation: 3, queuedOperations: [], lastUsed: 0 })).toBe(4)
    expect(nextOperation({ receiptOperation: 3, queuedOperations: [5, 4], lastUsed: 0 })).toBe(6)
    expect(nextOperation({ receiptOperation: 3, queuedOperations: [5], lastUsed: 9 })).toBe(10)
  })
})

describe('reserveOperation', () => {
  const nothing = { receipt: null, queued: [] }

  it('never hands the same number to two senders of one conversation', () => {
    expect(reserveOperation(1, nothing)).toBe(1)
    expect(reserveOperation(1, nothing)).toBe(2)
  })

  it('counts each conversation on its own', () => {
    reserveOperation(1, nothing)
    expect(reserveOperation(2, nothing)).toBe(1)
  })
})

describe('STALE_OPERATION', () => {
  it('matches the daemon\'s refusal of a passed number', () => {
    expect(STALE_OPERATION.test('operation is Stale or reused')).toBe(true)
    expect(STALE_OPERATION.test('connection lost')).toBe(false)
  })
})
