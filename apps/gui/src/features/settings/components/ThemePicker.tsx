import { useEffect, useMemo, useRef, useState, type CSSProperties } from 'react'
import { createPortal } from 'react-dom'
import { findTheme, THEMES, type ThemeEntry } from '@/features/workspace/theme/themes'
import { Icon } from '@/features/workspace/components/glyphs'

/** Room the menu needs below its button; with less, it opens upward so it is never clipped. */
const MENU_ROOM_PX = 380
const MENU_WIDTH_PX = 300
const MENU_GAP_PX = 6
/** The scroll this opening causes must not close the menu it is making room for. */
const OPENING_SCROLL_MS = 250
const EDGE_PX = 8
/** The smallest the list may be squeezed to when the window is short. */
const MENU_MIN_HEIGHT_PX = 160

type Props = {
  kind: 'dark' | 'light'
  value: string
  /** This slot's theme is the one showing: hovering a row previews it on the whole window. */
  inUse: boolean
  /** The other slot while a fixed mode is chosen. */
  dimmed: boolean
  onPick: (name: string) => void
  onPreview: (name: string | null) => void
}

function ThemeSwatch({ theme }: { theme: ThemeEntry }) {
  return (
    <span className="mx-swatch" style={{ background: theme.tokens.term }} aria-hidden="true">
      <span><i style={{ width: 12, background: theme.tokens.accent }} /><i style={{ width: 16, background: theme.tokens.fg, opacity: 0.7 }} /></span>
      <span><i style={{ width: 8, background: theme.tokens.ansiMag }} /><i style={{ width: 10, background: theme.tokens.ansiBlue }} /><i style={{ width: 6, background: theme.tokens.ansiGreen }} /></span>
    </span>
  )
}

/** A dropdown of the dark (or light) themes, with search; the theme in use is listed first. */
export function ThemePicker({ kind, value, inUse, dimmed, onPick, onPreview }: Props) {
  const [open, setOpen] = useState(false)
  const [query, setQuery] = useState('')
  const [place, setPlace] = useState<CSSProperties | null>(null)
  /** Where the menu mounts: the app root, so it reads the theme's colours and is not clipped by a card. */
  const [host, setHost] = useState<HTMLElement | null>(null)
  const root = useRef<HTMLDivElement>(null)
  const menu = useRef<HTMLDivElement>(null)
  const search = useRef<HTMLInputElement>(null)
  const openedAt = useRef(0)
  const light = kind === 'light'
  const noun = light ? 'light' : 'dark'
  const pool = useMemo(() => THEMES.filter((theme) => theme.light === light), [light])
  const shown = useMemo(() => {
    const needle = query.trim().toLowerCase()
    return pool
      .filter((theme) => !needle || `${theme.name} ${theme.family}`.toLowerCase().includes(needle))
      .sort((a, b) => Number(b.name === value) - Number(a.name === value))
  }, [pool, query, value])
  const current = findTheme(value)

  const close = () => {
    setOpen(false)
    setQuery('')
    onPreview(null)
  }

  useEffect(() => {
    if (!open) return
    const dismiss = () => {
      setOpen(false)
      setQuery('')
      onPreview(null)
    }
    const onPointerDown = (event: PointerEvent) => {
      const inside = (node: HTMLElement | null) => node !== null && event.target instanceof Node && node.contains(event.target)
      if (!inside(root.current) && !inside(menu.current)) dismiss()
    }
    // The menu is placed from the button once, so anything that moves the button closes it.
    const onScroll = (event: Event) => {
      if (Date.now() - openedAt.current < OPENING_SCROLL_MS) return
      if (!(event.target instanceof Node && menu.current?.contains(event.target))) dismiss()
    }
    document.addEventListener('pointerdown', onPointerDown)
    document.addEventListener('scroll', onScroll, true)
    window.addEventListener('resize', dismiss)
    return () => {
      document.removeEventListener('pointerdown', onPointerDown)
      document.removeEventListener('scroll', onScroll, true)
      window.removeEventListener('resize', dismiss)
    }
  }, [open, onPreview])

  // Focused without scrolling: scrolling the dialog would carry the button away from the menu placed beside it.
  useEffect(() => { if (open) search.current?.focus({ preventScroll: true }) }, [open])

  // A preview never outlives the picker.
  useEffect(() => () => { onPreview(null) }, [onPreview])

  const toggle = () => {
    if (open) {
      close()
      return
    }
    setHost(root.current?.closest<HTMLElement>('.mx-app') ?? document.body)
    openedAt.current = Date.now()
    // Near the bottom, bring the button up so the whole list fits below it, rather than squeezing the list.
    if (root.current && 'scrollIntoView' in root.current && window.innerHeight - root.current.getBoundingClientRect().bottom < MENU_ROOM_PX) {
      root.current.scrollIntoView({ block: 'start', behavior: 'instant' })
    }
    const rect = root.current?.getBoundingClientRect()
    if (rect) {
      const upward = window.innerHeight - rect.bottom < MENU_ROOM_PX && rect.top > window.innerHeight - rect.bottom
      const room = (upward ? rect.top : window.innerHeight - rect.bottom) - MENU_GAP_PX - EDGE_PX
      setPlace({
        left: Math.max(EDGE_PX, Math.min(rect.right - MENU_WIDTH_PX, window.innerWidth - MENU_WIDTH_PX - EDGE_PX)),
        ...(upward ? { bottom: window.innerHeight - rect.top + MENU_GAP_PX } : { top: rect.bottom + MENU_GAP_PX }),
        maxHeight: Math.max(room, MENU_MIN_HEIGHT_PX),
      })
    }
    setOpen(true)
  }

  return (
    <div ref={root} className={`mx-picker${dimmed ? ' is-dimmed' : ''}`}>
      <button type="button" className={`mx-picker-button${open ? ' is-open' : ''}`} aria-haspopup="listbox" aria-expanded={open} aria-label={`${light ? 'Light' : 'Dark'} theme: ${current.name}`} onClick={toggle}>
        <ThemeSwatch theme={current} />
        <span className="mx-picker-text">
          <span className="mx-theme-name">{current.name}</span>
          <span className="mx-theme-sub">{current.family}</span>
        </span>
        {inUse && <span className="mx-picker-pill">in use</span>}
        <svg className="mx-picker-chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.25" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
      </button>
      {open && place && host && createPortal(
        <div ref={menu} className="mx-picker-menu" style={place}>
          <label className="mx-search-field">
            <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
            <input
              ref={search}
              value={query}
              placeholder={`Search ${noun} themes`}
              aria-label={`Search ${noun} themes`}
              onChange={(event) => { setQuery(event.target.value) }}
              onKeyDown={(event) => {
                if (event.key !== 'Escape') return
                event.preventDefault()
                event.stopPropagation()
                close()
              }}
            />
          </label>
          <div role="listbox" aria-label={`${light ? 'Light' : 'Dark'} themes`} className="mx-picker-list" onMouseLeave={() => { onPreview(null) }}>
            {shown.map((theme) => (
              <button
                key={theme.name}
                type="button"
                role="option"
                aria-selected={theme.name === value}
                className="mx-theme-card"
                onMouseEnter={() => { if (inUse) onPreview(theme.name) }}
                onClick={() => { onPick(theme.name); close() }}
              >
                <ThemeSwatch theme={theme} />
                <span className="mx-picker-text">
                  <span className="mx-theme-name">{theme.name}</span>
                  <span className="mx-theme-sub">{theme.family}</span>
                </span>
                {theme.name === value && <Icon name="check" size={14} stroke="var(--accent)" strokeWidth={2.5} />}
              </button>
            ))}
            {shown.length === 0 && <span className="mx-picker-empty">No themes match.</span>}
          </div>
          <div className="mx-picker-foot">{pool.length} {noun} themes</div>
        </div>,
        host,
      )}
    </div>
  )
}
