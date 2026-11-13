import { useState } from 'react'
import { MessageSquare } from 'lucide-react'
import { CHAT_AGENTS, noChatReason, type ChatAgent, type ChatAvailability } from '@/features/workspace/model/chatView'
import { Button } from '@/components/ui/button'
import type { OpenFile } from '../openFile'
import type { ChatItemContext } from './ChatItems'
import type { PermissionAsk } from '@/data/preview'
import type { SessionStatus } from '@/features/workspace/model/session'
import type { PaneQueue } from '../useChatComposer'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { FULL_ACCESS_LABEL, FULL_ACCESS_WARNING } from '../fullAccess'
import { AGENT_LABEL } from '../modelName'
import { ChatSession } from './ChatSession'
import type { SessionUsage } from '../usageStrip'
import type { BranchMenuSpec } from './BranchMenu'
import type { ChatProblem } from '../problem'
import { StartingChat } from './StartingChat'
import type { RewindPlace } from '../editFromHere'

export type ChatPaneProps = {
  availability: ChatAvailability
  /** The session's folder, so file names read relative to it. */
  root?: string | null
  /** Shows a file's diff, or a turn's, in the Changes panel. */
  onOpenDiff?: ChatItemContext['openDiff']
  /** Opens a file a reply names, at its line when it gave one. */
  onOpenFile?: OpenFile | null
  /** Types into the terminal the agent runs in, so a chat that follows it can talk to it (ADR 0003). */
  typeInto?: ((text: string) => Promise<void>) | undefined
  /** What the pane's daemon holds for its working agent, and the ways to add to it and take one back. */
  paneQueue?: PaneQueue | undefined
  /** What the daemon read off the screen of a pane that is stopped on a question: the text and its choices. */
  asking?: string
  /** The permission prompt the agent's hook says the pane is stopped on. */
  ask?: PermissionAsk | null
  /** The session's id, so what is typed before its agent is ready is kept for it. */
  sessionId?: string
  /** The pane's live status, from the sidebar's source. */
  paneStatus?: SessionStatus
  /** What runs in the pane: the agent, or the shell's own name. */
  process: string
  /** The computer is connected to the daemon; starting an agent types into the shell. */
  online: boolean
  onBackToTerminal: () => void
  /** False for a chat session (ADR 0003): the agent has no terminal, so nothing here points to one. */
  hasTerminal?: boolean
  /** What the session's agent reported using (context, plan limits), for the strip under the message box. */
  usage?: SessionUsage | undefined
  /** The pane runs in a linked worktree, as the daemon says; absent for a chat session, which has no pane to ask about. */
  worktree?: boolean | undefined
  /** Types the agent's command into the shell (a shell tab only). */
  onStartAgent: (agent: ChatAgent) => void
  /** Starts a chat session (a structured conversation the daemon owns) in the shell's folder (a shell tab only). */
  onStartChat?: (agent: ChatAgent, fullAccess: boolean) => void
  /** Whether the full-access switch next to the start buttons starts on (the desktop setting). */
  defaultFullAccess?: boolean
  /** The project folder's name and the branch, for the tray under the message box. */
  folder?: string
  branch?: string | null
  /** A subagent whose own chat shows instead of the session's, and the way back. */
  subagent?: SubagentView | null
  onBackToMain?: () => void
  /** Shows the session's files beside the pane. */
  onOpenFiles?: () => void
  /** What is wrong with the chat, for the banner over its message box. */
  problem?: ChatProblem | null
  /** Where the tray's branch button can move the chat. */
  branchMenu?: BranchMenuSpec | undefined
  /** The effort the agent's own files say it runs at. */
  effort?: string | null | undefined
  /** Where a sent message's Edit from here restores the files from. */
  rewind?: RewindPlace | undefined
  /** Leaves out runs of tool calls that succeeded and edited nothing (the desktop setting). */
  hideSuccessfulTools?: boolean
}

/**
 * The chat card of a pane. It covers the pane's terminal, which stays mounted underneath, so
 * opening and closing it never touches the session (v18 B.3).
 */
export function ChatPane(props: ChatPaneProps) {
  const { availability } = props
  return (
    <section className="mx-chat-layer" aria-label="Chat">
      {availability.kind === 'ready'
        ? (
          <ChatSession
            key={availability.link.conversation}
            link={availability.link}
            agent={availability.agent}
            online={props.online}
            hasTerminal={props.hasTerminal ?? true}
            root={props.root ?? null}
            onOpenDiff={props.onOpenDiff ?? null}
            onOpenFile={props.onOpenFile ?? null}
            typeInto={props.typeInto}
            paneQueue={props.paneQueue}
            ask={props.ask ?? null}
            folder={props.folder ?? ''}
            branch={props.branch ?? null}
            subagent={props.subagent ?? null}
            onOpenFolder={props.onOpenFiles ?? null}
            {...(props.worktree === undefined ? {} : { worktree: props.worktree })}
            {...(props.usage === undefined ? {} : { usage: props.usage })}
            {...(props.hasTerminal === false ? {} : { onShowTerminal: props.onBackToTerminal })}
            {...(props.onBackToMain === undefined ? {} : { onBackToMain: props.onBackToMain })}
            problem={props.problem ?? null}
            {...(props.branchMenu === undefined ? {} : { branchMenu: props.branchMenu })}
            effort={props.effort ?? null}
            hideSuccessfulTools={props.hideSuccessfulTools ?? false}
            {...(props.rewind === undefined ? {} : { rewind: props.rewind })}
            {...(props.sessionId === undefined ? {} : { sessionId: props.sessionId })}
            {...(props.paneStatus === undefined ? {} : { paneStatus: props.paneStatus })}
          />
        )
        : availability.kind === 'no-events' && props.sessionId !== undefined
          ? <StartingChat {...(props.worktree === undefined ? {} : { worktree: props.worktree })} agent={availability.agent} sessionId={props.sessionId} onBackToTerminal={props.onBackToTerminal} folder={props.folder ?? ''} branch={props.branch ?? null} root={props.root ?? null} {...(props.paneStatus === undefined ? {} : { paneStatus: props.paneStatus })} {...(props.asking === undefined ? {} : { asking: props.asking })} {...(props.typeInto === undefined ? {} : { typeInto: props.typeInto })} />
          : <NoChat {...props} availability={availability} />}
    </section>
  )
}

type NoChatProps = ChatPaneProps & { availability: Exclude<ChatAvailability, { kind: 'ready' }> }

/** A session that has no chat: why, what runs here, and the way back (v18 B.3, shell tabs). */
function NoChat({ availability, process, online, onBackToTerminal, onStartAgent, onStartChat, defaultFullAccess = false }: NoChatProps) {
  const [fullAccess, setFullAccess] = useState(defaultFullAccess)
  return (
    <div className="flex flex-1 items-center justify-center overflow-y-auto p-6">
      <div className="flex w-full max-w-[420px] flex-col items-center gap-3 text-center">
        <span className="flex size-10 items-center justify-center rounded-xl border border-line2 bg-raise text-dim" aria-hidden="true">
          <MessageSquare className="size-5" />
        </span>
        <h2 className="m-0 text-[15px] font-semibold text-fg">Chat view is for agent sessions</h2>
        <p className="m-0 text-[13px] leading-relaxed text-fg2">{noChatReason(availability)}</p>
        <p className="m-0 font-mono text-[12px] text-dim">Running here: {process}</p>
        <div className="mt-1 flex flex-wrap items-center justify-center gap-2">
          {availability.kind === 'shell' && CHAT_AGENTS.map((agent) => (
            <Button key={agent} type="button" variant="outline" size="sm" disabled={!online} onClick={() => { onStartAgent(agent) }}>
              Start {agent} here
            </Button>
          ))}
          {availability.kind === 'shell' && onStartChat !== undefined && CHAT_AGENTS.map((agent) => (
            <Button key={`chat-${agent}`} type="button" variant="outline" size="sm" disabled={!online} onClick={() => { onStartChat(agent, fullAccess) }}>
              Start a {AGENT_LABEL[agent]} chat
            </Button>
          ))}
          <Button type="button" size="sm" onClick={onBackToTerminal}>Back to terminal</Button>
        </div>
        {availability.kind === 'shell' && onStartChat !== undefined && (
          <div className="grid gap-1 text-[12px]">
            <label className="flex cursor-pointer items-center justify-center gap-2 text-fg2">
              <input type="checkbox" checked={fullAccess} onChange={(event) => { setFullAccess(event.target.checked) }} />
              {FULL_ACCESS_LABEL}
            </label>
            {fullAccess && <span role="note" className="text-[var(--danger)]">{FULL_ACCESS_WARNING}</span>}
          </div>
        )}
      </div>
    </div>
  )
}
