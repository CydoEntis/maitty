import { useEffect, useMemo, useState } from 'react'
import { Popover } from 'radix-ui'
import { PopupContent } from '@/components/ui/popup'
import { ChoiceItem, ChoiceList } from '@/components/ui/choice-list'
import { findTheme, THEMES, type ThemeEntry } from '@/features/workspace/theme/themes'
import { Icon } from '@/features/workspace/components/glyphs'

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

  const change = (next: boolean) => {
    setOpen(next)
    if (!next) {
      setQuery('')
      onPreview(null)
    }
  }

  // A preview never outlives the picker.
  useEffect(() => () => { onPreview(null) }, [onPreview])

  return (
    <Popover.Root open={open} onOpenChange={change}>
      <div className={`mx-picker${dimmed ? ' is-dimmed' : ''}`}>
        <Popover.Trigger asChild>
          <button type="button" className={`mx-picker-button${open ? ' is-open' : ''}`} aria-label={`${light ? 'Light' : 'Dark'} theme: ${current.name}`}>
            <ThemeSwatch theme={current} />
            <span className="mx-picker-text">
              <span className="mx-theme-name">{current.name}</span>
              <span className="mx-theme-sub">{current.family}</span>
            </span>
            {inUse && <span className="mx-picker-pill">in use</span>}
            <svg className="mx-picker-chevron" width="12" height="12" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.25" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="m6 9 6 6 6-6" /></svg>
          </button>
        </Popover.Trigger>
      </div>
      <PopupContent align="end" collisionPadding={8} className="mx-picker-menu" aria-label={`${light ? 'Light' : 'Dark'} theme picker`} onEscapeKeyDown={(event) => { event.stopPropagation() }}>
        <label className="mx-search-field">
          <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" aria-hidden="true"><circle cx="11" cy="11" r="7" /><path d="m20 20-3.5-3.5" /></svg>
          <input
            autoFocus
            value={query}
            placeholder={`Search ${noun} themes`}
            aria-label={`Search ${noun} themes`}
            onChange={(event) => { setQuery(event.target.value) }}
            onKeyDown={(event) => { if (event.key === 'ArrowDown') { event.preventDefault(); event.currentTarget.closest('.mx-picker-menu')?.querySelector<HTMLElement>('[data-slot="choice-item"]')?.focus() } }}
          />
        </label>
        <ChoiceList aria-label={`${light ? 'Light' : 'Dark'} themes`} current={value} className="mx-picker-list" onMouseLeave={() => { onPreview(null) }}>
          {shown.map((theme) => (
            <ChoiceItem
              key={theme.name}
              value={theme.name}
              className="mx-theme-card"
              onMouseEnter={() => { if (inUse) onPreview(theme.name) }}
              onFocus={() => { if (inUse) onPreview(theme.name) }}
              onPick={() => { onPick(theme.name); change(false) }}
            >
              <ThemeSwatch theme={theme} />
              <span className="mx-picker-text">
                <span className="mx-theme-name">{theme.name}</span>
                <span className="mx-theme-sub">{theme.family}</span>
              </span>
              {theme.name === value && <Icon name="check" size={14} stroke="var(--accent)" strokeWidth={2.5} />}
            </ChoiceItem>
          ))}
          {shown.length === 0 && <span className="mx-picker-empty">No themes match.</span>}
        </ChoiceList>
        <div className="mx-picker-foot">{pool.length} {noun} themes</div>
      </PopupContent>
    </Popover.Root>
  )
}
