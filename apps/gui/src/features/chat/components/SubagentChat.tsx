import { useEffect, useState } from 'react'
import { WorkingMark } from './WorkingMark'
import { subagentWord, type SubagentView } from '@/features/workspace/model/subagents'
import { splitLead, paragraphsOf } from '../chatRows'
import { InlineMarkdown, Markdown } from './Markdown'

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
      <span className="mx-sub-bar-dim">Subagent</span>
      <strong>{subagent.name}</strong>
      <span className="mx-sub-bar-dim">· {subagentWord(subagent, now)}</span>
      <span className="mx-spacer" />
      <button type="button" className="mx-sub-back" onClick={onBack}>Back to main session</button>
    </div>
  )
}

/**
 * A subagent's own chat, from what its runtime reports: what it was asked, what it is doing while it
 * works, and its report once done. Nothing it did not report is shown or guessed.
 */
export function SubagentChat({ subagent, agent }: { subagent: SubagentView; agent: string }) {
  const { lead, rest } = splitLead(subagent.result === null ? [] : paragraphsOf(subagent.result))
  return (
    <div className="mx-chat-scroll" role="log" aria-label={`Subagent ${subagent.name}`}>
      <div className="mx-chat-column">
        {subagent.task !== null && <div className="mx-msg-user"><p>{subagent.task}</p></div>}
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
        {subagent.status === 'needs' && <p className="mx-sub-note">It is waiting for an answer. Answer it in the session&apos;s terminal.</p>}
        {subagent.status === 'stopped' && <p className="mx-sub-note">It stopped before it reported back.</p>}
        {subagent.status === 'done' && subagent.result === null && <p className="mx-sub-note">It finished without a report the runtime kept.</p>}
      </div>
    </div>
  )
}
