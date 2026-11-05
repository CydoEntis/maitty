import { useState } from 'react'
import { Check, ChevronRight, Circle, CircleDot, Pencil } from 'lucide-react'
import { Spinner } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { CommandEvent, EditEvent, TodoEvent, ToolEvent } from '../chatEvents'
import { isOmitted, redactionNote } from '../redaction'
import { elapsedMs, relativePath, todoProgress, tookText } from '../stepInfo'
import { Detail, StatusMark, StatusWord } from './cardParts'
import { DIFF_TONE, diffRows } from './diffRows'

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
      <button type="button" aria-expanded={open} className="flex h-7 max-w-full items-center gap-2 rounded-lg border border-line2 px-2.5 text-left font-mono text-[12px] text-fg2 hover:bg-hover" onClick={() => { setOpen((current) => !current) }}>
        <StatusMark status={event.status} />
        <span className="shrink-0 text-fg">{event.name}</span>
        {target !== '' && <span className="truncate text-dim">{target}</span>}
        <ChevronRight className={`size-3 shrink-0 text-dim transition-transform ${open ? 'rotate-90' : ''}`} aria-hidden="true" />
      </button>
      {open && (
        <div>
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
    return <span className="text-[11px] text-dim" title="This provider does not report an exit code for the command.">exit code unavailable</span>
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
      <span className="flex shrink-0 items-center gap-1.5 rounded-full bg-[color-mix(in_srgb,var(--accent)_14%,transparent)] px-2 py-0.5 text-[11.5px] text-[var(--accent)]">
        <Spinner className="mx-spin" />
        <span>Running · <ElapsedSince since={event.timing.startedAtMs ?? event.observed.firstSeenAtMs} /></span>
      </span>
    )
  }
  const took = elapsedMs(event, event.durationMs, 0)
  return (
    <span className="flex shrink-0 items-center gap-2">
      {took === null ? <span className="text-[11px] text-dim" title="The provider's saved history keeps no times.">time unavailable</span> : <span className="font-mono text-[11.5px] text-fg2">{tookText(took)}</span>}
      {event.status === 'completed' || event.status === 'failed' ? <ExitPill event={event} /> : <StatusWord status={event.status} />}
    </span>
  )
}

/** The command, its state, and the last three lines of its output (all of it on request). */
export function CommandCard({ event }: { event: CommandEvent }) {
  const [all, setAll] = useState(false)
  const omitted = isOmitted(event.output)
  const lines = event.output === '' || omitted ? [] : event.output.replace(/\n+$/, '').split('\n')
  const shown = all ? lines : lines.slice(-COMMAND_TAIL_LINES)
  return (
    <div role="group" aria-label={`Command ${event.command}`} className="overflow-hidden rounded-[10px] border border-line2 bg-term">
      <div className="flex min-h-9 items-center gap-2.5 px-3 py-1.5">
        <span className="font-mono text-[12.5px] text-dim" aria-hidden="true">$</span>
        <code className="min-w-0 flex-1 font-mono text-[12.5px] text-fg [overflow-wrap:anywhere]">{event.command === '' ? 'A command' : event.command}</code>
        <CommandState event={event} />
      </div>
      {omitted && <p className="m-0 border-t border-line2 px-3 py-2 text-[12px] text-dim">The output shows a secret file, so the history does not keep it.</p>}
      {shown.length > 0 && (
        <div className="border-t border-line2 px-3 py-2">
          <pre className="m-0 max-h-72 overflow-auto font-mono text-[11.5px] leading-[18px] whitespace-pre-wrap text-fg2 [overflow-wrap:anywhere]">{shown.join('\n')}</pre>
          {lines.length > COMMAND_TAIL_LINES && (
            <button type="button" aria-expanded={all} className="mt-1 text-[11.5px] text-dim hover:text-fg" onClick={() => { setAll((current) => !current) }}>
              {all ? 'Show less' : `Show all ${String(lines.length)} lines`}
            </button>
          )}
          <RedactionNote texts={[event.output]} />
        </div>
      )}
    </div>
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
      <Pencil className="size-3.5 shrink-0 text-dim" aria-hidden="true" />
      <span className="text-[12px] text-fg2">{CHANGE_WORD[event.change]}</span>
      <code className="min-w-0 flex-1 truncate text-left font-mono text-[12px] text-fg" title={name}>{name}</code>
      {event.diff.status === 'available' && event.diff.added > 0 && <span className="font-mono text-[11.5px] text-[var(--ok)]">+{event.diff.added}</span>}
      {event.diff.status === 'available' && event.diff.removed > 0 && <span className="font-mono text-[11.5px] text-[var(--danger)]">−{event.diff.removed}</span>}
      {event.status !== 'completed' && <StatusWord status={event.status} />}
    </>
  )
  return (
    <div role="group" aria-label={`${CHANGE_WORD[event.change]} ${name}`} className="overflow-hidden rounded-[10px] border border-line2 bg-term">
      {onOpen !== null && event.file !== null ? (
        <button type="button" aria-label={`Open the diff of ${name} in the Changes panel`} className="flex min-h-[34px] w-full items-center gap-2 px-3 hover:bg-hover" onClick={() => { if (event.file !== null) onOpen(event.file) }}>
          {headerBody}
        </button>
      ) : (
        <div className="flex min-h-[34px] items-center gap-2 px-3">{headerBody}</div>
      )}
      {event.diff.status === 'unavailable' && (
        <p className="m-0 border-t border-line2 px-3 py-2 text-[12px] text-dim">The conversation history did not keep this change&apos;s content, so its diff is unavailable.</p>
      )}
      {event.diff.status === 'omitted' && (
        <p className="m-0 border-t border-line2 px-3 py-2 text-[12px] text-dim">This looks like a secret file, so the history keeps its name but not its content.</p>
      )}
      {shown.length > 0 && (
        <div className="border-t border-line2 py-1 font-mono text-[11.5px] leading-[19px]">
          {shown.map((row) => (
            <div key={row.key} className={`flex ${DIFF_TONE[row.tone]}`}>
              <span className="w-[22px] shrink-0 text-center text-dim select-none" aria-hidden="true">{row.sign}</span>
              <span className="min-w-0 flex-1 pr-3 whitespace-pre-wrap text-fg2 [overflow-wrap:anywhere]">{row.text === '' ? ' ' : row.text}</span>
            </div>
          ))}
          {event.diff.status === 'available' && event.diff.truncated && <p className="m-0 px-3 pt-1 text-[11.5px] text-dim">The daemon cut this diff at its size limit.</p>}
        </div>
      )}
      {rows.length > FOLD_LINES && (
        <button type="button" aria-expanded={all} className="w-full border-t border-line2 px-3 py-1.5 text-left text-[11.5px] text-dim hover:text-fg" onClick={() => { setAll((current) => !current) }}>
          {all ? 'Show less' : `${String(rows.length - FOLD_LINES)} more lines`}
        </button>
      )}
    </div>
  )
}

/** The agent's own to-do list as the history carried it. Maitty adds no items. */
export function TodoCard({ event }: { event: TodoEvent }) {
  const progress = event.items === null ? null : todoProgress(event.items)
  return (
    <div role="group" aria-label="Checklist" className="rounded-[10px] border border-line2 bg-raise px-3 py-2">
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
                {item.state === 'done' ? <Check className="mt-0.5 size-3.5 shrink-0 text-[var(--ok)]" aria-label="Done" /> : item.state === 'current' ? <CircleDot className="mt-0.5 size-3.5 shrink-0 text-[var(--accent)]" aria-label="In progress" /> : <Circle className="mt-0.5 size-3.5 shrink-0 text-dim" aria-label="To do" />}
                <span className={item.state === 'done' ? 'text-dim line-through' : item.state === 'current' ? 'font-semibold text-fg' : 'text-fg2'}>{item.text}</span>
              </li>
            ))}
          </ul>
        )}
    </div>
  )
}
