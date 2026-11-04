export type ProgramColor = { from: string; to: string }

const HASH_SEED = 5381
const HASH_SHIFT = 5

/** The darker end of a colour: the same hue pulled toward black. */
const deeper = (color: string) => `color-mix(in oklab, ${color} 58%, black)`
const between = (a: string, b: string) => `color-mix(in oklab, ${a} 50%, ${b})`

/**
 * The colours an unrecognised program's icon is given, as CSS expressions over the theme's tokens
 * so a theme switch recolours them. Amber and red are left out: they only ever mean status.
 */
const PROGRAM_COLORS: ReadonlyArray<string> = [
  'var(--accent)',
  'var(--ansi-mag)',
  'var(--ansi-blue)',
  'var(--ansi-green)',
  between('var(--accent)', 'var(--ansi-mag)'),
  between('var(--ansi-blue)', 'var(--ansi-green)'),
  between('var(--ansi-mag)', 'var(--ansi-blue)'),
  between('var(--accent)', 'var(--ansi-blue)'),
  between('var(--ansi-green)', 'var(--accent)'),
  'color-mix(in oklab, var(--ansi-mag) 68%, var(--danger))',
]

/** A program's icon colour, chosen from its name so the same program is always the same colour. */
export function programColor(name: string): ProgramColor {
  let hash = HASH_SEED
  for (let index = 0; index < name.length; index += 1) hash = ((hash << HASH_SHIFT) + hash + name.charCodeAt(index)) | 0
  const from = PROGRAM_COLORS[Math.abs(hash) % PROGRAM_COLORS.length] ?? 'var(--accent)'
  return { from, to: deeper(from) }
}
