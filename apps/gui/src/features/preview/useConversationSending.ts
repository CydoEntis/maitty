import type { QueryClient, UseQueryResult } from '@tanstack/react-query'
import type { Dispatch, KeyboardEvent, SetStateAction } from 'react'
import { toast } from '@/lib/toast'
import { answerUserInput as answerDaemonInput, approveRequest, classifySubmissionFailure, loadMessages, resumeConversation as resumeDaemonConversation, submitMessage } from '@/api/daemon'
import type { Conversation, Message, SessionStatus } from '@/data/preview'
import { PREVIEW_APPROVAL_DELAY_MS, PREVIEW_RESPONSE_DELAY_MS, type SendFailure } from './shared'

/** What the open conversation's composer and approval dock do: send, check a delivery, resume, answer. */
export function useConversationSending({ setDrafts, setSendFailures, setMessagesById, setDenyQueuedFor, daemonDisconnected, deliveryNotice, draft, desktop, historyQuery, queryClient, setStatus, selected, sendFailure, setRecoveryPendingId, selectConversation, pendingApproval, denyDecision }: {
  setDrafts: Dispatch<SetStateAction<Record<string, string>>>
  setSendFailures: Dispatch<SetStateAction<Record<string, SendFailure>>>
  setMessagesById: Dispatch<SetStateAction<Record<string, Message[]>>>
  setDenyQueuedFor: Dispatch<SetStateAction<string | null>>
  daemonDisconnected: boolean
  deliveryNotice: "unknown" | "not-delivered" | null
  draft: string
  desktop: boolean
  historyQuery: UseQueryResult<Awaited<ReturnType<typeof loadMessages>>>
  queryClient: QueryClient
  setStatus: (conversationId: string, status: SessionStatus) => void
  selected: Conversation | undefined
  sendFailure: SendFailure | null
  setRecoveryPendingId: Dispatch<SetStateAction<string | null>>
  selectConversation: (conversationId: string) => void
  pendingApproval: Message | undefined
  denyDecision: "Cancel" | "Decline" | null
}) {
  function updateDraft(value: string) {
    if (!selected) return
    const activeConversation = selected
    setDrafts((current) => ({ ...current, [activeConversation.id]: value }))
  }

  function clearSendFailure(conversationId: string) {
    setSendFailures((current) =>
      Object.fromEntries(
        Object.entries(current).filter(([id]) => id !== conversationId),
      ),
    )
  }

  function pushMessage(conversationId: string, message: Message) {
    setMessagesById((current) => ({
      ...current,
      [conversationId]: [...(current[conversationId] ?? []), message],
    }))
  }

  function sendMessage() {
    if (!selected) return
    const activeConversation = selected
    // A queued deny-and-send is spent the moment a send is attempted.
    setDenyQueuedFor(null)
    if (daemonDisconnected) return
    if (deliveryNotice === 'unknown') return
    const text = draft.trim()
    if (
      !text ||
      activeConversation.status === 'working' ||
      activeConversation.status === 'needs-you' ||
      activeConversation.status === 'unknown'
    )
      return

    const conversationId = activeConversation.id
    if (desktop) {
      const operation = historyQuery.data?.nextOperation ?? 1
      const conversation = activeConversation
      void submitMessage(conversation, operation, text)
        .then(async () => {
          setDrafts((current) => ({ ...current, [conversationId]: '' }))
          clearSendFailure(conversationId)
          await refreshAfterSubmission(conversationId)
        })
        .catch(async () => {
          let outcome: ReturnType<typeof classifySubmissionFailure> = 'unknown'
          try {
            const latest = await queryClient.query({
              queryKey: ['conversation-history', conversationId],
              queryFn: () =>
                loadMessages(conversationId, conversation.provider),
              staleTime: 0,
            })
            outcome = classifySubmissionFailure(
              latest,
              conversation.generation ?? -1,
              operation,
            )
          } catch {
            // A failed receipt read cannot prove whether the provider received the prompt.
          }
          if (outcome === 'delivered') {
            setDrafts((current) => ({ ...current, [conversationId]: '' }))
            clearSendFailure(conversationId)
            await refreshAfterSubmission(conversationId)
            toast.message('Message was received; refreshing conversation')
            return
          }
          setSendFailures((current) => ({
            ...current,
            [conversationId]: { kind: outcome, operation },
          }))
          toast.error(
            outcome === 'unknown'
              ? 'Delivery is unknown. Check the native terminal before continuing.'
              : 'Message was not delivered. Your draft is safe to retry.',
          )
        })
      return
    }
    updateDraft('')
    pushMessage(conversationId, {
      id: crypto.randomUUID(),
      role: 'user',
      content: text,
      time: 'now',
    })
    setStatus(conversationId, 'working')

    window.setTimeout(() => {
      pushMessage(conversationId, {
        id: crypto.randomUUID(),
        role: 'assistant',
        content:
          'I’ve picked that up. This preview uses sample data; the real conversation connection is still being wired in.',
        time: 'now',
      })
      setStatus(conversationId, 'done')
    }, PREVIEW_RESPONSE_DELAY_MS)
  }

  async function checkSubmissionDelivery() {
    if (!selected) return
    const uncertainReceipt =
      historyQuery.data?.receipt?.delivery === 'SubmissionUncertain'
        ? historyQuery.data.receipt
        : null
    const operation = sendFailure?.operation ?? uncertainReceipt?.operation
    if (operation === undefined || selected.generation === undefined) return
    try {
      const latest = await loadMessages(selected.id, selected.provider)
      queryClient.setQueryData(['conversation-history', selected.id], latest)
      const outcome = classifySubmissionFailure(
        latest,
        selected.generation,
        operation,
      )
      if (outcome === 'delivered') {
        setDrafts((current) => ({ ...current, [selected.id]: '' }))
        clearSendFailure(selected.id)
        toast.message('The daemon confirmed this message was received')
      } else if (outcome === 'not-delivered') {
        setSendFailures((current) => ({
          ...current,
          [selected.id]: { kind: 'not-delivered', operation },
        }))
        toast.message(
          'The daemon confirmed this message was not delivered. Your draft is safe to retry.',
        )
      } else {
        setSendFailures((current) => ({
          ...current,
          [selected.id]: { kind: 'unknown', operation },
        }))
        toast.warning(
          'Delivery is still unknown. Check the native terminal before continuing.',
        )
      }
    } catch (error: unknown) {
      toast.error(
        error instanceof Error
          ? `Could not check delivery: ${error.message}`
          : 'Could not check delivery',
      )
    }
  }

  function resumeSelectedConversation() {
    if (
      !selected ||
      !desktop ||
      daemonDisconnected ||
      selected.execution !== 'Structured' ||
      selected.status !== 'recovery'
    )
      return
    const conversation = selected
    setRecoveryPendingId(conversation.id)
    void resumeDaemonConversation(conversation)
      .then(async (resumed) => {
        selectConversation(String(resumed.id))
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['conversations'] }),
          queryClient.invalidateQueries({
            queryKey: ['conversation-history', conversation.id],
          }),
        ])
      })
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error
            ? error.message
            : 'Could not resume this conversation',
        )
      })
      .finally(() => {
        setRecoveryPendingId(null)
      })
  }

  async function refreshAfterSubmission(conversationId: string) {
    await Promise.all([
      queryClient.invalidateQueries({ queryKey: ['conversations'] }),
      queryClient.invalidateQueries({
        queryKey: ['conversation-history', conversationId],
      }),
    ])
  }

  function resolveApproval(
    message: Message,
    decision: 'Accept' | 'AcceptForSession' | 'Decline' | 'Cancel',
  ) {
    if (!selected) return
    const activeConversation = selected
    if (daemonDisconnected) return
    if (desktop) {
      if (message.providerRequestId === undefined) return
      void approveRequest(
        activeConversation,
        message.providerRequestId,
        decision,
      )
        .then(async () => {
          await Promise.all([
            queryClient.invalidateQueries({ queryKey: ['conversations'] }),
            queryClient.invalidateQueries({
              queryKey: ['conversation-history', activeConversation.id],
            }),
          ])
        })
        .catch((error: unknown) => {
          toast.error(
            error instanceof Error
              ? error.message
              : 'Could not answer approval request',
          )
        })
      return
    }
    const allow = decision === 'Accept' || decision === 'AcceptForSession'
    setMessagesById((current) => ({
      ...current,
      [activeConversation.id]: (current[activeConversation.id] ?? []).map(
        (item) => (item.id === message.id ? { ...item, resolved: true } : item),
      ),
    }))
    pushMessage(activeConversation.id, {
      id: crypto.randomUUID(),
      role: 'assistant',
      content: allow
        ? 'Approved once. The test command is running in this preview.'
        : 'You declined the command. The agent did not run it.',
      time: 'now',
    })
    setStatus(activeConversation.id, allow ? 'working' : 'idle')
    if (allow) {
      window.setTimeout(() => {
        setStatus(activeConversation.id, 'done')
      }, PREVIEW_APPROVAL_DELAY_MS)
    }
  }

  function answerInput(message: Message, answers: Record<string, string[]>) {
    if (!selected) return
    const activeConversation = selected
    if (daemonDisconnected) return
    if (message.providerRequestId === undefined) return
    void answerDaemonInput(
      activeConversation,
      message.providerRequestId,
      answers,
    )
      .then(async () => {
        await Promise.all([
          queryClient.invalidateQueries({ queryKey: ['conversations'] }),
          queryClient.invalidateQueries({
            queryKey: ['conversation-history', activeConversation.id],
          }),
        ])
        toast.success(`Answers sent to ${activeConversation.provider}`)
      })
      .catch((error: unknown) => {
        toast.error(
          error instanceof Error ? error.message : 'Could not send answers',
        )
      })
  }

  function denyAndSend() {
    if (!selected) return
    const activeConversation = selected
    if (!pendingApproval || !denyDecision || !draft.trim()) return
    setDenyQueuedFor(activeConversation.id)
    resolveApproval(pendingApproval, denyDecision)
  }

  function handleComposerKeyDown(event: KeyboardEvent<HTMLTextAreaElement>) {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      if (denyDecision) denyAndSend()
      else sendMessage()
    }
  }
  return { sendMessage, resumeSelectedConversation, resolveApproval, answerInput, checkSubmissionDelivery, clearSendFailure, updateDraft, handleComposerKeyDown, denyAndSend }
}
