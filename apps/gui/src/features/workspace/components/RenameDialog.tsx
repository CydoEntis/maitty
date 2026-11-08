import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { useEffect, useRef, useState } from 'react'
import { MAX_SESSION_NAME_LENGTH, type SessionView } from '../model/session'

/** Names a session. An empty name goes back to showing where the session is. */
export function RenameDialog({ session, onSave, onCancel }: { session: SessionView; onSave: (name: string) => void; onCancel: () => void }) {
  const [value, setValue] = useState(session.name ?? '')
  const input = useRef<HTMLInputElement>(null)
  useEffect(() => {
    const focusField = () => {
      input.current?.focus()
      input.current?.select()
    }
    focusField()
    // The right-click menu that opened this is still closing, and its focus trap takes focus back as
    // it goes; claim it again once it has finished.
    const timer = window.setTimeout(() => {
      if (document.activeElement !== input.current) focusField()
    }, 0)
    return () => { window.clearTimeout(timer) }
  }, [])
  return (
    <Dialog open onOpenChange={(open) => { if (!open) onCancel() }}>
      <DialogContent unstyled asChild aria-describedby={undefined}>
      <form
        aria-label="Rename session"
        className="mx-rename"
        onSubmit={(event) => {
          event.preventDefault()
          onSave(value.trim())
        }}
        onKeyDown={(event) => {
          // Radix closes on Escape; this keeps the key from reaching the app's own handlers.
          if (event.key === 'Escape') event.stopPropagation()
        }}
      >
        <DialogTitle asChild><h2>Rename session</h2></DialogTitle>
        <p>{session.process} in {session.cwdLabel}. Leave it empty to show the folder again.</p>
        <input ref={input} aria-label="Session name" value={value} maxLength={MAX_SESSION_NAME_LENGTH} placeholder={session.title} onChange={(event) => { setValue(event.target.value) }} />
        <div className="mx-rename-actions">
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
          <button type="submit" className="mx-text-button is-primary">Save</button>
        </div>
      </form>
      </DialogContent>
    </Dialog>
  )
}
