import type { DragEvent, MouseEvent, ReactNode } from 'react'
import { ContextMenu as ContextMenuPrimitive } from 'radix-ui'
import { MAX_PANES, type PaneSlot } from '../model/layout'
import type { SessionView } from '../model/session'
import { AppIcon, Icon } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { PaneStatus } from './PaneStatus'

export type PaneCardProps = {
  pane: PaneSlot
  index: number
  session: SessionView
  paneCount: number
  /** Typing in any pane of this tab goes to the idle ones too. */
  broadcast: boolean
  onToggleBroadcast: () => void
  focused: boolean
  maximized: boolean
  dropTarget: boolean
  workingSince: number | null
  now: number
  primary: string
  shift: string
  terminal: ReactNode
  online: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  onFocus: (paneId: string) => void
  onSplit: (paneId: string, direction: 'right' | 'down') => void
  onToggleMaximize: (paneId: string) => void
  onClose: (paneId: string) => void
  onEndSession: (sessionId: string) => void
  onRenameSession: (sessionId: string) => void
  onDragStart: (paneId: string) => void
  onDragEnd: () => void
  /** `fromSidebar` is true when a session from the sidebar is being dragged over, not another pane. */
  onDragOver: (paneId: string, fromSidebar: boolean) => void
  /** `sessionId` is set when a session from the sidebar was dropped. */
  onDrop: (paneId: string, sessionId: string | null) => void
}

const stop = (event: MouseEvent) => { event.stopPropagation() }

/** A menu that opens a dialog must not hand focus back to the pane it came from: that would take it from the dialog. */
const keepFocus = (event: Event) => { event.preventDefault() }

export function PaneCard(props: PaneCardProps) {
  const { pane, session, paneCount, focused, maximized, primary, shift } = props
  const multi = paneCount > 1
  const ringed = focused && multi
  const full = paneCount >= MAX_PANES
  const status = session.status
  return (
    <ContextMenuPrimitive.Root>
      <ContextMenuPrimitive.Trigger asChild>
    <section
      className={`mx-pane${ringed ? ' is-focused' : ''}${props.broadcast ? ' is-broadcast' : ''}${props.dropTarget ? ' is-drop-target' : ''}`}
      aria-label={`Terminal ${String(props.index + 1)}: ${session.cwdLabel}${ringed ? ', focused' : ''}`}
      onMouseDown={() => { props.onFocus(pane.id) }}
      onDragOver={(event: DragEvent) => {
        event.preventDefault()
        props.onDragOver(pane.id, event.dataTransfer.types.includes(SESSION_DRAG_TYPE))
      }}
      onDrop={(event: DragEvent) => {
        event.preventDefault()
        props.onDrop(pane.id, event.dataTransfer.getData(SESSION_DRAG_TYPE) || null)
      }}
    >
      <div
        className={`mx-pane-head${multi ? ' is-draggable' : ''}`}
        draggable={multi}
        onDragStart={(event) => {
          event.dataTransfer.effectAllowed = 'move'
          event.dataTransfer.setData('text/plain', pane.id)
          props.onDragStart(pane.id)
        }}
        onDragEnd={props.onDragEnd}
        onDoubleClick={() => { props.onToggleMaximize(pane.id) }}
      >
        <AppIcon process={session.process} size={20} dimmed={status === 'unknown'} />
        <span className="mx-pane-title">
          <span className="mx-pane-proc">{session.process}</span>
          {session.name && <span className="mx-pane-name">{session.name}</span>}
          <span className="mx-pane-cwd">{session.cwdLabel}</span>
        </span>
        {props.broadcast && <span className="mx-broadcast-pill">◉ Broadcast</span>}
        {session.repoName && <span className="mx-pill mx-repo-pill" title={session.repoRoot ?? undefined}><Icon name="branch" />{session.repoName}</span>}
        <span className="mx-pane-controls">
          <button type="button" className={`mx-pane-control${props.broadcast ? ' is-on' : ''}`} aria-label={props.broadcast ? 'Stop broadcasting' : 'Broadcast typing to this tab'} aria-pressed={props.broadcast} title={`Broadcast to idle panes in this tab (${primary}${shift}B)`} disabled={!multi} onMouseDown={stop} onClick={props.onToggleBroadcast}><Icon name="broadcast" /></button>
          <button type="button" className="mx-pane-control" aria-label={`Split right (${primary}D)`} title={`Split right (${primary}D)`} disabled={full} onMouseDown={stop} onClick={() => { props.onSplit(pane.id, 'right') }}><Icon name="splitRight" /></button>
          <button type="button" className="mx-pane-control" aria-label={`Split down (${primary}${shift}D)`} title={`Split down (${primary}${shift}D)`} disabled={full} onMouseDown={stop} onClick={() => { props.onSplit(pane.id, 'down') }}><Icon name="splitDown" /></button>
          <button type="button" className="mx-pane-control" aria-label={maximized ? 'Restore layout' : 'Maximize pane'} title={maximized ? 'Restore layout' : 'Maximize pane'} disabled={!multi && !maximized} onMouseDown={stop} onClick={() => { props.onToggleMaximize(pane.id) }}>{maximized ? <Icon name="restore" /> : <Icon name="maximize" />}</button>
          <button type="button" className="mx-pane-control" aria-label="Close pane · the session keeps running" title="Close pane · the session keeps running" disabled={!multi} onMouseDown={stop} onClick={() => { props.onClose(pane.id) }}><Icon name="close" /></button>
        </span>
      </div>
      <div
        className="mx-pane-body"
        onMouseDown={(event) => {
          // A click on the pane's padding still puts the cursor in its terminal.
          if (event.target !== event.currentTarget) return
          event.preventDefault()
          event.currentTarget.querySelector<HTMLTextAreaElement>('.xterm-helper-textarea')?.focus()
        }}
      >
        {props.terminal}
      </div>
      <PaneStatus status={status} workingSince={props.workingSince} now={props.now} />
    </section>
      </ContextMenuPrimitive.Trigger>
      <ContextMenuPrimitive.Portal container={props.portalContainer}>
        <ContextMenuPrimitive.Content className="mx-menu" onCloseAutoFocus={keepFocus}>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={full} onSelect={() => { props.onSplit(pane.id, 'right') }}>Split right<span className="mx-menu-hint">{primary}D</span></ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={full} onSelect={() => { props.onSplit(pane.id, 'down') }}>Split down<span className="mx-menu-hint">{primary}{shift}D</span></ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!multi && !maximized} onSelect={() => { props.onToggleMaximize(pane.id) }}>{maximized ? 'Restore layout' : 'Maximize pane'}</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { props.onRenameSession(session.id) }}>Rename…</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { void navigator.clipboard.writeText(session.cwd) }}>Copy folder path</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!multi} onSelect={() => { props.onClose(pane.id) }}>Close pane<span className="mx-menu-hint">keeps running</span></ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item is-danger" disabled={!props.online} onSelect={() => { props.onEndSession(session.id) }}>Close session<span className="mx-menu-hint">ends it</span></ContextMenuPrimitive.Item>
        </ContextMenuPrimitive.Content>
      </ContextMenuPrimitive.Portal>
    </ContextMenuPrimitive.Root>
  )
}
