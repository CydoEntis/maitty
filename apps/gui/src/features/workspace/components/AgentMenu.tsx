import { useCallback, useState } from 'react'
import { ChevronDownIcon } from 'lucide-react'
import { DropdownMenu, DropdownMenuContent, DropdownMenuItem, DropdownMenuTrigger } from '@/components/ui/dropdown-menu'
import { CHAT_AGENTS, type ChatAgent } from '../model/chatView'
import { AppIcon } from './glyphs'

const AGENT_NAME: Readonly<Record<ChatAgent, string>> = { claude: 'Claude', codex: 'Codex' }
const MENU_ICON_PX = 18

type AgentMenuProps = {
  disabled: boolean
  /** The agent the + button starts, marked in the list. */
  defaultAgent: ChatAgent
  onPick: (agent: ChatAgent) => void
}

/** Next to +: start a chat with Claude or Codex without changing the default in Settings. */
export function AgentMenu({ disabled, defaultAgent, onPick }: AgentMenuProps) {
  // The list mounts inside the app root, where the theme's custom properties are set.
  const [container, setContainer] = useState<HTMLElement | null>(null)
  const findRoot = useCallback((button: HTMLButtonElement | null) => { setContainer(button?.closest<HTMLElement>('.mx-app') ?? null) }, [])
  return (
    <DropdownMenu>
      <DropdownMenuTrigger asChild>
        <button ref={findRoot} type="button" className="mx-new-button" aria-label="Choose an agent for a new session" title="Start a Claude or Codex chat" disabled={disabled}>
          <ChevronDownIcon className="size-3.5" aria-hidden="true" />
        </button>
      </DropdownMenuTrigger>
      <DropdownMenuContent container={container} align="end" aria-label="New chat with" className="bg-raise text-fg ring-pane-line [backdrop-filter:var(--blur-pop)]">
        {CHAT_AGENTS.map((agent) => (
          <DropdownMenuItem key={agent} onSelect={() => { onPick(agent) }}>
            <AppIcon process={agent} size={MENU_ICON_PX} />
            New {AGENT_NAME[agent]} chat{agent === defaultAgent ? ' (default)' : ''}
          </DropdownMenuItem>
        ))}
      </DropdownMenuContent>
    </DropdownMenu>
  )
}
