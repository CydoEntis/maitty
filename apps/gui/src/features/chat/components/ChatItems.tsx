import { memo, useContext, useEffect, type ReactNode } from 'react'
import { FileText } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { Card, CardHead } from '@/components/ui/card'
import { StatusMark } from '@/components/ui/status-mark'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import type { TurnSpan } from '@/features/inspect/turnRange'
import { dividerText, paragraphsOf, splitLead, streamingParagraphs, type ChatRow } from '../chatRows'
import type { ItemStatus } from '../chatEvents'
import { KnownRowsContext } from '../knownRows'
import { isWaitingStep } from '../stepInfo'
import { useTypedText } from '../useTypedText'
import { WorkingMark } from './WorkingMark'
import { ReducedMotionContext } from '@/features/workspace/components/motion'
import { parseAttachmentMessage } from '../attachmentMessage'
import { AgentsCard } from './AgentsCard'
import { SentAttachments } from './Attachments'
import { ApprovalCard, type ApprovalActions } from './ApprovalCard'
import { InlineMarkdown, Markdown } from './Markdown'
import { PlanCard } from './PlanCard'
import { StepView } from './StepCards'
import { ThoughtRow } from './ThoughtRow'
import { WorkCard } from './WorkCard'
import { MessageMenu } from './MessageMenu'
import { stripRestoreNote, type EditFromHere } from '../editFromHere'

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
  /**
   * Opens a file's diff in the Changes panel; `null` where there is none. A turn's changes row passes the turn too
   * (null when its times are unknown), so the panel shows what that turn changed.
   */
  openDiff: ((file: string, turn?: TurnSpan | null) => void) | null
  goToStep: ((key: string) => void) | null
  /** The id of the tool step a pending request is about, so that step says it is waiting. */
  waitingStep: string | null
  /** Edit from here on a message the person sent; absent where the chat offers none. */
  editFromHere?: EditFromHere | null
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
      return b.kind === 'changes' && sameStrings(a.files, b.files) && a.lines?.added === b.lines?.added && a.lines?.removed === b.lines?.removed && a.turn?.startedAtMs === b.turn?.startedAtMs && a.turn?.endedAtMs === b.turn?.endedAtMs
    case 'thought':
      return b.kind === 'thought' && sameStrings(a.texts, b.texts)
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
  return a.agent === b.agent && a.workingSince === b.workingSince && a.approval === b.approval && a.root === b.root && a.openDiff === b.openDiff && a.goToStep === b.goToStep && a.waitingStep === b.waitingStep && a.editFromHere === b.editFromHere && sameRow(a.row, b.row)
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
      {empty && <p className="mx-msg-empty">{status === 'running' ? 'Writing…' : 'The agent finished without writing a reply.'}</p>}
    </div>
  )
}

/**
 * What the person said: its text, and its attached files as chips. A message on its way is drawn the
 * same as one the history holds, so the files never show as the paths that carry them.
 */
function SaidText({ text: sent }: { text: string }) {
  // The note a restore put in front is for the agent; the person sees what they wrote.
  const text = stripRestoreNote(sent)
  const attached = parseAttachmentMessage(text)
  if (attached === null) return <p className="mx-bubble">{text}</p>
  // Attachments sit above the bubble, as they were added before the text was sent.
  return (
    <>
      <SentAttachments files={attached.files} />
      {attached.text !== '' && <p className="mx-bubble">{attached.text}</p>}
    </>
  )
}

/** One row of the list. Memoised: a streamed update re-renders only the rows whose events changed. */
export const ChatItem = memo(function ChatItem({ row, agent, workingSince, approval, root, openDiff, goToStep, waitingStep, editFromHere = null }: RowProps) {
  switch (row.kind) {
    case 'user': {
      // Only the daemon's own time: when the window first saw a message is not when it was sent.
      const sentAt = row.event.timing.startedAtMs ?? row.event.timing.completedAtMs
      return (
        <Frame>
          <div className="mx-msg-user">
            <SaidText text={row.event.text} />
            {(sentAt !== null || editFromHere !== null) && (
              <div className="mx-msg-meta">
                {sentAt !== null && <time className="mx-msg-time" dateTime={new Date(sentAt).toISOString()}>{clock(sentAt)}</time>}
                {editFromHere !== null && <MessageMenu message={row.event} editFromHere={editFromHere} />}
              </div>
            )}
          </div>
        </Frame>
      )
    }
    case 'report':
      return <Frame><p className="mx-report" title={row.text}><span className="mx-report-arrow" aria-hidden="true">↳</span><span className="mx-report-text">{row.text}</span></p></Frame>
    case 'echo':
      return (
        <Frame>
          <div className="mx-msg-user is-echo" aria-label="Sending">
            <SaidText text={row.text} />
            <div className="mx-msg-meta"><WorkingMark agent={agent} />Sending…</div>
          </div>
        </Frame>
      )
    case 'agent':
      return <Frame><AgentMessage rowKey={row.key} paragraphs={row.paragraphs} status={row.status} /></Frame>
    case 'step':
      return <Frame>{row.event.kind === 'todo' ? <PlanCard event={row.event} /> : <StepView event={row.event} root={root} openDiff={openDiff} alone waiting={isWaitingStep(waitingStep, row.event.id)} />}</Frame>
    case 'thought':
      return <Frame><ThoughtRow texts={row.texts} /></Frame>
    case 'work':
      return <Frame><WorkCard steps={row.events} root={root} openDiff={openDiff} waitingStep={waitingStep} /></Frame>
    case 'agents':
      return <Frame><AgentsCard agents={row.events} process={agent} /></Frame>
    case 'approval':
      return <Frame><ApprovalCard event={row.event} actions={approval} step={row.step} onGoToStep={goToStep} /></Frame>
    case 'divider':
      return (
        <div className={`mx-chat-row mx-divider${row.event.outcome === 'failed' ? ' is-failed' : ''}`} role="separator" aria-label={dividerText(row.event, clock)}>
          <span className="mx-divider-line" />
          {row.event.outcome !== 'completed' && <span aria-hidden="true"><StatusMark state={row.event.outcome === 'failed' ? 'failed' : 'stopped'} /></span>}
          <span className="mx-divider-text">{dividerText(row.event, clock)}</span>
          <span className="mx-divider-line" />
        </div>
      )
    case 'changes': {
      const first = row.files[0]
      // A head-only card (`ChangedFilesBar`): Review opens the side panel on what this turn changed.
      return (
        <Frame>
          <Card className="mx-changed">
            <CardHead>
              <FileText className="size-[13px] shrink-0 text-(--muted)" aria-hidden="true" />
              <span className="mx-changed-said">{row.files.length} changed {row.files.length === 1 ? 'file' : 'files'}</span>
              {row.lines !== null && <span className="mx-changed-counts"><span className="is-add">+{row.lines.added}</span><span className="is-remove">−{row.lines.removed}</span></span>}
              <span className="mx-spacer" />
              {openDiff !== null && first !== undefined && <Button variant="soft" size="sm" title="Show what this turn changed" onClick={() => { openDiff(first, row.turn) }}>Review</Button>}
            </CardHead>
          </Card>
        </Frame>
      )
    }
    case 'working':
      // Token counts are not shown: the daemon's history carries none, and an estimate would be invented.
      return (
        <Frame>
          <div className="mx-working">
            <WorkingMark agent={agent} />
            <span className="mx-working-text">{agent} is {row.phrase}</span>
            {workingSince !== null && <span className="mx-working-time"><ElapsedSince since={workingSince} /></span>}
          </div>
        </Frame>
      )
  }
}, sameProps)
