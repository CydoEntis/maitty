import { useState } from 'react'
import { Users } from 'lucide-react'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { SubagentEvent } from '../chatEvents'
import { agentsSummary, elapsedMs, tookText } from '../stepInfo'
import { Detail, StatusMark, StatusWord } from './cardParts'

/** One helper agent: its title, type, and when it is open, what it was asked and what it reported. */
export function AgentRow({ agent }: { agent: SubagentEvent }) {
  const [open, setOpen] = useState(false)
  const took = agent.status === 'running' ? null : elapsedMs(agent, null, 0)
  const hasMore = agent.prompt !== null || agent.report.trim() !== ''
  return (
    <li className="py-1">
      <button type="button" aria-expanded={hasMore ? open : undefined} disabled={!hasMore} className="flex min-h-[38px] w-full items-center gap-2.5 rounded-lg px-1 text-left enabled:hover:bg-hover" onClick={() => { setOpen((current) => !current) }}>
        <StatusMark status={agent.status} />
        <span className="min-w-0 flex-1">
          <span className="block text-[13px] text-fg [overflow-wrap:anywhere]">{agent.title ?? 'A helper agent'}</span>
          {agent.agentType !== null && <span className="block font-mono text-[11.5px] text-dim">{agent.agentType}</span>}
        </span>
        <span className="shrink-0 font-mono text-[11.5px] text-dim">
          {agent.status === 'running'
            ? <ElapsedSince since={agent.timing.startedAtMs ?? agent.observed.firstSeenAtMs} />
            : agent.status === 'completed'
              ? `Done${took === null ? '' : ` · ${tookText(took)}`}`
              : <StatusWord status={agent.status} />}
        </span>
      </button>
      {open && (
        <div className="pl-7">
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

/** "3 agents, 2 working", then a row per helper (v18 C.1). Shown in the chat and in the session panel. */
export function AgentsCard({ agents }: { agents: readonly SubagentEvent[] }) {
  return (
    <div role="group" aria-label="Helper agents" className="rounded-[10px] border border-line2 bg-raise px-3 py-2">
      <p className="m-0 mb-0.5 flex items-center gap-1.5 text-[11.5px] font-semibold text-fg2">
        <Users className="size-3.5 text-dim" aria-hidden="true" />
        {agentsSummary(agents)}
      </p>
      <ul className="m-0 list-none p-0">
        {agents.map((agent) => <AgentRow key={agent.id} agent={agent} />)}
      </ul>
    </div>
  )
}
