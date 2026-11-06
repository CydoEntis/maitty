import { terminalOf } from '@/api/daemon'
import type { Conversation, PermissionAsk, TerminalDetails } from '@/data/preview'

/** The status vocabulary of the desktop app. Each one has its own shape, never colour alone. */
export type SessionStatus = 'working' | 'needs' | 'done' | 'error' | 'idle' | 'unknown'

/** One daemon-owned shell as the window shows it. Every view reads this model, not the wire shape. */
export type SessionView = {
  id: string
  /** The daemon's pane number; it only grows, so it is also the creation order. Each machine numbers its own. */
  pane: number
  /** The number the host gave the machine this session lives on; 0 for this computer. */
  machine: number
  /** The machine's name; null for this computer. */
  machineName: string | null
  cwd: string
  /** The current folder with the home folder written as `~`. */
  cwdLabel: string
  repoRoot: string | null
  repoName: string | null
  branch: string | null
  /** The branch at a checkout's root, the folder deeper in (`crates/maitty-core`), or the `~` path outside a repo. */
  title: string
  /** The name you gave the session; null until you rename it. The daemon keeps it for every app. */
  name: string | null
  /** Pinned to the top of its project: it leads the project and sorts first in the tray. */
  pinned: boolean
  /** Put away from the project lists and tabs. It keeps running in the daemon. */
  archived: boolean
  /** The daemon no longer lists it, but its pane stays on screen until you close or restart it. */
  ended: boolean
  /** The repo name and folder within it, or the `~` path: what a prompt would show. */
  promptPath: string
  /** What runs in the shell. The daemon reports agents only, so anything else reads as `shell`. */
  process: string
  /** The last prompt the agent in this session was given; empty for a plain shell. */
  lastPrompt: string
  /** What a stopped agent is asking (the exact command and choices); empty when it is not stopped on a choice. */
  asking: string
  /** The permission prompt the agent's own hook says it is stopped on. */
  ask: PermissionAsk | null
  interrupted: TerminalDetails['interrupted']
  timeline: TerminalDetails['timeline']
  usage: TerminalDetails['usage']
  status: SessionStatus
  /** The status before the daemon disconnected. */
  lastKnownStatus: SessionStatus | null
  conversation: Conversation
}

export const SHELL_PROCESS = 'shell'

/** What leads the name of a session on another machine, so it is never taken for one here. */
export function machinePrefix(machineName: string | null | undefined): string {
  return machineName ? `${machineName} · ` : ''
}

/** What a row calls a session: the name you gave it, else where it is, led by its machine when it is not this computer. */
export function displayName(session: Pick<SessionView, 'name' | 'title'> & { machineName?: string | null }): string {
  return `${machinePrefix(session.machineName)}${session.name ?? session.title}`
}

/** The longest name a session may be given. */
export const MAX_SESSION_NAME_LENGTH = 60

/** The most characters of a prompt a popover shows before it ends the line with an ellipsis. */
const PROMPT_PREVIEW_CHARS = 160

/** A prompt on one line, cut to a readable length. */
export function truncatePrompt(text: string, limit = PROMPT_PREVIEW_CHARS): string {
  // Agents sometimes quote their own markup (`<task-notification>`); a card never shows it.
  const line = text.replace(/<\/?[a-z][\w-]*(?:\s[^>]*)?>/gi, ' ').replace(/\s+/g, ' ').trim()
  return line.length <= limit ? line : `${line.slice(0, limit - 1).trimEnd()}…`
}

/** The programs that are just the user's shell: a session running only one of these reads as "shell". */
const SHELL_PROGRAMS: ReadonlySet<string> = new Set(['sh', 'bash', 'zsh', 'fish', 'dash', 'ksh', 'tcsh', 'csh', 'nu', 'xonsh', 'pwsh', 'powershell', 'cmd'])

/** What a session is using: the agent the daemon detected, else the program running in it, else "shell". */
export function processName(agent: string | null | undefined, program: string | undefined): string {
  if (agent) return agent
  const name = (program ?? '').trim().split(/[\\/]/).at(-1)?.replace(/\.exe$/i, '').replace(/^-/, '').toLowerCase() ?? ''
  return name === '' || SHELL_PROGRAMS.has(name) ? SHELL_PROCESS : name
}
const SEPARATOR = /[\\/]/
// A folder deep in a repo is shown by its last two parts (`crates/maitty-core`).
const TITLE_PARTS = 2
const SHORT_NAME_LETTERS = 2

const STATUS_FROM_CONVERSATION: Record<Conversation['status'], SessionStatus> = {
  working: 'working',
  'needs-you': 'needs',
  done: 'done',
  idle: 'idle',
  recovery: 'error',
  unknown: 'unknown',
}

export const STATUS_WORD: Record<SessionStatus, string> = {
  working: 'working',
  needs: 'needs you',
  done: 'done',
  error: 'exited',
  idle: 'idle',
  unknown: 'unknown',
}

function trimTrailingSeparator(path: string): string {
  return path.length > 1 ? path.replace(/[\\/]+$/, '') : path
}

/**
 * One spelling for a Windows path, so the same folder always groups together: git writes a
 * worktree's main checkout as `C:/dev/x` while a shell's own folder reads `C:\dev\x`.
 */
function normalizeRoot(path: string): string {
  const drive = /^([A-Za-z]):[\\/]/.exec(path)
  return drive ? `${(drive[1] ?? '').toUpperCase()}${path.slice(1).replaceAll('/', '\\')}` : path
}

function baseName(path: string): string {
  return path.split(SEPARATOR).filter(Boolean).at(-1) ?? path
}

/** `path` with the home folder written as `~`. */
export function tildePath(path: string, home: string): string {
  const clean = trimTrailingSeparator(path)
  const homeClean = home ? trimTrailingSeparator(home) : ''
  if (!homeClean) return clean
  if (clean === homeClean) return '~'
  if (clean.startsWith(`${homeClean}/`) || clean.startsWith(`${homeClean}\\`)) return `~${clean.slice(homeClean.length).replaceAll('\\', '/')}`
  return clean
}

export function toSessionView(conversation: Conversation, home: string): SessionView {
  const cwd = trimTrailingSeparator(conversation.cwd ?? conversation.folder)
  const machineName = conversation.machineName ?? null
  // The home folder written as `~` is this computer's; another machine's paths are shown as they are.
  const homeHere = machineName === null ? home : ''
  const repoName = conversation.repo ?? null
  const repoRoot = repoName === null ? null : normalizeRoot(trimTrailingSeparator(conversation.folder))
  const inside = repoRoot !== null && cwd.length > repoRoot.length ? cwd.slice(repoRoot.length + 1).replaceAll('\\', '/') : ''
  const cwdLabel = tildePath(cwd, homeHere)
  const promptPath = repoName === null ? cwdLabel : inside === '' ? repoName : `${repoName}/${inside}`
  const branch = repoName !== null && conversation.branch && conversation.branch !== 'working tree' ? conversation.branch : null
  // At a checkout's root the row says its branch; deeper in, the folder is what tells sessions apart.
  const title = repoName === null ? cwdLabel : inside === '' ? branch ?? repoName : inside.split('/').slice(-TITLE_PARTS).join('/')
  const status = STATUS_FROM_CONVERSATION[conversation.status]
  const lastKnown = conversation.lastKnownStatus ? STATUS_FROM_CONVERSATION[conversation.lastKnownStatus] : null
  return {
    id: conversation.id,
    pane: conversation.nativePane ?? terminalOf(conversation.id) ?? 0,
    machine: conversation.machine ?? 0,
    machineName,
    cwd,
    cwdLabel,
    repoRoot,
    repoName,
    branch,
    title,
    name: conversation.named ? conversation.title.trim() || null : null,
    pinned: conversation.pinned ?? false,
    archived: conversation.archived ?? false,
    ended: false,
    promptPath,
    process: processName(conversation.agent, conversation.process),
    lastPrompt: (conversation.summary ?? '').trim(),
    asking: conversation.details?.asking ?? '',
    ask: conversation.details?.ask ?? null,
    interrupted: conversation.details?.interrupted ?? null,
    timeline: conversation.details?.timeline ?? [],
    usage: conversation.details?.usage ?? { context: null, cost: null, tokens: null },
    status,
    lastKnownStatus: lastKnown,
    conversation,
  }
}

/** The words after the process on a sidebar row's second line. */
export function statusText(session: SessionView): string {
  switch (session.status) {
    case 'working':
      return 'working'
    case 'needs':
      return 'needs you'
    case 'done':
      return 'finished'
    case 'error':
      return 'exited with error'
    case 'unknown':
      return session.lastKnownStatus ? `last seen ${STATUS_WORD[session.lastKnownStatus]}` : 'unknown'
    case 'idle':
      return 'idle'
  }
}

/**
 * Programs whose app icon already says what they are, so a row or tab need not write the name beside
 * it. Keep in step with the icons in `glyphs.tsx`; any other program gets a tinted letter, which does not.
 */
const ICONED_PROGRAMS: ReadonlySet<string> = new Set(['claude', 'codex', 'deepseek', 'gemini', 'opencode', 'cargo', 'rustc', 'shell'])

export function iconSaysProcess(process: string): boolean {
  const program = process.split(/\s+/)[0] ?? ''
  return program === '' || ICONED_PROGRAMS.has(program)
}

const STATUS_RANK: Record<SessionStatus, number> = { needs: 0, error: 0, working: 1, done: 2, idle: 2, unknown: 2 }

/** The session a tab reports: whatever needs you first, then whatever is working. */
export function worstSession(sessions: readonly SessionView[]): SessionView | undefined {
  return [...sessions].sort((a, b) => STATUS_RANK[a.status] - STATUS_RANK[b.status])[0]
}

export type GroupBy = 'repo' | 'folder'

/** Where a session's project is: its git repo's root, or (outside a repo, or grouping by folder) its own folder. */
function placeOf(session: Pick<SessionView, 'repoRoot' | 'cwd'>, groupBy: GroupBy): string {
  return (groupBy === 'repo' ? session.repoRoot : null) ?? session.cwd
}

/**
 * The project a session belongs to: its git repo's root, or (outside a repo, or grouping by folder)
 * its own folder. A folder of another machine is a project of its own, whatever a folder here is called.
 */
export function groupKeyOf(session: Pick<SessionView, 'repoRoot' | 'cwd'> & { machineName?: string | null }, groupBy: GroupBy): string {
  const place = placeOf(session, groupBy)
  return session.machineName ? `${session.machineName}\u0000${place}` : place
}

export type SessionGroup = {
  key: string
  name: string
  kind: 'repo' | 'folder'
  /** Shown on hover: the repo or folder path. */
  path: string
  /** The machine the project is on; null for this computer. */
  machineName: string | null
  sessions: SessionView[]
  containsFocused: boolean
  needsCount: number
  workingCount: number
}

/**
 * Sidebar groups: by the git repo that holds each shell's folder (a shell outside any repo is
 * grouped by its own folder), or always by folder. Groups sort by name; rows keep creation order,
 * so nothing moves when a status changes.
 */
export function groupSessions(sessions: readonly SessionView[], groupBy: GroupBy, focusedId: string | null, home: string): SessionGroup[] {
  const groups = new Map<string, SessionGroup>()
  for (const session of [...sessions].sort((a, b) => a.pane - b.pane)) {
    const repoRoot = groupBy === 'repo' ? session.repoRoot : null
    const key = groupKeyOf(session, groupBy)
    const place = placeOf(session, groupBy)
    let group = groups.get(key)
    if (!group) {
      group = {
        key,
        name: `${machinePrefix(session.machineName)}${repoRoot ? (session.repoName ?? baseName(place)) : session.cwdLabel === '~' ? '~' : baseName(place)}`,
        kind: repoRoot ? 'repo' : 'folder',
        path: tildePath(place, session.machineName ? '' : home),
        machineName: session.machineName,
        sessions: [],
        containsFocused: false,
        needsCount: 0,
        workingCount: 0,
      }
      groups.set(key, group)
    }
    group.sessions.push(session)
    if (session.id === focusedId) group.containsFocused = true
    if (session.status === 'needs') group.needsCount += 1
    if (session.status === 'working') group.workingCount += 1
  }
  return [...groups.values()].sort((a, b) => a.name.localeCompare(b.name))
}

const MAX_INITIALS = 3

/** The rail's short label for a group: initials of its parts (`maitty-docs` → `MD`), else two letters. */
export function groupInitials(group: Pick<SessionGroup, 'kind' | 'name'>): string {
  const parts = group.name.split(/[-_.]/).filter(Boolean)
  if (parts.length > 1) return parts.map((part) => part[0] ?? '').join('').slice(0, MAX_INITIALS).toUpperCase()
  const letters = group.name.slice(0, SHORT_NAME_LETTERS)
  return `${letters.slice(0, 1).toUpperCase()}${letters.slice(1).toLowerCase()}`
}
