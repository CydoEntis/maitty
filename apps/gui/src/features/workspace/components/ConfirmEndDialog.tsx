import { AlertDialog as AlertDialogPrimitive } from 'radix-ui'

export type EndRequest = { ids: string[]; label: string }

type Props = {
  request: EndRequest | null
  portalContainer: HTMLElement | null
  onCancel: () => void
  onConfirm: () => void
}

/** Ending a session stops its shell and anything running in it, so every path that does asks here first. */
export function ConfirmEndDialog({ request, portalContainer, onCancel, onConfirm }: Props) {
  const count = request?.ids.length ?? 0
  const many = count > 1
  return (
    <AlertDialogPrimitive.Root open={request !== null} onOpenChange={(open) => { if (!open) onCancel() }}>
      <AlertDialogPrimitive.Portal container={portalContainer}>
        <AlertDialogPrimitive.Overlay className="mx-confirm-scrim" />
        <AlertDialogPrimitive.Content className="mx-confirm">
          <AlertDialogPrimitive.Title className="mx-confirm-title">
            {many ? `End ${String(count)} sessions in ${request?.label ?? ''}?` : `End ${request?.label ?? 'this session'}?`}
          </AlertDialogPrimitive.Title>
          <AlertDialogPrimitive.Description className="mx-confirm-text">
            {many ? 'This stops their shells and anything running in them, agents included.' : 'This stops its shell and anything running in it.'} It can&apos;t be undone.
          </AlertDialogPrimitive.Description>
          <div className="mx-confirm-actions">
            <AlertDialogPrimitive.Cancel className="mx-confirm-button">Cancel</AlertDialogPrimitive.Cancel>
            <AlertDialogPrimitive.Action className="mx-confirm-button is-danger" onClick={onConfirm}>{many ? `End ${String(count)} sessions` : 'End session'}</AlertDialogPrimitive.Action>
          </div>
        </AlertDialogPrimitive.Content>
      </AlertDialogPrimitive.Portal>
    </AlertDialogPrimitive.Root>
  )
}
