/** How the agent is set up, from its own file: the permission mode and the model answering. */
export type ChatMeta = { readonly mode: string | null; readonly model: string | null }

/** Claude's permission modes in plain words. A mode not listed is shown as it was written. */
const MODE_WORDS: Readonly<Record<string, string>> = {
  default: 'Ask before running',
  plan: 'Plan',
  acceptEdits: 'Accept edits',
  auto: 'Auto',
  dontAsk: "Don't ask",
  bypassPermissions: 'Bypass permissions',
}

/**
 * The permission mode a Claude screen is showing, from the line under its prompt ("auto mode on (shift+tab to cycle)"); `default` when it
 * shows none, and null when the screen is empty. Used to show the new mode at once after Shift+Tab, before the agent's file says it.
 */
export function modeFromScreen(screen: string): string | null {
  const text = screen.trimEnd()
  if (text === '') return null
  const tail = text.split('\n').slice(-MODE_LINES).join('\n').toLowerCase()
  if (tail.includes('bypass permissions on')) return 'bypassPermissions'
  if (tail.includes('accept edits on')) return 'acceptEdits'
  if (tail.includes('plan mode on')) return 'plan'
  if (tail.includes('auto mode on')) return 'auto'
  return 'default'
}

/** How many of a screen's last lines can hold the mode line. */
const MODE_LINES = 8

/** A mode that asks about nothing: the header marks it, so it is never a quiet default. */
export const BYPASS_MODE = 'bypassPermissions'

export function modeLabel(mode: string | null): string | null {
  return mode === null ? null : (MODE_WORDS[mode] ?? mode)
}

// "claude-sonnet-5-5" and "claude-opus-4-1-20250805" are named by family and version.
const VERSIONED_MODEL = /^(?:claude-)?([a-z]+)-(\d+)-(\d+)(?:-\d{8})?$/
const CLAUDE_PREFIX = /^claude-/

export function modelLabel(model: string | null): string | null {
  if (model === null) return null
  const parts = VERSIONED_MODEL.exec(model)
  return parts === null ? model.replace(CLAUDE_PREFIX, '') : `${parts[1] ?? ''} ${parts[2] ?? ''}.${parts[3] ?? ''}`
}
