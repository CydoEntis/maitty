import { describe, expect, it } from 'vitest'
import type { ChatEvent, ItemFields } from './chatEvents'
import { countUserMessages, describeWork } from './workPhase'

const fields = (id: string, status: ItemFields['status'] = 'running'): ItemFields => ({
  id,
  turnId: 't1',
  status,
  observed: { firstSeenAtMs: 0 } as ItemFields['observed'],
  timing: { startedAtMs: null, completedAtMs: null },
})

const tool = (id: string, name: string, target: string | null, status: ItemFields['status'] = 'running'): ChatEvent =>
  ({ ...fields(id, status), kind: 'tool', name, target, input: null, detail: '' })
const command = (id: string, text: string): ChatEvent =>
  ({ ...fields(id), kind: 'command', command: text, output: '', exitCode: null, durationMs: null })
const edit = (id: string, file: string | null, toolName: string, change: 'add' | 'update'): ChatEvent =>
  ({ ...fields(id), kind: 'edit', tool: toolName, file, change, diff: { status: 'unavailable' } })
const user = (id: string, text: string): ChatEvent => ({ ...fields(id, 'completed'), kind: 'message', role: 'user', text })

describe('what the working line says', () => {
  it('says thinking when no step is running', () => {
    expect(describeWork([], null)).toBe('thinking')
    expect(describeWork([tool('a', 'Read', 'src/a.ts', 'completed')], null)).toBe('thinking')
  })

  it('names what a running tool is doing and what it acts on', () => {
    expect(describeWork([tool('a', 'Read', '/work/src/app.ts')], '/work')).toBe('reading src/app.ts')
    expect(describeWork([tool('a', 'Grep', 'TODO')], null)).toBe('searching TODO')
    expect(describeWork([tool('a', 'Frobnicate', null)], null)).toBe('using Frobnicate')
  })

  it('says writing for a new file and editing for a change to one', () => {
    expect(describeWork([edit('a', '/work/hello.txt', 'Write', 'add')], '/work')).toBe('writing hello.txt')
    expect(describeWork([edit('a', '/work/a.ts', 'Edit', 'update')], '/work')).toBe('editing a.ts')
  })

  it('shows the first line of a command, cut short', () => {
    expect(describeWork([command('a', 'npm test\nthen more')], null)).toBe('running npm test')
    expect(describeWork([command('a', 'x'.repeat(80))], null)).toBe(`running ${'x'.repeat(47)}…`)
  })

  it('reports the newest step still running, not an older one', () => {
    const events = [tool('a', 'Read', 'old.ts'), tool('b', 'Read', 'new.ts')]
    expect(describeWork(events, null)).toBe('reading new.ts')
  })
})

describe('telling a message just sent from the same words said before', () => {
  it('counts only the person\'s messages with exactly those words', () => {
    const events = [user('a', 'hello'), user('b', ' hello '), user('c', 'hello there')]
    expect(countUserMessages(events, 'hello')).toBe(2)
    expect(countUserMessages(events, 'goodbye')).toBe(0)
  })
})
