import type { MouseEvent } from 'react'

/** What a click is for when it lands on one of these: it takes focus itself, or does its own thing. */
const OWN_CLICK = 'a, button, input, textarea, select, summary, [role="button"], [role="radio"], [role="menuitem"], [contenteditable="true"]'

/**
 * A click on the chat's own text or empty space gives the focus back to the message box, as in any chat app, instead of
 * dropping it on the page, which left you clicking again to get back to typing. Controls keep their own click, and a drag
 * that selected text is left alone so the selection stays for its bar (Copy, Quote).
 */
export function keepComposerFocus(event: MouseEvent<HTMLElement>): void {
  const target = event.target
  if (!(target instanceof Element) || target.closest(OWN_CLICK) !== null) return
  if (window.getSelection()?.isCollapsed === false) return
  event.currentTarget.querySelector<HTMLTextAreaElement>('textarea.mx-box-text:not(:disabled)')?.focus({ preventScroll: true })
}
