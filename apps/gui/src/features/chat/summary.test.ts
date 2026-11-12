import { describe, expect, it } from 'vitest'
import type { ChatRow } from './chatRows'
import { summaryOf } from './summary'

const user = (key: string, text: string) => ({ kind: 'user', key, event: { kind: 'message', role: 'user', text } }) as unknown as ChatRow
const agent = (key: string, paragraphs: string[]) => ({ kind: 'agent', key, turnId: 't', paragraphs, status: 'completed' }) as unknown as ChatRow

describe('a summary to continue a chat from', () => {
  it('names the task it began with, the latest asks, and the last reply', () => {
    const text = summaryOf([user('1', 'Add a rate limiter'), agent('2', ['Done, see limiter.rs']), user('3', 'Now add tests'), agent('4', ['Tests are in', 'All green'])])
    expect(text).toContain('The task it began with:\nAdd a rate limiter')
    expect(text).toContain('- Now add tests')
    expect(text).toContain('Where it stood (your last reply there):\nTests are in\n\nAll green')
  })

  it('keeps only the latest few asks and clips long text', () => {
    const rows = ['a', 'b', 'c', 'd', 'e', 'f', 'g'].map((name) => user(name, name.repeat(2)))
    const text = summaryOf(rows)
    expect(text).not.toContain('- bb')
    expect(text).toContain('- gg')
    expect(summaryOf([user('1', 'x'.repeat(2000))])).toContain('…')
  })

  it('is still a usable note for a chat with nothing in it', () => {
    expect(summaryOf([])).toContain('pick up from here')
  })
})
