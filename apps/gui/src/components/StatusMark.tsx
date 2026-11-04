import { useEffect, useRef } from 'react'
import { cn } from '@/lib/utils'
import { attachSpinner, currentGlyph, isWebSpinner, spinnerFrames, useSpinnerStyle } from '@/lib/spinner-clock'
import { WebSpinner } from './WebSpinner'

export type StatusMarkState =
  'working' | 'needs-you' | 'done' | 'idle' | 'recovery' | 'unknown'

function prefersReducedMotion() {
  return (
    document.documentElement.dataset.reducedMotion === 'true' ||
    (typeof window.matchMedia === 'function' &&
      window.matchMedia('(prefers-reduced-motion: reduce)').matches)
  )
}

function Spinner() {
  // Reduced motion shows one static frame instead of animating.
  const still = prefersReducedMotion()
  const style = useSpinnerStyle()
  return isWebSpinner(style) ? <WebSpinner kind={style} className="" still={still} /> : <TextSpinner still={still} />
}

function TextSpinner({ still }: { still: boolean }) {
  const element = useRef<HTMLSpanElement>(null)
  useEffect(() => {
    if (still || !element.current) return
    return attachSpinner(element.current)
  }, [still])
  return <span ref={element}>{still ? spinnerFrames()[0] : currentGlyph()}</span>
}

/** Each state has a different shape, so color is never the only signal. */
export function StatusMark({
  status,
  className,
}: {
  status: StatusMarkState
  className?: string
}) {
  return (
    <span
      className={cn('status-mark', `status-${status}`, className)}
      aria-hidden="true"
    >
      {status === 'working' && <Spinner />}
      {status === 'needs-you' && (
        <>
          <i />
          <i />
        </>
      )}
      {status === 'done' && (
        <svg viewBox="0 0 12 12" width="9" height="9" fill="none">
          <path
            d="M2 6.5 4.8 9 10 3.2"
            stroke="currentColor"
            strokeWidth="2.2"
            strokeLinecap="round"
            strokeLinejoin="round"
          />
        </svg>
      )}
      {status === 'recovery' && (
        <svg viewBox="0 0 12 12" width="10" height="10" fill="none">
          <path
            d="M6 2v4.6M6 9.6v.1"
            stroke="currentColor"
            strokeWidth="2.2"
            strokeLinecap="round"
          />
        </svg>
      )}
    </span>
  )
}
