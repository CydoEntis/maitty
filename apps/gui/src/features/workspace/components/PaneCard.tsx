import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react'
import { MAX_PANES, type PaneSlot, type SplitDirection } from '../model/layout'
import type { SessionView } from '../model/session'
import { headerFit } from '../model/paneHeader'
import { rowTitle } from '../model/sidebar'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { beginPointerDrag } from './dragSession'
import { PaneViewSwitch } from './PaneViewSwitch'
import { MenuPopup, type MenuEntry } from './MenuPopup'
import { menuPoint } from './menuPoint'
import type { PaneView } from '../model/chatView'
import type { InspectView } from '@/features/inspect/InspectPanel'
import { useShortcutText, useTipWithKeys } from '@/features/shortcuts/platform'

/** The Files and Changes tray: which of the two the repository drawer shows for this pane, if any. */
export type PaneTool = Extract<InspectView, 'files' | 'changes'>

export type PaneCardProps = {
  pane: PaneSlot
  index: number
  session: SessionView
  paneCount: number
  /** Typing in any pane of this layout goes to its idle ones too. */
  broadcast: boolean
  focused: boolean
  maximized: boolean
  dropTarget: boolean
  terminal: ReactNode
  /** The view the pane shows. The terminal stays mounted under the chat, so it keeps streaming and keeps its size. */
  view?: PaneView
  /** Present when the pane has a terminal | chat switch; picking a view sends nothing to the daemon. */
  onViewChange?: (view: PaneView) => void
  /** The chat that covers the terminal while `view` is `chat`. */
  chat?: ReactNode
  /** Which tool of the tray is showing for this pane; null when the drawer is closed or shows another pane. */
  tool: PaneTool | null
  onTool: (tool: PaneTool) => void
  /** For an agent: whether a terminal you opened in its folder is open. Null for a shell, which has its own. */
  terminalOpen: boolean | null
  onToggleTerminal: () => void
  online: boolean
  /** Where portalled menus mount, so they read the theme's colours. */
  portalContainer: HTMLElement | null
  onFocus: (paneId: string) => void
  onSplit: (paneId: string, direction: SplitDirection) => void
  onToggleMaximize: (paneId: string) => void
  onClose: (paneId: string) => void
  /** Clears a shell's screen, as Ctrl+L does in it. */
  onClear: () => void
  onCopyPath: () => void
  /** Starts a new shell in an ended pane's folder, in its place. */
  onRestart: (paneId: string) => void
}

/** The pane's width as it is laid out, so its header can give up controls as it narrows. */
function useWidth(): [React.RefObject<HTMLElement | null>, number | null] {
  const element = useRef<HTMLElement>(null)
  const [width, setWidth] = useState<number | null>(null)
  useEffect(() => {
    const target = element.current
    if (!target || typeof ResizeObserver === 'undefined') return
    const observer = new ResizeObserver((entries) => {
      const box = entries[0]?.contentRect
      if (box) setWidth(Math.round(box.width))
    })
    observer.observe(target)
    return () => { observer.disconnect() }
  }, [])
  return [element, width]
}

const stop = (event: MouseEvent) => { event.stopPropagation() }

export function PaneCard(props: PaneCardProps) {
  const { pane, session, paneCount, focused, maximized } = props
  const tip = useTipWithKeys()
  const shortcutText = useShortcutText()
  const [box, width] = useWidth()
  const [menuAt, setMenuAt] = useState<{ x: number; y: number } | null>(null)
  const multi = paneCount > 1
  const full = paneCount >= MAX_PANES
  const ended = session.ended
  const view = props.view ?? 'terminal'
  const fit = headerFit(width)
  const title = rowTitle(session)
  const closeLabel = 'Close pane · ends its session'
  const toggleTool = (tool: PaneTool) => { props.onTool(tool) }

  const menuItems: MenuEntry[] = [
    { label: 'Split right', hint: shortcutText('split-right'), disabled: full, run: () => { props.onSplit(pane.id, 'right') } },
    { label: 'Split down', hint: shortcutText('split-down'), disabled: full, run: () => { props.onSplit(pane.id, 'down') } },
    { label: maximized ? 'Restore layout' : 'Maximize pane', disabled: !multi && !maximized, run: () => { props.onToggleMaximize(pane.id) } },
    'separator',
    ...(props.onViewChange
      ? [{ label: view === 'chat' ? 'Show terminal' : 'Show chat', hint: shortcutText('chat-toggle'), run: () => { props.onViewChange?.(view === 'chat' ? 'terminal' : 'chat') } }]
      : []),
    ...(props.terminalOpen === null
      ? [{ label: 'Clear', hint: 'Ctrl+L', disabled: !props.online || ended, run: props.onClear }, { label: 'Copy folder path', run: props.onCopyPath }]
      : [{ label: props.terminalOpen ? 'Close its terminal' : 'Open terminal here', disabled: !props.online, run: props.onToggleTerminal }]),
    { label: 'Files', run: () => { toggleTool('files') } },
    { label: 'Changes', run: () => { toggleTool('changes') } },
    'separator',
    { label: 'Close pane', danger: true, run: () => { props.onClose(pane.id) } },
  ]

  return (
    <section
      ref={box}
      className={`mx-pane${view === 'chat' ? ' is-chat' : ' is-terminal'}${multi ? ' is-card' : ''}${focused && multi ? ' is-focused' : ''}${props.broadcast ? ' is-broadcast' : ''}${props.dropTarget ? ' is-drop-target' : ''}`}
      aria-label={`Terminal ${String(props.index + 1)}: ${session.cwdLabel}${focused ? ', focused' : ''}`}
      data-session-id={session.id}
      data-pane={pane.id}
      onMouseDown={() => { props.onFocus(pane.id) }}
      onContextMenu={(event) => {
        // The terminal's own area opens the pane menu too; only the chat's text boxes keep the system menu.
        if (event.target instanceof Element && event.target.closest('textarea:not(.xterm-helper-textarea), input')) return
        event.preventDefault()
        setMenuAt(menuPoint(event, false))
      }}
    >
      <div
        className={`mx-pane-head${multi ? ' is-draggable' : ''}`}
        title={multi ? 'Drag to move this pane' : undefined}
        onPointerDown={(event) => {
          // Buttons in the header keep their own clicks; only the bar itself starts a drag.
          if (multi && !(event.target instanceof Element && event.target.closest('button, input'))) beginPointerDrag(event, { kind: 'pane', id: pane.id, label: title })
        }}
        onDoubleClick={() => { props.onToggleMaximize(pane.id) }}
      >
        <span className="mx-pane-icon">
          <AppIcon process={session.process} size={26} dimmed={session.status === 'unknown'} />
          <StatusBadge status={session.status} />
        </span>
        <span className="mx-pane-title">
          <span className="mx-pane-name">{title}</span>
          <span className="mx-pane-where">
            <span className="mx-pane-cwd" title={session.cwd}>{session.cwdLabel}</span>
            {session.branch && <span className="mx-pane-branch" title={`Branch ${session.branch}`}><Icon name="branch" size={10} />{session.branch}</span>}
          </span>
        </span>
        {props.broadcast && <span className="mx-broadcast-pill">◉ Broadcast</span>}
        <span className="mx-pane-controls">
          {props.onViewChange && fit.switcher && <PaneViewSwitch view={view} onChange={props.onViewChange} />}
          {fit.tools && (
            <>
              <span className="mx-pane-gap" aria-hidden="true" />
              <span className="mx-pane-tray" role="group" aria-label="Repository">
                <button type="button" className={`mx-tray-button${props.tool === 'files' ? ' is-on' : ''}`} aria-pressed={props.tool === 'files'} aria-label="Files" title={tip('Files', 'inspect')} onMouseDown={stop} onClick={() => { toggleTool('files') }}><Icon name="files" size={13} />{fit.toolLabels && 'Files'}</button>
                <button type="button" className={`mx-tray-button${props.tool === 'changes' ? ' is-on' : ''}`} aria-pressed={props.tool === 'changes'} aria-label="Changes" title="Changes" onMouseDown={stop} onClick={() => { toggleTool('changes') }}><Icon name="plus" size={13} />{fit.toolLabels && 'Changes'}</button>
              </span>
              <span className="mx-pane-divider" aria-hidden="true" />
            </>
          )}
          {fit.splits && (
            <>
              <button type="button" className="mx-pane-control" aria-label={tip('Split right', 'split-right')} title={tip('Split right', 'split-right')} disabled={full} onMouseDown={stop} onClick={() => { props.onSplit(pane.id, 'right') }}><Icon name="splitRight" /></button>
              <button type="button" className="mx-pane-control" aria-label={tip('Split down', 'split-down')} title={tip('Split down', 'split-down')} disabled={full} onMouseDown={stop} onClick={() => { props.onSplit(pane.id, 'down') }}><Icon name="splitDown" /></button>
              <button type="button" className="mx-pane-control" aria-label={maximized ? 'Restore layout' : 'Maximize pane'} title={maximized ? 'Restore layout' : 'Maximize pane'} disabled={!multi && !maximized} onMouseDown={stop} onClick={() => { props.onToggleMaximize(pane.id) }}><Icon name={maximized ? 'restore' : 'maximize'} /></button>
            </>
          )}
          <button type="button" className="mx-pane-control mx-pane-more" aria-label="More pane actions" title="More pane actions" aria-haspopup="menu" onMouseDown={stop} onClick={(event) => { setMenuAt(menuPoint(event, true)) }}>⋯</button>
          <button type="button" className="mx-pane-control is-close" aria-label={closeLabel} title={closeLabel} onMouseDown={stop} onClick={() => { props.onClose(pane.id) }}><Icon name="close" /></button>
        </span>
      </div>
      <div
        className={`mx-pane-body${view === 'chat' ? ' is-chat' : ''}`}
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
      <span className="mx-pane-ring" aria-hidden="true" />
      {menuAt && <MenuPopup title={title} at={menuAt} items={menuItems} portalContainer={props.portalContainer} onClose={() => { setMenuAt(null) }} />}
    </section>
  )
}
