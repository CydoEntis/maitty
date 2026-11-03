/** What a window may do to an open terminal from outside: look for text in it, and move focus to it. */
export type TerminalHandle = {
  /** Selects the next (or previous) match and scrolls to it; false when there is none. */
  find: (query: string, direction: 'next' | 'previous') => boolean
  clearFind: () => void
  focus: () => void
}
