import { describe, expect, it } from 'vitest'
import type { ChatRow } from './chatRows'
import { chatFindOf, findRow, registerChatFind } from './chatSearch'

const echo = (key: string, text: string): ChatRow => ({ kind: 'echo', key, text })
const agent = (key: string, ...paragraphs: string[]): ChatRow => ({ kind: 'agent', key, turnId: 't', paragraphs, status: 'completed' })

const rows = [echo('a', 'fix the Parser'), agent('b', 'I looked at it.', 'The parser drops the last row.'), echo('c', 'thanks'), agent('d', 'Done with the parser.')]

describe('finding text in a chat', () => {
  it('starts at the newest row that holds the text, whatever its letter case', () => {
    expect(findRow(rows, 'PARSER', null, 'next')).toBe('d')
    expect(findRow(rows, 'parser', null, 'previous')).toBe('d')
  })

  it('goes up to the row before, and round to the newest again', () => {
    expect(findRow(rows, 'parser', 'd', 'previous')).toBe('b')
    expect(findRow(rows, 'parser', 'b', 'previous')).toBe('a')
    expect(findRow(rows, 'parser', 'a', 'previous')).toBe('d')
  })

  it('goes down to the row after, and round to the oldest again', () => {
    expect(findRow(rows, 'parser', 'a', 'next')).toBe('b')
    expect(findRow(rows, 'parser', 'd', 'next')).toBe('a')
  })

  it('finds the only match again from itself', () => {
    expect(findRow(rows, 'thanks', 'c', 'next')).toBe('c')
  })

  it('finds nothing for text no row holds, a blank search or an empty chat', () => {
    expect(findRow(rows, 'lexer', null, 'next')).toBeNull()
    expect(findRow(rows, '   ', null, 'next')).toBeNull()
    expect(findRow([], 'parser', null, 'next')).toBeNull()
  })
})

describe('the chat a session shows', () => {
  it('is searchable while it is on screen, and not after it has gone', () => {
    const find = { find: () => true, clear: () => undefined }
    const gone = registerChatFind('s1', find)
    expect(chatFindOf('s1')).toBe(find)

    gone()
    expect(chatFindOf('s1')).toBeNull()
  })

  it('stays with the chat that took its place when the old one goes late', () => {
    const old = { find: () => false, clear: () => undefined }
    const current = { find: () => true, clear: () => undefined }
    const oldGone = registerChatFind('s1', old)
    registerChatFind('s1', current)

    oldGone()
    expect(chatFindOf('s1')).toBe(current)
  })
})
