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

  it('marks a markdown heading, a code span and a link', () => {
    expect(highlightLine('## Highlights', 'README.md')).toEqual([{ text: '## Highlights', kind: 'variable' }])
    expect(highlightLine('Run `cargo test` now', 'README.md')).toEqual([
      { text: 'Run ', kind: 'plain' },
      { text: '`cargo test`', kind: 'string' },
      { text: ' now', kind: 'plain' },
    ])
    expect(highlightLine('see [the guide](docs/guide.md)', 'README.md').at(-1)).toEqual({ text: '[the guide](docs/guide.md)', kind: 'variable' })
  })

  it('marks html and xml tags, attribute strings and comments', () => {
    expect(highlightLine('<a href="#install">Install</a>', 'index.html')).toEqual([
      { text: '<a', kind: 'variable' },
      { text: ' href=', kind: 'plain' },
      { text: '"#install"', kind: 'string' },
      { text: '>Install', kind: 'plain' },
      { text: '</a', kind: 'variable' },
      { text: '>', kind: 'plain' },
    ])
    expect(highlightLine('<!-- note -->', 'a.xml')).toEqual([{ text: '<!-- note -->', kind: 'comment' }])
  })

  it('treats config files and extensionless build files as # comment languages', () => {
    for (const path of ['.env', 'app.ini', 'Dockerfile', 'Makefile', '.gitignore', 'Cargo.lock']) {
      expect(highlightLine('x # note', path).at(-1), path).toEqual({ text: '# note', kind: 'comment' })
    }
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
