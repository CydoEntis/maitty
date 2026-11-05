import { useMemo, type ReactNode } from 'react'
import { PanelRightClose } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { AgentsCard } from '@/features/chat/components/AgentsCard'
import { useChatFeed } from '@/features/chat/useChatFeed'
import { useTipWithKeys } from '@/features/shortcuts/platform'
import { AppIcon } from '@/features/workspace/components/glyphs'
import type { ChatAvailability, ChatAgent, ChatLink } from '@/features/workspace/model/chatView'
import { ChecklistCard } from './ChecklistCard'
import { checklistOf, currentItemSince, emptyReason, helperAgentsOf } from './panelModel'

/** The docked panel is 320px wide (v18 C.2). */
export const PANEL_WIDTH_PX = 320

type Props = {
  mode: 'docked' | 'overlay'
  /** The session's name and what runs in it. */
  title: string
  process: string
  availability: ChatAvailability
  onClose: () => void
}

function Header({ title, process, agent, onClose }: { title: string; process: string; agent: ChatAgent | null; onClose: () => void }) {
  const tip = useTipWithKeys()
  return (
    <header className="flex h-[46px] shrink-0 items-center gap-2.5 border-b border-line2 px-3">
      {agent !== null && <AppIcon process={agent} size={24} />}
      <div className="min-w-0 flex-1">
        <h2 className="m-0 truncate text-[13px] font-semibold text-fg">{title}</h2>
        <p className="m-0 truncate font-mono text-[11px] text-dim">{process}</p>
      </div>
      <Button type="button" variant="ghost" size="icon" className="size-8 text-dim hover:text-fg" aria-label={tip('Hide session panel', 'session-panel')} onClick={onClose}>
        <PanelRightClose className="size-4" aria-hidden="true" />
      </Button>
    </header>
  )
}

function Nothing({ children }: { children: ReactNode }) {
  return (
    <div className="px-1 py-6 text-center">
      <p className="m-0 text-[13px] font-semibold text-fg2">Nothing to track yet</p>
      <p className="mt-1.5 mb-0 text-[12.5px] leading-relaxed text-dim">{children}</p>
    </div>
  )
}

/** What the session's own events say: its checklist and helper agents. Mounted only for a session with a chat link. */
function AgentBody({ link, agent }: { link: ChatLink; agent: ChatAgent }) {
  const { feed, events } = useChatFeed({ conversation: link.conversation, generation: link.generation, provider: agent })
  const working = feed.status === 'Working'
  const { checklist, unreadable } = useMemo(() => checklistOf(events), [events])
  const helpers = useMemo(() => helperAgentsOf(events), [events])
  const since = useMemo(() => (checklist === null ? null : currentItemSince(events, checklist)), [events, checklist])

  if (!feed.loaded) return <p className="m-0 px-1 py-6 text-center text-[12.5px] text-dim">Reading the conversation…</p>
  return (
    <>
      {checklist !== null && <ChecklistCard checklist={checklist} working={working} currentSince={since} />}
      {checklist === null && unreadable && <p className="m-0 rounded-[10px] border border-line2 bg-raise px-3 py-2 text-[12px] text-dim">The agent made a checklist, but the conversation history did not keep its items.</p>}
      {helpers.length > 0 && <AgentsCard agents={helpers} panel />}
      {checklist === null && !unreadable && helpers.length === 0 && (
        <Nothing>The agent has made no checklist and started no helper agents in this session.</Nothing>
      )}
    </>
  )
}

/**
 * The session panel (v18 C.2) for the focused session: its agent's checklist and helper agents, read
 * from the daemon's conversation events. A session with no agent or no event stream says why there is
 * nothing to show. Docked beside the panes on a wide window, an overlay on a compact one.
 */
export function SessionPanel({ mode, title, process, availability, onClose }: Props) {
  const agent = availability.kind === 'ready' ? availability.agent : null
  const body = (
    <aside
      aria-label="Session panel"
      className={`mx-card flex min-h-0 flex-col overflow-hidden ${mode === 'docked' ? 'm-[var(--edge)] shrink-0' : 'h-full'}`}
      style={{ width: PANEL_WIDTH_PX }}
    >
      <Header title={title} process={process} agent={agent} onClose={onClose} />
      <div className="flex min-h-0 flex-1 flex-col gap-3 overflow-y-auto p-3">
        {availability.kind === 'ready'
          ? <AgentBody key={availability.link.conversation} link={availability.link} agent={availability.agent} />
          : <Nothing>{emptyReason(availability)}</Nothing>}
      </div>
    </aside>
  )
  if (mode === 'docked') return body
  // Compact: over a 40% scrim, from the window's edge (v18 C.2). A press on the scrim closes it; Esc is the window's ladder.
  return (
    <div className="mx-drawer-scrim" onMouseDown={(event) => { if (event.target === event.currentTarget) onClose() }}>
      <div className="absolute top-10 right-2 bottom-2 max-w-[calc(100%-16px)] [.mx-app[data-side=right]_&]:right-auto [.mx-app[data-side=right]_&]:left-2">{body}</div>
    </div>
  )
}
