import { useState } from 'react'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { SubagentEvent } from '../chatEvents'
import { agentsSummary, elapsedMs, tookText } from '../stepInfo'
import { Detail, StatusMark, StatusWord } from './cardParts'

/** One helper agent: its title, type, and when it is open, what it was asked and what it reported. */
export function AgentRow({ agent }: { agent: SubagentEvent }) {
  const [open, setOpen] = useState(false)
  const took = agent.status === 'running' ? null : elapsedMs(agent, null, 0)
  const hasMore = agent.prompt !== null || agent.report.trim() !== ''
  const title = agent.title ?? 'a helper agent'
  return (
    <li>
      <button type="button" aria-expanded={hasMore ? open : undefined} disabled={!hasMore} className="mx-tool" title={agent.agentType ?? undefined} onClick={() => { setOpen((current) => !current) }}>
        <StatusMark status={agent.status} />
        <span className="mx-tool-text">
          {agent.status === 'running' ? `Running a subagent: ${title}` : `Ran a subagent: ${title}`}
          {' · '}
          {agent.status === 'running'
            ? <ElapsedSince since={agent.timing.startedAtMs ?? agent.observed.firstSeenAtMs} />
            : agent.status === 'completed'
              ? (took === null ? 'done' : tookText(took))
              : <StatusWord status={agent.status} />}
        </span>
      </button>
      {open && (
        <div className="mx-tool-detail">
          {agent.prompt !== null && (
            <>
              <p className="mt-1 mb-0 text-[11px] font-semibold text-dim">Asked</p>
              <Detail text={agent.prompt} empty="" />
            </>
          )}
          {agent.report.trim() !== '' && (
            <>
              <p className="mt-1.5 mb-0 text-[11px] font-semibold text-dim">Report</p>
              <Detail text={agent.report} empty="" />
            </>
          )}
        </div>
      )}
    </li>
  )
}

/** The subagents a turn ran, one pill each ("Ran a subagent: Check migration order · 4m 2s"). */
export function AgentsCard({ agents }: { agents: readonly SubagentEvent[] }) {
  return (
    <ul role="group" aria-label={agentsSummary(agents)} className="m-0 flex list-none flex-col items-start gap-2 p-0">
      {agents.map((agent) => <AgentRow key={agent.id} agent={agent} />)}
    </ul>
  )
}
