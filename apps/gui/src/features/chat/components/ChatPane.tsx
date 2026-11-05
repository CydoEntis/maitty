import { MessageSquare } from 'lucide-react'
import { CHAT_AGENTS, noChatReason, type ChatAgent, type ChatAvailability } from '@/features/workspace/model/chatView'
import { Button } from '@/components/ui/button'
import { ChatSession } from './ChatSession'

export type ChatPaneProps = {
  availability: ChatAvailability
  /** What runs in the pane: the agent, or the shell's own name. */
  process: string
  /** The computer is connected to the daemon; starting an agent types into the shell. */
  online: boolean
  onBackToTerminal: () => void
  /** Types the agent's command into the shell (a shell tab only). */
  onStartAgent: (agent: ChatAgent) => void
}

/**
 * The chat card of a pane. It covers the pane's terminal, which stays mounted underneath, so
 * opening and closing it never touches the session (v18 B.3).
 */
export function ChatPane(props: ChatPaneProps) {
  const { availability } = props
  return (
    <section className="mx-chat-layer mx-chat-in" aria-label="Chat">
      {availability.kind === 'ready' ? <ChatSession key={availability.link.conversation} link={availability.link} agent={availability.agent} /> : <NoChat {...props} availability={availability} />}
    </section>
  )
}

type NoChatProps = ChatPaneProps & { availability: Exclude<ChatAvailability, { kind: 'ready' }> }

/** A session that has no chat: why, what runs here, and the way back (v18 B.3, shell tabs). */
function NoChat({ availability, process, online, onBackToTerminal, onStartAgent }: NoChatProps) {
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
          <Button type="button" size="sm" onClick={onBackToTerminal}>Back to terminal</Button>
        </div>
      </div>
    </div>
  )
}
