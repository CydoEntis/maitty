import { toast } from '@/lib/toast'
import type { UpdaterState } from '@/features/updates/useUpdater'
import { Fragment, useEffect, useRef, useState, type CSSProperties, type KeyboardEvent, type ReactNode } from 'react'
import { THEMES } from '@/features/workspace/theme/themes'
import { ThemePicker } from './ThemePicker'
import { WINDOW_STYLES, isWindowStyle, type WindowStyleEntry } from '@/features/workspace/theme/windowStyles'
import { Panel } from '@/components/ui/panel'
import { PickerSelect, type PickerGroup, type PickerItem } from './PickerSelect'
import { Segmented, SegmentedItem } from '@/components/ui/radio-group'
import { Switch } from '@/components/ui/switch'
import { SpinnerPicker } from './SpinnerPicker'
import { PerformanceSection } from './PerformanceSection'
import { hotkeyRows, type Platform } from '@/features/workspace/model/shortcuts'
import { CHAT_AGENTS } from '@/features/workspace/model/chatView'
import { Keycaps } from '@/features/shortcuts/Keycaps'
import { Icon } from '@/features/workspace/components/glyphs'
import { errorMessage } from '@/lib/error-message'
import { BUNDLED_FONT_NAME, fontStackFor, installedMonospaceFamilies, WEIGHT_BOLD, WEIGHT_EXTRA_BOLD, WEIGHT_LIGHT, WEIGHT_MEDIUM, WEIGHT_REGULAR, WEIGHT_SEMIBOLD, type TerminalFontWeight } from '@/lib/terminalFont'
import type { Added, Grant, HostChoice, MachineActivity } from '@/api/machines'
import { DisconnectedSince } from '@/features/workspace/components/times'
import { installNote, UPDATE_WORDS, updateVerb, updateWarning, type MachineHead, type UpdateAction } from '@/features/workspace/model/machines'
import { UsagePage } from '@/features/usage/UsagePage'
import type { OpenSession } from '@/features/usage/UsageTable'
import { FOLDER_PATH } from '@/features/workspace/components/paths'
import { SCROLL_SPEED_CHOICES, SCROLLBACK_CHOICES, SIDEBAR_WIDTH_DEFAULT, TERMINAL_FONT_SIZE_MAX, TERMINAL_FONT_SIZE_MIN, UI_TEXT_SIZE_DEFAULT, UI_TEXT_SIZE_LARGE, UI_TEXT_SIZE_SMALL, type CursorStyle, type ScrollbackLines, type SidebarOrder, type ScrollSpeed, type SettingsPreferences } from '../preferences'

export type DesktopSettingsSection = 'general' | 'terminal' | 'appearance' | 'usage' | 'notifications' | 'machines' | 'keys' | 'daemon' | 'performance'

/** The other machines, for the Machines page: what the window knows of each, and what it can ask of the host. */
export type MachinesInfo = {
  /** The host has answered about the machines: before that the page says it is loading. */
  known: boolean
  list: readonly MachineHead[]
  onRetry: (name: string) => void
  onRemove: (name: string) => void
  /** Allow or stop one of a machine's powers over this computer. */
  onGrant: (name: string, grant: Grant, on: boolean) => void
  /**
   * Installs this version on a machine and restarts its daemon there; its saved sessions come
   * back. Settles when the update ended and has already told the person how it went.
   */
  onUpdate: (name: string, action: UpdateAction, whenIdle: boolean) => Promise<void>
  /**
   * How many sessions are open on a machine and how many are busy; null when it cannot say (its
   * app is too old, or it cannot be reached). The update question uses it to say what it will interrupt.
   */
  onCheckActivity: (name: string) => Promise<MachineActivity | null>
  /** Puts the same command on the clipboard, to run in a terminal instead. */
  onCopyCommand: (command: string) => void
  add: {
    /** Windows has no ssh agent by default: the page says to use keys. */
    windows: boolean
    /** The hosts of ~/.ssh/config that are not machines yet. Stable between renders. */
    loadHosts: () => Promise<HostChoice[]>
    /** Looks at the target and adds it. Rejects with the words to show when it cannot be added. */
    onAdd: (target: string, name?: string) => Promise<Added>
    /** Ends the look that is running. */
    onCancel: () => void
  }
}

export type DaemonInfo = {
  online: boolean
  sessionCount: number
  /** Reported by maittyd; rows are left out until it does. */
  version?: string
  socket?: string
}

export type UsageInfo = { budget: number | null; onBudget: (budget: number | null) => void; sessionForTerm: (term: number) => OpenSession | null; onOpenSession: (id: string) => void }

type DesktopSettingsProps = {
  section: DesktopSettingsSection
  /** The app's update check, when there is one to ask. */
  /** Shows a sample notification where notifications appear. */
  onTestNotice?: () => void
  updates?: { summary: string; status: UpdaterState['status']; onCheck: () => void; onDownload: () => void; onRestart: () => void }
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
  platform: Platform
  daemon: DaemonInfo
  /** The running app's version, for the performance report; null until known. */
  appVersion: string | null
  machines: MachinesInfo
  /** Resolves whether a typed folder exists. */
  validateFolder: (path: string) => Promise<boolean>
  /** Opens the system's folder picker at the folder given; the path picked, or null when it is cancelled. */
  onChooseFolder?: (current: string) => Promise<string | null>
  /** What the Usage & cost page needs from the window. */
  usage: UsageInfo
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
      { id: 'usage', label: 'Usage & cost', key: 'U', tint: 'var(--ansi-mag)', path: 'M4 20V10M10 20V4M16 20v-7M22 20H2' },
      { id: 'notifications', label: 'Notifications', key: 'N', tint: 'var(--warn)', path: 'M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a1.9 1.9 0 0 0 3.4 0' },
      { id: 'machines', label: 'Machines', key: 'M', tint: 'var(--ansi-green)', path: 'M3 4h18v6H3zM3 14h18v6H3zM7 7h.01M7 17h.01M17 7h.01M17 17h.01' },
      { id: 'keys', label: 'Hotkeys', key: '?', tint: 'var(--ansi-blue)', path: 'M2 6h20v12H2zM6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8' },
    ],
  },
  {
    label: 'ADVANCED',
    items: [
      { id: 'daemon', label: 'Daemon', key: 'D', tint: 'var(--accent)', path: 'M3 4h18v6H3zM3 14h18v6H3zM7 7h.01M7 17h.01' },
      { id: 'performance', label: 'Performance', key: 'P', tint: 'var(--ansi-green)', path: 'M3 17l5-6 4 4 5-8 4 5' },
    ],
  },
]

const PAGE: Record<DesktopSettingsSection, [string, string]> = {
  general: ['General', 'Where sessions start and what happens to them.'],
  terminal: ['Text & cursor', 'How every pane renders. Changes apply instantly.'],
  appearance: ['Appearance', 'Colours, spinner and motion for the whole window.'],
  usage: ['Usage & cost', 'What the agents reported using, by session. Never estimated.'],
  notifications: ['Notifications', 'When the system should tell you a session needs you.'],
  machines: ['Machines', 'Other computers whose sessions show up beside this one.'],
  keys: ['Hotkeys', 'Everything the window handles before the shell does.'],
  daemon: ['Daemon', 'The local maittyd that owns every session.'],
  performance: ['Performance', 'What the window is spending its time on, measured live.'],
}

const KEY_TO_SECTION: Record<string, DesktopSettingsSection> = { g: 'general', t: 'terminal', a: 'appearance', u: 'usage', n: 'notifications', m: 'machines', '?': 'keys', d: 'daemon', p: 'performance' }

/** Esc ends an open theme list, or a running add-machine, first; Settings stays. A part that handles Esc itself is marked `data-owns-escape`. */
const keepForOpenList = (event: globalThis.KeyboardEvent) => {
  const inside = event.target instanceof Element && event.target.closest('[data-owns-escape]') !== null
  if (inside || document.querySelector('.mx-picker-list') !== null) event.preventDefault()
}

function tint(color: string): CSSProperties {
  return { '--tint': color } as CSSProperties
}

/** Settings (⌘,): a dialog in the window that holds focus until it closes, then gives it back. */
export function DesktopSettings(props: DesktopSettingsProps) {
  const { section, preferences, onChange } = props
  const update = <K extends keyof SettingsPreferences>(key: K, value: SettingsPreferences[K]) => { onChange({ ...preferences, [key]: value }) }

  // Focus lands on the current section, not the first control; Radix traps it and gives it back on close.
  const focusSection = (event: Event) => {
    event.preventDefault()
    if (event.currentTarget instanceof HTMLElement) event.currentTarget.querySelector<HTMLElement>('.mx-settings-nav-item.is-current')?.focus()
  }

  const onKeyDown = (event: KeyboardEvent<HTMLDivElement>) => {
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
    <Panel label="Settings" onClose={props.onClose} scrimClassName="mx-settings-scrim" className="mx-settings" onKeyDown={onKeyDown} onOpenAutoFocus={focusSection} onEscapeKeyDown={keepForOpenList}>
      <>
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
            <button type="button" className="mx-icon-button mx-settings-close" aria-label="Close settings" title="Close (Esc)" onClick={props.onClose}><Icon name="close" size={14} /></button>
          </header>
          <div className="mx-settings-scroll">
            <div className="mx-settings-sections" key={section}>
              {section === 'general' && <GeneralSection preferences={preferences} update={update} validateFolder={props.validateFolder} onChooseFolder={props.onChooseFolder} onShowWelcome={props.onShowWelcome} updates={props.updates} />}
              {section === 'terminal' && <TerminalSection preferences={preferences} update={update} />}
              {section === 'appearance' && <AppearanceSection preferences={preferences} update={update} systemReducedMotion={props.systemReducedMotion} systemDark={props.systemDark} onPreviewTheme={props.onPreviewTheme} />}
              {section === 'usage' && <UsageSection usage={props.usage} />}
              {section === 'notifications' && <NotificationsSection preferences={preferences} update={update} onTestNotice={props.onTestNotice} />}
              {section === 'machines' && <MachinesSection machines={props.machines} />}
              {section === 'keys' && <HotkeysSection platform={props.platform} />}
              {section === 'daemon' && <DaemonSection daemon={props.daemon} />}
              {section === 'performance' && <PerformanceSection preferences={preferences} update={update} appVersion={props.appVersion} />}
            </div>
          </div>
        </div>
      </>
    </Panel>
  )
}

type Update = <K extends keyof SettingsPreferences>(key: K, value: SettingsPreferences[K]) => void

export function Card({ title, color, iconPath, meta, children, foot, footRow = false }: { title: string; color: string; iconPath: ReactNode; meta?: ReactNode; children?: ReactNode; foot?: ReactNode; footRow?: boolean }) {
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

export function Row({ label, detail, children }: { label: string; detail?: string; children: ReactNode }) {
  return (
    <div className="mx-scard-row">
      <div className="mx-scard-text"><span>{label}</span>{detail && <span>{detail}</span>}</div>
      {children}
    </div>
  )
}

function SegmentedChoice<T extends string | number>({ label, value, options, onChange, tall = false }: { label: string; value: T; options: ReadonlyArray<readonly [T, ReactNode]>; onChange: (value: T) => void; tall?: boolean }) {
  return (
    <Segmented
      aria-label={label}
      tall={tall}
      value={String(value)}
      onValueChange={(next) => {
        const picked = options.find(([option]) => String(option) === next)
        if (picked) onChange(picked[0])
      }}
    >
      {options.map(([option, text]) => <SegmentedItem key={String(option)} value={String(option)}>{text}</SegmentedItem>)}
    </Segmented>
  )
}

const VALIDATE_DELAY_MS = 250
type FolderCheck = 'saved' | 'missing' | 'checking'

const YOLO_AGENTS = [
  { key: 'yoloClaude', label: 'Claude', detail: 'Lets you pick Bypass permissions in a chat, where Claude asks about nothing. New chats still start in the mode you had.' },
  { key: 'yoloCodex', label: 'Codex', detail: 'Starts new Codex sessions with approvals and the sandbox off, so it runs commands and edits files without asking.' },
  { key: 'yoloGemini', label: 'Gemini', detail: 'Starts new Gemini sessions in its yolo mode, which approves every tool call without asking.' },
] as const

function GeneralSection({ preferences, update, validateFolder, onChooseFolder, onShowWelcome, updates }: { preferences: SettingsPreferences; update: Update; validateFolder: (path: string) => Promise<boolean>; onChooseFolder: ((current: string) => Promise<string | null>) | undefined; onShowWelcome: () => void; updates: DesktopSettingsProps['updates'] }) {
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
          <div className="mx-folder-row">
            <div className={`mx-folder-field${missing ? ' is-invalid' : ''}`}>
              <span className="mono" style={{ fontSize: 12, color: 'var(--dim)' }}>cd</span>
              <input value={draft} placeholder="~" aria-label="Start folder" aria-invalid={missing} spellCheck={false} onChange={(event) => { setDraft(event.target.value) }} />
            </div>
            {onChooseFolder && (
              <button type="button" className="mx-text-button" onClick={() => { void onChooseFolder(draft.trim()).then((picked) => { if (picked) setDraft(picked) }) }}>Browse…</button>
            )}
          </div>
        </div>
      </Card>
      <Card
        title="Export folder"
        color="var(--ansi-green)"
        iconPath={<path d={FOLDER_PATH} />}
        foot="Code blocks and tables you save from a chat go here. An existing file is never replaced; the new one gets a number."
      >
        <div style={{ padding: '0 16px 14px' }}>
          <div className="mx-folder-row">
            <div className="mx-folder-field">
              <span className="mono" style={{ fontSize: 12, color: preferences.exportFolder === '' ? 'var(--dim)' : 'var(--fg)' }}>{preferences.exportFolder === '' ? 'Your Downloads folder' : preferences.exportFolder}</span>
            </div>
            {onChooseFolder && (
              <button type="button" className="mx-text-button" aria-label="Choose the export folder" onClick={() => { void onChooseFolder(preferences.exportFolder).then((picked) => { if (picked) update('exportFolder', picked) }) }}>Browse…</button>
            )}
            {preferences.exportFolder !== '' && <button type="button" className="mx-text-button" onClick={() => { update('exportFolder', '') }}>Use Downloads</button>}
          </div>
        </div>
      </Card>
      <Card title="Window" color="var(--ansi-blue)" iconPath={<><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M3 16h18" /></>}>
        <Row label="Status bar" detail="A line at the bottom with quick controls for theme, layout, text size and shortcuts.">
          <Switch aria-label="Status bar" checked={preferences.statusBar} onCheckedChange={(value) => { update('statusBar', value) }} />
        </Row>
      </Card>
      {updates && (
        <Card title="Updates" color="var(--ansi-green)" iconPath={<path d="M12 4v10m0 0l-4-4m4 4l4-4M5 19h14" />}>
          <Row label="Maitty updates" detail={updates.summary}>
            <UpdateControl updates={updates} />
          </Row>
          <Row label="Download updates automatically" detail="A new release is fetched in the background. Installing waits until you restart.">
            <Switch aria-label="Download updates automatically" checked={preferences.autoDownloadUpdates} onCheckedChange={(value) => { update('autoDownloadUpdates', value) }} />
          </Row>
          <Row label="Install updates when idle" detail="Never while an agent is working.">
            <Switch aria-label="Install updates when idle" checked={preferences.restartWhenIdle} onCheckedChange={(value) => { update('restartWhenIdle', value) }} />
          </Row>
        </Card>
      )}
      <Card title="Sessions" color="var(--ansi-mag)" iconPath={<><rect x="4" y="4" width="16" height="7" rx="2" /><rect x="4" y="13" width="16" height="7" rx="2" /><path d="M8 7.5h.01M8 16.5h.01" /></>}>
        <Row label="Reopen layout on launch" detail="Restore the panes and splits you had last time.">
          <Switch aria-label="Reopen layout on launch" checked={preferences.restoreLayout} onCheckedChange={(value) => { update('restoreLayout', value) }} />
        </Row>
        <Row label="Closing the window" detail="Sessions are owned by maittyd and keep running.">
          <span style={{ fontSize: 12, color: 'var(--dim)' }}>Always keeps running</span>
        </Row>
        <Row label="Welcome" detail="Replay the short intro: where sessions start and the keys to know.">
          <button type="button" className="mx-scard-button" onClick={onShowWelcome}>Show the welcome again</button>
        </Row>
      </Card>
      <Card
        title="Yolo mode"
        color="var(--danger)"
        iconPath={<path d="M12 3 5 6v6c0 4.4 3 7.6 7 9 4-1.4 7-4.6 7-9V6z" />}
        foot="Off for every agent by default. Only sessions started after you turn one on get it. Every session already running has to be closed and started again."
      >
        {YOLO_AGENTS.map(({ key, label, detail }) => (
          <Row key={key} label={label} detail={detail}>
            <Switch aria-label={`Yolo mode for ${label}`} checked={preferences[key]} onCheckedChange={(value) => {
              update(key, value)
              if (value) toast.message(`Yolo mode for ${label} is on for new sessions only`, { description: `Sessions already running keep how they started. Restart one, or start a new session, to get it.` })
            }} />
          </Row>
        ))}
      </Card>
      <Card title="Chat view" color="var(--accent)" iconPath={<path d="M5 5h14a1 1 0 0 1 1 1v9a1 1 0 0 1-1 1h-7l-4 3.5V16H5a1 1 0 0 1-1-1V6a1 1 0 0 1 1-1z" />}>
        <Row label="Default agent" detail="The agent a new session starts. A plain terminal is always one click away.">
          <SegmentedChoice label="Default agent" value={preferences.defaultAgent} options={AGENT_OPTIONS} onChange={(value) => { update('defaultAgent', value) }} />
        </Row>
        <Row label="New sessions open as" detail="Which view a new session opens in. The agent runs in a terminal either way, so you can switch between its chat and its terminal at any time.">
          <SegmentedChoice label="New sessions open as" value={preferences.newSessionView} options={VIEW_OPTIONS} onChange={(value) => { update('newSessionView', value) }} />
        </Row>
        <Row label="Full access in new chats" detail="Start new chats with the agent's skip-permissions mode: no sandbox and no approval prompts, so the agent can change files and run commands without asking. Each chat can still turn it off or on when it starts.">
          <Switch aria-label="Full access in new chats" checked={preferences.chatFullAccessDefault} onCheckedChange={(value) => { update('chatFullAccessDefault', value) }} />
        </Row>
        {CHAT_AGENTS.map((agent) => (
          <Row key={agent} label={`${agent === 'claude' ? 'Claude' : 'Codex'} sessions open in`} detail="Only a session with a chat opens in it; the switch on each pane changes one session.">
            <SegmentedChoice label={`${agent === 'claude' ? 'Claude' : 'Codex'} sessions open in`} value={preferences.chatViewDefault[agent]} options={CHAT_VIEW_OPTIONS} onChange={(value) => { update('chatViewDefault', { ...preferences.chatViewDefault, [agent]: value }) }} />
          </Row>
        ))}
      </Card>
    </>
  )
}

const CHAT_VIEW_OPTIONS = [['terminal', 'Terminal'], ['chat', 'Chat']] as const
const ICON_COLOR_OPTIONS = [['color', 'Color'], ['grayscale', 'Grayscale']] as const
/** The chip beside the update button, only where it says something the button does not. */
const UPDATE_CHIP: Partial<Record<UpdaterState['status'], string>> = { idle: 'Not checked', current: 'Up to date', available: 'Update available', failed: 'Failed' }

/** Where the update is: one button that says it, turning while it works and asking for the restart once it is ready. */
function UpdateControl({ updates }: { updates: NonNullable<DesktopSettingsProps['updates']> }) {
  const chip = UPDATE_CHIP[updates.status]
  const busy = updates.status === 'checking' || updates.status === 'downloading'
  const action = updateAction(updates)
  return (
    <>
      {chip !== undefined && <span className={`mx-upd-chip is-${updates.status}`}>{chip}</span>}
      <button type="button" className={`mx-text-button mx-upd-button${updates.status === 'ready' ? ' is-primary is-ready' : ''}`} disabled={busy} aria-busy={busy} onClick={action.run}>
        {busy && <span className="mx-upd-spin" aria-hidden="true" />}
        {action.label}
      </button>
    </>
  )
}

/** The one button that fits where the update is. */
function updateAction(updates: NonNullable<DesktopSettingsProps['updates']>): { label: string; run: () => void } {
  switch (updates.status) {
    case 'available': return { label: 'Download', run: updates.onDownload }
    case 'ready': return { label: 'Restart now', run: updates.onRestart }
    case 'failed': return { label: 'Retry', run: updates.onCheck }
    case 'checking': return { label: 'Checking…', run: updates.onCheck }
    case 'downloading': return { label: 'Downloading…', run: updates.onCheck }
    case 'idle':
    case 'current': return { label: 'Check now', run: updates.onCheck }
  }
}
const TOAST_POSITIONS = [['top-left', 'Top left'], ['top-center', 'Top center'], ['top-right', 'Top right'], ['bottom-left', 'Bottom left'], ['bottom-center', 'Bottom center'], ['bottom-right', 'Bottom right']] as const
const ORDER_OPTIONS: ReadonlyArray<readonly [SidebarOrder, string]> = [['creation', 'Creation'], ['priority', 'Priority']]
const GROUP_OPTIONS = [['status', 'Status'], ['project', 'Project'], ['machine', 'Machine']] as const
const DOCK_OPTIONS = [['panel', 'Side panel'], ['under', 'Under the chat']] as const
const SIDE_OPTIONS = [['left', 'Left'], ['right', 'Right']] as const
const ICON_OPTIONS = [['small', 'Small'], ['medium', 'Medium'], ['large', 'Large']] as const
const DENSITY_OPTIONS = [['compact', 'Compact'], ['comfortable', 'Comfortable'], ['spacious', 'Spacious']] as const
const UI_TEXT_OPTIONS = [[UI_TEXT_SIZE_SMALL, 'Small'], [UI_TEXT_SIZE_DEFAULT, 'Medium'], [UI_TEXT_SIZE_LARGE, 'Large']] as const

const FONT_SIZES = Array.from({ length: TERMINAL_FONT_SIZE_MAX - TERMINAL_FONT_SIZE_MIN + 1 }, (_, index) => {
  const size = TERMINAL_FONT_SIZE_MIN + index
  return [size, String(size)] as const
})
const WEIGHT_NAMES: Readonly<Record<TerminalFontWeight, string>> = { [WEIGHT_LIGHT]: 'Light', [WEIGHT_REGULAR]: 'Regular', [WEIGHT_MEDIUM]: 'Medium', [WEIGHT_SEMIBOLD]: 'Semibold', [WEIGHT_BOLD]: 'Bold', [WEIGHT_EXTRA_BOLD]: 'Extra bold' }
const weightOptions = (weights: readonly TerminalFontWeight[]) => weights.map((weight) => [weight, WEIGHT_NAMES[weight]] as const)
const NORMAL_WEIGHTS = weightOptions([WEIGHT_LIGHT, WEIGHT_REGULAR, WEIGHT_MEDIUM, WEIGHT_SEMIBOLD])
const BOLD_WEIGHTS = weightOptions([WEIGHT_SEMIBOLD, WEIGHT_BOLD, WEIGHT_EXTRA_BOLD])
const CURSORS: ReadonlyArray<readonly [CursorStyle, ReactNode]> = (['block', 'bar', 'underline'] as const).map((shape) => [shape, <><span className={`mx-cursor-glyph is-${shape}`} aria-hidden="true" />{shape === 'bar' ? 'Bar' : shape === 'block' ? 'Block' : 'Underline'}</>] as const)
const THOUSAND = 1000
const SCROLLBACKS: ReadonlyArray<readonly [ScrollbackLines, string]> = SCROLLBACK_CHOICES.map((lines) => [lines, `${String(lines / THOUSAND)}k`] as const)
const SCROLL_SPEEDS: ReadonlyArray<readonly [ScrollSpeed, string]> = SCROLL_SPEED_CHOICES.map((speed) => [speed, speed === 1 ? 'Normal' : `${String(speed)}×`] as const)

function caretStyle(cursor: CursorStyle): CSSProperties {
  return { width: cursor === 'bar' ? '2px' : '.6em', height: cursor === 'underline' ? '2px' : '1.25em', alignSelf: cursor === 'underline' ? 'flex-end' : 'center' }
}

function PromptPrefix() {
  return <><span style={{ color: 'var(--accent-text)', fontWeight: 600 }}>maitty</span><span style={{ color: 'var(--ansi-mag)' }}> (main)</span><span style={{ color: 'var(--dim)' }}> ❯ </span></>
}

/** The font picker: Geist Mono (saved as empty) and the monospace families installed here, each set in its own face. */
function FontFamilyField({ value, onCommit }: { value: string; onCommit: (family: string) => void }) {
  const [installed, setInstalled] = useState<string[]>([])
  // Measured after the first paint so opening the page is not held up by it.
  useEffect(() => {
    const timer = window.setTimeout(() => { setInstalled(installedMonospaceFamilies()) }, 0)
    return () => { window.clearTimeout(timer) }
  }, [])
  const item = (name: string, sub: string): PickerItem => ({
    value: name,
    label: name,
    sub,
    tile: <span className="mx-font-tile" style={{ fontFamily: fontStackFor(name) }} aria-hidden="true">Aa</span>,
    labelStyle: { fontFamily: fontStackFor(name) },
  })
  const names = installed.filter((name) => name !== BUNDLED_FONT_NAME)
  // A family saved earlier that this computer no longer reports still shows as the current choice.
  const saved = value !== '' && value !== BUNDLED_FONT_NAME && !names.includes(value) ? [item(value, 'Saved choice')] : []
  const groups: PickerGroup[] = [
    { items: [item(BUNDLED_FONT_NAME, 'Default, bundled with Maitty')] },
    ...(saved.length + names.length > 0 ? [{ label: 'Installed here', items: [...saved, ...names.map((name) => item(name, 'Monospace'))] }] : []),
  ]
  return <PickerSelect label="Terminal font family" value={value === '' ? BUNDLED_FONT_NAME : value} groups={groups} onPick={(next) => { onCommit(next === BUNDLED_FONT_NAME ? '' : next) }} width={240} />
}

function TerminalSection({ preferences, update }: { preferences: SettingsPreferences; update: Update }) {
  const family = preferences.terminalFontFamily === '' ? BUNDLED_FONT_NAME : preferences.terminalFontFamily
  return (
    <>
      <Card title="Text" color="var(--ansi-blue)" iconPath={<path d="M5 6h14M12 6v12" />} meta={<span className="mono" style={{ fontSize: 12, color: 'var(--fg2)' }}>{family} · {preferences.terminalFontSize}px</span>}
        foot="Leave the font empty for Geist Mono, or name any monospace font installed on this computer; if it is missing, Geist Mono is used. Regular at 400 can look thin on Linux: try Medium.">
        <div style={{ padding: '0 16px 14px', display: 'flex', flexDirection: 'column', gap: 12 }}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Size</span>
            <SegmentedChoice label="Terminal text size" value={preferences.terminalFontSize} options={FONT_SIZES} onChange={(value) => { update('terminalFontSize', value) }} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Font</span>
            <FontFamilyField value={preferences.terminalFontFamily} onCommit={(next) => { update('terminalFontFamily', next) }} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Weight</span>
            <SegmentedChoice label="Terminal text weight" value={preferences.terminalFontWeight} options={NORMAL_WEIGHTS} onChange={(value) => { update('terminalFontWeight', value) }} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Bold</span>
            <SegmentedChoice label="Terminal bold weight" value={preferences.terminalFontWeightBold} options={BOLD_WEIGHTS} onChange={(value) => { update('terminalFontWeightBold', value) }} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Cursor</span>
            <SegmentedChoice label="Cursor" value={preferences.cursorStyle} options={CURSORS} onChange={(value) => { update('cursorStyle', value) }} />
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 14 }}>
            <span className="mx-field-label">Blink</span>
            <Switch aria-label="Blinking cursor" checked={preferences.cursorBlink} onCheckedChange={(value) => { update('cursorBlink', value) }} />
          </div>
          <div className="mx-preview" style={{ fontSize: preferences.terminalFontSize, fontFamily: fontStackFor(preferences.terminalFontFamily), fontWeight: preferences.terminalFontWeight }} aria-label="Terminal preview" role="img">
            <div><PromptPrefix /><span style={{ fontWeight: preferences.terminalFontWeightBold }}>cargo</span> test <span style={{ color: 'var(--ansi-blue)' }}>-p</span> maitty-core</div>
            <div style={{ color: 'var(--fg2)' }}>test pane_terminal::replay_then_stream ... <span style={{ color: 'var(--ok)' }}>ok</span></div>
            <div style={{ display: 'flex', alignItems: 'center' }}><PromptPrefix /><span className={`mx-caret${preferences.cursorBlink ? ' is-blinking' : ''}`} style={caretStyle(preferences.cursorStyle)} /></div>
          </div>
        </div>
      </Card>
      <Card title="Scrollback" color="var(--ansi-green)" iconPath={<path d="m5 8 4 4-4 4M12 17h7" />}>
        <Row label="Lines kept per pane" detail="Older output is dropped from the GUI. maittyd keeps the full history.">
          <SegmentedChoice label="Lines kept per pane" value={preferences.scrollback} options={SCROLLBACKS} onChange={(value) => { update('scrollback', value) }} />
        </Row>
        <Row label="Scroll speed" detail="Lines per wheel notch. Normal is xterm's own pace, which feels slow at this line height.">
          <SegmentedChoice label="Scroll speed" value={preferences.scrollSpeed} options={SCROLL_SPEEDS} onChange={(value) => { update('scrollSpeed', value) }} />
        </Row>
        <Row label="Screen reader support" detail="Mirrors each terminal's text into the page so a screen reader can read it. It makes every pane much heavier to draw, so leave it off unless you use one.">
          <Switch aria-label="Screen reader support" checked={preferences.terminalScreenReader} onCheckedChange={(value) => { update('terminalScreenReader', value) }} />
        </Row>
      </Card>
    </>
  )
}


const MODE_OPTIONS = [['system', 'System'], ['dark', 'Dark'], ['light', 'Light']] as const

/** A window style as the picker shows it: a small window in the style's own shape and the theme in use, then its name and line. */
function windowStyleItem(style: WindowStyleEntry): PickerItem {
  return {
    value: style.id,
    label: style.name,
    sub: style.blurb,
    tile: <span className="mx-style-tile" data-shape={style.id} aria-hidden="true"><i /></span>,
    labelStyle: { fontFamily: style.font },
  }
}

function AppearanceSection({ preferences, update, systemReducedMotion, systemDark, onPreviewTheme }: { preferences: SettingsPreferences; update: Update; systemReducedMotion: boolean; systemDark: boolean; onPreviewTheme: (name: string | null) => void }) {
  const reduced = preferences.reduceMotion || systemReducedMotion
  const darkInUse = preferences.themeMode === 'dark' || (preferences.themeMode === 'system' && systemDark)
  const modeNote = preferences.themeMode === 'system' ? `Follows your OS. Right now: ${systemDark ? 'dark' : 'light'}.` : `Always uses your ${preferences.themeMode} theme.`
  return (
    <>
      <Card title="Look" color="var(--accent)" iconPath={<><rect x="4" y="5" width="16" height="14" rx="3" /><path d="M4 10h16" /></>} meta={<span className="mx-scard-meta">{THEMES.length} themes installed</span>}
        foot="Amber and red keep their meaning in every theme."
      >
        <Row label="Window style" detail="Same features, different finish.">
          <PickerSelect label="Window style" value={preferences.style} groups={[{ items: WINDOW_STYLES.map((style) => windowStyleItem(style)) }]} onPick={(style) => { if (isWindowStyle(style)) update('style', style) }} />
        </Row>
        <Row label="Appearance" detail={modeNote}>
          <SegmentedChoice label="Appearance" value={preferences.themeMode} options={MODE_OPTIONS} onChange={(value) => { update('themeMode', value) }} />
        </Row>
        <Row label="Dark theme" detail="Used when the window is dark.">
          <ThemePicker kind="dark" value={preferences.darkTheme} inUse={darkInUse} dimmed={!darkInUse && preferences.themeMode !== 'system'} onPick={(name) => { update('darkTheme', name) }} onPreview={onPreviewTheme} />
        </Row>
        <Row label="Light theme" detail="Used when the window is light.">
          <ThemePicker kind="light" value={preferences.lightTheme} inUse={!darkInUse} dimmed={darkInUse && preferences.themeMode !== 'system'} onPick={(name) => { update('lightTheme', name) }} onPreview={onPreviewTheme} />
        </Row>
        <Row label="Working spinner" detail="Shown on tabs, pane headers and sidebar rows while something runs. The TUI ones use its exact frames.">
          <SpinnerPicker value={preferences.spinner} reduced={reduced} onPick={(spinner) => { update('spinner', spinner) }} />
        </Row>
      </Card>
      <Card title="Surfaces" color="var(--ansi-mag)" iconPath={<><circle cx="12" cy="12" r="8" /><path d="M12 4a8 8 0 0 0 0 16" /></>}>
        <Row label="Icons" detail="Agent and folder icons in their own colours, or neutral gray tiles.">
          <SegmentedChoice label="Icons" value={preferences.iconColor} options={ICON_COLOR_OPTIONS} onChange={(value) => { update('iconColor', value) }} />
        </Row>
        <Row label="Background glow" detail="Two faint tints of the theme behind the cards. Off is a flat ground; a web view without WebGL always uses the flat one.">
          <Switch aria-label="Background glow" checked={preferences.backgroundGlow} onCheckedChange={(value) => { update('backgroundGlow', value) }} />
        </Row>
        <Row label="See-through terminal" detail="Lets the card show behind a pane's text when WebGL draws it. Programs that paint their own backgrounds, like Claude Code under its prompt, then show as boxes.">
          <Switch aria-label="See-through terminal" checked={preferences.terminalSeeThrough} onCheckedChange={(value) => { update('terminalSeeThrough', value) }} />
        </Row>
      </Card>
      <Card title="Layout" color="var(--ansi-blue)" iconPath={<><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M3 14h18M9 4v10" /></>}>
        <Row label="Group sessions by" detail="Status lists what needs you first, then what is working, finished and idle. Project and Machine put each under its own heading.">
          <SegmentedChoice label="Group sessions by" value={preferences.sidebarGroup} options={GROUP_OPTIONS} onChange={(value) => { update('sidebarGroup', value) }} />
        </Row>
        <Row label="Terminals open in" detail="An agent's own terminals: a tab of the side panel, or a drawer pinned under its chat.">
          <SegmentedChoice label="Terminals open in" value={preferences.terminalDock} options={DOCK_OPTIONS} onChange={(value) => { update('terminalDock', value) }} />
        </Row>
      </Card>
      <Card title="Sidebar" color="var(--accent)" iconPath={<><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M9 4v16" /></>}
        foot="Session order applies when the sidebar is grouped by machine."
      >
        <Row label="Session order" detail="Creation keeps every row where it was put. Priority moves a session that needs you to the top.">
          <SegmentedChoice label="Session order" value={preferences.sidebarOrder} options={ORDER_OPTIONS} onChange={(value) => { update('sidebarOrder', value) }} />
        </Row>
        <Row label="Sidebar position" detail="Which side of the window the sidebar sits on. Inspect opens from the other side.">
          <SegmentedChoice label="Sidebar position" value={preferences.sidebarSide} options={SIDE_OPTIONS} onChange={(value) => { update('sidebarSide', value) }} />
        </Row>
      </Card>
      <Card title="Interface size" color="var(--ansi-green)" iconPath={<path d="M4 7h16M4 12h10M4 17h13" />}>
        <Row label="Density" detail="How much room the sidebar rows, pane headers and gaps get.">
          <SegmentedChoice label="Density" value={preferences.density} options={DENSITY_OPTIONS} onChange={(value) => { update('density', value) }} />
        </Row>
        <Row label="Sidebar icons" detail="The project tiles, session icons and the collapsed rail.">
          <SegmentedChoice label="Sidebar icon size" value={preferences.iconSize} options={ICON_OPTIONS} onChange={(value) => { update('iconSize', value) }} />
        </Row>
        <Row label="Sidebar text" detail="Folder names and session rows.">
          <SegmentedChoice label="Sidebar text size" value={preferences.sidebarTextSize} options={UI_TEXT_OPTIONS} onChange={(value) => { update('sidebarTextSize', value) }} />
        </Row>
        <Row label="Pane title text" detail="The names on each pane's header.">
          <SegmentedChoice label="Pane title text size" value={preferences.tabTextSize} options={UI_TEXT_OPTIONS} onChange={(value) => { update('tabTextSize', value) }} />
        </Row>
        <Row label="Sidebar width" detail="Drag the sidebar's edge to change it, or focus the edge and use the arrow keys.">
          <span className="mono" style={{ fontSize: 12, color: 'var(--fg2)' }}>{preferences.sidebarWidth}px</span>
          <button type="button" className="mx-text-button" disabled={preferences.sidebarWidth === SIDEBAR_WIDTH_DEFAULT} onClick={() => { update('sidebarWidth', SIDEBAR_WIDTH_DEFAULT) }}>Reset</button>
        </Row>
      </Card>
      <Card title="Motion" color="var(--ansi-blue)" iconPath={<><circle cx="12" cy="12" r="8" /><path d="M12 4a8 8 0 0 0 0 16" /></>}>
        <Row label="Reduce motion" detail="Turns off pulses, slide-ins and size animations, and holds spinners still. Your system setting is always respected.">
          <Switch aria-label="Reduce motion" checked={preferences.reduceMotion} onCheckedChange={(value) => { update('reduceMotion', value) }} />
        </Row>
      </Card>
    </>
  )
}

function HotkeysSection({ platform }: { platform: Platform }) {
  return (
    <Card title="Hotkeys" color="var(--ansi-blue)" iconPath={<><rect x="3" y="7" width="18" height="10" rx="2" /><path d="M7 11h.01M11 11h.01M15 11h.01M8 14h8" /></>} meta={<span className="mx-scard-meta">{platform === 'mac' ? '⌘ here, Ctrl+Shift on Windows and Linux' : 'Ctrl+Shift here, ⌘ on macOS'}</span>} foot="In a focused pane, every other key goes straight to the shell.">
      <div style={{ display: 'flex', flexDirection: 'column' }}>
        {hotkeyRows(platform).map((row) => (
          <div key={row.action} className="mx-hotkey-row">
            <span>{row.action}</span>
            <span className="mx-keys">
              {row.chords.map((chord, index) => (
                <Fragment key={chord.join('+')}>
                  {index > 0 && <span className="mx-or">or</span>}
                  <Keycaps chord={chord} className="mx-chord" labelled />
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
function UsageSection({ usage }: { usage: UsageInfo }) {
  return <UsagePage {...usage} />
}

const AGENT_OPTIONS = [['claude', 'Claude'], ['codex', 'Codex']] as const
const VIEW_OPTIONS = [['chat', 'Chat'], ['terminal', 'Terminal']] as const

function NotificationsSection({ preferences, update, onTestNotice }: { preferences: SettingsPreferences; update: Update; onTestNotice: (() => void) | undefined }) {
  return (
    <Card
      title="System notifications"
      color="var(--warn)"
      iconPath={<path d="M6 8a6 6 0 0 1 12 0c0 7 3 9 3 9H3s3-2 3-9M10.3 21a1.9 1.9 0 0 0 3.4 0" />}
      foot="Clicking a notification brings the window forward and shows that session. A notification names the session and where it is; it never shows what the agent asked, so nothing private appears on a locked screen. The system asks once whether Maitty may send them."
    >
      <Row label="Needs you" detail="An agent stopped for an answer or an approval.">
        <Switch aria-label="Notify when a session needs you" checked={preferences.notifyNeedsYou} onCheckedChange={(value) => { update('notifyNeedsYou', value) }} />
      </Row>
      <Row label="Done" detail="An agent finished its turn.">
        <Switch aria-label="Notify when a session finishes" checked={preferences.notifyFinished} onCheckedChange={(value) => { update('notifyFinished', value) }} />
      </Row>
      <Row label="Error" detail="A session's program exited with an error.">
        <Switch aria-label="Notify when a session errors" checked={preferences.notifyError} onCheckedChange={(value) => { update('notifyError', value) }} />
      </Row>
      <Row label="Position" detail="Where notification cards appear in the window. Done is green, needs you amber and an error red.">
        <div className="mx-pos-grid" role="group" aria-label="Notification position">
          {TOAST_POSITIONS.map(([id, label]) => (
            <button key={id} type="button" className={`mx-pos-cell${preferences.toastPosition === id ? ' is-on' : ''}`} aria-label={label} aria-pressed={preferences.toastPosition === id} title={label} onClick={() => { update('toastPosition', id) }} />
          ))}
        </div>
        {onTestNotice && <button type="button" className="mx-text-button" onClick={onTestNotice}>Show a test</button>}
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


function MachinesSection({ machines }: { machines: MachinesInfo }) {
  return (
    <>
      <MachineList machines={machines} />
      {machines.known && <AddMachineCard add={machines.add} />}
    </>
  )
}

type HostsState = { status: 'loading' } | { status: 'ready'; hosts: readonly HostChoice[] } | { status: 'failed'; message: string }

function AddMachineCard({ add }: { add: MachinesInfo['add'] }) {
  const [hosts, setHosts] = useState<HostsState>({ status: 'loading' })
  const [text, setText] = useState('')
  const [checking, setChecking] = useState<string | null>(null)
  const [naming, setNaming] = useState<string | null>(null)
  const [name, setName] = useState('')
  const [note, setNote] = useState<{ kind: 'ok' | 'error'; text: string } | null>(null)
  const { loadHosts } = add
  useEffect(() => {
    let gone = false
    loadHosts().then(
      (list) => { if (!gone) setHosts({ status: 'ready', hosts: list }) },
      (error: unknown) => { if (!gone) setHosts({ status: 'failed', message: errorMessage(error, 'Could not read ~/.ssh/config') }) },
    )
    return () => { gone = true }
  }, [loadHosts])

  const submit = (target: string, chosenName?: string) => {
    setChecking(target)
    setNote(null)
    add.onAdd(target, chosenName).then(
      (result) => {
        setChecking(null)
        switch (result.outcome) {
          case 'needs-name':
            setNaming(target)
            setName('')
            return
          case 'cancelled':
            setNote({ kind: 'ok', text: 'Cancelled. Nothing was added.' })
            return
          case 'added':
            setNote({ kind: 'ok', text: `Added ${result.name}.${result.version ? ` ${result.version} answered there.` : ''} Connecting.` })
            break
          case 'login':
            setNote({ kind: 'ok', text: `Added ${result.name}. ssh needs you to log in: use the session named “ssh ${result.name}”.` })
            break
          case 'login-manual':
            setNote({ kind: 'error', text: `Added ${result.name}. Run “ssh ${result.target}” once in a terminal to log in, then retry (${result.why}).` })
            break
        }
        setText('')
        setNaming(null)
        setHosts((current) => current.status === 'ready' ? { status: 'ready', hosts: current.hosts.filter((host) => host.alias !== target) } : current)
      },
      (error: unknown) => {
        setChecking(null)
        setNote({ kind: 'error', text: errorMessage(error, 'Could not add the machine') })
      },
    )
  }

  const typed = text.trim()
  const shown = hosts.status === 'ready' ? hosts.hosts.filter((host) => `${host.alias} ${host.detail}`.toLowerCase().includes(typed.toLowerCase())) : []
  const icon = <><circle cx="12" cy="12" r="9" /><path d="M12 8v8M8 12h8" /></>
  return (
    <Card title="Add a machine" color="var(--accent)" iconPath={icon}>
      {add.windows && <p className="mx-scard-note">Windows: use ssh keys or an agent. Password prompts work in the session but are not remembered.</p>}
      {checking !== null
        ? (
          <div className="mx-scard-row" data-owns-escape onKeyDown={(event) => { if (event.key === 'Escape') { event.stopPropagation(); add.onCancel() } }}>
            <div className="mx-scard-text"><span role="status">Checking {checking}…</span><span>Esc ends ssh and adds nothing.</span></div>
            <button type="button" className="mx-scard-button" onClick={add.onCancel}>Cancel</button>
          </div>
        )
        : naming !== null
          ? (
            <form className="mx-scard-row" onSubmit={(event) => { event.preventDefault(); if (name.trim() !== '') submit(naming, name.trim()) }}>
              <div className="mx-scard-text"><span>Name for {naming}</span><span>A short name for the sidebar, such as build.</span></div>
              <input className="mx-scard-input" aria-label="Machine name" value={name} autoFocus onChange={(event) => { setName(event.target.value) }} />
              <button type="submit" className="mx-scard-button" disabled={name.trim() === ''}>Add</button>
              <button type="button" className="mx-scard-button" onClick={() => { setNaming(null) }}>Back</button>
            </form>
          )
          : (
            <form className="mx-scard-row" onSubmit={(event) => { event.preventDefault(); if (typed !== '') submit(typed) }}>
              <div className="mx-scard-text"><span>Host</span><span>Pick one below, or type user@host.</span></div>
              <input className="mx-scard-input" aria-label="Host or user@host" value={text} placeholder="user@host" onChange={(event) => { setText(event.target.value) }} />
              <button type="submit" className="mx-scard-button" disabled={typed === ''}>Add</button>
            </form>
          )}
      {note && <p className="mx-scard-note" role={note.kind === 'error' ? 'alert' : 'status'}>{note.text}</p>}
      {checking === null && naming === null && (
        hosts.status === 'loading'
          ? <p className="mx-scard-note" role="status">Loading hosts from ~/.ssh/config…</p>
          : hosts.status === 'failed'
            ? <p className="mx-scard-note" role="alert">{hosts.message}</p>
            : hosts.hosts.length === 0
              ? <p className="mx-scard-note">No hosts in ~/.ssh/config. Type user@host above.</p>
              : (
                <ul className="mx-host-list" aria-label="Hosts in ~/.ssh/config">
                  {shown.map((host) => (
                    <li key={host.alias}>
                      <button type="button" className="mx-host-row" onClick={() => { submit(host.alias) }}>
                        <span>{host.alias}</span><span className="mx-host-detail">{host.detail}</span>
                      </button>
                    </li>
                  ))}
                  {shown.length === 0 && <li className="mx-scard-note">No host matches “{typed}”. Add presses Enter for it as typed.</li>}
                </ul>
              )
      )}
    </Card>
  )
}

function MachineList({ machines }: { machines: MachinesInfo }) {
  const [confirming, setConfirming] = useState<string | null>(null)
  const [confirmingUpdate, setConfirmingUpdate] = useState<string | null>(null)
  const [activity, setActivity] = useState<MachineActivity | null>(null)
  const [checking, setChecking] = useState<string | null>(null)
  const askUpdate = (name: string, action: UpdateAction) => {
    setActivity(null)
    if (action === 'install') {
      setConfirmingUpdate(name)
      return
    }
    setChecking(name)
    void machines.onCheckActivity(name)
      .catch(() => null)
      .then((found) => { setActivity(found); setConfirmingUpdate(name) })
      .finally(() => { setChecking(null) })
  }
  const [updating, setUpdating] = useState<string | null>(null)
  const runUpdate = (name: string, action: UpdateAction, whenIdle: boolean) => {
    setConfirmingUpdate(null)
    setUpdating(name)
    void machines.onUpdate(name, action, whenIdle).finally(() => { setUpdating(null) })
  }
  const icon = <><rect x="4" y="4" width="16" height="7" rx="2" /><rect x="4" y="13" width="16" height="7" rx="2" /><path d="M8 7.5h.01M8 16.5h.01" /></>
  if (!machines.known) {
    return <Card title="Machines" color="var(--ansi-green)" iconPath={icon}><p className="mx-scard-note" role="status">Loading machines…</p></Card>
  }
  if (machines.list.length === 0) {
    return (
      <Card title="Machines" color="var(--ansi-green)" iconPath={icon}>
        <p className="mx-scard-note">No other machines. List them as [[machines]] in config.local.toml and restart the app.</p>
      </Card>
    )
  }
  return (
    <Card title="Machines" color="var(--ansi-green)" iconPath={icon} meta={<span className="mx-scard-meta">{machines.list.length}</span>}>
      {machines.list.map((head) => {
        const name = head.label
        const chip = head.chip
        const asking = confirming === name
        const askingUpdate = confirmingUpdate === name
        const busyUpdating = updating === name
        const action = head.updateAction
        const words = action === null ? null : UPDATE_WORDS[action]
        return (
          <Fragment key={head.key}>
          <div className="mx-scard-row mx-machine-row">
            <div className="mx-scard-text">
              <span>
                {name}
                {chip && <span className={`mx-machine-chip is-${chip.tone}`}> <span aria-hidden="true">{chip.glyph}</span> {chip.word}</span>}
              </span>
              {head.down && <span><DisconnectedSince at={head.downSince} /></span>}
              {words !== null && <span className="mx-machine-need">{words.need(name)}</span>}
              {words === null && head.why !== null && head.chip?.tone === 'bad' && <span className="mx-machine-fix">{head.why}</span>}
              {head.updateCommand !== null && <span className="mx-machine-fix">Or in a terminal: <span className="mono">{head.updateCommand}</span></span>}
              {asking && <span role="alert">Removes {name} from this list. Its sessions keep running on {name}.</span>}
              {askingUpdate && <span role="alert">{action === 'install' ? installNote(name) : updateWarning(name, activity)}</span>}
              {busyUpdating && words !== null && <span role="status">{words.running(name)}</span>}
            </div>
            {askingUpdate && action === 'install'
              ? (
                <>
                  <button type="button" className="mx-scard-button is-primary" onClick={() => { runUpdate(name, action, false) }}>Install on {name} now</button>
                  <button type="button" className="mx-scard-button" onClick={() => { setConfirmingUpdate(null) }}>Cancel</button>
                </>
              )
              : askingUpdate && action !== null
              ? (
                <>
                  <button type="button" className="mx-scard-button is-danger" onClick={() => { runUpdate(name, action, false) }}>{updateVerb(action, activity)} {name} now</button>
                  <button type="button" className="mx-scard-button" onClick={() => { runUpdate(name, action, true) }}>{updateVerb(action, activity)} {name} when idle</button>
                  <button type="button" className="mx-scard-button" onClick={() => { setConfirmingUpdate(null) }}>Cancel</button>
                </>
              )
              : asking
              ? (
                <>
                  <button type="button" className="mx-scard-button is-danger" onClick={() => { setConfirming(null); machines.onRemove(name) }}>Remove {name}</button>
                  <button type="button" className="mx-scard-button" onClick={() => { setConfirming(null) }}>Keep</button>
                </>
              )
              : (
                <>
                  {action !== null && words !== null && <button type="button" className="mx-scard-button is-primary" disabled={updating !== null || checking !== null} aria-label={`${words.verb} Maitty on ${name}`} onClick={() => { askUpdate(name, action) }}>{checking === name ? 'Checking…' : words.verb}</button>}
                  {head.updateCommand !== null && <button type="button" className="mx-scard-button" aria-label={`Copy the update command for ${name}`} onClick={() => { if (head.updateCommand !== null) machines.onCopyCommand(head.updateCommand) }}>Copy command</button>}
                  <button type="button" className="mx-scard-button" disabled={chip?.word === 'connected'} aria-label={`Retry ${name}`} onClick={() => { machines.onRetry(name) }}>Retry</button>
                  <button type="button" className="mx-scard-button" aria-label={`Remove ${name}`} onClick={() => { setConfirming(name) }}>Remove</button>
                </>
              )}
          </div>
          <Row label={`Copy to your clipboard`} detail={`Let ${name} put text on your clipboard.`}>
            <Switch aria-label={`Allow ${name} to copy to your clipboard`} checked={head.grants.clipboard} onCheckedChange={(on) => { machines.onGrant(name, 'clipboard', on) }} />
          </Row>
          <Row label="Notifications" detail={`Let ${name} raise notices, and tell you when its sessions need you or finish.`}>
            <Switch aria-label={`Allow ${name} to notify you`} checked={head.grants.notices} onCheckedChange={(on) => { machines.onGrant(name, 'notices', on) }} />
          </Row>
          </Fragment>
        )
      })}
    </Card>
  )
}
