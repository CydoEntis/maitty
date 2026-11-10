import type { Dispatch, SetStateAction } from 'react'
import { useQuery } from '@tanstack/react-query'
import { defaultSessionFolder, loadConversations, loadDaemonRestartStatus } from '@/api/daemon'
import { daemonProtocolMismatch } from '@/lib/daemon-connection'

/** The daemon pushes every change (`sessions-changed`); this slow poll only catches a stream that went quiet. */
const SESSION_REFRESH_MS = 15_000

const SECOND_MS = 1000

const MAX_RETRY_EXPONENT = 3

/** Failed reads in a row before the window says the service is unreachable. */
const OFFLINE_AFTER_FAILURES = 2

const BACKOFF_BASE = 2

/** Reads the sessions and the home folder from the service, and says whether it answers and speaks this version. */
export function useServiceReads({ failures, setFailures }: {
  failures: number
  setFailures: Dispatch<SetStateAction<number>>
}) {
  // While maittyd is unreachable, try again after 1, 2, 4, then every 8 seconds.
  const retryDelay = BACKOFF_BASE ** Math.min(MAX_RETRY_EXPONENT, Math.max(0, failures - 1)) * SECOND_MS
  const sessionsQuery = useQuery({
    queryKey: ['conversations'],
    queryFn: async () => {
      try {
        const loaded = await loadConversations()
        setFailures(0)
        return loaded
      } catch (error) {
        setFailures((count) => count + 1)
        throw error
      }
    },
    retry: false,
    refetchInterval: (query) => (query.state.status === 'error' ? retryDelay : SESSION_REFRESH_MS),
  })
  const homeQuery = useQuery({ queryKey: ['home-folder'], queryFn: defaultSessionFolder, staleTime: Infinity, retry: false })
  const home = homeQuery.data ?? ''
  const mismatch = sessionsQuery.isError ? daemonProtocolMismatch(sessionsQuery.error) : null
  // One read that failed while the sessions are known is tried again a second later before anything is called
  // unreachable: a single lost read is not an outage, and marking every session unknown for it is a false alarm.
  // Another version is certain at once.
  const oneLostRead = failures < OFFLINE_AFTER_FAILURES && mismatch === null && sessionsQuery.data !== undefined
  const online = !sessionsQuery.isError || oneLostRead
  // A service newer than this app cannot be restarted into agreement; only updating the app helps.
  const serviceNewer = mismatch !== null && mismatch.daemonProtocol > mismatch.guiProtocol
  const restartStatusQuery = useQuery({ queryKey: ['daemon-restart-status', mismatch?.daemonProtocol], queryFn: loadDaemonRestartStatus, enabled: Boolean(mismatch), retry: false })
  return { sessionsQuery, online, home, retryDelay, serviceNewer, mismatch, restartStatusQuery }
}
