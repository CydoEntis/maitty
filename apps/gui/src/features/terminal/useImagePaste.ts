import { useCallback, useEffect, useRef } from 'react'
import { toast } from '@/lib/toast'
import { machineNameOf } from '@/api/machines'
import { oversizeMessage, saveImageLocally } from '@/api/imagePaste'
import { errorMessage } from '@/lib/error-message'
import { formatDroppedPaths } from './droppedPaths'

type Options = {
  /** Sends the bytes to the machine for the pane (`useFileSends`); progress, Cancel and the typed path follow from its toast. */
  sendImage: (sessionId: string, machine: string, bytes: Uint8Array) => void
  /** Types `text` into the pane of `sessionId`; false when that pane is no longer open. */
  typeInto: (sessionId: string, text: string) => boolean
  /** Whether the machine with this name is `connected`. */
  isConnected: (machine: string) => boolean
  /** The Maitty service is reachable: typing into a pane of this computer needs it. */
  online: boolean
  /** The path is written the way this computer's shell reads it. */
  windows: boolean
}

/** The id of the one toast a paste on this computer uses, so a second paste replaces it. */
const LOCAL_TOAST = 'image-paste'

/**
 * What a paste of an image into a pane does (ticket 19). On a pane of this computer the host keeps
 * the image in its private cache and its path is typed into the pane, as the terminal app does. On a
 * pane of another machine the image is sent to it as a file is sent, over that machine's ssh line,
 * and the path it is kept at there is typed. Returns the function the pane calls with the clipboard's
 * image; every refusal is one error toast and nothing is sent.
 */
export function useImagePaste(options: Options): (sessionId: string, image: Blob) => void {
  const latest = useRef(options)
  useEffect(() => { latest.current = options })

  return useCallback((sessionId: string, image: Blob) => {
    const { sendImage, isConnected, online, windows } = latest.current
    const tooBig = oversizeMessage(image.size)
    if (tooBig !== null) {
      toast.error(tooBig)
      return
    }
    const machine = machineNameOf(sessionId)
    if (machine === null) {
      if (!online) {
        toast.error('Reconnect to the Maitty service before pasting an image')
        return
      }
      toast.loading('Saving the image…', { id: LOCAL_TOAST })
      readBytes(image)
        .then(saveImageLocally)
        .then((path) => {
          if (!latest.current.typeInto(sessionId, formatDroppedPaths([path], windows))) {
            toast.error('The pane is closed; the image was not pasted', { id: LOCAL_TOAST })
            return
          }
          toast.success('Pasted the image', { id: LOCAL_TOAST, duration: 2000 })
        })
        .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not paste the image'), { id: LOCAL_TOAST }) })
      return
    }
    if (!isConnected(machine)) {
      toast.error(`${machine} is not connected; reconnect before sending an image to it`)
      return
    }
    readBytes(image)
      .then((bytes) => { sendImage(sessionId, machine, bytes) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not read the image')) })
  }, [])
}

async function readBytes(image: Blob): Promise<Uint8Array> {
  return new Uint8Array(await image.arrayBuffer())
}
