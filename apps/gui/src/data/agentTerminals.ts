const STORAGE_KEY = 'maitty.gui.agent-terminals.v1'

/** The terminals this window opened for an agent, by the agent's session id, in the order they were opened. */
export type AgentTerminals = ReadonlyMap<string, readonly string[]>

function isIdList(value: unknown): value is string[] {
  return Array.isArray(value) && value.every((item) => typeof item === 'string')
}

/**
 * The terminals each agent had last time. Like a split layout, which shell belongs to which agent is
 * this window's own knowledge: the daemon lists every one of them as a session.
 */
export function readAgentTerminals(): AgentTerminals {
  try {
    const value: unknown = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? '{}')
    if (typeof value !== 'object' || value === null || Array.isArray(value)) return new Map()
    return new Map(Object.entries(value).filter((entry): entry is [string, string[]] => isIdList(entry[1])))
  } catch (error) {
    console.warn('Ignoring invalid Maitty agent terminals', error)
    return new Map()
  }
}

export function writeAgentTerminals(terminals: AgentTerminals): void {
  try {
    localStorage.setItem(STORAGE_KEY, JSON.stringify(Object.fromEntries(terminals)))
  } catch (error) {
    console.warn('Could not keep the agent terminals', error)
  }
}

/**
 * `known` with `terminalId` added to `agentId`, and what no longer exists dropped so the list does not
 * grow for ever: an agent that is gone, and a terminal that is gone.
 */
export function withAgentTerminal(known: AgentTerminals, agentId: string, terminalId: string, existing: ReadonlySet<string>): AgentTerminals {
  const kept = liveAgentTerminals(known, existing)
  return new Map(kept).set(agentId, [...(kept.get(agentId) ?? []), terminalId])
}

/**
 * The terminals that are an agent's right now: both the agent and the terminal are sessions the daemon
 * lists. A terminal whose agent is gone is not in here, so it is listed as a session of its own and is
 * never lost.
 */
export function liveAgentTerminals(known: AgentTerminals, existing: ReadonlySet<string>): AgentTerminals {
  const live = new Map<string, readonly string[]>()
  for (const [agentId, terminalIds] of known) {
    if (!existing.has(agentId)) continue
    const here = terminalIds.filter((id) => existing.has(id))
    if (here.length > 0) live.set(agentId, here)
  }
  return live
}

/** Every terminal that is some agent's, for leaving them out of the session list. */
export function ownedTerminalIds(live: AgentTerminals): ReadonlySet<string> {
  return new Set([...live.values()].flat())
}
