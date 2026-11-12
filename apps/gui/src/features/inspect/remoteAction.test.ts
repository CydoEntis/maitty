import { describe, expect, it } from 'vitest'
import { remoteAction } from './remoteAction'

describe('the remote button', () => {
  it('fetches when the branch is level', () => {
    expect(remoteAction(0, 0)).toMatchObject({ action: 'fetch', label: 'Fetch', blocked: null })
  })

  it('pulls when behind and pushes when ahead, with the count', () => {
    expect(remoteAction(0, 3)).toMatchObject({ action: 'pull', label: 'Pull ↓3', blocked: null })
    expect(remoteAction(2, 0)).toMatchObject({ action: 'push', label: 'Push ↑2', blocked: null })
  })

  it('says why when the branch has diverged', () => {
    const diverged = remoteAction(1, 2)
    expect(diverged.action).toBe('pull')
    expect(diverged.blocked).toContain('cannot be fast-forwarded')
  })
})
