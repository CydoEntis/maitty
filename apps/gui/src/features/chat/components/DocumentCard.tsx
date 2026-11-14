import { useState } from 'react'
import { FileText } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardBody, CardHead } from '@/components/ui/card'
import { copyText as writeClipboard } from '@/lib/clipboard'
import { toast } from '@/lib/toast'
import type { WrittenDocument } from '../writtenDocument'
import { FileBadge } from './cardParts'
import { Markdown } from './Markdown'

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
    <Card role="group" aria-label={`Write ${path}`} className="mx-doc">
      <CardHead>
        <FileText className="size-3.5 shrink-0 text-(--muted)" aria-hidden="true" />
        <FileBadge path={path} />
        <span className="mx-doc-lines">· wrote {document.lineCount} {document.lineCount === 1 ? 'line' : 'lines'}</span>
        <span className="mx-spacer" />
        <Button variant="soft" size="sm" aria-expanded={open} onClick={() => { setOpen((current) => !current) }}>{open ? 'Close' : 'Read'}</Button>
        <Button variant="ghost" size="sm" onClick={() => { copyText(document.text) }}>Copy</Button>
      </CardHead>
      <CardBody>
        <span className="mx-doc-title">{document.title ?? name}</span>
        {open ? <Markdown className="mx-doc-body" text={document.text} /> : document.excerpt !== '' && <Markdown className="mx-doc-excerpt" text={document.excerpt} />}
      </CardBody>
    </Card>
  )
}
