/** How a person answers an agent's permission prompt from the chat. */
export type AskChoice = 'approve' | 'deny'

/**
 * The keys that answer Claude Code's permission menu, checked against Claude Code 2.1.319 (see
 * docs/agent-status.md): the menu numbers its choices with "Yes" first, so `1` allows once; Esc
 * refuses. `y` and `n` do nothing there. Codex's keys have not been verified, so the daemon does
 * not mark its prompts answerable and the chat sends nothing for them.
 */
export const ANSWER_KEYS: Record<AskChoice, string> = { approve: '1', deny: '\x1b' }
