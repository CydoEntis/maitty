import { describe, expect, it } from 'vitest'
import type { SubagentActivity } from '@/data/preview'
import { durationWords, listedSubagents, subagentWord, toSubagentView } from './subagents'

const activity = (extra: Partial<SubagentActivity> = {}): SubagentActivity => ({ id: 'a', name: 'Explore', state: 'Working', task: null, startedAt: 1000, updatedAt: 1100, latest: null, result: null, ...extra })

describe('a subagent as the window shows it', () => {
  it('is named by the first line of its task, else by its kind', () => {
    expect(toSubagentView(activity({ task: '\nCheck migration order\nthen report' })).name).toBe('Check migration order')
    expect(toSubagentView(activity()).name).toBe('Explore')
    expect(toSubagentView(activity({ task: 'x'.repeat(80) })).name).toHaveLength(48)
  })

  it('reads the runtime’s states as working, needs you, done or stopped', () => {
    expect(toSubagentView(activity({ state: 'Starting' })).status).toBe('working')
    expect(toSubagentView(activity({ state: 'AwaitingApproval' })).status).toBe('needs')
    expect(toSubagentView(activity({ state: 'WaitingForInput' })).status).toBe('needs')
    expect(toSubagentView(activity({ state: 'Completed' })).status).toBe('done')
    expect(toSubagentView(activity({ state: 'ConnectionLost' })).status).toBe('stopped')
  })

  it('is listed while it works or waits for you, and when it stopped only until the agent\'s turn is over', () => {
    const all = [
      toSubagentView(activity({ id: 'working', state: 'Working' })),
      toSubagentView(activity({ id: 'asking', state: 'AwaitingApproval' })),
      toSubagentView(activity({ id: 'finished', state: 'Completed' })),
      toSubagentView(activity({ id: 'lost', state: 'ConnectionLost' })),
      toSubagentView(activity({ id: 'failed', state: 'Failed' })),
    ]
    expect(listedSubagents(all, true).map((subagent) => subagent.id)).toEqual(['working', 'asking', 'lost', 'failed'])
    expect(listedSubagents(all, false).map((subagent) => subagent.id)).toEqual(['working', 'asking'])
  })

  it('says how long it has run, how long it took, or its state when no time was reported', () => {
    expect(subagentWord(toSubagentView(activity()), 1070)).toBe('1m 10s')
    expect(subagentWord(toSubagentView(activity({ state: 'Completed' })), 5000)).toBe('done in 1m 40s')
    expect(subagentWord(toSubagentView(activity({ state: 'AwaitingApproval' })), 5000)).toBe('needs you')
    expect(subagentWord(toSubagentView(activity({ startedAt: null })), 5000)).toBe('working')
  })

  it('writes a duration the way the sidebar does', () => {
    expect(durationWords(48)).toBe('48s')
    expect(durationWords(263)).toBe('4m 23s')
    expect(durationWords(3900)).toBe('1h 05m')
  })
})
