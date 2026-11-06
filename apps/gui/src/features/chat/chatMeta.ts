/** How the agent is set up, from its own file: the permission mode and the model answering. */
export type ChatMeta = { readonly mode: string | null; readonly model: string | null }

/** Claude's permission modes in plain words. A mode not listed is shown as it was written. */
const MODE_WORDS: Readonly<Record<string, string>> = {
  default: 'Default',
  plan: 'Plan',
  acceptEdits: 'Accept edits',
  auto: 'Auto',
  dontAsk: "Don't ask",
  bypassPermissions: 'Bypass permissions',
}

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
