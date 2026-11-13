import { describe, expect, it } from 'vitest'
import { buildAttachmentMessage, parseAttachmentMessage } from './attachmentMessage'
import type { ChatEvent } from './chatEvents'
import { confirmBody, restoreNote, stripRestoreNote, turnSpanOf, withRestoreNote } from './editFromHere'
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

describe('a message that already carried a note', () => {
  it('is read without it, so editing from it again does not bring the old note back', () => {
    const sent = `${restoreNote('first try')}\nsecond try`
    expect(stripRestoreNote(sent)).toBe('second try')
    expect(restoreNote(sent)).toContain('"second try"')
    expect(restoreNote(sent)).not.toContain('first try')
  })

  it('keeps a message that only looks like one', () => {
    expect(stripRestoreNote('[Note: mine]\nhello')).toBe('[Note: mine]\nhello')
  })

  it('keeps the chips of a message that was only attachments', () => {
    const files = buildAttachmentMessage('', [{ path: 'C:/shot.png' }])
    expect(stripRestoreNote(`${restoreNote(files)}\n\n${files}`)).toBe(files)
  })
})

describe('a message sent with the note', () => {
  it('has the note as its own first line', () => {
    expect(withRestoreNote('[n]', 'go on')).toBe('[n]\ngo on')
    expect(withRestoreNote(null, 'go on')).toBe('go on')
  })

  it('still reads as attachments when it had no words', () => {
    const files = buildAttachmentMessage('', [{ path: 'C:/shot.png' }])
    expect(parseAttachmentMessage(withRestoreNote(restoreNote(files), files))?.files).toHaveLength(1)
  })
})

describe('the snippet the note quotes', () => {
  it('cuts by characters, never inside one, and drops what is not text', () => {
    const note = restoreNote(`${'a'.repeat(79)}\u{1F600}tail\u200b`)
    expect(note).toContain(`"${'a'.repeat(79)}\u{1F600}…"`)
    expect(/\p{Cs}/u.test(note)).toBe(false)
    expect(restoreNote('zero\u200bwidth')).toContain('"zerowidth"')
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
