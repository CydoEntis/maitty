import type { MouseEvent } from 'react'
import { COPY_TOAST_ID, COPY_TOAST_MS } from '@/features/terminal/copyToast'
import { toast } from '@/lib/toast'

/** Where a selection is being edited, not read: releasing the mouse there copies nothing. */
const EDITED = 'textarea, input, [contenteditable="true"]'

/**
 * Selecting text in the chat copies it once the mouse is released, as selecting in a terminal pane does, and a
 * toast says so. Only a selection that started in this chat counts, so one left behind elsewhere is not copied
 * by a click here.
 */
export function copySelection(event: MouseEvent<HTMLElement>): void {
  if (event.target instanceof Element && event.target.closest(EDITED) !== null) return
  const selection = window.getSelection()
  if (selection === null || selection.isCollapsed || !event.currentTarget.contains(selection.anchorNode)) return
  const text = selection.toString()
  if (text.trim() === '') return
  navigator.clipboard.writeText(text).then(
    () => { toast.success('Copied', { id: COPY_TOAST_ID, duration: COPY_TOAST_MS }) },
    () => { toast.error('Could not copy the selection') },
  )
}
