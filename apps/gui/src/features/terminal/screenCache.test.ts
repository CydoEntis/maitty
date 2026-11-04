import { beforeEach, describe, expect, it } from 'vitest'
import { appendToScreen, cachedScreen, clearScreens, MAX_SCREEN_BYTES, MAX_SCREENS, replaceScreen, screenKey } from './screenCache'

const bytes = (text: string) => new TextEncoder().encode(text)
const text = (value: Uint8Array | undefined) => new TextDecoder().decode(value)

beforeEach(clearScreens)

describe('screen cache', () => {
  it('keeps a replay and the output after it, and a new replay replaces both', () => {
    replaceScreen('a', 3, bytes('screen '))
    appendToScreen('a', 4, bytes('more'))
    expect(cachedScreen('a')?.seq).toBe(4)
    expect(text(cachedScreen('a')?.bytes)).toBe('screen more')

    replaceScreen('a', 9, bytes('fresh'))
    expect(text(cachedScreen('a')?.bytes)).toBe('fresh')
  })

  it('ignores output for a pane it holds no screen of', () => {
    appendToScreen('none', 1, bytes('x'))
    expect(cachedScreen('none')).toBeNull()
  })

  it('bounds each pane and never keeps a truncated sequence', () => {
    replaceScreen('big', 1, new Uint8Array(MAX_SCREEN_BYTES))
    appendToScreen('big', 2, bytes('x'))
    expect(cachedScreen('big')?.seq).toBe(1)
    replaceScreen('huge', 1, new Uint8Array(MAX_SCREEN_BYTES + 1))
    expect(cachedScreen('huge')).toBeNull()
  })

  it('forgets the least recently used pane past the pane bound', () => {
    for (let pane = 0; pane < MAX_SCREENS; pane += 1) replaceScreen(screenKey(0, pane, 0), 1, bytes('s'))
    cachedScreen(screenKey(0, 0, 0))
    replaceScreen(screenKey(0, MAX_SCREENS, 0), 1, bytes('s'))
    expect(cachedScreen(screenKey(0, 1, 0))).toBeNull()
    expect(cachedScreen(screenKey(0, 0, 0))).not.toBeNull()
  })
})
