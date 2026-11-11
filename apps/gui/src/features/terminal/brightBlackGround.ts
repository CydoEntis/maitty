/**
 * Programs paint a "bright black" background expecting a faint slab, as Claude Code does under the person's
 * prompt. The palette's bright black is a text colour here, readable on the card, so as a background it is a
 * glaring bar. xterm has one colour per slot, so the background form is rewritten to a true colour before the
 * bytes reach it. The TUI does the same when it draws.
 */

export type Channels = readonly [red: number, green: number, blue: number]

const ESC = 0x1b
const CSI_OPEN = 0x5b
const SGR_FINAL = 0x6d
/** `ESC [`: where a control sequence's parameters start. */
const CSI_PREFIX_LENGTH = 2
const DIGIT_FIRST = 0x30
const DIGIT_LAST = 0x39
const SEPARATOR = 0x3b
const PARAMETER_FIRST = 0x20
const PARAMETER_LAST = 0x3f

const BRIGHT_BLACK_GROUND = '100'
const BRIGHT_BLACK_INDEX = '8'
const SET_GROUND = '48'
const COLOUR_SETTERS: ReadonlySet<string> = new Set(['38', SET_GROUND, '58'])
const INDEXED = '5'
const TRUE_COLOUR = '2'
/** How many parameters an indexed (`48;5;n`) and a true (`48;2;r;g;b`) colour take, the setter included. */
const INDEXED_LENGTH = 3
const TRUE_COLOUR_LENGTH = 5
/** The longest unfinished sequence held for the next write. A longer one is not a colour change, so it goes on as it is. */
const HELD_MAX = 64

const isSgrByte = (byte: number) => (byte >= DIGIT_FIRST && byte <= DIGIT_LAST) || byte === SEPARATOR

/** The SGR parameters with every bright black background named as `ground`, or `null` when there is none. */
function rewritten(parameters: string, ground: string): string | null {
  const parts = parameters.split(';')
  const out: string[] = []
  let changed = false
  for (let index = 0; index < parts.length; index += 1) {
    const part = parts[index] ?? ''
    if (part === BRIGHT_BLACK_GROUND) {
      out.push(ground)
      changed = true
    } else if (COLOUR_SETTERS.has(part) && parts[index + 1] === INDEXED) {
      // The index of a 256-colour setter is a colour, never a parameter of its own: `38;5;100` is not a background.
      const slab = part === SET_GROUND && parts[index + INDEXED_LENGTH - 1] === BRIGHT_BLACK_INDEX
      if (slab) out.push(ground)
      else out.push(...parts.slice(index, index + INDEXED_LENGTH))
      changed ||= slab
      index += INDEXED_LENGTH - 1
    } else if (COLOUR_SETTERS.has(part) && parts[index + 1] === TRUE_COLOUR) {
      out.push(...parts.slice(index, index + TRUE_COLOUR_LENGTH))
      index += TRUE_COLOUR_LENGTH - 1
    } else {
      out.push(part)
    }
  }
  return changed ? out.join(';') : null
}

/** Where the sequence opened at `start` ends (its final byte), or -1 when the bytes stop before it does. */
function sequenceEnd(bytes: Uint8Array, start: number): number {
  if (start + 1 >= bytes.length) return -1
  if (bytes[start + 1] !== CSI_OPEN) return start + 1
  for (let index = start + CSI_PREFIX_LENGTH; index < bytes.length; index += 1) {
    const byte = bytes[index] ?? 0
    if (byte < PARAMETER_FIRST || byte > PARAMETER_LAST) return index
  }
  return -1
}

function joined(first: Uint8Array, second: Uint8Array): Uint8Array {
  if (first.length === 0) return second
  const all = new Uint8Array(first.length + second.length)
  all.set(first)
  all.set(second, first.length)
  return all
}

/**
 * A rewriter for one terminal's output. It keeps the unfinished end of a write, so a colour change cut in two
 * by the transport is still seen whole. `ground` is read at each write, so a change of theme applies to what
 * is drawn next.
 */
export function brightBlackGround(ground: () => Channels | null): (bytes: Uint8Array) => Uint8Array {
  let held: Uint8Array = new Uint8Array(0)
  const encoder = new TextEncoder()
  const decoder = new TextDecoder()
  return (incoming) => {
    const channels = ground()
    const bytes = joined(held, incoming)
    held = new Uint8Array(0)
    if (channels === null) return bytes
    const colour = `${SET_GROUND};${TRUE_COLOUR};${channels.join(';')}`
    const pieces: Uint8Array[] = []
    let copied = 0
    let end = bytes.length
    for (let start = bytes.indexOf(ESC); start !== -1; start = bytes.indexOf(ESC, start + 1)) {
      const last = sequenceEnd(bytes, start)
      if (last === -1) {
        if (bytes.length - start <= HELD_MAX) {
          held = bytes.slice(start)
          end = start
        }
        break
      }
      if (bytes[last] !== SGR_FINAL || bytes[start + 1] !== CSI_OPEN) continue
      const body = bytes.subarray(start + CSI_PREFIX_LENGTH, last)
      if (!body.every(isSgrByte)) continue
      const next = rewritten(decoder.decode(body), colour)
      if (next === null) continue
      pieces.push(bytes.subarray(copied, start + CSI_PREFIX_LENGTH), encoder.encode(next))
      copied = last
    }
    if (pieces.length === 0) return end === bytes.length ? bytes : bytes.subarray(0, end)
    pieces.push(bytes.subarray(copied, end))
    return pieces.reduce(joined, new Uint8Array(0))
  }
}
