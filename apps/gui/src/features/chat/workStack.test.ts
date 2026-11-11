import { describe, expect, it } from 'vitest'
import { commandEvent, editEvent, toolEvent } from './eventFixtures'
import { stackCards, stackLines, stepsBeyondStack } from './workStack'

describe('the stack under a run of steps', () => {
  it('holds the latest three commands and edits, newest first, and never a read or a search', () => {
    const steps = [commandEvent('1'), toolEvent('2'), editEvent('3'), commandEvent('4'), toolEvent('5', { name: 'Grep' }), editEvent('6')]

    expect(stackCards(steps).map((step) => step.id)).toEqual(['6', '4', '3'])
  })

  it('is empty for a run of only reads and searches', () => {
    expect(stackCards([toolEvent('1'), toolEvent('2', { name: 'Grep' })])).toEqual([])
  })

  it('counts the steps without a card only once a command or edit is left out', () => {
    expect(stepsBeyondStack([toolEvent('1'), commandEvent('2'), editEvent('3')])).toBe(0)
    expect(stepsBeyondStack([commandEvent('1'), toolEvent('2'), editEvent('3'), commandEvent('4'), editEvent('5')])).toBe(2)
  })

  it('shows the end of a command’s output and the first changed lines of an edit', () => {
    expect(stackLines(commandEvent('c', { output: 'one\n\ntwo\nthree\n' })).map((line) => line.text)).toEqual(['two', 'three'])
    expect(stackLines(editEvent('e'))).toEqual([
      { mark: '−', text: 'old', tone: 'remove' },
      { mark: '+', text: 'new', tone: 'add' },
    ])
  })

  it('shows nothing for output the history withheld, or a diff it did not keep', () => {
    expect(stackLines(commandEvent('c', { output: '[content of a secret file omitted]' }))).toEqual([])
    expect(stackLines(editEvent('e', { diff: { status: 'omitted' } }))).toEqual([])
  })
})
