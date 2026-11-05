import { describe, expect, it } from 'vitest'
import { checklistOf, currentItemSince, emptyReason, helperAgentsOf, touchedFilesOf } from './panelModel'
import { commandEvent, editEvent, subagentEvent, toolEvent, todoEvent } from '@/features/chat/eventFixtures'

const done = (text: string) => ({ text, state: 'done' as const })
const current = (text: string) => ({ text, state: 'current' as const })
const pending = (text: string) => ({ text, state: 'pending' as const })
const at = (event: ReturnType<typeof todoEvent>, startedAtMs: number) => ({ ...event, timing: { startedAtMs, completedAtMs: null } })

describe('the checklist', () => {
  it('is the last list that carried its items, with its done count', () => {
    const events = [todoEvent('a', [pending('one'), pending('two')]), commandEvent('c'), todoEvent('b', [done('one'), current('two'), pending('three')])]

    const { checklist } = checklistOf(events)

    expect(checklist?.event.id).toBe('b')
    expect(checklist?.done).toBe(1)
    expect(checklist?.items.map((item) => item.text)).toEqual(['one', 'two', 'three'])
  })

  it('skips a later list whose items the history did not keep, rather than showing nothing', () => {
    const { checklist, unreadable } = checklistOf([todoEvent('a', [done('one')]), todoEvent('b', null)])

    expect(checklist?.event.id).toBe('a')
    expect(unreadable).toBe(false)
  })

  it('says the items were not kept when no list has any', () => {
    expect(checklistOf([todoEvent('a', null)])).toEqual({ checklist: null, unreadable: true })
    expect(checklistOf([commandEvent('c')])).toEqual({ checklist: null, unreadable: false })
  })

  it('dates the current item from the first list in an unbroken run that named it current', () => {
    const events = [
      at(todoEvent('a', [current('one'), pending('two')]), 1_000),
      at(todoEvent('b', [done('one'), current('two')]), 2_000),
      at(todoEvent('c', [done('one'), current('two'), pending('three')]), 3_000),
    ]
    const { checklist } = checklistOf(events)

    expect(checklist && currentItemSince(events, checklist)).toBe(2_000)
  })

  it('restarts the run when the current item was something else in between', () => {
    const events = [
      at(todoEvent('a', [current('two')]), 1_000),
      at(todoEvent('b', [done('two'), current('three')]), 2_000),
      at(todoEvent('c', [current('two'), pending('three')]), 3_000),
    ]
    const { checklist } = checklistOf(events)

    expect(checklist && currentItemSince(events, checklist)).toBe(3_000)
  })

  it('has no time when nothing is current', () => {
    const events = [todoEvent('a', [done('one')])]
    const { checklist } = checklistOf(events)

    expect(checklist && currentItemSince(events, checklist)).toBeNull()
  })
})

describe('helper agents and touched files', () => {
  it('lists the helper agents in order', () => {
    expect(helperAgentsOf([subagentEvent('h1'), commandEvent('c'), subagentEvent('h2')]).map((agent) => agent.id)).toEqual(['h1', 'h2'])
  })

  it('lists files once, in first-touched order, summing edits and letting an edit outrank a read', () => {
    const files = touchedFilesOf([
      toolEvent('r1', { name: 'Read', target: 'a.ts' }),
      toolEvent('r2', { name: 'Read', target: 'b.ts' }),
      editEvent('e1', { file: 'a.ts' }),
      editEvent('e2', { file: 'a.ts' }),
      editEvent('e3', { file: 'new.ts', change: 'add', diff: { status: 'available', hunks: [], added: 4, removed: 0, truncated: false } }),
      editEvent('e4', { file: 'new.ts' }),
      toolEvent('r3', { name: 'Grep', target: 'x' }),
    ])

    expect(files).toEqual([
      { file: 'a.ts', kind: 'edited', added: 2, removed: 2 },
      { file: 'b.ts', kind: 'read', added: 0, removed: 0 },
      { file: 'new.ts', kind: 'created', added: 5, removed: 1 },
    ])
  })

  it('counts nothing for an edit whose diff the history did not keep', () => {
    expect(touchedFilesOf([editEvent('e', { file: 'a.ts', diff: { status: 'unavailable' } })])).toEqual([{ file: 'a.ts', kind: 'edited', added: 0, removed: 0 }])
  })
})

describe('why there is nothing to track', () => {
  it.each([
    [{ kind: 'shell' } as const, /shell session/],
    [{ kind: 'unsupported', agent: 'gemini' } as const, /gemini has no event stream/],
    [{ kind: 'remote', agent: 'claude', machineName: 'devbox' } as const, /runs on devbox.*local-only/],
    [{ kind: 'no-events', agent: 'codex' } as const, /no conversation for this codex session/],
  ])('says it for %j', (availability, expected) => {
    expect(emptyReason(availability)).toMatch(expected)
  })
})
