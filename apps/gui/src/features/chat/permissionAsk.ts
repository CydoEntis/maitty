import type { ApprovalDecision } from './chatEvents'

/** Each answer to a request the chat runs, and the key that gives it: the same letters as typing into the terminal. */
export const ANSWER_KEY: Readonly<Record<Exclude<ApprovalDecision, 'Cancel'>, string>> = { Accept: 'y', AcceptForSession: 'a', Decline: 'n' }

/** How a person answers an agent's permission prompt from the chat. */
export type AskChoice = 'approve' | 'deny'

/**
 * The keys that answer Claude Code's permission menu, checked against Claude Code 2.1.319 (see
 * docs/agent-status.md): the menu numbers its choices with "Yes" first, so `1` allows once; Esc
 * refuses. `y` and `n` do nothing there. Codex's keys have not been verified, so the daemon does
 * not mark its prompts answerable and the chat sends nothing for them.
 */
export const ANSWER_KEYS: Record<AskChoice, string> = { approve: '1', deny: '\x1b' }

/**
 * The key that picks a choice of a question Claude asks: its menu numbers the choices from 1, so the
 * number is the key. The menu also lists "Type something" and "Chat about this" after them, which
 * are left to the terminal.
 */
export const choiceKey = (index: number): string => String(index + 1)
