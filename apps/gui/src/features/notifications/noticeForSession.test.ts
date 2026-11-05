import { describe, expect, it, vi } from 'vitest'
import type { SessionStatus, SessionView } from '@/features/workspace/model/session'
import { SELF_DISMISS_MS, sessionNoticeFor } from './noticeForSession'

function session(status: SessionStatus, asking = ''): SessionView {
  return { id: 't1', name: null, title: '(root)', process: 'codex', cwdLabel: '(root)', status, asking } as unknown as SessionView
}
const project = { name: 'maitty' }

describe('session notice', () => {
  it('names the kind and project in the title, and what it is asking in the body', () => {
    const notice = sessionNoticeFor(session('needs', '$ git push origin main'), project, { open: vi.fn(), allow: vi.fn() })
    expect(notice).toMatchObject({ kind: 'needs', title: 'Needs you · maitty', body: 'codex in (root): $ git push origin main' })
  })

  it('offers Allow once, then Open, for a session stopped on a choice, and waits for you', () => {
    const allow = vi.fn()
    const notice = sessionNoticeFor(session('needs', '$ git push'), project, { open: vi.fn(), allow })
    expect(notice?.actions.map((action) => action.label)).toEqual(['Allow once', 'Open'])
    expect(notice?.actions[0]?.primary).toBe(true)
    expect(notice?.countdownMs).toBeUndefined()
  })

  it('offers only Open when nothing is being asked, or when it cannot be answered', () => {
    expect(sessionNoticeFor(session('needs'), project, { open: vi.fn(), allow: vi.fn() })?.actions.map((action) => action.label)).toEqual(['Open'])
    expect(sessionNoticeFor(session('needs', '$ ls'), project, { open: vi.fn(), allow: null })?.actions.map((action) => action.label)).toEqual(['Open'])
  })

  it('lets a finished or failed card go by itself, with a countdown and no Allow once', () => {
    for (const status of ['done', 'error'] as const) {
      const notice = sessionNoticeFor(session(status, '$ ls'), project, { open: vi.fn(), allow: vi.fn() })
      expect(notice?.countdownMs).toBe(SELF_DISMISS_MS)
      expect(notice?.actions.map((action) => action.label)).toEqual(['Open'])
    }
  })

  it('makes no card for a state that needs nothing from you', () => {
    expect(sessionNoticeFor(session('idle'), project, { open: vi.fn(), allow: null })).toBeNull()
  })
})
