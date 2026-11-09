import type { MachineHead } from './machines'
import { folderNameOf, inCreationOrder, matchesFilter, SHELL_PROCESS, type SessionStatus, type SessionView } from './session'

/**
 * The most urgent first: what needs you, an agent that stopped, what is working, then what finished,
 * and a shell whose last command failed at the end. Within a rank the sessions keep their creation
 * order, so a row only moves when its state changes.
 */
const URGENCY: Record<SessionStatus, number> = { needs: 0, error: 1, working: 2, done: 3, unknown: 4, idle: 5 }
const EXITED_SHELL = 6

function urgencyOf(session: SessionView): number {
  return session.status === 'error' && isShell(session) ? EXITED_SHELL : URGENCY[session.status]
}

/** The order the sidebar lists a machine's sessions in: as created, or with what needs you first and the rest as created. */
export type SidebarOrder = 'creation' | 'priority'

export function bySidebarOrder(sessions: readonly SessionView[], order: SidebarOrder): SessionView[] {
  const created = inCreationOrder(sessions)
  return order === 'priority' ? created.sort((a, b) => Number(b.status === 'needs') - Number(a.status === 'needs')) : created
}

export function byUrgency(sessions: readonly SessionView[]): SessionView[] {
  return inCreationOrder(sessions).sort((a, b) => urgencyOf(a) - urgencyOf(b))
}

/** The project a row names on its second line: the repository's folder, else the folder it is in. */
export function projectOf(session: SessionView): string {
  return session.repoName ?? folderNameOf(session)
}

/**
 * The square before a project's name: one of five colours, chosen from its name so a project is
 * always the same colour. They are CSS variables (`--proj-1` to `--proj-5`), never amber or red.
 */
const PROJECT_COLORS = 5
const HASH_SEED = 5381
const HASH_SHIFT = 5

export function projectColor(project: string): string {
  let hash = HASH_SEED
  for (let index = 0; index < project.length; index += 1) hash = ((hash << HASH_SHIFT) + hash + project.charCodeAt(index)) | 0
  return `var(--proj-${String((Math.abs(hash) % PROJECT_COLORS) + 1)})`
}

export function isShell(session: Pick<SessionView, 'process'>): boolean {
  return session.process === SHELL_PROCESS
}

/**
 * What a row and a pane header call a session: the name you gave it, else the agent the daemon
 * detected, else the folder. A program running in a shell (`node`, `vim`) is not a name: it comes
 * and goes with each command, and a title that followed it would change under you.
 */
export function rowTitle(session: SessionView): string {
  if (session.name !== null) return session.name
  return session.conversation.agent ? session.process : folderNameOf(session)
}

/** The words at the right of a row: only a stopped agent or an exited shell says anything. */
export function rowWord(session: SessionView): string {
  if (session.status !== 'error') return ''
  return isShell(session) ? 'exited' : 'stopped'
}

export type ProjectGroup = { name: string; color: string; sessions: SessionView[]; needs: number }

/** A machine's rows grouped by project, in the order each project first appears in the urgency list. */
export function projectGroups(sessions: readonly SessionView[]): ProjectGroup[] {
  const groups = new Map<string, SessionView[]>()
  for (const session of sessions) {
    const name = projectOf(session)
    groups.set(name, [...(groups.get(name) ?? []), session])
  }
  return [...groups.entries()].map(([name, members]) => ({ name, color: projectColor(name), sessions: members, needs: members.filter((member) => member.status === 'needs').length }))
}

export type MachineGroup = { head: MachineHead; sessions: SessionView[] }

/**
 * The sidebar's groups: one per machine (this computer first), each with its sessions that match
 * the filter, most urgent first. With a filter, a machine with nothing matching is left out; with no
 * sessions anywhere and no other machine there are no groups, only the note that says how to start
 * one. Another machine keeps its header with no sessions, as a machine with no app yet has none and
 * its header is where it says so.
 */
export function machineGroups(sessions: readonly SessionView[], heads: readonly MachineHead[], query: string, keepEmpty = false, order: SidebarOrder = 'creation'): MachineGroup[] {
  const filtering = query.trim() !== ''
  // A lone machine with nothing on it has no heading, unless something (a draft chat) is waiting there.
  if (!keepEmpty && sessions.length === 0 && heads.length <= 1) return []
  return heads
    .map((head) => ({ head, sessions: bySidebarOrder(sessions.filter((session) => session.machine === head.machine && matchesFilter(session, query)), order) }))
    .filter((group) => !filtering || group.sessions.length > 0)
}
