import { beforeEach, describe, expect, it } from 'vitest'
import { followComposer, forgetComposers, peekComposer, updateComposer } from './composerStore'

beforeEach(() => { forgetComposers() })

describe('what a message box holds when its session\'s chat gets a new number', () => {
  it('goes with the session to the new chat', () => {
    followComposer('pane-1', 7)
    updateComposer(7, (state) => ({ ...state, draft: 'half a message' }))

    followComposer('pane-1', 9)

    expect(peekComposer(9).draft).toBe('half a message')
    expect(peekComposer(7).draft).toBe('')
  })

  it('stays where it is while the number is the same, and for another session', () => {
    followComposer('pane-1', 7)
    updateComposer(7, (state) => ({ ...state, draft: 'mine' }))

    followComposer('pane-1', 7)
    followComposer('pane-2', 9)

    expect(peekComposer(7).draft).toBe('mine')
    expect(peekComposer(9).draft).toBe('')
  })

  it('does not replace text already written in the new chat', () => {
    followComposer('pane-1', 7)
    updateComposer(7, (state) => ({ ...state, draft: 'old' }))
    updateComposer(9, (state) => ({ ...state, draft: 'new' }))

    followComposer('pane-1', 9)

    expect(peekComposer(9).draft).toBe('new')
  })
})
