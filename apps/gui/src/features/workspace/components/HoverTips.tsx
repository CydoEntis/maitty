import { useEffect, useState } from 'react'
import { createPortal } from 'react-dom'

const SHOW_DELAY_MS = 350
const GAP_PX = 8
const EDGE_PX = 8
/** Half the tooltip's widest size, so a tip centred near a window edge stays inside it. */
const HALF_WIDTH_PX = 120
const FLIP_BELOW_PX = 44
const CENTRE_RATIO = 0.5

type Tip = { text: string; rect: DOMRect }

/**
 * A themed tooltip for anything with a `title`, in place of the browser's slow, unstyled one.
 * Hovering takes the `title` off the element and puts it back on leaving, so the browser's own
 * tooltip never doubles up with this one.
 */
export function HoverTips() {
  const [tip, setTip] = useState<Tip | null>(null)

  useEffect(() => {
    let current: HTMLElement | null = null
    let timer = 0

    const release = () => {
      window.clearTimeout(timer)
      if (current?.dataset.tip !== undefined && !current.hasAttribute('title')) current.setAttribute('title', current.dataset.tip)
      current = null
      setTip(null)
    }
    const over = (event: PointerEvent) => {
      const target = event.target instanceof Element ? event.target.closest<HTMLElement>('[title], [data-tip]') : null
      if (!target || target === current) return
      release()
      const text = target.getAttribute('title')
      if (!text) return
      target.dataset.tip = text
      target.removeAttribute('title')
      current = target
      timer = window.setTimeout(() => { setTip({ text, rect: target.getBoundingClientRect() }) }, SHOW_DELAY_MS)
    }
    const out = (event: PointerEvent) => {
      if (current && !(event.relatedTarget instanceof Node && current.contains(event.relatedTarget))) release()
    }

    document.addEventListener('pointerover', over)
    document.addEventListener('pointerout', out)
    document.addEventListener('pointerdown', release)
    document.addEventListener('keydown', release)
    window.addEventListener('blur', release)
    return () => {
      release()
      document.removeEventListener('pointerover', over)
      document.removeEventListener('pointerout', out)
      document.removeEventListener('pointerdown', release)
      document.removeEventListener('keydown', release)
      window.removeEventListener('blur', release)
    }
  }, [])

  if (!tip) return null
  const centre = Math.min(Math.max(tip.rect.left + tip.rect.width * CENTRE_RATIO, EDGE_PX + HALF_WIDTH_PX), window.innerWidth - EDGE_PX - HALF_WIDTH_PX)
  const below = tip.rect.bottom + FLIP_BELOW_PX < window.innerHeight
  const place = below ? { top: tip.rect.bottom + GAP_PX } : { bottom: window.innerHeight - tip.rect.top + GAP_PX }
  return createPortal(<div role="tooltip" className="mx-tip" style={{ left: centre, ...place }}>{tip.text}</div>, document.body)
}
