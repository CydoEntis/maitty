import { describe, expect, it } from 'vitest'
import { fileMention, pathFromFolder, withPath } from './fileMention'

describe('when @ opens the file picker', () => {
  it('opens for an @ at the start of the message', () => {
    expect(fileMention('@src/ma')).toEqual({ start: 0, query: 'src/ma' })
  })

  it('opens for an @ after a space or a new line, with nothing typed yet', () => {
    expect(fileMention('look at @')).toEqual({ start: 8, query: '' })
    expect(fileMention('first line\n@app')).toEqual({ start: 11, query: 'app' })
  })

  it('does not open for an @ inside a word, as in an email address', () => {
    expect(fileMention('mail me@example.com')).toBeNull()
    expect(fileMention('me@')).toBeNull()
  })

  it('closes once a space follows what was typed', () => {
    expect(fileMention('@src/main.rs ')).toBeNull()
  })
})

describe('the text a picked file leaves in the box', () => {
  it('replaces the @ and what was typed with the path and a space', () => {
    const draft = 'fix the bug in @mai'
    const mention = fileMention(draft)
    if (mention === null) throw new Error('no mention')
    expect(withPath(draft, mention, 'src/main.rs')).toBe('fix the bug in src/main.rs ')
  })

  it('quotes a path that holds a space', () => {
    const draft = '@notes'
    const mention = fileMention(draft)
    if (mention === null) throw new Error('no mention')
    expect(withPath(draft, mention, 'docs/my notes.md')).toBe('"docs/my notes.md" ')
  })
})

describe('a path read from the session folder', () => {
  it('is the daemon path as it is when the session works at the checkout top', () => {
    expect(pathFromFolder('src/main.rs', { folder: '/dev/app', checkout: '/dev/app' })).toBe('src/main.rs')
  })

  it('is the daemon path as it is outside a repository', () => {
    expect(pathFromFolder('notes.md', { folder: '/home/me/notes', checkout: null })).toBe('notes.md')
  })

  it('drops the folder a session works in from paths under it', () => {
    expect(pathFromFolder('crates/core/src/lib.rs', { folder: '/dev/app/crates/core', checkout: '/dev/app' })).toBe('src/lib.rs')
  })

  it('climbs out of the folder for a path elsewhere in the checkout', () => {
    expect(pathFromFolder('crates/shared/src/lib.rs', { folder: '/dev/app/crates/core', checkout: '/dev/app' })).toBe('../shared/src/lib.rs')
    expect(pathFromFolder('README.md', { folder: '/dev/app/crates/core', checkout: '/dev/app' })).toBe('../../README.md')
  })

  it('reads a Windows folder with either separator and drive case', () => {
    expect(pathFromFolder('apps/gui/src/main.tsx', { folder: 'C:\\dev\\app\\apps\\gui', checkout: 'c:/dev/app' })).toBe('src/main.tsx')
  })
})
