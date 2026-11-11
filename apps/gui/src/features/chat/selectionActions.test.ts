import { describe, expect, it } from 'vitest'
import { draftWithQuote, quoteOf } from './selectionActions'

describe('quoting selected text', () => {
  it('puts each line behind a quote mark and drops the blank lines at its ends', () => {
    expect(quoteOf('\n  \nfirst line  \n\nsecond line\n\n')).toBe('> first line\n>\n> second line')
  })

  it('starts the draft with the quote and leaves a line to reply on', () => {
    expect(draftWithQuote('', 'the plan')).toBe('> the plan\n\n')
  })

  it('adds the quote under what was already written', () => {
    expect(draftWithQuote('I disagree:\n', 'the plan')).toBe('I disagree:\n\n> the plan\n\n')
  })
})
