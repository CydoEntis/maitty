import { describe, expect, it } from 'vitest'
import { highlightLine } from './highlight'

describe('highlightLine', () => {
  it('marks keywords, strings, numbers and comments in code files', () => {
    expect(highlightLine("const n = 42 // 'x'", 'a.ts')).toEqual([
      { text: 'const', kind: 'keyword' },
      { text: ' ', kind: 'plain' },
      { text: 'n', kind: 'plain' },
      { text: ' = ', kind: 'plain' },
      { text: '42', kind: 'number' },
      { text: ' ', kind: 'plain' },
      { text: "// 'x'", kind: 'comment' },
    ])
  })

  it('treats # as a comment only in languages that use it', () => {
    expect(highlightLine('x = 1  # note', 'a.py').at(-1)).toEqual({
      text: '# note',
      kind: 'comment',
    })
    expect(highlightLine('#[derive(Debug)]', 'a.rs')[0]?.kind).not.toBe(
      'comment',
    )
  })

  it('leaves unknown file types and very long lines untouched', () => {
    expect(highlightLine('const x = 1', 'notes.txt')).toEqual([
      { text: 'const x = 1', kind: 'plain' },
    ])
    const long = 'a'.repeat(5000)
    expect(highlightLine(long, 'bundle.js')).toEqual([
      { text: long, kind: 'plain' },
    ])
  })
})
