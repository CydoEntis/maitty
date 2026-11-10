import { useState } from 'react'
import { TerminalView } from '@/TerminalView'
import { useWorkspace } from '../workspaceContext'
import { chatAvailability, type PaneView } from '../model/chatView'
import { toast } from '@/lib/toast'
import { sendTerminalInput } from '@/api/daemon'
import { errorMessage } from '@/lib/error-message'
import { isChatSession } from '../model/chatSession'
import { PaneCard } from './PaneCard'
import { LOCAL_MACHINE } from '@/api/machines'
import { AgentTerminals } from './AgentTerminals'
import { startsFolded } from '../model/terminalDrawer'
import { ChatPane } from '@/features/chat/components/ChatPane'
import { NoTerminalNote } from '@/features/chat/components/NoTerminalNote'
import { type Arrangement, focusPane, type PaneSlot, toggleMaximize } from '../model/layout'
import { folderNameOf } from '../model/session'

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
  const { paneSessions, chatOf, openSubagent, ownsTerminals, repoPaneOf, outsidePanes, dropId, dragId, paneViews, showChanges, online, setOpenSubagent, togglePanel, preferences, setPreferences, startChatHere, inspectOpen, addAgentTerminal, root, look, modalOpen, mirrorInput, openPath, pasteImage, terminalHandles, setLayout, split, closePaneById, sendTo, copyPathOf, restartPane } = useWorkspace()
  const [drawerFolded, setDrawerFolded] = useState(startsFolded)
  const session = paneSessions.get(pane.sessionId)
  if (!session) return null
  const focused = pane.id === shown.focus && !kept
  const chat = chatOf(session)
  const chatOnly = isChatSession(session.conversation)
  const subagent = openSubagent?.sessionId === session.id ? session.subagents.find((child) => child.id === openSubagent.subagentId) : undefined
  const typeInto = chatOnly ? undefined : (text: string) => sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, text)
  const terminals = ownsTerminals(session)
  const dock = preferences.terminalDock
  const pinned = terminals && dock === 'under' && !kept
  // The panel has something for this pane: its repository, or terminals that live there. A pane of another machine keeps the button, which says why not.
  const hasPanel = session.machine !== LOCAL_MACHINE || repoPaneOf(session) !== null || (terminals && dock === 'panel')
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
      view={chat.view}
      {...(chatOnly || !hasChat(chat.availability) ? {} : { onViewChange: (view: PaneView) => { paneViews.choose(session.id, view) } })}
      chat={chat.view === 'chat' && !kept ? (
        <ChatPane
          availability={chat.availability}
          hasTerminal={!chatOnly}
          root={session.repoRoot ?? session.cwd}
          onOpenDiff={showChanges}
          typeInto={typeInto}
          ask={session.ask}
          asking={session.asking}
          sessionId={session.id}
          paneStatus={session.status}
          process={session.process}
          online={online}
          folder={folderNameOf(session)}
          branch={session.branch}
          subagent={subagent ?? null}
          subagentCount={session.subagents.length}
          onBackToMain={() => { setOpenSubagent(null) }}
          onOpenFiles={() => { togglePanel(pane.id, session.id, 'files') }}
          onBackToTerminal={() => { paneViews.choose(session.id, 'terminal') }}
          onStartAgent={(agent) => {
            paneViews.choose(session.id, 'terminal')
            void sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, `${agent}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${agent}`)) })
          }}
          defaultFullAccess={preferences.chatFullAccessDefault}
          onStartChat={(agent, fullAccess) => { startChatHere(session, agent, fullAccess) }}
        />
      ) : undefined}
      panelOpen={inspectOpen && focused}
      {...(hasPanel ? { onPanel: () => { togglePanel(pane.id, session.id) } } : {})}
      changed={{ added: session.diffAdded, removed: session.diffRemoved }}
      terminalDock={terminals ? dock : null}
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
      onToggleMaximize={(paneId) => { setLayout((current) => toggleMaximize(current, paneId)) }}
      onClose={closePaneById}
      onClear={() => { void sendTo(session, CLEAR_SCREEN).catch((error: unknown) => { toast.error(errorMessage(error, 'Could not clear the screen')) }) }}
      onCopyPath={() => { copyPathOf(session.id) }}
      onRestart={restartPane}
    />
  )
}
