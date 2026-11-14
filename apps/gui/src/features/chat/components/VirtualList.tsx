import { flushSync } from 'react-dom'
import { useCallback, useContext, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode, type WheelEvent } from 'react'
import { ArrowDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { ReducedMotionContext } from '@/features/workspace/components/motion'
import { buildLayout, followAfterScroll, glidePlan, glideTop, isAtBottom, visibleRange, type FollowState } from './windowing'

type Props<T> = {
  items: readonly T[]
  /** A key that stays with the item for its whole life, never its index. */
  getKey: (item: T) => string
  /** Height used for a row until it has been measured. */
  estimate: number
  renderItem: (item: T) => ReactNode
  label: string
  /** Rows drawn beyond the view, each way. */
  overscan?: number
  /** Scrolls the row with this key to the top of the view; a new `nonce` scrolls again to the same row. */
  scrollTarget?: { key: string; nonce: number } | null
}

/** Used until the list has been laid out, and where there is no layout (tests). */
const FALLBACK_VIEWPORT_PX = 600
const DEFAULT_OVERSCAN = 6
/**
 * A scroll is the reader's only for this long after they last touched the list (wheel, pointer, key
 * or finger). A web view also sends scroll events nobody asked for, as it settles a view the list has
 * just moved or resized; read as the reader moving up, one of those stopped the list following.
 */
const READER_SCROLL_MS = 1200

/**
 * A scrolling list that draws only the rows near the view, so a conversation of thousands of items
 * costs what a screenful costs. Rows are placed with `transform` (no layout work per scroll), their
 * heights are measured as they are drawn, and the list follows its newest row until the reader
 * moves up; then it offers a way back to it.
 */
export function VirtualList<T>({ items, getKey, estimate, renderItem, label, overscan = DEFAULT_OVERSCAN, scrollTarget = null }: Props<T>) {
  const scroller = useRef<HTMLDivElement>(null)
  // Whether the list follows its newest row, and where it was last seen (`followAfterScroll`).
  const follow = useRef<FollowState>({ following: true, top: 0, total: 0 })
  // Where the list itself last put the view, so the scroll event that write causes is not read as the reader's.
  const wrote = useRef<number | null>(null)
  // When the reader last touched the list, and whether they still hold it (a scrollbar being dragged).
  const touchedAt = useRef(Number.NEGATIVE_INFINITY)
  const held = useRef(false)
  const touched = useCallback(() => { touchedAt.current = performance.now() }, [])
  useEffect(() => {
    const release = () => {
      if (!held.current) return
      held.current = false
      touched()
    }
    window.addEventListener('pointerup', release)
    window.addEventListener('pointercancel', release)
    return () => {
      window.removeEventListener('pointerup', release)
      window.removeEventListener('pointercancel', release)
    }
  }, [touched])
  const [heights, setHeights] = useState<ReadonlyMap<string, number>>(new Map())
  const [scrollTop, setScrollTop] = useState(0)
  const [viewport, setViewport] = useState(FALLBACK_VIEWPORT_PX)
  const [following, setFollowing] = useState(true)

  const keys = useMemo(() => items.map(getKey), [items, getKey])
  const layout = useMemo(() => buildLayout(keys, heights, estimate), [keys, heights, estimate])
  const range = visibleRange(layout, scrollTop, viewport, overscan)

  // Measures are applied before the browser paints, so a row that has just come in is never drawn at its estimate for a
  // frame and then moved: the ref callbacks note heights while React commits and the layout effect below applies them;
  // the observer's reports (a row that grew) are applied at once, outside React's commit.
  const pending = useRef(new Map<string, number>())
  const apply = useCallback(() => {
    const batch = pending.current
    if (batch.size === 0) return
    pending.current = new Map()
    setHeights((current) => {
      let changed = false
      const next = new Map(current)
      for (const [key, height] of batch) {
        if (next.get(key) === height) continue
        next.set(key, height)
        changed = true
      }
      return changed ? next : current
    })
  }, [])
  useLayoutEffect(() => { apply() })

  const record = useCallback((element: HTMLElement) => {
    const key = element.dataset.key
    const height = Math.round(element.offsetHeight)
    // A row with no height has not been laid out (or there is no layout engine): its estimate stands.
    if (key === undefined || height === 0) return
    pending.current.set(key, height)
  }, [])

  // One observer for every drawn row; a row reports its own height when it changes (an expanded card, a streamed message).
  const observer = useRef<ResizeObserver | null>(null)
  const measure = useCallback((element: HTMLDivElement | null) => {
    if (element === null) return
    if (observer.current === null && typeof ResizeObserver !== 'undefined') {
      observer.current = new ResizeObserver((entries) => {
        for (const entry of entries) record(entry.target as HTMLElement)
        flushSync(apply)
      })
    }
    observer.current?.observe(element)
    record(element)
  }, [record, apply])
  useEffect(() => () => {
    observer.current?.disconnect()
    observer.current = null
  }, [])

  useLayoutEffect(() => {
    const element = scroller.current
    if (!element) return
    const update = () => { setViewport(element.clientHeight || FALLBACK_VIEWPORT_PX) }
    update()
    if (typeof ResizeObserver === 'undefined') return
    const watch = new ResizeObserver(update)
    watch.observe(element)
    return () => { watch.disconnect() }
  }, [])

  // The list moves the view itself: remember what it wrote and where the view is.
  const place = useCallback((element: HTMLDivElement, top: number, total: number) => {
    element.scrollTop = top
    wrote.current = element.scrollTop
    follow.current = { ...follow.current, top: element.scrollTop, total }
    setScrollTop(element.scrollTop)
  }, [])

  // Keep the newest row in view while the list follows it.
  useLayoutEffect(() => {
    const element = scroller.current
    if (!element) return
    if (!follow.current.following) {
      follow.current = { ...follow.current, total: layout.total }
      return
    }
    place(element, layout.total, layout.total)
    // The view's own height is a dependency too: a message box that grows must not leave the newest row under it.
  }, [layout.total, viewport, place])

  // A link to a row (an approval's step) moves the view there and stops following the newest row.
  const handled = useRef<number | null>(null)
  useLayoutEffect(() => {
    const element = scroller.current
    if (!element || scrollTarget === null || handled.current === scrollTarget.nonce) return
    const at = keys.indexOf(scrollTarget.key)
    if (at === -1) return
    handled.current = scrollTarget.nonce
    follow.current = { ...follow.current, following: false }
    place(element, layout.offsets[at] ?? 0, layout.total)
    setFollowing(false)
  }, [scrollTarget, keys, layout, place])

  const onScroll = () => {
    const element = scroller.current
    if (!element) return
    setScrollTop(element.scrollTop)
    const ownWrite = wrote.current === element.scrollTop
    const byReader = held.current || performance.now() - touchedAt.current < READER_SCROLL_MS
    wrote.current = null
    const height = element.clientHeight || FALLBACK_VIEWPORT_PX
    follow.current = followAfterScroll(follow.current, { scrollTop: element.scrollTop, viewport: height, total: layout.total }, ownWrite || !byReader)
    setFollowing(follow.current.following)
    // A scroll nobody asked for that left a following list short of its newest row is put right.
    if (!ownWrite && !byReader && follow.current.following && !isAtBottom(element.scrollTop, height, layout.total)) place(element, layout.total, layout.total)
  }

  // A wheel move up arrives before the scroll it causes, and a streaming commit in between would undo it.
  const onWheel = (event: WheelEvent) => {
    touched()
    stopGlide()
    // The scrolling region is only as wide as the chat column; a wheel turn over the space beside it scrolls it too.
    const element = scroller.current
    if (element !== null && event.target instanceof Node && !element.contains(event.target)) element.scrollTop += event.deltaY
    // Nothing above to scroll to (a short chat, or the top already) is not leaving the newest row.
    if (event.deltaY >= 0 || !follow.current.following || (scroller.current?.scrollTop ?? 0) <= 0) return
    follow.current = { ...follow.current, following: false }
    setFollowing(false)
  }

  // The jump to the newest row glides there, so the reader sees which way the chat went; the reader's own scroll, a
  // new jump or the list going away ends it, and with motion reduced it is a plain jump.
  const reducedMotion = useContext(ReducedMotionContext)
  const glide = useRef<number | null>(null)
  const totalNow = useRef(layout.total)
  useLayoutEffect(() => { totalNow.current = layout.total }, [layout.total])
  const stopGlide = useCallback(() => {
    if (glide.current !== null) cancelAnimationFrame(glide.current)
    glide.current = null
  }, [])
  useEffect(() => stopGlide, [stopGlide])
  const jumpToLatest = () => {
    const element = scroller.current
    if (!element) return
    stopGlide()
    const arrive = () => {
      glide.current = null
      follow.current = { ...follow.current, following: true }
      place(element, totalNow.current, totalNow.current)
      setFollowing(true)
    }
    const bottomNow = () => element.scrollHeight - element.clientHeight
    const plan = reducedMotion ? null : glidePlan(element.scrollTop, bottomNow(), element.clientHeight)
    if (plan === null) {
      arrive()
      return
    }
    const began = performance.now()
    const step = (now: number) => {
      const share = (now - began) / plan.ms
      if (share >= 1) {
        arrive()
        return
      }
      // The bottom is read each frame: a reply that is still arriving moves it.
      place(element, glideTop(plan.from, bottomNow(), share), totalNow.current)
      glide.current = requestAnimationFrame(step)
    }
    place(element, plan.from, totalNow.current)
    glide.current = requestAnimationFrame(step)
  }

  const drawn = items.slice(range.start, range.end)
  return (
    <div className="relative flex min-h-0 flex-1 flex-col" onWheel={onWheel}>
      <div ref={scroller} role="region" aria-label={label} tabIndex={0} className="mx-chat-scroller min-h-0 flex-1 overflow-y-auto overscroll-contain" onScroll={onScroll} onPointerDown={() => { held.current = true; touched(); stopGlide() }} onKeyDown={() => { touched(); stopGlide() }} onTouchMove={() => { touched(); stopGlide() }}>
        <div className="relative" style={{ height: layout.total }}>
          {drawn.map((item, index) => {
            const at = range.start + index
            const key = keys[at] ?? ''
            return (
              <div key={key} ref={measure} data-key={key} className="absolute inset-x-0 top-0" style={{ transform: `translateY(${String(layout.offsets[at] ?? 0)}px)` }}>
                {renderItem(item)}
              </div>
            )
          })}
        </div>
      </div>
      {!following && layout.total > viewport && (
        <Button type="button" variant="soft" size="icon" className="mx-chat-jump size-8 rounded-full bg-raise" aria-label="Jump to the latest message" onClick={jumpToLatest}>
          <ArrowDown className="size-4" aria-hidden="true" />
        </Button>
      )}
    </div>
  )
}
