import { useCallback, useEffect, useRef } from 'react'
import { toast } from 'sonner'
import { cancelFileSend, listenFileSends, newTransferId, sendFilesToMachine, type SendEvent } from '@/api/fileSends'
import { errorMessage } from '@/lib/error-message'
import { remotePathText } from './dropRoute'
import { SendProgress } from './SendProgress'

/** What the window knows of a send it started. */
type Send = {
  /** The pane the paths are typed into. */
  sessionId: string
  machine: string
  count: number
  /** The file now going, for the toasts. */
  file: string
  /** Paths kept on the machine that could not be typed (the pane went away, or the path was unfit). */
  untyped: string[]
}

type Options = {
  /** Types `text` into the pane of `sessionId`; false when that pane is no longer open. */
  typeInto: (sessionId: string, text: string) => boolean
}

const noun = (count: number) => (count === 1 ? 'file' : 'files')

/** How long a finished send's toast stays, as sonner's own default. */
const TOAST_MS = 4000

/** Settings that put a toast back to a plain one after it was a progress toast; sonner merges updates into the toast it has. */
const PLAIN = { description: undefined, action: undefined, dismissible: true, closeButton: true, duration: TOAST_MS } as const

/**
 * Files dropped on a pane of another machine: starts the send, shows a progress toast with Cancel,
 * and types each path the machine kept the file at into the pane as it lands (ADR 0050). The host
 * does the work off the UI thread and only reports back.
 */
export function useFileSends({ typeInto }: Options) {
  const sends = useRef(new Map<string, Send>())
  const typeRef = useRef(typeInto)
  useEffect(() => { typeRef.current = typeInto })

  const showProgress = useCallback((transfer: string, send: Send, index: number, percent: number | null) => {
    const where = send.count > 1 ? ` (${String(index + 1)} of ${String(send.count)})` : ''
    toast.loading(`Sending ${send.file} to ${send.machine}${where}`, {
      id: transfer,
      duration: Infinity,
      // Hiding the toast would not stop the send; Cancel does.
      dismissible: false,
      closeButton: false,
      description: <SendProgress file={send.file} percent={percent} />,
      action: {
        label: 'Cancel',
        onClick: (event) => {
          // The toast stays until the host says the send has stopped.
          event.preventDefault()
          cancelFileSend(transfer).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not stop the send')) })
        },
      },
    })
  }, [])

  useEffect(() => {
    let gone = false
    let stop: (() => void) | undefined
    const onEvent = (event: SendEvent) => {
      const send = sends.current.get(event.transfer)
      if (send === undefined) return
      switch (event.kind) {
        case 'progress':
          send.file = event.file
          send.count = event.count
          showProgress(event.transfer, send, event.index, event.percent)
          return
        case 'sent': {
          send.file = event.file
          send.count = event.count
          // The machine is not trusted: only a path fit to type is, and only into the pane the drop was for.
          const text = remotePathText(event.remotePath)
          if (text === null || !typeRef.current(send.sessionId, text)) send.untyped.push(text === null ? event.file : event.remotePath)
          if (event.index + 1 < event.count) showProgress(event.transfer, send, event.index + 1, null)
          return
        }
        case 'done':
          sends.current.delete(event.transfer)
          toast.success(`Sent ${String(event.count)} ${noun(event.count)} to ${send.machine}`, {
            ...PLAIN,
            id: event.transfer,
            description: send.untyped.length > 0 ? `Not typed into the pane: ${send.untyped.join(', ')}` : undefined,
          })
          return
        case 'failed':
          sends.current.delete(event.transfer)
          toast.error(event.unsent > 1 ? `${event.message} (${String(event.unsent - 1)} more not sent)` : event.message, { ...PLAIN, id: event.transfer })
          return
        case 'cancelled':
          sends.current.delete(event.transfer)
          toast.message(`Stopped sending to ${send.machine}${event.sent > 0 ? `; ${String(event.sent)} of ${String(send.count)} were sent` : ''}`, { ...PLAIN, id: event.transfer })
          return
      }
    }
    listenFileSends(onEvent).then((unlisten) => {
      if (gone) unlisten()
      else stop = unlisten
    }, (error: unknown) => { toast.error(errorMessage(error, 'Could not follow file sends')) })
    return () => {
      gone = true
      stop?.()
    }
  }, [showProgress])

  /** Send `paths` to `machine` for the pane `sessionId`. The host's refusals (a folder, too many, too big) show as an error toast. */
  return useCallback((sessionId: string, machine: string, paths: readonly string[]) => {
    const transfer = newTransferId()
    const send: Send = { sessionId, machine, count: paths.length, file: paths.length === 1 ? (paths[0]?.split(/[\\/]/).pop() ?? 'file') : `${String(paths.length)} files`, untyped: [] }
    sends.current.set(transfer, send)
    showProgress(transfer, send, 0, null)
    sendFilesToMachine(machine, paths, transfer).catch((error: unknown) => {
      sends.current.delete(transfer)
      toast.error(errorMessage(error, `Could not send to ${machine}`), { ...PLAIN, id: transfer })
    })
  }, [showProgress])
}
