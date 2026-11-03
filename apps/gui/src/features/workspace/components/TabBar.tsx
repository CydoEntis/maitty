import { useCallback, useEffect, useRef, useState, type KeyboardEvent } from 'react'
import { createPortal } from 'react-dom'
import { ContextMenu as ContextMenuPrimitive } from 'radix-ui'
import type { Tab } from '../model/layout'
import { iconSaysProcess, MAX_SESSION_NAME_LENGTH, worstSession, type SessionView } from '../model/session'
import { InlineName } from './InlineName'
import { AppIcon, Icon, PauseBars, Spinner } from './glyphs'
import { STATUS_LABEL } from '../model/statusText'
import { useEffectClass } from './motion'

type TabBarProps = {
  /** The tabs of the project that is showing; other projects' tabs stay hidden until you go there. */
  tabs: Tab[]
  activeTabId: string | null
  sessions: ReadonlyMap<string, SessionView>
  primary: string
  canStart: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  onSelect: (tabId: string) => void
  onClose: (tabId: string) => void
  /** Names the session a tab shows; an empty name clears it. */
  onRenameSession: (sessionId: string, name: string) => void
  /** What the + button says it does: it opens a shell in the showing project. */
  newTabLabel: string
  onNewTab: () => void
  /** Sessions in the inbox, and whether any needs you (the count then turns amber). */
  inboxCount: number
  inboxNeeds: boolean
  onOpenInbox: () => void
  /** The Inspect button: whether its panel is open and the focused pane's changed lines (null: not known). */
  inspect: { open: boolean; additions: number; deletions: number } | null
  onToggleInspect: () => void
  /** Present in a window too narrow for the sidebar: the button that slides it in over the panes. */
  onOpenSessions?: () => void
}

/** How far one press of an edge button scrolls: most of a screenful, never less than a tab or so. */
const SCROLL_STEP_RATIO = 0.6
const MIN_SCROLL_STEP_PX = 120
/** Within this many pixels of an end counts as being at it. */
const EDGE_PX = 1
/** The room kept around the active tab when it is scrolled into view. */
const REVEAL_MARGIN_PX = 30

/**
 * Tabs that do not fit scroll sideways. This tracks which ends have more tabs beyond them, scrolls
 * with the wheel, and keeps the active tab in view so switching tabs never lands off-screen.
 */
function useTabScroll(activeTabId: string | null, tabCount: number) {
  const listRef = useRef<HTMLDivElement>(null)
  const [edges, setEdges] = useState({ left: false, right: false, over: false })
  const measure = useCallback(() => {
    const list = listRef.current
    if (!list) return
    const left = list.scrollLeft > EDGE_PX
    const right = list.scrollLeft + list.clientWidth < list.scrollWidth - EDGE_PX
    const over = list.scrollWidth > list.clientWidth + EDGE_PX
    setEdges((current) => (current.left === left && current.right === right && current.over === over ? current : { left, right, over }))
  }, [])

  useEffect(() => {
    const list = listRef.current
    if (!list) return
    measure()
    // A vertical wheel turn scrolls the tabs sideways, so the wheel reaches tabs that run off the bar.
    const onWheel = (event: WheelEvent) => {
      if (event.deltaX !== 0 || list.scrollWidth <= list.clientWidth) return
      event.preventDefault()
      list.scrollLeft += event.deltaY
    }
    list.addEventListener('wheel', onWheel, { passive: false })
    const observer = 'ResizeObserver' in window ? new ResizeObserver(measure) : null
    observer?.observe(list)
    window.addEventListener('resize', measure)
    return () => {
      list.removeEventListener('wheel', onWheel)
      observer?.disconnect()
      window.removeEventListener('resize', measure)
    }
  }, [measure, tabCount])

  // Measured against the list's own box (not offsetLeft, and not scrollIntoView, which would also scroll the page).
  useEffect(() => {
    const list = listRef.current
    const tab = list?.querySelector<HTMLElement>('[role="tab"][aria-selected="true"]')
    if (!list || !tab) return
    const box = list.getBoundingClientRect()
    const at = tab.getBoundingClientRect()
    if (at.left < box.left + REVEAL_MARGIN_PX) list.scrollLeft += at.left - box.left - REVEAL_MARGIN_PX
    else if (at.right > box.right - REVEAL_MARGIN_PX) list.scrollLeft += at.right - box.right + REVEAL_MARGIN_PX
  }, [activeTabId, tabCount])

  const scroll = (direction: -1 | 1) => {
    const list = listRef.current
    if (list) list.scrollLeft += direction * Math.max(MIN_SCROLL_STEP_PX, list.clientWidth * SCROLL_STEP_RATIO)
  }
  return { listRef, edges, measure, scroll }
}

export function TabBar(props: TabBarProps) {
  const { tabs, activeTabId } = props
  const { listRef, edges, measure, scroll } = useTabScroll(activeTabId, tabs.length)
  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    const step = event.key === 'ArrowRight' ? 1 : event.key === 'ArrowLeft' ? -1 : 0
    if (step === 0) return
    const elements = Array.from(event.currentTarget.querySelectorAll<HTMLElement>('[role="tab"]'))
    const index = elements.findIndex((element) => element === document.activeElement)
    const next = elements[(index + step + elements.length) % elements.length]
    if (!next) return
    event.preventDefault()
    next.focus()
  }
  return (
    <div className="mx-tabbar">
      {props.onOpenSessions && (
        <button type="button" className="mx-inbox-button mx-sessions-button" aria-label="Sessions" title={`Sessions (${props.primary}B)`} onClick={props.onOpenSessions}>Sessions</button>
      )}
      {tabs.length > 0 && (
        <div className="mx-tabstrip">
          {edges.left && <button type="button" className="mx-icon-button mx-tab-scroll is-left" aria-label="Scroll tabs left" title="Scroll tabs left" onClick={() => { scroll(-1) }}><Icon name="chevron" /></button>}
          <div ref={listRef} role="tablist" aria-label="Tabs" className={`mx-tablist mx-no-scrollbar${edges.left ? ' has-left' : ''}${edges.right ? ' has-right' : ''}`} onKeyDown={onKeyDown} onScroll={measure}>
            {tabs.map((tab) => (
              <TabButton key={tab.id} tab={tab} active={tab.id === activeTabId} sessions={props.sessions} primary={props.primary} canStart={props.canStart} portalContainer={props.portalContainer} onSelect={props.onSelect} onClose={props.onClose} onNewTab={props.onNewTab} onRenameSession={props.onRenameSession} />
            ))}
          </div>
          {edges.right && <button type="button" className="mx-icon-button mx-tab-scroll" aria-label="Scroll tabs right" title="Scroll tabs right" onClick={() => { scroll(1) }}><Icon name="chevron" /></button>}
        </div>
      )}
      <button type="button" className="mx-icon-button mx-new-tab" aria-label={props.newTabLabel} title={props.newTabLabel} disabled={!props.canStart} onClick={props.onNewTab}><Icon name="plus" /></button>
      {edges.over && <AllTabs tabs={tabs} activeTabId={activeTabId} sessions={props.sessions} onSelect={props.onSelect} />}
      <span className="mx-spacer" />
      {props.inspect && (
        <button type="button" className={`mx-inbox-button${props.inspect.open ? ' is-open' : ''}`} aria-pressed={props.inspect.open} aria-label="Repository" title={`Repository (${props.primary}I)`} onClick={props.onToggleInspect}>
          <Icon name="repo" />
          {(props.inspect.additions > 0 || props.inspect.deletions > 0) && (
            <span className="mx-inspect-stat"><span className="is-add">+{props.inspect.additions}</span> <span className="is-remove">−{props.inspect.deletions}</span></span>
          )}
        </button>
      )}
      <button type="button" className="mx-inbox-button" aria-label={`Inbox, ${String(props.inboxCount)}${props.inboxNeeds ? ', something needs you' : ''}`} title={`Inbox (${props.primary}J)`} onClick={props.onOpenInbox}>
        <Icon name="inbox" />
        {props.inboxCount > 0 && <span key={props.inboxCount} className={`mx-inbox-count${props.inboxNeeds ? ' is-needs' : ''}`}>{props.inboxCount}</span>}
      </button>
    </div>
  )
}

/** Every tab of the project with its state, for when they run off the bar. Opens leftward from its button. */
function AllTabs({ tabs, activeTabId, sessions, onSelect }: { tabs: Tab[]; activeTabId: string | null; sessions: ReadonlyMap<string, SessionView>; onSelect: (tabId: string) => void }) {
  const [open, setOpen] = useState(false)
  const root = useRef<HTMLDivElement>(null)
  useEffect(() => {
    if (!open) return
    const onKey = (event: globalThis.KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.stopPropagation()
      setOpen(false)
    }
    const onPointer = (event: MouseEvent) => {
      if (event.target instanceof Node && !root.current?.contains(event.target)) setOpen(false)
    }
    window.addEventListener('keydown', onKey, true)
    window.addEventListener('mousedown', onPointer, true)
    return () => {
      window.removeEventListener('keydown', onKey, true)
      window.removeEventListener('mousedown', onPointer, true)
    }
  }, [open])
  const summaries = tabs.map((tab) => {
    const shown = tab.panes.flatMap((pane) => {
      const session = sessions.get(pane.sessionId)
      return session ? [session] : []
    })
    const focused = sessions.get(tab.panes.find((pane) => pane.id === tab.focus)?.sessionId ?? '') ?? shown[0]
    return { tab, focused, status: (worstSession(shown) ?? focused)?.status ?? 'idle', panes: tab.panes.length }
  })
  const needsYou = summaries.some((summary) => summary.status === 'needs')
  return (
    <div ref={root} className="mx-alltabs">
      <button type="button" className={`mx-alltabs-button${open ? ' is-open' : ''}`} aria-haspopup="menu" aria-expanded={open} aria-label="All tabs" title="All tabs" onClick={() => { setOpen(!open) }}>
        <span>{tabs.length}</span>
        <svg width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
        {needsYou && <span className="mx-alltabs-needs" aria-label="A tab needs you" />}
      </button>
      {open && (
        <div role="menu" aria-label="All tabs" className="mx-alltabs-menu">
          {summaries.map(({ tab, focused, status, panes }) => (
            <button key={tab.id} type="button" role="menuitem" className={`mx-alltabs-row${tab.id === activeTabId ? ' is-active' : ''}`} onClick={() => { setOpen(false); onSelect(tab.id) }}>
              <AppIcon process={focused?.process ?? 'shell'} size={20} />
              <strong>{focused?.process ?? 'shell'}</strong>
              <span className="mx-alltabs-folder">{focused?.name ?? focused?.promptPath ?? ''}</span>
              {status === 'working' && <Spinner />}
              {status === 'needs' && <span className="mx-alltabs-state is-needs">Needs you</span>}
              {panes > 1 && <span className="mx-alltabs-count">{panes}</span>}
            </button>
          ))}
        </div>
      )}
    </div>
  )
}

/** The most pane icons one tab shows; the label's +N counts the rest. */
const TAB_ICON_LIMIT = 3
/** The gap between a tab and the popover listing its panes. */
const TAB_POPOVER_GAP_PX = 8

/** Every pane in a tab, with what each is running and its state. */
function TabPopover({ sessions, anchor }: { sessions: readonly SessionView[]; anchor: DOMRect }) {
  return (
    <div role="tooltip" className="mx-rail-popover is-tab" style={{ top: anchor.bottom + TAB_POPOVER_GAP_PX, left: anchor.left }}>
      <div className="mx-pop-title-line">{sessions.length} {sessions.length === 1 ? 'pane' : 'panes'} in this tab</div>
      <ul className="mx-pop-sessions">
        {sessions.map((session) => (
          <li key={session.id} className="mx-pop-pane">
            <AppIcon process={session.process} size={20} dimmed={session.status === 'unknown'} />
            <span className="mx-pop-pane-text"><span className="mx-pop-title">{session.process} <span className="mx-pop-status">{session.cwdLabel}</span></span></span>
            <span className={`mx-pop-state is-${session.status}`}>{STATUS_LABEL[session.status]}</span>
          </li>
        ))}
      </ul>
    </div>
  )
}

type TabButtonProps = { tab: Tab; active: boolean; sessions: ReadonlyMap<string, SessionView>; primary: string; canStart: boolean; portalContainer: HTMLElement | null; onSelect: (id: string) => void; onClose: (id: string) => void; onNewTab: () => void; onRenameSession: (sessionId: string, name: string) => void }

function TabButton({ tab, active, sessions, primary, canStart, portalContainer, onSelect, onClose, onNewTab, onRenameSession }: TabButtonProps) {
  const focused = sessions.get(tab.panes.find((pane) => pane.id === tab.focus)?.sessionId ?? tab.panes[0]?.sessionId ?? '')
  const shown = tab.panes.flatMap((pane) => {
    const session = sessions.get(pane.sessionId)
    return session ? [session] : []
  })
  const worst = worstSession(shown) ?? focused
  const status = worst?.status ?? 'idle'
  const effect = useEffectClass(status, active)
  const process = focused?.process ?? 'shell'
  const path = focused?.promptPath ?? ''
  const multi = tab.panes.length > 1
  const [anchor, setAnchor] = useState<DOMRect | null>(null)
  const [renaming, setRenaming] = useState(false)
  const handFocusToField = useRef(false)
  return (
    <ContextMenuPrimitive.Root>
      <ContextMenuPrimitive.Trigger asChild>
    <div
      role="tab"
      tabIndex={active ? 0 : -1}
      aria-selected={active}
      className={`mx-tab${active ? ' is-active' : ''} ${effect}`}
      onMouseEnter={(event) => { setAnchor(event.currentTarget.getBoundingClientRect()) }}
      onMouseLeave={() => { setAnchor(null) }}
      onFocus={(event) => { setAnchor(event.currentTarget.getBoundingClientRect()) }}
      onBlur={() => { setAnchor(null) }}
      onClick={() => { onSelect(tab.id) }}
      onKeyDown={(event) => {
        if (event.key === 'Enter' || event.key === ' ') {
          event.preventDefault()
          onSelect(tab.id)
        }
      }}
    >
      <span className="mx-tab-icons" aria-hidden="true">
        {(shown.length > 0 ? shown : focused ? [focused] : []).slice(0, TAB_ICON_LIMIT).map((session) => <AppIcon key={session.id} process={session.process} size={20} />)}
        {shown.length === 0 && !focused && <AppIcon process={process} size={20} />}
      </span>
      <span className="mx-tab-label">{!iconSaysProcess(process) && <span>{process}</span>}{multi && <span className="mx-tab-more">+{tab.panes.length - 1}</span>} {renaming && focused
        ? <InlineName value={focused.name ?? ''} placeholder={path} label={`Name for ${focused.title}`} maxLength={MAX_SESSION_NAME_LENGTH} onCommit={(name) => { setRenaming(false); onRenameSession(focused.id, name) }} onCancel={() => { setRenaming(false) }} />
        : <span className="mx-tab-folder">{focused?.name ?? path}</span>}</span>
      {status === 'needs' && <span className="mx-tab-needs" title="Needs you"><PauseBars /></span>}
      {status === 'working' && <Spinner />}
      {status === 'done' && <span className="mx-tab-done" title="Done"><Icon name="check" /></span>}
      {status === 'error' && <span className="mx-tab-error" title="Exited with error"><Icon name="bang" /></span>}
      <button
        type="button"
        tabIndex={-1}
        className="mx-tab-close"
        aria-label="Close tab. Sessions keep running."
        title="Close tab · sessions keep running"
        onClick={(event) => {
          event.stopPropagation()
          onClose(tab.id)
        }}
      >
        <Icon name="close" size={11} strokeWidth={2.25} />
      </button>
    </div>
      </ContextMenuPrimitive.Trigger>
      {anchor && portalContainer && createPortal(<TabPopover sessions={shown} anchor={anchor} />, portalContainer)}
      <ContextMenuPrimitive.Portal container={portalContainer}>
        {/* The name field takes focus itself; the menu must not hand it back to the tab. */}
        <ContextMenuPrimitive.Content className="mx-menu" onCloseAutoFocus={(event) => { if (handFocusToField.current) { event.preventDefault(); handFocusToField.current = false } }}>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!focused} onSelect={() => { handFocusToField.current = true; setRenaming(true) }}>Rename</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!canStart} onSelect={onNewTab}>New tab<span className="mx-menu-hint">{primary}T</span></ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { onClose(tab.id) }}>Close tab<span className="mx-menu-hint">sessions keep running</span></ContextMenuPrimitive.Item>
        </ContextMenuPrimitive.Content>
      </ContextMenuPrimitive.Portal>
    </ContextMenuPrimitive.Root>
  )
}
