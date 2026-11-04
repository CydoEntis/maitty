import { useEffect, useRef, type ReactNode } from 'react'
import { attachSpinner, currentGlyph, isWideSpinner, spinnerFrames } from '@/lib/spinner-clock'
import { SHELL_PROCESS, type SessionStatus } from '../model/session'
import { CLAUDE_MARK, CODEX_MARK, DEEPSEEK_MARK, GEMINI_MARK, OPENCODE_MARK } from './agentMarks'
import { folderStyle } from './folderStyle'
import { FOLDER_PATH } from './paths'
import { useReducedMotion } from './motion'

/** The working spinner. The shared clock writes each frame into the element itself, so a tick never re-renders React. */
export function Spinner({ className = 'mx-spin', frozen = false }: { className?: string; frozen?: boolean }) {
  const reduced = useReducedMotion()
  const still = reduced || frozen
  const element = useRef<HTMLSpanElement>(null)
  useEffect(() => {
    if (still || !element.current) return
    return attachSpinner(element.current)
  }, [still])
  const classes = [className, isWideSpinner() && 'is-wide'].filter(Boolean).join(' ')
  return <span ref={element} className={classes} aria-hidden="true">{still ? spinnerFrames()[0] : currentGlyph()}</span>
}

type SvgProps = { size: number; strokeWidth?: number; children: ReactNode; className?: string; fill?: string; stroke?: string }

function Svg({ size, strokeWidth = 2, children, className, fill = 'none', stroke = 'currentColor' }: SvgProps) {
  return (
    <svg className={className} width={size} height={size} viewBox="0 0 24 24" fill={fill} stroke={stroke} strokeWidth={strokeWidth} strokeLinecap="round" strokeLinejoin="round" aria-hidden="true" focusable="false">
      {children}
    </svg>
  )
}

type IconProps = { size?: number; strokeWidth?: number; stroke?: string }

const ICON_DRAWINGS = {
  collapse: () => <Svg size={15} strokeWidth={1.8}><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M9 4v16M15 10l-2 2 2 2" /></Svg>,
  expand: () => <Svg size={16} strokeWidth={1.8}><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M9 4v16M13 10l2 2-2 2" /></Svg>,
  plus: ({ size = 15 }: IconProps) => <Svg size={size}><path d="M12 5v14M5 12h14" /></Svg>,
  arrowUp: ({ size = 14, strokeWidth = 2 }: IconProps) => <Svg size={size} strokeWidth={strokeWidth}><path d="M12 19V5M6 11l6-6 6 6" /></Svg>,
  arrowDown: ({ size = 14, strokeWidth = 2 }: IconProps) => <Svg size={size} strokeWidth={strokeWidth}><path d="M12 5v14M6 13l6 6 6-6" /></Svg>,
  close: ({ size = 13, strokeWidth = 2 }: IconProps) => <Svg size={size} strokeWidth={strokeWidth}><path d="M6 6l12 12M18 6 6 18" /></Svg>,
  chevron: () => <Svg className="mx-chevron" size={12} strokeWidth={2.25} stroke="var(--dim)"><path d="m9 6 6 6-6 6" /></Svg>,
  folder: () => (
    <svg width="13" height="13" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinejoin="round" aria-hidden="true" focusable="false"><path d={FOLDER_PATH} /></svg>
  ),
  gear: () => (
    <Svg size={13} strokeWidth={1.75}><circle cx="12" cy="12" r="3" /><path d="M19.4 15a1.7 1.7 0 0 0 .3 1.8l.1.1a2 2 0 1 1-2.8 2.8l-.1-.1a1.7 1.7 0 0 0-2.8 1.2V21a2 2 0 1 1-4 0v-.1a1.7 1.7 0 0 0-2.9-1.2l-.1.1a2 2 0 1 1-2.8-2.8l.1-.1a1.7 1.7 0 0 0-1.2-2.8H3a2 2 0 1 1 0-4h.1a1.7 1.7 0 0 0 1.2-2.9l-.1-.1a2 2 0 1 1 2.8-2.8l.1.1a1.7 1.7 0 0 0 2.8-1.2V3a2 2 0 1 1 4 0v.1a1.7 1.7 0 0 0 2.9 1.2l.1-.1a2 2 0 1 1 2.8 2.8l-.1.1a1.7 1.7 0 0 0 1.2 2.8H21a2 2 0 1 1 0 4h-.1a1.7 1.7 0 0 0-1.5 1.1z" /></Svg>
  ),
  splitRight: () => <Svg size={16} strokeWidth={1.6}><rect x="3" y="4.5" width="18" height="15" rx="2.5" /><path d="M12 4.5v15" /></Svg>,
  splitDown: () => <Svg size={16} strokeWidth={1.6}><rect x="3" y="4.5" width="18" height="15" rx="2.5" /><path d="M3 12h18" /></Svg>,
  maximize: () => <Svg size={15} strokeWidth={1.8}><path d="M4 9V4h5M20 9V4h-5M4 15v5h5M20 15v5h-5" /></Svg>,
  restore: () => <Svg size={15} strokeWidth={1.8}><path d="M9 4v5H4M15 4v5h5M9 20v-5H4M15 20v-5h5" /></Svg>,
  branch: ({ size = 11, strokeWidth = 2.2 }: IconProps) => (
    <Svg size={size} strokeWidth={strokeWidth} stroke="var(--ansi-mag)"><circle cx="6" cy="6" r="2.5" /><circle cx="6" cy="18" r="2.5" /><circle cx="18" cy="8" r="2.5" /><path d="M6 8.5v7M18 10.5c0 4-6 3-10 5.5" /></Svg>
  ),
  broadcast: () => <Svg size={15} strokeWidth={1.8}><circle cx="12" cy="12" r="2" /><path d="M16.2 7.8a6 6 0 0 1 0 8.4M7.8 16.2a6 6 0 0 1 0-8.4M19 5a10 10 0 0 1 0 14M5 19A10 10 0 0 1 5 5" /></Svg>,
  inbox: ({ size = 16 }: IconProps) => <Svg size={size} strokeWidth={1.8}><path d="M22 12h-6l-2 3h-4l-2-3H2" /><path d="M5.45 5.11 2 12v6a2 2 0 0 0 2 2h16a2 2 0 0 0 2-2v-6l-3.45-6.89A2 2 0 0 0 16.76 4H7.24a2 2 0 0 0-1.79 1.11z" /></Svg>,
  repo: ({ size = 16 }: IconProps) => <Svg size={size} strokeWidth={1.8}><path d="M6 3v12" /><circle cx="18" cy="6" r="3" /><circle cx="6" cy="18" r="3" /><path d="M18 9a9 9 0 0 1-9 9" /></Svg>,
  pin: ({ size = 13 }: IconProps) => <Svg size={size} strokeWidth={1.8}><path d="M12 17v5" /><path d="M9 10.76a2 2 0 0 1-1.11 1.79l-1.78.9A2 2 0 0 0 5 15.24V16a1 1 0 0 0 1 1h12a1 1 0 0 0 1-1v-.76a2 2 0 0 0-1.11-1.79l-1.78-.9A2 2 0 0 1 15 10.76V7a1 1 0 0 1 1-1 2 2 0 0 0 0-4H8a2 2 0 0 0 0 4 1 1 0 0 1 1 1z" /></Svg>,
  pencil: ({ size = 13 }: IconProps) => <Svg size={size} strokeWidth={1.8}><path d="M12 20h9" /><path d="M16.5 3.5a2.1 2.1 0 0 1 3 3L7 19l-4 1 1-4Z" /></Svg>,
  download: ({ size = 15 }: IconProps) => <Svg size={size} strokeWidth={1.9}><path d="M12 4v11M7.5 10.5 12 15l4.5-4.5M5 19h14" /></Svg>,
  palette: ({ size = 12 }: IconProps) => <Svg size={size} strokeWidth={1.8}><path d="M12 3a9 9 0 1 0 0 18c1.3 0 2-.8 2-1.8 0-.5-.2-.9-.5-1.3-.3-.4-.5-.8-.5-1.3 0-1 .8-1.8 1.8-1.8H17a4 4 0 0 0 4-4C21 6.5 17 3 12 3Z" /><circle cx="7.5" cy="11" r="1" /><circle cx="10" cy="7" r="1" /><circle cx="15" cy="7.5" r="1" /></Svg>,
  layout: ({ size = 12 }: IconProps) => <Svg size={size} strokeWidth={1.8}><rect x="3" y="4" width="18" height="16" rx="2.5" /><path d="M3 10h18M10 10v10" /></Svg>,
  prompt: ({ size = 18 }: IconProps) => <Svg size={size}><path d="m5 8 4 4-4 4M12 17h7" /></Svg>,
  check: ({ size = 9, stroke = 'var(--ok)', strokeWidth = 3.5 }: IconProps) => <Svg size={size} strokeWidth={strokeWidth} stroke={stroke}><path d="M5 12.5 10 17 19 7" /></Svg>,
  bang: () => <Svg size={9} strokeWidth={3.5} stroke="var(--danger)"><path d="M12 6v7" /><path d="M12 18h.01" /></Svg>,
} satisfies Record<string, (props: IconProps) => ReactNode>

export type IconName = keyof typeof ICON_DRAWINGS

/** The design's line icons, drawn at the sizes and stroke widths the prototype uses. */
export function Icon({ name, ...props }: IconProps & { name: IconName }) {
  return ICON_DRAWINGS[name](props)
}

export function PauseBars() {
  return (
    <>
      <span className="mx-pause-bar" />
      <span className="mx-pause-bar" />
    </>
  )
}

type IconKind = { kind: 'shell' | 'codex' | 'claude' | 'deepseek' | 'gemini' | 'opencode' | 'rust'; glyph: ReactNode }

const MARK_SIZE = '66%'

function Mark({ d }: { d: string }) {
  return (
    <svg viewBox="0 0 24 24" width={MARK_SIZE} height={MARK_SIZE} fill="currentColor" fillRule="evenodd" clipRule="evenodd" aria-hidden="true" focusable="false"><path d={d} /></svg>
  )
}

const ICONS: Record<string, IconKind> = {
  claude: { kind: 'claude', glyph: <Mark d={CLAUDE_MARK} /> },
  codex: { kind: 'codex', glyph: <Mark d={CODEX_MARK} /> },
  deepseek: { kind: 'deepseek', glyph: <Mark d={DEEPSEEK_MARK} /> },
  gemini: { kind: 'gemini', glyph: <Mark d={GEMINI_MARK} /> },
  opencode: { kind: 'opencode', glyph: <Mark d={OPENCODE_MARK} /> },
  cargo: { kind: 'rust', glyph: 'rs' },
  rustc: { kind: 'rust', glyph: 'rs' },
}
const SHELL_ICON: IconKind = { kind: 'shell', glyph: '>_' }

type IconSize = 18 | 20 | 22 | 24 | 28

export function AppIcon({ process, size, dimmed = false }: { process: string; size: IconSize; dimmed?: boolean }) {
  const program = process.split(/\s+/)[0] ?? ''
  const icon = ICONS[program] ?? (program === SHELL_PROCESS || program === '' ? SHELL_ICON : null)
  const className = `mx-app-icon size-${String(size)}${dimmed ? ' is-dimmed' : ''}`
  if (icon) return <span className={`${className} is-${icon.kind}`} aria-hidden="true">{icon.glyph}</span>
  // Any other agent or program: its first letter on a colour taken from its name, so each stays recognisable.
  return <span className={`${className} is-tinted`} style={folderStyle(program)} aria-hidden="true">{program.charAt(0).toUpperCase()}</span>
}

/** The badge on a sidebar row's icon: a shape per status, so colour is never the only signal. */
export function StatusBadge({ status, rail = false }: { status: SessionStatus; rail?: boolean }) {
  switch (status) {
    // A row shows a working session with a spinner beside its title; the rail and the tray have no room for one.
    case 'working':
      return rail ? <span className="mx-badge is-working"><Spinner className="" /></span> : null
    case 'needs':
      return <span className="mx-badge is-needs"><PauseBars /></span>
    case 'done':
      return <span className="mx-badge is-done"><Icon name="check" /></span>
    case 'error':
      return <span className="mx-badge is-error"><Icon name="bang" /></span>
    case 'unknown':
      return rail ? null : <span className="mx-badge is-unknown" />
    case 'idle':
      return null
  }
}
