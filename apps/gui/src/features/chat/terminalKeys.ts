/** The keys a person presses to answer a menu the agent draws in its terminal, as the terminal receives them. */
const ESC_CODE = 27
const ENTER_CODE = 13
const ESC = String.fromCharCode(ESC_CODE)
const ENTER = String.fromCharCode(ENTER_CODE)

export const KEY_UP = `${ESC}[A`
export const KEY_DOWN = `${ESC}[B`
export const KEY_RIGHT = `${ESC}[C`
export const KEY_LEFT = `${ESC}[D`
export const KEY_ENTER = ENTER
export const KEY_ESCAPE = ESC

/** The most lines of the screen the tray shows: a menu is a few lines at the bottom, the rest is the conversation above it. */
const MOST_LINES = 14
const MOST_LINE_LENGTH = 140
const RULE = /^[\s─━═-]{8,}$/

/**
 * The part of the agent's terminal screen that is its question: what follows the last horizontal rule, or
 * else the last few lines. The text is only shown; nothing here decides status or reads an answer (ADR 0003).
 */
export function promptOfScreen(screen: string): string {
  const lines = screen.split('\n').map((line) => line.trimEnd())
  let from = 0
  for (const [at, line] of lines.entries()) if (RULE.test(line.trim())) from = at + 1
  const kept = lines.slice(from).filter((line, at, all) => line.trim() !== '' || (at > 0 && at < all.length - 1))
  while (kept.length > 0 && (kept[kept.length - 1] ?? '').trim() === '') kept.pop()
  return kept.slice(-MOST_LINES).map((line) => line.slice(0, MOST_LINE_LENGTH)).join('\n')
}
