import { useEffect, useState, type CSSProperties } from 'react'
import { Tooltip, TooltipContent, TooltipProvider, TooltipTrigger } from '@/components/ui/tooltip'
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { parseTip } from './tipText'

const SHOW_DELAY_MS = 420
/** After a tip was on screen, the next one comes almost at once, so moving along a row of buttons does not stall. */
const FOLLOW_ON_DELAY_MS = 40
const FOLLOW_ON_WINDOW_MS = 400
const GAP_PX = 8
const EDGE_PX = 8
/** A control this close to the top gets its tip below it, where there is room. */
const FLIP_BELOW_PX = 56
const CENTRE_RATIO = 0.5
/** What a tip never covers: a menu, or a rich hover card, which already says more than the tip would. */
const RICH_OVERLAY = '[role="menu"], .mx-ctx'
/** Controls that carry only an icon name their action by `aria-label` alone. */
const ICON_CONTROLS = 'button, [role="button"], [role="radio"], [role="switch"]'
/** Marks a control, such as an activity-bar item, whose tip goes beside it, away from the window edge. */
const SIDE_ATTRIBUTE = 'data-tip-side'
/** Marks a control that opens its own rich hover card, and so gets no tip. */
const HOVER_CARD_ATTRIBUTE = 'data-hovercard'

type Tip = { text: string; rect: DOMRect; side: boolean }

/** The control a tip belongs to and its words: a `title` (or the `data-tip` it was moved to), else an icon-only control's `aria-label`. */
function sourceOf(start: Element): { element: HTMLElement; text: string } | null {
  for (let element: Element | null = start; element; element = element.parentElement) {
    if (!(element instanceof HTMLElement)) continue
    // A rail row opens a hover card of its own; a tip beside it would say less and cover it.
    if (element.hasAttribute(HOVER_CARD_ATTRIBUTE)) return null
    const text = element.getAttribute('title') ?? element.dataset.tip
    if (text) return { element, text }
    const label = element.getAttribute('aria-label')
    if (label && element.matches(ICON_CONTROLS) && element.textContent.trim() === '') return { element, text: label }
  }
  return null
}

/**
 * A themed tooltip for anything with a `title`, and for icon-only controls with an `aria-label`
 * (v18 B.4), in place of the browser's slow, unstyled one. Hovering takes the `title` off the
 * element and puts it back on leaving, so the browser's own tooltip never doubles up with this one.
 */
export function HoverTips() {
  const [tip, setTip] = useState<Tip | null>(null)

  useEffect(() => {
    let current: HTMLElement | null = null
    let timer = 0
    let shown = false
    let lastVisibleAt = Number.NEGATIVE_INFINITY

    const release = () => {
      window.clearTimeout(timer)
      if (shown) lastVisibleAt = Date.now()
      shown = false
      if (current?.dataset.tip !== undefined && !current.hasAttribute('title')) current.setAttribute('title', current.dataset.tip)
      current = null
      setTip(null)
    }
    const dismiss = () => {
      window.clearTimeout(timer)
      if (shown) lastVisibleAt = Date.now()
      shown = false
      setTip(null)
    }
    const over = (event: PointerEvent) => {
      const source = event.target instanceof Element ? sourceOf(event.target) : null
      if (!source || source.element === current) return
      release()
      const { element, text } = source
      // The native tooltip would otherwise show even when ours is held back.
      if (element.hasAttribute('title')) {
        element.dataset.tip = text
        element.removeAttribute('title')
      }
      current = element
      const delay = Date.now() - lastVisibleAt < FOLLOW_ON_WINDOW_MS ? FOLLOW_ON_DELAY_MS : SHOW_DELAY_MS
      timer = window.setTimeout(() => {
        if (document.querySelector(RICH_OVERLAY)) return
        shown = true
        setTip({ text, rect: element.getBoundingClientRect(), side: element.hasAttribute(SIDE_ATTRIBUTE) })
      }, delay)
    }
    const out = (event: PointerEvent) => {
      if (current && !(event.relatedTarget instanceof Node && current.contains(event.relatedTarget))) release()
    }

    document.addEventListener('pointerover', over)
    document.addEventListener('pointerout', out)
    document.addEventListener('pointerdown', release)
    document.addEventListener('keydown', release)
    document.addEventListener('wheel', dismiss, { passive: true })
    window.addEventListener('blur', release)
    return () => {
      release()
      document.removeEventListener('pointerover', over)
      document.removeEventListener('pointerout', out)
      document.removeEventListener('pointerdown', release)
      document.removeEventListener('keydown', release)
      document.removeEventListener('wheel', dismiss)
      window.removeEventListener('blur', release)
    }
  }, [])

  const content = tip ? parseTip(tip.text) : null
  return (
    <TooltipProvider>
      <Tooltip open={tip !== null}>
        {/* The tip belongs to whatever the pointer is over, so its anchor is an empty box laid over that control. */}
        <TooltipTrigger asChild>
          <span aria-hidden="true" style={{ position: 'fixed', pointerEvents: 'none', left: tip?.rect.left ?? 0, top: tip?.rect.top ?? 0, width: tip?.rect.width ?? 0, height: tip?.rect.height ?? 0 }} />
        </TooltipTrigger>
        {tip && content && (
          <TooltipContent arrow={false} className="mx-tip" side={sideOf(tip)} sideOffset={GAP_PX} collisionPadding={EDGE_PX} style={{ '--tip-dx': '0px' } as CSSProperties}>
            <span className="mx-tip-line">
              <span className="mx-tip-name">{content.name}</span>
              {content.keys && <Keycaps chord={content.keys} className="mx-tip-keys" />}
            </span>
            {content.hint && <span className="mx-tip-hint">{content.hint}</span>}
          </TooltipContent>
        )}
      </Tooltip>
    </TooltipProvider>
  )
}

/** Above by default, below near the top of the window, beside the control for a side tip (away from the window edge). */
function sideOf({ rect, side }: Pick<Tip, 'rect' | 'side'>): 'top' | 'bottom' | 'left' | 'right' {
  if (side) return rect.left + rect.width * CENTRE_RATIO < window.innerWidth * CENTRE_RATIO ? 'right' : 'left'
  return rect.top < FLIP_BELOW_PX ? 'bottom' : 'top'
}
