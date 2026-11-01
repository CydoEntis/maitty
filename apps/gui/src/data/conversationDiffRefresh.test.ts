import { describe, expect, it } from 'vitest'
import { conversationDiffRefreshInterval } from './conversationDiffRefresh'

describe('conversation changes refresh', () => {
  it('stops polling after a Git query fails', () => {
    expect(conversationDiffRefreshInterval(true, 'error')).toBe(false)
  })

  it('refreshes only after a desktop diff has loaded', () => {
    expect(conversationDiffRefreshInterval(true, 'pending')).toBe(false)
    expect(conversationDiffRefreshInterval(true, 'success')).toBe(5000)
  })

  it('does not poll browser preview data', () => {
    expect(conversationDiffRefreshInterval(false, 'success')).toBe(false)
  })
})
