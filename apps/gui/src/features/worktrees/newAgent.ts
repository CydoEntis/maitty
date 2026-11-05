import type { PaletteItem } from '@/features/workspace/model/palette'
import { tildePath, type SessionGroup } from '@/features/workspace/model/session'
import { branchNameProblem } from './branchName'

/** The agents the dialog offers. The daemon accepts any configured preset; the window offers the two it can start. */
export const BRANCH_AGENTS = [
  { id: 'claude', label: 'Claude' },
  { id: 'codex', label: 'Codex' },
] as const
export type BranchAgent = (typeof BRANCH_AGENTS)[number]['id']

/** How the agent runs: in a terminal of its own, or as a chat session the daemon owns (ADR 0054). */
export const SESSION_MODES = [
  { id: 'terminal', label: 'Terminal', note: 'The agent runs in a terminal session, as if you typed it.' },
  { id: 'chat', label: 'Chat', note: 'A chat with a message box, Stop and a queue. The agent has no terminal. It asks before it runs a tool, unless you turn on full access.' },
] as const
export type SessionMode = (typeof SESSION_MODES)[number]['id']

/** The label and warning of the per-chat full-access switch, shared by every place that starts a chat. */
export const FULL_ACCESS_LABEL = 'Full access (skip permission prompts)'
export const FULL_ACCESS_WARNING = 'No sandbox and no approval prompts: the agent can change files and run commands without asking.'

/** The daemon's limit on a first prompt (characters); it checks again. */
export const MAX_PROMPT_CHARS = 8000

/** How many meaningful words of the prompt name the branch. */
const SLUG_WORDS = 4
const MAX_SLUG_CHARS = 40
export const BRANCH_PREFIX = 'agent/'

/** Words that say nothing about the task, so the branch keeps the ones that do. */
const FILLER: ReadonlySet<string> = new Set([
  'a', 'an', 'the', 'to', 'of', 'in', 'on', 'for', 'and', 'or', 'with', 'is', 'it', 'this', 'that', 'my', 'our', 'we', 'i',
  'please', 'can', 'could', 'you', 'would', 'should', 'let', 'lets', 'need', 'needs', 'want', 'make', 'sure', 'so', 'then',
])

/** `fix-the-login-bug` from "Fix the login bug in the settings page"; empty when the prompt has no word to name it by. */
export function promptSlug(prompt: string): string {
  const words = prompt.toLowerCase().split(/[^a-z0-9]+/).filter((word) => word !== '')
  const meaningful = words.filter((word) => !FILLER.has(word))
  // A prompt made only of filler ("do it") still names its branch by what it has.
  const chosen = (meaningful.length > 0 ? meaningful : words).slice(0, SLUG_WORDS)
  return chosen.join('-').slice(0, MAX_SLUG_CHARS).replace(/-+$/, '')
}

/** The branch the prompt suggests, or nothing until it has a word to go on. */
export function suggestedBranch(prompt: string): string {
  const slug = promptSlug(prompt)
  return slug === '' ? '' : `${BRANCH_PREFIX}${slug}`
}

const BRANCH_ALPHABET = /^[\w./-]+$/

/** Why the branch will not do, or null. The daemon checks the same rules again. */
export function agentBranchProblem(name: string): string | null {
  if (name === '') return 'Name the branch'
  if (!BRANCH_ALPHABET.test(name)) return 'Use letters, digits, _ . / and - only'
  const generic = branchNameProblem(name)
  if (generic !== null) return generic
  if (name === '@' || name.split('/').some((part) => part.startsWith('.') || part.endsWith('.lock'))) return 'No part of a branch name may start with a dot or end in .lock'
  return null
}

/** Why the first prompt will not do, or null. It is optional. */
export function promptProblem(prompt: string): string | null {
  const text = prompt.trim()
  if (text.length > MAX_PROMPT_CHARS) return `Keep the prompt under ${String(MAX_PROMPT_CHARS)} characters`
  // An agent reads a leading dash as one of its own options.
  if (text.startsWith('-')) return 'Begin the prompt with a word, not a dash'
  return null
}

/** Folder-name form of a branch, as the daemon makes it (`agent/fix-login` is `agent-fix-login`). */
function folderName(branch: string): string {
  return Array.from(branch, (char) => (/[\p{L}\p{N}\-_.]/u.test(char) ? char : '-')).join('')
}

/**
 * Where the daemon puts the worktree with its default naming: `<repo>-worktrees/<branch>` beside the
 * repository. A `[worktree] dir` setting changes the real place, so this is a preview.
 */
export function worktreeFolder(projectPath: string, branch: string, home: string): string {
  const trimmed = projectPath.replace(/[\\/]+$/, '')
  const cut = Math.max(trimmed.lastIndexOf('/'), trimmed.lastIndexOf('\\'))
  const separator = trimmed.includes('\\') && !trimmed.includes('/') ? '\\' : '/'
  const parent = cut > 0 ? trimmed.slice(0, cut) : ''
  const name = cut >= 0 ? trimmed.slice(cut + 1) : trimmed
  return tildePath(`${parent}${separator}${name}-worktrees${separator}${folderName(branch)}`, home)
}

/** A project the dialog can start an agent in: a local git repository the app already shows. */
export type AgentProject = {
  /** The sidebar group's key; for a local project, its folder. */
  key: string
  name: string
  /** The repository's folder on this computer. */
  path: string
}

/** The projects that can take a branch agent: this computer's git repositories. Another machine's wait for a later version. */
export function agentProjects(groups: readonly SessionGroup[]): AgentProject[] {
  return groups.filter((group) => group.machineName === null && group.kind === 'repo').map((group) => ({ key: group.key, name: group.name, path: group.key }))
}

/** The project a new agent defaults to: the focused session's, else the first. */
export function defaultProject(projects: readonly AgentProject[], focusedKey: string | null): AgentProject | undefined {
  return projects.find((project) => project.key === focusedKey) ?? projects[0]
}

/** The palette entry; absent when no project can take one, so it never opens an empty dialog. */
export function newAgentPaletteItem(projects: readonly AgentProject[], online: boolean, shortcut: string | undefined, run: () => void): PaletteItem | null {
  if (!online || projects.length === 0) return null
  return { id: 'new-agent', label: 'New agent on a branch…', kind: 'Action', run, ...(shortcut ? { shortcut } : {}) }
}
