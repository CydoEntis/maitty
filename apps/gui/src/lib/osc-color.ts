const HEX_COLOR = /^#([0-9a-f]{2})([0-9a-f]{2})([0-9a-f]{2})$/i

/**
 * A colour as a program's OSC 10/11 colour query is answered: `rgb:rrrr/gggg/bbbb`, each channel
 * doubled to 16 bits. Null for a colour not written as `#rrggbb`, which the themes never produce.
 */
export function oscColor(css: string): string | null {
  const match = HEX_COLOR.exec(css)
  if (!match) return null
  const [, red = '', green = '', blue = ''] = match
  return `rgb:${red}${red}/${green}${green}/${blue}${blue}`.toLowerCase()
}
