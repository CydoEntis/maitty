import { useEffect, useRef } from 'react'
import { AlertDialog, AlertDialogContent, AlertDialogDescription, AlertDialogTitle } from '@/components/ui/alert-dialog'

/** Asks before a session is ended. Enter confirms, Escape or a click outside cancels. */
export function ConfirmCloseDialog({ title, body, confirmLabel, onConfirm, onCancel }: { title: string; body: string; confirmLabel: string; onConfirm: () => void; onCancel: () => void }) {
  const confirm = useRef<HTMLButtonElement>(null)
  useEffect(() => {
    confirm.current?.focus()
    // The menu or key that opened this may still be handing focus back as it goes; claim it again.
    const timer = window.setTimeout(() => {
      if (document.activeElement !== confirm.current) confirm.current?.focus()
    }, 0)
    return () => { window.clearTimeout(timer) }
  }, [])
  return (
    <AlertDialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <AlertDialogContent
        aria-label={title}
        onOpenAutoFocus={(event) => { event.preventDefault(); confirm.current?.focus() }}
        onKeyDown={(event) => {
          // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
          if (event.key === 'Escape') event.stopPropagation()
        }}
      >
        <AlertDialogTitle asChild><h2>{title}</h2></AlertDialogTitle>
        <AlertDialogDescription asChild><p>{body}</p></AlertDialogDescription>
        <div className="mx-rename-actions">
          <button type="button" className="mx-confirm-button" onClick={onCancel}>Cancel</button>
          <button ref={confirm} type="button" className="mx-confirm-button is-danger" onClick={onConfirm}>{confirmLabel}</button>
        </div>
      </AlertDialogContent>
    </AlertDialog>
  )
}
