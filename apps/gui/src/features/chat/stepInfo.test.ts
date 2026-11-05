import { describe, expect, it } from 'vitest'
import { agentsSummary, clockText, describeStep, elapsedMs, relativePath, todoProgress, tookText } from './stepInfo'
import { commandEvent, editEvent, fields, subagentEvent, todoEvent, toolEvent } from './eventFixtures'

describe('elapsedMs', () => {
  const running = { ...fields('a', 'running'), observed: { firstSeenAtMs: 4_000, settledAtMs: null } }
  const timed = { ...fields('b'), timing: { startedAtMs: 1_000, completedAtMs: 3_500 } }

  it.each([
    ['a running item counts from the daemon start', { ...running, timing: { startedAtMs: 2_000, completedAtMs: null } }, null, 10_000, 8_000],
    ['a running item with no daemon start counts from when it was first seen', running, null, 10_000, 6_000],
    ['a finished item prefers the provider\'s own duration', timed, 1_500, 99_999, 1_500],
    ['a finished item otherwise uses the daemon times', timed, null, 99_999, 2_500],
    ['a finished item from saved history has no time', fields('c'), null, 99_999, null],
  ])('%s', (_name, item, duration, now, expected) => {
    expect(elapsedMs(item, duration, now)).toBe(expected)
  })
})

describe('time words', () => {
  it.each([[0, '0:00'], [61_000, '1:01'], [600_000, '10:00']])('clock %i -> %s', (ms, text) => { expect(clockText(ms)).toBe(text) })
  it.each([[400, '0.4s'], [7_000, '7s'], [330_000, '5m 30s']])('took %i -> %s', (ms, text) => { expect(tookText(ms)).toBe(text) })
})

describe('relativePath', () => {
  it.each([
    ['/repo/src/a.ts', '/repo', 'src/a.ts'],
    ['/repo/src/a.ts', '/repo/', 'src/a.ts'],
    ['C:\\Work\\app\\src\\a.ts', 'c:\\work\\app', 'src/a.ts'],
    ['/other/a.ts', '/repo', '/other/a.ts'],
    ['/repository/a.ts', '/repo', '/repository/a.ts'],
    ['/Repo/a.ts', '/repo', '/Repo/a.ts'],
    ['src/a.ts', '/repo', 'src/a.ts'],
    ['/repo/a.ts', null, '/repo/a.ts'],
  ])('%s under %s -> %s', (file, root, expected) => {
    expect(relativePath(file, root)).toBe(expected)
  })
})

describe('step descriptions and summaries', () => {
  it('names a step the way a link to it reads', () => {
    expect(describeStep(editEvent('e', { file: '/repo/src/a.ts' }), '/repo')).toBe('Edit src/a.ts')
    expect(describeStep(commandEvent('c', { command: 'pnpm test' }), null)).toBe('pnpm test')
    expect(describeStep(toolEvent('t', { name: 'Read', target: 'a.ts' }), null)).toBe('Read a.ts')
    expect(describeStep(toolEvent('t2', { name: 'Glob' }), null)).toBe('Glob')
    expect(describeStep(subagentEvent('s', { title: null }), null)).toBe('A helper agent')
  })

  it('counts checklist progress and helper agents', () => {
    const list = todoEvent('d', [{ text: 'a', state: 'done' }, { text: 'b', state: 'current' }, { text: 'c', state: 'pending' }])
    expect(todoProgress(list.items ?? [])).toEqual({ done: 1, total: 3 })
    expect(agentsSummary([subagentEvent('1', { status: 'running' }), subagentEvent('2')])).toBe('2 agents, 1 working')
    expect(agentsSummary([subagentEvent('1')])).toBe('1 agent, all done')
  })
})
