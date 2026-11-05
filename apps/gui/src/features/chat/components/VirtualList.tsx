import { useCallback, useEffect, useLayoutEffect, useMemo, useRef, useState, type ReactNode } from 'react'
import { ArrowDown } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { buildLayout, isAtBottom, visibleRange } from './windowing'

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
 * scrolls away; then it offers a way back to it.
 */
export function VirtualList<T>({ items, getKey, estimate, renderItem, label, overscan = DEFAULT_OVERSCAN, scrollTarget = null }: Props<T>) {
  const scroller = useRef<HTMLDivElement>(null)
  const following = useRef(true)
  const [heights, setHeights] = useState<ReadonlyMap<string, number>>(new Map())
  const [scrollTop, setScrollTop] = useState(0)
  const [viewport, setViewport] = useState(FALLBACK_VIEWPORT_PX)
  const [atBottom, setAtBottom] = useState(true)

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

  // Keep the newest row in view while the reader is at the bottom.
  useLayoutEffect(() => {
    const element = scroller.current
    if (!element || !following.current) return
    element.scrollTop = layout.total
    setScrollTop(element.scrollTop)
  }, [layout.total])

  // A link to a row (an approval's step) moves the view there and stops following the newest row.
  const handled = useRef<number | null>(null)
  useLayoutEffect(() => {
    const element = scroller.current
    if (!element || scrollTarget === null || handled.current === scrollTarget.nonce) return
    const at = keys.indexOf(scrollTarget.key)
    if (at === -1) return
    handled.current = scrollTarget.nonce
    following.current = false
    element.scrollTop = layout.offsets[at] ?? 0
    setScrollTop(element.scrollTop)
    setAtBottom(false)
  }, [scrollTarget, keys, layout])

  const onScroll = () => {
    const element = scroller.current
    if (!element) return
    setScrollTop(element.scrollTop)
    const bottom = isAtBottom(element.scrollTop, element.clientHeight || FALLBACK_VIEWPORT_PX, layout.total)
    following.current = bottom
    setAtBottom(bottom)
  }

  const jumpToLatest = () => {
    const element = scroller.current
    if (!element) return
    following.current = true
    element.scrollTop = layout.total
    setScrollTop(element.scrollTop)
    setAtBottom(true)
  }

  const drawn = items.slice(range.start, range.end)
  return (
    <div className="relative flex min-h-0 flex-1 flex-col">
      <div ref={scroller} role="region" aria-label={label} tabIndex={0} className="min-h-0 flex-1 overflow-y-auto overscroll-contain" onScroll={onScroll}>
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
      {!atBottom && (
        <Button type="button" variant="outline" size="icon" className="absolute right-4 bottom-3 size-8 rounded-full bg-raise" aria-label="Jump to the latest message" onClick={jumpToLatest}>
          <ArrowDown className="size-4" aria-hidden="true" />
        </Button>
      )}
    </div>
  )
}
