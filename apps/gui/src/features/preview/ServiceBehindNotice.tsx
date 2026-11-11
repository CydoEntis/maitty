import { useEffect, useState } from 'react'
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
}

const plural = (count: number, word: string) => `${String(count)} ${word}${count === 1 ? '' : 's'}`

/**
 * A new release leaves a running service alone, because it may hold sessions. Without this the app says it is up to
 * date while the service still runs the old code, so say so once and offer the restart, which asks first.
 */
export function ServiceBehindNotice({ sessionsQuery }: ServiceBehindNoticeProps) {
  const queryClient = useQueryClient()
  const [dialogOpen, setDialogOpen] = useState(false)
  const [pending, setPending] = useState(false)
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

  useEffect(() => {
    if (!version) return
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
  }, [version, canRestart])

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
