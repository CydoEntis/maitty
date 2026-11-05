/** How long nobody may type before the app counts as idle. */
const IDLE_MINUTES = 5
const MS_PER_MINUTE = 60_000
export const IDLE_INPUT_MS = IDLE_MINUTES * MS_PER_MINUTE

export type IdleReading = {
  now: number
  /** When a key was last pressed in this window. */
  lastInputAt: number
  /** The window is shown and in front. */
  windowActive: boolean
  /** An agent is working or a request is waiting on the user. */
  agentBusy: boolean
}

/**
 * Whether the app may update itself now. All three must hold: no agent working or waiting on an
 * answer (never mid-turn), no typing for five minutes, and the window out of sight (unfocused or
 * minimized), so the restart never lands on someone who is looking at it.
 */
export function isIdleForUpdate({ now, lastInputAt, windowActive, agentBusy }: IdleReading): boolean {
  if (agentBusy) return false
  if (now - lastInputAt < IDLE_INPUT_MS) return false
  return !windowActive
}

export type IdleStep = 'download' | 'install'

/** What an idle app does next about an update: fetch one that was found, install one that was fetched. */
export function idleUpdateStep(idle: boolean, status: string, offeredIsSkipped: boolean): IdleStep | null {
  if (!idle) return null
  if (status === 'available') return offeredIsSkipped ? null : 'download'
  if (status === 'ready') return 'install'
  return null
}
