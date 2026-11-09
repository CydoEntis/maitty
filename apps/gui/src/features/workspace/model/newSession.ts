import { projectColor, projectOf } from './sidebar'
import type { SessionView } from './session'

/** A project the New session dialog offers: the folder a session starts in, its name and colour. */
export type NewSessionProject = { folder: string; name: string; color: string; /** How many of the machine's sessions are in it. */ sessions?: number }

/**
 * The projects a machine has sessions in, the focused session's first, then the most recent: the
 * daemon numbers panes in creation order, so the highest number is the newest. A project is its
 * repository's root, else the session's folder. With none, the start folder stands in.
 */
export function machineProjects(sessions: readonly SessionView[], machine: number, focused: SessionView | undefined, startFolder: string): NewSessionProject[] {
  const mine = [...sessions].filter((session) => session.machine === machine && session.cwd !== '').sort((a, b) => b.pane - a.pane)
  const ordered = focused?.machine === machine ? [focused, ...mine] : mine
  const projects = new Map<string, NewSessionProject>()
  for (const session of ordered) {
    const folder = session.repoRoot ?? session.cwd
    const name = projectOf(session)
    if (!projects.has(folder)) projects.set(folder, { folder, name, color: projectColor(name) })
  }
  for (const session of mine) {
    const project = projects.get(session.repoRoot ?? session.cwd)
    if (project) project.sessions = (project.sessions ?? 0) + 1
  }
  if (projects.size === 0 && startFolder !== '') {
    const name = startFolder.split(/[/\\]/).filter(Boolean).at(-1) ?? startFolder
    projects.set(startFolder, { folder: startFolder, name, color: projectColor(name) })
  }
  return [...projects.values()]
}

/** How many of a first message's words name the session it starts. */
const NAME_WORDS = 3

/** A name for a session from its first message: its first few words, without a closing mark. */
export function nameFromMessage(message: string): string {
  return message.trim().split(/\s+/).slice(0, NAME_WORDS).join(' ').replace(/[.,;:!?]+$/, '')
}
