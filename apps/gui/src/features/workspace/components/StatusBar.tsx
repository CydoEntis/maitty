import { memo } from 'react'
import { DropdownMenu } from 'radix-ui'
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { type Chord } from '../model/shortcuts'
import { THEMES, type ThemeMode } from '../theme/themes'
import { WINDOW_STYLES, type WindowStyle } from '../theme/windowStyles'
import { Icon } from './glyphs'

const THEME_MODES: readonly { id: ThemeMode; label: string }[] = [
  { id: 'dark', label: 'Dark' },
  { id: 'light', label: 'Light' },
  { id: 'system', label: 'System' },
]
const COMMON_THEMES = 6

/** The machine the focused pane runs on: shown only while more than one machine is configured, so the common case stays quiet. */
export type FocusMachine = { name: string; word: string; tone: 'ok' | 'quiet' | 'warn' | 'bad' }

type Props = {
  online: boolean
  /** What to say instead of Reconnecting while the service is known to need something else (a restart after an update). */
  offlineText?: string | undefined
  machine: FocusMachine | null
  themeMode: ThemeMode
  /** Whether the theme in use is a light one: the quick list shows the themes of that kind. */
  lightInUse: boolean
  activeTheme: string
  style: WindowStyle
  textSize: number
  minTextSize: number
  maxTextSize: number
  shortcutChord: Chord
  portalContainer: HTMLElement | null
  onThemeMode: (mode: ThemeMode) => void
  onPickTheme: (name: string) => void
  onAllThemes: () => void
  onStyle: (style: WindowStyle) => void
  onTextSize: (size: number) => void
  onResetTextSize: () => void
  onOpenShortcuts: () => void
}

/** Quick controls at the bottom right; the left stays empty unless the daemon drops. Memoised, like the tab bar. */
export const StatusBar = memo(function StatusBar(props: Props) {
  const themes = THEMES.filter((theme) => theme.light === props.lightInUse).slice(0, COMMON_THEMES)
  const content = (className: string) => ({ className, sideOffset: 6, align: 'end' as const })
  return (
    <footer className="mx-statusbar" aria-label="Status">
      {!props.online && (
        <span className="mx-sb-item is-offline" role="status">
          <span className="mx-daemon-dot is-offline" aria-hidden="true" />
          {props.offlineText ?? 'Reconnecting to maittyd…'}
        </span>
      )}
      {props.machine !== null && (
        <span className="mx-sb-item mx-sb-machine" role="status" aria-label={`Focused pane runs on ${props.machine.name}, ${props.machine.word}`}>
          <span className={`mx-sb-machine-dot is-${props.machine.tone}`} aria-hidden="true" />
          <strong>{props.machine.name}</strong>
          <span className="mx-sb-machine-word">{props.machine.word}</span>
        </span>
      )}
      <span className="mx-spacer" />
      <DropdownMenu.Root>
        <DropdownMenu.Trigger className="mx-sb-item mx-sb-button" aria-label={`Theme: ${props.activeTheme}`}>
          <Icon name="palette" />{props.activeTheme}
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal container={props.portalContainer}>
          <DropdownMenu.Content {...content('mx-menu mx-sb-menu')}>
            <DropdownMenu.RadioGroup value={props.themeMode} onValueChange={(value) => { props.onThemeMode(value as ThemeMode) }}>
              <div className="mx-sb-modes" role="presentation">
                {THEME_MODES.map((mode) => (
                  <DropdownMenu.RadioItem key={mode.id} value={mode.id} className="mx-sb-mode" onSelect={(event) => { event.preventDefault(); props.onThemeMode(mode.id) }}>{mode.label}</DropdownMenu.RadioItem>
                ))}
              </div>
            </DropdownMenu.RadioGroup>
            <DropdownMenu.Separator className="mx-menu-sep" />
            {themes.map((theme) => (
              <DropdownMenu.Item key={theme.name} className="mx-menu-item" onSelect={() => { props.onPickTheme(theme.name) }}>
                <span className="mx-sb-check" aria-hidden="true">{theme.name === props.activeTheme ? <Icon name="check" size={12} /> : null}</span>{theme.name}
              </DropdownMenu.Item>
            ))}
            <DropdownMenu.Separator className="mx-menu-sep" />
            <DropdownMenu.Item className="mx-menu-item" onSelect={props.onAllThemes}><span className="mx-sb-check" />All themes in Settings…</DropdownMenu.Item>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <DropdownMenu.Root>
        <DropdownMenu.Trigger className="mx-sb-item mx-sb-button" aria-label={`Layout style: ${props.style}`}>
          <Icon name="layout" />{WINDOW_STYLES.find((item) => item.id === props.style)?.name}
        </DropdownMenu.Trigger>
        <DropdownMenu.Portal container={props.portalContainer}>
          <DropdownMenu.Content {...content('mx-menu mx-sb-menu')}>
            <DropdownMenu.RadioGroup value={props.style} onValueChange={(value) => { props.onStyle(value as WindowStyle) }}>
              {WINDOW_STYLES.map((item) => (
                <DropdownMenu.RadioItem key={item.id} value={item.id} className="mx-menu-item">
                  <span className="mx-sb-check" aria-hidden="true"><DropdownMenu.ItemIndicator><Icon name="check" size={12} /></DropdownMenu.ItemIndicator></span>{item.name}
                </DropdownMenu.RadioItem>
              ))}
            </DropdownMenu.RadioGroup>
          </DropdownMenu.Content>
        </DropdownMenu.Portal>
      </DropdownMenu.Root>
      <span className="mx-sb-size" role="group" aria-label="Terminal text size">
        <button type="button" className="mx-sb-step" aria-label="Smaller text" disabled={props.textSize <= props.minTextSize} onClick={() => { props.onTextSize(props.textSize - 1) }}>−</button>
        <button type="button" className="mx-sb-number" aria-label={`Text size ${String(props.textSize)}px. Reset to default`} onClick={props.onResetTextSize}>{props.textSize}px</button>
        <button type="button" className="mx-sb-step" aria-label="Larger text" disabled={props.textSize >= props.maxTextSize} onClick={() => { props.onTextSize(props.textSize + 1) }}>+</button>
      </span>
      <button type="button" className="mx-sb-item mx-sb-button" aria-label="Keyboard shortcuts" title="Keyboard shortcuts" onClick={props.onOpenShortcuts}>
        <Keycaps chord={props.shortcutChord} />
      </button>
    </footer>
  )
})
