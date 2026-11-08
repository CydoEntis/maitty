import { useEffect, useRef } from 'react'

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
    <div className="mx-settings-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onCancel() }}>
      <div
        role="alertdialog"
        aria-modal="true"
        aria-label={title}
        className="mx-rename"
        onKeyDown={(event) => {
          if (event.key !== 'Escape') return
          event.stopPropagation()
          onCancel()
        }}
      >
        <h2>{title}</h2>
        <p>{body}</p>
        <div className="mx-rename-actions">
          <button type="button" className="mx-confirm-button" onClick={onCancel}>Cancel</button>
          <button ref={confirm} type="button" className="mx-confirm-button is-danger" onClick={onConfirm}>{confirmLabel}</button>
        </div>
      </div>
    </div>
  )
}
