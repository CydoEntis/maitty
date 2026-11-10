import { type Dispatch, type RefObject, type SetStateAction, useCallback, useMemo, useState } from 'react'
import type { QueryClient } from '@tanstack/react-query'
import { type RemoteMachine, remoteStartRefusal, remoteTargetOf } from './model/machines'
import { toast } from '@/lib/toast'
import { addWorktree, defaultSessionFolder, startAgentSession as launchAgentSession, loadConversations, renameConversationOnDaemon, sendTerminalInput, sendWhenAgentReady, startChatSession, startShellSession, submitMessage, switchBranch } from '@/api/daemon'
import { LOCAL_MACHINE, startSessionOnMachine } from '@/api/machines'
import type { DesktopSettingsSection } from '@/features/settings/components/DesktopSettings'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { errorMessage } from '@/lib/error-message'
import { newWorktreeSession } from '@/features/worktrees/api'
import { rememberConversationPermissions } from '@/data/conversationPermissions'
import { reserveOperation } from '@/features/chat/sendOperation'
import { entrustFirstMessage, holdFirstMessage, holdStartChoice } from '@/features/chat/firstMessage'
import { connectAgentOnce } from './model/agentHooks'
import { nameFromMessage } from './model/newSession'
import { usePaneViews } from './usePaneViews'
import type { ChatAgent, PaneView, StructuredAgent } from './model/chatView'
import type { NewSessionRequest } from './components/NewSessionModal'
import type { NewChatRequest } from './components/NewChatPage'
import type { WireChoice } from '@/features/chat/agentChoice'
import { rememberYoloSession } from '@/features/chat/yoloSessions'
import { launchFlags, yoloFlags } from './model/newChatHost'
import { type Layout, openArrangement } from './model/layout'
import type { SessionView } from './model/session'
import { resolveStartFolder } from './model/startFolder'
import { makeId } from './makeId'

/** How long a new session on another machine is waited for, and how often its list is looked at. */
const REMOTE_START_WAIT_MS = 8000

const EXPECTED_AGENT_MS = 30_000

function omitKey<T>(record: Record<string, T>, key: string): Record<string, T> {
  return Object.fromEntries(Object.entries(record).filter(([name]) => name !== key))
}

const REMOTE_START_POLL_MS = 250

/** Every way the window starts a session: a shell, an agent in a terminal, a chat, a worktree, on this computer or another machine. */
export function useSessionStarts({ queryClient, sessions, setWorktreeFrom, setLayout, flash, focusedSession, setNewChatOpen, online, starting, setStarting, setStartedFrom, sessionMap, preferences, setExpectedAgents, paneViews, setSettingsSection, setNewSessionMachine, home, machinesRef }: {
  queryClient: QueryClient
  sessions: SessionView[]
  setWorktreeFrom: Dispatch<SetStateAction<SessionView | null>>
  setLayout: Dispatch<SetStateAction<Layout>>
  flash: (text: string) => void
  focusedSession: SessionView | undefined
  setNewChatOpen: Dispatch<SetStateAction<boolean>>
  online: boolean
  starting: boolean
  setStarting: Dispatch<SetStateAction<boolean>>
  setStartedFrom: Dispatch<SetStateAction<ReadonlySet<string> | null>>
  sessionMap: Map<string, SessionView>
  preferences: SettingsPreferences
  setExpectedAgents: Dispatch<SetStateAction<Record<string, ChatAgent>>>
  paneViews: ReturnType<typeof usePaneViews>
  setSettingsSection: Dispatch<SetStateAction<DesktopSettingsSection | null>>
  setNewSessionMachine: Dispatch<SetStateAction<number | null>>
  home: string
  machinesRef: RefObject<ReadonlyMap<number, RemoteMachine>>
}) {
  // The session the daemon made since `known` was the whole list, if one appeared.
  const newSessionSince = useCallback(async (known: ReadonlySet<string>): Promise<string | null> => {
    await queryClient.invalidateQueries({ queryKey: ['conversations'] })
    const fresh = await queryClient.query({ queryKey: ['conversations'], queryFn: loadConversations, staleTime: 0 })
    const created = fresh.filter((item) => item.kind === 'terminal' && !known.has(item.id)).sort((a, b) => (a.nativePane ?? 0) - (b.nativePane ?? 0)).at(-1)
    return created?.id ?? null
  }, [queryClient])

  const startShell = useCallback(async (folder: string, launch?: (folder: string) => Promise<void>): Promise<string | null> => {
    const known = new Set(sessions.map((session) => session.id))
    await (launch ?? startShellSession)(folder)
    return newSessionSince(known)
  }, [sessions, newSessionSince])

  const createWorktree = useCallback(async (from: SessionView, branch: string, base: string | null) => {
    const known = new Set(sessions.map((session) => session.id))
    await newWorktreeSession(from.pane, branch, base)
    const id = await newSessionSince(known)
    setWorktreeFrom(null)
    if (id) setLayout((current) => openArrangement(current, id, makeId))
    flash(`Worktree ${branch} ready`)
  }, [sessions, newSessionSince, setWorktreeFrom, setLayout, flash])

  // Lists again and opens the chat session the daemon just started, with its first prompt if there is one.
  const openStartedChat = useCallback(async (started: { id: number; generation: number }, prompt: string, choice: WireChoice | null = null) => {
    const id = String(started.id)
    await queryClient.invalidateQueries({ queryKey: ['conversations'] })
    await queryClient.query({ queryKey: ['conversations'], queryFn: loadConversations, staleTime: 0 })
    setLayout((current) => openArrangement(current, id, makeId))
    if (prompt !== '') {
      // The start has answered, so the agent is up: the first prompt is an ordinary send, number 1.
      const operation = reserveOperation(started.id, { receipt: null, queued: [] })
      await submitMessage({ id, generation: started.generation }, operation, prompt, choice ?? undefined)
    }
  }, [queryClient, setLayout])

  // A chat session in `path`, opened in a layout of its own. Chats are this computer's: another machine's shell asks first.
  const startChat = useCallback((path: string, agent: StructuredAgent, fullAccess: boolean) =>
    startChatSession(path, agent, null, fullAccess).then((started) => {
      if (fullAccess) rememberConversationPermissions(String(started.id), 'full-access')
      return openStartedChat(started, '')
    }).then(() => { flash(`${agent} chat started`) }), [openStartedChat, flash])


  // What the new chat page asks for: a structured chat in the folder, with the model and effort picked, and its first message.
  // Choosing another session (the sidebar, a shortcut) leaves the new chat page for it.
  const focusedSessionKey = focusedSession?.id
  const [seenFocusKey, setSeenFocusKey] = useState(focusedSessionKey)
  if (seenFocusKey !== focusedSessionKey) {
    setSeenFocusKey(focusedSessionKey)
    setNewChatOpen(false)
  }

  // Resolves once the session is placed (true) or the start did not happen or failed (false).
  const runStart = useCallback((folder: () => Promise<string>, place: (sessionId: string) => void, launch?: (folder: string) => Promise<void>): Promise<boolean> => {
    if (!online || starting) return Promise.resolve(false)
    setStarting(true)
    setStartedFrom(new Set(sessionMap.keys()))
    return folder()
      .then((path) => startShell(path, launch))
      .then((id) => {
        if (!id) return false
        place(id)
        return true
      })
      .catch((error: unknown) => {
        toast.error(errorMessage(error, 'Could not start a shell'))
        return false
      })
      .finally(() => { setStarting(false); setStartedFrom(null) })
  }, [online, starting, setStarting, setStartedFrom, sessionMap, startShell])

  // A new session is the default agent, running in a terminal the daemon owns, shown as its chat or
  // as its terminal as Settings says. It is one session either way: the view can be switched at any
  // time without restarting the agent (ADR 0003). A plain shell is `runStart` and has no chat view.
  // The agent is the new session's first command, so it starts as soon as the shell does instead of being typed in once the
  // shell is ready. Its status hooks are connected first: an agent already running never loads them, and its chat would
  // wait on them until it was restarted.
  const yolo = useMemo(() => ({ claude: preferences.yoloClaude, codex: preferences.yoloCodex, gemini: preferences.yoloGemini }), [preferences.yoloClaude, preferences.yoloCodex, preferences.yoloGemini])
  const launchAgent = useCallback((agent: ChatAgent, flags: string) => async (folder: string): Promise<void> => {
    await connectAgentOnce(agent).catch((error: unknown) => { console.warn(`Could not connect ${agent}'s status hooks`, error) })
    await launchAgentSession(folder, agent, `${flags}${yoloFlags(agent, yolo)}`.split(' ').filter((word) => word !== ''))
  }, [yolo])

  const startAgentSession = useCallback((folder: () => Promise<string>, agent: ChatAgent, view: PaneView, firstMessage = '', flags = '', afterPlace?: (id: string) => void): Promise<boolean> => {
    return runStart(folder, (id) => {
      // A first message waits under the session and goes out once the agent takes input; its words name the session.
      if (firstMessage !== '') {
        holdFirstMessage(id, firstMessage)
        entrustFirstMessage(id, firstMessage, sendWhenAgentReady)
        void renameConversationOnDaemon({ id }, nameFromMessage(firstMessage)).catch((error: unknown) => { console.warn('Could not name the new session', error) })
      }
      // The daemon learns which agent runs from the agent's own hook, a moment after it starts; until then the
      // session is known to be this agent, so it never shows as a shell.
      setExpectedAgents((current) => ({ ...current, [id]: agent }))
      if (yoloFlags(agent, yolo) !== '') rememberYoloSession(id)
      window.setTimeout(() => { setExpectedAgents((current) => omitKey(current, id)) }, EXPECTED_AGENT_MS)
      setLayout((current) => openArrangement(current, id, makeId))
      if (view === 'chat') paneViews.choose(id, 'chat')
      // In the same update as the new session opens, so the window never shows the page between the two.
      afterPlace?.(id)
    }, launchAgent(agent, flags))
  }, [runStart, launchAgent, setExpectedAgents, yolo, setLayout, paneViews])

  // "Start a Claude chat" on a shell's empty chat card: a chat session in that shell's folder.
  const startChatHere = useCallback((from: SessionView, agent: ChatAgent, fullAccess: boolean) => {
    const refusal = remoteStartRefusal(from.machine !== LOCAL_MACHINE, from.machineName, 'A chat session')
    if (refusal) {
      toast.message(refusal)
      return
    }
    if (!online || starting) return
    // Gemini has no structured chat: it starts as a terminal session shown as its chat.
    if (agent === 'gemini') {
      void startAgentSession(() => Promise.resolve(from.cwd), agent, 'chat')
      return
    }
    setStarting(true)
    startChat(from.cwd, agent, fullAccess)
      .catch((error: unknown) => { toast.error(errorMessage(error, `Could not start a ${agent} chat`)) })
      .finally(() => { setStarting(false) })
  }, [online, starting, setStarting, startChat, startAgentSession])

  // The new chat is the agent in a terminal the daemon owns, shown as its chat: one session whose view
  // switches to the terminal at any time (ADR 0003), the same for every agent.
  const startNewChat = useCallback(async (request: NewChatRequest): Promise<void> => {
    if (!online) throw new Error('The daemon is not reachable')
    // The branch is settled before the agent starts, and a failure stays on the page with its words.
    let folder = request.folder
    if (request.branch?.worktree) folder = await addWorktree(folder, request.branch.name, request.branch.create)
    else if (request.branch) await switchBranch(folder, request.branch.name, request.branch.create)
    // The page stays, with the message in it, until the chat that replaces it is there: closing it first showed the
    // empty window (or the last chat) for a moment between the two.
    await startAgentSession(() => Promise.resolve(folder), request.agent, 'chat', request.message, launchFlags(request.agent, request.model, request.reasoning, request.mode), (id) => {
      holdStartChoice(id, { model: request.model, reasoning: request.reasoning, mode: request.agent === 'claude' ? request.mode : null })
      setNewChatOpen(false)
    })
  }, [online, setNewChatOpen, startAgentSession])

  // New session (the shortcut, the palette, the sidebar's chat button): on this computer it is the new chat page.
  // Chats are this computer's, so another machine's session is asked for in the dialog, on that machine.
  const newSession = useCallback((machine?: number) => {
    setSettingsSection(null)
    const target = machine ?? remoteTargetOf(focusedSession)?.machine ?? LOCAL_MACHINE
    if (target === LOCAL_MACHINE) {
      setNewChatOpen(true)
      return
    }
    setNewSessionMachine(target)
  }, [focusedSession, setNewChatOpen, setNewSessionMachine, setSettingsSection])

  // A plain shell in the start folder: terminal view only, whatever the default agent is.
  // Another machine asks for a folder there first, so it opens the modal.
  const newTerminal = useCallback((machine?: number) => {
    setSettingsSection(null)
    const target = machine ?? remoteTargetOf(focusedSession)?.machine ?? LOCAL_MACHINE
    if (target !== LOCAL_MACHINE) {
      setNewSessionMachine(target)
      return
    }
    void runStart(async () => resolveStartFolder(preferences.startFolder, home || (await defaultSessionFolder())), (id) => { setLayout((current) => openArrangement(current, id, makeId)) })
  }, [setSettingsSection, focusedSession, runStart, setNewSessionMachine, preferences.startFolder, home, setLayout])

  // The sidebar's header for a machine and the palette both choose where the session goes.
  const newSessionOn = useCallback((machine: number) => {
    setSettingsSection(null)
    setNewSessionMachine(machine)
  }, [setNewSessionMachine, setSettingsSection])

  // Starts a shell on another machine and opens it once the machine lists it; resolves to its id.
  const startOnMachine = useCallback(async (target: { machine: number; name: string }, folder: string): Promise<string | null> => {
    const known = new Set((machinesRef.current.get(target.machine)?.conversations ?? []).map((item) => item.id))
    await startSessionOnMachine(target.machine, folder)
    // The machine tells its new session in its next list; its layout opens when it does.
    const deadline = Date.now() + REMOTE_START_WAIT_MS
    while (Date.now() < deadline) {
      const fresh = machinesRef.current.get(target.machine)?.conversations.find((item) => item.kind === 'terminal' && !known.has(item.id))
      if (fresh) {
        setLayout((current) => openArrangement(current, fresh.id, makeId))
        return fresh.id
      }
      await new Promise((resolve) => { setTimeout(resolve, REMOTE_START_POLL_MS) })
    }
    toast.error(`${target.name} did not report the new session`)
    return null
  }, [machinesRef, setLayout])

  // What the New session modal asks for: a shell or an agent, here or on another machine. The modal closes once it is started.
  const startFromModal = useCallback(async (request: NewSessionRequest) => {
    if (request.machine !== LOCAL_MACHINE) {
      const id = await startOnMachine({ machine: request.machine, name: request.machineName }, request.folder)
      if (id !== null && request.startAs !== 'terminal') {
        if (request.message !== '') {
          holdFirstMessage(id, request.message)
          entrustFirstMessage(id, request.message, sendWhenAgentReady)
        }
        await sendTerminalInput({ id, generation: 0 }, `${request.startAs}\r`)
      }
    } else if (request.startAs === 'terminal') {
      void runStart(() => Promise.resolve(request.folder), (id) => { setLayout((current) => openArrangement(current, id, makeId)) })
    } else {
      void startAgentSession(() => Promise.resolve(request.folder), request.startAs, preferences.newSessionView, request.message)
    }
    setNewSessionMachine(null)
  }, [setNewSessionMachine, startOnMachine, runStart, setLayout, startAgentSession, preferences.newSessionView])
  return { runStart, newSession, newTerminal, startAgentSession, newSessionOn, startChatHere, startNewChat, createWorktree, startFromModal }
}
