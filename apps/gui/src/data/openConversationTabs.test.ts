import { beforeEach, describe, expect, it, vi } from 'vitest'
import {
  closeConversationTab,
  openConversationTab,
  readOpenConversationTabs,
  reconcileConversationTabs,
  writeOpenConversationTabs,
} from './openConversationTabs'

describe('open conversation tabs', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.restoreAllMocks()
  })

  it('persists the ordered open conversations', () => {
    writeOpenConversationTabs(['first', 'second'])

    expect(readOpenConversationTabs()).toEqual(['first', 'second'])
  })

  it('ignores invalid data and duplicate or empty ids', () => {
    const warning = vi
      .spyOn(console, 'warn')
      .mockImplementation(() => undefined)
    localStorage.setItem('maitty.gui.tabs.v4', 'null')
    expect(readOpenConversationTabs()).toEqual([])

    localStorage.setItem('maitty.gui.tabs.v4', '["first","first",""]')
    expect(readOpenConversationTabs()).toEqual(['first'])
    expect(warning).toHaveBeenCalledOnce()
  })

  it('opens a conversation once and preserves its position when selected again', () => {
    expect(openConversationTab(['first', 'second'], 'first', 'second')).toEqual(
      ['first', 'second'],
    )
  })

  it('inserts a newly selected conversation after the current tab', () => {
    expect(openConversationTab(['first', 'third'], 'second', 'first')).toEqual([
      'first',
      'second',
      'third',
    ])
  })

  it('drops sessions that no longer exist and adds the selected session', () => {
    expect(
      reconcileConversationTabs(
        ['missing', 'second', 'second'],
        ['first', 'second'],
        'first',
      ),
    ).toEqual(['second', 'first'])
  })

  it('selects the next tab when the active tab closes without stopping its session', () => {
    expect(
      closeConversationTab(['first', 'second', 'third'], 'second', 'second'),
    ).toEqual({ tabIds: ['first', 'third'], selectedId: 'third' })
  })

  it('keeps the selected session when another tab closes and never closes the last tab', () => {
    expect(
      closeConversationTab(['first', 'second'], 'second', 'first'),
    ).toEqual({
      tabIds: ['second'],
      selectedId: 'second',
    })
    expect(closeConversationTab(['second'], 'second', 'second')).toEqual({
      tabIds: ['second'],
      selectedId: 'second',
    })
  })
})
