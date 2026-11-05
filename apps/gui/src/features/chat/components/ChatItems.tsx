import { memo, type ReactNode } from 'react'
import { Spinner, AppIcon } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import { dividerText, splitLead, type ChatRow } from '../chatRows'
import { parseAttachmentMessage } from '../attachmentMessage'
import { parseComment } from '../commentMessage'
import { relativePath } from '../stepInfo'
import { AgentsCard } from './AgentsCard'
import { SentAttachments } from './Attachments'
import { ApprovalCard, type ApprovalActions } from './ApprovalCard'
import { InlineMarkdown, Markdown } from './Markdown'
import { CommandCard, EditCard, TodoCard, ToolChip } from './StepCards'

/**
 * The rows of the chat (v18 C.1). Every fact the daemon's history carries is shown, and one it does
 * not carry is said to be unavailable, never guessed. Colours are theme tokens; only a step that is
 * running animates, so a long conversation costs nothing at rest.
 */

const COLUMN = 'mx-auto w-full max-w-[760px] px-5'

const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })

function Frame({ children }: { children: ReactNode }) {
  return <div className={`${COLUMN} py-[7px]`}><div className="ml-9">{children}</div></div>
}

/** What the rows need from the window besides the row itself. */
export type ChatItemContext = {
  agent: ChatAgent
  workingSince: number | null
  approval: ApprovalActions | null
  /** The session's folder, so file names are shown relative to it. */
  root: string | null
  /** Opens a file's diff in the session panel; `null` when the window has no panel. */
  openDiff: ((file: string) => void) | null
  goToStep: ((key: string) => void) | null
}

type RowProps = { row: ChatRow } & ChatItemContext

function sameStrings(a: readonly string[], b: readonly string[]): boolean {
  return a.length === b.length && a.every((value, index) => value === b[index])
}

/**
 * Whether a row would draw the same. Rows are rebuilt on every update, so identity cannot tell;
 * the events beneath keep their identity until they change, and that is what is compared.
 */
function sameRow(a: ChatRow, b: ChatRow): boolean {
  if (a.kind !== b.kind || a.key !== b.key) return false
  switch (a.kind) {
    case 'agent':
      return b.kind === 'agent' && a.status === b.status && sameStrings(a.paragraphs, b.paragraphs)
    case 'agents':
      return b.kind === 'agents' && a.events.length === b.events.length && a.events.every((event, index) => event === b.events[index])
    case 'approval':
      return b.kind === 'approval' && a.event === b.event && a.step?.key === b.step?.key && a.step?.label === b.step?.label
    case 'working':
      return true
    case 'user':
    case 'step':
    case 'divider':
      return 'event' in b && a.event === b.event
  }
}

function sameProps(a: RowProps, b: RowProps): boolean {
  return a.agent === b.agent && a.workingSince === b.workingSince && a.approval === b.approval && a.root === b.root && a.openDiff === b.openDiff && a.goToStep === b.goToStep && sameRow(a.row, b.row)
}

/** One row of the list. Memoised: a streamed update re-renders only the rows whose events changed. */
export const ChatItem = memo(function ChatItem({ row, agent, workingSince, approval, root, openDiff, goToStep }: RowProps) {
  switch (row.kind) {
    case 'user': {
      const comment = parseComment(row.event.text)
      const attached = comment === null ? parseAttachmentMessage(row.event.text) : null
      return (
        <div className={`${COLUMN} flex justify-end py-[7px]`}>
          {attached !== null ? (
            <div className="flex max-w-[85%] flex-col items-end">
              {attached.text !== '' && <p className="m-0 rounded-[16px_16px_5px_16px] border border-line2 bg-raise px-3.5 py-2 text-[14px] leading-[1.55] whitespace-pre-wrap text-fg [overflow-wrap:anywhere]">{attached.text}</p>}
              <SentAttachments files={attached.files} />
            </div>
          ) : comment === null ? (
            <p className="m-0 max-w-[85%] rounded-[16px_16px_5px_16px] border border-line2 bg-raise px-3.5 py-2 text-[14px] leading-[1.55] whitespace-pre-wrap text-fg [overflow-wrap:anywhere]">{row.event.text}</p>
          ) : (
            <div role="group" aria-label={`Comment on ${comment.file}:${String(comment.line)}`} className="max-w-[85%] rounded-[16px_16px_5px_16px] border border-line2 bg-raise px-3.5 py-2 text-[14px] leading-[1.55] text-fg [overflow-wrap:anywhere]">
              <p className="m-0 text-[11.5px] font-semibold text-fg2">Comment on <code className="font-mono">{relativePath(comment.file, root)}:{comment.line}</code></p>
              <pre className="my-1.5 overflow-x-auto rounded-lg border border-line2 bg-term px-2.5 py-1.5 font-mono text-[12px] leading-[1.5] whitespace-pre-wrap text-fg2">{comment.code === '' ? ' ' : comment.code}</pre>
              <p className="m-0 whitespace-pre-wrap">{comment.text}</p>
            </div>
          )}
        </div>
      )
    }
    case 'agent': {
      const { lead, rest } = splitLead(row.paragraphs)
      const empty = lead === '' && rest.length === 0
      return (
        <div className={`${COLUMN} flex gap-3 py-[7px]`}>
          <span className="mt-0.5 shrink-0"><AppIcon process={agent} size={24} /></span>
          <div className="flex min-w-0 flex-1 flex-col gap-2 text-[14px] leading-[1.6] [overflow-wrap:anywhere]">
            {lead !== '' && <p className="m-0 whitespace-pre-wrap text-fg"><InlineMarkdown text={lead} /></p>}
            {rest.length > 0 && <Markdown text={rest.join('\n\n')} className="text-fg2" />}
            {empty && <p className="m-0 text-dim">{row.status === 'running' ? 'Writing…' : 'An empty reply.'}</p>}
          </div>
        </div>
      )
    }
    case 'step':
      return (
        <Frame>
          {row.event.kind === 'edit' ? <EditCard event={row.event} root={root} onOpen={openDiff} />
            : row.event.kind === 'command' ? <CommandCard event={row.event} />
              : row.event.kind === 'todo' ? <TodoCard event={row.event} />
                : <ToolChip event={row.event} />}
        </Frame>
      )
    case 'agents':
      return <Frame><AgentsCard agents={row.events} /></Frame>
    case 'approval':
      return <Frame><ApprovalCard event={row.event} actions={approval} step={row.step} onGoToStep={goToStep} /></Frame>
    case 'divider':
      return (
        <div className={`${COLUMN} flex items-center gap-3 py-3`} role="separator" aria-label={dividerText(row.event, clock)}>
          <span className="h-px flex-1 bg-line2" />
          <span className="text-[11.5px] text-dim">{dividerText(row.event, clock)}</span>
          <span className="h-px flex-1 bg-line2" />
        </div>
      )
    case 'working':
      // Token counts are not shown: the daemon's history carries none, and an estimate would be invented.
      return (
        <div className={`${COLUMN} flex items-center gap-2 py-3 text-[12.5px] text-fg2`}>
          <Spinner className="mx-spin text-[var(--accent)]" />
          <span>{agent} is working{workingSince !== null && <> · <ElapsedSince since={workingSince} /></>}</span>
        </div>
      )
  }
}, sameProps)
