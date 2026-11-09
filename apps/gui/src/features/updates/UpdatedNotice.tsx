import { useEffect, useState } from 'react'
import { X } from 'lucide-react'
import { Dialog, DialogContent, DialogDescription, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { noteLines } from './noteLines'
import type { RestartedUpdate } from './updateMemory'

/** How long the toast stays before it goes on its own. */
export const UPDATED_TOAST_MS = 9000

type Props = {
  update: RestartedUpdate
  /** Called once the toast is gone and the dialog, if it was opened, is closed. */
  onDone: () => void
}

/** After a restart into an update: a quiet pill with a way into What's new, and that dialog. */
export function UpdatedNotice({ update, onDone }: Props) {
  const [toastShown, setToastShown] = useState(true)
  const [dialogOpen, setDialogOpen] = useState(false)
  const lines = noteLines(update.notes)

  useEffect(() => {
    if (!toastShown) return
    const timer = window.setTimeout(() => { setToastShown(false) }, UPDATED_TOAST_MS)
    return () => { window.clearTimeout(timer) }
  }, [toastShown])

  useEffect(() => {
    if (!toastShown && !dialogOpen) onDone()
  }, [toastShown, dialogOpen, onDone])

  return (
    <>
      {toastShown && (
        <div
          role="status"
          className="fixed bottom-5 left-1/2 z-[60] flex -translate-x-1/2 items-center gap-2 whitespace-nowrap rounded-full border border-[var(--line2)] bg-[var(--raise)] py-1.5 pr-1.5 pl-3.5 text-[12.5px] text-[var(--fg)] shadow-[0_12px_40px_rgba(0,0,0,.45)]"
        >
          <span aria-hidden="true" className="size-1.5 rounded-full bg-[var(--ok)]" />
          <strong className="font-semibold">Updated to {update.version}</strong>
          <span aria-hidden="true" className="text-[var(--dim)]">·</span>
          <button type="button" className="cursor-pointer rounded-full px-1.5 py-0.5 font-semibold text-[var(--accent-text)] hover:underline" onClick={() => { setDialogOpen(true); setToastShown(false) }}>What's new</button>
          <button type="button" aria-label="Dismiss" className="grid size-6 cursor-pointer place-items-center rounded-full text-[var(--muted)] hover:text-[var(--fg)]" onClick={() => { setToastShown(false) }}><X size={13} aria-hidden="true" /></button>
        </div>
      )}
      <Dialog open={dialogOpen} onOpenChange={setDialogOpen}>
        <DialogContent className="sm:max-w-[460px]">
          <DialogHeader>
            <DialogTitle>What's new in Maitty {update.version}</DialogTitle>
            <DialogDescription>{update.previous ? `Updated from ${update.previous}.` : 'Maitty is up to date.'}</DialogDescription>
          </DialogHeader>
          {lines.length > 0 && <ul className="m-0 grid list-disc gap-1.5 pl-5 text-[13px] text-[var(--fg2)]">{lines.map((line) => <li key={line}>{line}</li>)}</ul>}
          <p className="m-0 text-[12px] text-[var(--muted)]">{update.automatic ? 'Installed while nothing was running. No session was interrupted.' : 'No session was interrupted.'}</p>
        </DialogContent>
      </Dialog>
    </>
  )
}
