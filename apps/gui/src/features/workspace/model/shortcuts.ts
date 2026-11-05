/**
 * The window's keyboard shortcuts: one table that the key handler, the tooltips, the keycaps, the
 * menus, the palette and the shortcut overlay all read, so a key is written down once.
 *
 * The contract (v18 B.8, owner decision 2026-11-04): a pane's terminal owns every plain Ctrl+letter
 * on Windows and Linux (Ctrl+C, D, W, L, R, J ...), so the window uses Ctrl+Shift there. On macOS
 * the window uses Cmd and leaves Ctrl to the terminal.
 */

export type Platform = 'mac' | 'windows' | 'linux'

export function detectPlatform(userAgent: string): Platform {
  if (/Macintosh|Mac OS X/i.test(userAgent)) return 'mac'
  return /Windows/i.test(userAgent) ? 'windows' : 'linux'
}

export function isMacPlatform(userAgent: string): boolean {
  return detectPlatform(userAgent) === 'mac'
}

export type ShortcutAction =
  | 'new-session'
  | 'split-right'
  | 'split-down'
  | 'focus-previous'
  | 'focus-next'
  | 'close'
  | 'undo-close'
  | 'go-to-tab'
  | 'next-tab'
  | 'previous-tab'
  | 'toggle-sidebar'
  | 'settings'
  | 'palette'
  | 'inbox'
  | 'jump-needs'
  | 'inspect'
  | 'find'
  | 'new-worktree'
  | 'broadcast'
  | 'focus-mode'
  | 'shortcuts'
  | 'text-larger'
  | 'text-smaller'
  | 'overview'
  | 'session-panel'
  | 'usage'
  | 'performance'
  | 'chat-toggle'

/** The sections of the shortcut overlay, in the order it shows them. */
export const HOTKEY_GROUPS = ['Sessions', 'Panes', 'Find & jump', 'Window'] as const
export type HotkeyGroup = (typeof HOTKEY_GROUPS)[number]

type Modifier = 'cmd' | 'ctrl' | 'alt' | 'shift'

/** One key with the modifiers held for it. `cmd` exists on macOS only. */
export type Combo = {
  mods: readonly Modifier[]
  /** The key as it is drawn on a keycap and compared, case-insensitively, with `KeyboardEvent.key`. */
  key: string
  /** Matches with or without Shift: `+` is Shift+`=` on most layouts, and both mean "larger". */
  anyShift?: boolean
}

export type Binding = {
  action: ShortcutAction
  /** The overlay row and the palette entry. */
  label: string
  group: HotkeyGroup
  mac: readonly Combo[]
  /** Windows and Linux share one column. Empty: no key on those platforms (the feature is still reachable by menu or palette). */
  other: readonly Combo[]
  /** False until the feature behind it exists. The window ignores the key and the overlay hides it. */
  available: boolean
}

/** The digits of "go to tab"; the matcher takes any one of 1 to 9. */
const TAB_DIGITS = '1…9'
const TAB_DIGIT_PATTERN = /^[1-9]$/

const cmd = (key: string, ...more: Modifier[]): Combo => ({ mods: ['cmd', ...more], key })
const ctrlShift = (key: string, ...more: Modifier[]): Combo => ({ mods: ['ctrl', ...more, 'shift'], key })

/**
 * Where two actions need the same letter on Windows and Linux, the second takes Ctrl+Alt+Shift
 * (split down against split right, broadcast against collapse sidebar). macOS has no such clash
 * because it spends Cmd and Cmd+Shift on them separately.
 */
export const BINDINGS: readonly Binding[] = [
  { action: 'new-session', label: 'New session', group: 'Sessions', mac: [cmd('T')], other: [ctrlShift('T')], available: true },
  // Ctrl+Shift+N stays "new worktree" until brain dump (v18 C.6), which takes this key, is built.
  { action: 'new-worktree', label: 'New worktree', group: 'Sessions', mac: [cmd('N', 'shift')], other: [ctrlShift('N')], available: true },
  { action: 'split-right', label: 'Split right', group: 'Panes', mac: [cmd('D')], other: [ctrlShift('D')], available: true },
  { action: 'split-down', label: 'Split down', group: 'Panes', mac: [cmd('D', 'shift')], other: [ctrlShift('D', 'alt')], available: true },
  { action: 'focus-previous', label: 'Previous pane', group: 'Panes', mac: [cmd('[')], other: [{ mods: ['ctrl'], key: '[' }], available: true },
  { action: 'focus-next', label: 'Next pane', group: 'Panes', mac: [cmd(']')], other: [{ mods: ['ctrl'], key: ']' }], available: true },
  { action: 'close', label: 'Close pane or tab', group: 'Panes', mac: [cmd('W')], other: [ctrlShift('W')], available: true },
  // Closing ends the session, so there is no window to undo in yet.
  { action: 'undo-close', label: 'Undo close', group: 'Panes', mac: [cmd('Z')], other: [ctrlShift('Z')], available: false },
  { action: 'go-to-tab', label: 'Go to tab', group: 'Panes', mac: [cmd(TAB_DIGITS)], other: [{ mods: ['ctrl'], key: TAB_DIGITS }], available: true },
  { action: 'next-tab', label: 'Next tab in a project', group: 'Panes', mac: [{ mods: ['ctrl'], key: 'Tab' }], other: [{ mods: ['ctrl'], key: 'Tab' }], available: true },
  { action: 'previous-tab', label: 'Previous tab in a project', group: 'Panes', mac: [{ mods: ['ctrl', 'shift'], key: 'Tab' }], other: [{ mods: ['ctrl', 'shift'], key: 'Tab' }], available: true },
  { action: 'broadcast', label: 'Broadcast to the tab', group: 'Panes', mac: [cmd('B', 'shift')], other: [ctrlShift('B', 'alt')], available: true },
  { action: 'palette', label: 'Command palette', group: 'Find & jump', mac: [cmd('K')], other: [ctrlShift('K')], available: true },
  { action: 'inbox', label: 'Attention inbox', group: 'Find & jump', mac: [cmd('J', 'shift')], other: [ctrlShift('J')], available: true },
  // Plain Ctrl+J is a line feed in a terminal, so Windows and Linux have no key for this yet.
  { action: 'jump-needs', label: 'Jump to the next session that needs you', group: 'Find & jump', mac: [cmd('J')], other: [], available: true },
  { action: 'inspect', label: 'Inspect the repository', group: 'Find & jump', mac: [cmd('I')], other: [ctrlShift('I')], available: true },
  { action: 'find', label: 'Find in the pane or every session', group: 'Find & jump', mac: [cmd('F')], other: [ctrlShift('F')], available: true },
  { action: 'overview', label: 'Overview', group: 'Find & jump', mac: [cmd('0')], other: [{ mods: ['ctrl'], key: '0' }], available: false },
  { action: 'toggle-sidebar', label: 'Collapse sidebar', group: 'Window', mac: [cmd('B')], other: [ctrlShift('B')], available: true },
  { action: 'session-panel', label: 'Session panel', group: 'Window', mac: [cmd('\\')], other: [{ mods: ['ctrl'], key: '\\' }], available: false },
  { action: 'chat-toggle', label: 'Terminal or chat', group: 'Window', mac: [cmd('E', 'shift')], other: [ctrlShift('E')], available: false },
  { action: 'usage', label: 'Usage and cost', group: 'Window', mac: [cmd('U', 'shift')], other: [ctrlShift('U')], available: true },
  { action: 'performance', label: 'Performance', group: 'Window', mac: [cmd('P', 'shift')], other: [ctrlShift('P')], available: true },
  { action: 'focus-mode', label: 'Focus mode', group: 'Window', mac: [cmd('.')], other: [{ mods: ['ctrl'], key: '.' }], available: true },
  { action: 'settings', label: 'Settings', group: 'Window', mac: [cmd(',')], other: [{ mods: ['ctrl'], key: ',' }], available: true },
  { action: 'text-larger', label: 'Larger text', group: 'Window', mac: [{ mods: ['cmd'], key: '=', anyShift: true }], other: [{ mods: ['ctrl'], key: '=', anyShift: true }], available: true },
  { action: 'text-smaller', label: 'Smaller text', group: 'Window', mac: [{ mods: ['cmd'], key: '−' }], other: [{ mods: ['ctrl'], key: '−' }], available: true },
  { action: 'shortcuts', label: 'Keyboard shortcuts', group: 'Window', mac: [cmd('/')], other: [{ mods: ['ctrl'], key: '/' }], available: true },
]

/** Keys the window never takes: they belong to the terminal. Listed so the overlay can say so. */
const TERMINAL_KEYS: readonly { label: string; group: HotkeyGroup; letter: string }[] = [
  { label: 'Interrupt', group: 'Sessions', letter: 'C' },
  { label: 'Clear screen', group: 'Sessions', letter: 'L' },
]

/** What `KeyboardEvent.key` reports for the keycaps whose character differs, or has a second spelling (Shift+= is `+`). */
const EVENT_KEYS: Readonly<Record<string, readonly string[]>> = { '−': ['-'], '=': ['=', '+'] }

export function bindingFor(action: ShortcutAction): Binding {
  const found = BINDINGS.find((binding) => binding.action === action)
  if (!found) throw new Error(`No binding for ${action}`)
  return found
}

const MAC_GLYPH: Readonly<Record<Modifier, string>> = { cmd: '⌘', ctrl: '⌃', alt: '⌥', shift: '⇧' }
const WORD: Readonly<Record<Modifier, string>> = { cmd: 'Ctrl', ctrl: 'Ctrl', alt: 'Alt', shift: 'Shift' }
/** The order modifiers are written in: ⌘⌃⌥⇧ on macOS, Ctrl+Alt+Shift elsewhere. */
const MODIFIER_ORDER: readonly Modifier[] = ['cmd', 'ctrl', 'alt', 'shift']

/** One combo as the keys to press, in order: `['⌘', '⇧', 'D']` on macOS, `['Ctrl', 'Shift', 'D']` elsewhere. */
export type Chord = string[]

export function comboChord(combo: Combo, platform: Platform): Chord {
  const mods = MODIFIER_ORDER.filter((modifier) => combo.mods.includes(modifier)).map((modifier) => (platform === 'mac' ? MAC_GLYPH[modifier] : WORD[modifier]))
  return [...mods, combo.key]
}

/** Every key combination that does the action on this platform; empty when it has none or is not built. */
export function shortcutChords(binding: Binding, platform: Platform): Chord[] {
  return (platform === 'mac' ? binding.mac : binding.other).map((combo) => comboChord(combo, platform))
}

/** A chord written as one string: `⌘⇧D` on macOS, `Ctrl+Shift+D` elsewhere. */
export function writeChord(chord: Chord, platform: Platform): string {
  return chord.join(platform === 'mac' ? '' : '+')
}

/** The binding's first key combination as text, or '' when the platform has none. */
export function formatShortcut(binding: Binding, platform: Platform): string {
  const [first] = shortcutChords(binding, platform)
  return first ? writeChord(first, platform) : ''
}

/** What a tooltip or menu hint shows for an action on this platform. */
export function shortcutText(action: ShortcutAction, platform: Platform): string {
  return formatShortcut(bindingFor(action), platform)
}

type KeyInput = Pick<KeyboardEvent, 'key' | 'ctrlKey' | 'metaKey' | 'shiftKey' | 'altKey'>

export type ShortcutMatch = { action: ShortcutAction; /** For go-to-tab: the 1-based tab number. */ tab?: number }

function matches(combo: Combo, event: KeyInput, platform: Platform): boolean {
  const wanted = (modifier: Modifier) => combo.mods.includes(modifier)
  if (event.metaKey !== (platform === 'mac' && wanted('cmd'))) return false
  if (event.ctrlKey !== wanted('ctrl')) return false
  if (event.altKey !== wanted('alt')) return false
  if (!combo.anyShift && event.shiftKey !== wanted('shift')) return false
  const pressed = event.key.toLowerCase()
  if (combo.key === TAB_DIGITS) return TAB_DIGIT_PATTERN.test(pressed)
  return (EVENT_KEYS[combo.key] ?? [combo.key.toLowerCase()]).includes(pressed)
}

/**
 * The window shortcut this key press is, or null when it belongs to the shell. Modifiers must match
 * exactly, so Ctrl+T on Windows or Linux (a shell's own key) is never taken. Features not built yet
 * return null too, so their keys still reach the terminal.
 */
export function matchShortcut(event: KeyInput, platform: Platform): ShortcutMatch | null {
  for (const binding of BINDINGS) {
    if (!binding.available) continue
    for (const combo of platform === 'mac' ? binding.mac : binding.other) {
      if (!matches(combo, event, platform)) continue
      return binding.action === 'go-to-tab' ? { action: binding.action, tab: Number(event.key) } : { action: binding.action }
    }
  }
  return null
}

export type HotkeyRow = { action: string; group: HotkeyGroup; /** Alternatives: any one of these does it. */ chords: Chord[] }

/** The modifier keys drawn as symbols, which the keycaps set in the interface font. */
const MODIFIER_SYMBOLS: ReadonlySet<string> = new Set(['⌘', '⇧', '⌃', '⌥'])
export const isModifierSymbol = (key: string): boolean => MODIFIER_SYMBOLS.has(key)

/** The Hotkeys list, in the words the window uses. Actions without a key here, or not built yet, are left out. */
export function hotkeyRows(platform: Platform): HotkeyRow[] {
  const rows: HotkeyRow[] = BINDINGS.filter((binding) => binding.available)
    .map((binding) => ({ action: binding.label, group: binding.group, chords: shortcutChords(binding, platform) }))
    .filter((row) => row.chords.length > 0)
  const ctrl = platform === 'mac' ? MAC_GLYPH.ctrl : WORD.ctrl
  for (const key of TERMINAL_KEYS) rows.push({ action: key.label, group: key.group, chords: [[ctrl, key.letter]] })
  return rows
}
