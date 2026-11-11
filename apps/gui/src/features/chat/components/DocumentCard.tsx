import { toast } from '@/lib/toast'
import type { WrittenDocument } from '../writtenDocument'
import { copyText as writeClipboard } from '@/lib/clipboard'

function copyText(text: string): void {
  writeClipboard(text).then(
    () => { toast.message('Copied') },
    () => { toast.error('Could not copy the document') },
  )
}

type DocumentCardProps = {
  /** The file, relative to the session's folder. */
  path: string
  document: WrittenDocument
  /** Opens the file in the side panel; `null` where there is none. */
  onOpen: (() => void) | null
}

/** A plan or Markdown document the agent wrote, as a card to read: its title and first lines, not a diff. */
export function DocumentCard({ path, document, onOpen }: DocumentCardProps) {
  const name = path.slice(path.lastIndexOf('/') + 1)
  return (
    <div role="group" aria-label={`Write ${path}`} className="mx-step-card mx-doc">
      <div className="mx-doc-file">
        <span className="mx-ftag is-large">MD</span>
        <span className="mx-doc-path">{path} · wrote {document.lineCount} {document.lineCount === 1 ? 'line' : 'lines'}</span>
      </div>
      <div className="mx-doc-text">
        <span className="mx-doc-title">{document.title ?? name}</span>
        {document.excerpt !== '' && <span className="mx-doc-excerpt">{document.excerpt}</span>}
      </div>
      <div className="mx-doc-actions">
        {onOpen !== null && <button type="button" className="mx-soft-btn" onClick={onOpen}>Open</button>}
        <button type="button" className="mx-quiet-btn" onClick={() => { copyText(document.text) }}>Copy</button>
      </div>
    </div>
  )
}
