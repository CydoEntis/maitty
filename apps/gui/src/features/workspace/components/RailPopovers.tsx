import type { CSSProperties, ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { displayName, STATUS_WORD, truncatePrompt, type SessionGroup, type SessionView } from '../model/session'
import { folderStyle } from './folderStyle'
import { formatAgo, type SessionTrack } from '../model/tracking'
import { AppIcon, Icon, Spinner } from './glyphs'
import type { RailHover } from './railHover'

const GAP_PX = 10
const PROJECT_WIDTH_PX = 260
const SESSION_WIDTH_PX = 300
/** Keeps a popover for a row near the window's top or bottom inside the window. */
const EDGE_PX = 90
const HALF = 0.5

function Frame({ anchor, hover, className, children }: { anchor: DOMRect; hover: RailHover; className: string; children: ReactNode }) {
  const centre = anchor.top + anchor.height * HALF
  const top = Math.min(Math.max(centre, EDGE_PX), window.innerHeight - EDGE_PX)
  // With the sidebar on the right, the popover opens toward the middle of the window.
  const onRight = document.querySelector('.mx-app')?.getAttribute('data-side') === 'right'
  const width = className.includes('is-session') ? SESSION_WIDTH_PX : PROJECT_WIDTH_PX
  const left = onRight ? anchor.left - GAP_PX - width : anchor.right + GAP_PX
  const style = { top, left, '--arrow-shift': `${String(centre - top)}px` } as CSSProperties
  return (
    <div role="tooltip" className={`mx-railpop ${className}${onRight ? ' is-left' : ''}`} style={style} onMouseEnter={hover.hold} onMouseLeave={hover.leave}>
      {children}
    </div>
  )
}

function StateWord({ session }: { session: SessionView }) {
  return <span className={`mx-rp-state is-${session.status}`}>{STATUS_WORD[session.status]}</span>
}

function ProjectPopover({ group, hover, onShowSession }: { group: SessionGroup; hover: RailHover; onShowSession: (id: string) => void }) {
  return (
    <>
      <header className="mx-rp-head">
        <span className="mx-folder-tile" style={folderStyle(group.key, group.colorId)}><Icon name="folder" /></span>
        <span className="mx-rp-title">
          <strong>{group.name}</strong>
          <span className="mx-rp-path">{group.path}</span>
        </span>
      </header>
      <ul className="mx-rp-list">
        {group.sessions.map((session) => (
          <li key={session.id}>
            <button type="button" className="mx-rp-row" onClick={() => { onShowSession(session.id); hover.close() }}>
              <AppIcon process={session.process} size={20} dimmed={session.status === 'unknown'} />
              <span className="mx-rp-text">
                <span className="mx-rp-name">{displayName(session)}</span>
                <span className="mx-rp-proc">{session.process}</span>
              </span>
              <StateWord session={session} />
            </button>
          </li>
        ))}
      </ul>
      <footer className="mx-rp-hint">Click the folder to collapse · right-click for more</footer>
    </>
  )
}

type Activity = { glyph: string; tone: 'ok' | 'warn' | 'accent' | 'err'; title: string; detail?: string }

const ACTIVITY_LINES = 2
const ASKED_PREVIEW_CHARS = 90

/** What the session last did, in plain words: up to two lines, the most pressing first. */
function activityOf(session: SessionView): Activity[] {
  const lines: Activity[] = []
  if (session.status === 'needs' && session.asking) lines.push({ glyph: '›', tone: 'warn', title: 'Wants to run a command', detail: truncatePrompt(session.asking) })
  if (session.status === 'error') lines.push({ glyph: '!', tone: 'err', title: 'Exited with an error' })
  if (session.lastPrompt) lines.push({ glyph: '❯', tone: 'accent', title: `You asked: ${truncatePrompt(session.lastPrompt, ASKED_PREVIEW_CHARS)}` })
  const finished = [...session.timeline].reverse().find((event) => event.kind === 'CommandFinished')
  if (finished) {
    const failed = finished.detail !== '' && finished.detail !== '0'
    lines.push({ glyph: failed ? '!' : '✓', tone: failed ? 'err' : 'ok', title: 'Command finished', ...(finished.detail === '' ? {} : { detail: `exit ${finished.detail}` }) })
  }
  return lines.slice(0, ACTIVITY_LINES)
}

function PaneRow({ session, current }: { session: SessionView; current: boolean }) {
  return (
    <li className={`mx-sc-pane${current ? ' is-current' : ''}`}>
      <AppIcon process={session.process} size={18} />
      <span className="mx-sc-pane-name">{session.process}</span>
      <span className="mx-sc-pane-dir">{displayName(session)}</span>
      <span className={`mx-rp-state is-${session.status}`}>{STATUS_WORD[session.status]}</span>
    </li>
  )
}

function SessionPopover({ session, group, tabNumber, panes, changedAt, now }: { session: SessionView; group: SessionGroup; tabNumber: number | undefined; panes: readonly SessionView[]; changedAt: number | null; now: number }) {
  const activity = activityOf(session)
  const ago = changedAt === null ? '' : formatAgo(now - changedAt)
  return (
    <>
      <header className="mx-sc-head">
        <AppIcon process={session.process} size={22} />
        <span className="mx-sc-title">
          <span className="mx-sc-project">{group.name}</span>
          <strong>{displayName(session)}</strong>
        </span>
        <span className={`mx-sc-chip is-${session.status}`}>{session.status === 'working' && <Spinner />}{STATUS_WORD[session.status]}</span>
      </header>
      <div className="mx-sc-meta">
        <span className="mx-sc-path" dir="rtl"><bdi>{session.process} in {session.cwdLabel}</bdi></span>
        <span className="mx-sc-where">{tabNumber !== undefined ? `Tab ${String(tabNumber)} · open` : 'Not open in a tab'}{ago ? ` · ${ago}` : ''}</span>
      </div>
      {activity.length > 0 && (
        <ul className="mx-sc-activity" aria-label="Last activity">
          {activity.map((line) => (
            <li key={line.title}>
              <span className={`mx-sc-glyph is-${line.tone}`} aria-hidden="true">{line.glyph}</span>
              <span className="mx-sc-line"><span>{line.title}</span>{line.detail && <small>{line.detail}</small>}</span>
            </li>
          ))}
        </ul>
      )}
      {panes.length > 1 && (
        <section aria-label={`All ${String(panes.length)} panes in this tab`}>
          <h4 className="mx-sc-panes-head">All {panes.length} panes in this tab</h4>
          <ul className="mx-sc-panes">{panes.map((pane) => <PaneRow key={pane.id} session={pane} current={pane.id === session.id} />)}</ul>
        </section>
      )}
      <footer className="mx-rp-hint">right-click to rename or pin</footer>
    </>
  )
}

type LayerProps = {
  hover: RailHover
  groups: readonly SessionGroup[]
  tabOf: ReadonlyMap<string, number>
  /** Every pane of the tab a session is open in, for sessions whose tab has more than one. */
  tabPanes: ReadonlyMap<string, readonly SessionView[]>
  tracks: ReadonlyMap<string, SessionTrack>
  now: number
  portalContainer: HTMLElement | null
  onShowSession: (id: string) => void
}

/** The open rail popover, beside the item it is about. */
export function RailPopoverLayer({ hover, groups, tabOf, tabPanes, tracks, now, portalContainer, onShowSession }: LayerProps) {
  const { shown } = hover
  if (!shown || !portalContainer) return null
  if (shown.target.kind === 'folder') {
    const { key } = shown.target
    const group = groups.find((candidate) => candidate.key === key)
    if (!group) return null
    return createPortal(<Frame anchor={shown.anchor} hover={hover} className="is-project"><ProjectPopover group={group} hover={hover} onShowSession={onShowSession} /></Frame>, portalContainer)
  }
  const { id } = shown.target
  const group = groups.find((candidate) => candidate.sessions.some((session) => session.id === id))
  const session = group?.sessions.find((candidate) => candidate.id === id)
  if (!group || !session) return null
  return createPortal(<Frame anchor={shown.anchor} hover={hover} className="is-session"><SessionPopover session={session} group={group} tabNumber={tabOf.get(id)} panes={tabPanes.get(id) ?? []} changedAt={tracks.get(id)?.changedAt ?? null} now={now} /></Frame>, portalContainer)
}
