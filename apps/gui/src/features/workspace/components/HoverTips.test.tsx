import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, beforeEach, describe, expect, it, vi } from 'vitest'
import { HoverTips } from './HoverTips'

const wait = (ms: number) => { act(() => { vi.advanceTimersByTime(ms) }) }

describe('hover tips', () => {
  beforeEach(() => { vi.useFakeTimers() })
  afterEach(() => { vi.useRealTimers() })

  it('shows a title as a tooltip after a short hover and hides it on leaving', () => {
    render(<><HoverTips /><button type="button" title="Close session">x</button></>)
    const button = screen.getByRole('button')

    fireEvent.pointerOver(button)
    expect(button).not.toHaveAttribute('title')
    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    wait(430)
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
    wait(500)

    expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
  })

  describe('timing', () => {
    it('waits 420 ms for the first tip', () => {
      render(<><HoverTips /><button type="button" title="One">x</button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(419)
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
      wait(1)
      expect(screen.getByRole('tooltip')).toBeInTheDocument()
    })

    it('shows the next tip after 40 ms when one was on screen a moment ago', () => {
      render(<><HoverTips /><button type="button" title="One">a</button><button type="button" title="Two">b</button></>)
      const [one, two] = screen.getAllByRole('button')
      fireEvent.pointerOver(one as HTMLElement)
      wait(420)
      fireEvent.pointerOut(one as HTMLElement)
      wait(300)

      fireEvent.pointerOver(two as HTMLElement)
      wait(39)
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
      wait(1)
      expect(screen.getByRole('tooltip')).toHaveTextContent('Two')
    })

    it('goes back to 420 ms once 400 ms have passed with no tip', () => {
      render(<><HoverTips /><button type="button" title="One">a</button><button type="button" title="Two">b</button></>)
      const [one, two] = screen.getAllByRole('button')
      fireEvent.pointerOver(one as HTMLElement)
      wait(420)
      fireEvent.pointerOut(one as HTMLElement)
      wait(401)

      fireEvent.pointerOver(two as HTMLElement)
      wait(300)
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
      wait(120)
      expect(screen.getByRole('tooltip')).toHaveTextContent('Two')
    })

    it.each([
      ['a mouse press', () => { fireEvent.pointerDown(document.body) }],
      ['a key press', () => { fireEvent.keyDown(document.body, { key: 'a' }) }],
      ['the wheel', () => { fireEvent.wheel(document.body) }],
    ])('hides a tip on %s', (_name, act_) => {
      render(<><HoverTips /><button type="button" title="One">a</button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(430)
      expect(screen.getByRole('tooltip')).toBeInTheDocument()
      act_()
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    })
  })

  describe('where the words come from', () => {
    it('reads the aria-label of an icon-only button', () => {
      render(<><HoverTips /><button type="button" aria-label="Collapse sidebar"><svg /></button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(430)
      expect(screen.getByRole('tooltip')).toHaveTextContent('Collapse sidebar')
      expect(screen.getByRole('button')).toHaveAttribute('aria-label', 'Collapse sidebar')
    })

    it('reads the aria-label of an icon-only switch', () => {
      render(<><HoverTips /><span role="switch" aria-checked="false" aria-label="Broadcast" /></>)
      fireEvent.pointerOver(screen.getByRole('switch'))
      wait(430)
      expect(screen.getByRole('tooltip')).toHaveTextContent('Broadcast')
    })

    it('gives a button with text no tip from its aria-label', () => {
      render(<><HoverTips /><button type="button" aria-label="Sessions">Sessions</button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(500)
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    })

    it('prefers the title over the aria-label', () => {
      render(<><HoverTips /><button type="button" aria-label="Split right" title="Split right (Ctrl+Shift+D)"><svg /></button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(430)
      expect(screen.getByRole('tooltip')).toHaveTextContent('Split right')
      expect(screen.getByRole('tooltip').querySelectorAll('kbd')).toHaveLength(3)
    })
  })

  describe('content', () => {
    it('draws the trailing keys as keycaps and a dotted hint as a second line', () => {
      render(<><HoverTips /><button type="button" title="Inbox (⌘⇧J) · What needs you">x</button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(430)
      const tip = screen.getByRole('tooltip')
      expect(Array.from(tip.querySelectorAll('kbd')).map((key) => key.textContent)).toEqual(['⌘', '⇧', 'J'])
      expect(tip.querySelector('.mx-tip-name')).toHaveTextContent('Inbox')
      expect(tip.querySelector('.mx-tip-hint')).toHaveTextContent('What needs you')
    })
  })

  describe('while something richer is open', () => {
    it('shows no tip while a menu is open', () => {
      render(<><HoverTips /><div role="menu" aria-label="Menu" /><button type="button" title="One">x</button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(500)
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    })

    it('shows no tip while a hover card is open', () => {
      render(<><HoverTips /><div role="tooltip" className="mx-railpop">card</div><button type="button" title="One">x</button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(500)
      expect(screen.getAllByRole('tooltip')).toHaveLength(1)
    })

    it('gives a control that opens its own hover card no tip', () => {
      render(<><HoverTips /><button type="button" aria-label="build, claude, working" data-hovercard=""><svg /></button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      wait(500)
      expect(screen.queryByRole('tooltip')).not.toBeInTheDocument()
    })

    it('still keeps the browser’s own tooltip off the element', () => {
      render(<><HoverTips /><div role="menu" aria-label="Menu" /><button type="button" title="One">x</button></>)
      fireEvent.pointerOver(screen.getByRole('button'))
      expect(screen.getByRole('button')).not.toHaveAttribute('title')
    })
  })

  describe('placement', () => {
    const rectAt = (top: number, left = 400, height = 24, width = 24) => ({ top, left, bottom: top + height, right: left + width, width, height, x: left, y: top, toJSON: () => ({}) }) as DOMRect

    function show(rect: DOMRect, attributes: Record<string, string> = {}) {
      render(<><HoverTips /><button type="button" title="One" {...attributes}>x</button></>)
      const button = screen.getByRole('button')
      button.getBoundingClientRect = () => rect
      fireEvent.pointerOver(button)
      wait(430)
      return screen.getByRole('tooltip')
    }

    beforeEach(() => {
      Object.defineProperty(window, 'innerWidth', { configurable: true, value: 1000 })
      Object.defineProperty(window, 'innerHeight', { configurable: true, value: 800 })
    })

    it('puts the tip above the control by default', () => {
      const tip = show(rectAt(300))
      expect(tip.style.bottom).toBe('508px')
      expect(tip.style.top).toBe('')
    })

    it('puts the tip below a control within 56 px of the top', () => {
      const tip = show(rectAt(20))
      expect(tip.style.top).toBe('52px')
      expect(tip.style.bottom).toBe('')
    })

    it('keeps the centre 120 px from the window edges', () => {
      expect(show(rectAt(300, 0)).style.left).toBe('128px')
    })

    it('puts a side tip beside the control, away from the nearer window edge', () => {
      const tip = show(rectAt(300, 8), { 'data-tip-side': '' })
      expect(tip.style.left).toBe('40px')
      expect(tip.style.transform).toBe('translateY(-50%)')
    })
  })
})
