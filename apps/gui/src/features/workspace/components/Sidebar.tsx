import { useEffect, useRef, useState, type ReactNode } from 'react'
import { displayName, matchesFilter, STATUS_WORD, type SessionView } from '../model/session'
import type { SessionTrack } from '../model/tracking'
import { SidebarResizer } from './SidebarResizer'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { SessionRow } from './SessionRow'
import { useSessionMenu } from './SessionMenu'
import { RailPopoverLayer } from './RailPopovers'
import { useRailHover, type RailHover } from './railHover'
import { useTipWithKeys } from '@/features/shortcuts/platform'

export type SidebarProps = {
  /** Every session, in creation order. The filter field narrows what shows; it never reorders. */
  sessions: readonly SessionView[]
  focusedId: string | null
  tracks: ReadonlyMap<string, SessionTrack>
  mode: 'docked' | 'rail'
  online: boolean
  canStart: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  /** Collapses the docked sidebar to the rail, or expands the rail (over the window when it is narrow). */
  onToggleRail: () => void
  side: 'left' | 'right'
  /** The docked sidebar's width in px, and how the user changes it by dragging its edge. */
  width: number
  onResize: (width: number) => void
  onResizeEnd: (width: number) => void
  /** How many sessions the Inbox holds, and whether any of them needs you. */
  inboxCount: number
  inboxNeeds: boolean
  onOpenInbox: () => void
  onNewSession: () => void
  onShowSession: (id: string) => void
  onEndSession: (id: string) => void
  onSetName: (id: string, name: string) => void
  /** Opens the rename dialog, for the rail, which has no room to rename a session in place. */
  onRenameSession: (id: string) => void
  onPinSession: (id: string, pinned: boolean) => void
  /** Shows a session beside the pane that has focus. */
  onSplitSession: (id: string, direction: 'right' | 'down') => void
  onCopyPath: (id: string) => void
  onOpenSettings: () => void
  /** The footer's left slot while the daemon is reachable: the version, or an update that needs you. */
  updateSlot: ReactNode
  /** The rail's download button, present only while an update exists. */
  railUpdateSlot: ReactNode
}

export function Sidebar(props: SidebarProps) {
  const [resizing, setResizing] = useState(false)
  const className = `mx-sidebar mx-card${props.mode === 'rail' ? ' is-rail' : ''}${resizing ? ' is-resizing' : ''}`
  return (
    <nav aria-label="Sessions" className={className}>
      {props.mode === 'rail' ? <Rail {...props} /> : <Expanded {...props} />}
      {props.mode === 'docked' && <SidebarResizer side={props.side} width={props.width} onResize={props.onResize} onResizeEnd={props.onResizeEnd} onDragging={setResizing} />}
    </nav>
  )
}

/** The Inbox button with its count; amber while anything needs you. */
function InboxButton({ count, needs, onClick, rail = false }: { count: number; needs: boolean; onClick: () => void; rail?: boolean }) {
  const tip = useTipWithKeys()
  const label = `Inbox, ${String(count)}${needs ? ', something needs you' : ''}`
  return (
    <button type="button" className={`mx-inbox-button${rail ? ' mx-icon-button mx-rail-button' : ''}${needs ? ' is-needs' : ''}`} aria-label={label} title={tip('Inbox', 'inbox')} onClick={onClick}>
      <Icon name="inbox" size={15} />
      {count > 0 && <span className={`mx-inbox-count${needs ? ' is-needs' : ''}`}>{count}</span>}
    </button>
  )
}

/** `/` focuses the filter from anywhere outside a pane or another field. */
function useSlashFocus(input: React.RefObject<HTMLInputElement | null>) {
  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== '/' || event.ctrlKey || event.metaKey || event.altKey) return
      const target = event.target
      if (target instanceof Element && target.closest('input, textarea, select, [contenteditable="true"], [data-pane], .xterm')) return
      event.preventDefault()
      input.current?.focus()
    }
    window.addEventListener('keydown', onKeyDown)
    return () => { window.removeEventListener('keydown', onKeyDown) }
  }, [input])
}

function Expanded(props: SidebarProps) {
  const { sessions, online } = props
  const tip = useTipWithKeys()
  const hover = useRailHover()
  const input = useRef<HTMLInputElement>(null)
  const [query, setQuery] = useState('')
  useSlashFocus(input)
  const shown = sessions.filter((session) => matchesFilter(session, query))
  return (
    <div className="mx-sidebar-expanded">
      <RailPopoverLayer hover={hover} sessions={sessions} tracks={props.tracks} portalContainer={props.portalContainer} />
      <div className="mx-sidebar-head">
        <button type="button" className="mx-icon-button mx-collapse" aria-label="Collapse sidebar" title={tip('Collapse sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="collapse" /></button>
        <button type="button" className="mx-new-button" aria-label="New session" title={tip('New session', 'new-session')} disabled={!props.canStart} onClick={props.onNewSession}><Icon name="plus" size={15} /></button>
        <span className="mx-spacer" />
        <InboxButton count={props.inboxCount} needs={props.inboxNeeds} onClick={props.onOpenInbox} />
      </div>
      <div className="mx-filter">
        <Icon name="search" size={13} />
        <input
          ref={input}
          type="text"
          value={query}
          placeholder="Filter sessions"
          aria-label="Filter sessions"
          spellCheck={false}
          onChange={(event) => { setQuery(event.target.value) }}
          onKeyDown={(event) => {
            if (event.key !== 'Escape') return
            // Esc empties the field and gives the keyboard back; it must not reach a pane behind it.
            event.preventDefault()
            event.stopPropagation()
            setQuery('')
            event.currentTarget.blur()
          }}
        />
        {query === '' ? <kbd className="mx-filter-hint" aria-hidden="true">/</kbd> : <button type="button" className="mx-filter-clear" aria-label="Clear filter" onClick={() => { setQuery(''); input.current?.focus() }}><Icon name="close" size={11} /></button>}
      </div>
      <div className="mx-sidebar-list" role="list" aria-label="Sessions">
        {shown.map((session) => (
          <SessionRow
            key={session.id}
            session={session}
            track={props.tracks.get(session.id)}
            selected={session.id === props.focusedId}
            online={online}
            portalContainer={props.portalContainer}
            hover={hover}
            onShow={() => { props.onShowSession(session.id) }}
            onFilterFolder={setQuery}
            onSetName={(name) => { props.onSetName(session.id, name) }}
            onPin={(pinned) => { props.onPinSession(session.id, pinned) }}
            onSplit={(direction) => { props.onSplitSession(session.id, direction) }}
            onCopyPath={() => { props.onCopyPath(session.id) }}
            onEnd={() => { props.onEndSession(session.id) }}
          />
        ))}
        {sessions.length === 0 && <div className="mx-empty-note">No sessions yet. New session starts one.</div>}
        {sessions.length > 0 && shown.length === 0 && <div className="mx-empty-note">No session matches “{query}”.</div>}
      </div>
      <div className="mx-sidebar-foot">
        {online
          ? <span className="mx-foot-slot">{props.updateSlot}</span>
          : <><span className="mx-daemon-dot is-offline" aria-hidden="true" /><span className="mx-daemon-text is-offline" role="status">Reconnecting to maittyd…</span></>}
        <button type="button" className="mx-icon-button mx-settings-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" size={15} /></button>
      </div>
    </div>
  )
}

/** A collapsed session: its icon and badge, and a card beside it that says what is going on. */
function RailSessionButton(props: { session: SessionView; track: SessionTrack | undefined; selected: boolean; online: boolean; portalContainer: HTMLElement | null; hover: RailHover; onShow: () => void; onSetName: () => void; onPin: (pinned: boolean) => void; onSplit: (direction: 'right' | 'down') => void; onCopyPath: () => void; onEnd: () => void }) {
  const { session, track, selected, hover } = props
  const menu = useSessionMenu(session, { online: props.online, portalContainer: props.portalContainer, onSplit: props.onSplit, onCopyPath: props.onCopyPath, onRename: props.onSetName, onPin: props.onPin, onEnd: props.onEnd })
  const unread = Boolean(track?.unread) && !selected
  const words = `${displayName(session)}, ${session.process}, ${STATUS_WORD[session.status]}`
  return (
    <>
      <button
        type="button"
        className={`mx-rail-row${selected ? ' is-selected' : ''}`}
        aria-label={words}
        data-hovercard=""
        aria-current={selected ? 'page' : undefined}
        onMouseEnter={(event) => { hover.enter(session.id, event.currentTarget) }}
        onMouseLeave={hover.leave}
        onFocus={(event) => { hover.enter(session.id, event.currentTarget) }}
        onBlur={hover.leave}
        onContextMenu={(event) => { hover.close(); menu.onContextMenu(event) }}
        onClick={props.onShow}
      >
        <AppIcon process={session.process} size={32} dimmed={session.status === 'unknown'} />
        <StatusBadge status={session.status} rail />
        {unread && <span className="mx-rail-unread" />}
      </button>
      {menu.element}
    </>
  )
}

function Rail(props: SidebarProps) {
  const { sessions, focusedId, tracks, online } = props
  const tip = useTipWithKeys()
  const hover = useRailHover()
  const label = online ? `maittyd · ${String(sessions.length)} sessions` : 'Reconnecting to maittyd…'
  return (
    <div className="mx-rail">
      <RailPopoverLayer hover={hover} sessions={sessions} tracks={tracks} portalContainer={props.portalContainer} />
      <div className="mx-rail-top">
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Expand sidebar" title={tip('Expand sidebar', 'toggle-sidebar')} onClick={props.onToggleRail}><Icon name="expand" /></button>
        <button type="button" className="mx-icon-button mx-rail-button mx-rail-new" aria-label="New session" title={tip('New session', 'new-session')} disabled={!props.canStart} onClick={props.onNewSession}><Icon name="plus" /></button>
        <InboxButton rail count={props.inboxCount} needs={props.inboxNeeds} onClick={props.onOpenInbox} />
      </div>
      <div className="mx-rail-divider" />
      <div className="mx-rail-list mx-no-scrollbar">
        {sessions.map((session) => (
          <RailSessionButton
            key={session.id}
            session={session}
            track={tracks.get(session.id)}
            selected={session.id === focusedId}
            online={online}
            portalContainer={props.portalContainer}
            hover={hover}
            onShow={() => { props.onShowSession(session.id) }}
            onSetName={() => { props.onRenameSession(session.id) }}
            onPin={(pinned) => { props.onPinSession(session.id, pinned) }}
            onSplit={(direction) => { props.onSplitSession(session.id, direction) }}
            onCopyPath={() => { props.onCopyPath(session.id) }}
            onEnd={() => { props.onEndSession(session.id) }}
          />
        ))}
      </div>
      <div className="mx-rail-bottom">
        {props.railUpdateSlot}
        <span className={`mx-daemon-dot${online ? '' : ' is-offline'}`} role="img" aria-label={label} title={label} />
        <button type="button" className="mx-icon-button mx-rail-button" aria-label="Settings" title={tip('Settings', 'settings')} onClick={props.onOpenSettings}><Icon name="gear" /></button>
      </div>
    </div>
  )
}
