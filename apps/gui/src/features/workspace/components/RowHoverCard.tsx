import { useLayoutEffect, useRef, useState } from 'react'
import { WindowPortal } from '@/components/ui/portal'
import type { SessionView } from '../model/session'
import { useNow } from '@/lib/clock'
import { hoverCardText, placeHoverCard, statusWhyLine, type Box } from '../model/hoverCard'
import { rowTitle } from '../model/sidebar'
import { Ago } from './times'

type Props = { session: SessionView; row: Box; portalContainer: HTMLElement | null }

/**
 * The card beside a sidebar row the pointer rests on: what the session is, what it is doing, and its
 * changes. It only tells; it takes no pointer input, so it can never be in the way of the next row.
 */
export function RowHoverCard({ session, row, portalContainer }: Props) {
  const card = useRef<HTMLDivElement>(null)
  const [at, setAt] = useState<{ left: number; top: number } | null>(null)
  useLayoutEffect(() => {
    const height = card.current?.getBoundingClientRect().height ?? 0
    setAt(placeHoverCard(row, height, window.innerWidth, window.innerHeight))
  }, [row, session])
  const text = hoverCardText(session, rowTitle(session))
  const why = statusWhyLine(session, useNow())
  return (
    <WindowPortal container={portalContainer}>
    <div ref={card} role="tooltip" className="mx-hover-card" style={at === null ? { visibility: 'hidden' } : at}>
      <span className="mx-hover-head"><b>{text.title}</b><small>{text.meta}</small></span>
      <span className={`mx-hover-status is-${text.status}`}>
        <i aria-hidden="true" />
        <span><b>{text.headline}</b><span>{text.line}</span></span>
      </span>
      <span className="mx-hover-facts">
        <span>Agent</span><span>{text.agent}</span>
        <span>Changes</span><span>{text.changes}</span>
        {why !== null && <><span>Set by</span><span>{why}</span></>}
        <span>Last activity</span><span>{session.statusSince === null ? 'Not known' : <><Ago at={session.statusSince} /> ago</>}</span>
      </span>
    </div>
    </WindowPortal>
  )
}
