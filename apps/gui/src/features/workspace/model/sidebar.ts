import { isChatSession } from './chatSession'
import type { MachineHead } from './machines'
import { folderNameOf, inCreationOrder, limitedWord, SHELL_PROCESS, untilWord, type SessionStatus, type SessionView } from './session'

/**
 * The most urgent first: what needs you, an agent that stopped, one held by its plan limit, what is
 * working, then what finished, and a shell whose last command failed at the end. Within a rank the sessions keep their creation
 * order, so a row only moves when its state changes.
 */
const URGENCY: Record<SessionStatus, number> = { needs: 0, error: 1, limited: 2, working: 3, done: 4, unknown: 5, idle: 6 }
const EXITED_SHELL = 7

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

/**
 * Whether closing this session would lose something: it has not ended, and it is an agent, a chat, or a shell
 * with a program running in it. A plain idle shell loses nothing that Undo does not bring back.
 */
export function keepsRunningUnseen(session: SessionView): boolean {
  return !session.ended && !(isShell(session) && !isChatSession(session.conversation))
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

/** The words at the right of a row: only a stopped agent, an exited shell or a session held by its plan limit says anything. */
export function rowWord(session: SessionView, now: number = Date.now()): string {
  if (session.snoozedUntil !== null) return untilWord(session.snoozedUntil, now)
  if (session.status === 'limited') return limitedWord(session.resumeAt, now)
  // Its machine or the daemon is out of reach, or the list just missed it: the last status is not known to be true.
  if (session.status === 'unknown') return 'offline'
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

/** What the sidebar's headings are: what each session is doing, its project, or the machine it runs on. */
export type SidebarGrouping = 'status' | 'project' | 'machine'

export const SIDEBAR_GROUPINGS: ReadonlyArray<readonly [SidebarGrouping, string]> = [['status', 'Status'], ['project', 'Project'], ['machine', 'Machine']]

/**
 * A session listed under Terminals: one with no agent in it (a shell, or a program running in one, such as `pnpm dev`).
 * One that needs you stays with the agents, because what needs you is never at the bottom of the list.
 */
export function isTerminal(session: Pick<SessionView, 'conversation' | 'status'>): boolean {
  return !session.conversation.agent && session.status !== 'needs'
}

/** The terminals, as created: the last section of the status and project lists, so agents are never sorted among dev servers. */
export function terminalSessions(sessions: readonly SessionView[]): SessionView[] {
  return inCreationOrder(sessions.filter(isTerminal))
}

export function isSidebarGrouping(value: unknown): value is SidebarGrouping {
  return SIDEBAR_GROUPINGS.some(([grouping]) => grouping === value)
}

export type StatusSectionKey = 'needs' | 'working' | 'done' | 'idle'

/** The status sections in the order they are listed, each with the few words that say what it means. */
const STATUS_SECTIONS: ReadonlyArray<{ key: StatusSectionKey; name: string; hint: string }> = [
  { key: 'needs', name: 'Needs you', hint: 'waiting on you' },
  { key: 'working', name: 'Working', hint: 'running now' },
  { key: 'done', name: 'Finished', hint: 'ready to review' },
  { key: 'idle', name: 'Idle', hint: 'nothing to do' },
]

/** The section a session is listed in. An agent that stopped, or one held by its plan limit, is yours to look at, so it sits with what needs you. */
function sectionOf(session: SessionView): StatusSectionKey {
  switch (session.status) {
    case 'needs': return 'needs'
    case 'error':
    case 'limited': return 'needs'
    case 'working': return 'working'
    case 'done': return 'done'
    case 'idle':
    case 'unknown': return 'idle'
  }
}

export type StatusSection = { key: StatusSectionKey; name: string; hint: string; sessions: SessionView[] }

/** The agents under the status each has, as created within a section; a section with nothing in it is left out. Terminals have their own section (`terminalSessions`). */
export function statusSections(sessions: readonly SessionView[]): StatusSection[] {
  const created = inCreationOrder(sessions.filter((session) => !isTerminal(session)))
  return STATUS_SECTIONS
    .map((section) => ({ ...section, sessions: created.filter((session) => sectionOf(session) === section.key) }))
    .filter((section) => section.sessions.length > 0)
}

/**
 * Put away (`TermInfo.archived`) and listed only under Put away. It keeps running, and while Needs you would list it
 * (an agent that asks, stopped or is held by its plan limit, or a terminal that asks) it is listed there instead, so
 * nothing that waits on you is folded out of sight. The daemon's flag stays set: once it no longer needs you it goes
 * back under Put away, where the person left it. Only Bring back clears it.
 */
function isPutAway(session: SessionView): boolean {
  return (session.archived || session.snoozedUntil !== null) && (isTerminal(session) || sectionOf(session) !== 'needs')
}

/** The sessions the sections list, and the ones put away (as created), which only the Put away section lists. */
export function splitPutAway(sessions: readonly SessionView[]): { listed: SessionView[]; putAway: SessionView[] } {
  return {
    listed: sessions.filter((session) => !isPutAway(session)),
    putAway: inCreationOrder(sessions.filter(isPutAway)),
  }
}

/**
 * A finished or idle session in a terminal the daemon owns can be put away. A chat cannot: its daemon archive is how
 * a chat is closed, which stops its agent.
 */
export function canPutAway(session: SessionView): boolean {
  return !session.archived && session.snoozedUntil === null && !session.ended && (session.status === 'done' || session.status === 'idle') && !isChatSession(session.conversation)
}

/**
 * Snoozing (`TermInfo.snoozed_until`) puts a session away until a time. A finished, idle or limited session in a terminal the
 * daemon owns can be snoozed, once; one that works, asks or failed is not out of the way yet, and a chat cannot (the daemon
 * refuses it). The daemon wakes it by itself, and also when it comes to need you.
 */
export function canSnooze(session: SessionView): boolean {
  const resting = session.status === 'done' || session.status === 'idle' || session.status === 'limited'
  return resting && !session.archived && session.snoozedUntil === null && !session.ended && !isChatSession(session.conversation)
}

export type ProjectChoice = { name: string; color: string; count: number }

/** Every project that has a session, by name, for the list that narrows the sidebar to one of them. */
export function projectChoices(sessions: readonly SessionView[]): ProjectChoice[] {
  const counts = new Map<string, number>()
  for (const session of sessions) counts.set(projectOf(session), (counts.get(projectOf(session)) ?? 0) + 1)
  return [...counts.entries()]
    .map(([name, count]) => ({ name, color: projectColor(name), count }))
    .sort((a, b) => a.name.localeCompare(b.name))
}

/** The sessions of one project, or all of them when none is picked. */
export function inProject(sessions: readonly SessionView[], project: string | null): SessionView[] {
  return project === null ? [...sessions] : sessions.filter((session) => projectOf(session) === project)
}

export type ProjectSection = ProjectGroup & { key: string; head: MachineHead }

/**
 * Every project as one heading, by name. A project that is on two machines gets a heading for each,
 * this computer's first, because the same folder name on another machine is another checkout.
 * Agents only: terminals have their own section (`terminalSessions`).
 */
export function projectSections(sessions: readonly SessionView[], heads: readonly MachineHead[]): ProjectSection[] {
  return heads
    .flatMap((head) => projectGroups(inCreationOrder(sessions.filter((session) => session.machine === head.machine && !isTerminal(session))))
      .map((group) => ({ ...group, key: `${head.key}:${group.name}`, head })))
    .sort((a, b) => a.name.localeCompare(b.name))
}

export type MachineGroup = { head: MachineHead; sessions: SessionView[] }

/**
 * The sidebar's groups by machine: one per machine (this computer first), each with its sessions in
 * `order`. With no sessions anywhere and no other machine there are no groups, only the note that
 * says how to start one. Another machine keeps its header with no sessions, as a machine with no app
 * yet has none and its header is where it says so.
 */
export function machineGroups(sessions: readonly SessionView[], heads: readonly MachineHead[], keepEmpty = false, order: SidebarOrder = 'creation'): MachineGroup[] {
  // A lone machine with nothing on it has no heading, unless something (a draft chat) is waiting there.
  if (!keepEmpty && sessions.length === 0 && heads.length <= 1) return []
  return heads.map((head) => ({ head, sessions: bySidebarOrder(sessions.filter((session) => session.machine === head.machine), order) }))
}
