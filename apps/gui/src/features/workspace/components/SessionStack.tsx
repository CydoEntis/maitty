import { splitNote } from '../model/splitFold'
import { useEffect, useState, type ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { displayName, iconSaysProcess, STATUS_WORD, type SessionGroup, type SessionStatus, type SessionView } from '../model/session'
import { chooseCard, planStack, byPriority, type Activity } from '../model/stack'
import type { SessionTrack } from '../model/tracking'
import { Ago, ElapsedSince } from './times'
import type { TrayStyle } from '@/features/settings/preferences'
import { AppIcon, Icon, PauseBars, Spinner, StatusBadge } from './glyphs'
import { SESSION_DRAG_TYPE } from './dragSession'
import { SessionRow } from './SessionRow'
import { hoverHandlers, type RailHover } from './railHover'
import { useSessionMenu, type SessionMenuActions } from './SessionMenu'

const SWAP_MS = 320
/** The icons style lays its items out on a grid six wide, whatever is in it. */
const ICON_COLUMNS = 6

/** How far beside the sidebar the +N menu opens, and how close to the window's top it may sit. */
const MENU_GAP_PX = 8
const MENU_WIDTH_PX = 300
const MENU_TOP_MARGIN_PX = 12
const MENU_RISE_PX = 60

/** The words and colour a tray item or menu row gives a state; the idle ones say nothing. */
const STATE_WORD: Record<SessionView['status'], string> = { working: '', needs: 'needs you', done: 'done', error: 'exited', idle: '', unknown: 'unknown' }

function stateWord(session: SessionView, track: SessionTrack | undefined): ReactNode {
  if (session.status === 'working') return track?.workingSince ? <ElapsedSince since={track.workingSince} /> : ''
  return STATE_WORD[session.status]
}

export type StackProps = {
  group: SessionGroup
  focusedId: string | null
  /** The session last picked in this project, used as its main row while you are in another project. */
  remembered: string | undefined
  trayStyle: TrayStyle
  tracks: ReadonlyMap<string, SessionTrack>
  online: boolean
  /** The pane (counting from 1) each session has in its tab; absent for a session no tab shows. */
  paneOf: ReadonlyMap<string, number>
  /** How many quiet shell splits are folded into each session's row (see `foldSplits`). */
  splitsOf: ReadonlyMap<string, number>
  portalContainer: HTMLElement | null
  /** Opens a session's popover beside its row or tray item. */
  hover: RailHover
  onShowSession: (id: string) => void
  onSetName: (id: string, name: string) => void
  /** Opens the rename dialog, for a session with no room to rename it in place. */
  onRenameSession: (id: string) => void
  onPinSession: (id: string, pinned: boolean) => void
  onEndSession: (id: string) => void
}

/** True for a moment after `value` changes, so a row can play its hand-over animation once. */
function useChanged(value: string | undefined): boolean {
  const [seen, setSeen] = useState(value)
  const [changed, setChanged] = useState(false)
  if (seen !== value) {
    setSeen(value)
    setChanged(seen !== undefined)
  }
  useEffect(() => {
    if (!changed) return
    const timer = window.setTimeout(() => { setChanged(false) }, SWAP_MS)
    return () => { window.clearTimeout(timer) }
  }, [changed])
  return changed
}

/**
 * One project in the sidebar. Its sessions keep creation order: the ones created before the card are
 * compact tree rows above it, the card is the focused session in its own place, and the ones after it
 * follow in the chosen style. Clicking a compact row grows it into the card where it is; nothing moves.
 */
export function SessionStack(props: StackProps) {
  const { group, tracks } = props
  const activity: Activity = (id) => tracks.get(id)?.changedAt ?? 0
  const card = chooseCard(group.sessions, props.focusedId, props.remembered)
  const swapped = useChanged(card?.id)
  const [menuAnchor, setMenuAnchor] = useState<DOMRect | null>(null)
  if (!card) return null
  const plan = planStack(group.sessions, card, props.trayStyle)
  const hasAfter = plan.shown.length > 0
  const trayLabel = `Other sessions in ${group.name}`
  const moreLabel = `All ${String(group.sessions.length)} sessions in ${group.name}${plan.hiddenNeeds ? ', some hidden ones need you' : ''}`
  const stacked = props.trayStyle === 'stacked'
  const tree = props.trayStyle === 'tree'
  const itemActions = (session: SessionView): SessionMenuActions => ({ online: props.online, portalContainer: props.portalContainer, onShow: () => { props.onShowSession(session.id) }, onRename: () => { props.onRenameSession(session.id) }, onPin: (pinned) => { props.onPinSession(session.id, pinned) }, onEnd: () => { props.onEndSession(session.id) } })
  const treeRow = (session: SessionView, lastOfTree: boolean) => <TreeRow key={session.id} hover={props.hover} session={session} track={tracks.get(session.id)} selected={session.id === props.focusedId} pane={props.paneOf.get(session.id)} splits={props.splitsOf.get(session.id)} last={lastOfTree} actions={itemActions(session)} />
  const moreButton = (
    <button type="button" className={`mx-tray-more${plan.hiddenNeeds ? ' is-needs' : ''}${menuAnchor ? ' is-open' : ''}`} aria-haspopup="menu" aria-expanded={menuAnchor !== null} aria-label={moreLabel} title={moreLabel} onClick={(event) => { setMenuAnchor(menuAnchor ? null : event.currentTarget.getBoundingClientRect()) }}>
      {stacked || tree ? `${String(plan.hidden)} more` : `+${String(plan.hidden)}`}
    </button>
  )
  return (
    <div className={`mx-stack${hasAfter ? ' has-tray' : ''}`}>
      {plan.before.length > 0 && <div role="group" aria-label={trayLabel} className="mx-tree">{plan.before.map((session) => treeRow(session, false))}</div>}
      <SessionRow
        key={card.id}
        session={card}
        track={tracks.get(card.id)}
        selected={card.id === props.focusedId}
        online={props.online}
        splits={props.splitsOf.get(card.id)}
        swapped={swapped}
        portalContainer={props.portalContainer}
        hover={props.hover}
        onShow={() => { props.onShowSession(card.id) }}
        onSetName={(name) => { props.onSetName(card.id, name) }}
        onPin={(pinned) => { props.onPinSession(card.id, pinned) }}
        onEnd={() => { props.onEndSession(card.id) }}
      />
      {hasAfter && tree && (
        <div role="group" aria-label={trayLabel} className="mx-tree">
          {plan.shown.map((session, index) => treeRow(session, plan.hidden === 0 && index === plan.shown.length - 1))}
          {plan.hidden > 0 && <div className="mx-tree-more">{moreButton}</div>}
        </div>
      )}
      {hasAfter && !tree && (
        <div role="group" aria-label={trayLabel} className={`mx-tray is-${props.trayStyle}${plan.hidden > 0 ? ' has-more' : ''}`} style={stacked ? undefined : { gridTemplateColumns: trayColumns(props.trayStyle, plan.shown.length, plan.hidden > 0) }}>
          {plan.shown.map((session) => <TrayItem key={session.id} hover={props.hover} session={session} style={props.trayStyle} track={tracks.get(session.id)} selected={session.id === props.focusedId} pane={props.paneOf.get(session.id)} splits={props.splitsOf.get(session.id)} actions={itemActions(session)} />)}
          {plan.hidden > 0 && moreButton}
        </div>
      )}
      {menuAnchor && props.portalContainer && createPortal(<StackMenu {...props} anchor={menuAnchor} activity={activity} onClose={() => { setMenuAnchor(null) }} />, props.portalContainer)}
    </div>
  )
}

/** Names and icons share a grid of equal columns; +N takes a narrow one at the end. */
function trayColumns(style: TrayStyle, shown: number, more: boolean): string {
  const columns = style === 'icons' ? `repeat(${String(more ? shown : Math.max(shown, ICON_COLUMNS))}, minmax(0, 1fr))` : `repeat(${String(shown)}, minmax(0, 1fr))`
  return more ? `${columns} 30px` : columns
}

type TreeRowProps = { session: SessionView; track: SessionTrack | undefined; selected: boolean; pane: number | undefined; splits: number | undefined; last: boolean; actions: SessionMenuActions; hover: RailHover }

/** The state as one small mark: a spinner, the needs-you square, a check, a bang, a quiet dot or a dashed square. */
function TreeMark({ status }: { status: SessionStatus }) {
  switch (status) {
    case 'working': return <Spinner className="mx-tree-spin" />
    case 'needs': return <span className="mx-tree-needs"><PauseBars /></span>
    case 'done': return <span className="mx-tree-done"><Icon name="check" size={11} strokeWidth={3.2} /></span>
    case 'error': return <span className="mx-tree-error">!</span>
    case 'unknown': return <span className="mx-tree-unknown" />
    case 'idle': return <span className="mx-tree-idle" />
  }
}

/** A compact tree row: guide line and elbow, the state, the program, where it is and a state word. */
function TreeRow({ session, track, selected, pane, splits, last, actions, hover }: TreeRowProps) {
  const menu = useSessionMenu(session, actions)
  const unread = Boolean(track?.unread) && !selected
  const where = pane === undefined ? 'detached' : `pane ${String(pane)}`
  const label = `${displayName(session)}, ${session.process}, ${STATUS_WORD[session.status]}, ${where}`
  return (
    <>
      <button type="button" className={`mx-tree-item is-${session.status}${last ? ' is-last' : ''}`} draggable onDragStart={(event) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData(SESSION_DRAG_TYPE, session.id) }} {...hoverHandlers(hover, { kind: 'session', id: session.id })} onContextMenu={(event) => { hover.close(); menu.onContextMenu(event) }} aria-label={label} onClick={actions.onShow}>
        <span className="mx-tree-guide" aria-hidden="true" />
        <span className="mx-tree-elbow" aria-hidden="true" />
        <span className="mx-tree-mark"><TreeMark status={session.status} /></span>
        <span className="mx-tree-proc">{session.process.split(/\s+/)[0]}</span>
        <span className="mx-tree-where">{displayName(session)}</span>
        {splits !== undefined && <span className="mx-split-count" title="Quiet shell panes folded into this row">{splitNote(splits)}</span>}
        {unread && <span className="mx-tree-unread" aria-label="Unread" />}
        <span className={`mx-tree-word is-${session.status}`}>{stateWord(session, track)}</span>
      </button>
      {menu.element}
    </>
  )
}

type TrayItemProps = { session: SessionView; style: TrayStyle; track: SessionTrack | undefined; selected: boolean; pane: number | undefined; splits: number | undefined; actions: SessionMenuActions; hover: RailHover }

/** One of a project's other sessions: its icon with the state on it, and the process for the named styles. */
function TrayItem({ session, style, track, selected, pane, splits, actions, hover }: TrayItemProps) {
  const menu = useSessionMenu(session, actions)
  const unread = Boolean(track?.unread) && !selected
  const where = pane === undefined ? 'detached' : `pane ${String(pane)}`
  const label = `${displayName(session)}, ${session.process}, ${STATUS_WORD[session.status]}, ${where}`
  const word = stateWord(session, track)
  // The program's name, unless its icon says it. A name you gave takes that place in the named style (the stacked one writes it after).
  const program = iconSaysProcess(session.process) ? null : session.process.split(/\s+/)[0]
  const leadLabel = style === 'icons' ? null : style === 'names' ? (session.name ?? program) : program
  return (
    <>
      <button type="button" className={`mx-tray-item is-${style} is-${session.status}`} draggable onDragStart={(event) => { event.dataTransfer.effectAllowed = 'move'; event.dataTransfer.setData(SESSION_DRAG_TYPE, session.id) }} {...hoverHandlers(hover, { kind: 'session', id: session.id })} onContextMenu={(event) => { hover.close(); menu.onContextMenu(event) }} aria-label={label} onClick={actions.onShow}>
        <span className="mx-tray-icon">
          <AppIcon process={session.process} size={18} dimmed={session.status === 'unknown'} />
          <StatusBadge status={session.status} rail />
          {unread && <span className="mx-tray-unread" aria-label="Unread" />}
        </span>
        {leadLabel && <span className="mx-tray-proc">{leadLabel}</span>}
        {style === 'stacked' && <span className="mx-tray-where">{displayName(session)}</span>}
        {splits !== undefined && style !== 'icons' && <span className="mx-split-count" title="Quiet shell panes folded into this row">{splitNote(splits)}</span>}
        {style === 'stacked' && word && <span className={`mx-tray-word is-${session.status}`}>{word}</span>}
      </button>
      {menu.element}
    </>
  )
}

type MenuProps = StackProps & { anchor: DOMRect; activity: Activity; onClose: () => void }

/** Every session of a project, most urgent first, in a menu beside the sidebar. */
function StackMenu({ group, focusedId, tracks, anchor, activity, portalContainer, onShowSession, onClose }: MenuProps) {
  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.stopPropagation()
      onClose()
    }
    window.addEventListener('keydown', onKey, true)
    return () => { window.removeEventListener('keydown', onKey, true) }
  }, [onClose])
  const sidebar = portalContainer?.querySelector('nav.mx-sidebar')?.getBoundingClientRect()
  const onRight = portalContainer?.dataset.side === 'right'
  const left = onRight ? (sidebar?.left ?? anchor.left) - MENU_GAP_PX - MENU_WIDTH_PX : (sidebar?.right ?? anchor.right) + MENU_GAP_PX
  const top = Math.max(MENU_TOP_MARGIN_PX, anchor.top - MENU_RISE_PX)
  const sessions = [...group.sessions].sort(byPriority(activity))
  return (
    <>
      <div className="mx-menu-scrim" onMouseDown={onClose} aria-hidden="true" />
      <div role="menu" aria-label={group.name} className="mx-stack-menu" style={{ left, top }}>
        <div className="mx-stack-menu-head">
          <span className="mx-stack-menu-title">{group.name}</span>
          <span className="mx-stack-menu-count">{group.sessions.length} sessions</span>
          <span className="mx-spacer" />
          <button type="button" className="mx-icon-button" aria-label="Close" title="Close (esc)" onClick={onClose}><Icon name="close" size={11} strokeWidth={2.2} /></button>
        </div>
        <div className="mx-stack-menu-list">
          {sessions.map((session) => {
            const track = tracks.get(session.id)
            const selected = session.id === focusedId
            const status = session.status
            const word = status === 'working' ? 'Working' : status === 'needs' ? 'Needs you' : status === 'done' ? 'Done' : status === 'error' ? 'Exited' : status === 'unknown' ? 'Unknown' : 'Idle'
            const meta = status === 'working' ? (track?.workingSince ? <ElapsedSince since={track.workingSince} /> : '') : track?.changedAt ? <Ago at={track.changedAt} /> : ''
            return (
              <button key={session.id} type="button" role="menuitem" className={`mx-stack-menu-row${selected ? ' is-selected' : ''}`} onClick={() => { onClose(); onShowSession(session.id) }}>
                <span className="mx-tray-icon"><AppIcon process={session.process} size={24} dimmed={status === 'unknown'} /><StatusBadge status={status} rail /></span>
                <span className="mx-stack-menu-text">
                  <span className="mx-stack-menu-name">{displayName(session)}{track?.unread && !selected && <span className="mx-unread" aria-label="Unread" />}</span>
                  <span className="mx-stack-menu-proc">{session.process}</span>
                </span>
                <span className="mx-stack-menu-state">
                  <span className={`mx-stack-menu-word is-${status}`}>{status === 'working' && <Spinner />}{word}</span>
                  <span className="mx-stack-menu-meta">{meta}</span>
                </span>
              </button>
            )
          })}
        </div>
      </div>
    </>
  )
}
