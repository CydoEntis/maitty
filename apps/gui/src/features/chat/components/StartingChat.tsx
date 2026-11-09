import { useEffect, useMemo, useState } from 'react'
import { toast } from '@/lib/toast'
import { Spinner } from '@/features/workspace/components/glyphs'
import { connectAgent } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { NO_EVENTS_HINT_AFTER_MS, noEventsHint, type ChatAgent } from '@/features/workspace/model/chatView'
import { keepComposerFocus } from '../keepComposerFocus'
import { carryDraft, holdFirstMessage, useHeldFirstMessage, useStartChoice } from '../firstMessage'
import { BYPASS_MODE, modeLabel } from '../chatMeta'
import { AGENT_LABEL } from '../modelName'
import type { AttachmentIntake } from '../useAttachmentIntake'
import type { ChatComposer } from '../useChatComposer'
import { Composer } from './Composer'

const NO_ATTACH: AttachmentIntake = { pick: () => undefined, addPaths: () => undefined, addImage: () => undefined, remove: () => undefined }

type StartingChatProps = {
  agent: ChatAgent
  /** The session this chat belongs to: its held message and draft are kept under it. */
  sessionId: string
  onBackToTerminal: () => void
  /** The project folder's name and the branch, for the tray under the message box. */
  folder?: string
  branch?: string | null
}

/**
 * The chat of a session whose agent has not reported in yet (ADR 0003 holds status and conversation
 * to what the agent's own hooks say, so there is no conversation until then). The chat and its message
 * box are there at once. A message sent now is held and goes out when the real chat takes over; a
 * half-typed one is carried across.
 */
export function StartingChat({ agent, sessionId, onBackToTerminal, folder = '', branch = null }: StartingChatProps) {
  const [draft, setDraftState] = useState('')
  const held = useHeldFirstMessage(sessionId)
  const started = useStartChoice(sessionId)
  const [late, setLate] = useState(false)
  const [connecting, setConnecting] = useState(false)
  useEffect(() => {
    const timer = window.setTimeout(() => { setLate(true) }, NO_EVENTS_HINT_AFTER_MS)
    return () => { window.clearTimeout(timer) }
  }, [])

  const connect = () => {
    setConnecting(true)
    connectAgent(agent)
      .then(() => { toast.success(`${agent} is connected. Restart it in the terminal to load the hooks.`) })
      .catch((failure: unknown) => { toast.error(errorMessage(failure, `Could not connect ${agent}`)) })
      .finally(() => { setConnecting(false) })
  }

  const composer = useMemo<ChatComposer>(() => ({
    draft,
    setDraft: (text) => { setDraftState(text); carryDraft(sessionId, text) },
    attachments: [],
    attach: NO_ATTACH,
    // There is nothing to attach to yet, so no paperclip.
    readOnly: true,
    // The held message is drawn in the conversation, where it will stay, not as a queued row that would jump when the chat takes over.
    queue: [],
    removeQueued: () => undefined,
    submit: () => {
      const text = draft.trim()
      // One message is held at a time: a second would replace the first before it was ever sent.
      if (text === '' || held !== null) return
      holdFirstMessage(sessionId, text)
      setDraftState('')
      carryDraft(sessionId, '')
    },
    stop: () => undefined,
    running: false,
    sending: false,
    disabledReason: null,
    ready: false,
    error: null,
    sendText: () => Promise.resolve(false),
    echo: null,
  }), [draft, held, sessionId])

  return (
    <div className="mx-chat is-starting" onClick={keepComposerFocus}>
      {/* It floats over the conversation (chat.css), so it comes and goes without moving anything. */}
      {late && (
        <div className="mx-chat-notices">
          <span role="note" className="is-warn">{noEventsHint(agent)}</span>
          <span className="mx-spacer" />
          <button type="button" className="mx-sub-back" disabled={connecting} onClick={connect}>{connecting ? 'Connecting…' : `Connect ${agent}`}</button>
          <button type="button" className="mx-sub-back" onClick={onBackToTerminal}>Back to terminal</button>
        </div>
      )}
      {held === null ? (
        <p className="mx-chat-empty">Type below: it is sent as soon as {agent} is ready.</p>
      ) : (
        // The same list the real chat uses, so the message and the line under it are already where they will stay.
        <div className="mx-chat-scroller min-h-0 flex-1 overflow-y-auto">
          <div className="mx-chat-row"><div className="mx-msg-user is-echo" aria-label="Sending"><p>{held}</p></div></div>
          <div className="mx-chat-row"><div className="mx-working"><Spinner /><span>{agent} is starting…</span></div></div>
        </div>
      )}
      <Composer
        agent={agent}
        composer={composer}
        waiting={held !== null}
        facts={{
          mode: modeLabel(started?.mode ?? null),
          modeDanger: started?.mode === BYPASS_MODE,
          modeKey: started?.mode ?? null,
          subagents: 0,
          folder,
          branch,
          model: null,
          onOpenFolder: null,
          // The chips the real chat will have are drawn now, with what was picked, so nothing is added or removed when it takes over.
          ...(agent === 'claude' ? { agentChips: { locked: `${AGENT_LABEL[agent]} is starting. Change this once it is ready.`, ...(started === null ? {} : { start: { model: started.model, reasoning: started.reasoning } }) } } : {}),
        }}
      />
    </div>
  )
}
