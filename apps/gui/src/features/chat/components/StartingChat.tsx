import { useEffect, useMemo, useState } from 'react'
import { toast } from '@/lib/toast'
import { WorkingMark } from './WorkingMark'
import { connectAgent, reloadAgentOnDaemon, sendWhenAgentReady } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { NO_EVENTS_HINT_AFTER_MS, noEventsHint, waitingInTerminalHint, type ChatAgent } from '@/features/workspace/model/chatView'
import { keepComposerFocus } from '../keepComposerFocus'
import { buildAttachmentMessage, parseAttachmentMessage } from '../attachmentMessage'
import { SentAttachments } from './Attachments'
import { updateComposer, useComposerState } from '../composerStore'
import { carryDraft, draftKeyFor, entrustFirstMessage, holdFirstMessage, useHeldFirstMessage, useStartChoice } from '../firstMessage'
import { revokePreviews, useAttachmentIntake } from '../useAttachmentIntake'
import { BYPASS_MODE, modeLabel } from '../chatMeta'
import { AGENT_LABEL } from '../modelName'
import type { SessionStatus } from '@/features/workspace/model/session'
import type { ChatComposer } from '../useChatComposer'
import { Composer } from './Composer'
import { ScreenMenuTray } from './QuestionTray'
import { parseScreenMenu } from '../screenMenu'
import { isWorktree, useFolderBranches } from '@/features/workspace/model/newChatHost'
import { abilitiesOf } from '@/features/workspace/model/agentAbilities'

type StartingChatProps = {
  agent: ChatAgent
  /** The session this chat belongs to: its held message and draft are kept under it. */
  sessionId: string
  onBackToTerminal: () => void
  /** The project folder's name and the branch, for the tray under the message box. */
  folder?: string
  branch?: string | null
  /** The folder's full path, to say whether it is a worktree, as the real chat will. */
  root?: string | null
  /** The pane runs in a linked worktree, as the daemon says; it comes before what the folder's list says. */
  worktree?: boolean
  /** What the daemon says the session is doing: Needs before any chat events means a question only the terminal shows. */
  paneStatus?: SessionStatus
  /** The question on the agent's screen, as the daemon read it: shown here with its choices so it can be answered without the terminal. */
  asking?: string
  /** Types into the agent's terminal. */
  typeInto?: (text: string) => Promise<void>
}

/** A held message as the chat will show it: its words, and the files it names as chips, not as the block that carries them. */
function echoed(message: string) {
  const parsed = parseAttachmentMessage(message)
  if (parsed === null) return <p>{message}</p>
  return (
    <>
      {parsed.text !== '' && <p>{parsed.text}</p>}
      <SentAttachments files={parsed.files} />
    </>
  )
}

/**
 * The chat of a session whose agent has not reported in yet (ADR 0003 holds status and conversation
 * to what the agent's own hooks say, so there is no conversation until then). The chat and its message
 * box are there at once. A message sent now is held and goes out when the real chat takes over; a
 * half-typed one is carried across.
 */
export function StartingChat({ agent, sessionId, onBackToTerminal, folder = '', branch = null, root = null, worktree: paneWorktree, paneStatus, asking = '', typeInto }: StartingChatProps) {
  const waiting = paneStatus === 'needs'
  const [branches] = useFolderBranches(root ?? '')
  const menu = waiting && typeInto !== undefined ? parseScreenMenu(asking) : null
  const sendKeys = async (keys: string[]) => {
    for (const key of keys) await typeInto?.(key)
  }
  const [draft, setDraftState] = useState('')
  const held = useHeldFirstMessage(sessionId)
  const started = useStartChoice(sessionId)
  const [late, setLate] = useState(false)
  // Files attached now go out with the message held for the real chat, named by path in it.
  const draftKey = draftKeyFor(sessionId)
  const { attachments } = useComposerState(draftKey)
  const attach = useAttachmentIntake({ conversation: draftKey, blocked: null })
  useEffect(() => () => {
    updateComposer(draftKey, (state) => { revokePreviews(state.attachments); return { ...state, attachments: [] } })
  }, [draftKey])
  const [connecting, setConnecting] = useState(false)
  useEffect(() => {
    const timer = window.setTimeout(() => { setLate(true) }, NO_EVENTS_HINT_AFTER_MS)
    return () => { window.clearTimeout(timer) }
  }, [])

  const connect = () => {
    setConnecting(true)
    connectAgent(agent)
      .then(() => reloadAgentOnDaemon({ id: sessionId }))
      .then(() => { toast.success(`${agent} is connected and restarted to load the hooks.`) })
      .catch((failure: unknown) => { toast.error(errorMessage(failure, `Could not connect ${agent}`)) })
      .finally(() => { setConnecting(false) })
  }

  const composer = useMemo<ChatComposer>(() => ({
    draft,
    setDraft: (text) => { setDraftState(text); carryDraft(sessionId, text) },
    attachments,
    attach,
    readOnly: true,
    attachable: true,
    // The held message is drawn in the conversation, where it will stay, not as a queued row that would jump when the chat takes over.
    queue: [],
    removeQueued: () => undefined,
    editQueued: () => undefined,
    submit: () => {
      const text = draft.trim()
      // One message is held at a time: a second would replace the first before it was ever sent.
      if ((text === '' && attachments.length === 0) || held !== null) return
      const message = buildAttachmentMessage(text, attachments)
      holdFirstMessage(sessionId, message)
      entrustFirstMessage(sessionId, message, sendWhenAgentReady)
      updateComposer(draftKey, (state) => ({ ...state, attachments: [] }))
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
  }), [draft, held, sessionId, attachments, attach, draftKey])

  const abilities = abilitiesOf(agent)
  return (
    <div className="mx-chat is-starting" onClick={keepComposerFocus}>
      {/* It floats over the conversation (chat.css), so it comes and goes without moving anything. A question that can be
          answered here is its own notice (the tray over the message box), so this one stays away. */}
      {menu === null && (late || waiting) && (
        <div className="mx-chat-notices">
          <span role="note" className="is-warn">{waiting ? waitingInTerminalHint(agent) : noEventsHint(agent)}</span>
          <span className="mx-spacer" />
          <button type="button" className="mx-sub-back" onClick={onBackToTerminal}>Open terminal</button>
          {!waiting && <button type="button" className="mx-sub-back" disabled={connecting} onClick={connect}>{connecting ? 'Connecting…' : `Connect ${agent}`}</button>}
        </div>
      )}
      {held === null ? (
        <p className="mx-chat-empty">Type below: it is sent as soon as {agent} is ready.</p>
      ) : (
        // The same list the real chat uses, so the message and the line under it are already where they will stay.
        <div className="mx-chat-scroller min-h-0 flex-1 overflow-y-auto">
          <div className="mx-chat-row"><div className="mx-msg-user is-echo" aria-label="Sending">{echoed(held)}</div></div>
          <div className="mx-chat-row"><div className="mx-working"><WorkingMark agent={agent} /><span>{agent} is starting…</span></div></div>
        </div>
      )}
      <Composer
        agent={agent}
        composer={composer}
        waiting={held !== null}
        question={menu === null ? null : <ScreenMenuTray agent={agent} menu={menu} send={sendKeys} />}
        facts={{
          mode: modeLabel(started?.mode ?? null),
          modeDanger: started?.mode === BYPASS_MODE,
          modeKey: started?.mode ?? null,
          folder,
          branch,
          worktree: paneWorktree ?? isWorktree(branches, root),
          model: null,
          onOpenFolder: null,
          // The chips the real chat will have are drawn now, with what was picked, so nothing is added or removed when it takes over.
          ...(abilities.permissionModes?.switchBy === 'command' && typeInto !== undefined ? { modeLocked: `${AGENT_LABEL[agent]} is starting. Change this once it is ready.` } : {}),
          ...(abilities.changeAgent ? { agentChips: { locked: `${AGENT_LABEL[agent]} is starting. Change this once it is ready.`, ...(started === null ? {} : { start: { model: started.model, reasoning: started.reasoning } }) } } : {}),
        }}
      />
    </div>
  )
}
