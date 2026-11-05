import { useEffect, useState } from 'react'
import { File as FileIcon, FileArchive, Image as ImageIcon, X } from 'lucide-react'
import { attachmentsExist, type AttachmentKind } from '@/api/attachments'
import type { SentAttachment } from '../attachmentMessage'
import type { DraftAttachment } from '../composerStore'

const KIND_ICON = { image: ImageIcon, archive: FileArchive, file: FileIcon } as const

const BYTES_PER_KB = 1024
const KB_PER_MB = 1024

/** A size in words a person reads: "812 KB", "3.4 MB". */
function sizeText(bytes: number): string {
  if (bytes < BYTES_PER_KB) return `${String(bytes)} B`
  const kb = bytes / BYTES_PER_KB
  if (kb < KB_PER_MB) return `${String(Math.max(1, Math.round(kb)))} KB`
  return `${(kb / KB_PER_MB).toFixed(1)} MB`
}

const CHIP = 'flex min-w-0 items-center gap-2 rounded-lg border border-line2 bg-term px-2 py-1 text-[12px] text-fg2'

function Icon({ kind }: { kind: AttachmentKind }) {
  const Glyph = KIND_ICON[kind]
  return <Glyph className="size-4 shrink-0 text-dim" aria-hidden="true" />
}

/** The files attached to the draft, each with a thumbnail when the window holds the picture and a remove button. */
export function DraftAttachments({ attachments, onRemove }: { attachments: readonly DraftAttachment[]; onRemove: (id: string) => void }) {
  if (attachments.length === 0) return null
  return (
    <ul className="m-0 flex list-none flex-wrap gap-1.5 p-0" aria-label="Attached files">
      {attachments.map((attachment) => (
        <li key={attachment.id} className={`${CHIP} max-w-[240px]`}>
          {attachment.preview === null ? (
            <Icon kind={attachment.kind} />
          ) : (
            <img src={attachment.preview} alt="" className="size-8 shrink-0 rounded object-cover" />
          )}
          <span className="flex min-w-0 flex-col leading-tight">
            <span className="truncate text-fg" title={attachment.name}>{attachment.name}</span>
            <span className="text-[11px] text-dim">{sizeText(attachment.size)}</span>
          </span>
          <button
            type="button"
            aria-label={`Remove ${attachment.name}`}
            title={`Remove ${attachment.name}`}
            className="ml-auto inline-flex size-5 shrink-0 items-center justify-center rounded text-dim hover:text-fg focus-visible:outline-2 focus-visible:outline-[var(--accent-line)]"
            onClick={() => { onRemove(attachment.id) }}
          >
            <X className="size-3.5" aria-hidden="true" />
          </button>
        </li>
      ))}
    </ul>
  )
}

/** Whether each kept file is still there; unknown (true) until the host answers, so a chip does not flash "gone". */
function useStillThere(paths: readonly string[]): readonly boolean[] {
  const key = paths.join('\n')
  const [there, setThere] = useState<{ key: string; flags: readonly boolean[] } | null>(null)
  useEffect(() => {
    let current = true
    attachmentsExist(key.split('\n'))
      .then((flags) => { if (current) setThere({ key, flags }) })
      // Not knowing is not the file being gone: the chips stay as they are.
      .catch((error: unknown) => { console.warn('Could not check the attached files', error) })
    return () => { current = false }
  }, [key])
  return there?.key === key ? there.flags : paths.map(() => true)
}

/** The files a sent message carried, from its stored paths; a file that was cleared reads "no longer available". */
export function SentAttachments({ files }: { files: readonly SentAttachment[] }) {
  const there = useStillThere(files.map((file) => file.path))
  return (
    <ul className="m-0 mt-1.5 flex list-none flex-wrap justify-end gap-1.5 p-0" aria-label="Attached files">
      {files.map((file, index) => (
        <li key={`${file.path}-${String(index)}`} className={`${CHIP} max-w-[240px]`}>
          <Icon kind={file.kind} />
          <span className="truncate text-fg" title={file.name}>{file.name}</span>
          {there[index] === false && <span className="shrink-0 text-[11px] text-dim">no longer available</span>}
        </li>
      ))}
    </ul>
  )
}
