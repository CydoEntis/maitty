import { movePanes } from './paneTransition'
import { useCallback, useState } from 'react'
import { TerminalView } from '@/TerminalView'
import { useWorkspace } from '../workspaceContext'
import { chatAvailability, type PaneView } from '../model/chatView'
import { toast } from '@/lib/toast'
import { answerHelpers, dropQueuedFollowUp, isDesktop, queueFollowUp, sendTerminalInput } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { isChatSession } from '../model/chatSession'
import { carryDraftToTerminal } from '../model/draftCarry'
import { PaneCard } from './PaneCard'
import { LOCAL_MACHINE } from '@/api/machines'
import { AgentTerminals } from './AgentTerminals'
import { startsFolded } from '../model/terminalDrawer'
import type { TurnSpan } from '@/features/inspect/turnRange'
import { CODEX_SUBMIT_GAP_MS } from '@/features/chat/useChatComposer'
import { contextFull, type ChatProblem } from '@/features/chat/problem'
import { ChatPane } from '@/features/chat/components/ChatPane'
import { NoTerminalNote } from '@/features/chat/components/NoTerminalNote'
import { type Arrangement, focusPane, type PaneSlot, toggleMaximize } from '../model/layout'
import { folderNameOf } from '../model/session'
import { isShell } from '../model/sidebar'
import { moveRefusal } from '../model/moveToWorktree'
import { guardLocal } from '@/lib/localOnly'
import type { RewindPlace } from '@/features/chat/editFromHere'
import { fileSearchOf } from '@/features/chat/fileMention'

/** Whether a session can show a chat at all: an agent the app reads. A shell or another program has none, so it gets no switch. */
function hasChat(availability: ReturnType<typeof chatAvailability>): boolean {
  return availability.kind === 'ready' || availability.kind === 'no-events'
}

/** Ctrl+L: a shell clears its screen and keeps what was typed. */
const CLEAR_SCREEN = '\x0c'

/**
 * One pane of the layout: its header, and the session's terminal with its chat over it when the chat shows.
 * A pane of a tab kept out of sight keeps only its terminal, paused: no chat reads, takes keys or answers for it.
 */
export function WorkspacePane({ shown, pane, kept }: {
  shown: Arrangement
  pane: PaneSlot
  kept: boolean
}) {
  const { paneSessions, chatOf, openSubagent, ownsTerminals, repoPaneOf, sharedFolderRefusalOf, outsidePanes, dropId, dragId, paneViews, showChanges, online, setOpenSubagent, togglePanel, preferences, setPreferences, startChatHere, handoffSession, inspectOpen, addAgentTerminal, root, look, modalOpen, mirrorInput, openPath, pasteImage, terminalHandles, setLayout, split, closePaneById, sendTo, copyPathOf, restartPane, setRenameId , heads, retryNamed, newChatWithSummary , moveToWorktree } = useWorkspace()
  const [drawerFolded, setDrawerFolded] = useState(startsFolded)
  // Bound to this pane, so a row opened from a pane without the focus focuses it and shows its own session's changes.
  const openDiff = useCallback((file: string, turn?: TurnSpan | null) => { showChanges(pane.id, pane.sessionId, file, turn) }, [showChanges, pane.id, pane.sessionId])
  const session = paneSessions.get(pane.sessionId)
  if (!session) return null
  const focused = pane.id === shown.focus && !kept
  const chat = chatOf(session)
  const chatOnly = isChatSession(session.conversation)
  const subagent = openSubagent?.sessionId === session.id ? session.subagents.find((child) => child.id === openSubagent.subagentId) : undefined
  const typeInto = chatOnly ? undefined : (text: string) => sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, text)
  // A pane's daemon holds what is sent while its agent works; a chat session's own daemon queue is its conversation's.
  const paneQueue = chatOnly ? undefined : {
    queued: session.queued,
    add: (text: string, first: boolean) => queueFollowUp(session, text, first),
    drop: (index: number, text: string) => dropQueuedFollowUp(session, index, text),
  }
  const showTerminal = () => {
    paneViews.choose(session.id, 'terminal')
    void carryDraftToTerminal(session).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not move your message to the terminal')) })
  }
  const head = session.machine === LOCAL_MACHINE ? undefined : heads.find((candidate) => candidate.machine === session.machine)
  const fullPercent = contextFull({ context: session.usage.context, limits: [] })
  // One problem at a time, the one that matters most: a lost machine, then a stopped agent, then a full context.
  const problem: ChatProblem | null = head?.down === true && head.name !== null
    ? { kind: 'offline', machine: head.label, since: head.downSince, onRetry: () => { retryNamed(head.name ?? '') } }
    : session.status === 'error' && !session.ended && !isShell(session)
      ? { kind: 'crashed', agent: session.process, code: session.conversation.details?.exit?.code ?? null, onViewLog: () => { showTerminal() }, onRestart: chatOnly ? null : () => { restartPane(pane.id) } }
      : fullPercent !== null && typeInto !== undefined
        ? { kind: 'context', agent: session.process, percent: fullPercent, onNewWithSummary: (summary) => { newChatWithSummary(session, summary) }, onCompact: () => { void typeInto('/compact').then(() => new Promise((resolve) => { window.setTimeout(resolve, CODEX_SUBMIT_GAP_MS) })).then(() => typeInto('\r')).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not compact')) }) } }
        : null
  const terminals = ownsTerminals(session)
  // Restoring reads and writes the checkout, so it is this computer's only, like Inspect.
  const rewindTerm = repoPaneOf(session)
  // A restore would also undo the files of another session in the same checkout.
  const rewindRefusal = guardLocal(session, 'rewind') ?? (rewindTerm === null ? null : sharedFolderRefusalOf(session))
  const fileSearch = fileSearchOf(session, { term: rewindTerm, machineDown: head?.down === true, desktop: isDesktop() })
  const rewind: RewindPlace | undefined = rewindRefusal !== null ? { refusal: rewindRefusal } : rewindTerm !== null ? { term: rewindTerm } : undefined
  const dock = preferences.terminalDock
  // A shell is a terminal already: its further terminals stay in the side panel, never under it.
  const shell = isShell(session)
  const pinned = terminals && !shell && dock === 'under' && !kept
  // The panel has something for this pane: its repository, or terminals that live there. A pane of another machine keeps the button, which says why not.
  const hasPanel = session.machine !== LOCAL_MACHINE || repoPaneOf(session) !== null || (terminals && (shell || dock === 'panel'))
  return (
    <PaneCard
      pane={pane}
      index={shown.panes.findIndex((candidate) => candidate.id === pane.id)}
      session={session}
      paneCount={shown.panes.length}
      broadcast={shown.broadcast === true}
      focused={focused && !outsidePanes}
      maximized={shown.max === pane.id}
      dropTarget={dropId === pane.id && dragId !== pane.id}
      dragging={dragId === pane.id}
      dropWord={dragId === null ? 'Drop to show it here' : 'Drop to swap'}
      view={chat.view}
      {...(chatOnly || !hasChat(chat.availability) ? {} : { onViewChange: (view: PaneView) => { if (view === 'terminal') showTerminal(); else paneViews.choose(session.id, view) } })}
      chat={chat.view === 'chat' && !kept ? (
        <ChatPane
          availability={chat.availability}
          hasTerminal={!chatOnly}
          worktree={chatOnly ? undefined : session.worktree}
          root={session.repoRoot ?? session.cwd}
          onOpenDiff={openDiff}
          onOpenFile={(path, line) => { openPath(session, path, line ?? undefined) }}
          typeInto={typeInto}
          paneQueue={paneQueue}
          ask={session.ask}
          asking={session.asking}
          sessionId={session.id}
          problem={problem}
          effort={session.agentEffort ?? null}
          rewind={rewind}
          {...(fileSearch === null ? {} : { fileSearch })}
          {...(chatOnly || session.machine !== LOCAL_MACHINE || session.branch === null ? {} : { branchMenu: { folder: session.cwd, onNewWorktree: () => { void moveToWorktree(session) }, ...(moveRefusal(session) === null ? {} : { newWorktreeBlocked: moveRefusal(session) ?? '' }) } })}
          paneStatus={session.status}
          usage={{ context: session.usage.context, limits: session.limits }}
          process={session.process}
          online={online}
          folder={folderNameOf(session)}
          branch={session.branch}
          subagent={subagent ?? null}
          onBackToMain={() => { setOpenSubagent(null) }}
          onOpenFiles={() => { togglePanel(pane.id, session.id, 'files') }}
          onBackToTerminal={showTerminal}
          onStartAgent={(agent) => {
            paneViews.choose(session.id, 'terminal')
            void sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, `${agent}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${agent}`)) })
          }}
          defaultFullAccess={preferences.chatFullAccessDefault}
          hideSuccessfulTools={preferences.hideSuccessfulTools}
          onStartChat={(agent, fullAccess) => { startChatHere(session, agent, fullAccess) }}
        />
      ) : undefined}
      panelOpen={inspectOpen && focused}
      {...(hasPanel ? { onPanel: () => { togglePanel(pane.id, session.id) } } : {})}
      changed={{ added: session.diffAdded, removed: session.diffRemoved }}
      terminalDock={terminals && !shell ? dock : null}
      onNewTerminal={() => { addAgentTerminal(session.id) }}
      onDock={(terminalDock) => { setPreferences((current) => ({ ...current, terminalDock })) }}
      {...(pinned ? { drawer: <AgentTerminals agent={session} dock="under" focused={false} folded={drawerFolded} onFold={setDrawerFolded} />, drawerFolded, onDrawerFold: setDrawerFolded } : {})}
      online={online}
      portalContainer={root}
      terminal={chatOnly ? <NoTerminalNote /> :
        <TerminalView
          key={session.id}
          conversation={session.conversation}
          desktop
          fontSize={preferences.terminalFontSize}
          look={look}
          label={`Shell in ${session.cwdLabel}`}
          inputDisabled={!online}
          ended={session.ended}
          focused={focused && !modalOpen && chat.view === 'terminal'}
          paused={kept || chat.view === 'chat'}
          onInput={(data) => { mirrorInput(session.id, data) }}
          onOpenPath={(path, line) => { openPath(session, path, line) }}
          onPasteImage={(image) => { pasteImage(session.id, image) }}
          onHandle={(handle) => {
            if (handle) terminalHandles.current.set(session.id, handle)
            else terminalHandles.current.delete(session.id)
          }}
        />
      }
      onFocus={(paneId) => { setLayout((current) => focusPane(current, paneId)) }}
      onSplit={(paneId, direction) => {
        setLayout((current) => focusPane(current, paneId))
        split(paneId, direction)
      }}
      onToggleMaximize={(paneId) => { movePanes(() => { setLayout((current) => toggleMaximize(current, paneId)) }) }}
      onClose={closePaneById}
      onClear={() => { void sendTo(session, CLEAR_SCREEN).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not clear the screen')) }) }}
      onCopyPath={() => { copyPathOf(session.id) }}
      onRename={() => { setRenameId(session.id) }}
      onHandoff={(agent) => { void handoffSession(session, agent) }}
      onRestart={restartPane}
      onAnswerHelpers={(choice) => { void answerHelpers(session.conversation, choice).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not answer the helper request')) }) }}
    />
  )
}
