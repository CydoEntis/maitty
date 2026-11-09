import { connectAgent } from '@/api/daemon'

type Agent = Parameters<typeof connectAgent>[0]

const connecting = new Map<Agent, Promise<void>>()

/**
 * An agent's status hooks are connected once per run of the window (`maitty integrate` is idempotent, but it is a
 * process to start). A chat that starts waits for this, so it is asked ahead of time where the window can; a failure
 * is forgotten so the next start tries again.
 */
export function connectAgentOnce(agent: Agent): Promise<void> {
  const known = connecting.get(agent)
  if (known) return known
  const started = connectAgent(agent).catch((error: unknown) => {
    connecting.delete(agent)
    throw error
  })
  connecting.set(agent, started)
  return started
}

/** For tests. */
export function forgetConnectedAgents(): void {
  connecting.clear()
}
