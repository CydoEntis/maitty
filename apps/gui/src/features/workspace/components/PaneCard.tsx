import { useEffect, useRef, useState, type MouseEvent, type ReactNode } from 'react'
import { MAX_PANES, type PaneSlot, type SplitDirection } from '../model/layout'
import type { SessionView } from '../model/session'
import { headerFit } from '../model/paneHeader'
import { keepsRunningUnseen, rowTitle } from '../model/sidebar'
import { AppIcon, Icon, StatusBadge } from './glyphs'
import { beginPointerDrag } from './dragSession'
import { PaneViewSwitch } from './PaneViewSwitch'
import { MenuPopup, type MenuEntry } from './MenuPopup'
import { menuPoint } from './menuPoint'
import { CHAT_AGENTS, type ChatAgent, type PaneView } from '../model/chatView'
import { agentName } from '../model/agentMessage'
import { SecretAskBar } from './SecretAskBar'
import { pullRequestHeadline, pullRequestTone } from '../model/pullRequest'
import type { TerminalDock } from '@/features/settings/preferences'
import { useShortcutText, useTipWithKeys } from '@/features/shortcuts/platform'

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
  /** This pane is the one being dragged. */
  dragging: boolean
  /** What letting go over this pane does, said on it while it is the target. */
  dropWord: string
  terminal: ReactNode
  /** The view the pane shows. The terminal stays mounted under the chat, so it keeps streaming and keeps its size. */
  view?: PaneView
  /** Present when the pane has a terminal | chat switch; picking a view sends nothing to the daemon. */
  onViewChange?: (view: PaneView) => void
  /** The chat that covers the terminal while `view` is `chat`. */
  chat?: ReactNode
  /** The side panel is open for this pane. */
  panelOpen: boolean
  /** Opens the side panel for this pane, or closes it; absent when the panel has nothing to show for it. */
  onPanel?: () => void
  /** Lines the repository says changed, for the dot on the side-panel button. */
  changed: { added: number; removed: number }
  /** For an agent that can have terminals of its own: where they show. Null for a shell, which is one. */
  terminalDock: TerminalDock | null
  onNewTerminal: () => void
  onDock: (dock: TerminalDock) => void
  /** The agent's terminals when they are pinned under its chat, and whether that drawer is folded away. */
  drawer?: ReactNode
  drawerFolded?: boolean
  onDrawerFold?: (folded: boolean) => void
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
  onRename: () => void
  /** Starts another agent beside this one's work, with a brief of it. */
  onHandoff: (agent: ChatAgent) => void
  /** Starts a new shell in an ended pane's folder, in its place. */
  onRestart: (paneId: string) => void
  /** Answers the session's request to start helpers; absent where it cannot be answered. */
  onAnswerHelpers?: (choice: 'allow' | 'always' | 'no') => void
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
  // One pane of several closes alone when its session has work in it (`useClosing`); otherwise closing ends the session.
  const closeLabel = multi && keepsRunningUnseen(session) ? 'Close pane · its session keeps running' : 'Close pane · ends its session'
  const hasChanges = props.changed.added + props.changed.removed > 0
  const panelTip = hasChanges ? `Side panel · +${String(props.changed.added)} −${String(props.changed.removed)}` : 'Side panel'
  const drawerTip = props.drawerFolded === true ? 'Show the terminals' : 'Hide the terminals'

  // An agent's work can move to another agent; a shell or an ended session has none to move.
  const handoffItems: MenuEntry[] = CHAT_AGENTS.includes(session.process as ChatAgent) && !ended
    ? CHAT_AGENTS.filter((agent) => agent !== session.process).map((agent) => ({ label: `Hand off to ${agentName(agent)}`, disabled: !props.online, run: () => { props.onHandoff(agent) } }))
    : []

  const menuItems: MenuEntry[] = [
    { label: 'Split right', hint: shortcutText('split-right'), disabled: full, run: () => { props.onSplit(pane.id, 'right') } },
    { label: 'Split down', hint: shortcutText('split-down'), disabled: full, run: () => { props.onSplit(pane.id, 'down') } },
    ...(props.onPanel ? [{ label: 'Changes and files', hint: shortcutText('inspect'), run: props.onPanel }] : []),
    ...(props.terminalDock === null ? [] : [
      props.terminalDock === 'panel'
        ? { label: 'Pin terminals under chat', run: () => { props.onDock('under') } }
        : { label: 'Move terminals to side panel', run: () => { props.onDock('panel') } },
    ]),
    ...handoffItems,
    'separator',
    { label: 'Rename…', hint: 'F2', run: props.onRename },
    { label: maximized ? 'Restore layout' : 'Maximize pane', disabled: !multi && !maximized, run: () => { props.onToggleMaximize(pane.id) } },
    ...(props.onViewChange
      ? [{ label: view === 'chat' ? 'Show terminal' : 'Show chat', hint: shortcutText('chat-toggle'), run: () => { props.onViewChange?.(view === 'chat' ? 'terminal' : 'chat') } }]
      : []),
    ...(props.terminalDock === null
      ? [{ label: 'Clear', hint: 'Ctrl+L', disabled: !props.online || ended, run: props.onClear }, { label: 'Copy folder path', run: props.onCopyPath }]
      : [{ label: 'New terminal', disabled: !props.online, run: props.onNewTerminal }]),
    // The only pane has nothing to close but its session, which the sidebar's row does.
    ...(multi ? ['separator' as const, { label: 'Close pane', danger: true, run: () => { props.onClose(pane.id) } }] : []),
  ]

  return (
    <>
    <section
      ref={box}
      className={`mx-pane${view === 'chat' ? ' is-chat' : ' is-terminal'}${multi ? ' is-card' : ''}${focused && multi ? ' is-focused' : ''}${props.broadcast ? ' is-broadcast' : ''}${props.dropTarget ? ' is-drop-target' : ''}${props.dragging ? ' is-dragging' : ''}`}
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
            {session.branch && (session.worktree
              ? <span className="mx-pane-branch is-worktree" title={`Worktree on branch ${session.branch}`}><Icon name="worktree" size={10} />{session.branch}</span>
              : <span className="mx-pane-branch" title={`Branch ${session.branch}`}><Icon name="branch" size={10} />{session.branch}</span>)}
            {session.pr && (
              <span className={`mx-pane-pr is-${pullRequestTone(session.pr)}`} title={`${session.pr.title} (${session.pr.url})`}>{pullRequestHeadline(session.pr)}</span>
            )}
          </span>
        </span>
        {props.broadcast && <span className="mx-broadcast-pill">◉ Broadcast</span>}
        <span className="mx-pane-controls">
          {props.onViewChange && fit.switcher && <PaneViewSwitch view={view} onChange={props.onViewChange} />}
          {fit.split && <button type="button" className="mx-pane-control" aria-label={tip('Split right', 'split-right')} title={tip('Split right', 'split-right')} disabled={full} onMouseDown={stop} onClick={() => { props.onSplit(pane.id, 'right') }}><Icon name="splitRight" /></button>}
          {props.drawer !== undefined && props.onDrawerFold && (
            <button type="button" className={`mx-pane-control${props.drawerFolded === true ? '' : ' is-on'}`} aria-pressed={props.drawerFolded !== true} aria-label={drawerTip} title={drawerTip} onMouseDown={stop} onClick={() => { props.onDrawerFold?.(props.drawerFolded !== true) }}><Icon name="drawer" /></button>
          )}
          {props.onPanel && (
            <button type="button" className={`mx-pane-control mx-pane-panel${props.panelOpen ? ' is-on' : ''}`} aria-pressed={props.panelOpen} aria-label="Side panel" title={tip(panelTip, 'inspect')} onMouseDown={stop} onClick={props.onPanel}>
              <Icon name="sidePanel" />
              {hasChanges && <span className="mx-pane-dot" aria-hidden="true" />}
            </button>
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
      {session.helperAsk.length > 0 && props.onAnswerHelpers && (
        <div className="mx-pane-ended is-ask" role="alert" aria-label="Wants to start helpers">
          <span className="mx-pane-ended-text">
            <strong>{`Wants to start ${String(session.helperAsk.length)} helper${session.helperAsk.length === 1 ? '' : 's'}`}</strong>
            <small title={session.helperAsk.join('\n')}>{session.helperAsk.join(' · ')}</small>
          </span>
          <button type="button" className="mx-pane-ended-button is-primary" disabled={!props.online} title="This session may start helpers from now on" onMouseDown={stop} onClick={() => { props.onAnswerHelpers?.('allow') }}>Allow</button>
          <button type="button" className="mx-pane-ended-button" disabled={!props.online} title="Every session of this project may, from now on" onMouseDown={stop} onClick={() => { props.onAnswerHelpers?.('always') }}>Always in this project</button>
          <button type="button" className="mx-pane-ended-button" disabled={!props.online} title="It does the work alone" onMouseDown={stop} onClick={() => { props.onAnswerHelpers?.('no') }}>No</button>
        </div>
      )}
      {session.secretAsk.map((ask) => <SecretAskBar key={ask.id} ask={ask} machine={session.machine} online={props.online} />)}
      {ended && (
        <div className="mx-pane-ended" role="status" aria-label="Session ended">
          <span className="mx-pane-ended-text"><strong>Session ended</strong><small>Its output is kept above.</small></span>
          <button type="button" className="mx-pane-ended-button is-primary" disabled={!props.online} onMouseDown={stop} onClick={() => { props.onRestart(pane.id) }}>Restart shell</button>
          <button type="button" className="mx-pane-ended-button" onMouseDown={stop} onClick={() => { props.onClose(pane.id) }}>Close pane</button>
        </div>
      )}
      {props.dropTarget && <span className="mx-pane-drop" aria-hidden="true">{props.dropWord}</span>}
      <span className="mx-pane-ring" aria-hidden="true" />
    </section>
    {/* A card of its own under the pane, not a part of it. */}
    {props.drawer}
    {/* Beside the pane, not in it: a pane has `contain: paint`, which would place the menu's fixed anchor from the pane's corner. */}
    {menuAt && <MenuPopup key={`${String(menuAt.x)},${String(menuAt.y)}`} title={title} untitled at={menuAt} items={menuItems} portalContainer={props.portalContainer} onClose={() => { setMenuAt(null) }} />}
    </>
  )
}
