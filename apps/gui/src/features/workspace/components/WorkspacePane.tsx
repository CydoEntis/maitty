import { type TerminalLook, TerminalView } from '@/TerminalView'
import { type ChatAgent, chatAvailability, type ChatAvailability, type PaneView } from '../model/chatView'
import type { Dispatch, RefObject, SetStateAction } from 'react'
import { toast } from '@/lib/toast'
import { sendTerminalInput } from '@/api/daemon'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'
import type { InspectView } from '@/features/inspect/InspectPanel'
import { isChatSession } from '../model/chatSession'
import type { TerminalHandle } from '@/features/terminal/handle'
import { PaneCard, type PaneTool } from './PaneCard'
import { ChatPane } from '@/features/chat/components/ChatPane'
import { NoTerminalNote } from '@/features/chat/components/NoTerminalNote'
import { usePaneViews } from '../usePaneViews'
import type { OpenSubagent } from './Sidebar'
import { type Arrangement, focusPane, type Layout, type PaneSlot, type SplitDirection, toggleMaximize } from '../model/layout'
import { folderNameOf, type SessionView } from '../model/session'
import { isShell } from '../model/sidebar'

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
export function WorkspacePane({ shown, pane, kept, paneSessions, chatOf, openSubagent, agentTerminals, sessionMap, outsidePanes, dropId, dragId, paneViews, showChanges, online, setOpenSubagent, togglePaneTool, preferences, startChatHere, inspectOpen, inspectView, toggleAgentTerminal, root, look, modalOpen, mirrorInput, openPath, pasteImage, terminalHandles, setLayout, split, closePaneById, sendTo, copyPathOf, restartPane }: {
  shown: Arrangement
  pane: PaneSlot
  kept: boolean
  paneSessions: ReadonlyMap<string, SessionView>
  chatOf: (session: SessionView) => { availability: ChatAvailability; view: PaneView }
  openSubagent: OpenSubagent | null
  agentTerminals: ReadonlyMap<string, string>
  sessionMap: Map<string, SessionView>
  outsidePanes: boolean
  dropId: string | null
  dragId: string | null
  paneViews: ReturnType<typeof usePaneViews>
  showChanges: () => void
  online: boolean
  setOpenSubagent: Dispatch<SetStateAction<OpenSubagent | null>>
  togglePaneTool: (paneId: string, sessionId: string, tool: PaneTool) => void
  preferences: SettingsPreferences
  startChatHere: (from: SessionView, agent: ChatAgent, fullAccess: boolean) => void
  inspectOpen: boolean
  inspectView: InspectView
  toggleAgentTerminal: (agentId: string) => void
  root: HTMLDivElement | null
  look: TerminalLook
  modalOpen: boolean
  mirrorInput: (fromSessionId: string, data: string | Uint8Array) => void
  openPath: (session: SessionView, path: string, line?: number) => void
  pasteImage: (sessionId: string, image: Blob) => void
  terminalHandles: RefObject<Map<string, TerminalHandle>>
  setLayout: Dispatch<SetStateAction<Layout>>
  split: (paneId: string | null, direction: SplitDirection) => void
  closePaneById: (paneId: string) => void
  sendTo: (target: SessionView, data: string) => Promise<void>
  copyPathOf: (sessionId: string) => void
  restartPane: (paneId: string) => void
}) {
  const session = paneSessions.get(pane.sessionId)
  if (!session) return null
  const focused = pane.id === shown.focus && !kept
  const chat = chatOf(session)
  const chatOnly = isChatSession(session.conversation)
  const subagent = openSubagent?.sessionId === session.id ? session.subagents.find((child) => child.id === openSubagent.subagentId) : undefined
  const typeInto = chatOnly ? undefined : (text: string) => sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, text)
  const terminalOpen = isShell(session) ? null : agentTerminals.has(session.id) && sessionMap.has(agentTerminals.get(session.id) ?? '')
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
          onOpenFiles={() => { togglePaneTool(pane.id, session.id, 'files') }}
          onBackToTerminal={() => { paneViews.choose(session.id, 'terminal') }}
          onStartAgent={(agent) => {
            paneViews.choose(session.id, 'terminal')
            void sendTerminalInput({ id: session.id, generation: session.conversation.generation ?? 0 }, `${agent}\r`).catch((error: unknown) => { toast.error(errorMessage(error, `Could not start ${agent}`)) })
          }}
          defaultFullAccess={preferences.chatFullAccessDefault}
          onStartChat={(agent, fullAccess) => { startChatHere(session, agent, fullAccess) }}
        />
      ) : undefined}
      tool={inspectOpen && focused && (inspectView === 'files' || inspectView === 'changes') ? inspectView : null}
      onTool={(tool) => { togglePaneTool(pane.id, session.id, tool) }}
      terminalOpen={terminalOpen}
      onToggleTerminal={() => { toggleAgentTerminal(session.id) }}
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
