import { describe, expect, it } from 'vitest'
import { findPathLinks } from './pathLinks'

describe('findPathLinks', () => {
  it('finds a relative path with its line number', () => {
    expect(findPathLinks('error at src/main.rs:42:7 here')).toEqual([{ start: 9, end: 25, path: 'src/main.rs', line: 42 }])
  })

  it('finds a path with no line and drops trailing punctuation', () => {
    expect(findPathLinks('edited ./a/b.ts.')).toEqual([{ start: 7, end: 15, path: './a/b.ts' }])
  })

  it('finds a Windows path', () => {
    expect(findPathLinks('see C:\\dev\\notes.md')[0]?.path).toBe('C:\\dev\\notes.md')
  })

  it('leaves versions, numbers, bare words and URLs alone', () => {
    expect(findPathLinks('v1.2.3 took 3.5s, see https://example.com/page.html and e.g. this')).toEqual([])
  })

  it('leaves a dotfile with no extension alone', () => {
    expect(findPathLinks('.gitignore')).toEqual([])
  })
})
