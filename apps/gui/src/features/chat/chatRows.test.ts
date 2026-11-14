import { describe, expect, it } from 'vitest'
import type { TurnEvent } from './chatEvents'
import { approvalEvent, commandEvent, editEvent, messageEvent, reasoningEvent, todoEvent, toolEvent } from './eventFixtures'
import { buildRows, dividerText, formatDuration, paragraphsOf, splitLead, streamingParagraphs } from './chatRows'

const message = messageEvent
const command = (id: string, turnId = 't1') => ({ ...commandEvent(id), turnId })
const turn = (outcome: TurnEvent['outcome'], timed: boolean): TurnEvent => ({ kind: 'turn', id: 'turn:t1', turnId: 't1', outcome, startedAtMs: 1_000, endedAtMs: 8_000, timed })

const writtenDoc = (id: string) => editEvent(id, { tool: 'Write', file: '/home/me/Downloads/plan.md', status: 'completed', change: 'add', diff: { status: 'available', hunks: [{ header: '@@ -0,0 +1,2 @@', lines: [{ kind: 'add', text: '# Plan' }, { kind: 'add', text: 'Do it.' }] }], added: 2, removed: 0, truncated: false } })

describe('buildRows', () => {
  it('shows reasoning between two tool steps as its own row, so the steps do not fold across it', () => {
    const rows = buildRows([command('c1'), reasoningEvent('r1', 'Weighing two fixes'), command('c2')], false)

    expect(rows.map((row) => row.kind)).toEqual(['step', 'thought', 'step'])
    expect(rows[1]).toMatchObject({ key: 'r1', turnId: 't1', texts: ['Weighing two fixes'] })
  })

  it('ends the run of steps before a thought and splits an agent message around it', () => {
    const rows = buildRows([command('c1'), command('c2'), reasoningEvent('r1', 'Hm.'), command('c3'), message('a1', 'agent', 'A.'), reasoningEvent('r2', 'Hm.'), message('a2', 'agent', 'B.')], false)

    expect(rows.map((row) => row.kind)).toEqual(['work', 'thought', 'step', 'agent', 'thought', 'agent'])
  })

  it('merges reasoning that follows each other in one turn into one row, and keeps another turn apart', () => {
    const rows = buildRows([reasoningEvent('r1', 'First.'), reasoningEvent('r2', 'Second.'), reasoningEvent('r3', 'Other turn.', 't2')], false)

    expect(rows).toMatchObject([{ kind: 'thought', key: 'r1', texts: ['First.', 'Second.'] }, { kind: 'thought', key: 'r3', texts: ['Other turn.'] }])
  })

  it('shows a Markdown document the agent wrote as a row of its own, not folded into the run around it', () => {
    const rows = buildRows([command('c1'), command('c2'), writtenDoc('doc'), command('c3'), command('c4')], false)

    expect(rows.map((row) => row.kind)).toEqual(['work', 'step', 'work'])
    expect(rows[1]).toMatchObject({ key: 'doc' })
  })

  it('joins the agent messages of one turn that follow each other into one message', () => {
    const rows = buildRows([message('u', 'user', 'go'), message('a1', 'agent', 'First.\n\nSecond.'), message('a2', 'agent', 'Third.')], false)

    expect(rows.map((row) => row.kind)).toEqual(['user', 'agent'])
    expect(rows[1]).toMatchObject({ key: 'a1', paragraphs: ['First.', 'Second.', 'Third.'] })
  })

  it('keeps the order of text and steps: a step between two messages ends the first', () => {
    const rows = buildRows([message('a1', 'agent', 'Looking.'), command('c'), message('a2', 'agent', 'Found it.')], false)

    expect(rows.map((row) => row.kind)).toEqual(['agent', 'step', 'agent'])
  })

  it('folds steps that follow each other into one row, keyed by the first so it stays put as more join', () => {
    const rows = buildRows([message('a1', 'agent', 'Looking.'), command('c1'), command('c2'), command('c3'), message('a2', 'agent', 'Done.')], false)

    expect(rows.map((row) => row.kind)).toEqual(['agent', 'work', 'agent'])
    expect(rows[1]).toMatchObject({ key: 'c1' })
    expect(rows[1]?.kind === 'work' && rows[1].events.map((event) => event.id)).toEqual(['c1', 'c2', 'c3'])
  })

  it('says under a finished turn how many files it changed, each file once, with the lines when every diff was kept', () => {
    const diff = (added: number, removed: number) => ({ status: 'available' as const, hunks: [], added, removed, truncated: false })
    const rows = buildRows([editEvent('e1', { file: 'a.ts', diff: diff(3, 1) }), editEvent('e2', { file: 'a.ts', diff: diff(2, 0) }), editEvent('e3', { file: 'b.ts', diff: diff(1, 1) }), turn('completed', true)], false)

    expect(rows.at(-1)).toEqual({ kind: 'changes', key: 'changes:t1', files: ['a.ts', 'b.ts'], lines: { added: 6, removed: 2 }, turn: { startedAtMs: 1_000, endedAtMs: 8_000 } })
  })

  it('gives a changes row no turn times when the daemon did not time it', () => {
    const rows = buildRows([editEvent('e1', { file: 'a.ts' }), turn('completed', false)], false)
    expect(rows.at(-1)).toMatchObject({ kind: 'changes', turn: null })
  })

  it('gives no line counts when a diff was not kept, and no card for a turn that changed nothing', () => {
    const rows = buildRows([editEvent('e1', { file: 'a.ts', diff: { status: 'unavailable' } }), turn('completed', true)], false)
    expect(rows.at(-1)).toMatchObject({ kind: 'changes', lines: null })
    expect(buildRows([command('c'), turn('completed', true)], false).map((row) => row.kind)).toEqual(['step', 'divider'])
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

  it('draws a message just sent before the history has it, and takes the agent to be at work on it', () => {
    const rows = buildRows([message('a', 'agent', 'Earlier.')], false, null, 'hello')

    expect(rows.map((row) => row.kind)).toEqual(['agent', 'echo', 'working'])
    expect(rows[1]).toMatchObject({ kind: 'echo', text: 'hello' })
    expect(rows[2]).toMatchObject({ kind: 'working', phrase: 'thinking' })
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
    expect(dividerText(turn('completed', true), clock)).toBe('Finished in 7s · 10:11 AM')
    expect(dividerText(turn('failed', true), clock)).toBe('Failed after 7s · 10:11 AM')
    expect(dividerText(turn('completed', false), clock)).toBe('Turn finished')
    expect(dividerText(turn('interrupted', false), clock)).toBe('Turn stopped')
  })

  it('writes a duration in the largest units that fit', () => {
    expect([0, 7_000, 72_000, 3_900_000].map(formatDuration)).toEqual(['0s', '7s', '1m 12s', '1h 5m'])
  })
})

describe('the paragraphs of a reply that is still arriving', () => {
  it('repairs only the last paragraph', () => {
    expect(streamingParagraphs(['One **done**.', 'Two **no'])).toEqual(['One **done**.', 'Two **no**'])
  })

  it('drops a last paragraph that is only a partial table or marker', () => {
    expect(streamingParagraphs(['Intro.', '| a | b |\n|---'])).toEqual(['Intro.'])
    expect(streamingParagraphs(['Intro.', '-'])).toEqual(['Intro.'])
    expect(streamingParagraphs([])).toEqual([])
  })
})

describe('buildRows with Hide successful tool calls', () => {
  const read = (id: string, over: Parameters<typeof toolEvent>[1] = {}) => toolEvent(id, { target: `${id}.ts`, ...over })
  const kinds = (events: Parameters<typeof buildRows>[0], hide: boolean) => buildRows(events, false, null, null, true, hide).map((row) => row.kind)
  const failedRun = [message('u', 'user', 'go'), commandEvent('c', { status: 'failed', exitCode: 1 }), read('r1'), read('r2'), read('r3'), turn('completed', true)]

  it('merges thoughts that quiet runs stood between, as they would be merged without the runs', () => {
    const rows = buildRows([reasoningEvent('t1', 'One.'), read('r1'), reasoningEvent('t2', 'Two.')], false, null, null, true, true)

    expect(rows).toMatchObject([{ kind: 'thought', key: 't1', texts: ['One.', 'Two.'] }])
  })

  it('keeps a thought between two agent messages as a row of its own, so they stay two messages', () => {
    const rows = buildRows([message('a1', 'agent', 'A.'), read('r1'), reasoningEvent('t1', 'Hm.'), read('g1', { name: 'Grep' }), message('a2', 'agent', 'B.')], false, null, null, true, true)

    expect(rows.map((row) => row.kind)).toEqual(['agent', 'thought', 'agent'])
  })

  it('draws every run while the setting is off', () => {
    expect(kinds([read('r1'), read('r2'), read('r3')], false)).toEqual(['work'])
  })

  it('leaves out a run of reads and searches that all succeeded, and a lone one', () => {
    expect(kinds([message('u', 'user', 'go'), read('r1'), read('r2'), read('r3'), turn('completed', true)], true)).toEqual(['user', 'divider'])
    expect(kinds([message('u', 'user', 'go'), read('g', { name: 'Grep' }), turn('completed', true)], true)).toEqual(['user', 'divider'])
  })

  it('keeps a command that succeeded, since it may have changed files', () => {
    expect(kinds([commandEvent('c', { exitCode: 0 })], true)).toEqual(['step'])
    expect(kinds([read('r1'), commandEvent('c', { exitCode: 0 }), read('r2')], true)).toEqual(['work'])
  })

  it('keeps a tool of unknown kind that succeeded, an MCP tool named like a search among them', () => {
    expect(kinds([read('m', { name: 'mcp__files__write' })], true)).toEqual(['step'])
    expect(kinds([read('m', { name: 'mcp__fs__search_and_replace' })], true)).toEqual(['step'])
    expect(kinds([read('m', { name: 'mcp__x__update_list' })], true)).toEqual(['step'])
  })

  it('keeps a run with a failed command in it, whole', () => {
    const rows = buildRows(failedRun, false, null, null, true, true)

    expect(rows.map((row) => row.kind)).toEqual(['user', 'work', 'divider'])
    expect(rows[1]?.kind === 'work' && rows[1].events.map((event) => event.id)).toEqual(['c', 'r1', 'r2', 'r3'])
  })

  it("keeps a run that edited a file, and the turn's changes row under it", () => {
    expect(kinds([read('r1'), editEvent('e1'), turn('completed', true)], true)).toEqual(['work', 'divider', 'changes'])
  })

  it('keeps a run of reads still running, stopped, or with a failed read', () => {
    expect(kinds([read('r1'), read('r2', { status: 'running' })], true)).toEqual(['work'])
    expect(kinds([read('r1'), read('r2', { status: 'interrupted' })], true)).toEqual(['work'])
    expect(kinds([read('r1'), read('r2', { status: 'failed' })], true)).toEqual(['work'])
  })

  it('keeps the to-do list, which is not a tool call', () => {
    expect(kinds([todoEvent('todo', [{ text: 'Plan', state: 'current' }])], true)).toEqual(['step'])
  })

  it('reads the two parts of a message a hidden run stood between as one message', () => {
    const rows = buildRows([message('a1', 'agent', 'Looking.'), read('r1'), read('r2'), message('a2', 'agent', 'Found it.')], false, null, null, true, true)

    expect(rows).toEqual([{ kind: 'agent', key: 'a1', turnId: 't1', paragraphs: ['Looking.', 'Found it.'], status: 'completed' }])
  })

  it('points an approval at no row when the step it is about was hidden', () => {
    const rows = buildRows([read('r1'), approvalEvent(4, { stepId: 'r1' })], false, null, null, true, true)

    expect(rows).toMatchObject([{ kind: 'approval', step: null }])
  })
})
