import { machineLabel } from '@/data/machineNicknames'
import { terminalOf } from '@/api/daemon'
import type { Conversation, PermissionAsk, TerminalDetails } from '@/data/preview'
import type { Check } from '@/features/inspect/checks'
import type { PullRequest } from './pullRequest'
import { listedSubagents, toSubagentView, type SubagentView } from './subagents'

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
  /** It runs in a linked worktree, as the daemon says for the pane. */
  worktree: boolean
  /** Marked with a pin glyph. A pin never reorders the list. */
  pinned: boolean
  /** Lines added and removed in the session's checkout against HEAD; zero outside git. */
  diffAdded: number
  diffRemoved: number
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
  /** The agent's own conversation id; null when unknown. */
  agentSession?: string | null
  /** The effort the agent's own files say it runs at; null when they do not say. */
  agentEffort?: string | null
  /** When the status last changed, in milliseconds, as the daemon says; null when it did not say. */
  statusSince: number | null
  /** What set the status, in the daemon's own words (`hook: Stop`); empty when it gave no reason. */
  statusWhy: string
  timeline: TerminalDetails['timeline']
  usage: TerminalDetails['usage']
  /** The plan limits of its agent, as last reported; empty when it reported none. */
  limits: NonNullable<TerminalDetails['limits']>
  /** The pull request of its branch, as the daemon polls it; null when the branch has none. */
  pr: PullRequest | null
  /** What its agent asked the person to give it with `maitty_request_secret`; empty when it asked for nothing. */
  secretAsk: readonly SecretAsk[]
  /** The checks that ran for its last turns; empty when none did. */
  checks: readonly Check[]
  /** The messages its daemon holds for the working agent, oldest first: each is typed in when a turn ends. */
  queued: readonly string[]
  /** The tasks of the helpers the session asked to start and waits on the person for; empty when it asked for none. */
  helperAsk: readonly string[]
  /** The agent's subagents, in the order its runtime reported them. */
  subagents: SubagentView[]
  status: SessionStatus
  /** The status before the daemon disconnected. */
  lastKnownStatus: SessionStatus | null
  conversation: Conversation
}

/** A secret an agent is waiting for. The value is never here: it only goes from the person to the daemon. */
export type SecretAsk = { id: number; label: string; reason: string }

export const SHELL_PROCESS = 'shell'
const MS_PER_SECOND = 1000

/** What leads the name of a session on another machine, so it is never taken for one here. */
export function machinePrefix(machineName: string | null | undefined): string {
  return machineName ? `${machineLabel(machineName)} · ` : ''
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

const STATUS_FROM_CONVERSATION: Record<Conversation['status'], SessionStatus> = {
  working: 'working',
  'needs-you': 'needs',
  done: 'done',
  idle: 'idle',
  recovery: 'error',
  unknown: 'unknown',
}

/**
 * The daemon's status for a session, with a failed last command shown as exited. The daemon sets
 * `exit` (a non-zero code from the shell) and clears it when work starts again, so the window only
 * displays it; a session that is working or needs you shows that instead.
 */
function statusOf(conversation: Conversation): SessionStatus {
  const status = STATUS_FROM_CONVERSATION[conversation.status]
  const failed = conversation.details?.exit != null
  return failed && (status === 'idle' || status === 'done') ? 'error' : status
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

/**
 * A row's repository, branch and title once a session has shown them, kept while it stays in the same
 * folder. A `git pull` or rebase makes the branch blank or switch for a moment, and a row that follows
 * it jumps; the labels change only when the session moves to another folder.
 */
export function keepLabels(view: SessionView, before: SessionView | undefined): SessionView {
  if (before === undefined || before.repoName === null || before.branch === null || before.cwd !== view.cwd) return view
  return { ...view, repoName: before.repoName, branch: before.branch, title: before.title, promptPath: before.promptPath }
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
  const status = statusOf(conversation)
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
    worktree: conversation.linked ?? false,
    pinned: conversation.pinned ?? false,
    diffAdded: conversation.diffAdded ?? 0,
    diffRemoved: conversation.diffRemoved ?? 0,
    archived: conversation.archived ?? false,
    ended: false,
    promptPath,
    process: processName(conversation.agent, conversation.process),
    lastPrompt: (conversation.summary ?? '').trim(),
    asking: conversation.details?.asking ?? '',
    ask: conversation.details?.ask ?? null,
    interrupted: conversation.details?.interrupted ?? null,
    agentSession: conversation.details?.agentSession ?? null,
    agentEffort: conversation.details?.agentEffort ?? null,
    statusSince: (conversation.details?.since ?? 0) > 0 ? (conversation.details?.since ?? 0) * MS_PER_SECOND : null,
    statusWhy: conversation.details?.statusWhy ?? '',
    timeline: conversation.details?.timeline ?? [],
    usage: conversation.details?.usage ?? { context: null, cost: null, tokens: null },
    limits: conversation.details?.limits ?? [],
    pr: conversation.details?.pr ?? null,
    checks: conversation.details?.checks ?? [],
    secretAsk: conversation.details?.secretAsk ?? [],
    queued: conversation.details?.queued ?? [],
    helperAsk: conversation.details?.helperAsk ?? [],
    // Which helpers a row lists (`listedSubagents`): not one that finished, nor one that stopped once the turn is over.
    subagents: listedSubagents((conversation.details?.subagents ?? []).map(toSubagentView), status === 'working'),
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

/**
 * The sessions in the sidebar's order: creation order, which is the daemon's pane number (it only
 * counts up). Status, pin and focus never move a row.
 */
export function inCreationOrder(sessions: readonly SessionView[]): SessionView[] {
  return [...sessions].sort((a, b) => a.pane - b.pane || a.machine - b.machine)
}

/** What a row calls a session: the name you gave it, else the program running in it. */
export function rowName(session: Pick<SessionView, 'name' | 'process'>): string {
  return session.name ?? session.process
}

/** The folder a row's badge shows: its last segment. */
export function folderNameOf(session: Pick<SessionView, 'repoRoot' | 'cwd'>): string {
  const place = session.repoRoot ?? session.cwd
  return baseName(place)
}
