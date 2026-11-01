import { describe, expect, it } from 'vitest'
import { conversationDiffRefreshInterval } from './conversationDiffRefresh'

describe('conversation changes refresh', () => {
  it('stops polling after a Git query fails', () => {
    expect(conversationDiffRefreshInterval(true, 'error')).toBe(false)
  })

  it('refreshes a desktop diff while it is loading or current', () => {
    expect(conversationDiffRefreshInterval(true, 'pending')).toBe(3000)
    expect(conversationDiffRefreshInterval(true, 'success')).toBe(3000)
  })

  it('does not poll browser preview data', () => {
    expect(conversationDiffRefreshInterval(false, 'success')).toBe(false)
  })
})
