import { beforeEach, describe, expect, it } from 'vitest'
import { liveAgentTerminals, ownedTerminalIds, readAgentTerminals, withAgentTerminal, writeAgentTerminals } from './agentTerminals'

beforeEach(() => { localStorage.clear() })

describe('agent terminals', () => {
  it('adds a terminal after the ones the agent already has', () => {
    const one = withAgentTerminal(new Map(), 'a1', 't2', new Set(['a1', 't2']))
    const two = withAgentTerminal(one, 'a1', 't3', new Set(['a1', 't2', 't3']))
    expect(two.get('a1')).toEqual(['t2', 't3'])
  })

  it('drops an agent that is gone and a terminal that is gone when one is added', () => {
    const known = new Map([['gone', ['t9']], ['a1', ['t2', 'closed']]])
    const next = withAgentTerminal(known, 'a1', 't3', new Set(['a1', 't2', 't3', 't9']))
    expect([...next.entries()]).toEqual([['a1', ['t2', 't3']]])
  })

  it('counts a terminal as an agent’s only while both are sessions, so one whose agent is gone is listed again', () => {
    const known = new Map([['a1', ['t2']], ['gone', ['t9']]])
    const live = liveAgentTerminals(known, new Set(['a1', 't2', 't9']))
    expect([...ownedTerminalIds(live)]).toEqual(['t2'])
  })

  it('remembers them between runs, and reads nothing from a value that is not a list of ids', () => {
    writeAgentTerminals(new Map([['a1', ['t2', 't3']]]))
    expect(readAgentTerminals().get('a1')).toEqual(['t2', 't3'])
    localStorage.setItem('maitty.gui.agent-terminals.v1', JSON.stringify({ a1: 'nope', a2: [1, 2] }))
    expect(readAgentTerminals().size).toBe(0)
    localStorage.setItem('maitty.gui.agent-terminals.v1', 'not json')
    expect(readAgentTerminals().size).toBe(0)
  })
})
