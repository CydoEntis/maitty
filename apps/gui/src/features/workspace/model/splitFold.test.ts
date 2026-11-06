import { describe, expect, it } from 'vitest'
import type { SessionView } from './session'
import { foldSplits, splitNote } from './splitFold'

function session(id: string, process: string, status: SessionView['status']): SessionView {
  return { id, process, status } as SessionView
}

describe('folding the quiet shell splits of a tab', () => {
  const lead = session('a', 'claude', 'working')
  const quiet = session('b', 'shell', 'idle')
  const agent = session('c', 'codex', 'idle')
  const busy = session('d', 'shell', 'working')
  const tab = [lead, quiet, agent, busy]
  const panes = new Map(tab.map((item) => [item.id, tab]))

  it('folds an idle shell split into the first pane and counts it, leaving agents and busy shells', () => {
    const { visible, folded, leadOf } = foldSplits(tab, panes)
    expect(visible.map((item) => item.id)).toEqual(['a', 'c', 'd'])
    expect(folded.get('a')).toBe(1)
    expect(leadOf.get('b')).toBe('a')
  })

  it('leaves a session that is not a split, and a split whose first pane is in another project', () => {
    expect(foldSplits([quiet], new Map()).visible).toEqual([quiet])
    expect(foldSplits([quiet], panes).visible).toEqual([quiet])
  })

  it('says how many are folded', () => {
    expect(splitNote(1)).toBe('+1 pane')
    expect(splitNote(3)).toBe('+3 panes')
  })
})
