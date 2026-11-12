import { describe, expect, it } from 'vitest'
import { conversationKey, machineArgOfKey, splitConversationKey } from './conversationKey'

describe('a conversation key', () => {
  it('is the conversation number itself on this computer', () => {
    expect(conversationKey(0, 7)).toBe(7)
    expect(splitConversationKey(7)).toEqual({ machine: 0, id: 7 })
    expect(machineArgOfKey(7)).toEqual({})
  })

  it('keeps the same number on two machines apart, and splits back', () => {
    const [a, b] = [conversationKey(1, 5), conversationKey(2, 5)]
    expect(a).not.toBe(b)
    expect(a).not.toBe(5)
    expect(splitConversationKey(a ?? -1)).toEqual({ machine: 1, id: 5 })
    expect(machineArgOfKey(b ?? -1)).toEqual({ machine: 2 })
  })

  it('refuses a number too large to keep apart', () => {
    expect(conversationKey(1, 2 ** 32)).toBeNull()
    expect(conversationKey(1, -1)).toBeNull()
    expect(conversationKey(70_000, 1)).toBeNull()
  })
})
