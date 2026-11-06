import { useEffect, useMemo, useState } from 'react'
import { NO_EVENTS_HINT_AFTER_MS, noEventsHint, type ChatAgent } from '@/features/workspace/model/chatView'
import { carryDraft, holdFirstMessage, releaseFirstMessage, useHeldFirstMessage } from '../firstMessage'
import type { AttachmentIntake } from '../useAttachmentIntake'
import type { ChatComposer } from '../useChatComposer'
import { Composer } from './Composer'

/** The id of the held message in the composer's queue list; the real queue's ids are operation numbers. */
const HELD_ID = -1

const NO_ATTACH: AttachmentIntake = { pick: () => undefined, addPaths: () => undefined, addImage: () => undefined, remove: () => undefined }

type StartingChatProps = {
  agent: ChatAgent
  /** The session this chat belongs to: its held message and draft are kept under it. */
  sessionId: string
  onBackToTerminal: () => void
}

/**
 * The chat of a session whose agent has not reported in yet (ADR 0003 holds status and conversation
 * to what the agent's own hooks say, so there is no conversation until then). The chat and its message
 * box are there at once. A message sent now is held and goes out when the real chat takes over; a
 * half-typed one is carried across.
 */
export function StartingChat({ agent, sessionId, onBackToTerminal }: StartingChatProps) {
  const [draft, setDraftState] = useState('')
  const held = useHeldFirstMessage(sessionId)
  const [late, setLate] = useState(false)
  useEffect(() => {
    const timer = window.setTimeout(() => { setLate(true) }, NO_EVENTS_HINT_AFTER_MS)
    return () => { window.clearTimeout(timer) }
  }, [])

  const composer = useMemo<ChatComposer>(() => ({
    draft,
    setDraft: (text) => { setDraftState(text); carryDraft(sessionId, text) },
    attachments: [],
    attach: NO_ATTACH,
    // There is nothing to attach to yet, so no paperclip.
    readOnly: true,
    queue: held === null ? [] : [{ id: HELD_ID, text: held }],
    removeQueued: () => { releaseFirstMessage(sessionId) },
    submit: () => {
      const text = draft.trim()
      if (text === '') return
      holdFirstMessage(sessionId, text)
      setDraftState('')
      carryDraft(sessionId, '')
    },
    stop: () => undefined,
    running: false,
    sending: false,
    disabledReason: null,
    error: null,
    sendText: () => Promise.resolve(false),
    echo: null,
  }), [draft, held, sessionId])

  return (
    <div className="flex min-h-0 flex-1 flex-col">
      <div className="flex min-h-8 shrink-0 items-center gap-2 border-b border-line2 px-4 text-[12px] text-fg2">
        <span className="font-semibold text-fg">Starting {agent}…</span>
        {late && <span role="note" className="text-[var(--warn)]">{noEventsHint(agent)}</span>}
        <button type="button" className="ml-auto text-dim underline-offset-2 hover:text-fg hover:underline" onClick={onBackToTerminal}>Back to terminal</button>
      </div>
      <p className="m-0 flex flex-1 items-center justify-center px-6 text-center text-[13px] text-dim">
        No messages yet. Type below: it is sent as soon as {agent} is ready.
      </p>
      <Composer agent={agent} composer={composer} />
    </div>
  )
}
