import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode, type WheelEvent } from 'react'
import { ArrowDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { buildLayout, followAfterScroll, visibleRange, type FollowState } from './windowing'

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
  const [heights, setHeights] = useState<ReadonlyMap<string, number>>(new Map())
  const [scrollTop, setScrollTop] = useState(0)
  const [viewport, setViewport] = useState(FALLBACK_VIEWPORT_PX)
  const [following, setFollowing] = useState(true)

  const keys = useMemo(() => items.map(getKey), [items, getKey])
  const layout = useMemo(() => buildLayout(keys, heights, estimate), [keys, heights, estimate])
  const range = visibleRange(layout, scrollTop, viewport, overscan)

  // Measures arrive from the observer in bursts; they are applied once a frame.
  const pending = useRef(new Map<string, number>())
  const frame = useRef(0)
  const flush = useCallback(() => {
    if (frame.current !== 0) return
    frame.current = window.requestAnimationFrame(() => {
      frame.current = 0
      const batch = pending.current
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
    })
  }, [])

  const record = useCallback((element: HTMLElement) => {
    const key = element.dataset.key
    const height = Math.round(element.offsetHeight)
    // A row with no height has not been laid out (or there is no layout engine): its estimate stands.
    if (key === undefined || height === 0) return
    pending.current.set(key, height)
    flush()
  }, [flush])

  // One observer for every drawn row; a row reports its own height when it changes (an expanded card, a streamed message).
  const observer = useRef<ResizeObserver | null>(null)
  const measure = useCallback((element: HTMLDivElement | null) => {
    if (element === null) return
    if (observer.current === null && typeof ResizeObserver !== 'undefined') {
      observer.current = new ResizeObserver((entries) => { for (const entry of entries) record(entry.target as HTMLElement) })
    }
    observer.current?.observe(element)
    record(element)
  }, [record])
  useEffect(() => () => {
    observer.current?.disconnect()
    observer.current = null
    window.cancelAnimationFrame(frame.current)
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
  }, [layout.total, place])

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
    const causedByList = wrote.current === element.scrollTop
    wrote.current = null
    follow.current = followAfterScroll(follow.current, { scrollTop: element.scrollTop, viewport: element.clientHeight || FALLBACK_VIEWPORT_PX, total: layout.total }, causedByList)
    setFollowing(follow.current.following)
  }

  // A wheel move up arrives before the scroll it causes, and a streaming commit in between would undo it.
  const onWheel = (event: WheelEvent) => {
    if (event.deltaY >= 0 || !follow.current.following) return
    follow.current = { ...follow.current, following: false }
    setFollowing(false)
  }

  const jumpToLatest = () => {
    const element = scroller.current
    if (!element) return
    follow.current = { ...follow.current, following: true }
    place(element, layout.total, layout.total)
    setFollowing(true)
  }

  const drawn = items.slice(range.start, range.end)
  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      <div ref={scroller} role="region" aria-label={label} tabIndex={0} className="min-h-0 flex-1 overflow-y-auto overscroll-contain" onScroll={onScroll} onWheel={onWheel}>
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
      {!following && (
        <Button type="button" variant="outline" size="icon" className="absolute right-4 bottom-3 size-8 rounded-full bg-raise" aria-label="Jump to the latest message" onClick={jumpToLatest}>
          <ArrowDown className="size-4" aria-hidden="true" />
        </Button>
      )}
    </div>
  )
}
