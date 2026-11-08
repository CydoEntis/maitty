import { act, fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it } from 'vitest'
import { VirtualList } from './VirtualList'

const ROW = 50
const rows = (count: number) => Array.from({ length: count }, (_, index) => `row-${String(index)}`)

function list(count: number) {
  return (
    <VirtualList items={rows(count)} getKey={(key) => key} estimate={ROW} label="Conversation" renderItem={(key) => <p>{key}</p>} />
  )
}

/** jsdom has no layout, so the scroller's position is a plain number the test can set and read. */
function scroller(): HTMLElement {
  const element = screen.getByRole('region', { name: 'Conversation' })
  let top = 0
  Object.defineProperty(element, 'scrollTop', { configurable: true, get: () => top, set: (value: number) => { top = value } })
  return element
}

const latest = () => screen.queryByRole('button', { name: 'Jump to the latest message' })

describe('a conversation list that follows its newest row', () => {
  it('offers no jump to the latest message when the whole chat is already on screen, even after a wheel move up', () => {
    render(list(2))
    const element = scroller()

    fireEvent.wheel(element, { deltaY: -3 })

    expect(latest()).toBeNull()
  })

  it('stays on the newest row while the reader has not moved', () => {
    const { rerender } = render(list(20))
    const element = scroller()
    rerender(list(21))

    expect(element.scrollTop).toBe(21 * ROW)
    expect(latest()).toBeNull()
  })

  it('stops following when the reader moves up a little, and a growing last row leaves the view alone', () => {
    const { rerender } = render(list(20))
    const element = scroller()
    rerender(list(21))
    const reading = 21 * ROW - 10

    act(() => { element.scrollTop = reading })
    fireEvent.scroll(element)
    rerender(list(22))

    expect(element.scrollTop).toBe(reading)
    expect(latest()).not.toBeNull()
  })

  it('stops at once on a wheel move up, before its scroll event', () => {
    const { rerender } = render(list(20))
    const element = scroller()
    rerender(list(21))

    fireEvent.wheel(element, { deltaY: -3 })
    rerender(list(22))

    expect(element.scrollTop).toBe(21 * ROW)
    expect(latest()).not.toBeNull()
  })

  it('follows again from the button, and when the reader scrolls back down to the bottom', () => {
    const { rerender } = render(list(20))
    const element = scroller()
    rerender(list(21))
    fireEvent.wheel(element, { deltaY: -3 })
    act(() => { element.scrollTop = 500 })
    fireEvent.scroll(element)

    fireEvent.click(latest() as HTMLElement)
    expect(latest()).toBeNull()
    expect(element.scrollTop).toBe(21 * ROW)

    fireEvent.wheel(element, { deltaY: -3 })
    act(() => { element.scrollTop = 400 })
    fireEvent.scroll(element)
    act(() => { element.scrollTop = 21 * ROW })
    fireEvent.scroll(element)
    expect(latest()).toBeNull()
  })
})
