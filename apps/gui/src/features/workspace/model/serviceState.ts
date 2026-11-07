/** How many failed tries the window makes before it calls the service down. */
export const RECONNECT_TRIES = 5

/**
 * What the window tells about maittyd:
 * - `ok`: connected;
 * - `reconnecting`: a try failed, more are coming ("Try N of 5");
 * - `down`: every try failed;
 * - `update`: the running service is another version and must be restarted (never a connection problem).
 */
export type ServiceState = 'ok' | 'reconnecting' | 'down' | 'update'

export function serviceStateOf({ online, failures, mismatch }: { online: boolean; failures: number; mismatch: boolean }): ServiceState {
  if (online) return 'ok'
  if (mismatch) return 'update'
  return failures >= RECONNECT_TRIES ? 'down' : 'reconnecting'
}

/** The try the bar names: the one that just failed, from 1 to the last. */
export function tryNumber(failures: number): number {
  return Math.min(RECONNECT_TRIES, Math.max(1, failures))
}
