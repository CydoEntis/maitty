import { describe, expect, it } from 'vitest'
import { buildRows, paragraphsOf, splitLead } from './chatRows'
import { approvalEvent, commandEvent, subagentEvent, toolEvent } from './eventFixtures'

describe('helper agents and approval links', () => {
  it('groups helper agents that follow each other into one card keyed by the first', () => {
    const rows = buildRows([subagentEvent('h1'), subagentEvent('h2'), commandEvent('c'), subagentEvent('h3')], false)

    expect(rows.map((row) => row.kind)).toEqual(['agents', 'step', 'agents'])
    expect(rows[0]).toMatchObject({ key: 'h1', events: [{ id: 'h1' }, { id: 'h2' }] })
  })

  it.each([
    ['a command step', [commandEvent('t1:c', { command: 'pnpm test' })], 't1:c', 'pnpm test'],
    ['a tool step', [toolEvent('t1:r', { name: 'Read', target: 'src/a.ts' })], 't1:r', 'Read src/a.ts'],
    ['a helper agent in a group', [subagentEvent('t1:h1'), subagentEvent('t1:h2', { title: 'Second' })], 't1:h2', 'Second'],
    ['the first file of a patch of several', [toolEvent('t1:p#0', { name: 'p' })], 't1:p', 'p'],
  ] as const)('points an approval at %s', (_name, steps, stepId, label) => {
    const rows = buildRows([...steps, approvalEvent(1, { stepId })], false)
    const approval = rows.find((row) => row.kind === 'approval')

    expect(approval).toMatchObject({ step: { label } })
    expect(approval?.kind === 'approval' && approval.step !== null && rows.some((row) => row.key === approval.step?.key)).toBe(true)
  })

  it('leaves an approval unlinked when its step is not in the conversation', () => {
    const rows = buildRows([approvalEvent(1, { stepId: 't1:missing' })], false)

    expect(rows[0]).toMatchObject({ kind: 'approval', step: null })
  })
})

describe('code fences stay whole', () => {
  it('keeps blank lines inside a fenced block and lets an open fence run to the end', () => {
    expect(paragraphsOf('Run this:\n\n```sh\na\n\nb\n```\n\nDone.')).toEqual(['Run this:', '```sh\na\n\nb\n```', 'Done.'])
    expect(paragraphsOf('```\nopen\n\nstill open')).toEqual(['```\nopen\n\nstill open'])
  })

  it('takes no lead sentence from a code block, list or heading', () => {
    expect(splitLead(['```sh\nls. now\n```'])).toEqual({ lead: '', rest: ['```sh\nls. now\n```'] })
    expect(splitLead(['- one. two'])).toEqual({ lead: '', rest: ['- one. two'] })
    expect(splitLead(['# Title. More'])).toEqual({ lead: '', rest: ['# Title. More'] })
  })
})
