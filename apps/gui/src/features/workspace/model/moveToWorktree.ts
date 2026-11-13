import type { SessionView } from './session'
import { suggestBranchName } from './worktreeName'

/** The ids Claude and Codex use are UUID-like; anything else is not typed into a shell. */
const SESSION_ID = /^[A-Za-z0-9-]{8,80}$/
/** A folder that could end a quoted word or start a substitution in either shell is refused, not escaped. */
const UNSAFE_FOLDER_CHARS = new Set(['"', "'", '`', '$', '%'])
const FIRST_PRINTABLE = 0x20
function unsafeFolder(folder: string): boolean {
  for (let i = 0; i < folder.length; i += 1) {
    if (UNSAFE_FOLDER_CHARS.has(folder.charAt(i)) || folder.charCodeAt(i) < FIRST_PRINTABLE) return true
  }
  return false
}
/** How long the agent gets to exit after `/exit` before the move is given up. */
export const EXIT_WAIT_MS = 15_000
export const EXIT_POLL_MS = 250

/** Why a session cannot be moved to a worktree now, or null when it can. */
export function moveRefusal(session: SessionView): string | null {
  if (session.process !== 'claude' && session.process !== 'codex') return 'Only Claude and Codex chats can move'
  if (session.agentSession == null || !SESSION_ID.test(session.agentSession)) return 'This chat has no saved conversation to carry over yet'
  if (session.status === 'working' || session.status === 'needs') return 'Wait until the agent is idle; moving restarts it'
  return null
}

/** The words after the agent's name that resume a saved conversation. Null when the id is not safe to type. */
export function resumeWords(agent: 'claude' | 'codex', sessionId: string): string[] | null {
  if (!SESSION_ID.test(sessionId)) return null
  return agent === 'claude' ? ['--resume', sessionId] : ['resume', sessionId]
}

/**
 * The line typed into the pane's shell after the agent exits: go to the new folder and resume the
 * same conversation there. Null when the folder or id is not safe to type.
 */
export function resumeLine(agent: 'claude' | 'codex', sessionId: string, folder: string, windows: boolean): string | null {
  const words = resumeWords(agent, sessionId)
  if (words === null || folder === '' || unsafeFolder(folder)) return null
  const then = windows ? ';' : '&&'
  const resume = [agent, ...words, ...(agent === 'codex' ? ['-C', `"${folder}"`] : [])].join(' ')
  return `cd "${folder}" ${then} ${resume}`
}

/** A branch name for the new worktree that no existing branch or worktree uses. */
export function worktreeBranch(taken: readonly string[]): string {
  return suggestBranchName(new Set(taken))
}
