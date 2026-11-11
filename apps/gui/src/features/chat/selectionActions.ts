import type { MouseEvent } from 'react'

/** Where a selection is being edited, not read: releasing the mouse there offers nothing. */
const EDITED = 'textarea, input, [contenteditable="true"]'
/** The bar itself: a release on one of its buttons is that button's click, not a new selection. */
export const SELECTION_BAR = '.mx-select-bar'

/** Text selected in the chat, and where the mouse let go of it, measured from the chat's top left corner. */
export type PickedText = { readonly text: string; readonly x: number; readonly y: number }

/**
 * What a mouse release in the chat selected, or null when it selected nothing to act on: a plain click, a
 * selection that began outside this chat (left behind elsewhere), or one in the message box. `undefined` means
 * the release was on the selection bar, so what is picked stands.
 */
export function pickedText(event: MouseEvent<HTMLElement>): PickedText | null | undefined {
  const target = event.target
  if (target instanceof Element && target.closest(SELECTION_BAR) !== null) return undefined
  if (target instanceof Element && target.closest(EDITED) !== null) return null
  const selection = window.getSelection()
  if (selection === null || selection.isCollapsed || !event.currentTarget.contains(selection.anchorNode)) return null
  const text = selection.toString()
  if (text.trim() === '') return null
  const frame = event.currentTarget.getBoundingClientRect()
  // The chat may be drawn larger or smaller than it is laid out (Settings › Chat text size): the bar is placed in
  // the chat's own measure, so the distance on screen is taken back to it.
  const drawn = event.currentTarget.offsetWidth > 0 ? frame.width / event.currentTarget.offsetWidth : 1
  const scale = drawn > 0 ? drawn : 1
  return { text, x: (event.clientX - frame.left) / scale, y: (event.clientY - frame.top) / scale }
}

/** The text as a Markdown quote: each line behind `> `, with the blank lines at its ends dropped. */
export function quoteOf(text: string): string {
  return text.replace(/^\s*\n|\n\s*$/g, '').split('\n').map((line) => (line.trim() === '' ? '>' : `> ${line.trimEnd()}`)).join('\n')
}

/** The draft with the quote added under what was already written, and an empty line after it to reply on. */
export function draftWithQuote(draft: string, text: string): string {
  const written = draft.trimEnd()
  return `${written === '' ? '' : `${written}\n\n`}${quoteOf(text)}\n\n`
}
