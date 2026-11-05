import { useCallback, useEffect, useMemo, useState, type KeyboardEvent } from 'react'
import { ArrowLeft, Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import type { ConversationDiff } from '@/api/daemon'
import type { SentComment } from './commentStore'
import { setCommentComposerOpen } from './commentStore'
import { parsePatch, type PatchLine } from './patchLines'

type DiffFile = ConversationDiff['files'][number]

/** A very long diff is drawn in steps, so opening a generated file never builds thousands of rows at once. */
const ROWS_PER_STEP = 1500

const LINE_TONE: Record<PatchLine['kind'], string> = {
  add: 'bg-[color-mix(in_srgb,var(--ok)_10%,transparent)]',
  remove: 'bg-[color-mix(in_srgb,var(--danger)_10%,transparent)]',
  context: '',
  hunk: 'text-dim',
}

type Props = {
  file: DiffFile
  agent: string
  /** Comments already written on this file. */
  comments: readonly SentComment[]
  /** Why a comment would wait rather than go at once, or `null` when it goes now. */
  deliveryNote: string | null
  onBack: () => void
  onComment: (comment: { line: number; code: string; text: string }) => void
}

/** The comments on one line, for its marker and its tooltip. */
function commentsAt(comments: readonly SentComment[], line: number): readonly SentComment[] {
  return comments.filter((comment) => comment.line === line)
}

function CommentComposer({ line, agent, note, onSend, onCancel }: { line: number; agent: string; note: string | null; onSend: (text: string) => void; onCancel: () => void }) {
  const [text, setText] = useState('')
  const send = () => {
    if (text.trim() !== '') onSend(text.trim())
  }
  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.nativeEvent.isComposing) return
    // Enter sends and so does Ctrl/Cmd+Enter; Shift+Enter adds a line (v18 C.2, C.9).
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      send()
    } else if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      onCancel()
    }
  }
  return (
    <div className="mx-2 my-1.5 rounded-lg border border-[var(--accent-line)] bg-raise p-2 ring-2 ring-[var(--accent-soft)]">
      <textarea
        autoFocus
        rows={2}
        aria-label={`Comment on line ${String(line)}`}
        placeholder={`Tell ${agent} what to change here…`}
        value={text}
        className="block max-h-40 min-h-[40px] w-full resize-none bg-transparent text-[12.5px] leading-[1.5] text-fg outline-none [field-sizing:content] placeholder:text-dim"
        onChange={(event) => { setText(event.target.value) }}
        onKeyDown={onKeyDown}
      />
      {note !== null && <p className="m-0 mt-1 text-[11.5px] text-dim">{note}</p>}
      <div className="mt-1.5 flex justify-end gap-1.5">
        <Button type="button" size="sm" variant="ghost" onClick={onCancel}>Cancel</Button>
        <Button type="button" size="sm" className="bg-brand text-[var(--on-accent)] hover:bg-brand/85" disabled={text.trim() === ''} onClick={send}>Send to {agent}</Button>
      </div>
    </div>
  )
}

/**
 * One file's diff with a comment composer per line (v18 C.2). Hovering a line shows a + button; it
 * opens a composer right under the line. A comment goes to the agent as a message; the line keeps a
 * count marker whose tooltip lists what was written. Removed lines have no number in the file as it
 * is now, so they take no comment.
 */
export function DiffView({ file, agent, comments, deliveryNote, onBack, onComment }: Props) {
  const lines = useMemo(() => parsePatch(file.patch), [file.patch])
  const [composing, setComposing] = useState<number | null>(null)
  const [limit, setLimit] = useState(ROWS_PER_STEP)
  const cancel = useCallback(() => { setComposing(null) }, [])

  // Esc is the top layer of the window's ladder while a composer is open, so it closes before anything else.
  useEffect(() => {
    setCommentComposerOpen(composing !== null, cancel)
    return () => { setCommentComposerOpen(false, null) }
  }, [composing, cancel])

  const shown = lines.slice(0, limit)
  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex shrink-0 items-center gap-2 border-b border-line2 px-2 py-1.5">
        <Button type="button" variant="ghost" size="icon" className="size-7 shrink-0 text-dim hover:text-fg" aria-label="Back to changes" onClick={onBack}>
          <ArrowLeft className="size-4" aria-hidden="true" />
        </Button>
        {/* Truncated from the start, so the file name stays visible. */}
        <span className="min-w-0 flex-1 truncate text-left font-mono text-[12px] text-fg" style={{ direction: 'rtl' }} title={file.path}><bdi>{file.path}</bdi></span>
        <span className="shrink-0 font-mono text-[11.5px] text-[var(--ok)]">+{file.additions}</span>
        <span className="shrink-0 font-mono text-[11.5px] text-[var(--danger)]">−{file.deletions}</span>
      </div>
      <p className="m-0 shrink-0 px-3 py-1.5 text-[11.5px] text-dim">Hover a line and click + to leave a comment for {agent}.</p>
      {file.binary ? (
        <p className="m-0 px-3 py-2 text-[12px] text-dim">Binary file; there is no diff to show.</p>
      ) : lines.length === 0 ? (
        <p className="m-0 px-3 py-2 text-[12px] text-dim">There is no text diff for this file.</p>
      ) : (
        <div className="min-h-0 flex-1 overflow-auto pb-2 font-mono text-[11.5px] leading-[19px]" role="region" aria-label={`Diff of ${file.path}`}>
          {shown.map((line) => {
            if (line.kind === 'hunk') return <div key={line.key} className={`px-3 ${LINE_TONE.hunk}`}>{line.text}</div>
            const newNo = line.newNo
            const here = newNo === null ? [] : commentsAt(comments, newNo)
            const composerLine = newNo !== null && composing === newNo ? newNo : null
            const active = composerLine !== null
            return (
              <div key={line.key}>
                <div className={`group relative flex ${active ? 'bg-[var(--accent-soft)]' : LINE_TONE[line.kind]}`}>
                  {newNo !== null && (
                    <button
                      type="button"
                      aria-label={`Comment on line ${String(newNo)}`}
                      className="absolute top-[1.5px] left-0.5 z-10 flex size-4 items-center justify-center rounded bg-brand text-[var(--on-accent)] opacity-0 group-hover:opacity-100 focus-visible:opacity-100"
                      onClick={() => { setComposing(newNo) }}
                    >
                      <Plus className="size-3" aria-hidden="true" />
                    </button>
                  )}
                  <span className="w-[30px] shrink-0 pr-1.5 text-right text-dim select-none" aria-hidden="true">{newNo ?? ''}</span>
                  <span className="w-[14px] shrink-0 text-center text-dim select-none" aria-hidden="true">{line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ''}</span>
                  <span className="min-w-0 flex-1 pr-2 whitespace-pre-wrap text-fg2 [overflow-wrap:anywhere]">{line.text === '' ? ' ' : line.text}</span>
                  {here.length > 0 && (
                    <span className="mr-2 shrink-0 self-center rounded-full bg-brand px-1.5 text-[10.5px] leading-4 text-[var(--on-accent)]" role="img" aria-label={`${String(here.length)} comment${here.length === 1 ? '' : 's'}: ${here.map((comment) => comment.text).join('; ')}`} title={here.map((comment) => comment.text).join('\n')}>
                      {here.length}
                    </span>
                  )}
                </div>
                {composerLine !== null && (
                  <CommentComposer
                    line={composerLine}
                    agent={agent}
                    note={deliveryNote}
                    onCancel={cancel}
                    onSend={(text) => {
                      onComment({ line: composerLine, code: line.text, text })
                      setComposing(null)
                    }}
                  />
                )}
              </div>
            )
          })}
          {lines.length > limit && (
            <button type="button" className="w-full px-3 py-1.5 text-left text-[11.5px] text-dim hover:text-fg" onClick={() => { setLimit((current) => current + ROWS_PER_STEP) }}>
              Show {Math.min(ROWS_PER_STEP, lines.length - limit)} more of {lines.length - limit} lines
            </button>
          )}
        </div>
      )}
    </div>
  )
}
