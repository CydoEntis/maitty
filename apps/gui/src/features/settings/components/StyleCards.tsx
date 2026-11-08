import type { CSSProperties } from 'react'
import { activeThemeName, findTheme, themeStyle, type ThemeMode } from '@/features/workspace/theme/themes'
import { WINDOW_STYLES, withWindowStyle, type WindowStyle, type WindowStyleEntry } from '@/features/workspace/theme/windowStyles'

/** One line each, under the style's name. */
const STYLE_META: Readonly<Record<WindowStyle, string>> = {
  glass: 'Soft frosted glass',
  mono: 'One width, greyscale',
  console: 'IDE panels, true black',
  pixel: '8-bit, hard edges',
}

type Props = {
  value: WindowStyle
  choice: { themeMode: ThemeMode; darkTheme: string; lightTheme: string }
  systemDark: boolean
  onPick: (style: WindowStyle) => void
}

/**
 * The window style picker: a card per style with a live mini preview. A preview is drawn from the
 * style's own tokens (set on the preview element), so it shows that style in the theme in use without
 * a colour of its own.
 */
export function StyleCards({ value, choice, systemDark, onPick }: Props) {
  const base = findTheme(activeThemeName(choice, systemDark))
  return (
    <div role="radiogroup" aria-label="Window style" className="mx-style-cards">
      {WINDOW_STYLES.map((style) => (
        <StyleCard key={style.id} style={style} theme={base} selected={style.id === value} onPick={onPick} />
      ))}
    </div>
  )
}

function StyleCard({ style, theme, selected, onPick }: { style: WindowStyleEntry; theme: ReturnType<typeof findTheme>; selected: boolean; onPick: (style: WindowStyle) => void }) {
  const tokens = themeStyle(withWindowStyle(theme, style.id)) as CSSProperties
  return (
    <button type="button" role="radio" aria-checked={selected} className={`mx-style-card${selected ? ' is-selected' : ''}`} onClick={() => { onPick(style.id) }}>
      <span className="mx-sp" data-sp={style.id} style={tokens} aria-hidden="true">
        <i className="mx-sp-side">
          <b className="mx-sp-row is-current"><b className="mx-sp-dot is-accent" /><b className="mx-sp-bar is-fg2" /></b>
          <b className="mx-sp-row"><b className="mx-sp-dot is-warn" /><b className="mx-sp-bar" /></b>
          <b className="mx-sp-row"><b className="mx-sp-dot is-dim" /><b className="mx-sp-bar" /></b>
        </i>
        <i className="mx-sp-pane">
          <b className="mx-sp-aa">Aa</b>
          <b className="mx-sp-line is-long" />
          <b className="mx-sp-line is-short" />
          <span className="mx-sp-fill" />
          <b className="mx-sp-box"><b className="mx-sp-send" /></b>
        </i>
      </span>
      <span className="mx-style-foot">
        <span className="mx-style-text"><span className="mx-style-name">{style.name}</span><span className="mx-style-blurb">{STYLE_META[style.id]}</span></span>
        <span className="mx-style-check" aria-hidden="true">
          {selected && <svg width="10" height="10" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="3.4" strokeLinecap="round" strokeLinejoin="round"><path d="m5 12 5 5 9-10" /></svg>}
        </span>
      </span>
    </button>
  )
}
