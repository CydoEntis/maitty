import { toast } from 'sonner'
import {
  approveRequest,
  loadConversationDiff,
  loadMessages,
} from '@/api/daemon'
import type { Conversation, Message } from '@/data/preview'
import { errorMessage } from '@/lib/error-message'

type ToastActions = {
  open: () => void
  /** Called after an approval was answered from the toast, so lists can refresh. */
  answered: () => void
}

async function findPendingCommandApproval(
  conversation: Conversation,
): Promise<Message | null> {
  const history = await loadMessages(conversation.id, conversation.provider)
  const pending = [...history.messages]
    .reverse()
    .find(
      (message) =>
        message.role === 'approval' &&
        !message.resolved &&
        message.approvalKind === 'Command',
    )
  return pending ?? null
}

/**
 * A session you are not looking at needs you. Approvals and recovery stay until
 * acted on or dismissed; only a choice the provider offered gets a button.
 */
export async function showAttentionToast(
  conversation: Conversation,
  actions: ToastActions,
): Promise<void> {
  const open = { label: 'Open', onClick: actions.open }
  if (conversation.status === 'recovery') {
    toast.error(`${conversation.title} needs recovery`, {
      description: 'Resume it to continue. Nothing is replayed automatically.',
      duration: Infinity,
      closeButton: true,
      action: open,
    })
    return
  }

  const approval = await findPendingCommandApproval(conversation).catch(
    () => null,
  )
  const requestId = approval?.providerRequestId
  if (
    approval &&
    requestId !== undefined &&
    approval.approvalChoices?.includes('Accept')
  ) {
    toast.warning(`${conversation.title} needs your approval`, {
      description: `$ ${approval.detail ?? approval.content}`,
      duration: Infinity,
      closeButton: true,
      action: {
        label: 'Allow once',
        onClick: () => {
          void approveRequest(conversation, requestId, 'Accept')
            .then(() => {
              actions.answered()
              toast.success('Command approved')
            })
            .catch((error: unknown) => {
              toast.error(errorMessage(error, 'Could not approve the command'))
            })
        },
      },
      cancel: open,
    })
    return
  }

  toast.warning(`${conversation.title} needs your attention`, {
    duration: Infinity,
    closeButton: true,
    action: open,
  })
}

/** A session finished; the toast says how much changed once the daemon reports it. */
export async function showFinishedToast(
  conversation: Conversation,
  actions: Pick<ToastActions, 'open'>,
): Promise<void> {
  const summary = await loadConversationDiff(conversation)
    .then((diff) =>
      diff.total_files === 0
        ? undefined
        : `${String(diff.total_files)} ${diff.total_files === 1 ? 'file' : 'files'} changed +${String(diff.additions)} −${String(diff.deletions)}`,
    )
    .catch(() => undefined)
  toast.success(`${conversation.title} is done`, {
    description: summary,
    closeButton: true,
    action: { label: 'Open', onClick: actions.open },
  })
}
