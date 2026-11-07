import { describe, expect, it } from 'vitest'
import { addCrew, addMate, addToCrew, CAPTAIN_ID, chatSubtitle, initialCrew, needsCount, notAboard, receive, replier, select, send } from './crew'

const NOON = new Date(2026, 10, 7, 12, 5)

describe('who answers a message', () => {
  it('is the crewmate whose chat it is', () => {
    expect(replier(initialCrew(), 'm2', 'hello')).toBe('m2')
  })

  it('is the agent a crew message @names, matched by the start of its name', () => {
    expect(replier(initialCrew(), 'c1', 'can @rev look at this')).toBe('m2')
    expect(replier(initialCrew(), 'c1', '@sam’s please')).toBe('samd')
  })

  it('takes each agent of the crew in turn when nobody is named, and never a person', () => {
    const state = initialCrew()
    const first = replier(state, 'c1', 'anyone?')
    expect(['cap', 'm2', 'samd']).toContain(first)
    expect(replier(state, 'c1', '@sam')).not.toBe('sam')
  })
})

describe('sending and receiving', () => {
  it('puts your message in the chat and starts the replier typing', () => {
    const next = send(initialCrew(), 'm1', '  Summarise it  ', NOON)
    expect(next.lines.m1?.at(-1)).toEqual({ kind: 'message', from: 'you', text: 'Summarise it', time: '12:05' })
    expect(next.typing).toEqual({ chat: 'm1', who: 'm1' })
  })

  it('ignores a blank message', () => {
    const state = initialCrew()
    expect(send(state, 'm1', '   ', NOON)).toBe(state)
  })

  it('adds the answer and clears the typing mark', () => {
    const answered = receive(send(initialCrew(), 'm3', 'go ahead', NOON), NOON)
    expect(answered.typing).toBeNull()
    expect(answered.lines.m3?.at(-1)).toMatchObject({ kind: 'message', from: 'm3' })
  })

  it('does nothing when nobody is typing', () => {
    const state = initialCrew()
    expect(receive(state, NOON)).toBe(state)
  })
})

describe('opening a chat', () => {
  it('selects it and clears its unread count', () => {
    const state = select(initialCrew(), 'm3')
    expect(state.selected).toBe('m3')
    expect(state.mates.find((mate) => mate.id === 'm3')?.unread).toBe(0)
    expect(select(initialCrew(), 'c1').crews.find((crew) => crew.id === 'c1')?.unread).toBe(0)
  })

  it('counts the crewmates waiting on you', () => {
    expect(needsCount(initialCrew())).toBe(1)
  })
})

describe('adding crewmates and crews', () => {
  it('adds a crewmate, trims its name, fills in a missing role and opens its chat', () => {
    const next = addMate(initialCrew(), { name: '  Scribe ', role: '', agent: 'codex' })
    const added = next.mates.at(-1)
    expect(added).toMatchObject({ name: 'Scribe', agent: 'codex', role: 'Works with your Captain', status: 'idle' })
    expect(next.selected).toBe(added?.id)
  })

  it('adds nothing for a blank name', () => {
    const state = initialCrew()
    expect(addMate(state, { name: ' ', role: 'x', agent: 'claude' })).toBe(state)
    expect(addCrew(state, '')).toBe(state)
  })

  it('starts a crew with the Captain aboard and opens it', () => {
    const next = addCrew(initialCrew(), 'Ops crew')
    expect(next.crews.at(-1)).toMatchObject({ name: 'Ops crew', members: [CAPTAIN_ID] })
    expect(next.selected).toBe(next.crews.at(-1)?.id)
  })

  it('offers only crewmates that are not aboard, and adds one once', () => {
    const state = initialCrew()
    expect(notAboard(state, 'c2').map((mate) => mate.id)).toEqual(['m2', 'm3'])
    const added = addToCrew(state, 'c2', 'm3')
    expect(notAboard(added, 'c2').map((mate) => mate.id)).toEqual(['m2'])
    expect(addToCrew(added, 'c2', 'm3')).toBe(added)
  })
})

describe('chat subtitles', () => {
  it('names a crewmate’s job and agent, and a crew’s size and people', () => {
    const state = initialCrew()
    expect(chatSubtitle(state, 'm2')).toBe('Reviews diffs before they merge · Codex')
    expect(chatSubtitle(state, 'c1')).toBe('4 aboard · 1 person')
    expect(chatSubtitle(state, 'c2')).toBe('2 aboard · 0 people')
  })
})
