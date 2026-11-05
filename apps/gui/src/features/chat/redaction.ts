// What the daemon puts where it keeps a secret out of the history (crates/maitty-core
// .../conversation/redact.rs). The strings are the contract: the daemon writes them, and the cards
// recognise them so a hidden value reads as hidden, never as an empty diff or an error.

/** Replaces a token- or key-shaped string, inside text that is otherwise kept as written. */
export const REDACTED_MARKER = '[redacted]'
/** Replaces the text of a secret file's content, edit or diff; the path and counts stay. */
export const OMITTED_MARKER = '[content of a secret file omitted]'

/** Whether `text` is nothing but the omitted-content marker. */
export function isOmitted(text: string): boolean {
  return text.trim() === OMITTED_MARKER
}

/** The muted line to show under text the daemon masked or left out, or `null` when it did neither. */
export function redactionNote(...texts: readonly (string | null)[]): string | null {
  const joined = texts.filter((text) => text !== null).join('\n')
  if (joined.includes(OMITTED_MARKER)) return 'The content of a secret file is not kept in the history, so it is not shown.'
  if (joined.includes(REDACTED_MARKER)) return 'Some of this looked like a secret (a token or a key), so the daemon hid it.'
  return null
}
