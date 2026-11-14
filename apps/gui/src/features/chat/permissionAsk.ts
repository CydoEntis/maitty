import type { ApprovalDecision } from './chatEvents'

/** Each answer to a request the chat runs, and the key that gives it: the same letters as typing into the terminal. */
export const ANSWER_KEY: Readonly<Record<Exclude<ApprovalDecision, 'Cancel'>, string>> = { Accept: 'y', AcceptForSession: 'a', Decline: 'n' }

/** How a person answers an agent's permission prompt from the chat. */
export type AskChoice = 'approve' | 'deny'

/**
 * The keys that answer Claude Code's permission menu, checked against Claude Code 2.1.319 (see
 * docs/agent-status.md): the menu numbers its choices with "Yes" first, so `1` allows once; Esc
 * refuses. `y` and `n` do nothing there.
 */
export const ANSWER_KEYS: Record<AskChoice, string> = { approve: '1', deny: '\x1b' }

/**
 * The keys that answer Codex's command and file-change approval menus, checked against codex-cli
 * 0.166.0 in a terminal: `y` ("Yes, proceed") ran the command or applied the patch once, Esc ("No,
 * and tell Codex what to do differently") ran and changed nothing. The daemon marks a Codex prompt
 * answerable only while its screen is that menu (`codex_prompt_answerable`), so any other Codex
 * prompt falls back to the terminal's own menu and the arrow-key row.
 */
export const CODEX_ANSWER_KEYS: Record<AskChoice, string> = { approve: 'y', deny: '\x1b' }

/**
 * The key that picks a choice of a question Claude asks: its menu numbers the choices from 1, so the
 * number is the key. The menu also lists "Type something" and "Chat about this" after them, which
 * are left to the terminal.
 */
export const choiceKey = (index: number): string => String(index + 1)

/**
 * The keys that answer a prompt of several questions, checked against Claude Code 2.1.322: the number of each
 * question's choice, in the order the prompt shows the questions (each one moves it on to the next), then `1`,
 * which is "Submit answers" on the page that reviews them.
 */
export function answerKeys(picked: readonly number[]): string[] {
  return [...picked.map(choiceKey), SUBMIT_ANSWERS_KEY]
}
const SUBMIT_ANSWERS_KEY = '1'
/** The prompt redraws after each key; the next one waits this long, so it lands on the question that follows. */
export const ANSWER_STEP_MS = 250
