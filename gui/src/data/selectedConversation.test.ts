import { beforeEach, describe, expect, it, vi } from 'vitest'
import { readSelectedConversation, writeSelectedConversation } from './selectedConversation'

describe('selected conversation preference', () => {
  beforeEach(() => {
    localStorage.clear()
    vi.restoreAllMocks()
  })

  it('starts without a saved selection', () => {
    expect(readSelectedConversation()).toBeNull()
  })

  it('restores a saved conversation ID', () => {
    writeSelectedConversation('42')

    expect(readSelectedConversation()).toBe('42')
  })

  it('ignores an empty saved ID', () => {
    localStorage.setItem('maitty.gui.selection.v1', '')
    const warning = vi.spyOn(console, 'warn').mockImplementation(() => undefined)

    expect(readSelectedConversation()).toBeNull()
    expect(warning).toHaveBeenCalledOnce()
  })
})
