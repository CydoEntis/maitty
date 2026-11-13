import { useEffect, useMemo, useRef, useState, type ClipboardEvent, type CSSProperties, type KeyboardEvent, type ReactNode } from 'react'
import { toast } from '@/lib/toast'
import { pasteFromHost, pickFromClipboard, unsupportedMessage } from '@/features/terminal/clipboardImage'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import { AppIcon, Icon } from '@/features/workspace/components/glyphs'
import { registerComposerDrop } from '../composerDrop'
import { updateComposer, useComposerDraft, useComposerSlice } from '../composerStore'
import type { ChatComposer } from '../useChatComposer'
import { defaultChoice, useAgentChoice, type AgentChoice } from '../agentChoice'
import { DraftAttachments } from './Attachments'
import { AgentPicker } from './AgentPicker'
import { modelName } from '../modelName'
import { PermissionMenu } from './PermissionMenu'
import { QueuedMessages } from './QueuedMessages'
import { MAX_SHOWN, SlashPicker } from './SlashPicker'
import { FilePicker } from './FilePicker'
import { useFileMention } from '../useFileMention'
import type { FileSearchPlace } from '../fileMention'
import { asSkills, commandText, matchCommands, skillQuery, slashQuery, useSlashCommands, withTerminalCommands, type SlashCommand } from '../slashCommands'
import { CLAUDE_TERMINAL_COMMANDS } from '../terminalCommands'
import { CODEX_COMMANDS } from '../codexCommands'
import { isBangLine } from '../slashCommands'
import { BranchMenu, type BranchMenuSpec } from './BranchMenu'
import { usageItems, type SessionUsage } from '../usageStrip'
import { nowSeconds } from '@/features/workspace/model/newChatHost'

/** A key with nothing stored under it, for a composer whose view holds its own draft. */
const NO_CONVERSATION = -1

/** What the settings tray under the box says about the session. */
export type ComposerFacts = {
  /** What the session's agent reported using; the strip under the tray shows it, and is not drawn when there is nothing. */
  usage?: SessionUsage | undefined
  /** The permission mode in words, from the agent's own file; null when it has not said. */
  mode: string | null
  /** The mode asks about nothing, so it is drawn in the danger colour. */
  modeDanger: boolean
  /** Whether the folder is a worktree of its repository, not the main checkout; null when that is not known. */
  worktree?: boolean | null | undefined
  /** Given where the branch can be changed from the tray (a session on this computer); without it the branch is only said. */
  branchMenu?: BranchMenuSpec | undefined
  folder: string
  branch: string | null
  /** The model the agent's own file says it uses; null when it has not said. */
  model: string | null
  /** The effort the agent's own files say it runs at, when they do. */
  effort?: string | null | undefined
  onOpenFolder: (() => void) | null
  /** For a chat that follows the agent's terminal and can type into it: send the agent one of its own commands (`/model x`). */
  onCommand?: ((text: string) => void) | undefined
  /** The same, for the permission mode: moves the agent to the picked mode (by its Shift+Tab). */
  onPickMode?: ((mode: string) => void) | undefined
  /** The modes the dropdown offers and whose they are; Claude's when not given. */
  modes?: readonly string[] | undefined
  modeAgent?: string | undefined
  /** Why the mode cannot be picked yet, for a chat that will offer it: the button is drawn now, greyed, so it does not change shape later. */
  modeLocked?: string | undefined
  /** The mode as the agent's file or screen wrote it, to tick it in the dropdown. */
  modeKey?: string | null | undefined
  /** Bypass permissions can be picked: this session started with Claude allowing it. */
  bypassAllowed?: boolean | undefined
  /**
   * Set for an agent whose model and mode can be changed (Claude): both chips are always drawn, and when they cannot be used right
   * now (the agent is starting or working) they stay where they are, greyed out with `locked` as the reason, so the box never
   * rearranges. `start` is what a chat with no conversation yet was started with.
   */
  agentChips?: {
    readonly locked: string | null
    /** Why the model and effort cannot be picked, when that differs from `locked`: a pick made while the agent works is held for the turn's end, so only the mode waits. */
    readonly pickerLocked?: string | null
    readonly start?: AgentChoice
  } | undefined
}

type Props = {
  agent: ChatAgent
  composer: ChatComposer
  /** The chat's conversation number; with it the composer offers the agent, model and reasoning picker. */
  conversation?: number
  /** Whose slash commands the box offers as a slash is typed, and the folder they are the commands of; absent for an agent with none to list. */
  commandsFrom?: {
    readonly agent: string
    readonly folder: string
    /** For an agent with a terminal: types a command there and shows it. With it the box also offers the commands only the terminal runs. */
    readonly inTerminal?: ((command: string) => void) | undefined
  } | undefined
  /** The question tray: what the agent asks, slid up from behind the box. Null when it asks nothing. */
  question?: ReactNode
  facts: ComposerFacts
  /** A message went in and the agent has not taken it up yet: the send button shows it is working and sends nothing more. */
  waiting?: boolean
  /** Where an `@` looks for files; absent where the daemon cannot be asked (another machine, no pane). */
  files?: FileSearchPlace | undefined
}

const NO_CHANGE = () => undefined
const PERCENT = 100

/**
 * The message box, in three layers (README: message box): the question tray when the agent asks, the
 * box (text, attach, model picker, send), and the settings tray under it. Enter sends, Shift+Enter
 * adds a line, Esc interrupts the running turn. While the agent works, sending queues the message;
 * while a request waits for an answer, or the daemon is offline, the box is off and says why.
 */
export function Composer({ agent, composer, conversation, commandsFrom, question = null, facts, waiting = false, files }: Props) {
  const { setDraft, attachments, attach, readOnly, queue, removeQueued, editQueued, sendQueuedNow, submit, stop, running, sending, disabledReason, error } = composer
  const storedDraft = useComposerDraft(composer.conversationKey ?? NO_CONVERSATION)
  const restoreNote = useComposerSlice(composer.conversationKey ?? NO_CONVERSATION, (state) => state.note)
  const draft = composer.draft ?? storedDraft
  const off = disabledReason !== null
  const box = useRef<HTMLDivElement>(null)
  const text = useRef<HTMLTextAreaElement>(null)
  // A chat that opens takes the typing, as the terminal under it would, when nothing else on the page has it:
  // the new-chat page has just closed, or the starting chat's box was replaced by this one.
  useEffect(() => {
    const active = document.activeElement
    if (active !== null && active !== document.body) return
    const pane = text.current?.closest('.mx-pane')
    if (pane?.classList.contains('is-card') === true && !pane.classList.contains('is-focused')) return
    text.current?.focus({ preventScroll: true })
  }, [])
  // The desktop shell takes file drops for itself, so the composer asks to be told of the ones that land on it.
  const { addPaths } = attach
  useEffect(() => {
    const element = box.current
    return element === null ? undefined : registerComposerDrop(element, addPaths)
  }, [addPaths])

  // Text first, as in the terminal: a clipboard with any text pastes the text; only a bare image is attached.
  const onPaste = (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const pick = pickFromClipboard(event.clipboardData)
    if (pick.kind === 'none') pasteFromHost(attach.addImage)
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

  // The agent's commands, offered while a slash and a name are typed. Esc puts the list away until the text changes.
  // Codex's own box names a skill after a `$`, so that opens the list too, with only its skills.
  const naming = off || commandsFrom?.agent !== 'codex' ? null : skillQuery(draft)
  const query = off || commandsFrom === undefined ? null : slashQuery(draft) ?? naming
  const [closedFor, setClosedFor] = useState<string | null>(null)
  const picking = query !== null && closedFor !== draft
  const slash = useSlashCommands(commandsFrom ?? null, picking)
  const inTerminal = commandsFrom?.inTerminal
  // Claude's are its own list plus the ones only its terminal runs; Codex's are the window's list and the skills
  // Codex itself lists. The terminal's commands are offered only to an agent that has a terminal to run them in.
  const skillsOnly = naming !== null
  const offered = useMemo(() => {
    if (commandsFrom?.agent === 'codex') {
      const skills = asSkills(slash.commands)
      if (skillsOnly) return skills
      return [...(inTerminal === undefined ? CODEX_COMMANDS.filter((command) => command.terminal !== true) : CODEX_COMMANDS), ...skills]
    }
    const own = withTerminalCommands(slash.commands, CLAUDE_TERMINAL_COMMANDS)
    return inTerminal === undefined ? own.filter((command) => command.terminal !== true) : own
  }, [slash.commands, inTerminal, commandsFrom?.agent, skillsOnly])
  const matches = useMemo(() => (query === null ? [] : matchCommands(offered, query)), [offered, query])
  // The keys start at the closest match each time the typed name changes.
  const [cursor, setCursor] = useState({ query: '', index: 0 })
  const active = cursor.query === query ? Math.min(cursor.index, Math.max(0, Math.min(matches.length, MAX_SHOWN) - 1)) : 0
  const complete = (command: SlashCommand) => {
    setDraft(commandText(command))
    text.current?.focus()
  }
  // A command that takes nothing runs at once, as it does in the terminal; one that takes something waits for it.
  const run = (command: SlashCommand) => {
    // A skill is named in a message, never run by itself.
    if (command.hint !== '' || command.skill === true) {
      complete(command)
      return
    }
    if (command.terminal === true && inTerminal !== undefined) {
      // Its menu is the terminal's to show, and its keys would land in a running turn.
      if (running) {
        toast.message(`${agent} is working. Run /${command.name} when it finishes.`)
        return
      }
      setDraft('')
      inTerminal(`/${command.name}`)
      return
    }
    setDraft(`/${command.name}`)
    submit()
    text.current?.focus()
  }
  // The @ picker follows the caret: it is open only while the caret sits, with nothing selected, at the end of the text.
  const [caretAtEnd, setCaretAtEnd] = useState(true)
  const followCaret = (area: HTMLTextAreaElement) => { setCaretAtEnd(area.selectionStart === area.selectionEnd && area.selectionEnd === area.value.length) }
  const mention = useFileMention(off ? null : files ?? null, draft, caretAtEnd, setDraft, () => { text.current?.focus() })
  const pickerKey = (event: KeyboardEvent<HTMLTextAreaElement>): boolean => {
    if (mention.onKey(event)) return true
    if (!picking) return false
    if (event.key === 'Escape') {
      setClosedFor(draft)
      return true
    }
    const chosen = matches[active]
    if (chosen === undefined) return false
    const shown = Math.min(matches.length, MAX_SHOWN)
    if (event.key === 'ArrowDown') setCursor({ query, index: (active + 1) % shown })
    else if (event.key === 'ArrowUp') setCursor({ query, index: (active + shown - 1) % shown })
    else if (event.key === 'Tab' && !event.shiftKey) complete(chosen)
    else if (event.key === 'Enter' && !event.shiftKey) run(chosen)
    else return false
    return true
  }

  const onKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    // Enter while an input method is composing text confirms the text; it is not a send.
    if (event.nativeEvent.isComposing) return
    if (pickerKey(event)) {
      event.preventDefault()
      return
    }
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
        <QueuedMessages agent={agent} queue={queue} onRemove={removeQueued} onEdit={(id) => { editQueued(id); text.current?.focus() }} onSendNow={sendQueuedNow} />
        {question}
        <div className={`mx-box${off ? ' is-locked' : ''}`}>
          {picking && (
            <SlashPicker agent={agent} matches={matches} active={active} loading={slash.loading} error={slash.error} onPick={run} onHover={(index) => { setCursor({ query, index }) }} />
          )}
          {mention.open && (
            <FilePicker found={mention.found} active={mention.active} onPick={mention.pick} onHover={mention.hover} />
          )}
          <DraftAttachments attachments={attachments} onRemove={attach.remove} />
          <textarea
            ref={text}
            aria-label={`Message ${agent}`}
            {...(picking && matches.length > 0 ? { role: 'combobox', 'aria-expanded': true, 'aria-controls': 'mx-slash-list', 'aria-activedescendant': `mx-slash-${String(active)}` } : {})}
            {...(mention.open && (mention.found?.paths.length ?? 0) > 0 ? { role: 'combobox', 'aria-expanded': true, 'aria-controls': 'mx-file-list', 'aria-activedescendant': `mx-file-${String(mention.active)}` } : {})}
            rows={2}
            value={draft}
            disabled={off}
            placeholder={placeholder}
            className="mx-box-text"
            onChange={(event) => { setDraft(event.target.value); followCaret(event.target) }}
            onSelect={(event) => { followCaret(event.currentTarget) }}
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
                ? <ModelPicker agent={agent} conversation={conversation} model={facts.model} effort={facts.effort ?? null} live={readOnly ? facts.onCommand : undefined} locked={facts.agentChips.pickerLocked === undefined ? facts.agentChips.locked : facts.agentChips.pickerLocked} />
                : <AgentPicker agent={agent} choice={facts.agentChips.start ?? defaultChoice()} onChange={NO_CHANGE} locked={facts.agentChips.locked ?? undefined} />
            ) : (
              <>
                {conversation !== undefined && (!readOnly || facts.onCommand !== undefined) && <ModelPicker agent={agent} conversation={conversation} model={facts.model} effort={facts.effort ?? null} live={readOnly ? facts.onCommand : undefined} locked={null} />}
                {readOnly && facts.onCommand === undefined && <ModelLabel agent={agent} model={facts.model} />}
              </>
            )}
            {stopping ? (
              <button type="button" className="mx-send is-stop" aria-label={`Stop ${agent}`} title={`${agent} is working. Stop it (Esc)`} onClick={stop}><span className="mx-stop-square" aria-hidden="true" /></button>
            ) : busy ? (
              // Sent, and not yet a turn to stop: the same square, off until there is one.
              <button type="button" className="mx-send is-stop is-off is-busy" aria-label={`${agent} is working`} aria-busy="true" title={`${agent} is working`} disabled><span className="mx-stop-square" aria-hidden="true" /></button>
            ) : (
              <button type="button" className={`mx-send${off || empty ? ' is-off' : ''}`} aria-label={running ? 'Queue message' : 'Send message'} disabled={off || sending || empty} onClick={submit}>
                <Icon name="arrowUp" size={15} strokeWidth={2.2} />
              </button>
            )}
          </div>
        </div>
        <SettingsTray facts={facts} />
        {readOnly && (agent === 'claude' || agent === 'gemini') && isBangLine(draft) && <p className="mx-composer-note">Runs in the terminal, not as a message. Open the terminal to answer anything it asks.</p>}
        {restoreNote !== null && composer.conversationKey !== undefined && (
          <p className="mx-composer-note">
            Your next message starts with a line telling {agent} the files were restored.{' '}
            <button type="button" className="mx-composer-note-action" onClick={() => { updateComposer(composer.conversationKey ?? NO_CONVERSATION, (state) => ({ ...state, note: null })) }}>Leave it out</button>
          </p>
        )}
        {error !== null && <p role="alert" className="mx-composer-error">{error}. Your message is kept; send it again to retry.</p>}
      </div>
    </div>
  )
}

function ModeIcon() {
  return <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="1.8" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true"><path d="M12 3 5 6v6c0 4.4 3 7.6 7 9 4-1.4 7-4.6 7-9V6z" /></svg>
}

function modeLockedWhy(facts: Pick<ComposerFacts, 'modeLocked' | 'agentChips'>): string {
  return facts.modeLocked ?? facts.agentChips?.locked ?? 'Not available for this chat'
}

/**
 * The meters at the right of the tray: how full the session's context is, as a small ring, and how much of each of the
 * plan's limits is used, as a ring with its window (`Week 49%`). Only what the agent reported is drawn (ADR 0009); with
 * nothing reported there is none. When a limit starts over is in its tooltip.
 */
/** The short name a meter wears beside its figure: Context, 5h, Week. */
const usageName = (item: { window: string | null }): string => (item.window === null ? 'Context' : item.window === 'week' ? 'Week' : item.window)

function UsageMeters({ usage }: { usage: SessionUsage }) {
  const items = usageItems(usage, nowSeconds())
  if (items.length === 0) return null
  return (
    <span className="mx-usage" role="group" aria-label="Usage">
      {items.map((item) => {
        const used = item.window === null ? `How much of this chat's memory is used: ${String(item.percent)}%` : item.resetsIn === null ? `${item.label}: ${String(item.percent)}% used` : `${item.label}: ${String(item.percent)}% used · resets in ${item.resetsIn}`
        return (
          <span key={item.key} className={`mx-usage-item is-${item.tone}`} title={used}>
            <span className="mx-usage-name">{usageName(item)}</span>
            <span className="mx-usage-figure">{item.percent}%</span>
            <span className="mx-usage-ring" role="meter" aria-label={item.label} aria-valuemin={0} aria-valuemax={PERCENT} aria-valuenow={item.percent} style={{ '--share': `${String(item.percent)}%` } as CSSProperties} />
          </span>
        )
      })}
    </span>
  )
}

/** The darker tray under the box, one line: the folder, the branch (or the worktree, in its own colour and with its tree), how the agent is set up, and at the right what it has used. */
function SettingsTray({ facts }: { facts: ComposerFacts }) {
  return (
    <div className="mx-settings-tray">
      {facts.onOpenFolder === null
        ? <span className="mx-tray-fact"><Icon name="folder" />{facts.folder}</span>
        : <button type="button" className="mx-tray-fact is-button" title="Show its files" onClick={facts.onOpenFolder}><Icon name="folder" />{facts.folder}</button>}
      {facts.branch !== null && facts.branchMenu !== undefined && <BranchMenu spec={facts.branchMenu} branch={facts.branch} worktree={facts.worktree === true} />}
      {facts.branch !== null && facts.branchMenu === undefined && (
        <span className={`mx-tray-fact is-branch${facts.worktree === true ? ' is-worktree' : ''}`} title={facts.worktree === true ? `Worktree: its own copy of the project, on branch ${facts.branch}` : facts.worktree === false ? `Branch ${facts.branch}, in the project's own folder` : `Branch ${facts.branch}`} aria-label={`${facts.worktree === true ? 'Worktree' : 'Branch'} ${facts.branch}`}>
          <Icon name={facts.worktree === true ? 'worktree' : 'branch'} size={13} strokeWidth={1.9} stroke="currentColor" />
          <span className="mx-tray-branch">{facts.branch}</span>
        </span>
      )}
      {facts.modeLocked !== undefined || (facts.agentChips !== undefined && (facts.agentChips.locked !== null || facts.onPickMode === undefined)) ? (
        // aria-disabled, not disabled: a disabled button gets no tooltip, and the reason is the point.
        <button type="button" aria-disabled="true" className={`mx-tray-fact is-locked${facts.modeDanger ? ' is-danger' : ''}`} title={modeLockedWhy(facts)} aria-label={`Permission mode: ${facts.mode ?? 'not reported yet'}. ${modeLockedWhy(facts)}`}>
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
        <PermissionMenu
          agent={facts.modeAgent ?? 'Claude'}
          label="Permission mode"
          mode={facts.modeKey ?? ''}
          {...(facts.modes === undefined ? {} : { modes: facts.modes })}
          onPick={facts.onPickMode}
          bypassOff={facts.bypassAllowed === true ? undefined : { note: 'needs a new session', why: 'Claude only allows this when it starts with Yolo mode on (Settings, General). This session started without it: turn the setting on and start a new session.' }}
        >
          <button type="button" className={`mx-tray-fact is-button${facts.modeDanger ? ' is-danger' : ''}`} title="Permission mode" aria-label={`Permission mode: ${facts.mode ?? 'not reported yet'}. Pick another.`}>
            <ModeIcon />{facts.mode ?? 'Permissions'}<Icon name="chevronDown" size={10} />
          </button>
        </PermissionMenu>
      )}
      <span className="mx-spacer" />
      {facts.usage !== undefined && <UsageMeters usage={facts.usage} />}
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
function ModelPicker({ agent, conversation, model, effort, live, locked }: { agent: ChatAgent; conversation: number; model: string | null; effort: string | null; live?: ((command: string) => void) | undefined; locked: string | null }) {
  const [choice, change] = useAgentChoice(conversation, agent)
  return <AgentPicker agent={agent} choice={choice} onChange={change} currentModel={model} currentEffort={effort} locked={locked ?? undefined} {...(live ? { live } : {})} />
}
