import { useCallback, useEffect, useRef } from 'react'
import { toast } from 'sonner'
import { attachLocalFile, chooseFilesToAttach, MOST_ATTACHMENTS } from '@/api/attachments'
import { oversizeMessage, saveImageLocally } from '@/api/imagePaste'
import { errorMessage } from '@/lib/error-message'
import { peekComposer, updateComposer, type DraftAttachment } from './composerStore'

/** What the composer can do with files; every refusal is one error toast and nothing is added. */
export type AttachmentIntake = {
  /** Opens the file picker. */
  pick: () => void
  /** Keeps the files at these paths (a drop, the picker) for the message. */
  addPaths: (paths: readonly string[]) => void
  /** Keeps a pasted image for the message and shows its thumbnail. */
  addImage: (image: Blob) => void
  remove: (id: string) => void
}

type Input = {
  conversation: number
  /** Why nothing can be attached now (offline, an approval waits, read-only), or null. */
  blocked: string | null
  /**
   * The machine the agent runs on, or null for this computer. Only this computer is built (ADR 0053):
   * the other-machine form would send the file with `send_files_to_machine` / `send_pasted_image`
   * and name the far-side path in the message.
   */
  machine?: string | null
}

const baseName = (path: string) => path.split(/[\\/]/).pop() ?? path

const noun = (count: number) => (count === 1 ? 'file' : 'files')

function newId(): string {
  return `attachment-${crypto.randomUUID()}`
}

export function revokePreviews(attachments: readonly DraftAttachment[]): void {
  for (const attachment of attachments) if (attachment.preview !== null) URL.revokeObjectURL(attachment.preview)
}

function previewOf(image: Blob): string | null {
  return typeof URL.createObjectURL === 'function' ? URL.createObjectURL(image) : null
}

/**
 * Brings files into the composer's draft. Copies and checks happen on the host's worker, never on
 * the UI thread; a send in flight is not affected. A file is added only when it fits: at most
 * `MOST_ATTACHMENTS` per message, and a drop that would pass that is refused whole, never cut.
 */
export function useAttachmentIntake({ conversation, blocked, machine = null }: Input): AttachmentIntake {
  const latest = useRef({ conversation, blocked, machine })
  useEffect(() => { latest.current = { conversation, blocked, machine } })

  /** The words for why nothing is added now, or null when files can be added. */
  const refusal = useCallback((incoming: number): string | null => {
    const { conversation: id, blocked: reason, machine: where } = latest.current
    if (reason !== null) return reason
    if (where !== null) return `Attaching files to a chat on ${where} is not available yet`
    const have = peekComposer(id).attachments.length
    if (have + incoming > MOST_ATTACHMENTS) {
      return have === 0
        ? `At most ${String(MOST_ATTACHMENTS)} files can be attached to a message; ${String(incoming)} were chosen`
        : `A message holds at most ${String(MOST_ATTACHMENTS)} files; ${String(have)} already attached`
    }
    return null
  }, [])

  const append = useCallback((item: DraftAttachment) => {
    updateComposer(latest.current.conversation, (state) => ({ ...state, attachments: [...state.attachments, item] }))
  }, [])

  const addPaths = useCallback((paths: readonly string[]) => {
    const chosen = paths.filter((path) => path !== '')
    if (chosen.length === 0) return
    const refused = refusal(chosen.length)
    if (refused !== null) {
      toast.error(refused)
      return
    }
    const target = latest.current.conversation
    const toastId = `attach-${String(target)}`
    toast.loading(`Adding ${chosen.length === 1 ? baseName(chosen[0] ?? '') : `${String(chosen.length)} ${noun(chosen.length)}`}…`, { id: toastId })
    void (async () => {
      let failed = 0
      for (const path of chosen) {
        try {
          const kept = await attachLocalFile(path)
          // The message may have been filled by another add while this file was copied.
          if (peekComposer(target).attachments.length >= MOST_ATTACHMENTS) {
            failed += 1
            toast.error(`A message holds at most ${String(MOST_ATTACHMENTS)} files`)
            break
          }
          updateComposer(target, (state) => ({ ...state, attachments: [...state.attachments, { id: newId(), ...kept, preview: null }] }))
        } catch (error) {
          failed += 1
          toast.error(errorMessage(error, `Could not attach ${baseName(path)}`))
        }
      }
      if (failed === chosen.length) toast.dismiss(toastId)
      else toast.success(`Attached ${String(chosen.length - failed)} ${noun(chosen.length - failed)}`, { id: toastId, duration: 2000 })
    })()
  }, [refusal])

  const addImage = useCallback((image: Blob) => {
    const refused = refusal(1)
    if (refused !== null) {
      toast.error(refused)
      return
    }
    const tooBig = oversizeMessage(image.size)
    if (tooBig !== null) {
      toast.error(tooBig)
      return
    }
    const toastId = `attach-${String(latest.current.conversation)}`
    toast.loading('Adding the image…', { id: toastId })
    image.arrayBuffer()
      .then((buffer) => saveImageLocally(new Uint8Array(buffer)))
      .then((path) => {
        append({ id: newId(), path, name: 'Pasted image', size: image.size, kind: 'image', preview: previewOf(image) })
        toast.success('Attached the image', { id: toastId, duration: 2000 })
      })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not attach the image'), { id: toastId }) })
  }, [refusal, append])

  const pick = useCallback(() => {
    const refused = refusal(1)
    if (refused !== null) {
      toast.error(refused)
      return
    }
    chooseFilesToAttach()
      .then((paths) => { if (paths !== null) addPaths(paths) })
      .catch((error: unknown) => { toast.error(errorMessage(error, 'Could not open the file picker')) })
  }, [refusal, addPaths])

  const remove = useCallback((id: string) => {
    const gone = peekComposer(latest.current.conversation).attachments.find((attachment) => attachment.id === id)
    if (gone === undefined) return
    revokePreviews([gone])
    updateComposer(latest.current.conversation, (state) => ({ ...state, attachments: state.attachments.filter((attachment) => attachment.id !== id) }))
  }, [])

  return { pick, addPaths, addImage, remove }
}
