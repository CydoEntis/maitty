import { describe, expect, it } from 'vitest'
import { parseTip } from './tipText'

describe('parsing a tooltip', () => {
  it('leaves a plain name alone', () => {
    expect(parseTip('Close session')).toEqual({ name: 'Close session', keys: null, hint: null })
  })

  it.each([
    ['Split right (⌘D)', 'Split right', ['⌘', 'D']],
    ['Split down (⌘⇧D)', 'Split down', ['⌘', '⇧', 'D']],
    ['Next tab (⌃Tab)', 'Next tab', ['⌃', 'Tab']],
    ['Split down (Ctrl+Alt+Shift+D)', 'Split down', ['Ctrl', 'Alt', 'Shift', 'D']],
    ['Settings (Ctrl+,)', 'Settings', ['Ctrl', ',']],
    ['Larger text (Ctrl++)', 'Larger text', ['Ctrl', '+']],
    ['Close (Esc)', 'Close', ['Esc']],
  ])('turns the trailing keys of "%s" into keycaps', (text, name, keys) => {
    expect(parseTip(text)).toEqual({ name, keys, hint: null })
  })

  it.each(['Close project tabs (3)', 'Open (read only)', 'Inbox, 2 (new)'])('keeps "%s" as words', (text) => {
    expect(parseTip(text).keys).toBeNull()
    expect(parseTip(text).name).toBe(text)
  })

  it('splits a second line off at the dot', () => {
    expect(parseTip('Inbox · What needs you across every project')).toEqual({ name: 'Inbox', keys: null, hint: 'What needs you across every project' })
  })

  it('reads keys that close either the whole text or the name', () => {
    expect(parseTip('Inbox · Needs you, failed, finished (⌘⇧J)')).toEqual({ name: 'Inbox', keys: ['⌘', '⇧', 'J'], hint: 'Needs you, failed, finished' })
    expect(parseTip('Inbox (⌘⇧J) · Needs you, failed, finished')).toEqual({ name: 'Inbox', keys: ['⌘', '⇧', 'J'], hint: 'Needs you, failed, finished' })
  })
})
