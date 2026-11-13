import type { SessionView } from './session'
import { isShell } from './sidebar'

export const SHARED_CHECKOUT_REFUSAL = 'Another session is using this folder'

/** Where a restore would write: the checkout's top folder on a machine. Null when the daemon did not say. */
export type RestorePlace = { readonly machine: number; readonly checkout: string | null }

/** One spelling per folder: git and the shell write separators differently, and Windows and macOS ignore case. */
const pathKey = (path: string): string => path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()

/**
 * Whether the session's files are the checkout's. Its own checkout decides when the daemon gave one, so a linked
 * worktree nested in the main checkout keeps its own files; otherwise any folder inside the checkout shares them.
 */
function inCheckout(session: SessionView, checkout: string): boolean {
  if (session.checkout !== null) return pathKey(session.checkout) === pathKey(checkout)
  const folder = pathKey(session.cwd)
  const top = pathKey(checkout)
  return folder === top || folder.startsWith(`${top}/`)
}

/**
 * Why Edit from here would undo another session's files, or null when it would not: another open session runs a
 * program other than a plain shell in the same checkout. `own` is the session itself and the terminals it owns.
 * The daemon refuses a restore only while a session there works; this covers an idle one too.
 */
export function sharedCheckoutRefusal(sessions: readonly SessionView[], place: RestorePlace, own: ReadonlySet<string>): string | null {
  const { checkout } = place
  if (checkout === null) return null
  const shared = sessions.some((other) => !own.has(other.id) && !other.ended && !isShell(other) && other.machine === place.machine && inCheckout(other, checkout))
  return shared ? SHARED_CHECKOUT_REFUSAL : null
}
