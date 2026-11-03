import { fireEvent, render, screen } from '@testing-library/react'
import { describe, expect, it, vi } from 'vitest'
import { SIDEBAR_WIDTH_DEFAULT, SIDEBAR_WIDTH_MAX, SIDEBAR_WIDTH_MIN } from '@/features/settings/preferences'
import { SidebarResizer } from './SidebarResizer'

function setup(width = 300, side: 'left' | 'right' = 'left') {
  const handlers = { onResize: vi.fn(), onResizeEnd: vi.fn(), onDragging: vi.fn() }
  render(<SidebarResizer side={side} width={width} {...handlers} />)
  return { handle: screen.getByRole('separator', { name: 'Resize sidebar' }), ...handlers }
}

describe('sidebar resize handle', () => {
  it('follows the pointer while dragging and saves the width on release', () => {
    const { handle, onResize, onResizeEnd, onDragging } = setup(300)

    fireEvent.pointerDown(handle, { clientX: 300, pointerId: 1 })
    fireEvent.pointerMove(handle, { clientX: 340, pointerId: 1 })
    fireEvent.pointerUp(handle, { clientX: 350, pointerId: 1 })

    expect(onDragging).toHaveBeenNthCalledWith(1, true)
    expect(onResize).toHaveBeenCalledWith(340)
    expect(onResizeEnd).toHaveBeenCalledWith(350)
    expect(onDragging).toHaveBeenLastCalledWith(false)
  })

  it('never goes narrower or wider than its limits', () => {
    const { handle, onResize } = setup(300)

    fireEvent.pointerDown(handle, { clientX: 300, pointerId: 1 })
    fireEvent.pointerMove(handle, { clientX: -500, pointerId: 1 })
    expect(onResize).toHaveBeenLastCalledWith(SIDEBAR_WIDTH_MIN)
    fireEvent.pointerMove(handle, { clientX: 5000, pointerId: 1 })
    expect(onResize).toHaveBeenLastCalledWith(SIDEBAR_WIDTH_MAX)
  })

  it('resizes with the arrow keys and resets with Home or a double-click', () => {
    const { handle, onResizeEnd } = setup(300)

    fireEvent.keyDown(handle, { key: 'ArrowRight' })
    expect(onResizeEnd).toHaveBeenLastCalledWith(316)
    fireEvent.keyDown(handle, { key: 'ArrowLeft' })
    expect(onResizeEnd).toHaveBeenLastCalledWith(284)
    fireEvent.keyDown(handle, { key: 'Home' })
    expect(onResizeEnd).toHaveBeenLastCalledWith(SIDEBAR_WIDTH_DEFAULT)
    fireEvent.doubleClick(handle)
    expect(onResizeEnd).toHaveBeenLastCalledWith(SIDEBAR_WIDTH_DEFAULT)
  })

  it('tells assistive tech its value and range', () => {
    const { handle } = setup(300)

    expect(handle).toHaveAttribute('aria-valuenow', '300')
    expect(handle).toHaveAttribute('aria-valuemin', String(SIDEBAR_WIDTH_MIN))
    expect(handle).toHaveAttribute('aria-valuemax', String(SIDEBAR_WIDTH_MAX))
  })

  it('widens when dragged toward the middle of the window with the sidebar on the right', () => {
    const { handle, onResize, onResizeEnd } = setup(300, 'right')

    fireEvent.pointerDown(handle, { clientX: 900, pointerId: 1 })
    fireEvent.pointerMove(handle, { clientX: 860, pointerId: 1 })
    fireEvent.pointerUp(handle, { clientX: 850, pointerId: 1 })

    expect(onResize).toHaveBeenCalledWith(340)
    expect(onResizeEnd).toHaveBeenCalledWith(350)
  })

  it('reverses the arrow keys on the right', () => {
    const { handle, onResizeEnd } = setup(300, 'right')

    fireEvent.keyDown(handle, { key: 'ArrowLeft' })

    expect(onResizeEnd).toHaveBeenCalledWith(316)
  })
})
