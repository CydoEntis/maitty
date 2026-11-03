import { describe, expect, it } from 'vitest'
import { formatDroppedPaths } from './droppedPaths'

describe('dropped paths', () => {
  it('types a plain path as it is, with a space after for the next word', () => {
    expect(formatDroppedPaths(['/home/vox/shot.png'], false)).toBe('/home/vox/shot.png ')
    expect(formatDroppedPaths(['C:\\Users\\vox\\shot.png'], true)).toBe('C:\\Users\\vox\\shot.png ')
  })

  it('escapes spaces and shell characters on Linux and macOS', () => {
    expect(formatDroppedPaths(['/home/vox/My Shot (2).png'], false)).toBe('/home/vox/My\\ Shot\\ \\(2\\).png ')
  })

  it('quotes a Windows path that has spaces', () => {
    expect(formatDroppedPaths(['C:\\Users\\vox\\My Shot.png'], true)).toBe('"C:\\Users\\vox\\My Shot.png" ')
  })

  it('separates several files with spaces', () => {
    expect(formatDroppedPaths(['/a/one.png', '/a/two words.png'], false)).toBe('/a/one.png /a/two\\ words.png ')
  })

  it('never ends in a newline, so a drop cannot run anything', () => {
    expect(formatDroppedPaths(['/a/b\n.png'], false)).not.toMatch(/\n$/)
    expect(formatDroppedPaths(['/a/b'], false).endsWith('\r')).toBe(false)
  })

  it('types nothing for nothing', () => {
    expect(formatDroppedPaths([], false)).toBe(' ')
    expect(formatDroppedPaths([''], false)).toBe(' ')
  })
})
