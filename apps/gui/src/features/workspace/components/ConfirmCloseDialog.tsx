import { useEffect, useRef } from 'react'
import { AlertDialog, AlertDialogContent, AlertDialogDescription, AlertDialogTitle } from '@/components/ui/alert-dialog'

/**
 * Asks before something that cannot be taken back. Enter presses the focused button, Escape or a click outside cancels.
 * `firstFocus` is Cancel for a dialog that opens on its own while the person may be typing, so a stray Enter does nothing.
 */
export function ConfirmCloseDialog({ title, body, confirmLabel, onConfirm, onCancel, firstFocus = 'confirm' }: { title: string; body: string; confirmLabel: string; onConfirm: () => void; onCancel: () => void; firstFocus?: 'confirm' | 'cancel' }) {
  const confirmButton = useRef<HTMLButtonElement>(null)
  const cancelButton = useRef<HTMLButtonElement>(null)
  const first = firstFocus === 'confirm' ? confirmButton : cancelButton
  useEffect(() => {
    first.current?.focus()
    // The menu or key that opened this may still be handing focus back as it goes; claim it again.
    const timer = window.setTimeout(() => {
      if (document.activeElement !== first.current) first.current?.focus()
    }, 0)
    return () => { window.clearTimeout(timer) }
  }, [first])
  return (
    <AlertDialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <AlertDialogContent
        aria-label={title}
        onOpenAutoFocus={(event) => { event.preventDefault(); first.current?.focus() }}
        onKeyDown={(event) => {
          // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
          if (event.key === 'Escape') event.stopPropagation()
        }}
      >
        <AlertDialogTitle asChild><h2>{title}</h2></AlertDialogTitle>
        <AlertDialogDescription asChild><p>{body}</p></AlertDialogDescription>
        <div className="mx-rename-actions">
          <button ref={cancelButton} type="button" className="mx-confirm-button" onClick={onCancel}>Cancel</button>
          <button ref={confirmButton} type="button" className="mx-confirm-button is-danger" onClick={onConfirm}>{confirmLabel}</button>
        </div>
      </AlertDialogContent>
    </AlertDialog>
  )
}
