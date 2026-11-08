import { createElement } from 'react'
import { toast } from '@/lib/toast'
import {
  approveRequest,
  loadConversationDiff,
  loadMessages,
} from '@/api/daemon'
import { DiffStat } from '@/components/DiffStat'
import type { Conversation, Message } from '@/data/preview'
import { errorMessage } from '@/lib/error-message'
import { showNotice } from './notify'

const FINISHED_COUNTDOWN_MS = 9000

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
    showNotice(
      {
        kind: 'recovery',
        project: conversation.project,
        title: conversation.title,
        body: 'Resume it to continue. Nothing is replayed automatically.',
        actions: [{ ...open, primary: true }],
      },
      { sticky: true },
    )
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
    showNotice(
      {
        kind: 'needs',
        project: conversation.project,
        title: conversation.title,
        body: createElement(
          'code',
          { className: 'notice-command' },
          `$ ${approval.detail ?? approval.content}`,
        ),
        actions: [
          {
            label: 'Allow once',
            primary: true,
            onClick: () => {
              void approveRequest(conversation, requestId, 'Accept')
                .then(() => {
                  actions.answered()
                })
                .catch((error: unknown) => {
                  toast.error(
                    errorMessage(error, 'Could not approve the command'),
                  )
                })
            },
          },
          open,
        ],
      },
      { sticky: true },
    )
    return
  }

  showNotice(
    {
      kind: 'needs',
      project: conversation.project,
      title: conversation.title,
      body: 'This conversation needs your attention.',
      actions: [{ ...open, primary: true }],
    },
    { sticky: true },
  )
}

/** A session finished; the card says how much changed once the daemon reports it. */
export async function showFinishedToast(
  conversation: Conversation,
  actions: Pick<ToastActions, 'open'> & { review: () => void },
): Promise<void> {
  const diff = await loadConversationDiff(conversation).catch(() => null)
  const changed = diff && diff.total_files > 0 ? diff : null
  showNotice({
    kind: 'finished',
    project: conversation.project,
    title: conversation.title,
    body: changed
      ? createElement(
          'span',
          { className: 'notice-summary' },
          `${String(changed.total_files)} ${changed.total_files === 1 ? 'file' : 'files'} changed `,
          createElement(DiffStat, {
            additions: changed.additions,
            deletions: changed.deletions,
          }),
        )
      : undefined,
    actions: [
      { label: 'Open', onClick: actions.open },
      ...(changed ? [{ label: 'View changes', onClick: actions.review }] : []),
    ],
    countdownMs: FINISHED_COUNTDOWN_MS,
  })
}
