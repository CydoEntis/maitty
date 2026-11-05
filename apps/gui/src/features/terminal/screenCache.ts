/**
 * The last screen each pane showed, kept in memory so switching back to a session paints at once
 * instead of waiting for the daemon's replay. It is the raw output the view received since the
 * last replay, which is a complete screen to start from, so writing it into a fresh emulator
 * rebuilds what was on screen. The daemon's replay then confirms or replaces it.
 */

const KIB = 1024
const MIB = KIB * KIB
const PANE_MIB = 2
const ALL_PANES_MIB = 16

/** One pane's screen stops growing here; past it the screen is the older one and the replay fixes it. */
export const MAX_SCREEN_BYTES = PANE_MIB * MIB
/** All panes together, so many open sessions cannot hold unbounded memory. */
export const MAX_TOTAL_BYTES = ALL_PANES_MIB * MIB
/** At most this many panes are kept, least recently used first out. */
export const MAX_SCREENS = 16

export type CachedScreen = { seq: number; bytes: Uint8Array }

type Entry = { seq: number; parts: Uint8Array[]; size: number }

const screens = new Map<string, Entry>()
let totalBytes = 0

export function screenKey(machine: number, term: number, generation: number): string {
  return `${String(machine)}/${String(term)}/${String(generation)}`
}

function drop(key: string) {
  const entry = screens.get(key)
  if (!entry) return
  totalBytes -= entry.size
  screens.delete(key)
}

function evict() {
  // A Map iterates in insertion order and `touch` re-inserts, so the first key is the least recently used.
  while (screens.size > MAX_SCREENS || totalBytes > MAX_TOTAL_BYTES) {
    const oldest = screens.keys().next()
    if (oldest.done) return
    drop(oldest.value)
  }
}

function touch(key: string, entry: Entry) {
  screens.delete(key)
  screens.set(key, entry)
}

/** A complete screen (the daemon's history or a replay) replaces whatever was kept. */
export function replaceScreen(key: string, seq: number, bytes: Uint8Array) {
  drop(key)
  if (bytes.length > MAX_SCREEN_BYTES) return
  screens.set(key, { seq, parts: [bytes], size: bytes.length })
  totalBytes += bytes.length
  evict()
}

/** Output after the kept screen. Once the screen is full it stays as it was: still a valid, older screen. */
export function appendToScreen(key: string, seq: number, bytes: Uint8Array) {
  const entry = screens.get(key)
  if (!entry || entry.size + bytes.length > MAX_SCREEN_BYTES) return
  entry.parts.push(bytes)
  entry.size += bytes.length
  entry.seq = seq
  totalBytes += bytes.length
  touch(key, entry)
  evict()
}

export function cachedScreen(key: string): CachedScreen | null {
  const entry = screens.get(key)
  if (!entry) return null
  const bytes = new Uint8Array(entry.size)
  let offset = 0
  for (const part of entry.parts) {
    bytes.set(part, offset)
    offset += part.length
  }
  touch(key, entry)
  return { seq: entry.seq, bytes }
}

export function clearScreens() {
  screens.clear()
  totalBytes = 0
}
