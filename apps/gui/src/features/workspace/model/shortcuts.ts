export type ShortcutAction =
  | 'new-session'
  | 'split-right'
  | 'split-down'
  | 'focus-previous'
  | 'focus-next'
  | 'close'
  | 'next-tab'
  | 'previous-tab'
  | 'toggle-sidebar'
  | 'settings'
  | 'palette'
  | 'inbox'
  | 'broadcast'
  | 'focus-mode'

type KeyInput = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>

/**
 * The few keys the window keeps. Everything else, inside a focused pane, goes to the shell. ⌘ is
 * the Command key on macOS and Ctrl elsewhere; on macOS Ctrl stays with the shell (Ctrl+D, Ctrl+W).
 */
export function matchShortcut(event: KeyInput, mac: boolean): ShortcutAction | null {
  if (event.altKey) return null
  if (event.ctrlKey && !event.metaKey && event.key === 'Tab') return event.shiftKey ? 'previous-tab' : 'next-tab'
  const primary = mac ? event.metaKey && !event.ctrlKey : event.ctrlKey && !event.metaKey
  if (!primary) return null
  const key = event.key.toLowerCase()
  if (key === 'd') return event.shiftKey ? 'split-down' : 'split-right'
  if (event.shiftKey) return key === 'b' ? 'broadcast' : null
  switch (key) {
    case 't':
      return 'new-session'
    case '[':
      return 'focus-previous'
    case ']':
      return 'focus-next'
    case 'w':
      return 'close'
    case 'b':
      return 'toggle-sidebar'
    case ',':
      return 'settings'
    case 'k':
      return 'palette'
    case 'j':
      return 'inbox'
    case '.':
      return 'focus-mode'
    default:
      return null
  }
}

export function isMacPlatform(userAgent: string): boolean {
  return /Macintosh|Mac OS X/i.test(userAgent)
}

/** How the platform writes a chord: `⌘B` and `⌘⇧D` on macOS, `Ctrl+B` and `Ctrl+Shift+D` elsewhere. */
export function keyLabels(mac: boolean): { primary: string; shift: string; control: string } {
  return mac ? { primary: '⌘', shift: '⇧', control: '⌃' } : { primary: 'Ctrl+', shift: 'Shift+', control: 'Ctrl+' }
}

/** One key combination as the keys to press, in order: `['⌘', '⇧', 'D']` on macOS, `['Ctrl', 'Shift', 'D']` elsewhere. */
export type Chord = string[]

export type HotkeyRow = { action: string; /** Alternatives: any one of these does it. */ chords: Chord[] }

/** The modifier keys drawn as symbols, which the keycaps set in the interface font. */
const MODIFIER_SYMBOLS: ReadonlySet<string> = new Set(['⌘', '⇧', '⌃', '⌥'])
export const isModifierSymbol = (key: string): boolean => MODIFIER_SYMBOLS.has(key)

/** The Hotkeys list, in the words the window uses. */
export function hotkeyRows(mac: boolean): HotkeyRow[] {
  const primary = mac ? '⌘' : 'Ctrl'
  const shift = mac ? '⇧' : 'Shift'
  const control = mac ? '⌃' : 'Ctrl'
  return [
    { action: 'New session', chords: [[primary, 'T']] },
    { action: 'Split right', chords: [[primary, 'D']] },
    { action: 'Split down', chords: [[primary, shift, 'D']] },
    { action: 'Move focus between panes', chords: [[primary, '['], [primary, ']']] },
    { action: 'Close pane or tab', chords: [[primary, 'W']] },
    { action: 'Next / previous tab in a project', chords: [[control, 'Tab'], [control, shift, 'Tab']] },
    { action: 'Collapse sidebar', chords: [[primary, 'B']] },
    { action: 'Command palette', chords: [[primary, 'K']] },
    { action: 'Attention inbox', chords: [[primary, 'J']] },
    { action: 'Broadcast to the tab', chords: [[primary, shift, 'B']] },
    { action: 'Focus mode', chords: [[primary, '.']] },
    { action: 'Interrupt', chords: [[control, 'C']] },
    { action: 'Clear screen', chords: [[control, 'L']] },
    { action: 'Settings', chords: [[primary, ',']] },
  ]
}
