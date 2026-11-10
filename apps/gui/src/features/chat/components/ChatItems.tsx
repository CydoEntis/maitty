import { memo, useContext, useEffect, useState, type ReactNode } from 'react'
import { Icon, Spinner } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import { dividerText, paragraphsOf, splitLead, streamingParagraphs, type ChatRow } from '../chatRows'
import type { ItemStatus } from '../chatEvents'
import { KnownRowsContext } from '../knownRows'
import { useTypedText } from '../useTypedText'
import { ReducedMotionContext } from '@/features/workspace/components/motion'
import { parseAttachmentMessage } from '../attachmentMessage'
import { AgentsCard } from './AgentsCard'
import { SentAttachments } from './Attachments'
import { ApprovalCard, type ApprovalActions } from './ApprovalCard'
import { InlineMarkdown, Markdown } from './Markdown'
import { CommandCard, EditCard, TodoCard, ToolChip } from './StepCards'
import { tookText } from '../stepInfo'
import { workDurationMs, workStatus, workSummary, type WorkStep } from '../workSummary'

/**
 * The rows of the chat (v18 C.1). Every fact the daemon's history carries is shown, and one it does
 * not carry is said to be unavailable, never guessed. Colours are theme tokens; only a step that is
 * running animates, so a long conversation costs nothing at rest.
 */

const clock = (ms: number) => new Date(ms).toLocaleTimeString([], { hour: 'numeric', minute: '2-digit' })

/** One row of the 760px column; rows sit 14px apart. */
function Frame({ children }: { children: ReactNode }) {
  return <div className="mx-chat-row">{children}</div>
}

/** What the rows need from the window besides the row itself. */
export type ChatItemContext = {
  agent: ChatAgent
  workingSince: number | null
  approval: ApprovalActions | null
  /** The session's folder, so file names are shown relative to it. */
  root: string | null
  /** Opens a file's diff in the Changes panel; `null` where there is none. */
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
    case 'changes':
      return b.kind === 'changes' && sameStrings(a.files, b.files) && a.lines?.added === b.lines?.added && a.lines?.removed === b.lines?.removed
    case 'work':
      return b.kind === 'work' && a.events.length === b.events.length && a.events.every((event, index) => event === b.events[index])
    case 'approval':
      return b.kind === 'approval' && a.event === b.event && a.step?.key === b.step?.key && a.step?.label === b.step?.label
    case 'working':
      return b.kind === 'working' && a.phrase === b.phrase
    case 'echo':
      return b.kind === 'echo' && a.text === b.text
    case 'report':
      return b.kind === 'report' && a.text === b.text
    case 'user':
    case 'step':
    case 'divider':
      return 'event' in b && a.event === b.event
  }
}

function sameProps(a: RowProps, b: RowProps): boolean {
  return a.agent === b.agent && a.workingSince === b.workingSince && a.approval === b.approval && a.root === b.root && a.openDiff === b.openDiff && a.goToStep === b.goToStep && sameRow(a.row, b.row)
}

/**
 * An agent's reply. One still arriving is read in its finished form, so a half-typed mark does not flash, and is typed in
 * as it comes rather than appearing in chunks; a finished one is read as written, once the typing has caught up.
 */
function AgentMessage({ rowKey, paragraphs, status }: { rowKey: string; paragraphs: readonly string[]; status: ItemStatus }) {
  const reduced = useContext(ReducedMotionContext)
  const known = useContext(KnownRowsContext)
  const target = (status === 'running' ? streamingParagraphs(paragraphs) : paragraphs).join('\n\n')
  const typed = useTypedText(target, !reduced, known !== null && !known.has(rowKey))
  // Typed in once: drawn again, this row shows whole.
  useEffect(() => { known?.add(rowKey) }, [known, rowKey])
  const { lead, rest } = splitLead(typed === target ? (status === 'running' ? streamingParagraphs(paragraphs) : paragraphs) : paragraphsOf(typed))
  const empty = lead === '' && rest.length === 0
  return (
    <div className="mx-msg-agent">
      {lead !== '' && <p className="mx-msg-lead"><InlineMarkdown text={lead} /></p>}
      {rest.length > 0 && <Markdown text={rest.join('\n\n')} className="mx-msg-rest" />}
      {empty && <p className="mx-msg-empty">{status === 'running' ? 'Writing…' : 'An empty reply.'}</p>}
    </div>
  )
}

/**
 * What the person said: its text, and its attached files as chips. A message on its way is drawn the
 * same as one the history holds, so the files never show as the paths that carry them.
 */
function SaidText({ text }: { text: string }) {
  const attached = parseAttachmentMessage(text)
  if (attached === null) return <p>{text}</p>
  return (
    <div className="flex max-w-[85%] flex-col items-end">
      {attached.text !== '' && <p className="mx-bubble">{attached.text}</p>}
      <SentAttachments files={attached.files} />
    </div>
  )
}

function Step({ event, root, openDiff }: { event: WorkStep | Extract<ChatRow, { kind: 'step' }>['event']; root: string | null; openDiff: ((file: string) => void) | null }) {
  if (event.kind === 'edit') return <EditCard event={event} root={root} onOpen={openDiff} />
  if (event.kind === 'command') return <CommandCard event={event} />
  if (event.kind === 'todo') return <TodoCard event={event} />
  return <ToolChip event={event} />
}

/**
 * A run of steps as one line saying what it did; a click opens the steps themselves. One that failed starts open,
 * so a failure is never folded away.
 */
function WorkCard({ steps, root, openDiff }: { steps: readonly WorkStep[]; root: string | null; openDiff: ((file: string) => void) | null }) {
  const status = workStatus(steps)
  const [open, setOpen] = useState(status === 'failed')
  const took = workDurationMs(steps)
  return (
    <div className={`mx-work${open ? ' is-open' : ''}`}>
      <button type="button" className="mx-work-head" aria-expanded={open} title={open ? 'Hide the steps' : 'Show the steps'} onClick={() => { setOpen((current) => !current) }}>
        {status === 'running' ? <Spinner /> : <span className={`mx-work-mark is-${status}`} aria-hidden="true"><Icon name={status === 'failed' ? 'close' : 'check'} size={10} /></span>}
        <span className="mx-work-text">{workSummary(steps)}{status === 'failed' && ' · a step failed'}</span>
        {took !== null && status !== 'running' && <span className="mx-work-took">{tookText(took)}</span>}
        <span className={`mx-caret-wrap${open ? ' is-open' : ''}`}><Icon name="caret" /></span>
      </button>
      {open && <div className="mx-work-steps">{steps.map((step) => <Step key={step.id} event={step} root={root} openDiff={openDiff} />)}</div>}
    </div>
  )
}

/** One row of the list. Memoised: a streamed update re-renders only the rows whose events changed. */
export const ChatItem = memo(function ChatItem({ row, agent, workingSince, approval, root, openDiff, goToStep }: RowProps) {
  switch (row.kind) {
    case 'user':
      return <Frame><div className="mx-msg-user"><SaidText text={row.event.text} /></div></Frame>
    case 'report':
      return <Frame><p className="mx-report"><Icon name="check" size={11} />{row.text}</p></Frame>
    case 'echo':
      return (
        <Frame>
          <div className="mx-msg-user is-echo" aria-label="Sending"><SaidText text={row.text} /></div>
        </Frame>
      )
    case 'agent':
      return <Frame><AgentMessage rowKey={row.key} paragraphs={row.paragraphs} status={row.status} /></Frame>
    case 'step':
      return <Frame><Step event={row.event} root={root} openDiff={openDiff} /></Frame>
    case 'work':
      return <Frame><WorkCard steps={row.events} root={root} openDiff={openDiff} /></Frame>
    case 'agents':
      return <Frame><AgentsCard agents={row.events} /></Frame>
    case 'approval':
      return <Frame><ApprovalCard event={row.event} actions={approval} step={row.step} onGoToStep={goToStep} /></Frame>
    case 'divider':
      return (
        <div className="mx-chat-row flex items-center gap-3" role="separator" aria-label={dividerText(row.event, clock)}>
          <span className="h-px flex-1 bg-line2" />
          <span className="text-[11.5px] text-dim">{dividerText(row.event, clock)}</span>
          <span className="h-px flex-1 bg-line2" />
        </div>
      )
    case 'changes': {
      const first = row.files[0]
      const said = `${String(row.files.length)} changed ${row.files.length === 1 ? 'file' : 'files'}`
      const counts = row.lines !== null && <span className="mx-changed-counts"><span className="is-add">+{row.lines.added}</span><span className="is-remove">−{row.lines.removed}</span></span>
      return (
        <Frame>
          {openDiff !== null && first !== undefined
            ? <button type="button" className="mx-changed" title="Show the changes" onClick={() => { openDiff(first) }}><Icon name="changes" size={14} /><span>{said}</span>{counts}<Icon name="chevron" /></button>
            : <p className="mx-changed is-still"><Icon name="changes" size={14} /><span>{said}</span>{counts}</p>}
        </Frame>
      )
    }
    case 'working':
      // Token counts are not shown: the daemon's history carries none, and an estimate would be invented.
      return (
        <Frame>
          <div className="mx-working">
            <Spinner />
            <span>{agent} is {row.phrase}{workingSince !== null && <> · <ElapsedSince since={workingSince} /></>}</span>
          </div>
        </Frame>
      )
  }
}, sameProps)
