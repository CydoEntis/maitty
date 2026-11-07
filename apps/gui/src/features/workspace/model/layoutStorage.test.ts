import { beforeEach, describe, expect, it } from 'vitest'
import { EMPTY_LAYOUT, openArrangement, type MakeId } from './layout'
import { LAYOUT_RESET_NOTICE, readLayout, STORAGE_KEY, writeLayout } from './layoutStorage'

const makeId: MakeId = (prefix) => `${prefix}-1`

describe('saved layout', () => {
  beforeEach(() => { localStorage.clear() })

  it('starts empty on a first launch, with nothing to say', () => {
    expect(readLayout()).toEqual({ layout: EMPTY_LAYOUT, reset: false })
  })

  it('reads back what it wrote', () => {
    const layout = openArrangement(EMPTY_LAYOUT, 't4', makeId)
    writeLayout(layout)
    expect(readLayout()).toEqual({ layout, reset: false })
  })

  it('drops a tab layout from an older version and says so once', () => {
    localStorage.setItem('maitty.gui.layout.v2', JSON.stringify({ tabs: [], activeTabId: null }))
    expect(readLayout()).toEqual({ layout: EMPTY_LAYOUT, reset: true })
    expect(localStorage.getItem('maitty.gui.layout.v2')).toBeNull()
    writeLayout(EMPTY_LAYOUT)
    expect(readLayout().reset).toBe(false)
    expect(localStorage.getItem(STORAGE_KEY)).not.toBeNull()
  })

  it('does not reset a layout that is already the new kind', () => {
    localStorage.setItem('maitty.gui.layout.v2', '{}')
    writeLayout(EMPTY_LAYOUT)
    expect(readLayout().reset).toBe(false)
  })

  it('ignores a saved layout it cannot read', () => {
    localStorage.setItem(STORAGE_KEY, '{ not json')
    expect(readLayout()).toEqual({ layout: EMPTY_LAYOUT, reset: false })
  })

  it('names the reset in words', () => {
    expect(LAYOUT_RESET_NOTICE).toMatch(/reset/)
  })
})
