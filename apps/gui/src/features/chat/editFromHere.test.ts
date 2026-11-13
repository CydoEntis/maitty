import { describe, expect, it } from 'vitest'
import { buildAttachmentMessage } from './attachmentMessage'
import type { ChatEvent } from './chatEvents'
import { confirmBody, restoreNote, turnSpanOf } from './editFromHere'
import { messageEvent } from './eventFixtures'

const turnEnd = (turnId: string, timed: boolean): ChatEvent => ({ kind: 'turn', id: `turn:${turnId}`, turnId, outcome: 'completed', startedAtMs: 1_000, endedAtMs: 9_000, timed })

describe('the note the next message starts with', () => {
  it('is one line naming the message, however many lines it had', () => {
    const note = restoreNote('Fix the parser\n\nthen run the tests')
    expect(note.split('\n')).toHaveLength(1)
    expect(note).toContain('"Fix the parser then run the tests"')
    expect(note).toMatch(/restored the files to how they were before/)
  })

  it('names a long message by its start, and an attachment message by its words', () => {
    expect(restoreNote('x'.repeat(200))).toContain(`"${'x'.repeat(80)}…"`)
    const attached = buildAttachmentMessage('look at this', [{ path: 'C:/shot.png' }])
    expect(restoreNote(attached)).toContain('"look at this"')
  })
})

describe('the turn a message started', () => {
  it('is found by the message turn, with the daemon times', () => {
    const events = [messageEvent('u1', 'user', 'Fix it'), turnEnd('t1', true)]
    expect(turnSpanOf(events, 't1')).toEqual({ startedAtMs: 1_000, endedAtMs: 9_000 })
  })

  it('has no span while it runs or when its times are not the daemon\'s', () => {
    expect(turnSpanOf([messageEvent('u1', 'user', 'Fix it')], 't1')).toBeNull()
    expect(turnSpanOf([turnEnd('t1', false)], 't1')).toBeNull()
  })
})

describe('the confirm before a restore', () => {
  it('says how many files change, that what is there is kept, and that the conversation is not rewound', () => {
    const body = confirmBody(2, 'claude', false)
    expect(body).toContain('2 files change back')
    expect(body).toContain('"before a restore" snapshot')
    expect(body).toContain("claude's conversation is not rewound")
  })

  it('says where a draft already in the box goes', () => {
    expect(confirmBody(1, 'codex', true)).toContain('above what you have typed')
  })
})
