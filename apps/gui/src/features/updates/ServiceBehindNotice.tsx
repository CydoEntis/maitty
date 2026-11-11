import { useEffect, useRef, useState } from 'react'
import { useQuery, useQueryClient, type UseQueryResult } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { toast } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { loadDaemonUpdateStatus, restartStaleDaemon } from '@/api/daemon'
import type { Conversation } from '@/data/preview'

const TOAST_ID = 'service-behind'
const STATUS_KEY = ['daemon-update-status']

type ServiceBehindNoticeProps = {
  sessionsQuery: UseQueryResult<Conversation[]>
  /**
   * Restart the older service without asking when it can be done with nothing lost: its sessions are saved and no
   * agent is working or waiting, so each one comes back as it was (Settings › General › Install updates when idle).
   */
  restartWhenIdle?: boolean
}

/** Whether restarting the service now interrupts nobody: it can be restarted, and it says no agent is mid-turn. */
function restartsUnasked(status: { behind: boolean; canRestart: boolean; activeTurnCount: number | null } | undefined, restartWhenIdle: boolean): boolean {
  return restartWhenIdle && status !== undefined && status.behind && status.canRestart && status.activeTurnCount === 0
}

const plural = (count: number, word: string) => `${String(count)} ${word}${count === 1 ? '' : 's'}`

/**
 * A new release leaves a running service alone, because it may hold sessions. Without this the app says it is up to
 * date while the service still runs the old code, and none of the release's fixes to the service apply. When no
 * agent is mid-turn the service is restarted at once (idle agents come back resumed); otherwise this says so and
 * offers the restart, which asks first.
 */
export function ServiceBehindNotice({ sessionsQuery, restartWhenIdle = false }: ServiceBehindNoticeProps) {
  const queryClient = useQueryClient()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [pending, setPending] = useState(false)
  const [autoFailed, setAutoFailed] = useState(false)
  const statusQuery = useQuery({
    queryKey: STATUS_KEY,
    queryFn: loadDaemonUpdateStatus,
    enabled: sessionsQuery.isSuccess,
    retry: false,
    staleTime: Infinity,
  })
  const status = statusQuery.data
  const version = status?.behind ? status.daemonVersion : null
  const canRestart = Boolean(status?.canRestart)
  const unasked = restartsUnasked(status, restartWhenIdle)
  const sessionCount = status?.sessionCount ?? 0
  const refetchSessions = sessionsQuery.refetch

  // Tried once for each version found behind: a restart that failed leaves the notice and its button.
  const tried = useRef<string | null>(null)
  useEffect(() => {
    if (!version || !unasked || tried.current === version) return
    tried.current = version
    restartStaleDaemon(version).then(
      async () => {
        await refetchSessions()
        await queryClient.invalidateQueries({ queryKey: STATUS_KEY })
        toast.success('Maitty service updated', { description: sessionCount === 0 ? 'It was restarted to finish the update.' : `It was restarted to finish the update. ${plural(sessionCount, 'session')} came back.` })
      },
      (error: unknown) => {
        console.warn('Could not restart the older Maitty service', error)
        setAutoFailed(true)
      },
    )
  }, [version, unasked, sessionCount, refetchSessions, queryClient])

  useEffect(() => {
    // While the unasked restart runs there is nothing to ask.
    if (!version || (unasked && !autoFailed)) return
    toast.warning(`The Maitty service is still on ${version}`, {
      id: TOAST_ID,
      duration: Infinity,
      description: canRestart
        ? 'This app is newer. Restart the service to finish updating.'
        : 'This app is newer, but its sessions are not saved, so Maitty is leaving it running.',
      action: canRestart ? { label: 'Restart…', onClick: () => { setDialogOpen(true) } } : undefined,
    })
    return () => {
      toast.dismiss(TOAST_ID)
    }
  }, [version, canRestart, unasked, autoFailed])

  if (!status || !version) return null

  const description =
    status.activeTurnCount === null
      ? `The service cannot say which agents are working. Restarting may interrupt active work, and output since the last save may be lost.`
      : status.activeTurnCount > 0
        ? `${plural(status.sessionCount, 'session')} are saved and come back. ${plural(status.activeTurnCount, 'agent turn')} in progress will stop, and you can resume them from their rows.`
        : `${plural(status.sessionCount, 'session')} are saved and come back. No agent is working right now.`

  return (
    <Dialog
      open={dialogOpen}
      onOpenChange={(open) => {
        if (!pending) setDialogOpen(open)
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Restart the Maitty service?</DialogTitle>
          <DialogDescription>{`It is on ${version}, and this app is newer. ${description}`}</DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button
            variant="outline"
            disabled={pending}
            onClick={() => {
              setDialogOpen(false)
            }}
          >
            Not now
          </Button>
          <Button
            disabled={!canRestart || pending}
            onClick={() => {
              setPending(true)
              void restartStaleDaemon(version)
                .then(async () => {
                  setDialogOpen(false)
                  toast.dismiss(TOAST_ID)
                  await sessionsQuery.refetch()
                  await queryClient.invalidateQueries({ queryKey: STATUS_KEY })
                })
                .catch((error: unknown) => {
                  toast.error(error instanceof Error ? error.message : typeof error === 'string' ? error : 'Could not restart the Maitty service')
                  void queryClient.invalidateQueries({ queryKey: STATUS_KEY })
                })
                .finally(() => {
                  setPending(false)
                })
            }}
          >
            {pending ? (
              <>
                <LoaderCircle size={14} />
                Restarting…
              </>
            ) : (
              'Restart service'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}
