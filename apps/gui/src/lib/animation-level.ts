/** How much a status effect may move: Waves loop, Sweep crosses once, none draws nothing. */
export type AnimationLevel = 'waves' | 'sweep' | 'none'

/** A status effect the user can pick. */
export type EffectStyle = 'waves' | 'sweep'

/** Richest first: stepping down means moving right. */
const LEVELS: readonly AnimationLevel[] = ['waves', 'sweep', 'none']

export const LEVEL_NAME: Record<AnimationLevel, string> = { waves: 'Waves', sweep: 'Sweep', none: 'no effects' }

/** One level cheaper, and no cheaper than none. */
export function stepDown(level: AnimationLevel): AnimationLevel {
  const next = LEVELS[LEVELS.indexOf(level) + 1]
  return next ?? 'none'
}

/** The richest level a web view can afford before anything is measured: a software rasteriser repaints a looping layer every frame. */
export function levelForGpu(gpu: 'webgl' | 'software'): AnimationLevel {
  return gpu === 'webgl' ? 'waves' : 'sweep'
}

/** The style a status gets under a cap: its own choice, the cap's cheaper style, or none. */
export function styleUnderCap(style: EffectStyle, cap: AnimationLevel): EffectStyle | null {
  if (cap === 'none') return null
  if (cap === 'sweep') return 'sweep'
  return style
}
