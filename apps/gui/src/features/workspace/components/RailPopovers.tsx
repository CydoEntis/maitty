import type { CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import { folderNameOf, rowName, STATUS_WORD, statusText, truncatePrompt, type SessionView } from '../model/session'
import type { SessionTrack } from '../model/tracking'
import { Ago } from './times'
import { AppIcon, Spinner } from './glyphs'
import type { RailHover } from './railHover'

const GAP_PX = 10
const CARD_WIDTH_PX = 300
/** Keeps a card for a row near the window's top or bottom inside the window. */
const EDGE_PX = 90
const HALF = 0.5

/** What the session is saying right now, in plain words: the exact request when it asks, else its state. */
function phraseOf(session: SessionView): string {
  if (session.status === 'needs' && session.asking) return truncatePrompt(session.asking)
  return statusText(session)
}

function SessionCard({ session, changedAt }: { session: SessionView; changedAt: number | null }) {
  return (
    <>
      <header className="mx-sc-head">
        <AppIcon process={session.process} size={22} />
        <span className="mx-sc-title">
          <span className="mx-sc-project">{folderNameOf(session)}</span>
          <strong>{rowName(session)}</strong>
        </span>
        <span className={`mx-sc-chip is-${session.status}`}>{session.status === 'working' && <Spinner />}{STATUS_WORD[session.status]}</span>
      </header>
      <div className="mx-sc-meta">
        <span className="mx-sc-path" dir="rtl"><bdi>{session.cwdLabel}</bdi></span>
        <span className="mx-sc-where">{session.machineName ?? 'This computer'}{changedAt !== null && <Ago at={changedAt} prefix=" · " />}</span>
      </div>
      <p className="mx-sc-phrase">{phraseOf(session)}</p>
    </>
  )
}

type LayerProps = {
  hover: RailHover
  sessions: readonly SessionView[]
  tracks: ReadonlyMap<string, SessionTrack>
  portalContainer: HTMLElement | null
}

/** The rail's hover card: the name, folder, machine and status of the session under the pointer. */
export function RailPopoverLayer({ hover, sessions, tracks, portalContainer }: LayerProps) {
  const { shown } = hover
  if (!shown || !portalContainer) return null
  const session = sessions.find((candidate) => candidate.id === shown.id)
  if (!session) return null
  const centre = shown.anchor.top + shown.anchor.height * HALF
  const top = Math.min(Math.max(centre, EDGE_PX), window.innerHeight - EDGE_PX)
  // With the sidebar on the right, the card opens toward the middle of the window.
  const onRight = document.querySelector('.mx-app')?.getAttribute('data-side') === 'right'
  const left = onRight ? shown.anchor.left - GAP_PX - CARD_WIDTH_PX : shown.anchor.right + GAP_PX
  const style = { top, left, '--arrow-shift': `${String(centre - top)}px` } as CSSProperties
  return createPortal(
    <div role="tooltip" className={`mx-railpop is-session${onRight ? ' is-left' : ''}`} style={style} onMouseEnter={hover.hold} onMouseLeave={hover.leave}>
      <SessionCard session={session} changedAt={tracks.get(session.id)?.changedAt ?? null} />
    </div>,
    portalContainer,
  )
}
