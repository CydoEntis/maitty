import { useEffect, useState, type DragEvent, type MouseEvent, type ReactNode } from 'react'
import { ContextMenu as ContextMenuPrimitive } from 'radix-ui'
import { MAX_PANES, type PaneSlot, type SplitDirection } from '../model/layout'
import type { SessionView } from '../model/session'
import { AppIcon, Icon } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { runsProgram, useArmed } from './useArmed'
import { PaneStatus } from './PaneStatus'
import { PaneViewSwitch } from './PaneViewSwitch'
import type { PaneView } from '../model/chatView'
import { usageParts } from '../model/usage'
import { useShortcutText, useTipWithKeys } from '@/features/shortcuts/platform'

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
  terminal: ReactNode
  /** The view the pane shows. The terminal stays mounted under the chat, so it keeps streaming and keeps its size. */
  view?: PaneView
  /** Present when the pane has a Terminal | Chat switch; picking a view sends nothing to the daemon. */
  onViewChange?: (view: PaneView) => void
  /** The chat that covers the terminal while `view` is `chat`. */
  chat?: ReactNode
  online: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  onFocus: (paneId: string) => void
  onSplit: (paneId: string, direction: SplitDirection) => void
  onToggleMaximize: (paneId: string) => void
  onClose: (paneId: string) => void
  /** Starts a new shell in an ended pane's folder, in its place. */
  onRestart: (paneId: string) => void
  onEndSession: (sessionId: string) => void
  onRenameSession: (sessionId: string) => void
  onDragStart: (paneId: string) => void
  onDragEnd: () => void
  /** `fromSidebar` is true when a session from the sidebar is being dragged over, not another pane. */
  onDragOver: (paneId: string, fromSidebar: boolean) => void
  /** `sessionId` is set when a session from the sidebar was dropped. */
  onDrop: (paneId: string, sessionId: string | null) => void
}

/** How long the terminal's slide back in lasts; a little longer than its animation. */
const RETURN_MS = 460

/** True for a moment after the pane goes from the chat back to the terminal, so the terminal plays its slide in once. */
function useReturning(view: PaneView): boolean {
  const [seen, setSeen] = useState(view)
  const [returning, setReturning] = useState(false)
  if (seen !== view) {
    setSeen(view)
    setReturning(view === 'terminal')
  }
  useEffect(() => {
    if (!returning) return
    const timer = window.setTimeout(() => { setReturning(false) }, RETURN_MS)
    return () => { window.clearTimeout(timer) }
  }, [returning])
  return returning
}

const stop = (event: MouseEvent) => { event.stopPropagation() }

/** A menu that opens a dialog must not hand focus back to the pane it came from: that would take it from the dialog. */
const keepFocus = (event: Event) => { event.preventDefault() }

export function PaneCard(props: PaneCardProps) {
  const { pane, session, paneCount, focused, maximized } = props
  const tip = useTipWithKeys()
  const shortcutText = useShortcutText()
  const multi = paneCount > 1
  const full = paneCount >= MAX_PANES
  const status = session.status
  const ended = session.ended
  const closing = useArmed(runsProgram(session))
  const view = props.view ?? 'terminal'
  const returning = useReturning(view)
  const closeLabel = closing.armed ? `Click again to close and stop ${session.process}` : 'Close pane · ends its session'
  return (
    <ContextMenuPrimitive.Root>
      <ContextMenuPrimitive.Trigger asChild>
    <section
      className={`mx-pane${view === 'chat' ? '' : ' is-terminal'}${focused ? ' is-focused' : ''}${props.broadcast ? ' is-broadcast' : ''}${props.dropTarget ? ' is-drop-target' : ''}`}
      aria-label={`Terminal ${String(props.index + 1)}: ${session.cwdLabel}${focused ? ', focused' : ''}`}
      data-session-id={session.id}
      data-pane={pane.id}
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
        {usageParts(session.usage).length > 0 && <span className="mx-pill mx-usage-pill" title="What the agent reports this session has used">{usageParts(session.usage).join(' · ')}</span>}
        {session.repoName && <span className="mx-pill mx-repo-pill" title={session.repoRoot ?? undefined}><Icon name="branch" />{session.repoName}</span>}
        <span className="mx-pane-controls">
          {props.onViewChange && <PaneViewSwitch view={view} onChange={props.onViewChange} />}
          <button type="button" className={`mx-pane-control${props.broadcast ? ' is-on' : ''}`} aria-label={props.broadcast ? 'Stop broadcasting' : 'Broadcast typing to this tab'} aria-pressed={props.broadcast} title={tip('Broadcast to idle panes in this tab', 'broadcast')} disabled={!multi} onMouseDown={stop} onClick={props.onToggleBroadcast}><Icon name="broadcast" /></button>
          <button type="button" className="mx-pane-control" aria-label={tip('Split right', 'split-right')} title={tip('Split right', 'split-right')} disabled={full} onMouseDown={stop} onClick={() => { props.onSplit(pane.id, 'right') }}><Icon name="splitRight" /></button>
          <button type="button" className="mx-pane-control" aria-label={tip('Split down', 'split-down')} title={tip('Split down', 'split-down')} disabled={full} onMouseDown={stop} onClick={() => { props.onSplit(pane.id, 'down') }}><Icon name="splitDown" /></button>
          <button type="button" className="mx-pane-control" aria-label={maximized ? 'Restore layout' : 'Maximize pane'} title={maximized ? 'Restore layout' : 'Maximize pane'} disabled={!multi && !maximized} onMouseDown={stop} onClick={() => { props.onToggleMaximize(pane.id) }}>{maximized ? <Icon name="restore" /> : <Icon name="maximize" />}</button>
          <button type="button" className={`mx-pane-control is-close${closing.armed ? ' is-armed' : ''}`} aria-label={closeLabel} title={closeLabel} onMouseDown={stop} onBlur={closing.disarm} onClick={() => { if (closing.press()) props.onClose(pane.id) }}><Icon name="close" /></button>
        </span>
      </div>
      <div
        className={`mx-pane-body${view === 'chat' ? ' is-chat' : ''}${returning ? ' mx-term-in' : ''}`}
        onMouseDown={(event) => {
          // A click on the pane's padding still puts the cursor in its terminal.
          if (event.target !== event.currentTarget) return
          event.preventDefault()
          event.currentTarget.querySelector<HTMLTextAreaElement>('.xterm-helper-textarea')?.focus()
        }}
      >
        {props.terminal}
        {view === 'chat' && props.chat}
      </div>
      {ended && (
        <div className="mx-pane-ended" role="status" aria-label="Session ended">
          <span className="mx-pane-ended-text"><strong>Session ended</strong><small>Its output is kept above.</small></span>
          <button type="button" className="mx-pane-ended-button is-primary" disabled={!props.online} onMouseDown={stop} onClick={() => { props.onRestart(pane.id) }}>Restart shell</button>
          <button type="button" className="mx-pane-ended-button" onMouseDown={stop} onClick={() => { props.onClose(pane.id) }}>Close pane</button>
        </div>
      )}
      {!ended && status !== 'idle' && view === 'terminal' && <PaneStatus status={status} workingSince={props.workingSince} />}
    </section>
      </ContextMenuPrimitive.Trigger>
      <ContextMenuPrimitive.Portal container={props.portalContainer}>
        <ContextMenuPrimitive.Content className="mx-menu" onCloseAutoFocus={keepFocus}>
          <ContextMenuPrimitive.Sub>
            <ContextMenuPrimitive.SubTrigger className="mx-menu-item" disabled={full}>Split<span className="mx-menu-hint">›</span></ContextMenuPrimitive.SubTrigger>
            <ContextMenuPrimitive.Portal container={props.portalContainer}>
              <ContextMenuPrimitive.SubContent className="mx-menu" sideOffset={6}>
                <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { props.onSplit(pane.id, 'up') }}>Split up</ContextMenuPrimitive.Item>
                <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { props.onSplit(pane.id, 'down') }}>Split down<span className="mx-menu-hint">{shortcutText('split-down')}</span></ContextMenuPrimitive.Item>
                <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { props.onSplit(pane.id, 'left') }}>Split left</ContextMenuPrimitive.Item>
                <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { props.onSplit(pane.id, 'right') }}>Split right<span className="mx-menu-hint">{shortcutText('split-right')}</span></ContextMenuPrimitive.Item>
              </ContextMenuPrimitive.SubContent>
            </ContextMenuPrimitive.Portal>
          </ContextMenuPrimitive.Sub>
          <ContextMenuPrimitive.Item className="mx-menu-item" disabled={!multi && !maximized} onSelect={() => { props.onToggleMaximize(pane.id) }}>{maximized ? 'Restore layout' : 'Maximize pane'}</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { props.onRenameSession(session.id) }}>Rename…</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Item className="mx-menu-item" onSelect={() => { void navigator.clipboard.writeText(session.cwd) }}>Copy folder path</ContextMenuPrimitive.Item>
          <ContextMenuPrimitive.Separator className="mx-menu-sep" />
          <ContextMenuPrimitive.Item className={`mx-menu-item is-danger${closing.armed ? ' is-armed' : ''}`} disabled={!props.online && !ended} onSelect={(event) => { if (ended) props.onClose(pane.id); else if (closing.press()) props.onEndSession(session.id); else event.preventDefault() }}>{ended ? 'Close pane' : closing.armed ? `Close and stop ${session.process}? Click again` : 'Close session'}<span className="mx-menu-hint">{ended ? '' : closing.armed ? '' : 'ends it'}</span></ContextMenuPrimitive.Item>
        </ContextMenuPrimitive.Content>
      </ContextMenuPrimitive.Portal>
    </ContextMenuPrimitive.Root>
  )
}
