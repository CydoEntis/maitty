import { useEffect, useState } from 'react'
import { Button } from '@/components/ui/button'
import { Tag } from '@/components/ui/tag'
import { WorkingMark } from './WorkingMark'
import { subagentWord, type SubagentView } from '@/features/workspace/model/subagents'
import { splitLead, paragraphsOf } from '../chatRows'
import { InlineMarkdown, Markdown } from './Markdown'
import { ChatItem } from './ChatItems'
import { useHelperRecord } from '../useHelperRecord'
import type { ChatAgent } from '@/features/workspace/model/chatView'

const SECOND_MS = 1000

/** Now in unix seconds, ticking once a second while `live`, so a running time counts up. */
function useNowSeconds(live: boolean): number {
  const [now, setNow] = useState(() => Math.floor(Date.now() / SECOND_MS))
  useEffect(() => {
    if (!live) return
    const timer = window.setInterval(() => { setNow(Math.floor(Date.now() / SECOND_MS)) }, SECOND_MS)
    return () => { window.clearInterval(timer) }
  }, [live])
  return now
}

/** The bar over a subagent's chat: which subagent, how long, and the way back to its session. */
export function SubagentBar({ subagent, onBack }: { subagent: SubagentView; onBack: () => void }) {
  const now = useNowSeconds(subagent.status === 'working')
  return (
    <div className="mx-sub-bar" role="status">
      <Button variant="ghost" size="sm" aria-label="Back to main session" onClick={onBack}>‹ Main session</Button>
      <span className="mx-sub-bar-dim" aria-hidden="true">·</span>
      <span className="mx-sub-bar-kind">Subagent</span>
      <strong className="mx-sub-bar-name">{subagent.name}</strong>
      <span className="mx-spacer" />
      <span className="mx-sub-bar-time">{subagentWord(subagent, now)}</span>
      <Tag>read-only</Tag>
    </div>
  )
}

type ChatProps = {
  subagent: SubagentView
  agent: ChatAgent
  /** The pane on this computer whose helper it is; null for one on another machine, whose record is not read. */
  term?: number | null
  /** The session's folder, so file names are shown relative to it. */
  root?: string | null
}

/**
 * A subagent's own chat: its own steps, read from the record its runtime keeps for it (ADR 0010), and
 * where there is none, what the runtime reported: what it was asked, what it is doing, and its report.
 * Nothing it did not do or report is shown or guessed.
 */
export function SubagentChat({ subagent, agent, term = null, root = null }: ChatProps) {
  const record = useHelperRecord(term, subagent.id, agent, subagent.status === 'working')
  if (record?.kind === 'rows' && record.rows.length > 0) {
    return (
      <div className="mx-chat-scroll" role="log" aria-label={`Subagent ${subagent.name}`}>
        <div className="mx-chat-column">
          {record.startMissing && <p className="mx-sub-note">Its earlier steps are not shown.</p>}
          {record.rows.map((row) => (
            <ChatItem key={row.key} row={row} agent={agent} workingSince={null} approval={null} root={root} openDiff={null} goToStep={null} waitingStep={null} />
          ))}
          <SubagentNote subagent={subagent} />
        </div>
      </div>
    )
  }
  const { lead, rest } = splitLead(subagent.result === null ? [] : paragraphsOf(subagent.result))
  return (
    <div className="mx-chat-scroll" role="log" aria-label={`Subagent ${subagent.name}`}>
      <div className="mx-chat-column">
        {subagent.task !== null && <div className="mx-msg-user"><p className="mx-bubble">{subagent.task}</p></div>}
        {lead !== '' && (
          <div className="mx-msg-agent">
            <p className="mx-msg-lead"><InlineMarkdown text={lead} /></p>
            {rest.length > 0 && <Markdown text={rest.join('\n\n')} className="mx-msg-rest" />}
          </div>
        )}
        {subagent.status === 'working' && (
          <div className="mx-working">
            <WorkingMark agent={agent} />
            <span>{subagent.latest ?? `${agent} is working`}</span>
          </div>
        )}
        <SubagentNote subagent={subagent} />
        {subagent.status === 'done' && subagent.result === null && <p className="mx-sub-note">It finished without a report the runtime kept.</p>}
      </div>
    </div>
  )
}

/** What the runtime says of a helper that is not simply working or done. */
function SubagentNote({ subagent }: { subagent: SubagentView }) {
  if (subagent.status === 'needs') return <p className="mx-sub-note">It is waiting for an answer. Answer it in the session&apos;s terminal.</p>
  if (subagent.status === 'stopped') return <p className="mx-sub-note">It stopped before it reported back.</p>
  return null
}
