import { beforeEach, describe, expect, it } from 'vitest'
import { EMPTY_LAYOUT, type Layout } from './layout'
import { STORAGE_KEY, STORAGE_KEY_V1, readLayout, writeLayout } from './layoutStorage'

const local: Layout = {
  tabs: [{ id: 'tab1', panes: [{ id: 'p1', sessionId: 't3', col: 0 }, { id: 'p2', sessionId: '42', col: 1 }], focus: 'p1', max: null }],
  activeTabId: 'tab1',
}

const withMachines: Layout = {
  tabs: [{ id: 'tab1', panes: [{ id: 'p1', sessionId: 't3', col: 0 }, { id: 'p2', sessionId: 'build/t9', col: 1 }], focus: 'p2', max: null }],
  activeTabId: 'tab1',
}

beforeEach(() => { localStorage.clear() })

describe('the saved tab layout', () => {
  it('starts empty when nothing was saved', () => {
    expect(readLayout()).toEqual(EMPTY_LAYOUT)
  })

  it('reads a layout saved by v1 as it was and keeps it under v2', () => {
    localStorage.setItem(STORAGE_KEY_V1, JSON.stringify(local))
    expect(readLayout()).toEqual(local)
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY) ?? 'null')).toEqual(local)
  })

  it('leaves the v1 entry for an older app', () => {
    localStorage.setItem(STORAGE_KEY_V1, JSON.stringify(local))
    readLayout()
    writeLayout(withMachines)
    expect(JSON.parse(localStorage.getItem(STORAGE_KEY_V1) ?? 'null')).toEqual(local)
  })

  it('prefers what v2 saved to what v1 did', () => {
    localStorage.setItem(STORAGE_KEY_V1, JSON.stringify(local))
    localStorage.setItem(STORAGE_KEY, JSON.stringify(withMachines))
    expect(readLayout()).toEqual(withMachines)
  })

  it('keeps the sessions of other machines by name through a restart', () => {
    writeLayout(withMachines)
    expect(readLayout()).toEqual(withMachines)
  })

  it('ignores a saved layout it cannot read', () => {
    localStorage.setItem(STORAGE_KEY_V1, '{ not json')
    expect(readLayout()).toEqual(EMPTY_LAYOUT)
  })
})
