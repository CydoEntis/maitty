import { memo, useState, type ReactNode } from 'react'
import { Check, ChevronRight, Circle, CircleDot, Pencil, SquareTerminal, Users } from 'lucide-react'
import { AppIcon, Spinner } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import type { CommandEvent, DiffHunk, EditEvent, ItemStatus, SubagentEvent, TodoEvent, ToolEvent } from '../chatEvents'
import { dividerText, splitLead, type ChatRow } from '../chatRows'
import { ApprovalCard, type ApprovalActions } from './ApprovalCard'

/**
 * The rows of the chat (v18 C.1). Cards here are the plain, complete forms: every fact the daemon's
 * history carries is shown, and one it does not carry is said to be unavailable, never guessed.
 * Colours are theme tokens; nothing here animates, so a long conversation costs nothing at rest.
 */

const COLUMN = 'mx-auto w-full max-w-[760px] px-5'
const FOLD_LINES = 12
const COMMAND_TAIL_LINES = 3
const CHIP_PREVIEW_CHARS = 90

const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })

function Frame({ children, indent = true }: { children: ReactNode; indent?: boolean }) {
  return <div className={`${COLUMN} py-[7px]`}><div className={indent ? 'ml-9' : ''}>{children}</div></div>
}

const STATUS_WORD: Record<ItemStatus, string> = { running: 'Running', completed: 'Done', failed: 'Failed', interrupted: 'Interrupted' }

function StatusMark({ status }: { status: ItemStatus }) {
  if (status === 'running') return <Spinner className="mx-spin text-[var(--accent)]" />
  if (status === 'completed') return <Check className="size-3.5 text-[var(--ok)]" aria-hidden="true" />
  return <SquareTerminal className={`size-3.5 ${status === 'failed' ? 'text-[var(--danger)]' : 'text-dim'}`} aria-hidden="true" />
}

function StatusWord({ status }: { status: ItemStatus }) {
  return <span className={`text-[11px] ${status === 'failed' ? 'text-[var(--danger)]' : 'text-dim'}`}>{STATUS_WORD[status]}</span>
}

type RowProps = { row: ChatRow; agent: ChatAgent; workingSince: number | null; approval: ApprovalActions | null }

/** One row of the list. Memoised: a streamed update re-renders only the rows whose events changed. */
export const ChatItem = memo(function ChatItem({ row, agent, workingSince, approval }: RowProps) {
  switch (row.kind) {
    case 'user':
      return (
        <div className={`${COLUMN} flex justify-end py-[7px]`}>
          <p className="m-0 max-w-[85%] rounded-[16px_16px_5px_16px] border border-line2 bg-raise px-3.5 py-2 text-[14px] leading-[1.55] whitespace-pre-wrap text-fg [overflow-wrap:anywhere]">{row.event.text}</p>
        </div>
      )
    case 'agent': {
      const { lead, rest } = splitLead(row.paragraphs)
      return (
        <div className={`${COLUMN} flex gap-3 py-[7px]`}>
          <span className="mt-0.5 shrink-0"><AppIcon process={agent} size={24} /></span>
          <div className="min-w-0 flex-1 text-[14px] leading-[1.6] [overflow-wrap:anywhere]">
            {lead !== '' && <p className="m-0 whitespace-pre-wrap text-fg">{lead}</p>}
            {rest.map((paragraph, index) => <p key={index} className="mt-2 mb-0 whitespace-pre-wrap text-fg2">{paragraph}</p>)}
            {lead === '' && <p className="m-0 text-dim">{row.status === 'running' ? 'Writing…' : 'An empty reply.'}</p>}
          </div>
        </div>
      )
    }
    case 'step':
      return <Frame><Step event={row.event} /></Frame>
    case 'approval':
      return <Frame><ApprovalCard event={row.event} actions={approval} /></Frame>
    case 'divider':
      return (
        <div className={`${COLUMN} flex items-center gap-3 py-3`} role="separator" aria-label={dividerText(row.event, clock)}>
          <span className="h-px flex-1 bg-line2" />
          <span className="text-[11.5px] text-dim">{dividerText(row.event, clock)}</span>
          <span className="h-px flex-1 bg-line2" />
        </div>
      )
    case 'working':
      return (
        <div className={`${COLUMN} flex items-center gap-2 py-3 text-[12.5px] text-fg2`}>
          <Spinner className="mx-spin text-[var(--accent)]" />
          <span>{agent} is working{workingSince !== null && <> · <ElapsedSince since={workingSince} /></>}</span>
        </div>
      )
  }
})

function Step({ event }: { event: Extract<ChatRow, { kind: 'step' }>['event'] }) {
  switch (event.kind) {
    case 'edit':
      return <EditCard event={event} />
    case 'command':
      return <CommandCard event={event} />
    case 'todo':
      return <TodoCard event={event} />
    case 'subagent':
      return <SubagentCard event={event} />
    case 'tool':
      return <ToolChip event={event} />
  }
}

function ToolChip({ event }: { event: ToolEvent }) {
  const [open, setOpen] = useState(false)
  const preview = event.detail.trim().split('\n')[0]?.slice(0, CHIP_PREVIEW_CHARS) ?? ''
  return (
    <div>
      <button type="button" aria-expanded={open} className="flex max-w-full items-center gap-2 rounded-lg border border-line2 px-2.5 py-1 text-left font-mono text-[12px] text-fg2 hover:bg-hover" onClick={() => { setOpen((current) => !current) }}>
        <StatusMark status={event.status} />
        <span className="shrink-0 text-fg">{event.name}</span>
        {preview !== '' && !open && <span className="truncate text-dim">{preview}</span>}
        <ChevronRight className={`size-3 shrink-0 text-dim transition-transform ${open ? 'rotate-90' : ''}`} aria-hidden="true" />
      </button>
      {open && <Detail text={event.detail} empty="The conversation history kept no detail for this step." />}
    </div>
  )
}

function Detail({ text, empty }: { text: string; empty: string }) {
  return text.trim() === ''
    ? <p className="mt-1.5 mb-0 text-[12px] text-dim">{empty}</p>
    : <pre className="mt-1.5 mb-0 max-h-48 overflow-auto rounded-lg border border-line2 bg-term p-2.5 font-mono text-[11.5px] leading-[1.55] whitespace-pre-wrap text-fg2 [overflow-wrap:anywhere]">{text}</pre>
}

function CommandCard({ event }: { event: CommandEvent }) {
  const [all, setAll] = useState(false)
  const lines = event.output === '' ? [] : event.output.replace(/\n+$/, '').split('\n')
  const shown = all ? lines : lines.slice(-COMMAND_TAIL_LINES)
  return (
    <div role="group" aria-label={`Command ${event.command}`} className="overflow-hidden rounded-[10px] border border-line2 bg-term">
      <div className="flex min-h-9 items-center gap-2.5 px-3 py-1.5">
        <span className="font-mono text-[12.5px] text-dim" aria-hidden="true">$</span>
        <code className="min-w-0 flex-1 font-mono text-[12.5px] text-fg [overflow-wrap:anywhere]">{event.command === '' ? 'A command' : event.command}</code>
        <span className="flex shrink-0 items-center gap-1.5"><StatusMark status={event.status} /><StatusWord status={event.status} /></span>
      </div>
      {shown.length > 0 && (
        <div className="border-t border-line2 px-3 py-2">
          <pre className="m-0 max-h-72 overflow-auto font-mono text-[11.5px] leading-[1.55] whitespace-pre-wrap text-fg2 [overflow-wrap:anywhere]">{shown.join('\n')}</pre>
          {lines.length > COMMAND_TAIL_LINES && (
            <button type="button" className="mt-1 text-[11.5px] text-dim hover:text-fg" onClick={() => { setAll((current) => !current) }}>
              {all ? 'Show less' : `Show all ${String(lines.length)} lines`}
            </button>
          )}
        </div>
      )}
    </div>
  )
}

type DiffRow = { key: string; sign: string; text: string; tone: 'add' | 'remove' | 'context' | 'header' }

function diffRows(hunks: readonly DiffHunk[]): DiffRow[] {
  return hunks.flatMap((hunk, hunkIndex) => [
    ...(hunk.header === '' ? [] : [{ key: `${String(hunkIndex)}h`, sign: '', text: hunk.header, tone: 'header' as const }]),
    ...hunk.lines.map((line, lineIndex): DiffRow => ({ key: `${String(hunkIndex)}:${String(lineIndex)}`, sign: line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : ' ', text: line.text, tone: line.kind })),
  ])
}

const DIFF_TONE: Record<DiffRow['tone'], string> = {
  add: 'bg-[color-mix(in_srgb,var(--ok)_10%,transparent)]',
  remove: 'bg-[color-mix(in_srgb,var(--danger)_10%,transparent)]',
  context: '',
  header: 'text-dim',
}

const CHANGE_WORD = { add: 'Created', update: 'Edited', delete: 'Deleted', unknown: 'Changed' } as const

export function EditCard({ event }: { event: EditEvent }) {
  const [all, setAll] = useState(false)
  const file = event.file ?? 'a file'
  const rows = event.diff.status === 'available' ? diffRows(event.diff.hunks) : []
  const shown = all ? rows : rows.slice(0, FOLD_LINES)
  return (
    <div role="group" aria-label={`${CHANGE_WORD[event.change]} ${file}`} className="overflow-hidden rounded-[10px] border border-line2 bg-term">
      <div className="flex min-h-[34px] items-center gap-2 px-3">
        <Pencil className="size-3.5 shrink-0 text-dim" aria-hidden="true" />
        <span className="text-[12px] text-fg2">{CHANGE_WORD[event.change]}</span>
        <code className="min-w-0 flex-1 truncate font-mono text-[12px] text-fg" title={file}>{file}</code>
        {event.diff.status === 'available' && event.diff.added > 0 && <span className="font-mono text-[11.5px] text-[var(--ok)]">+{event.diff.added}</span>}
        {event.diff.status === 'available' && event.diff.removed > 0 && <span className="font-mono text-[11.5px] text-[var(--danger)]">−{event.diff.removed}</span>}
        {event.status !== 'completed' && <StatusWord status={event.status} />}
      </div>
      {event.diff.status === 'unavailable' && (
        <p className="m-0 border-t border-line2 px-3 py-2 text-[12px] text-dim">The conversation history did not keep this change&apos;s content, so its diff is unavailable.</p>
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
        <button type="button" className="w-full border-t border-line2 px-3 py-1.5 text-left text-[11.5px] text-dim hover:text-fg" onClick={() => { setAll((current) => !current) }}>
          {all ? 'Show less' : `${String(rows.length - FOLD_LINES)} more lines`}
        </button>
      )}
    </div>
  )
}

function TodoCard({ event }: { event: TodoEvent }) {
  return (
    <div role="group" aria-label="Checklist" className="rounded-[10px] border border-line2 bg-raise px-3 py-2">
      <p className="m-0 mb-1 text-[11.5px] font-semibold text-fg2">
        {event.items === null ? 'Checklist' : `Checklist · ${String(event.items.filter((item) => item.state === 'done').length)} of ${String(event.items.length)}`}
      </p>
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

function SubagentCard({ event }: { event: SubagentEvent }) {
  const [open, setOpen] = useState(false)
  return (
    <div role="group" aria-label="Helper agent" className="rounded-[10px] border border-line2 bg-raise px-3 py-2">
      <div className="flex items-center gap-2">
        <Users className="size-3.5 shrink-0 text-dim" aria-hidden="true" />
        <span className="min-w-0 flex-1 text-[13px] text-fg [overflow-wrap:anywhere]">{event.title ?? 'A helper agent'}</span>
        {event.agentType !== null && <span className="font-mono text-[11px] text-dim">{event.agentType}</span>}
        <StatusMark status={event.status} /><StatusWord status={event.status} />
      </div>
      {event.report.trim() !== '' && (
        <>
          <button type="button" aria-expanded={open} className="mt-1 text-[11.5px] text-dim hover:text-fg" onClick={() => { setOpen((current) => !current) }}>{open ? 'Hide its report' : 'Show its report'}</button>
          {open && <Detail text={event.report} empty="" />}
        </>
      )}
    </div>
  )
}
