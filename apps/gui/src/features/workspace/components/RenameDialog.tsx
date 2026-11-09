import { Dialog, DialogContent, DialogTitle } from '@/components/ui/dialog'
import { useEffect, useRef, useState } from 'react'
import { MAX_MACHINE_NICKNAME_LENGTH } from '../model/machines'
import { MAX_SESSION_NAME_LENGTH, type SessionView } from '../model/session'

type NameDialogProps = { heading: string; hint: string; field: string; initial: string; placeholder: string; maxLength: number; onSave: (name: string) => void; onCancel: () => void }

/** Names a session. An empty name goes back to showing where the session is. */
export function RenameDialog({ session, onSave, onCancel }: { session: SessionView; onSave: (name: string) => void; onCancel: () => void }) {
  return <NameDialog heading="Rename session" hint={`${session.process} in ${session.cwdLabel}. Leave it empty to show the folder again.`} field="Session name" initial={session.name ?? ''} placeholder={session.title} maxLength={MAX_SESSION_NAME_LENGTH} onSave={onSave} onCancel={onCancel} />
}

/** Gives another machine a name of your own in the sidebar; its real name, which its sessions go by, is untouched. */
export function RenameMachineDialog({ machine, nickname, onSave, onCancel }: { machine: string; nickname: string; onSave: (name: string) => void; onCancel: () => void }) {
  return <NameDialog heading="Rename machine" hint={`Only the name shown here changes; ${machine} keeps its own. Leave it empty to show ${machine} again.`} field="Machine name" initial={nickname} placeholder={machine} maxLength={MAX_MACHINE_NICKNAME_LENGTH} onSave={onSave} onCancel={onCancel} />
}

function NameDialog({ heading, hint, field, initial, placeholder, maxLength, onSave, onCancel }: NameDialogProps) {
  const [value, setValue] = useState(initial)
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
        aria-label={heading}
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
        <DialogTitle asChild><h2>{heading}</h2></DialogTitle>
        <p>{hint}</p>
        <input ref={input} aria-label={field} value={value} maxLength={maxLength} placeholder={placeholder} onChange={(event) => { setValue(event.target.value) }} />
        <div className="mx-rename-actions">
          <button type="button" className="mx-text-button" onClick={onCancel}>Cancel</button>
          <button type="submit" className="mx-text-button is-primary">Save</button>
        </div>
      </form>
      </DialogContent>
    </Dialog>
  )
}
