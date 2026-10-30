import { beforeEach, describe, expect, it } from 'vitest'
import { readConversationDrafts, writeConversationDrafts } from './conversationDrafts'

describe('conversation drafts', () => {
  beforeEach(() => {
    localStorage.clear()
  })

  it('loads no drafts when none have been saved', () => {
    expect(readConversationDrafts()).toEqual({})
  })

  it('keeps drafts associated with their conversation', () => {
    const drafts = { 'thread-a': 'Review the changes', 'thread-b': 'Check the build' }
    writeConversationDrafts(drafts)

    expect(readConversationDrafts()).toEqual(drafts)
  })

  it('ignores corrupt draft data', () => {
    localStorage.setItem('maitty.gui.drafts.v1', '{broken')

    expect(readConversationDrafts()).toEqual({})
  })

  it('removes a draft after it is cleared', () => {
    writeConversationDrafts({ 'thread-a': 'Review the changes' })
    writeConversationDrafts({ 'thread-a': '' })

    expect(readConversationDrafts()).toEqual({})
  })
})
