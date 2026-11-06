import { useCallback, useEffect, useRef } from 'react'
import { toast } from 'sonner'
import { listenFileSends, newTransferId, sendFilesToMachine, type SendEvent } from '@/api/fileSends'
import { sendImageToMachine } from '@/api/imagePaste'
import { errorMessage } from '@/lib/error-message'
import { remotePathText } from './dropRoute'
import { PLAIN_TOAST, showSendProgress } from './sendToast'

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
  /** A pasted image rather than dropped files: the toasts call it the image. */
  image: boolean
}

type Options = {
  /** Types `text` into the pane of `sessionId`; false when that pane is no longer open. */
  typeInto: (sessionId: string, text: string) => boolean
}

const noun = (count: number) => (count === 1 ? 'file' : 'files')

/**
 * Files dropped on a pane of another machine: starts the send, shows a progress toast with Cancel,
 * and types each path the machine kept the file at into the pane as it lands (ADR 0007). The host
 * does the work off the UI thread and only reports back.
 */
export function useFileSends({ typeInto }: Options) {
  const sends = useRef(new Map<string, Send>())
  const typeRef = useRef(typeInto)
  useEffect(() => { typeRef.current = typeInto })

  const showProgress = useCallback((transfer: string, send: Send, index: number, percent: number | null) => {
    showSendProgress(transfer, send, index, percent)
  }, [])

  /** Shows the progress toast at once and starts the send through `start`, which the host may refuse. */
  const begin = useCallback((sessionId: string, machine: string, what: Pick<Send, 'count' | 'file' | 'image'>, start: (transfer: string) => Promise<void>) => {
    const transfer = newTransferId()
    const send: Send = { sessionId, machine, ...what, untyped: [] }
    sends.current.set(transfer, send)
    showProgress(transfer, send, 0, null)
    start(transfer).catch((error: unknown) => {
      sends.current.delete(transfer)
      toast.error(errorMessage(error, `Could not send to ${machine}`), { ...PLAIN_TOAST, id: transfer })
    })
  }, [showProgress])

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
          toast.success(send.image ? `Sent the image to ${send.machine}` : `Sent ${String(event.count)} ${noun(event.count)} to ${send.machine}`, {
            ...PLAIN_TOAST,
            id: event.transfer,
            description: send.untyped.length > 0 ? `Not typed into the pane: ${send.untyped.join(', ')}` : undefined,
          })
          return
        case 'failed':
          sends.current.delete(event.transfer)
          toast.error(event.unsent > 1 ? `${event.message} (${String(event.unsent - 1)} more not sent)` : event.message, { ...PLAIN_TOAST, id: event.transfer })
          return
        case 'cancelled':
          sends.current.delete(event.transfer)
          toast.message(`Stopped sending to ${send.machine}${event.sent > 0 ? `; ${String(event.sent)} of ${String(send.count)} were sent` : ''}`, { ...PLAIN_TOAST, id: event.transfer })
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
  const sendFiles = useCallback((sessionId: string, machine: string, paths: readonly string[]) => {
    const file = paths.length === 1 ? (paths[0]?.split(/[\\/]/).pop() ?? 'file') : `${String(paths.length)} files`
    begin(sessionId, machine, { count: paths.length, file, image: false }, (transfer) => sendFilesToMachine(machine, paths, transfer))
  }, [begin])

  /** Send a pasted image to `machine` for the pane `sessionId`, as a drop of one file is sent; the host's refusals show as an error toast. */
  const sendImage = useCallback((sessionId: string, machine: string, bytes: Uint8Array) => {
    begin(sessionId, machine, { count: 1, file: 'image', image: true }, (transfer) => sendImageToMachine(machine, bytes, transfer))
  }, [begin])

  return { sendFiles, sendImage }
}
