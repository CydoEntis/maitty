import { useState } from 'react'
import { Check, Circle, CircleDot } from 'lucide-react'
import { HighlightedLine } from '@/components/HighlightedLine'
import { Spinner } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { CommandEvent, EditEvent, TodoEvent, ToolEvent } from '../chatEvents'
import { isOmitted, redactionNote } from '../redaction'
import { elapsedMs, relativePath, todoProgress, tookText } from '../stepInfo'
import { Detail, StatusMark, StatusWord } from './cardParts'
import { diffRows, type DiffRow } from './diffRows'

/**
 * The step cards of the chat (v18 C.1): tool chip, command card, edit block and checklist. A fact
 * the daemon's history does not carry is said to be unavailable, never guessed. Colours are theme
 * tokens, and nothing here animates except the spinner of a step that is running.
 */

export const FOLD_LINES = 12
export const COMMAND_TAIL_LINES = 3
const CHIP_TARGET_CHARS = 90

/** A compact mono chip: what ran and on what; click opens its input and result, bounded. */
export function ToolChip({ event }: { event: ToolEvent }) {
  const [open, setOpen] = useState(false)
  const target = event.target?.slice(0, CHIP_TARGET_CHARS) ?? ''
  return (
    <div>
      <button type="button" aria-expanded={open} className="mx-tool" onClick={() => { setOpen((current) => !current) }}>
        <StatusMark status={event.status} />
        <span className="mx-tool-text">{event.name}{target !== '' && ` ${target}`}</span>
      </button>
      {open && (
        <div className="mx-tool-detail">
          {event.input !== null && (
            <>
              <p className="mt-1.5 mb-0 text-[11px] font-semibold text-dim">Input</p>
              <Detail text={event.input} empty="" />
            </>
          )}
          {event.input !== null && event.detail.trim() !== '' && <p className="mt-1.5 mb-0 text-[11px] font-semibold text-dim">Result</p>}
          <Detail text={event.detail} empty={event.input === null ? 'The conversation history kept no detail for this step.' : 'It returned nothing, or the history did not keep it.'} />
          <RedactionNote texts={[event.input, event.detail]} />
        </div>
      )}
    </div>
  )
}

/** A muted line saying the daemon hid a secret in the text above it; nothing when it hid none. */
function RedactionNote({ texts }: { texts: readonly (string | null)[] }) {
  const note = redactionNote(...texts)
  return note === null ? null : <p className="mt-1 mb-0 text-[11.5px] text-dim">{note}</p>
}

function ExitPill({ event }: { event: CommandEvent }) {
  if (event.exitCode === null) {
    // Claude reports no exit code: say only what is known (it finished), without claiming it passed.
    return event.status === 'failed'
      ? <StatusWord status="failed" />
      : <Check className="size-3.5 text-dim" aria-label="Finished" role="img"><title>Finished. This provider does not report an exit code.</title></Check>
  }
  const ok = event.exitCode === 0
  return (
    <span className={`rounded-full px-2 py-px font-mono text-[11px] ${ok ? 'bg-[color-mix(in_srgb,var(--ok)_14%,transparent)] text-[var(--ok)]' : 'bg-[color-mix(in_srgb,var(--danger)_14%,transparent)] text-[var(--danger)]'}`}>
      exit {event.exitCode}
    </span>
  )
}

function CommandState({ event }: { event: CommandEvent }) {
  if (event.status === 'running') {
    return (
      <span className="flex shrink-0 items-center gap-1.5 rounded-full bg-[color-mix(in_srgb,var(--accent)_14%,transparent)] px-2 py-0.5 text-[11.5px] text-[var(--accent-text)]">
        <Spinner className="mx-spin" />
        <span>Running · <ElapsedSince since={event.timing.startedAtMs ?? event.observed.firstSeenAtMs} /></span>
      </span>
    )
  }
  const took = elapsedMs(event, event.durationMs, 0)
  return (
    <span className="flex shrink-0 items-center gap-2">
      {took !== null && <span className="font-mono text-[11.5px] text-fg2">{tookText(took)}</span>}
      {event.status === 'completed' || event.status === 'failed' ? <ExitPill event={event} /> : <StatusWord status={event.status} />}
    </span>
  )
}

/**
 * A command the agent ran. Closed it is one line, the command's first line and how it ended, as the
 * agent's own terminal shows it; opened it is the whole command and the last three lines of its
 * output (all of it on request). One that failed starts open: that is the one worth reading.
 */
export function CommandCard({ event }: { event: CommandEvent }) {
  const [open, setOpen] = useState(event.status === 'failed')
  const [all, setAll] = useState(false)
  const omitted = isOmitted(event.output)
  const lines = event.output === '' || omitted ? [] : event.output.replace(/\n+$/, '').split('\n')
  const shown = all ? lines : lines.slice(-COMMAND_TAIL_LINES)
  const typed = event.command.split('\n')
  const first = typed[0] ?? ''
  const folded = typed.length > 1 ? `${first} …` : first
  return (
    <div role="group" aria-label={`Command ${first}`} className="mx-step-card">
      <button type="button" className="mx-step-head" aria-expanded={open} title={open ? 'Hide the command and its output' : 'Show the command and its output'} onClick={() => { setOpen((current) => !current) }}>
        <span className="self-start py-[7px] font-mono text-[12px] leading-[18px] text-dim" aria-hidden="true">$</span>
        <code className={`mx-step-command${open ? '' : ' is-folded'}`}>{event.command === '' ? 'A command' : open ? event.command : folded}</code>
        <CommandState event={event} />
      </button>
      {open && <CommandOutput event={event} omitted={omitted} shown={shown} total={lines.length} all={all} onAll={() => { setAll((current) => !current) }} />}
    </div>
  )
}

function CommandOutput({ event, omitted, shown, total, all, onAll }: { event: CommandEvent; omitted: boolean; shown: readonly string[]; total: number; all: boolean; onAll: () => void }) {
  return (
    <>
      {omitted && <p className="mx-step-note">The output shows a secret file, so the history does not keep it.</p>}
      {shown.length > 0 && <pre className="mx-step-output">{shown.join('\n')}</pre>}
      {shown.length > 0 && <RedactionNote texts={[event.output]} />}
      {total > COMMAND_TAIL_LINES && (
        <button type="button" aria-expanded={all} className="mx-step-more" onClick={onAll}>
          {all ? 'Show less' : `Show all ${String(total)} lines`}
        </button>
      )}
    </>
  )
}

const CHANGE_WORD = { add: 'Created', update: 'Edited', delete: 'Deleted', unknown: 'Changed' } as const

type EditCardProps = {
  event: EditEvent
  /** The session's folder, so the file is shown relative to it. */
  root: string | null
  /** Opens this file's diff in the Changes panel; absent where there is no repository. */
  onOpen: ((file: string) => void) | null
}

/** A file change: header with counts, the diff folded past 12 lines. */
export function EditCard({ event, root, onOpen }: EditCardProps) {
  const [all, setAll] = useState(false)
  const file = event.file === null ? null : relativePath(event.file, root)
  const name = file ?? 'a file'
  const rows = event.diff.status === 'available' ? diffRows(event.diff.hunks) : []
  const shown = all ? rows : rows.slice(0, FOLD_LINES)
  const headerBody = (
    <>
      <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="var(--muted)" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" /></svg>
      <span>{CHANGE_WORD[event.change]}</span>
      <code className="mx-step-path" title={name}>{name}</code>
      {event.diff.status === 'available' && event.diff.added > 0 && <span className="mx-step-add">+{event.diff.added}</span>}
      {event.diff.status === 'available' && event.diff.removed > 0 && <span className="mx-step-del">−{event.diff.removed}</span>}
      {event.status !== 'completed' && <StatusWord status={event.status} />}
    </>
  )
  return (
    <div role="group" aria-label={`${CHANGE_WORD[event.change]} ${name}`} className="mx-step-card">
      {onOpen !== null && event.file !== null ? (
        <button type="button" aria-label={`Open the diff of ${name} in the Changes panel`} className="mx-step-head" onClick={() => { if (event.file !== null) onOpen(event.file) }}>
          {headerBody}
        </button>
      ) : (
        <div className="mx-step-head">{headerBody}</div>
      )}
      {event.diff.status === 'unavailable' && <p className="mx-step-note">The conversation history did not keep this change&apos;s content, so its diff is unavailable.</p>}
      {event.diff.status === 'omitted' && <p className="mx-step-note">This looks like a secret file, so the history keeps its name but not its content.</p>}
      {shown.length > 0 && (
        <div className="mx-diff">
          {shown.map((row) => <DiffLine key={row.key} row={row} path={file ?? ''} />)}
          {event.diff.status === 'available' && event.diff.truncated && <p className="mx-step-note">The daemon cut this diff at its size limit.</p>}
        </div>
      )}
      {rows.length > FOLD_LINES && (
        <button type="button" aria-expanded={all} className="mx-step-more" onClick={() => { setAll((current) => !current) }}>
          {all ? 'Show less' : `${String(rows.length - FOLD_LINES)} more lines`}
        </button>
      )}
    </div>
  )
}

const LINE_TONE: Record<DiffRow['tone'], string> = { add: ' is-add', remove: ' is-del', context: '', header: '' }

/** One line of a diff: a hunk header in blue, or the line with its sign and the file's syntax colours. */
function DiffLine({ row, path }: { row: DiffRow; path: string }) {
  if (row.tone === 'header') return <div className="mx-diff-hunk">{row.text}</div>
  return (
    <div className={`mx-diff-line${LINE_TONE[row.tone]}`}>
      <span className="mx-diff-sign" aria-hidden="true">{row.sign}</span>
      <span className="mx-diff-text">{row.text === '' ? ' ' : <HighlightedLine line={row.text} path={path} />}</span>
    </div>
  )
}

/** The agent's own to-do list as the history carried it. Maitty adds no items. */
export function TodoCard({ event }: { event: TodoEvent }) {
  const progress = event.items === null ? null : todoProgress(event.items)
  return (
    <div role="group" aria-label="Checklist" className="mx-step-card px-3 py-2">
      <p className="m-0 mb-1 text-[11.5px] font-semibold text-fg2">
        {progress === null ? 'Checklist' : `Checklist · ${String(progress.done)} of ${String(progress.total)}`}
      </p>
      {progress !== null && progress.total > 0 && (
        <div className="mb-1.5 h-[3px] overflow-hidden rounded-full bg-line2" aria-hidden="true">
          <div className="h-full origin-left bg-[var(--accent)]" style={{ transform: `scaleX(${String(progress.done / progress.total)})` }} />
        </div>
      )}
      {event.items === null
        ? <p className="m-0 text-[12px] text-dim">The conversation history did not keep this checklist&apos;s items.</p>
        : (
          <ul className="m-0 list-none p-0">
            {event.items.map((item, index) => (
              <li key={index} className="flex items-start gap-2 py-0.5 text-[13px]">
                {item.state === 'done' ? <Check className="mt-0.5 size-3.5 shrink-0 text-[var(--ok)]" aria-label="Done" /> : item.state === 'current' ? <CircleDot className="mt-0.5 size-3.5 shrink-0 text-[var(--accent-text)]" aria-label="In progress" /> : <Circle className="mt-0.5 size-3.5 shrink-0 text-dim" aria-label="To do" />}
                <span className={item.state === 'done' ? 'text-dim line-through' : item.state === 'current' ? 'font-semibold text-fg' : 'text-fg2'}>{item.text}</span>
              </li>
            ))}
          </ul>
        )}
    </div>
  )
}
