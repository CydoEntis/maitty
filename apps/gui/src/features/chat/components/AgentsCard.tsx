import { useState } from 'react'
import { Card, CardHead } from '@/components/ui/card'
import { AppIcon } from '@/features/workspace/components/glyphs'
import { ElapsedSince } from '@/features/workspace/components/times'
import type { SubagentEvent } from '../chatEvents'
import { agentsSummary, elapsedMs, tookText } from '../stepInfo'
import { StepGlyph } from './cardParts'

const TILE_SIZE = 22

function AgentTime({ agent }: { agent: SubagentEvent }) {
  if (agent.status === 'running') return <span className="mx-step-meta is-working"><ElapsedSince since={agent.timing.startedAtMs ?? agent.observed.firstSeenAtMs} /></span>
  if (agent.status === 'failed') return <span className="mx-step-meta is-danger">failed</span>
  if (agent.status === 'interrupted') return <span className="mx-step-meta is-dim">stopped</span>
  const took = elapsedMs(agent, null, 0)
  return <span className="mx-step-meta is-dim">{took === null ? 'done' : tookText(took)}</span>
}

type AgentRowProps = {
  agent: SubagentEvent
  /** The program the helper runs under, for its tile. */
  process: string
}

/** One helper agent: its tile and status, its title and type, what it was asked, and its report when opened. */
export function AgentRow({ agent, process }: AgentRowProps) {
  const [open, setOpen] = useState(false)
  const report = agent.report.trim()
  const hasMore = agent.prompt !== null || report !== ''
  const asked = agent.prompt?.split('\n')[0] ?? null
  return (
    <li>
      <button type="button" aria-expanded={hasMore ? open : undefined} disabled={!hasMore} className="mx-agent-row" onClick={() => { setOpen((current) => !current) }}>
        <span className="mx-agent-tile">
          <AppIcon process={process} size={TILE_SIZE} />
          <span className="mx-agent-badge"><StepGlyph status={agent.status} /></span>
        </span>
        <span className="mx-agent-text">
          <span className="mx-agent-name">
            <strong>{agent.title ?? 'A helper agent'}</strong>
            {agent.agentType !== null && <span>{agent.agentType}</span>}
          </span>
          {asked !== null && <span className="mx-agent-asked">{asked}</span>}
        </span>
        <AgentTime agent={agent} />
      </button>
      {open && (
        <div className="mx-agent-detail">
          {agent.prompt !== null && <p className="mx-agent-prompt">{agent.prompt}</p>}
          {report !== '' && <p className="mx-agent-report">{agent.report}</p>}
        </div>
      )}
    </li>
  )
}

type AgentsCardProps = { agents: readonly SubagentEvent[]; process: string }

/** The helper agents a turn ran. One is a row by itself; several share a card that counts them. */
export function AgentsCard({ agents, process }: AgentsCardProps) {
  const rows = agents.map((agent) => <AgentRow key={agent.id} agent={agent} process={process} />)
  if (agents.length === 1) return <ul role="group" aria-label={agentsSummary(agents)} className="mx-agents">{rows}</ul>
  const working = agents.filter((agent) => agent.status === 'running').length
  const done = agents.filter((agent) => agent.status === 'completed').length
  return (
    <Card role="group" aria-label={agentsSummary(agents)}>
      <CardHead>
        <span>{agents.length} agents</span>
        <span className="mx-agents-counts">{[working > 0 ? `${String(working)} working` : null, done > 0 ? `${String(done)} done` : null].filter((part) => part !== null).join(' · ')}</span>
      </CardHead>
      <ul className="mx-agents is-carded">{rows}</ul>
    </Card>
  )
}
