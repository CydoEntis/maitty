import { act, fireEvent, render, screen } from '@testing-library/react'
import { afterEach, describe, expect, it, vi } from 'vitest'
import type { Tab } from '../model/layout'
import { TabBar } from './TabBar'

const makeTabs = (count: number): Tab[] => Array.from({ length: count }, (_, index) => ({
  id: `tab-${String(index)}`,
  panes: [{ id: `pane-${String(index)}`, sessionId: `t${String(index)}`, col: 0 }],
  focus: `pane-${String(index)}`,
  max: null,
}))

// jsdom lays nothing out, so say how wide the tab list is and how much it holds.
function withGeometry(scrollWidth: number, clientWidth: number) {
  Object.defineProperty(HTMLElement.prototype, 'scrollWidth', { configurable: true, get: () => scrollWidth })
  Object.defineProperty(HTMLElement.prototype, 'clientWidth', { configurable: true, get: () => clientWidth })
}

afterEach(() => {
  Reflect.deleteProperty(HTMLElement.prototype, 'scrollWidth')
  Reflect.deleteProperty(HTMLElement.prototype, 'clientWidth')
})

function renderBar(count: number, activeTabId = 'tab-0') {
  const props = { tabs: makeTabs(count), activeTabId, sessions: new Map(), canStart: true, portalContainer: null, onSelect: vi.fn(), onClose: vi.fn(), onCloseOthers: vi.fn(), onFocusPane: vi.fn(), onClosePane: vi.fn(), onRenameSession: vi.fn(), newTabLabel: 'New session in maitty', onNewTab: vi.fn(), inboxCount: 0, inboxNeeds: false, onOpenInbox: vi.fn(), inspect: null, onToggleInspect: vi.fn() }
  const view = render(<TabBar {...props} />)
  const list = screen.getByRole('tablist', { name: 'Tabs' })
  Object.defineProperty(list, 'scrollLeft', { configurable: true, writable: true, value: 0 })
  return { ...view, props, list }
}

describe('tab bar scrolling', () => {
  it('shows no scroll buttons when every tab fits', () => {
    withGeometry(300, 300)
    renderBar(3)

    expect(screen.queryByRole('button', { name: 'Scroll tabs right' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Scroll tabs left' })).not.toBeInTheDocument()
  })

  it('offers to scroll toward the tabs that run off the bar, and back', () => {
    withGeometry(1000, 300)
    const { list } = renderBar(12)

    expect(screen.getByRole('button', { name: 'Scroll tabs right' })).toBeInTheDocument()
    expect(screen.queryByRole('button', { name: 'Scroll tabs left' })).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'Scroll tabs right' }))
    expect(list.scrollLeft).toBe(180)
    fireEvent.scroll(list)

    expect(screen.getByRole('button', { name: 'Scroll tabs left' })).toBeInTheDocument()
    fireEvent.click(screen.getByRole('button', { name: 'Scroll tabs left' }))
    expect(list.scrollLeft).toBe(0)
  })

  it('stops offering the right button once the last tab is reached', () => {
    withGeometry(1000, 300)
    const { list } = renderBar(12)

    list.scrollLeft = 700
    fireEvent.scroll(list)

    expect(screen.queryByRole('button', { name: 'Scroll tabs right' })).not.toBeInTheDocument()
    expect(screen.getByRole('button', { name: 'Scroll tabs left' })).toBeInTheDocument()
  })

  it('turns the mouse wheel into a sideways scroll when the tabs overflow', () => {
    withGeometry(1000, 300)
    const { list } = renderBar(12)

    const event = new WheelEvent('wheel', { deltaY: 50, bubbles: true, cancelable: true })
    act(() => { list.dispatchEvent(event) })

    expect(list.scrollLeft).toBe(50)
    expect(event.defaultPrevented).toBe(true)
  })

  it('leaves the wheel alone when every tab fits', () => {
    withGeometry(300, 300)
    const { list } = renderBar(3)

    const event = new WheelEvent('wheel', { deltaY: 50, bubbles: true, cancelable: true })
    act(() => { list.dispatchEvent(event) })

    expect(list.scrollLeft).toBe(0)
    expect(event.defaultPrevented).toBe(false)
  })

  it('scrolls the active tab into view when it changes, so switching never lands off-screen', () => {
    withGeometry(1000, 300)
    const rect = (left: number, right: number) => ({ left, right, top: 0, bottom: 36, width: right - left, height: 36, x: left, y: 0, toJSON: () => ({}) }) as DOMRect
    const { rerender, props, list } = renderBar(12)
    vi.spyOn(list, 'getBoundingClientRect').mockReturnValue(rect(100, 400))
    const target = screen.getAllByRole('tab')[9]
    if (!target) throw new Error('no tab')
    vi.spyOn(target, 'getBoundingClientRect').mockReturnValue(rect(520, 640))

    rerender(<TabBar {...props} activeTabId="tab-9" />)

    // 640 is 240 past the list's right edge (400), plus the 30px of room kept beside it.
    expect(list.scrollLeft).toBe(270)
  })
})

describe('new tab', () => {
  it('says which project the + opens a shell in, and does it', () => {
    withGeometry(300, 300)
    const { props } = renderBar(2)

    fireEvent.click(screen.getByRole('button', { name: 'New session in maitty' }))

    expect(props.onNewTab).toHaveBeenCalledTimes(1)
  })
})

describe('all tabs menu', () => {
  it('appears only when the tabs overflow', () => {
    withGeometry(300, 300)
    renderBar(3)
    expect(screen.queryByRole('button', { name: 'All tabs' })).not.toBeInTheDocument()
  })

  it('lists every tab with its count, jumps to the one picked, and closes', () => {
    withGeometry(1000, 300)
    const { props } = renderBar(12)

    const button = screen.getByRole('button', { name: 'All tabs' })
    expect(button).toHaveTextContent('12')
    fireEvent.click(button)
    const items = screen.getAllByRole('menuitem')
    expect(items).toHaveLength(12)

    const picked = items[7]
    if (!picked) throw new Error('no item')
    fireEvent.click(picked)

    expect(props.onSelect).toHaveBeenCalledWith('tab-7')
    expect(screen.queryByRole('menu', { name: 'All tabs' })).not.toBeInTheDocument()
  })

  it('closes on Escape and on a click outside', () => {
    withGeometry(1000, 300)
    renderBar(12)

    fireEvent.click(screen.getByRole('button', { name: 'All tabs' }))
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('menu', { name: 'All tabs' })).not.toBeInTheDocument()

    fireEvent.click(screen.getByRole('button', { name: 'All tabs' }))
    fireEvent.mouseDown(document.body)
    expect(screen.queryByRole('menu', { name: 'All tabs' })).not.toBeInTheDocument()
  })
})

describe('tab bar in a small window', () => {
  const base = { tabs: makeTabs(1), activeTabId: 'tab-0', sessions: new Map(), canStart: true, portalContainer: null, onSelect: vi.fn(), onClose: vi.fn(), onCloseOthers: vi.fn(), onFocusPane: vi.fn(), onClosePane: vi.fn(), onRenameSession: vi.fn(), newTabLabel: 'New tab', onNewTab: vi.fn(), inboxCount: 2, inboxNeeds: true, onOpenInbox: vi.fn(), inspect: { open: false, additions: 0, deletions: 0 }, onToggleInspect: vi.fn() }

  it('leaves Repository and Inbox to the activity bar in a wide window, with no Panels menu either', () => {
    render(<TabBar {...base} />)

    expect(screen.queryByRole('button', { name: 'Repository' })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Inbox, 2/ })).not.toBeInTheDocument()
    expect(screen.queryByRole('button', { name: /^Panels/ })).not.toBeInTheDocument()
  })

  it('folds them into one Panels menu that carries the inbox count and the needs-you mark', () => {
    const onOpenInbox = vi.fn()
    const onToggleInspect = vi.fn()
    render(<TabBar {...base} foldPanels onOpenInbox={onOpenInbox} onToggleInspect={onToggleInspect} />)

    expect(screen.queryByRole('button', { name: 'Repository' })).not.toBeInTheDocument()
    const panels = screen.getByRole('button', { name: 'Panels, inbox 2, something needs you' })
    fireEvent.click(panels)
    fireEvent.click(screen.getByRole('menuitem', { name: 'Repository' }))
    expect(onToggleInspect).toHaveBeenCalledOnce()
    expect(screen.queryByRole('menu', { name: 'Panels' })).not.toBeInTheDocument()

    fireEvent.click(panels)
    fireEvent.click(screen.getByRole('menuitem', { name: /^Inbox/ }))
    expect(onOpenInbox).toHaveBeenCalledOnce()
  })

  it('closes the Panels menu with Escape', () => {
    render(<TabBar {...base} foldPanels />)

    fireEvent.click(screen.getByRole('button', { name: /^Panels/ }))
    expect(screen.getByRole('menu', { name: 'Panels' })).toBeInTheDocument()
    fireEvent.keyDown(window, { key: 'Escape' })
    expect(screen.queryByRole('menu', { name: 'Panels' })).not.toBeInTheDocument()
  })

  it('marks the Sessions button when something needs you', () => {
    render(<TabBar {...base} onOpenSessions={vi.fn()} />)

    expect(screen.getByRole('button', { name: 'Sessions, something needs you' })).toBeInTheDocument()
  })

  it('has a plain Sessions button when nothing does', () => {
    render(<TabBar {...base} inboxNeeds={false} onOpenSessions={vi.fn()} />)

    expect(screen.getByRole('button', { name: 'Sessions' })).toBeInTheDocument()
  })
})
