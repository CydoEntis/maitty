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
/** The most lines kept of what is asked above a menu's options: its end, where the question itself is. */
export const MOST_QUESTION_LINES = 12
/** The most lines of options shown, from the first: more than a screen holds, so a real menu is never cut. */
export const MOST_OPTION_LINES = 60
/** How many rules back a menu is looked for: Claude draws one above its question menu and one inside it. */
const RULES_TRIED = 2
const RULE = /^[\s─━═-]{8,}$/
const SELECTED = /^\s*❯/
const NUMBERED = /^\s*(?:❯\s*)?\d+\.\s/
const FIRST_NUMBERED = /^\s*(?:❯\s*)?1\.\s/
const INDENTED = /^\s+\S/
/** An option numbered 2 or more: the menu it belongs to began above it. */
const LATER_OPTION = /^\s*(?:❯\s*)?(?:[2-9]|\d{2,})\.\s/

/** Whether a line of the shown options is the one the terminal's cursor is on. */
export function isSelected(line: string): boolean {
  return SELECTED.test(line)
}

/**
 * Whether the line at `at` is a menu's selected option. Claude also puts `❯` before a message the person
 * sent, so the mark alone is not enough: the option is numbered, or another choice sits right beside it.
 */
function isSelectedOption(lines: readonly string[], at: number): boolean {
  const line = lines[at] ?? ''
  if (!isSelected(line)) return false
  return NUMBERED.test(line) || [lines[at - 1], lines[at + 1]].some((beside) => beside !== undefined && INDENTED.test(beside))
}

/**
 * Where the options start: the top of the unbroken block of options that holds the selected one, so a
 * numbered list in the question above is not taken for them. -1 when no option is selected.
 */
function startOfOptions(lines: readonly string[]): number {
  let start = lines.findIndex((_, at) => isSelectedOption(lines, at))
  while (start > 0 && !FIRST_NUMBERED.test(lines[start] ?? '')) {
    const above = lines[start - 1] ?? ''
    if (!NUMBERED.test(above) && !INDENTED.test(above)) break
    start -= 1
  }
  return start
}

/**
 * Whether these lines hold a menu from its top: one option is selected, and they do not start partway down
 * the numbers. Claude draws a rule inside its question menu, above the last option, so the lines under the
 * last rule can be the menu's end and not the menu.
 */
function holdsWholeMenu(lines: readonly string[]): boolean {
  const first = lines.find((line) => line.trim() !== '')
  return startOfOptions(lines) !== -1 && !(first !== undefined && LATER_OPTION.test(first))
}

function withoutBlankEnds(lines: readonly string[]): string[] {
  const first = lines.findIndex((line) => line.trim() !== '')
  if (first === -1) return []
  const last = lines.findLastIndex((line) => line.trim() !== '')
  return lines.slice(first, last + 1)
}

/** A menu's lines within what the card shows: the end of what is asked, then the options from the first. */
function cappedMenu(lines: readonly string[]): string[] {
  const start = startOfOptions(lines)
  return withoutBlankEnds([...lines.slice(0, start).slice(-MOST_QUESTION_LINES), ...lines.slice(start, start + MOST_OPTION_LINES)])
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
  const menuFrom = afterRules.slice(-RULES_TRIED).findLast((from) => holdsWholeMenu(lines.slice(from)))
  const kept = withoutBlankEnds(lines.slice(menuFrom ?? afterRules.at(-1) ?? 0).filter((line) => !isRule(line)))
  // A menu's lines are shown whole: the card wraps them, and a row the terminal wrapped arrives as one long line.
  if (menuFrom !== undefined) return cappedMenu(kept).join('\n')
  return kept.slice(-MOST_LINES).map((line) => line.slice(0, MOST_LINE_LENGTH)).join('\n')
}

/**
 * The shown question in two parts: what is asked, and the options from the first one down. The card keeps the
 * first in view and scrolls the second, so a long menu never pushes its question away.
 */
export function partsOfPrompt(prompt: string): { question: string; options: string } {
  const lines = prompt.split('\n')
  const start = startOfOptions(lines)
  if (start === -1) return { question: '', options: prompt }
  return { question: withoutBlankEnds(lines.slice(0, start)).join('\n'), options: lines.slice(start).join('\n') }
}
