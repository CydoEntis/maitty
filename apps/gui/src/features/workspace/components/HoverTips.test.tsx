import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { HoverTips } from './HoverTips'

describe('hover tips', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('shows a title as a tooltip after a short hover and hides it on leaving', () => {
    render(<><HoverTips /><button type="button" title="Close session">x</button></>)
    const button = screen.getByRole('button')

    fireEvent.pointerOver(button)
    expect(button).not.toHaveAttribute('title')
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    act(() => { vi.advanceTimersByTime(400) })
    expect(screen.getByRole('tooltip')).toHaveTextContent('Close session')

    fireEvent.pointerOut(button)
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    expect(button).toHaveAttribute('title', 'Close session')
  })

  it('shows nothing for a quick pass over the element', () => {
    render(<><HoverTips /><button type="button" title="Split right">x</button></>)
    const button = screen.getByRole('button')

    fireEvent.pointerOver(button)
    fireEvent.pointerOut(button)
    act(() => { vi.advanceTimersByTime(500) })

    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
  })
})
