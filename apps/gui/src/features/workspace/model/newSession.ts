import type { FolderListing } from '@/api/folders'
import { LOCAL_MACHINE } from '@/api/machines'
import type { ChatAgent, PaneView } from './chatView'
import type { MachineHead } from './machines'
import type { SessionView } from './session'
import { splitTyped, withoutTrailingSlash } from './folderPath'

/** One machine the session can go to. Offline ones are shown, greyed out. */
export type NewSessionMachine = { id: number; name: string; online: boolean; note: string }

/** What a new session runs: a plain shell, or an agent typed into one. */
export type StartAs = 'terminal' | ChatAgent

/** The three steps of the New session modal, in the order they are asked. */
export const NEW_SESSION_STEPS = ['project', 'machine', 'agent'] as const
export type NewSessionStep = (typeof NEW_SESSION_STEPS)[number]

/** Why the Chat view cannot be chosen, or null when it can. Shells and other machines only have a terminal. */
export function viewLockReason(startAs: StartAs, machine: number): string | null {
  if (startAs === 'terminal') return 'Shells always open in Terminal'
  if (machine !== LOCAL_MACHINE) return 'Chat runs on this computer'
  return null
}

/** The view a session opens in: the choice, unless it is locked to the terminal. */
export function effectiveStartView(choice: PaneView, startAs: StartAs, machine: number): PaneView {
  return viewLockReason(startAs, machine) === null ? choice : 'terminal'
}

/** Text that is a path (`~/code`, `/srv/app`) rather than part of a project's name. */
export function isPathLike(typed: string): boolean {
  const text = typed.trim()
  return text.startsWith('~') || text.startsWith('/')
}

/** The last segment of a folder: what a pill or a row calls it. */
export function folderName(path: string): string {
  const folder = withoutTrailingSlash(path)
  if (folder === '/' || folder === '~') return folder
  return folder.slice(folder.lastIndexOf('/') + 1) || folder
}

/** What the grey text after the caret offers: the rest to show, and the whole folder Tab takes. */
export type Completion = { rest: string; folder: string }

/**
 * The best completion of what is typed. A name completes from the known projects, shown as the rest
 * of the project's name; a path completes from the folders that really exist on the machine
 * (`listing` is the one of the folder the typed path is inside). Nothing when nothing continues it.
 */
export function completionFor(typed: string, projects: readonly string[], listing: FolderListing | null): Completion | null {
  if (typed.trim() === '' || typed !== typed.trimStart()) return null
  if (isPathLike(typed)) {
    if (listing?.exists !== true) return null
    const { folder, part } = splitTyped(typed)
    const wanted = part.toLowerCase()
    const entry = listing.entries.find((candidate) => (part.startsWith('.') || !candidate.name.startsWith('.')) && candidate.name.toLowerCase().startsWith(wanted) && candidate.name.length > part.length)
    return entry ? { rest: entry.name.slice(part.length), folder: `${folder}${entry.name}/` } : null
  }
  const wanted = typed.toLowerCase()
  for (const project of projects) {
    const name = folderName(project)
    if (name.length > typed.length && name.toLowerCase().startsWith(wanted)) return { rest: name.slice(typed.length), folder: project }
  }
  return null
}

/** The projects whose name or path contains what is typed, best (prefix) matches first. */
export function matchingProjects(typed: string, projects: readonly string[]): string[] {
  const wanted = typed.trim().toLowerCase()
  if (wanted === '' || isPathLike(wanted)) return [...projects]
  const rank = (project: string): number | null => {
    const name = folderName(project).toLowerCase()
    if (name.startsWith(wanted)) return 0
    return name.includes(wanted) || project.toLowerCase().includes(wanted) ? 1 : null
  }
  return projects.filter((project) => rank(project) !== null).sort((a, b) => (rank(a) ?? 0) - (rank(b) ?? 0))
}

/** The machines the modal offers: this computer, then the others, each with how its line is. */
export function modalMachines(heads: readonly MachineHead[]): NewSessionMachine[] {
  if (heads.length === 0) return [{ id: LOCAL_MACHINE, name: 'This computer', online: true, note: '' }]
  return heads.map((head) => ({
    id: head.machine,
    name: head.name ?? 'This computer',
    online: !head.down && head.chip?.tone !== 'bad' && head.chip?.word !== 'connecting',
    note: head.chip?.word ?? '',
  }))
}

/**
 * Folders sessions of a machine are in, most recent first. The daemon numbers panes in creation
 * order, so the highest number is the most recent.
 */
export function recentFolders(sessions: readonly SessionView[], machine: number): string[] {
  const folders = [...sessions]
    .filter((session) => session.machine === machine && session.cwd !== '')
    .sort((a, b) => b.pane - a.pane)
    .map((session) => session.cwd)
  return [...new Set(folders)]
}
