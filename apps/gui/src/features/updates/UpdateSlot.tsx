import { useEffect, useLayoutEffect, useRef, useState, type CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import { Icon } from '@/features/workspace/components/glyphs'
import type { UpdaterState } from './useUpdater'

const MAX_NOTES = 5
const PERCENT = 100
const CARD_GAP_PX = 8
const CARD_EDGE_PX = 12
const CARD_WIDTH_PX = 360

type Props = {
  variant: 'footer' | 'rail'
  state: UpdaterState
  /** The running version. */
  version: string | null
  /** A release the user chose to skip: it gets no pill. */
  skipped: string | null
  portalContainer: HTMLElement | null
  onInstall: () => void
  onRestartNow: () => void
  onCheck: () => void
  onSkip: (version: string) => void
}

function noteLines(notes: string): string[] {
  return notes.split('\n').map((line) => line.replace(/^[-*]\s*/, '').trim()).filter(Boolean).slice(0, MAX_NOTES)
}

type Offer = Exclude<UpdaterState, { status: 'idle' | 'checking' | 'current' }>

function offerOf(state: UpdaterState, skipped: string | null): Offer | null {
  if (state.status === 'available') return state.version === skipped ? null : state
  if (state.status === 'downloading' || state.status === 'ready' || state.status === 'failed') return state
  return null
}

/** The sidebar footer's left slot: the version, or what an update needs from you, with a card that opens upward. */
export function UpdateSlot({ variant, state, version, skipped, portalContainer, onInstall, onRestartNow, onCheck, onSkip }: Props) {
  const [open, setOpen] = useState(false)
  const [place, setPlace] = useState<{ left: number; bottom: number } | null>(null)
  const trigger = useRef<HTMLButtonElement>(null)
  const card = useRef<HTMLDivElement>(null)
  const offer = offerOf(state, skipped)

  useLayoutEffect(() => {
    if (!open || !trigger.current) return
    const rect = trigger.current.getBoundingClientRect()
    const onRight = portalContainer?.dataset.side === 'right'
    const left = onRight ? rect.right - CARD_WIDTH_PX : rect.left
    setPlace({ left: Math.max(CARD_EDGE_PX, left), bottom: window.innerHeight - rect.top + CARD_GAP_PX })
  }, [open, portalContainer])

  useEffect(() => {
    if (!open) return
    const onKeyDown = (event: KeyboardEvent) => {
      if (event.key !== 'Escape') return
      event.stopPropagation()
      setOpen(false)
      trigger.current?.focus()
    }
    const onPointerDown = (event: PointerEvent) => {
      const target = event.target
      if (!(target instanceof Node)) return
      if (card.current?.contains(target) || trigger.current?.contains(target)) return
      setOpen(false)
    }
    document.addEventListener('keydown', onKeyDown, true)
    document.addEventListener('pointerdown', onPointerDown, true)
    return () => {
      document.removeEventListener('keydown', onKeyDown, true)
      document.removeEventListener('pointerdown', onPointerDown, true)
    }
  }, [open])

  if (!offer) {
    if (variant === 'rail' || version === null) return null
    return <span className="mx-version">Version <span className="mono">{version}</span></span>
  }

  const percent = offer.status === 'downloading' && offer.progress !== null ? Math.round(offer.progress * PERCENT) : null
  const label = offer.status === 'available' ? `Update to ${offer.version}`
    : offer.status === 'downloading' ? (percent === null ? 'Downloading…' : `${String(percent)}%`)
    : offer.status === 'ready' ? 'Restart to update'
    : 'Update failed'
  const cardBody = (() => {
    switch (offer.status) {
      case 'available': {
        const lines = noteLines(offer.notes)
        return (
          <>
            <h3>Maitty {offer.version} is available</h3>
            {version && <p className="mx-upd-sub">You have {version}</p>}
            {lines.length > 0 && <ul className="mx-upd-notes">{lines.map((line) => <li key={line}>{line}</li>)}</ul>}
            <div className="mx-upd-actions">
              <button type="button" className="mx-upd-primary" onClick={() => { onInstall() }}>Download</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { onSkip(offer.version); setOpen(false) }}>Skip this version</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { setOpen(false) }}>Later</button>
            </div>
          </>
        )
      }
      case 'downloading':
        return (
          <>
            <h3>Downloading Maitty {offer.version}</h3>
            <div className="mx-upd-bar" role="progressbar" aria-valuemin={0} aria-valuemax={PERCENT} aria-valuenow={percent ?? undefined}><span style={{ width: `${String(percent ?? 0)}%` }} /></div>
            <p className="mx-upd-sub">{percent === null ? 'Working out the size…' : `${String(percent)}%`}</p>
            <div className="mx-upd-actions"><button type="button" className="mx-upd-ghost" onClick={() => { setOpen(false) }}>Hide</button></div>
          </>
        )
      case 'ready':
        return (
          <>
            <h3>Maitty {offer.version} is ready</h3>
            <ul className="mx-upd-notes is-safe">
              <li>Your sessions keep running in maittyd.</li>
              <li>Your tabs and splits reopen.</li>
              <li>Agents in the middle of a turn are not interrupted.</li>
            </ul>
            <div className="mx-upd-actions">
              <button type="button" className="mx-upd-primary" onClick={() => { setOpen(false); onRestartNow() }}>Restart now</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { setOpen(false) }}>On next launch</button>
            </div>
          </>
        )
      case 'failed':
        return (
          <>
            <h3>Update failed</h3>
            <p className="mx-upd-error">{offer.message}</p>
            <div className="mx-upd-actions">
              <button type="button" className="mx-upd-primary" onClick={() => { onCheck() }}>Retry</button>
              <button type="button" className="mx-upd-ghost" onClick={() => { void navigator.clipboard.writeText(offer.message) }}>Copy details</button>
            </div>
          </>
        )
    }
  })()

  return (
    <>
      <button
        ref={trigger}
        type="button"
        className={`mx-upd-pill is-${offer.status} is-${variant}`}
        aria-haspopup="dialog"
        aria-expanded={open}
        aria-label={variant === 'rail' ? `${label}. Open update details` : undefined}
        style={percent === null ? undefined : { '--p': `${String(percent)}%` } as CSSProperties}
        onClick={() => { setOpen((current) => !current) }}
      >
        {variant === 'rail' ? <Icon name="download" /> : (
          <>
            {offer.status === 'available' && <span className="mx-upd-dot" aria-hidden="true" />}
            {offer.status === 'downloading' && <span className="mx-upd-ring" aria-hidden="true" />}
            {label}
          </>
        )}
      </button>
      {open && place && createPortal(
        <div ref={card} role="dialog" aria-label="Update" className="mx-upd-card" style={{ left: place.left, bottom: place.bottom }}>{cardBody}</div>,
        portalContainer ?? document.body,
      )}
    </>
  )
}
