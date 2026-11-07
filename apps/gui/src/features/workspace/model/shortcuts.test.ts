import { describe, expect, it } from 'vitest'
import { ESC_LADDER, topmostOpen } from './escLadder'
import { BINDINGS, HOTKEY_GROUPS, bindingFor, detectPlatform, formatShortcut, hotkeyRows, matchShortcut, shortcutChords, shortcutText, type Platform, type ShortcutAction } from './shortcuts'

type Key = { key: string; ctrlKey: boolean; metaKey: boolean; shiftKey: boolean; altKey: boolean }

/** The key press a chord written as text stands for, so the table below is the only place a key is spelled out. */
function pressOf(text: string, platform: Platform): Key {
  const symbols = text.match(/^[⌘⌃⌥⇧]*/)?.[0] ?? ''
  const parts = platform === 'mac' ? [...Array.from(symbols), text.slice(symbols.length)] : text.split('+')
  const named = parts.at(-1) ?? ''
  const mods = new Set(parts.slice(0, -1))
  const shifted = mods.has('Shift') || mods.has('⇧')
  const key = named === '−' ? '-' : shifted ? named : named.toLowerCase()
  return { key, ctrlKey: mods.has('Ctrl') || mods.has('⌃'), metaKey: mods.has('⌘'), shiftKey: shifted, altKey: mods.has('Alt') || mods.has('⌥') }
}

/** One row per binding: what the keycap says on macOS and on Windows and Linux ('' = no key there). */
const TABLE: readonly [ShortcutAction, string, string][] = [
  ['new-session', '⌘T', 'Ctrl+Shift+T'],
  ['new-terminal', '⌘⇧T', 'Ctrl+Alt+Shift+T'],
  ['new-worktree', '⌘⇧N', 'Ctrl+Shift+N'],
  ['split-right', '⌘D', 'Ctrl+Shift+D'],
  ['split-down', '⌘⇧D', 'Ctrl+Alt+Shift+D'],
  ['focus-previous', '⌘[', 'Ctrl+['],
  ['focus-next', '⌘]', 'Ctrl+]'],
  ['close', '⌘W', 'Ctrl+Shift+W'],
  ['undo-close', '⌘Z', 'Ctrl+Shift+Z'],
  ['broadcast', '⌘⇧B', 'Ctrl+Alt+Shift+B'],
  ['palette', '⌘K', 'Ctrl+Shift+K'],
  ['inbox', '⌘J', 'Ctrl+Shift+J'],
  ['jump-needs', '⌘⇧J', ''],
  ['inspect', '⌘I', 'Ctrl+Shift+I'],
  ['find', '⌘F', 'Ctrl+Shift+F'],
  ['toggle-sidebar', '⌘B', 'Ctrl+Shift+B'],
  ['chat-toggle', '⌘⇧E', 'Ctrl+Shift+E'],
  ['usage', '⌘⇧U', 'Ctrl+Shift+U'],
  ['performance', '⌘⇧P', 'Ctrl+Shift+P'],
  ['focus-mode', '⌘.', 'Ctrl+.'],
  ['settings', '⌘,', 'Ctrl+,'],
  ['text-larger', '⌘=', 'Ctrl+='],
  ['text-smaller', '⌘−', 'Ctrl+−'],
  ['shortcuts', '⌘/', 'Ctrl+/'],
]

const PLATFORMS: readonly Platform[] = ['mac', 'windows', 'linux']

describe('the shortcut table', () => {
  it('has a row for every binding and nothing else', () => {
    expect(TABLE.map(([action]) => action).sort()).toEqual(BINDINGS.map((binding) => binding.action).sort())
  })

  it.each(TABLE)('writes %s as %s on macOS and %s on Windows and Linux', (action, mac, other) => {
    expect(formatShortcut(bindingFor(action), 'mac')).toBe(mac)
    expect(formatShortcut(bindingFor(action), 'windows')).toBe(other)
    expect(formatShortcut(bindingFor(action), 'linux')).toBe(other)
    expect(shortcutText(action, 'mac')).toBe(mac)
  })

  it('splits a chord into its keycaps', () => {
    expect(shortcutChords(bindingFor('split-down'), 'mac')).toEqual([['⌘', '⇧', 'D']])
    expect(shortcutChords(bindingFor('split-down'), 'windows')).toEqual([['Ctrl', 'Alt', 'Shift', 'D']])
    expect(shortcutChords(bindingFor('jump-needs'), 'linux')).toEqual([])
  })

  it('gives no two actions the same key on a platform', () => {
    for (const platform of PLATFORMS) {
      const written = TABLE.map(([action]) => formatShortcut(bindingFor(action), platform)).filter((text) => text !== '')
      expect(new Set(written).size).toBe(written.length)
    }
  })
})

describe('matching a key press', () => {
  const live = TABLE.filter(([action]) => bindingFor(action).available)

  it.each(live)('%s is taken from %s on macOS and from %s on Windows and Linux', (action, mac, other) => {
    const expected = { action }
    const press = (text: string, platform: Platform) => matchShortcut(pressOf(text, platform), platform)
    expect(press(mac, 'mac')).toEqual(expected)
    if (other !== '') {
      expect(press(other, 'windows')).toEqual(expected)
      expect(press(other, 'linux')).toEqual(expected)
    }
  })

  it('has no key registered for a feature that is not built', () => {
    expect(BINDINGS.filter((binding) => !binding.available)).toEqual([])
  })

  it('never takes a bare Ctrl+letter on Windows or Linux', () => {
    for (const platform of ['windows', 'linux'] as const) {
      for (const letter of 'abcdefghijklmnopqrstuvwxyz') {
        for (const key of [letter, letter.toUpperCase()]) {
          expect(matchShortcut({ key, ctrlKey: true, metaKey: false, shiftKey: false, altKey: false }, platform), `Ctrl+${key}`).toBeNull()
        }
      }
    }
  })

  it('claims no bare Ctrl+letter in its table on Windows or Linux', () => {
    for (const binding of BINDINGS) {
      for (const combo of binding.other) {
        const bare = combo.mods.length === 1 && combo.mods[0] === 'ctrl'
        expect(bare && /^[a-z]$/i.test(combo.key), `${binding.action} ${combo.key}`).toBe(false)
      }
    }
  })

  it('leaves Ctrl alone on macOS and Cmd alone elsewhere', () => {
    expect(matchShortcut(pressOf('Ctrl+d', 'linux'), 'mac')).toBeNull()
    expect(matchShortcut({ key: 't', ctrlKey: false, metaKey: true, shiftKey: false, altKey: false }, 'linux')).toBeNull()
  })

  it('needs the modifiers exactly, so Ctrl+Shift+T is a new session and Ctrl+Alt+Shift+T a new terminal', () => {
    expect(matchShortcut({ key: 'T', ctrlKey: true, metaKey: false, shiftKey: true, altKey: false }, 'linux')).toEqual({ action: 'new-session' })
    expect(matchShortcut({ key: 'T', ctrlKey: true, metaKey: false, shiftKey: true, altKey: true }, 'linux')).toEqual({ action: 'new-terminal' })
    expect(matchShortcut({ key: 't', ctrlKey: false, metaKey: false, shiftKey: true, altKey: false }, 'linux')).toBeNull()
    // macOS spends Cmd and Cmd+Shift on the two.
    expect(matchShortcut({ key: 't', ctrlKey: false, metaKey: true, shiftKey: false, altKey: false }, 'mac')).toEqual({ action: 'new-session' })
    expect(matchShortcut({ key: 'T', ctrlKey: false, metaKey: true, shiftKey: true, altKey: false }, 'mac')).toEqual({ action: 'new-terminal' })
  })

  it('passes keys the window does not own to the shell', () => {
    for (const platform of PLATFORMS) {
      expect(matchShortcut({ key: 'c', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false }, platform)).toBeNull()
      expect(matchShortcut({ key: 'l', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false }, platform)).toBeNull()
      expect(matchShortcut({ key: 't', ctrlKey: false, metaKey: false, shiftKey: false, altKey: false }, platform)).toBeNull()
    }
  })

  it('takes Ctrl and Shift plus = as larger text on a layout where + needs Shift', () => {
    expect(matchShortcut({ key: '+', ctrlKey: true, metaKey: false, shiftKey: true, altKey: false }, 'linux')).toEqual({ action: 'text-larger' })
  })

  it('takes the Inbox on Cmd+J on macOS and Ctrl+Shift+J elsewhere, and leaves a bare Ctrl+J to the terminal', () => {
    expect(matchShortcut({ key: 'j', ctrlKey: false, metaKey: true, shiftKey: false, altKey: false }, 'mac')).toEqual({ action: 'inbox' })
    expect(matchShortcut({ key: 'J', ctrlKey: true, metaKey: false, shiftKey: true, altKey: false }, 'linux')).toEqual({ action: 'inbox' })
    expect(matchShortcut({ key: 'j', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false }, 'linux')).toBeNull()
  })

  it('has no tab shortcuts: Ctrl+Tab and Ctrl+1 belong to the terminal now', () => {
    expect(matchShortcut({ key: 'Tab', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false }, 'windows')).toBeNull()
    expect(matchShortcut({ key: '1', ctrlKey: true, metaKey: false, shiftKey: false, altKey: false }, 'windows')).toBeNull()
  })
})

describe('the shortcut lists', () => {
  it('lists the hotkeys in the platform’s words', () => {
    expect(hotkeyRows('mac').find((row) => row.action === 'Split down')).toEqual({ action: 'Split down', group: 'Panes', chords: [['⌘', '⇧', 'D']] })
    expect(hotkeyRows('windows').find((row) => row.action === 'Split down')).toEqual({ action: 'Split down', group: 'Panes', chords: [['Ctrl', 'Alt', 'Shift', 'D']] })
  })

  it('shows the terminal’s own keys as the terminal’s', () => {
    expect(hotkeyRows('linux').find((row) => row.action === 'Interrupt')?.chords).toEqual([['Ctrl', 'C']])
    expect(hotkeyRows('mac').find((row) => row.action === 'Interrupt')?.chords).toEqual([['⌃', 'C']])
  })

  it('leaves out an action with no key on the platform, and one that is not built', () => {
    expect(hotkeyRows('linux').some((row) => row.action.startsWith('Jump to the next'))).toBe(false)
    expect(hotkeyRows('mac').some((row) => row.action.startsWith('Jump to the next'))).toBe(true)
    expect(hotkeyRows('mac').some((row) => row.action === 'Overview')).toBe(false)
    expect(hotkeyRows('mac').some((row) => row.action === 'Session panel')).toBe(false)
    expect(hotkeyRows('mac').some((row) => row.action === 'Undo close')).toBe(true)
  })

  it('puts every hotkey in a group of the overlay, and every group has some', () => {
    const rows = hotkeyRows('linux')
    expect(rows.every((row) => HOTKEY_GROUPS.includes(row.group))).toBe(true)
    for (const group of HOTKEY_GROUPS) expect(rows.some((row) => row.group === group)).toBe(true)
  })

  it('knows the platform from the user agent', () => {
    expect(detectPlatform('Mozilla/5.0 (Macintosh; Intel Mac OS X 14_0)')).toBe('mac')
    expect(detectPlatform('Mozilla/5.0 (Windows NT 10.0; Win64; x64)')).toBe('windows')
    expect(detectPlatform('Mozilla/5.0 (X11; Linux x86_64)')).toBe('linux')
  })
})

describe('the Esc ladder', () => {
  it('closes the topmost layer first, in the order of v18 C.9', () => {
    expect(ESC_LADDER.indexOf('palette')).toBeLessThan(ESC_LADDER.indexOf('drawer'))
    expect(topmostOpen({ settings: true, drawer: true, 'focus-mode': true })).toBe('settings')
    expect(topmostOpen({ palette: true, settings: true })).toBe('palette')
    expect(topmostOpen({ palette: true, drawer: true })).toBe('palette')
  })

  it('leaves Esc to the terminal when nothing is open', () => {
    expect(topmostOpen({})).toBeNull()
    expect(topmostOpen({ palette: false, drawer: false })).toBeNull()
  })
})
