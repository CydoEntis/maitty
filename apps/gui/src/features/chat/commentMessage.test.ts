import { describe, expect, it } from 'vitest'
import { formatComment, parseComment, type LineComment } from './commentMessage'

const comment: LineComment = { file: 'src/app.ts', line: 12, code: 'const x = 1', text: 'Rename this to count.' }

describe('a line comment as a message', () => {
  it('writes the file, line, quoted code and text in one message', () => {
    expect(formatComment(comment)).toBe('Comment on src/app.ts:12\n> const x = 1\n\nRename this to count.')
  })

  it.each([
    ['plain', comment],
    ['a multi-line comment', { ...comment, text: 'First.\n\nSecond, with a colon: 3' }],
    ['a Windows path', { ...comment, file: 'C:\\work\\app.ts' }],
    ['a path holding a colon and digits', { ...comment, file: 'dir:2/app.ts:5' }],
    ['empty code', { ...comment, code: '' }],
  ])('reads back what it wrote: %s', (_name, original) => {
    expect(parseComment(formatComment(original))).toEqual(original)
  })

  it('keeps the quoted line to one bounded line', () => {
    const written = formatComment({ ...comment, code: `a\r\nb${'x'.repeat(500)}` })
    const quoted = written.split('\n')[1] ?? ''

    expect(quoted.startsWith('> a b')).toBe(true)
    expect(quoted.length).toBeLessThanOrEqual(242)
  })

  it.each([
    'fix the test',
    'Comment on the design: it is fine',
    'Comment on src/a.ts:12\nno quoted line',
    'Comment on src/a.ts:x\n> code\n\ntext',
    '',
  ])('does not take %j for a comment', (message) => {
    expect(parseComment(message)).toBeNull()
  })
})
