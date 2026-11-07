import { describe, expect, it } from 'vitest'
import { fleetsFromSnapshot } from './fleets'

const leaf = (term: number) => ({ Leaf: term })
const split = (a: unknown, b: unknown) => ({ Split: { horizontal: true, ratio: 0.5, a, b } })

function workspace(id: number, name: string, tabs: unknown[], extra: Record<string, unknown> = {}) {
  return { id, name, cwd: '/home/me', active_tab: 1, git: null, worktree: false, color: 2, is_new: false, group: null, tabs: tabs.map((layout, index) => ({ id: index + 1, name: 't', layout, focus: 0 })), ...extra }
}

describe('the fleets in a snapshot', () => {
  it('reads each workspace as a fleet with the sessions its tabs hold, in the order their panes were made', () => {
    const fleets = fleetsFromSnapshot({ workspaces: [workspace(1, 'Maitty', [split(leaf(7), leaf(3)), leaf(5)]), workspace(2, 'Docs', [leaf(9)], { worktree: true })] })
    expect(fleets).toEqual([
      { key: '0:1', machine: 0, id: 1, name: 'Maitty', color: 2, worktree: false, sessionIds: ['t3', 't5', 't7'] },
      { key: '0:2', machine: 0, id: 2, name: 'Docs', color: 2, worktree: true, sessionIds: ['t9'] },
    ])
  })

  it('names another machine in its fleets, keys and session ids, so two machines never meet', () => {
    const [fleet] = fleetsFromSnapshot({ workspaces: [workspace(1, 'build', [leaf(4)])] }, { id: 3, name: 'devbox' })
    expect(fleet).toMatchObject({ key: '3:1', machine: 3, sessionIds: ['devbox/t4'] })
  })

  it('leaves out a workspace it cannot read, and keeps the rest', () => {
    const fleets = fleetsFromSnapshot({ workspaces: [{ id: 'x' }, workspace(2, 'Docs', [leaf(1)])] })
    expect(fleets.map((fleet) => fleet.name)).toEqual(['Docs'])
  })

  it('reads a snapshot with no workspaces as no fleets', () => {
    expect(fleetsFromSnapshot({})).toEqual([])
  })
})
