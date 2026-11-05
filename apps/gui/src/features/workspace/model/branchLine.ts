import { displayName, machinePrefix, SHELL_PROCESS, type SessionView } from './session'

/**
 * The branch an agent is on, for a line under its name. A row at a checkout's root already names its
 * branch, so the line is for rows that say something else, and never for a plain shell.
 */
export function branchLineOf(session: Pick<SessionView, 'branch' | 'process' | 'name' | 'title' | 'machineName'>): string | null {
  if (session.branch === null || session.process === SHELL_PROCESS) return null
  return displayName(session) === `${machinePrefix(session.machineName)}${session.branch}` ? null : session.branch
}
