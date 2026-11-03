export type FolderColor = { from: string; to: string }

const HASH_SEED = 5381
const HASH_SHIFT = 5

/** The darker end of a folder colour: the same hue pulled toward black. */
const deeper = (color: string) => `color-mix(in oklab, ${color} 58%, black)`
const between = (a: string, b: string) => `color-mix(in oklab, ${a} 50%, ${b})`

/**
 * The colours projects are given on their own, as CSS expressions over the theme's tokens so a
 * theme switch recolours them. Amber and red are left out: they only ever mean status.
 */
const AUTO_COLORS: ReadonlyArray<{ id: string; color: string }> = [
  { id: 'auto-teal', color: 'var(--accent)' },
  { id: 'auto-violet', color: 'var(--ansi-mag)' },
  { id: 'auto-blue', color: 'var(--ansi-blue)' },
  { id: 'auto-green', color: 'var(--ansi-green)' },
  { id: 'auto-teal-violet', color: between('var(--accent)', 'var(--ansi-mag)') },
  { id: 'auto-blue-green', color: between('var(--ansi-blue)', 'var(--ansi-green)') },
  { id: 'auto-violet-blue', color: between('var(--ansi-mag)', 'var(--ansi-blue)') },
  { id: 'auto-teal-blue', color: between('var(--accent)', 'var(--ansi-blue)') },
  { id: 'auto-green-teal', color: between('var(--ansi-green)', 'var(--accent)') },
  { id: 'auto-rose', color: 'color-mix(in oklab, var(--ansi-mag) 68%, var(--danger))' },
]

const DEFAULT_COLOR: FolderColor = { from: 'var(--accent)', to: deeper('var(--accent)') }

/** The colours a project can be given by hand. Never amber or red: those stay reserved for status. */
export const PROJECT_COLORS: ReadonlyArray<{ id: string; label: string; color: string }> = [
  { id: 'teal', label: 'Teal', color: 'var(--accent)' },
  { id: 'violet', label: 'Violet', color: 'var(--ansi-mag)' },
  { id: 'blue', label: 'Blue', color: 'var(--ansi-blue)' },
  { id: 'green', label: 'Green', color: 'var(--ansi-green)' },
  { id: 'pink', label: 'Pink', color: '#F2A0C4' },
  { id: 'orange', label: 'Orange', color: '#E8946A' },
]

/** A folder's colour, chosen from its key so the same folder is always the same colour. */
export function folderColor(key: string, chosen?: string): FolderColor {
  const picked = PROJECT_COLORS.find((candidate) => candidate.id === chosen) ?? AUTO_COLORS.find((candidate) => candidate.id === chosen)
  if (picked) return { from: picked.color, to: deeper(picked.color) }
  return { from: AUTO_COLORS[preferredIndex(key)]?.color ?? DEFAULT_COLOR.from, to: DEFAULT_COLOR.to }
}

function preferredIndex(key: string): number {
  let hash = HASH_SEED
  for (let index = 0; index < key.length; index += 1) hash = ((hash << HASH_SHIFT) + hash + key.charCodeAt(index)) | 0
  return Math.abs(hash) % AUTO_COLORS.length
}

/**
 * A colour id for every project, so no two projects share one while colours last. Each project
 * keeps the colour you gave it; the rest take the colour their path points to, or the next unused
 * one when another project already has it. Taken in path order, so the same projects always get
 * the same colours.
 */
export function assignProjectColors(keys: readonly string[], chosen: Readonly<Record<string, string>>): Map<string, string> {
  const assigned = new Map<string, string>()
  const uses = new Map<string, number>()
  const take = (key: string, id: string) => {
    assigned.set(key, id)
    uses.set(id, (uses.get(id) ?? 0) + 1)
  }
  for (const key of keys) {
    const mine = chosen[key]
    if (mine) take(key, mine)
  }
  for (const key of [...keys].sort()) {
    if (assigned.has(key)) continue
    const start = preferredIndex(key)
    const order = AUTO_COLORS.map((_, offset) => AUTO_COLORS[(start + offset) % AUTO_COLORS.length]?.id ?? '')
    const free = order.find((id) => !uses.has(id))
    // More projects than colours: reuse the one used least, starting from the preferred.
    take(key, free ?? [...order].sort((a, b) => (uses.get(a) ?? 0) - (uses.get(b) ?? 0))[0] ?? AUTO_COLORS[0]?.id ?? '')
  }
  return assigned
}
