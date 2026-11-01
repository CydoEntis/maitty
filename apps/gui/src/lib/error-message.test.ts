import { describe, expect, it } from 'vitest'
import { errorMessage } from './error-message'

describe('errorMessage', () => {
  it('keeps useful Tauri string errors', () => {
    expect(errorMessage('Codex app-server could not start', 'Conversation failed')).toBe('Codex app-server could not start')
  })

  it('uses Error messages', () => {
    expect(errorMessage(new Error('Codex app-server could not start'), 'Conversation failed')).toBe('Codex app-server could not start')
  })

  it('uses the fallback when the failure has no message', () => {
    expect(errorMessage({ code: 'unknown' }, 'Conversation failed')).toBe('Conversation failed')
    expect(errorMessage('  ', 'Conversation failed')).toBe('Conversation failed')
  })
})
