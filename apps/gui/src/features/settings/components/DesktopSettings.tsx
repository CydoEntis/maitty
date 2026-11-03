import { Fragment, useEffect, useLayoutEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from 'react'
import { spinnerDefinition, type SpinnerStyle } from '@/lib/spinner-clock'
import { THEMES } from '@/features/workspace/theme/themes'
import { ThemePicker } from './ThemePicker'
import { hotkeyRows, isModifierSymbol, type Chord } from '@/features/workspace/model/shortcuts'
import { PROJECT_COLORS } from '@/features/workspace/model/folderColor'
import { Icon } from '@/features/workspace/components/glyphs'
import { FOLDER_PATH } from '@/features/workspace/components/paths'
import { SCROLLBACK_CHOICES, SIDEBAR_WIDTH_DEFAULT, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN, UI_TEXT_SIZE_DEFAULT, UI_TEXT_SIZE_LARGE, UI_TEXT_SIZE_SMALL, type CursorStyle, type ScrollbackLines, type SettingsPreferences, type StyleChoice } from '../preferences'

export type DesktopSettingsSection = 'general' | 'terminal' | 'appearance' | 'notifications' | 'keys' | 'daemon'

export type DaemonInfo = {
  online: boolean
  sessionCount: number
  /** Reported by maittyd; rows are left out until it does. */
  version?: string
  socket?: string
}

type DesktopSettingsProps = {
  section: DesktopSettingsSection
  /** The app's update check, when there is one to ask. */
  updates?: { summary: string; checking: boolean; onCheck: () => void }
  onSection: (section: DesktopSettingsSection) => void
  preferences: SettingsPreferences
  onChange: (preferences: SettingsPreferences) => void
  /** The system asks for reduced motion; the switch then cannot turn motion back on. */
  systemReducedMotion: boolean
  /** The operating system is set to dark; System mode follows it. */
  systemDark: boolean
  /** A theme hovered in a theme dropdown shows on the whole window; null ends the preview. */
  onPreviewTheme: (name: string | null) => void
  /** Closes Settings and plays the first-run welcome again. */
  onShowWelcome: () => void
  mac: boolean
  daemon: DaemonInfo
  /** The projects in the sidebar, so each can be given a colour. */
  projects: ReadonlyArray<{ key: string; name: string; colorId: string | undefined }>
  /** Resolves whether a typed folder exists. */
  validateFolder: (path: string) => Promise<boolean>
  onClose: () => void
}

type NavItem = { id: DesktopSettingsSection; label: string; key: string; tint: string; path: string }

const NAV: Array<{ label: string; items: NavItem[] }> = [
  { label: 'ESSENTIALS', items: [{ id: 'general', label: 'General', key: 'G', tint: 'var(--accent)', path: 'M21 4h-7M10 4H3M21 12h-9M8 12H3M21 20h-5M12 20H3M14 2v4M8 10v4M16 18v4' }] },
  {
    label: 'TERMINAL',
    items: [
      { id: 'terminal', label: 'Text & cursor', key: 'T', tint: 'var(--ansi-blue)', path: 'M4 7V4h16v3M9 20h6M12 4v16' },
      { id: 'appearance', label: 'Appearance', key: 'A', tint: 'var(--ansi-mag)', path: 'M12 3a9 9 0 1 0 0 18c1 0 1.5-.8 1.5-1.6 0-.9-.8-1.3-.8-2.2 0-.9.7-1.6 1.6-1.6H16a5 5 0 0 0 5-5c0-4.4-4-7.6-9-7.6zM7.5 11.5h.01M10.5 7.5h.01M15.5 8.5h.01' },
    ],
  },
  {
    label: 'WORKSPACE',
    items: [
      { id: 'notifications', label: 'Notifications', key: 'N', tint: 'var(--warn)', path: 'M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a1.9 1.9 0 0 0 3.4 0' },
      { id: 'keys', label: 'Hotkeys', key: '?', tint: 'var(--ansi-blue)', path: 'M2 6h20v12H2zM6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8' },
    ],
  },
  { label: 'ADVANCED', items: [{ id: 'daemon', label: 'Daemon', key: 'D', tint: 'var(--accent)', path: 'M3 4h18v6H3zM3 14h18v6H3zM7 7h.01M7 17h.01' }] },
]

const PAGE: Record<DesktopSettingsSection, [string, string]> = {
  general: ['General', 'Where sessions start and what happens to them.'],
  terminal: ['Text & cursor', 'How every pane renders. Changes apply instantly.'],
  appearance: ['Appearance', 'Colours, spinner and motion for the whole window.'],
  notifications: ['Notifications', 'When the system should tell you a session needs you.'],
  keys: ['Hotkeys', 'Everything the window handles before the shell does.'],
  daemon: ['Daemon', 'The local maittyd that owns every session.'],
}

const KEY_TO_SECTION: Record<string, DesktopSettingsSection> = { g: 'general', t: 'terminal', a: 'appearance', n: 'notifications', '?': 'keys', d: 'daemon' }

const FOCUSABLE = 'button:not([disabled]), input:not([disabled]), [tabindex]:not([tabindex="-1"])'

function tint(color: string): CSSProperties {
  return { '--tint': color } as CSSProperties
}

/** Settings (⌘,): a dialog in the window that holds focus until it closes, then gives it back. */
export function DesktopSettings(props: DesktopSettingsProps) {
  const { section, preferences, onChange } = props
  const dialog = useRef<HTMLDivElement>(null)
  const update = <K extends keyof SettingsPreferences>(key: K, value: SettingsPreferences[K]) => { onChange({ ...preferences, [key]: value }) }

  useLayoutEffect(() => {
    const restore = document.activeElement instanceof HTMLElement ? document.activeElement : null
    dialog.current?.querySelector<HTMLElement>('.mx-settings-nav-item.is-current')?.focus()
    return () => { restore?.focus() }
  }, [])

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
    if (event.key === 'Escape') {
      event.preventDefault()
      event.stopPropagation()
      props.onClose()
      return
    }
    if (event.key === 'Tab') {
      const focusable = Array.from(dialog.current?.querySelectorAll<HTMLElement>(FOCUSABLE) ?? [])
      const first = focusable[0]
      const last = focusable.at(-1)
      if (!first || !last) return
      if (event.shiftKey && document.activeElement === first) {
        event.preventDefault()
        last.focus()
      } else if (!event.shiftKey && document.activeElement === last) {
        event.preventDefault()
        first.focus()
      }
      return
    }
    const typing = event.target instanceof HTMLInputElement || event.target instanceof HTMLTextAreaElement
    if (typing || event.ctrlKey || event.metaKey || event.altKey) return
    const target = KEY_TO_SECTION[event.key.toLowerCase()]
    if (target) {
      event.preventDefault()
      props.onSection(target)
    }
  }

  const [title, subtitle] = PAGE[section]
  return (
    <div className="mx-settings-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) props.onClose() }}>
      <div ref={dialog} role="dialog" aria-modal="true" aria-label="Settings" className="mx-settings" onKeyDown={onKeyDown}>
        <nav aria-label="Settings sections" className="mx-settings-nav">
          <span className="mx-settings-nav-title">Settings</span>
          {NAV.map((group) => (
            <div key={group.label} className="mx-settings-nav-group">
              <span className="mx-settings-nav-label">{group.label}</span>
              {group.items.map((item) => (
                <button key={item.id} type="button" className={`mx-settings-nav-item${section === item.id ? ' is-current' : ''}`} aria-current={section === item.id ? 'page' : undefined} onClick={() => { props.onSection(item.id) }}>
                  <span className="mx-tint-tile" style={tint(item.tint)}>
                    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d={item.path} /></svg>
                  </span>
                  <span className="mx-settings-nav-text">{item.label}</span>
                  <span className="mx-settings-nav-key" aria-hidden="true">{item.key}</span>
                </button>
              ))}
            </div>
          ))}
        </nav>
        <div className="mx-settings-body">
          <header className="mx-settings-header">
            <div className="mx-settings-heading" key={section}>
              <h2>{title}</h2>
              <p>{subtitle}</p>
            </div>
            <button type="button" className="mx-settings-close" aria-label="Close settings" onClick={props.onClose}>Close<kbd>esc</kbd></button>
          </header>
          <div className="mx-settings-scroll">
            <div className="mx-settings-sections" key={section}>
              {section === 'general' && <GeneralSection preferences={preferences} update={update} validateFolder={props.validateFolder} onShowWelcome={props.onShowWelcome} updates={props.updates} />}
              {section === 'terminal' && <TerminalSection preferences={preferences} update={update} />}
              {section === 'appearance' && <AppearanceSection preferences={preferences} update={update} systemReducedMotion={props.systemReducedMotion} systemDark={props.systemDark} onPreviewTheme={props.onPreviewTheme} projects={props.projects} />}
              {section === 'notifications' && <NotificationsSection preferences={preferences} update={update} />}
              {section === 'keys' && <HotkeysSection mac={props.mac} />}
              {section === 'daemon' && <DaemonSection daemon={props.daemon} />}
            </div>
          </div>
        </div>
      </div>
    </div>
  )
}

type Update = <K extends keyof SettingsPreferences>(key: K, value: SettingsPreferences[K]) => void

function Card({ title, color, iconPath, meta, children, foot, footRow = false }: { title: string; color: string; iconPath: ReactNode; meta?: ReactNode; children?: ReactNode; foot?: ReactNode; footRow?: boolean }) {
  return (
    <section className="mx-scard" aria-label={title}>
      <div className="mx-scard-head">
        <span className="mx-scard-icon" style={tint(color)}>
          <svg width="11" height="11" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2.4" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">{iconPath}</svg>
        </span>
        <span className="mx-scard-title">{title}</span>
        {meta}
      </div>
      {children}
      {foot && <div className={`mx-scard-foot${footRow ? ' is-row' : ''}`}>{foot}</div>}
    </section>
  )
}

function Row({ label, detail, children }: { label: string; detail?: string; children: ReactNode }) {
  return (
    <div className="mx-scard-row">
      <div className="mx-scard-text"><span>{label}</span>{detail && <span>{detail}</span>}</div>
      {children}
    </div>
  )
}

export function Switch({ label, checked, disabled = false, onChange }: { label: string; checked: boolean; disabled?: boolean; onChange: (checked: boolean) => void }) {
  return (
    <button type="button" role="switch" aria-checked={checked} aria-label={label} disabled={disabled} className="mx-switch" onClick={() => { onChange(!checked) }}>
      <span className="mx-switch-knob" />
    </button>
  )
}

function Segmented<T extends string | number>({ label, value, options, onChange, tall = false }: { label: string; value: T; options: ReadonlyArray<readonly [T, string]>; onChange: (value: T) => void; tall?: boolean }) {
  return (
    <div role="radiogroup" aria-label={label} className={`mx-segmented${tall ? ' is-tall' : ''}`}>
      {options.map(([option, text]) => (
        <button key={String(option)} type="button" role="radio" aria-checked={option === value} onClick={() => { onChange(option) }}>{text}</button>
      ))}
    </div>
  )
}

const VALIDATE_DELAY_MS = 250
type FolderCheck = 'saved' | 'missing' | 'checking'

function GeneralSection({ preferences, update, validateFolder, onShowWelcome, updates }: { preferences: SettingsPreferences; update: Update; validateFolder: (path: string) => Promise<boolean>; onShowWelcome: () => void; updates: DesktopSettingsProps['updates'] }) {
  const [draft, setDraft] = useState(preferences.startFolder)
  const [check, setCheck] = useState<FolderCheck>('saved')
  const latest = useRef({ preferences, update })
  useEffect(() => { latest.current = { preferences, update } })
  useEffect(() => {
    const typed = draft.trim()
    if (typed === latest.current.preferences.startFolder) {
      setCheck('saved')
      return
    }
    setCheck('checking')
    let current = true
    const timer = window.setTimeout(() => {
      void validateFolder(typed)
        .catch(() => false)
        .then((exists) => {
          if (!current) return
          setCheck(exists ? 'saved' : 'missing')
          if (exists) latest.current.update('startFolder', typed)
        })
    }, VALIDATE_DELAY_MS)
    return () => {
      current = false
      window.clearTimeout(timer)
    }
  }, [draft, validateFolder])
  const missing = check === 'missing'
  return (
    <>
      <Card
        title="Start folder"
        color="var(--accent)"
        iconPath={<path d={FOLDER_PATH} />}
        meta={<span className="mx-scard-meta" style={{ fontSize: 11.5, color: missing ? 'var(--danger)' : 'var(--dim)' }} role="status">{missing ? 'Folder not found' : check === 'checking' ? 'Checking…' : 'Saved'}</span>}
        foot="Every new session opens a shell here. Leave it empty for your home folder. Projects appear in the sidebar when a session cds into a git repo."
      >
        <div style={{ padding: '0 16px 14px' }}>
          <div className={`mx-folder-field${missing ? ' is-invalid' : ''}`}>
            <span className="mono" style={{ fontSize: 12, color: 'var(--dim)' }}>cd</span>
            <input value={draft} placeholder="~" aria-label="Start folder" aria-invalid={missing} spellCheck={false} onChange={(event) => { setDraft(event.target.value) }} />
          </div>
        </div>
      </Card>
      <Card title="Window" color="var(--ansi-blue)" iconPath={<><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M3 16h18" /></>}>
        <Row label="Status bar" detail="A line at the bottom with the daemon, the branch, what is working and what needs you.">
          <Switch label="Status bar" checked={preferences.statusBar} onChange={(value) => { update('statusBar', value) }} />
        </Row>
      </Card>
      {updates && (
        <Card title="Updates" color="var(--ansi-green)" iconPath={<path d="M12 4v10m0 0l-4-4m4 4l4-4M5 19h14" />}>
          <Row label="Maitty updates" detail={updates.summary}>
            <button type="button" className="mx-text-button" disabled={updates.checking} onClick={updates.onCheck}>Check now</button>
          </Row>
        </Card>
      )}
      <Card title="Sessions" color="var(--ansi-mag)" iconPath={<><rect x="4" y="4" width="16" height="7" rx="2" /><rect x="4" y="13" width="16" height="7" rx="2" /><path d="M8 7.5h.01M8 16.5h.01" /></>}>
        <Row label="Reopen tabs on launch" detail="Restore the tabs and splits you had last time.">
          <Switch label="Reopen tabs on launch" checked={preferences.restoreTabs} onChange={(value) => { update('restoreTabs', value) }} />
        </Row>
        <Row label="Group sessions by" detail="Sessions outside a git repo are grouped by their own folder.">
          <Segmented label="Group sessions by" value={preferences.groupBy} options={[['repo', 'Repository'], ['folder', 'Folder']] as const} onChange={(value) => { update('groupBy', value) }} />
        </Row>
        <Row label="Closing the window" detail="Sessions are owned by maittyd and keep running.">
          <span style={{ fontSize: 12, color: 'var(--dim)' }}>Always keeps running</span>
        </Row>
        <Row label="Welcome" detail="Replay the short intro: where sessions start and the keys to know.">
          <button type="button" className="mx-scard-button" onClick={onShowWelcome}>Show the welcome again</button>
        </Row>
      </Card>
    </>
  )
}

/** The colours with one project's choice set, or cleared when `id` is null. */
function setColor(colors: Record<string, string>, key: string, id: string | null): Record<string, string> {
  const rest = Object.fromEntries(Object.entries(colors).filter(([project]) => project !== key))
  return id === null ? rest : { ...rest, [key]: id }
}

const STYLE_CARDS: ReadonlyArray<{ id: StyleChoice; name: string; blurb: string; foot: string }> = [
  { id: 'flat', name: 'Flat', blurb: 'Solid panels, soft corners, quiet', foot: 'Solid panels, no glow. Status sweeps start off.' },
  { id: 'tiled', name: 'Tiled', blurb: 'Edge-to-edge panes, square corners, hairline dividers', foot: 'Edge-to-edge panes with square corners. Status sweeps start off.' },
  { id: 'glass', name: 'Glass', blurb: 'Floating frosted cards, colour icons, status sweeps', foot: 'Floating glass cards and glow.' },
]
const ICON_COLOR_OPTIONS = [['color', 'Color'], ['grayscale', 'Grayscale']] as const
const TRAY_OPTIONS = [['names', 'With names'], ['icons', 'Icons'], ['stacked', 'Stacked']] as const
const LEAD_OPTIONS = [['focused', 'Focused'], ['attention', 'Needs attention']] as const
const ICON_OPTIONS = [['small', 'Small'], ['medium', 'Medium'], ['large', 'Large']] as const
const DENSITY_OPTIONS = [['compact', 'Compact'], ['comfortable', 'Comfortable'], ['spacious', 'Spacious']] as const
const UI_TEXT_OPTIONS = [[UI_TEXT_SIZE_SMALL, 'Small'], [UI_TEXT_SIZE_DEFAULT, 'Medium'], [UI_TEXT_SIZE_LARGE, 'Large']] as const

const FONT_SIZES = Array.from({ length: TERMINAL_FONT_SIZE_MAX - TERMINAL_FONT_SIZE_MIN + 1 }, (_, index) => {
  const size = TERMINAL_FONT_SIZE_MIN + index
  return [size, String(size)] as const
})
const CURSORS: ReadonlyArray<readonly [CursorStyle, string]> = [['block', 'Block'], ['bar', 'Bar'], ['underline', 'Underline']]
const THOUSAND = 1000
const SCROLLBACKS: ReadonlyArray<readonly [ScrollbackLines, string]> = SCROLLBACK_CHOICES.map((lines) => [lines, `${String(lines / THOUSAND)}k`] as const)

function caretStyle(cursor: CursorStyle): CSSProperties {
  return { width: cursor === 'bar' ? '2px' : '.6em', height: cursor === 'underline' ? '2px' : '1.25em', alignSelf: cursor === 'underline' ? 'flex-end' : 'center' }
}

function PromptPrefix() {
  return <><span style={{ color: 'var(--accent)', fontWeight: 600 }}>maitty</span><span style={{ color: 'var(--ansi-mag)' }}> (main)</span><span style={{ color: 'var(--dim)' }}> ❯ </span></>
}

function TerminalSection({ preferences, update }: { preferences: SettingsPreferences; update: Update }) {
  return (
    <>
      <Card title="Text" color="var(--ansi-blue)" iconPath={<path d="M5 6h14M12 6v12" />} meta={<span className="mono" style={{ fontSize: 12, color: 'var(--fg2)' }}>Geist Mono · {preferences.terminalFontSize}px</span>}>
        <div style={{ padding: '0 16px 14px', display: 'flex', flexDirection: 'column', gap: 12 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Size</span>
            <Segmented label="Terminal text size" value={preferences.terminalFontSize} options={FONT_SIZES} onChange={(value) => { update('terminalFontSize', value) }} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Cursor</span>
            <Segmented label="Cursor" value={preferences.cursorStyle} options={CURSORS} onChange={(value) => { update('cursorStyle', value) }} />
          </div>
          <div className="mx-preview" style={{ fontSize: preferences.terminalFontSize }} aria-label="Terminal preview" role="img">
            <div><PromptPrefix /><span style={{ fontWeight: 600 }}>cargo</span> test <span style={{ color: 'var(--ansi-blue)' }}>-p</span> maitty-core</div>
            <div style={{ color: 'var(--fg2)' }}>test pane_terminal::replay_then_stream ... <span style={{ color: 'var(--ok)' }}>ok</span></div>
            <div style={{ display: 'flex', alignItems: 'center' }}><PromptPrefix /><span className="mx-caret" style={caretStyle(preferences.cursorStyle)} /></div>
          </div>
        </div>
      </Card>
      <Card title="Scrollback" color="var(--ansi-green)" iconPath={<path d="m5 8 4 4-4 4M12 17h7" />}>
        <Row label="Lines kept per pane" detail="Older output is dropped from the GUI. maittyd keeps the full history.">
          <Segmented label="Lines kept per pane" value={preferences.scrollback} options={SCROLLBACKS} onChange={(value) => { update('scrollback', value) }} />
        </Row>
      </Card>
    </>
  )
}

const SPINNERS: ReadonlyArray<readonly [SpinnerStyle, string]> = [['braille', 'Braille'], ['snake', 'Snake'], ['bars', 'Bars']]
const PREVIEW_TICK_MS = 40
const SECONDS_PER_MINUTE = 60
const SECONDS_DIGITS = 2

function usePreviewClock(running: boolean): number {
  const [now, setNow] = useState(() => Date.now())
  useEffect(() => {
    if (!running) return
    const timer = window.setInterval(() => { setNow(Date.now()) }, PREVIEW_TICK_MS)
    return () => { window.clearInterval(timer) }
  }, [running])
  return now
}

const MODE_OPTIONS = [['system', 'System'], ['dark', 'Dark'], ['light', 'Light']] as const

function AppearanceSection({ preferences, update, systemReducedMotion, systemDark, onPreviewTheme, projects }: { preferences: SettingsPreferences; update: Update; systemReducedMotion: boolean; systemDark: boolean; onPreviewTheme: (name: string | null) => void; projects: DesktopSettingsProps['projects'] }) {
  const reduced = preferences.reduceMotion || systemReducedMotion
  const now = usePreviewClock(!reduced)
  const darkInUse = preferences.themeMode === 'dark' || (preferences.themeMode === 'system' && systemDark)
  const modeNote = preferences.themeMode === 'system' ? `Follows your OS. Right now: ${systemDark ? 'dark' : 'light'}.` : `Always uses your ${preferences.themeMode} theme.`
  const seconds = String(Math.floor(now / THOUSAND) % SECONDS_PER_MINUTE).padStart(SECONDS_DIGITS, '0')
  const effects: Array<{ key: 'effectWorking' | 'effectNeedsYou' | 'effectDone'; label: string; detail: string; color: string; glyph: ReactNode; short: string; fx: string }> = [
    { key: 'effectWorking', label: 'Working', detail: 'Teal sweep that repeats', color: 'var(--accent)', glyph: <SpinnerFrame spinner={preferences.spinner} now={now} reduced={reduced} />, short: '0:42', fx: 'mx-fx-working mx-fx-strong' },
    { key: 'effectNeedsYou', label: 'Needs you', detail: 'Amber glow that breathes', color: 'var(--warn)', glyph: '◆', short: 'Allow?', fx: 'mx-fx-needs mx-fx-strong' },
    { key: 'effectDone', label: 'Done', detail: 'One green sweep, then still', color: 'var(--ok)', glyph: '✓', short: 'Done', fx: 'mx-fx-done mx-fx-strong' },
  ]
  return (
    <>
      <Card title="Window style" color="var(--accent)" iconPath={<><rect x="4" y="5" width="16" height="14" rx="3" /><path d="M4 10h16" /></>} meta={<span className="mx-scard-meta">Same features, different finish</span>}
        foot={STYLE_CARDS.find((card) => card.id === preferences.style)?.foot}
      >
        <div role="radiogroup" aria-label="Window style" className="mx-style-cards">
          {STYLE_CARDS.map((card) => {
            const selected = card.id === preferences.style
            return (
              <button key={card.id} type="button" role="radio" aria-checked={selected} className={`mx-style-card${selected ? ' is-selected' : ''}`} onClick={() => { update('style', card.id) }}>
                <span className={`mx-style-preview is-${card.id}`} aria-hidden="true"><i className="mx-sp-side" /><i className="mx-sp-pane"><b /><b /><b /></i></span>
                <span className="mx-style-name">{card.name}{selected && <span aria-hidden="true">✓</span>}</span>
                <span className="mx-style-blurb">{card.blurb}</span>
              </button>
            )
          })}
        </div>
        <Row label="Icons" detail="Agent and folder icons in their own colours, or neutral gray tiles.">
          <Segmented label="Icons" value={preferences.iconColor} options={ICON_COLOR_OPTIONS} onChange={(value) => { update('iconColor', value) }} />
        </Row>
      </Card>
      <Card title="Theme" color="var(--ansi-mag)" iconPath={<><circle cx="12" cy="12" r="8" /><path d="M12 4a8 8 0 0 0 0 16" /></>} meta={<span className="mx-scard-meta">{THEMES.length} installed</span>}
        foot="Amber and red keep their meaning in every theme."
      >
        <Row label="Appearance" detail={modeNote}>
          <Segmented label="Appearance" value={preferences.themeMode} options={MODE_OPTIONS} onChange={(value) => { update('themeMode', value) }} />
        </Row>
        <Row label="Dark theme" detail="Used when the window is dark.">
          <ThemePicker kind="dark" value={preferences.darkTheme} inUse={darkInUse} dimmed={!darkInUse && preferences.themeMode !== 'system'} onPick={(name) => { update('darkTheme', name) }} onPreview={onPreviewTheme} />
        </Row>
        <Row label="Light theme" detail="Used when the window is light.">
          <ThemePicker kind="light" value={preferences.lightTheme} inUse={!darkInUse} dimmed={darkInUse && preferences.themeMode !== 'system'} onPick={(name) => { update('lightTheme', name) }} onPreview={onPreviewTheme} />
        </Row>
      </Card>
      <Card title="Working spinner" color="var(--accent)" iconPath={<path d="M12 3a9 9 0 1 0 9 9" />} meta={<span className="mx-scard-meta">Live preview</span>} foot="Same frames as the Maitty TUI. Shown on tabs, pane headers and sidebar rows while something runs.">
        <div role="radiogroup" aria-label="Working spinner" className="mx-spinner-grid">
          {SPINNERS.map(([id, name]) => {
            const definition = spinnerDefinition(id)
            const selected = preferences.spinner === id
            return (
              <button key={id} type="button" role="radio" aria-checked={selected} aria-label={name} className="mx-spinner-card" onClick={() => { update('spinner', id) }}>
                <span style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
                  <span className="mx-spinner-glyph"><SpinnerFrame spinner={id} now={now} reduced={reduced} /></span>
                  <span style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 2 }}>
                    <span style={{ fontSize: 13, fontWeight: 600, color: 'var(--fg)' }}>{name}</span>
                    <span style={{ fontSize: 11.5, color: 'var(--muted)', whiteSpace: 'nowrap' }}>{definition.frames.length} frames · {definition.intervalMs} ms</span>
                  </span>
                  {selected && <Icon name="check" size={15} stroke="var(--accent)" strokeWidth={2.5} />}
                </span>
                <span className="mx-spinner-sample"><span style={{ color: 'var(--accent)' }}><SpinnerFrame spinner={id} now={now} reduced={reduced} /></span>cargo test<span style={{ flex: 1 }} /><span style={{ color: 'var(--dim)' }}>0:{seconds}</span></span>
              </button>
            )
          })}
        </div>
      </Card>
      <Card title="Sidebar" color="var(--accent)" iconPath={<><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M9 4v16" /></>}
        foot={<>{preferences.trayStyle === 'stacked' ? 'One per line under the main row, with folder and status.' : preferences.trayStyle === 'icons' ? 'Small icon tabs under the main row.' : 'Small named tabs under the main row.'} {preferences.leadMode === 'attention' ? 'The session that most needs you leads: needs you, error, done, working.' : 'The session you are in leads; the others are sorted by attention.'}</>}
      >
        <Row label="Other sessions in a project" detail="How a project's extra sessions sit under its main row.">
          <Segmented label="Other sessions in a project" value={preferences.trayStyle} options={TRAY_OPTIONS} onChange={(value) => { update('trayStyle', value) }} />
        </Row>
        <Row label="Main row shows" detail="Which session leads each project.">
          <Segmented label="Main row shows" value={preferences.leadMode} options={LEAD_OPTIONS} onChange={(value) => { update('leadMode', value) }} />
        </Row>
        {projects.length > 0 && (
          <div className="mx-scard-row is-stacked">
            <div className="mx-scard-text"><span>Project colors</span><span>Amber and red are left out: they only ever mean needs you and error.</span></div>
            {projects.map((project) => (
              <div key={project.key} className="mx-project-color">
                <span className="mx-project-color-name">{project.name}</span>
                <div role="radiogroup" aria-label={`Color for ${project.name}`} className="mx-swatches">
                  {PROJECT_COLORS.map((color) => (
                    <button key={color.id} type="button" role="radio" aria-checked={project.colorId === color.id} aria-label={color.label} title={color.label} className="mx-swatch-dot" style={{ background: color.color }} onClick={() => { update('projectColors', setColor(preferences.projectColors, project.key, project.colorId === color.id ? null : color.id)) }} />
                  ))}
                </div>
              </div>
            ))}
          </div>
        )}
      </Card>
      <Card title="Interface size" color="var(--ansi-green)" iconPath={<path d="M4 7h16M4 12h10M4 17h13" />}>
        <Row label="Density" detail="How much room the sidebar rows, tabs and panes get.">
          <Segmented label="Density" value={preferences.density} options={DENSITY_OPTIONS} onChange={(value) => { update('density', value) }} />
        </Row>
        <Row label="Sidebar icons" detail="The project tiles, session icons and the collapsed rail.">
          <Segmented label="Sidebar icon size" value={preferences.iconSize} options={ICON_OPTIONS} onChange={(value) => { update('iconSize', value) }} />
        </Row>
        <Row label="Sidebar text" detail="Folder names and session rows.">
          <Segmented label="Sidebar text size" value={preferences.sidebarTextSize} options={UI_TEXT_OPTIONS} onChange={(value) => { update('sidebarTextSize', value) }} />
        </Row>
        <Row label="Tab text" detail="The names on the tab bar.">
          <Segmented label="Tab text size" value={preferences.tabTextSize} options={UI_TEXT_OPTIONS} onChange={(value) => { update('tabTextSize', value) }} />
        </Row>
        <Row label="Sidebar width" detail="Drag the sidebar's edge to change it, or focus the edge and use the arrow keys.">
          <span className="mono" style={{ fontSize: 12, color: 'var(--fg2)' }}>{preferences.sidebarWidth}px</span>
          <button type="button" className="mx-text-button" disabled={preferences.sidebarWidth === SIDEBAR_WIDTH_DEFAULT} onClick={() => { update('sidebarWidth', SIDEBAR_WIDTH_DEFAULT) }}>Reset</button>
        </Row>
      </Card>
      <Card title="Motion" color="var(--ansi-blue)" iconPath={<><circle cx="12" cy="12" r="8" /><path d="M12 4a8 8 0 0 0 0 16" /></>}>
        <Row label="Reduce motion" detail="Turns off the shimmer, pulses and slide-ins. Your system setting is always respected.">
          <Switch label="Reduce motion" checked={preferences.reduceMotion} onChange={(value) => { update('reduceMotion', value) }} />
        </Row>
        <div className={`mx-effects${reduced ? ' is-locked' : ''}`}>
          <div className="mx-scard-text"><span>Status effects</span><span>{reduced ? 'Off while Reduce motion is on.' : 'Choose which states animate on sidebar rows and tabs. Icons and words always show.'}</span></div>
          {effects.map((effect) => {
            const on = preferences[effect.key]
            return (
              <div key={effect.key} className="mx-effect-row">
                <span className={`mx-effect-chip${on && !reduced ? ` ${effect.fx}` : ''}`} style={{ color: effect.color }} aria-hidden="true">{effect.glyph}<span>{effect.short}</span></span>
                <span style={{ flex: 1, minWidth: 0, display: 'flex', flexDirection: 'column', gap: 1 }}>
                  <span style={{ fontSize: 12.5, color: 'var(--fg)' }}>{effect.label}</span>
                  <span style={{ fontSize: 11.5, color: 'var(--dim)' }}>{effect.detail}</span>
                </span>
                <Switch label={effect.label} checked={on && !reduced} disabled={reduced} onChange={(value) => { update(effect.key, value) }} />
              </div>
            )
          })}
        </div>
      </Card>
    </>
  )
}

function SpinnerFrame({ spinner, now, reduced }: { spinner: SpinnerStyle; now: number; reduced: boolean }) {
  const { frames, intervalMs } = spinnerDefinition(spinner)
  return <>{frames[reduced ? 0 : Math.floor(now / intervalMs) % frames.length]}</>
}

/** A key combination as a row of key-shaped caps; the modifier symbols are set in the interface font. */
function Keycaps({ chord }: { chord: Chord }) {
  return (
    <span className="mx-chord" role="group" aria-label={chord.join(' ')}>
      {chord.map((key) => <kbd key={key} className={`mx-keycap${isModifierSymbol(key) ? ' is-symbol' : ''}`}>{key}</kbd>)}
    </span>
  )
}

function HotkeysSection({ mac }: { mac: boolean }) {
  return (
    <Card title="Hotkeys" color="var(--ansi-blue)" iconPath={<><rect x="3" y="7" width="18" height="10" rx="2" /><path d="M7 11h.01M11 11h.01M15 11h.01M8 14h8" /></>} meta={<span className="mx-scard-meta">{mac ? '⌘ is Ctrl on Windows and Linux' : 'Ctrl is ⌘ on macOS'}</span>} foot="In a focused pane, every other key goes straight to the shell.">
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {hotkeyRows(mac).map((row) => (
          <div key={row.action} className="mx-hotkey-row">
            <span>{row.action}</span>
            <span className="mx-keys">
              {row.chords.map((chord, index) => (
                <Fragment key={chord.join('+')}>
                  {index > 0 && <span className="mx-or">or</span>}
                  <Keycaps chord={chord} />
                </Fragment>
              ))}
            </span>
          </div>
        ))}
      </div>
    </Card>
  )
}

/** Native notifications: sent only while the window is in the background, for the session you are not looking at. */
function NotificationsSection({ preferences, update }: { preferences: SettingsPreferences; update: Update }) {
  return (
    <Card
      title="System notifications"
      color="var(--warn)"
      iconPath={<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a1.9 1.9 0 0 0 3.4 0" />}
      foot="Clicking a notification brings the window forward and shows that session. A notification names the session and where it is; it never shows what the agent asked, so nothing private appears on a locked screen. The system asks once whether Maitty may send them."
    >
      <Row label="Needs you" detail="An agent stopped for an answer or an approval.">
        <Switch label="Notify when a session needs you" checked={preferences.notifyNeedsYou} onChange={(value) => { update('notifyNeedsYou', value) }} />
      </Row>
      <Row label="Done" detail="An agent finished its turn.">
        <Switch label="Notify when a session finishes" checked={preferences.notifyFinished} onChange={(value) => { update('notifyFinished', value) }} />
      </Row>
      <Row label="Error" detail="A session's program exited with an error.">
        <Switch label="Notify when a session errors" checked={preferences.notifyError} onChange={(value) => { update('notifyError', value) }} />
      </Row>
    </Card>
  )
}

function DaemonSection({ daemon }: { daemon: DaemonInfo }) {
  const color = daemon.online ? 'var(--ok)' : 'var(--danger)'
  return (
    <Card title="maittyd" color="var(--ansi-green)" iconPath={<><rect x="4" y="4" width="16" height="7" rx="2" /><rect x="4" y="13" width="16" height="7" rx="2" /><path d="M8 7.5h.01M8 16.5h.01" /></>}
      meta={<span role="status" style={{ display: 'flex', alignItems: 'center', gap: 6, fontSize: 12, color }}><span style={{ width: 7, height: 7, borderRadius: '50%', background: color }} />{daemon.online ? 'Connected' : 'Disconnected'}</span>}
    >
      {daemon.version && <Row label="Version" detail="Shared by the GUI and the Maitty TUI."><span className="mx-scard-value">{daemon.version}</span></Row>}
      {daemon.socket && <Row label="Socket" detail="Where clients connect."><span className="mx-scard-value">{daemon.socket}</span></Row>}
      <Row label="Sessions" detail="Owned by the daemon. The GUI only attaches."><span className="mx-scard-value">{daemon.sessionCount}</span></Row>
    </Card>
  )
}

