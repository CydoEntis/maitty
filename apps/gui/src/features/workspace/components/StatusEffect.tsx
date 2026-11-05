import type { AppliedEffect } from './motion'

/** One wave band is a sine curve over two full wavelengths filled down to the bottom edge. */
const WAVE_WIDTH = 200
const WAVE_HEIGHT = 20
/** A quadratic segment is half a wavelength, and the viewBox holds two identical halves. */
const SEGMENTS_PER_WAVELENGTH = 2
const IDENTICAL_HALVES = 2
/** The quadratic control point sits midway along its segment. */
const MIDWAY = 2

type Band = { baseline: number; amplitude: number; wavelengthsPerHalf: number }

/**
 * A closed path for a sine band across `WAVE_WIDTH` units. The curve repeats every half of it, so a
 * layer that is twice as wide as its row can slide by exactly -50% and loop without a seam.
 */
function wavePath({ baseline, amplitude, wavelengthsPerHalf }: Band): string {
  const halfSteps = wavelengthsPerHalf * IDENTICAL_HALVES * SEGMENTS_PER_WAVELENGTH
  const step = WAVE_WIDTH / halfSteps
  let path = `M0 ${String(baseline)} Q${String(step / MIDWAY)} ${String(baseline - amplitude)} ${String(step)} ${String(baseline)}`
  for (let index = 2; index <= halfSteps; index += 1) path += ` T${String(step * index)} ${String(baseline)}`
  return `${path} V${String(WAVE_HEIGHT)} H0 Z`
}

// Two bands drift left at 18 s and a third drifts right at 28 s, so the layers slide through each other.
const NEAR_BANDS = [
  wavePath({ baseline: 11, amplitude: 4, wavelengthsPerHalf: 1 }),
  wavePath({ baseline: 14, amplitude: 3, wavelengthsPerHalf: 2 }),
]
const FAR_BANDS = [wavePath({ baseline: 12, amplitude: 5, wavelengthsPerHalf: 1 })]

function WaveLayer({ paths, reverse }: { paths: readonly string[]; reverse?: boolean }) {
  return (
    <span className={`mx-fx-wave${reverse ? ' is-reverse' : ''}`}>
      <svg viewBox={`0 0 ${String(WAVE_WIDTH)} ${String(WAVE_HEIGHT)}`} preserveAspectRatio="none" focusable="false">
        {paths.map((path) => <path key={path} d={path} />)}
      </svg>
    </span>
  )
}

/**
 * The status effect of a row or tab: an absolutely positioned child that is clipped to its host,
 * paints once and then only moves with `transform` (docs/DESIGN.md, Status effects).
 */
export function StatusEffect({ effect }: { effect: AppliedEffect }) {
  if (effect.style === null || effect.status === null) return null
  return (
    <span className="mx-fx" data-style={effect.style} data-status={effect.status} aria-hidden="true">
      {effect.style === 'waves'
        ? <><WaveLayer paths={NEAR_BANDS} /><WaveLayer paths={FAR_BANDS} reverse /></>
        : <span className="mx-fx-band" />}
    </span>
  )
}
