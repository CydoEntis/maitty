import { describe, expect, it } from 'vitest'
import { previewConversations } from './preview'
import { markConversationsDisconnected } from './connection'

describe('disconnected conversations', () => {
  it('marks cached session states unknown while retaining their last known status', () => {
    const [working] = previewConversations
    if (!working) throw new Error('Expected a preview conversation')

    const [disconnected] = markConversationsDisconnected([working])

    expect(disconnected).toMatchObject({
      id: working.id,
      status: 'unknown',
      lastKnownStatus: working.status,
    })
  })

  it('does not replace a last known state when an already disconnected session is disconnected again', () => {
    const [working] = previewConversations
    if (!working) throw new Error('Expected a preview conversation')

    const [disconnected] = markConversationsDisconnected(markConversationsDisconnected([working]))

    expect(disconnected?.lastKnownStatus).toBe(working.status)
  })
})
