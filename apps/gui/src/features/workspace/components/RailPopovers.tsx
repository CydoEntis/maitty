import type { CSSProperties, ReactNode } from 'react'
import { createPortal } from 'react-dom'
import { displayName, STATUS_WORD, truncatePrompt, type SessionGroup, type SessionView } from '../model/session'
import { folderStyle } from './folderStyle'
import { AppIcon, Icon } from './glyphs'
import type { RailHover } from './railHover'

const GAP_PX = 10
/** Keeps a popover for a row near the window's top or bottom inside the window. */
const EDGE_PX = 90
const HALF = 0.5

function Frame({ anchor, hover, className, children }: { anchor: DOMRect; hover: RailHover; className: string; children: ReactNode }) {
  const centre = anchor.top + anchor.height * HALF
  const top = Math.min(Math.max(centre, EDGE_PX), window.innerHeight - EDGE_PX)
  const style = { top, left: anchor.right + GAP_PX, '--arrow-shift': `${String(centre - top)}px` } as CSSProperties
  return (
    <div role="tooltip" className={`mx-railpop ${className}`} style={style} onMouseEnter={hover.hold} onMouseLeave={hover.leave}>
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
        {group.branch && <span className="mx-rp-branch"><Icon name="branch" />{group.branch}</span>}
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

function SessionPopover({ session, group, tabNumber }: { session: SessionView; group: SessionGroup; tabNumber: number | undefined }) {
  const detail = session.status === 'needs' && session.asking ? session.asking : session.lastPrompt
  return (
    <>
      <header className="mx-rp-head">
        <span className="mx-rp-dot" style={folderStyle(group.key, group.colorId)} aria-hidden="true" />
        <span className="mx-rp-title"><strong>{group.name} · {displayName(session)}</strong></span>
        <StateWord session={session} />
      </header>
      <div className="mx-rp-path">{session.process} · {session.cwdLabel}</div>
      {detail && <p className={`mx-rp-well${session.status === 'needs' ? ' is-needs' : ''}`}>{truncatePrompt(detail)}</p>}
      <footer className="mx-rp-hint">{tabNumber !== undefined ? `In tab ${String(tabNumber)} · ` : ''}right-click for more</footer>
    </>
  )
}

type LayerProps = {
  hover: RailHover
  groups: readonly SessionGroup[]
  tabOf: ReadonlyMap<string, number>
  portalContainer: HTMLElement | null
  onShowSession: (id: string) => void
}

/** The open rail popover, beside the item it is about. */
export function RailPopoverLayer({ hover, groups, tabOf, portalContainer, onShowSession }: LayerProps) {
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
  return createPortal(<Frame anchor={shown.anchor} hover={hover} className="is-session"><SessionPopover session={session} group={group} tabNumber={tabOf.get(id)} /></Frame>, portalContainer)
}
