import type { Chord } from '../model/shortcuts'

/** A tooltip's parts (v18 B.4): the name, the keys that do it as keycaps, and a muted second line. */
export type TipContent = { name: string; keys: Chord | null; hint: string | null }

const HINT_SEPARATOR = ' · '
const TRAILING_PARENTHESES = /\s*\(([^()]+)\)\s*$/
const MAC_MODIFIERS = '⌘⌃⌥⇧'
const MAC_CHORD = new RegExp(`^([${MAC_MODIFIERS}]+)(\\S+)$`)
const MODIFIER_WORDS: ReadonlySet<string> = new Set(['Ctrl', 'Shift', 'Alt', 'Cmd', 'Meta', 'Option'])
/** Keys that are a shortcut on their own, such as "Close (Esc)". */
const LONE_KEYS: ReadonlySet<string> = new Set(['Esc', 'Enter', 'Tab', 'Space', 'Backspace', 'Delete'])

/** The keys inside a trailing "(…)", or null when the text is something else, such as "(3)" or "(read only)". */
function chordOf(inner: string): Chord | null {
  const text = inner.trim()
  const mac = MAC_CHORD.exec(text)
  if (mac?.[1] !== undefined && mac[2] !== undefined) return [...Array.from(mac[1]), mac[2]]
  if (LONE_KEYS.has(text)) return [text]
  // A "+" that ends the text is the key itself ("Ctrl++"), not a separator.
  const parts = text.split(/\+(?=.)/)
  const hasModifier = parts.length > 1 && parts.slice(0, -1).every((part) => MODIFIER_WORDS.has(part))
  return hasModifier ? parts : null
}

function withoutKeys(text: string): { rest: string; keys: Chord | null } {
  const found = TRAILING_PARENTHESES.exec(text)
  const keys = found?.[1] === undefined ? null : chordOf(found[1])
  return keys && found ? { rest: text.slice(0, found.index), keys } : { rest: text, keys: null }
}

/**
 * "Split right (Ctrl+Shift+D)" is a name with its keys; "Name · more words" has a second line.
 * The keys may close either the whole text or the name before the " · ".
 */
export function parseTip(text: string): TipContent {
  const whole = withoutKeys(text.trim())
  const split = whole.rest.indexOf(HINT_SEPARATOR)
  if (split < 0) return { name: whole.rest.trim(), keys: whole.keys, hint: null }
  const head = withoutKeys(whole.rest.slice(0, split))
  const hint = whole.rest.slice(split + HINT_SEPARATOR.length).trim()
  return { name: head.rest.trim(), keys: whole.keys ?? head.keys, hint: hint === '' ? null : hint }
}
