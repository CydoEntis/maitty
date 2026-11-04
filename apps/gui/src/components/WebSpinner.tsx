import type { ReactNode } from 'react'
import type { WebSpinner as WebSpinnerKind } from '@/lib/spinner-clock'

const SPOKE_COUNT = 8
const FULL_TURN_DEGREES = 360
const SPOKE_STEP_DEGREES = FULL_TURN_DEGREES / SPOKE_COUNT
const CENTRE = 12

/** The shapes are drawn once; CSS in workspace.css (`.mx-wspin`) moves them with transform and opacity only. */
const DRAWINGS: Record<WebSpinnerKind, ReactNode> = {
  circle: (
    <>
      <circle cx="12" cy="12" r="9" fill="none" stroke="currentColor" strokeWidth="3" opacity="0.25" />
      <path d="M12 3a9 9 0 0 1 9 9" fill="none" stroke="currentColor" strokeWidth="3" strokeLinecap="round" />
    </>
  ),
  dots: (
    <>
      <circle className="mx-wspin-dot" cx="5" cy="12" r="2.6" />
      <circle className="mx-wspin-dot" cx="12" cy="12" r="2.6" />
      <circle className="mx-wspin-dot" cx="19" cy="12" r="2.6" />
    </>
  ),
  equalizer: (
    <>
      <rect className="mx-wspin-bar" x="4" y="3" width="4" height="18" rx="1.5" />
      <rect className="mx-wspin-bar" x="10" y="3" width="4" height="18" rx="1.5" />
      <rect className="mx-wspin-bar" x="16" y="3" width="4" height="18" rx="1.5" />
    </>
  ),
  spokes: (
    <>
      {Array.from({ length: SPOKE_COUNT }, (_, index) => (
        <line key={index} x1="12" y1="2.5" x2="12" y2="7" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" opacity={(index + 1) / SPOKE_COUNT} transform={`rotate(${String(index * SPOKE_STEP_DEGREES)} ${String(CENTRE)} ${String(CENTRE)})`} />
      ))}
    </>
  ),
  ping: (
    <>
      <circle cx="12" cy="12" r="3.2" />
      <circle className="mx-wspin-ring" cx="12" cy="12" r="10" fill="none" stroke="currentColor" strokeWidth="2" />
    </>
  ),
}

/**
 * A web spinner: inline SVG in the text colour, 1em square, in its own `contain: paint; will-change:
 * transform` span so a frame repaints nothing around it. `still` (Reduce motion, a frozen row) shows
 * the shape without its animation.
 */
export function WebSpinner({ kind, className = 'mx-spin', still = false }: { kind: WebSpinnerKind; className?: string; still?: boolean }) {
  const classes = [className, 'mx-wspin', still && 'is-still'].filter(Boolean).join(' ')
  return (
    <span className={classes} data-kind={kind} aria-hidden="true">
      <svg viewBox="0 0 24 24" fill="currentColor" focusable="false">{DRAWINGS[kind]}</svg>
    </span>
  )
}
