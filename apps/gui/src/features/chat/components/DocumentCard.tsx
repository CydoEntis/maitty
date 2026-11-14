import { TypeTag } from '@/components/ui/tag'
import { useState } from 'react'
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
}

/** A plan or Markdown document the agent wrote, as a card to read: its title and first lines, not a diff. */
export function DocumentCard({ path, document }: DocumentCardProps) {
  // Read in place: the file may be outside the project (a Downloads folder), where no panel shows it, and its diff is empty.
  const [open, setOpen] = useState(false)
  const name = path.slice(path.lastIndexOf('/') + 1)
  return (
    <div role="group" aria-label={`Write ${path}`} className="mx-step-card mx-doc">
      <div className="mx-doc-file">
        <TypeTag tag="MD" />
        <span className="mx-doc-path">{path} · wrote {document.lineCount} {document.lineCount === 1 ? 'line' : 'lines'}</span>
      </div>
      <div className="mx-doc-text">
        <span className="mx-doc-title">{document.title ?? name}</span>
        {!open && document.excerpt !== '' && <span className="mx-doc-excerpt">{document.excerpt}</span>}
      </div>
      {open && <pre className="mx-doc-body">{document.text}</pre>}
      <div className="mx-doc-actions">
        <button type="button" className="mx-soft-btn" aria-expanded={open} onClick={() => { setOpen((current) => !current) }}>{open ? 'Close' : 'Read'}</button>
        <button type="button" className="mx-quiet-btn" onClick={() => { copyText(document.text) }}>Copy</button>
      </div>
    </div>
  )
}
