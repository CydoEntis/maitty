import { describe, expect, it } from 'vitest'
import { rememberSessions, withEnded } from './ended'
import { EMPTY_LAYOUT, openInNewTab, removeSessions, splitPane, type Layout, type MakeId } from './layout'
import type { SessionView } from './session'

const session = (id: string): SessionView => ({ id, status: 'working', ended: false }) as SessionView

function ids(): MakeId {
  let next = 0
  return (prefix) => `${prefix}-${String((next += 1))}`
}

function layoutOf(...sessionIds: string[]): Layout {
  const makeId = ids()
  const [first, ...rest] = sessionIds
  let layout = openInNewTab(EMPTY_LAYOUT, first ?? 's', makeId)
  for (const id of rest) layout = splitPane(layout, null, 'right', id, makeId)
  return layout
}

describe('ended sessions', () => {
  it('remembers a session a pane still shows after the daemon stops listing it', () => {
    const layout = layoutOf('a', 'b')
    const first = rememberSessions(new Map(), [session('a'), session('b')], layout)

    const next = rememberSessions(first, [session('a')], layout)

    expect([...next.keys()].sort()).toEqual(['a', 'b'])
  })

  it('forgets a session once no pane shows it', () => {
    const first = rememberSessions(new Map(), [session('a'), session('b')], layoutOf('a', 'b'))

    const next = rememberSessions(first, [session('a')], layoutOf('a'))

    expect([...next.keys()]).toEqual(['a'])
  })

  it('shows a remembered session that is no longer listed as ended and idle, and leaves listed ones alone', () => {
    const remembered = new Map([['a', session('a')], ['b', session('b')]])

    const all = withEnded(new Map([['a', session('a')]]), remembered)

    expect(all.get('a')).toMatchObject({ ended: false, status: 'working' })
    expect(all.get('b')).toMatchObject({ ended: true, status: 'idle' })
  })
})

describe('removing sessions on purpose', () => {
  it('takes out only the named sessions and leaves other panes, even unlisted ones', () => {
    const layout = layoutOf('a', 'b', 'c')

    const removed = removeSessions(layout, new Set(['b']))

    expect(removed.tabs[0]?.panes.map((pane) => pane.sessionId)).toEqual(['a', 'c'])
  })
})
