import { describe, expect, it } from 'vitest'
import type { TurnEvent } from './chatEvents'
import { approvalEvent, commandEvent, messageEvent } from './eventFixtures'
import { buildRows, dividerText, formatDuration, paragraphsOf, splitLead } from './chatRows'

const message = messageEvent
const command = (id: string, turnId = 't1') => ({ ...commandEvent(id), turnId })
const turn = (outcome: TurnEvent['outcome'], timed: boolean): TurnEvent => ({ kind: 'turn', id: 'turn:t1', turnId: 't1', outcome, startedAtMs: 1_000, endedAtMs: 8_000, timed })

describe('buildRows', () => {
  it('joins the agent messages of one turn that follow each other into one message', () => {
    const rows = buildRows([message('u', 'user', 'go'), message('a1', 'agent', 'First.\n\nSecond.'), message('a2', 'agent', 'Third.')], false)

    expect(rows.map((row) => row.kind)).toEqual(['user', 'agent'])
    expect(rows[1]).toMatchObject({ key: 'a1', paragraphs: ['First.', 'Second.', 'Third.'] })
  })

  it('keeps the order of text and steps: a step between two messages ends the first', () => {
    const rows = buildRows([message('a1', 'agent', 'Looking.'), command('c'), message('a2', 'agent', 'Found it.')], false)

    expect(rows.map((row) => row.kind)).toEqual(['agent', 'step', 'agent'])
  })

  it('never joins messages of different turns', () => {
    const rows = buildRows([message('a1', 'agent', 'One.', 't1'), message('a2', 'agent', 'Two.', 't2')], false)

    expect(rows.map((row) => row.kind)).toEqual(['agent', 'agent'])
  })

  it('keeps one stable key per row and adds the working line last only while a turn runs', () => {
    const events = [message('u', 'user', 'go'), message('a', 'agent', 'On it.')]

    expect(buildRows(events, false).map((row) => row.key)).toEqual(['u', 'a'])
    expect(buildRows(events, true).map((row) => row.key)).toEqual(['u', 'a', 'working'])
  })

  it('turns an approval and a divider into rows of their own', () => {
    const rows = buildRows([approvalEvent(3), turn('completed', true)], false)

    expect(rows.map((row) => row.kind)).toEqual(['approval', 'divider'])
  })
})

describe('paragraphs and the lead sentence', () => {
  it('splits on blank lines only', () => {
    expect(paragraphsOf('a\nstill a\n\nb\n \n\nc')).toEqual(['a\nstill a', 'b', 'c'])
  })

  it('takes the first sentence as the lead and leaves the rest as the body', () => {
    expect(splitLead(['I fixed it. The test passes now.', 'Details follow.'])).toEqual({ lead: 'I fixed it.', rest: ['The test passes now.', 'Details follow.'] })
    expect(splitLead(['No full stop here'])).toEqual({ lead: 'No full stop here', rest: [] })
    expect(splitLead([])).toEqual({ lead: '', rest: [] })
    // A table with no outer pipes is not a sentence either.
    const table = 'a | b\n--- | ---\n1 | 2'
    expect(splitLead([table, 'After.'])).toEqual({ lead: '', rest: [table, 'After.'] })
  })
})

describe('turn dividers', () => {
  const clock = () => '10:11 AM'

  it('says how long a turn took only when the daemon gave its times', () => {
    expect(dividerText(turn('completed', true), clock)).toBe('Cooked for 7s · done 10:11 AM')
    expect(dividerText(turn('failed', true), clock)).toBe('Failed after 7s · 10:11 AM')
    expect(dividerText(turn('completed', false), clock)).toBe('Turn finished')
    expect(dividerText(turn('interrupted', false), clock)).toBe('Turn interrupted')
  })

  it('writes a duration in the largest units that fit', () => {
    expect([0, 7_000, 72_000, 3_900_000].map(formatDuration)).toEqual(['0s', '7s', '1m 12s', '1h 5m'])
  })
})
