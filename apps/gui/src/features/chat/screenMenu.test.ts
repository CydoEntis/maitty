import { describe, expect, it } from 'vitest'
import { keysToPick, parseScreenMenu } from './screenMenu'

const TRUST = 'Accessing workspace:\n\n/home/vox\n\nQuick safety check: do you trust this folder?\n❯ 1. No, exit\n2. Yes, I trust this folder'

describe('the menu an agent is stopped on', () => {
  it('reads the question, the choices and where the cursor is', () => {
    const menu = parseScreenMenu(TRUST)
    expect(menu).toEqual({ question: expect.stringContaining('Quick safety check') as string, options: ['No, exit', 'Yes, I trust this folder'], cursor: 0 })
  })

  it('has no menu when the text holds fewer than two choices', () => {
    expect(parseScreenMenu('Allow this?')).toBeNull()
    expect(parseScreenMenu('only\n1. one choice')).toBeNull()
  })

  it('moves the cursor down to the wanted choice, then presses Enter', () => {
    const menu = parseScreenMenu(TRUST)
    if (menu === null) throw new Error('no menu')
    const keys = keysToPick(menu, 1)
    expect(keys).toHaveLength(2)
    expect(keys[0]).toBe(String.fromCharCode(27) + '[B')
    expect(keys[1]).toBe(String.fromCharCode(13))
    expect(keysToPick(menu, 0)).toEqual([String.fromCharCode(13)])
  })

  it('moves up when the cursor is below the wanted choice', () => {
    const menu = parseScreenMenu('Trust?\n1. Yes\n❯ 2. No')
    if (menu === null) throw new Error('no menu')
    expect(menu.cursor).toBe(1)
    expect(keysToPick(menu, 0)[0]).toBe(String.fromCharCode(27) + '[A')
  })
})
