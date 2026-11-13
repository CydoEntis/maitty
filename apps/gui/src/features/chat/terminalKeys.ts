import { linesOfScreen } from './screenText'

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

/** The most lines shown of a screen whose menu was not found: the last few, since the rest is the conversation above. */
const MOST_LINES = 14
const MOST_LINE_LENGTH = 140
const RULE = /^[\s─━═-]{8,}$/
const SELECTED = /^\s*❯/
const OPTION = /^\s*(?:❯|\d+\.\s)/
/** An option numbered 2 or more: the menu it belongs to began above it. */
const LATER_OPTION = /^\s*(?:❯\s*)?(?:[2-9]|\d{2,})\.\s/

/** Whether a line of the shown question is the option the terminal's cursor is on. */
export function isSelected(line: string): boolean {
  return SELECTED.test(line)
}

/**
 * Whether these lines hold a menu from its top: one option is selected, and they do not start partway down
 * the numbers. Claude draws a rule inside its question menu, above the last option, so the lines under the
 * last rule can be the menu's end and not the menu.
 */
function holdsWholeMenu(lines: readonly string[]): boolean {
  const first = lines.find((line) => line.trim() !== '')
  return lines.some(isSelected) && !(first !== undefined && LATER_OPTION.test(first))
}

function withoutBlankEnds(lines: readonly string[]): string[] {
  const first = lines.findIndex((line) => line.trim() !== '')
  if (first === -1) return []
  const last = lines.findLastIndex((line) => line.trim() !== '')
  return lines.slice(first, last + 1)
}

/**
 * The part of the agent's terminal screen that is its question, as plain text: the whole menu under the rule
 * that opens it, or else the last few lines under the last rule. The text is only shown; nothing here decides
 * status or reads an answer (ADR 0003).
 */
export function promptOfScreen(screen: string): string {
  const lines = linesOfScreen(screen)
  const isRule = (line: string) => RULE.test(line.trim())
  const afterRules = lines.flatMap((line, at) => (isRule(line) ? [at + 1] : []))
  const menuFrom = afterRules.findLast((from) => holdsWholeMenu(lines.slice(from)))
  const from = menuFrom ?? afterRules.at(-1) ?? 0
  const kept = withoutBlankEnds(lines.slice(from).filter((line) => !isRule(line)))
  const shown = menuFrom === undefined ? kept.slice(-MOST_LINES) : kept
  return shown.map((line) => line.slice(0, MOST_LINE_LENGTH)).join('\n')
}

/**
 * The shown question in two parts: what is asked, and the options from the first one down. The card keeps the
 * first in view and scrolls the second, so a long menu never pushes its question away.
 */
export function partsOfPrompt(prompt: string): { question: string; options: string } {
  const lines = prompt.split('\n')
  const first = lines.findIndex((line) => OPTION.test(line))
  if (first === -1) return { question: '', options: prompt }
  return { question: withoutBlankEnds(lines.slice(0, first)).join('\n'), options: lines.slice(first).join('\n') }
}
