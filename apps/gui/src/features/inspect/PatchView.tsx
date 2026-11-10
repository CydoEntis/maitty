import { Fragment, useState } from 'react'
import { errorMessage } from '@/lib/error-message'
import { commentMessage, patchLines, type PatchLine } from './patchLines'

/** Lets a line of the diff be commented on: the comment goes to the agent as a message. */
export type PatchComments = {
  /** The file the diff is of, as the agent knows it. */
  path: string
  /** The agent's name, for the button. */
  agent: string
  onSend: (message: string) => Promise<void>
}

type Sent = { index: number; text: string }

function canComment(line: PatchLine): boolean {
  return line.line !== null
}

/** The two line numbers of a diff line, drawn by the stylesheet so that copying the diff copies only its text. */
function Numbers({ line }: { line: PatchLine }) {
  return <i className="mx-patch-no" data-old={line.oldLine ?? ''} data-new={line.newLine ?? ''} aria-hidden="true" />
}

/**
 * One file's unified diff, each line coloured by what it did and numbered on both sides. Git's own
 * header lines are left out: the panel already names the file. With `comments`, a click on a line
 * opens a box to tell the agent about it.
 */
export function PatchView({ patch, comments }: { patch: string; comments?: PatchComments }) {
  const [open, setOpen] = useState<number | null>(null)
  const [draft, setDraft] = useState('')
  const [sending, setSending] = useState(false)
  const [failure, setFailure] = useState<string | null>(null)
  const [sent, setSent] = useState<readonly Sent[]>([])
  const lines = patchLines(patch)
  const firstHunk = lines.findIndex((line) => line.kind === 'hunk')

  const openAt = (index: number) => {
    if (sending) return
    setOpen(index)
    setDraft('')
    setFailure(null)
  }
  const send = (index: number, line: PatchLine) => {
    if (!comments || draft.trim() === '') return
    const text = draft.trim()
    setSending(true)
    setFailure(null)
    comments.onSend(commentMessage(comments.path, line, text))
      .then(() => {
        setSent((current) => [...current, { index, text }])
        setOpen(null)
      })
      .catch((error: unknown) => { setFailure(errorMessage(error, `Could not send the comment to ${comments.agent}`)) })
      .finally(() => { setSending(false) })
  }

  return (
    <pre className="mx-patch is-numbered" aria-label="Diff">
      {lines.map((line, index) => {
        // The first hunk's header only says where the file starts; a later one marks a gap in it.
        if (index <= firstHunk) return null
        if (line.kind === 'hunk') return <span key={index} className="mx-patch-gap" aria-hidden="true" />
        const commentable = comments !== undefined && canComment(line)
        return (
          <Fragment key={index}>
            {commentable ? (
              <span
                className={`mx-patch-line is-${line.kind} is-commentable`}
                data-line={line.line ?? ''}
                role="button"
                tabIndex={0}
                title="Click to comment"
                onClick={() => { openAt(index) }}
                onKeyDown={(event) => { if (event.key === 'Enter' || event.key === ' ') { event.preventDefault(); openAt(index) } }}
              ><Numbers line={line} />{line.text}{'\n'}</span>
            ) : (
              <span className={`mx-patch-line is-${line.kind}`} data-line={line.line ?? ''}><Numbers line={line} />{line.text}{'\n'}</span>
            )}
            {sent.filter((comment) => comment.index === index).map((comment, at) => (
              <span key={at} className="mx-patch-sent"><b>You</b> {comment.text} <i>· sent to {comments?.agent}</i></span>
            ))}
            {comments !== undefined && open === index && (
              <span className="mx-patch-comment">
                <textarea
                  className="mx-patch-comment-text"
                  aria-label={`Comment on line ${String(line.line ?? '')}`}
                  placeholder={`Tell ${comments.agent} about this line`}
                  rows={2}
                  autoFocus
                  value={draft}
                  disabled={sending}
                  onChange={(event) => { setDraft(event.target.value) }}
                  onKeyDown={(event) => {
                    // Esc closes the box, not the whole panel.
                    if (event.key === 'Escape') { event.stopPropagation(); setOpen(null) }
                    if (event.key === 'Enter' && (event.metaKey || event.ctrlKey)) { event.preventDefault(); send(index, line) }
                  }}
                />
                {failure !== null && <span className="mx-patch-comment-error" role="alert">{failure}</span>}
                <span className="mx-patch-comment-actions">
                  <button type="button" className="mx-text-button" disabled={sending} onClick={() => { setOpen(null) }}>Cancel</button>
                  <button type="button" className="mx-text-button is-primary" disabled={sending || draft.trim() === ''} onClick={() => { send(index, line) }}>{sending ? 'Sending…' : `Send to ${comments.agent}`}</button>
                </span>
              </span>
            )}
          </Fragment>
        )
      })}
    </pre>
  )
}
