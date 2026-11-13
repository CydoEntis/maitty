import { describe, expect, it } from 'vitest'
import type { ClosedSession } from '@/api/daemon'
import { reopenRefusal, reopenRequest } from './recentSessions'

const closed = (over: Partial<ClosedSession> = {}): ClosedSession => ({
  agent: 'claude',
  session: '6f1c2a9e-1b2c-4d5e-8f90-123456789abc',
  cwd: '/dev/app',
  branch: 'dev',
  title: 'Fix the parser',
  at: 1_700_000_000,
  folder_present: true,
  ...over,
})

describe('reopening a closed session', () => {
  it('starts its agent in its folder with its conversation', () => {
    expect(reopenRequest(closed())).toMatchObject({ agent: 'claude', folder: '/dev/app', resume: '6f1c2a9e-1b2c-4d5e-8f90-123456789abc', machine: 0, branch: null, message: '' })
    expect(reopenRequest(closed({ agent: 'codex' }))?.agent).toBe('codex')
  })

  it('refuses a session whose folder is gone', () => {
    expect(reopenRefusal(closed({ folder_present: false }))).toBe('folder removed')
    expect(reopenRequest(closed({ folder_present: false }))).toBeNull()
  })

  it('refuses an id the shared rule would not type', () => {
    for (const session of ['--dangerous', 'a b', 'a;rm', '']) {
      expect(reopenRefusal(closed({ session }))).toBe('no conversation to resume')
      expect(reopenRequest(closed({ session }))).toBeNull()
    }
  })

  it('refuses an agent it cannot resume', () => {
    expect(reopenRefusal(closed({ agent: 'gemini' }))).toBe('cannot be reopened here')
    expect(reopenRequest(closed({ agent: 'gemini' }))).toBeNull()
  })
})
