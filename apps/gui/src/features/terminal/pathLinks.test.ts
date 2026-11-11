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

describe('web addresses in terminal output', () => {
  it('finds an address with its scheme and a bare site name, without the punctuation that ends the sentence', async () => {
    const { findWebLinks } = await import('./pathLinks')
    const line = '1. Sign in to app.hubspot.com with the account, or see https://example.com/a?b=1&c=2.'
    const links = findWebLinks(line)
    expect(links.map((link) => link.url)).toEqual(['https://app.hubspot.com', 'https://example.com/a?b=1&c=2'])
    expect(links.map((link) => line.slice(link.start, link.end))).toEqual(['app.hubspot.com', 'https://example.com/a?b=1&c=2'])
    expect(findWebLinks('docs.rs/tokio/latest.').map((link) => link.url)).toEqual([])
    expect(findWebLinks('see github.com/CydoEntis/maitty, then go').map((link) => link.url)).toEqual(['https://github.com/CydoEntis/maitty'])
  })

  it('leaves file names, emails and versions alone', async () => {
    const { findWebLinks } = await import('./pathLinks')
    for (const line of ['src/main.rs:42', 'notes.md and a.b.ts', 'mail me@example.com now', 'v1.2.3 released', './app.config.ts']) {
      expect(findWebLinks(line), line).toEqual([])
    }
  })

  it('no longer offers a site name as a file to open', () => {
    expect(findPathLinks('Sign in to app.hubspot.com today').map((link) => link.path)).toEqual([])
    expect(findPathLinks('edit src/app.com.ts and notes.md').map((link) => link.path)).toEqual(['src/app.com.ts', 'notes.md'])
  })
})
