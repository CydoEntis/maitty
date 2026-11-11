import { describe, expect, it } from 'vitest'
import type { CommandEvent, EditEvent, ItemStatus, ToolEvent } from './chatEvents'
import { workDurationMs, workStatus, workSummary } from './workSummary'

const base = (id: string, status: ItemStatus = 'completed', startedAtMs: number | null = null, completedAtMs: number | null = null) => ({
  id, turnId: 't', status, observed: { firstSeenAtMs: 0, settledAtMs: 0 }, timing: { startedAtMs, completedAtMs },
})
const tool = (id: string, name: string, target: string | null, status: ItemStatus = 'completed'): ToolEvent => ({ ...base(id, status), kind: 'tool', name, target, input: null, detail: '' })
const edit = (id: string, file: string): EditEvent => ({ ...base(id), kind: 'edit', tool: 'Edit', file, change: 'update', diff: { status: 'unavailable' } })
const command = (id: string, startedAtMs: number | null = null, completedAtMs: number | null = null): CommandEvent => ({ ...base(id, 'completed', startedAtMs, completedAtMs), kind: 'command', command: 'ls', output: '', exitCode: 0, durationMs: null })

describe('a run of steps in one line', () => {
  it('counts files once, and says what kind of work it was', () => {
    const steps = [tool('1', 'Read', 'a.ts'), tool('2', 'Read', 'b.ts'), tool('3', 'Read', 'a.ts'), edit('4', 'a.ts'), edit('5', 'a.ts'), edit('6', 'c.ts'), command('7')]
    expect(workSummary(steps)).toBe('Read 2 files, edited 2, ran 1 command')
  })

  it('names the files when nothing was read first, and counts searches and other tools', () => {
    expect(workSummary([edit('1', 'a.ts'), tool('2', 'Grep', 'x'), tool('3', 'WebFetch', 'u'), tool('4', 'WebFetch', 'v')])).toBe('Searched 1 time, edited 1 file, used 2 tools')
  })

  it('is running while a step is, and failed when one failed', () => {
    expect(workStatus([tool('1', 'Read', 'a', 'failed'), tool('2', 'Read', 'b', 'running')])).toBe('running')
    expect(workStatus([tool('1', 'Read', 'a', 'failed'), tool('2', 'Read', 'b')])).toBe('failed')
    expect(workStatus([tool('1', 'Read', 'a')])).toBe('completed')
  })

  it('times the run only from times the daemon gave', () => {
    expect(workDurationMs([command('1', 1000, 2000), command('2', 3000, 9000)])).toBe(8000)
    expect(workDurationMs([command('1'), command('2', 3000, 9000)])).toBeNull()
  })
})
