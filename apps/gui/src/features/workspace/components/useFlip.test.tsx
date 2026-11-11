import { fireEvent, render } from '@testing-library/react'
import { useRef } from 'react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import { useFlip } from './useFlip'

/** jsdom lays nothing out, so each row's place is what the test says it is. */
const tops = new Map<string, number>()

function List({ rows }: { rows: readonly string[] }) {
  const list = useRef<HTMLDivElement>(null)
  useFlip(list, false)
  return <div ref={list} data-testid="list">{rows.map((row) => <div key={row} data-flip={row}>{row}</div>)}</div>
}

function prepare(container: HTMLElement, folding: () => boolean) {
  const root = container.querySelector<HTMLElement>('[data-testid="list"]') as HTMLElement
  const glides = vi.fn(() => ({ cancel: vi.fn() }))
  for (const row of Array.from(root.querySelectorAll<HTMLElement>('[data-flip]'))) {
    Object.defineProperty(row, 'offsetTop', { configurable: true, get: () => tops.get(row.dataset.flip ?? '') ?? 0 })
    Object.defineProperty(row, 'offsetParent', { configurable: true, get: () => root })
    row.animate = glides as unknown as HTMLElement['animate']
  }
  root.getAnimations = (() => (folding() ? [{ transitionProperty: 'grid-template-rows', playState: 'running' }] : [])) as unknown as HTMLElement['getAnimations']
  return { root, glides }
}

afterEach(() => { tops.clear() })

describe('rows that slide to their new place', () => {
  it('glides a row that moved', () => {
    tops.set('a', 0).set('b', 40)
    const { container, rerender } = render(<List rows={['a', 'b']} />)
    const { glides } = prepare(container, () => false)
    // The first draw was before the places were known to the test; this one records them.
    rerender(<List rows={['a', 'b']} />)
    glides.mockClear()
    rerender(<List rows={['a', 'b']} />)
    expect(glides).not.toHaveBeenCalled()

    tops.set('b', 96)
    rerender(<List rows={['a', 'b']} />)
    expect(glides).toHaveBeenCalledTimes(1)
  })

  it('leaves the rows to a fold that is moving them, and starts from where they are once it ends', () => {
    tops.set('a', 0).set('b', 40)
    const { container, rerender } = render(<List rows={['a', 'b']} />)
    let folds = true
    const { root, glides } = prepare(container, () => folds)
    rerender(<List rows={['a', 'b']} />)

    // Half way through a session's subagents opening, the list is drawn again: the row under them is part way down.
    tops.set('b', 68)
    rerender(<List rows={['a', 'b']} />)
    expect(glides).not.toHaveBeenCalled()

    // The fold ends with the row at its place, and nothing draws the list again to see it.
    tops.set('b', 96)
    folds = false
    const ended = new Event('transitionend', { bubbles: true })
    Object.defineProperty(ended, 'propertyName', { value: 'grid-template-rows' })
    fireEvent(root, ended)
    rerender(<List rows={['a', 'b']} />)
    expect(glides).not.toHaveBeenCalled()
  })
})
