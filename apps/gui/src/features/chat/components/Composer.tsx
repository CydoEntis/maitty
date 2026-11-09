import { useEffect, useRef, type ClipboardEvent, type KeyboardEvent, type ReactNode } from 'react'
import { toast } from '@/lib/toast'
import { pickFromClipboard, unsupportedMessage } from '@/features/terminal/clipboardImage'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import { AppIcon, Icon, Spinner } from '@/features/workspace/components/glyphs'
import { registerComposerDrop } from '../composerDrop'
import { useComposerDraft } from '../composerStore'
import type { ChatComposer } from '../useChatComposer'
import { defaultChoice, useAgentChoice, type AgentChoice } from '../agentChoice'
import { DraftAttachments } from './Attachments'
import { AgentPicker } from './AgentPicker'
import { modelName } from '../modelName'
import { BYPASS_MODE, modeLabel, PICKABLE_MODES } from '../chatMeta'
import { DropdownMenu, DropdownMenuContent, DropdownMenuRadioGroup, DropdownMenuRadioItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'

/** A key with nothing stored under it, for a composer whose view holds its own draft. */
const NO_CONVERSATION = -1

/** What the settings tray under the box says about the session. */
export type ComposerFacts = {
  /** The permission mode in words, from the agent's own file; null when it has not said. */
  mode: string | null
  /** The mode asks about nothing, so it is drawn in the danger colour. */
  modeDanger: boolean
  subagents: number
  folder: string
  branch: string | null
  /** The model the agent's own file says it uses; null when it has not said. */
  model: string | null
  onOpenFolder: (() => void) | null
  /** For a chat that follows the agent's terminal and can type into it: send the agent one of its own commands (`/model x`). */
  onCommand?: ((text: string) => void) | undefined
  /** The same, for the permission mode: moves the agent to the picked mode (by its Shift+Tab). */
  onPickMode?: ((mode: string) => void) | undefined
  /** The mode as the agent's file or screen wrote it, to tick it in the dropdown. */
  modeKey?: string | null | undefined
  /** Bypass permissions can be picked: this session started with Claude allowing it. */
  bypassAllowed?: boolean | undefined
  /**
   * Set for an agent whose model and mode can be changed (Claude): both chips are always drawn, and when they cannot be used right
   * now (the agent is starting or working) they stay where they are, greyed out with `locked` as the reason, so the box never
   * rearranges. `start` is what a chat with no conversation yet was started with.
   */
  agentChips?: { readonly locked: string | null; readonly start?: AgentChoice } | undefined
}

type Props = {
  agent: ChatAgent
  composer: ChatComposer
  /** The chat's conversation number; with it the composer offers the agent, model and reasoning picker. */
  conversation?: number
  /** The question tray: what the agent asks, slid up from behind the box. Null when it asks nothing. */
  question?: ReactNode
  facts: ComposerFacts
  /** A message went in and the agent has not taken it up yet: the send button shows it is working and sends nothing more. */
  waiting?: boolean
}

const NO_CHANGE = () => undefined

/**
 * The message box, in three layers (README: message box): the question tray when the agent asks, the
 * box (text, attach, model picker, send), and the settings tray under it. Enter sends, Shift+Enter
 * adds a line, Esc interrupts the running turn. While the agent works, sending queues the message;
 * while a request waits for an answer, or the daemon is offline, the box is off and says why.
 */
export function Composer({ agent, composer, conversation, question = null, facts, waiting = false }: Props) {
  const { setDraft, attachments, attach, readOnly, queue, removeQueued, submit, stop, running, sending, disabledReason, error } = composer
  const storedDraft = useComposerDraft(composer.conversationKey ?? NO_CONVERSATION)
  const draft = composer.draft ?? storedDraft
  const off = disabledReason !== null
  const box = useRef<HTMLDivElement>(null)
  // The desktop shell takes file drops for itself, so the composer asks to be told of the ones that land on it.
  const { addPaths } = attach
  useEffect(() => {
    const element = box.current
    return element === null ? undefined : registerComposerDrop(element, addPaths)
  }, [addPaths])

  // Text first, as in the terminal: a clipboard with any text pastes the text; only a bare image is attached.
  const onPaste = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const pick = pickFromClipboard(event.clipboardData)
    if (pick.kind === 'text' || pick.kind === 'none') return
    event.preventDefault()
    if (pick.kind === 'image') attach.addImage(pick.image)
    else toast.error(unsupportedMessage(pick.type))
  }
  const attachLabel = off ? `Attach files: ${disabledReason}` : 'Attach files'
  const placeholder = disabledReason ?? (running ? `Queue a message for when ${agent} finishes` : `Message ${agent}`)
  const empty = draft.trim() === '' && attachments.length === 0
  // With nothing typed while a turn runs, the round button stops the agent instead of sending.
  const stopping = running && empty && !off
  const busy = sending || waiting

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    // Enter while an input method is composing text confirms the text; it is not a send.
    if (event.nativeEvent.isComposing) return
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault()
      submit()
    } else if (event.key === 'Escape' && running) {
      event.preventDefault()
      stop()
    }
  }

  return (
    <div ref={box} className="mx-composer">
      <div className="mx-composer-column">
        {queue.length > 0 && (
          <ul className="mx-queue" aria-label="Queued messages">
            {queue.map((message) => (
              <li key={message.id} className="mx-queued">
                <span className="mx-queued-text">{message.text}</span>
                <span className="mx-queued-note">
                  Queued · sent when {agent} finishes this turn, even if you close the window
                  <button type="button" className="mx-queued-remove" onClick={() => { removeQueued(message.id) }}><Icon name="close" size={10} />Remove</button>
                </span>
              </li>
            ))}
          </ul>
        )}
        {question}
        <div className={`mx-box${off ? ' is-locked' : ''}`}>
          <DraftAttachments attachments={attachments} onRemove={attach.remove} />
          <textarea
            aria-label={`Message ${agent}`}
            rows={2}
            value={draft}
            disabled={off}
            placeholder={placeholder}
            className="mx-box-text"
            onChange={(event) => { setDraft(event.target.value) }}
            onKeyDown={onKeyDown}
            onPaste={onPaste}
          />
          <div className="mx-box-row">
            {(composer.attachable ?? !readOnly) && (
              // aria-disabled, not disabled: a disabled button gets no focus or tooltip, and the reason is the point.
              <button type="button" className="mx-box-attach" aria-label={attachLabel} title={attachLabel} aria-disabled={off} onClick={() => { if (!off) attach.pick() }}>
                <Icon name="plus" size={16} />
              </button>
            )}
            <span className="mx-spacer" />
            {facts.agentChips !== undefined ? (
              conversation !== undefined
                ? <ModelPicker agent={agent} conversation={conversation} model={facts.model} live={readOnly ? facts.onCommand : undefined} locked={facts.agentChips.locked} />
                : <AgentPicker agent={agent} choice={facts.agentChips.start ?? defaultChoice()} onChange={NO_CHANGE} locked={facts.agentChips.locked ?? undefined} />
            ) : (
              <>
                {conversation !== undefined && (!readOnly || facts.onCommand !== undefined) && <ModelPicker agent={agent} conversation={conversation} model={facts.model} live={readOnly ? facts.onCommand : undefined} locked={null} />}
                {readOnly && facts.onCommand === undefined && <ModelLabel agent={agent} model={facts.model} />}
              </>
            )}
            {stopping ? (
              <button type="button" className="mx-send is-stop is-working" aria-label={`Stop ${agent}`} title={`${agent} is working. Stop it (Esc)`} onClick={stop}><Spinner /><span className="mx-stop-square" aria-hidden="true" /></button>
            ) : busy ? (
              <button type="button" className="mx-send is-off is-busy" aria-label={`${agent} is working`} aria-busy="true" title={`${agent} is working`} disabled><Spinner /></button>
            ) : (
              <button type="button" className={`mx-send${off || empty ? ' is-off' : ''}`} aria-label={running ? 'Queue message' : 'Send message'} disabled={off || sending || empty} onClick={submit}>
                <Icon name="arrowUp" size={15} strokeWidth={2.2} />
              </button>
            )}
          </div>
        </div>
        <SettingsTray facts={facts} />
        {error !== null && <p role="alert" className="mx-composer-error">{error}. Your message is kept; send it again to retry.</p>}
      </div>
    </div>
  )
}

function ModeIcon() {
  return <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 3 5 6v6c0 4.4 3 7.6 7 9 4-1.4 7-4.6 7-9V6z" /></svg>
}

/** The darker tray under the box: how the agent is set up, its subagents, its folder and branch. */
function SettingsTray({ facts }: { facts: ComposerFacts }) {
  return (
    <div className="mx-settings-tray">
      {facts.agentChips !== undefined && (facts.agentChips.locked !== null || facts.onPickMode === undefined) ? (
        // aria-disabled, not disabled: a disabled button gets no tooltip, and the reason is the point.
        <button type="button" aria-disabled="true" className={`mx-tray-fact is-locked${facts.modeDanger ? ' is-danger' : ''}`} title={facts.agentChips.locked ?? 'Not available for this chat'} aria-label={`Permission mode: ${facts.mode ?? 'not reported yet'}. ${facts.agentChips.locked ?? 'Not available for this chat'}`}>
          <ModeIcon />
          {facts.mode ?? 'Permissions'}
          <Icon name="chevronDown" size={10} />
        </button>
      ) : facts.onPickMode === undefined ? (
        <span className={`mx-tray-fact${facts.modeDanger ? ' is-danger' : ''}`} title={facts.mode === null ? 'Not reported by the agent yet' : "The permission mode, from the agent's own file. Cycle it in the terminal with Shift+Tab."}>
          <ModeIcon />
          {facts.mode ?? 'Permissions'}
        </span>
      ) : (
        <DropdownMenu>
          <DropdownMenuTrigger asChild>
            <button type="button" className={`mx-tray-fact is-button${facts.modeDanger ? ' is-danger' : ''}`} title="Permission mode" aria-label={`Permission mode: ${facts.mode ?? 'not reported yet'}. Pick another.`}>
              <ModeIcon />
              {facts.mode ?? 'Permissions'}
              <Icon name="chevronDown" size={10} />
            </button>
          </DropdownMenuTrigger>
          <DropdownMenuContent side="top" align="start" aria-label="Permission mode" className="w-[240px] text-fg">
            <DropdownMenuRadioGroup value={facts.modeKey ?? ''} onValueChange={facts.onPickMode}>
              {PICKABLE_MODES.map((key) => (
                <DropdownMenuRadioItem
                  key={key}
                  value={key}
                  disabled={key === BYPASS_MODE && facts.bypassAllowed !== true}
                  title={key === BYPASS_MODE && facts.bypassAllowed !== true ? 'Claude only allows this when it starts with Yolo mode on (Settings, General). This session started without it: turn the setting on and start a new session.' : undefined}
                  className="text-[12.5px] text-fg focus:bg-hover focus:text-fg"
                >
                  {modeLabel(key)}
                  {key === BYPASS_MODE && <span className="ml-auto pl-2 text-[11px] text-dim">{facts.bypassAllowed === true ? 'asks nothing' : 'needs a new session'}</span>}
                </DropdownMenuRadioItem>
              ))}
            </DropdownMenuRadioGroup>
          </DropdownMenuContent>
        </DropdownMenu>
      )}
      <span className="mx-tray-fact" title="Subagents this session started">
        <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M16 21v-2a4 4 0 0 0-4-4H6a4 4 0 0 0-4 4v2" /><circle cx="9" cy="7" r="4" /><path d="M22 21v-2a4 4 0 0 0-3-3.87" /><path d="M16 3.13a4 4 0 0 1 0 7.75" /></svg>
        {facts.subagents > 0 ? `Subagents · ${String(facts.subagents)}` : 'Subagents'}
      </span>
      {facts.onOpenFolder === null
        ? <span className="mx-tray-fact"><Icon name="folder" />{facts.folder}</span>
        : <button type="button" className="mx-tray-fact is-button" title="Show its files" onClick={facts.onOpenFolder}><Icon name="folder" />{facts.folder}</button>}
      <span className="mx-spacer" />
      {facts.branch !== null && <span className="mx-tray-fact is-branch"><Icon name="branch" size={13} strokeWidth={1.9} stroke="currentColor" />{facts.branch}</span>}
    </div>
  )
}

/** The model a chat that follows a terminal agent runs; it is changed in the terminal, so it is only said. Drawn from the start, with just the agent's name until the agent says its model, so it is never added later. */
function ModelLabel({ agent, model }: { agent: ChatAgent; model: string | null }) {
  return (
    <span className="mx-model-button is-static" title="The model, from the agent's own file. Change it in the terminal with /model.">
      <span className="mx-model-icon"><AppIcon process={agent} size={14} /></span>
      <span className="mx-model-name">{modelName(agent, undefined, model)}</span>
    </span>
  )
}

/** The model and reasoning the next message uses, from the agent picker. */
function ModelPicker({ agent, conversation, model, live, locked }: { agent: ChatAgent; conversation: number; model: string | null; live?: ((command: string) => void) | undefined; locked: string | null }) {
  const [choice, change] = useAgentChoice(conversation, agent)
  return <AgentPicker agent={agent} choice={choice} onChange={change} currentModel={model} locked={locked ?? undefined} {...(live ? { live } : {})} />
}
