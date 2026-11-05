import { toast } from 'sonner'
import { listenFileSends, newTransferId, type SendEvent } from '@/api/fileSends'
import { errorMessage } from '@/lib/error-message'
import { typablePath } from '@/features/terminal/dropRoute'
import { PLAIN_TOAST, showSendProgress } from '@/features/terminal/sendToast'

/** A file the other machine kept; `remotePath` was checked here as well as in the host. */
export type Landed = { readonly remotePath: string; readonly file: string; readonly bytes: number }

type Options = {
  machine: string
  /** What the toast calls the file now going, before the host names it. */
  label: string
  /** A pasted image rather than files. */
  image: boolean
  count: number
  /** Starts the send under `transfer`; rejects with the host's words when it refuses. */
  start: (transfer: string) => Promise<void>
  /** Called as each file lands, so a send that stops half way keeps the ones that got there. */
  onLanded: (landed: Landed) => void
}

const noun = (count: number) => (count === 1 ? 'file' : 'files')

/**
 * Sends files for a chat on another machine and tells `onLanded` of each one that arrived (ADR 0053).
 * It is the pane's send (ADR 0050: the host's `send_files_to_machine` / `send_pasted_image`, its caps,
 * its private far-side folder, its progress toast with Cancel) with the chat's ending: the far-side path
 * is not typed into a pane but kept for the message. Resolves when the send has ended, however it ended.
 */
export async function sendAttachments({ machine, label, image, count, start, onLanded }: Options): Promise<void> {
  const transfer = newTransferId()
  const shown = { machine, count, file: label, image }
  let unlisten: (() => void) | undefined
  let end = (): void => undefined
  const ended = new Promise<void>((resolve) => { end = resolve })
  // Untrusted far-side paths that were refused; named once the send ends.
  const refused: string[] = []
  let landed = 0

  const onEvent = (event: SendEvent) => {
    if (event.transfer !== transfer) return
    switch (event.kind) {
      case 'progress':
        shown.file = event.file
        shown.count = event.count
        showSendProgress(transfer, shown, event.index, event.percent)
        return
      case 'sent': {
        shown.file = event.file
        shown.count = event.count
        // The machine is not trusted: only a path fit to name in a message is used.
        const path = typablePath(event.remotePath)
        if (path === null) refused.push(event.file)
        else {
          landed += 1
          onLanded({ remotePath: path, file: event.file, bytes: event.bytes })
        }
        if (event.index + 1 < event.count) showSendProgress(transfer, shown, event.index + 1, null)
        return
      }
      case 'done': {
        const unusable = refused.length > 0 ? `Not attached, the machine named an unusable path: ${refused.join(', ')}` : undefined
        if (landed === 0) toast.error(unusable ?? 'Nothing was attached', { ...PLAIN_TOAST, id: transfer })
        else toast.success(image ? 'Attached the image' : `Attached ${String(landed)} ${noun(landed)}`, { ...PLAIN_TOAST, id: transfer, description: unusable })
        end()
        return
      }
      case 'failed':
        toast.error(event.unsent > 1 ? `${event.message} (${String(event.unsent - 1)} more not sent)` : event.message, { ...PLAIN_TOAST, id: transfer })
        end()
        return
      case 'cancelled':
        toast.message(`Stopped sending to ${machine}${landed > 0 ? `; ${String(landed)} of ${String(shown.count)} were attached` : ''}`, { ...PLAIN_TOAST, id: transfer })
        end()
    }
  }

  try {
    // Listening comes first, so no event of the send is missed.
    unlisten = await listenFileSends(onEvent)
  } catch (error) {
    toast.error(errorMessage(error, 'Could not follow the file send'))
    return
  }
  showSendProgress(transfer, shown, 0, null)
  try {
    await start(transfer)
  } catch (error) {
    unlisten()
    toast.error(errorMessage(error, `Could not send to ${machine}`), { ...PLAIN_TOAST, id: transfer })
    return
  }
  await ended
  unlisten()
}
