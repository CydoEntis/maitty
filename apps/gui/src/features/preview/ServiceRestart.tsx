import type { Dispatch, SetStateAction } from 'react'
import type { UseQueryResult } from '@tanstack/react-query'
import { LoaderCircle } from 'lucide-react'
import { toast } from '@/lib/toast'
import { Button } from '@/components/ui/button'
import { Dialog, DialogContent, DialogDescription, DialogFooter, DialogHeader, DialogTitle } from '@/components/ui/dialog'
import { loadDaemonRestartStatus, restartMismatchedDaemon } from '@/api/daemon'
import type { Conversation } from '@/data/preview'

type RestartStatus = Awaited<ReturnType<typeof loadDaemonRestartStatus>>

/** Asks before restarting an older service, saying what the restart does to its sessions. */
export function ServiceRestartDialog({ daemonRestartDialogOpen, daemonRestartPending, setDaemonRestartDialogOpen, daemonRestartStatus, daemonRestartStatusQuery, mismatch, setDaemonRestartPending, sessionsQuery }: {
  daemonRestartDialogOpen: boolean
  daemonRestartPending: boolean
  setDaemonRestartDialogOpen: Dispatch<SetStateAction<boolean>>
  daemonRestartStatus: RestartStatus | undefined
  daemonRestartStatusQuery: UseQueryResult<RestartStatus>
  mismatch: { daemonProtocol: number; guiProtocol: number }
  setDaemonRestartPending: Dispatch<SetStateAction<boolean>>
  sessionsQuery: UseQueryResult<Conversation[]>
}) {
  return (
    <Dialog
      open={daemonRestartDialogOpen}
      onOpenChange={(open) => {
        if (!daemonRestartPending) setDaemonRestartDialogOpen(open)
      }}
    >
      <DialogContent>
        <DialogHeader>
          <DialogTitle>Restart the older Maitty service?</DialogTitle>
          <DialogDescription>
            {daemonRestartStatus?.canRestart
              ? daemonRestartStatus.activeTurnCount === null
                ? `The old service has ${String(daemonRestartStatus.sessionCount)} saved session${daemonRestartStatus.sessionCount === 1 ? '' : 's'} but cannot report which agents are active. Restarting will interrupt active work, and output since the last save may be lost.`
                : daemonRestartStatus.activeTurnCount > 0
                  ? `The old service has ${String(daemonRestartStatus.sessionCount)} session${daemonRestartStatus.sessionCount === 1 ? '' : 's'} and ${String(daemonRestartStatus.activeTurnCount)} active agent turn${daemonRestartStatus.activeTurnCount === 1 ? '' : 's'}. Active turns will stop, and output since the last save may be lost.`
                  : `The old service has ${String(daemonRestartStatus.sessionCount)} saved session${daemonRestartStatus.sessionCount === 1 ? '' : 's'} and no active agent turns. Restarting lets this GUI reconnect to the updated service.`
              : daemonRestartStatusQuery.isError
                ? 'Maitty could not confirm the service status. No restart has happened; retry the check or reconnect.'
                : daemonRestartStatusQuery.isPending
                  ? 'Checking whether the local service can be safely replaced.'
                  : mismatch.daemonProtocol > mismatch.guiProtocol
                    ? 'This service is newer than the one bundled with this GUI. Update the GUI before reconnecting; Maitty will not replace a newer service.'
                    : daemonRestartStatus?.sessionCount
                      ? `This older service owns ${String(daemonRestartStatus.sessionCount)} sessions but is not saving them, so a restart would lose them. Maitty will leave it running.`
                      : daemonRestartStatus?.activeTurnCount === null
                        ? 'Maitty could not verify that the older service is idle, so it will leave the service running.'
                        : 'This older service still reports active work, so Maitty will leave it running.'}
          </DialogDescription>
        </DialogHeader>
        <DialogFooter>
          <Button
            variant="outline"
            disabled={daemonRestartPending}
            onClick={() => {
              setDaemonRestartDialogOpen(false)
            }}
          >
            Cancel
          </Button>
          <Button
            disabled={!daemonRestartStatus?.canRestart || daemonRestartPending}
            onClick={() => {
              if (!daemonRestartStatus?.canRestart) return
              setDaemonRestartPending(true)
              void restartMismatchedDaemon(mismatch.daemonProtocol)
                .then(async () => {
                  setDaemonRestartDialogOpen(false)
                  await sessionsQuery.refetch()
                })
                .catch((error: unknown) => {
                  toast.error(
                    error instanceof Error
                      ? error.message
                      : typeof error === 'string'
                        ? error
                        : 'Could not restart the Maitty service',
                  )
                  void daemonRestartStatusQuery.refetch()
                })
                .finally(() => {
                  setDaemonRestartPending(false)
                })
            }}
          >
            {daemonRestartPending ? (
              <>
                <LoaderCircle size={14} />
                Restarting…
              </>
            ) : (
              'Restart and reconnect'
            )}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  )
}

/** The button that opens the restart dialog, labelled with what a restart would do. */
export function ServiceRestartButton({ daemonRestartStatusQuery, daemonRestartPending, setDaemonRestartDialogOpen }: {
  daemonRestartStatusQuery: UseQueryResult<RestartStatus>
  daemonRestartPending: boolean
  setDaemonRestartDialogOpen: Dispatch<SetStateAction<boolean>>
}) {
  return (
    <Button
      variant="outline"
      size="sm"
      disabled={
        daemonRestartStatusQuery.isPending ||
        daemonRestartPending ||
        Boolean(
          daemonRestartStatusQuery.data &&
          !daemonRestartStatusQuery.data.canRestart,
        )
      }
      onClick={() => {
        if (daemonRestartStatusQuery.isError)
          void daemonRestartStatusQuery.refetch()
        else setDaemonRestartDialogOpen(true)
      }}
    >
      {daemonRestartStatusQuery.isPending
        ? 'Checking service…'
        : daemonRestartStatusQuery.isError
          ? 'Retry restart check'
          : daemonRestartStatusQuery.data.canRestart
            ? 'Restart service'
            : daemonRestartStatusQuery.data.sessionCount > 0 ||
                daemonRestartStatusQuery.data.activeTurnCount === null
              ? 'Service in use'
              : 'Restart unavailable'}
    </Button>
  )
}
