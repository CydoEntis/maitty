import { beforeEach, describe, expect, it, vi } from 'vitest'

const daemon = vi.hoisted(() => ({ connectAgent: vi.fn<(agent: string) => Promise<void>>() }))
vi.mock('@/api/daemon', () => daemon)

import { connectAgentOnce, forgetConnectedAgents } from './agentHooks'

describe('connecting an agent\'s status hooks', () => {
  beforeEach(() => {
    vi.clearAllMocks()
    forgetConnectedAgents()
  })

  it('runs `maitty integrate` once per agent, however many chats start', async () => {
    daemon.connectAgent.mockResolvedValue(undefined)

    await Promise.all([connectAgentOnce('claude'), connectAgentOnce('claude')])
    await connectAgentOnce('claude')
    await connectAgentOnce('codex')

    expect(daemon.connectAgent.mock.calls).toEqual([['claude'], ['codex']])
  })

  it('tries again after a failure instead of remembering it', async () => {
    daemon.connectAgent.mockRejectedValueOnce(new Error('no maitty on PATH')).mockResolvedValue(undefined)

    await expect(connectAgentOnce('claude')).rejects.toThrow('no maitty on PATH')
    await connectAgentOnce('claude')

    expect(daemon.connectAgent).toHaveBeenCalledTimes(2)
  })
})
