import { describe, expect, it, vi } from 'vitest'
import type { SessionStatus, SessionView } from '@/features/workspace/model/session'
import { SELF_DISMISS_MS, sessionNoticeFor } from './noticeForSession'

function session(status: SessionStatus, asking = '', extra: Partial<SessionView> = {}): SessionView {
  return { id: 't1', name: null, title: '(root)', process: 'codex', cwdLabel: '(root)', status, asking, ask: null, ...extra } as unknown as SessionView
}
const project = { name: 'maitty' }

describe('session notice', () => {
  it('names the session and what happened in the title, and what it is asking in the body', () => {
    const notice = sessionNoticeFor(session('needs', '$ git push origin main'), project, { open: vi.fn(), allow: vi.fn() })
    expect(notice).toMatchObject({ kind: 'needs', title: '(root) needs you', body: 'Wants to run $ git push origin main' })
  })

  it('offers Allow, then Open session, for a session stopped on a choice, and waits for you', () => {
    const allow = vi.fn()
    const notice = sessionNoticeFor(session('needs', '$ git push'), project, { open: vi.fn(), allow })
    expect(notice?.actions.map((action) => action.label)).toEqual(['Allow', 'Open session'])
    expect(notice?.actions[0]?.primary).toBe(true)
    expect(notice?.countdownMs).toBeUndefined()
  })

  it('offers only Open when nothing is being asked, or when it cannot be answered', () => {
    expect(sessionNoticeFor(session('needs'), project, { open: vi.fn(), allow: vi.fn() })?.actions.map((action) => action.label)).toEqual(['Open session'])
    expect(sessionNoticeFor(session('needs', '$ ls'), project, { open: vi.fn(), allow: null })?.actions.map((action) => action.label)).toEqual(['Open session'])
  })

  it('says what a Claude permission prompt asks from its hook, and offers Allow for it', () => {
    const ask = { tool: 'Write', detail: 'src/notes.txt', answerable: true }
    const notice = sessionNoticeFor(session('needs', '', { process: 'claude', ask }), project, { open: vi.fn(), allow: vi.fn() })
    expect(notice?.body).toBe('Wants to run Write · src/notes.txt')
    expect(notice?.actions.map((action) => action.label)).toEqual(['Allow', 'Open session'])
  })

  it('never shows Claude\'s screen text, and offers no Allow for a question that is not a permission prompt', () => {
    const banner = 'Claude Code v2.1.294 Sonnet 5.5 /home/vox Using Sonnet 5.5 ask me a question in 2 seconds'
    const notice = sessionNoticeFor(session('needs', banner, { process: 'claude' }), project, { open: vi.fn(), allow: vi.fn() })
    expect(notice?.body).toBe('claude is waiting on you')
    expect(notice?.actions.map((action) => action.label)).toEqual(['Open session'])
  })

  it('lets a finished or failed card go by itself, with a countdown and no Allow', () => {
    for (const status of ['done', 'error'] as const) {
      const notice = sessionNoticeFor(session(status, '$ ls'), project, { open: vi.fn(), allow: vi.fn() })
      expect(notice?.countdownMs).toBe(SELF_DISMISS_MS)
      expect(notice?.actions.map((action) => action.label)).toEqual(['Open session'])
    }
  })

  it('makes no card for a state that needs nothing from you', () => {
    expect(sessionNoticeFor(session('idle'), project, { open: vi.fn(), allow: null })).toBeNull()
  })
})
