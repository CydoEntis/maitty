import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { isBangLine } from './slashCommands'
import { typeMessage } from './useChatComposer'

describe('a shell line starting with !', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('is a bang line when it is one line with a command after the !', () => {
    expect(isBangLine('!ls')).toBe(true)
    expect(isBangLine('! gh auth refresh -h github.com -s delete_repo')).toBe(true)
    expect(isBangLine('!')).toBe(false)
    expect(isBangLine('! ')).toBe(false)
    expect(isBangLine('!ls\nmore')).toBe(false)
    expect(isBangLine('hello !ls')).toBe(false)
  })

  it('is typed for Claude as a ! and then the command, not pasted', async () => {
    const typed: string[] = []
    const done = typeMessage((text) => { typed.push(text); return Promise.resolve() }, 'claude', '! gh auth status')
    await vi.runAllTimersAsync()
    await done
    expect(typed).toEqual(['!', 'gh auth status\r'])
  })

  it('stays an ordinary message for an agent with no shell mode', async () => {
    const typed: string[] = []
    await typeMessage((text) => { typed.push(text); return Promise.resolve() }, 'codex', '!ls')
    expect(typed).toHaveLength(1)
    expect(typed[0]).toContain('!ls')
    expect(typed[0]).toContain('\x1b[200~')
  })
})
