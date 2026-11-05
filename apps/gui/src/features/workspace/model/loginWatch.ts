/**
 * A first login in a session: ssh asks a person for a password or a host key in a terminal
 * session of this computer, and the machine is dialled again once that session closes. The
 * window never sees the session's id when it is made, so the session is the first new one of
 * this computer. A session that never shows up is given up on after `LOGIN_WAIT_MS`, so the
 * machine is dialled anyway.
 */
export type LoginWatch = {
  machine: string
  /** The terminal numbers of this computer when the session was asked for. */
  before: ReadonlySet<number>
  startedAt: number
  /** The session's number once it has shown up. */
  term: number | null
}

export const LOGIN_WAIT_MS = 20_000

/** How often a watched login is looked at. */
export const LOGIN_CHECK_MS = 1000

export function startLoginWatch(machine: string, localTerms: Iterable<number>, now: number): LoginWatch {
  return { machine, before: new Set(localTerms), startedAt: now, term: null }
}

/** Where the watch is now, and whether to dial: `watch` is null when it is over. */
export function advanceLoginWatch(watch: LoginWatch, localTerms: ReadonlySet<number>, now: number): { watch: LoginWatch | null; dial: boolean } {
  if (watch.term === null) {
    const shown = [...localTerms].find((term) => !watch.before.has(term))
    if (shown !== undefined) return { watch: { ...watch, term: shown }, dial: false }
    return now - watch.startedAt >= LOGIN_WAIT_MS ? { watch: null, dial: true } : { watch, dial: false }
  }
  return localTerms.has(watch.term) ? { watch, dial: false } : { watch: null, dial: true }
}
