import { CODEX_START_MODES, PICKABLE_MODES } from '@/features/chat/chatMeta'
import { ANSWER_KEYS, type AskChoice } from '@/features/chat/permissionAsk'
import type { ChatAgent, StructuredAgent } from './chatView'

/** Where the composer's slash commands come from, and how Codex's differ. */
export type SlashCommands = {
  /** `agent`: the agent lists its own, in its folder. `window`: a fixed list the window keeps. */
  readonly list: 'agent' | 'window'
  /** A `$` and a name opens the list too, with only the agent's skills. */
  readonly skillsAfterDollar: boolean
  /** A typed command is taken only when its Enter comes a moment after it. */
  readonly enterAfterPause: boolean
}

export type PermissionModes = {
  readonly modes: readonly string[]
  /** How a running agent's mode is changed from the chat: Shift+Tab until its screen shows it, or its own command. */
  readonly switchBy: 'shift-tab' | 'command'
}

/** A letter the sidebar's answer buttons send. */
export type SidebarAnswer = 'y' | 'a' | 'n'

export type AgentAbilities = {
  readonly slashCommands: SlashCommands | null
  readonly permissionModes: PermissionModes | null
  /** The daemon can run the agent as a chat with no terminal; otherwise its chat is a terminal session shown as one. */
  readonly structuredChat: boolean
  readonly moveToWorktree: boolean
  /** The keys the chat types to answer the agent's permission menu, verified against that agent; null where none are. */
  readonly answerKeys: Readonly<Record<AskChoice, string>> | null
  /**
   * The keys the sidebar types for its answer buttons. They disagree with `answerKeys`: for Claude the sidebar
   * sends `n` and `a` as letters, which its menu ignores, and for every other agent it sends unverified letters.
   */
  readonly sidebarAnswerKeys: Readonly<Record<SidebarAnswer, string>>
  /** A chat that follows the agent's terminal changes its model and effort from the chips, by typing the agent's commands. */
  readonly changeAgent: boolean
  /** The agent's prompt takes a typed `!` line as a command for its own shell, run in its terminal. */
  readonly bangToTerminal: boolean
  /** This computer's daemon lists the agent's saved conversations, and one can be resumed from the New chat page. */
  readonly resumeSaved: boolean
  /** The host can ask the agent which models it offers; otherwise the fixed list stands. */
  readonly discoversModels: boolean
  /** The agent reports what it asks through its own hooks, so its screen is never read for the question. */
  readonly hookedAsks: boolean
}

const LETTERS: Readonly<Record<SidebarAnswer, string>> = { y: 'y', a: 'a', n: 'n' }

const NONE: AgentAbilities = {
  slashCommands: null,
  permissionModes: null,
  structuredChat: false,
  moveToWorktree: false,
  answerKeys: null,
  sidebarAnswerKeys: LETTERS,
  changeAgent: false,
  bangToTerminal: false,
  resumeSaved: false,
  discoversModels: false,
  hookedAsks: false,
}

const AGENT_ABILITIES: Readonly<Record<ChatAgent, AgentAbilities>> = {
  claude: {
    slashCommands: { list: 'agent', skillsAfterDollar: false, enterAfterPause: false },
    permissionModes: { modes: PICKABLE_MODES, switchBy: 'shift-tab' },
    structuredChat: true,
    moveToWorktree: true,
    answerKeys: ANSWER_KEYS,
    sidebarAnswerKeys: { ...LETTERS, y: ANSWER_KEYS.approve },
    changeAgent: true,
    bangToTerminal: true,
    resumeSaved: true,
    discoversModels: true,
    hookedAsks: true,
  },
  codex: {
    slashCommands: { list: 'window', skillsAfterDollar: true, enterAfterPause: true },
    permissionModes: { modes: CODEX_START_MODES, switchBy: 'command' },
    structuredChat: true,
    moveToWorktree: true,
    answerKeys: null,
    sidebarAnswerKeys: LETTERS,
    changeAgent: false,
    bangToTerminal: false,
    resumeSaved: true,
    discoversModels: true,
    hookedAsks: false,
  },
  gemini: { ...NONE, bangToTerminal: true },
}

/**
 * What an agent can do, by the name the daemon reports. The name must match exactly, as the checks this
 * replaces compare it; any other agent, and a plain shell, can do none of it. Pass the raw
 * `session.process` or a `ChatAgent` (already trimmed and lower-cased by `chatAgentOf`), never a
 * display label.
 */
export function abilitiesOf(agent: string | null | undefined): AgentAbilities {
  return agent != null && Object.hasOwn(AGENT_ABILITIES, agent) ? AGENT_ABILITIES[agent as ChatAgent] : NONE
}

/** The abilities only a structured agent has, so the daemon calls behind them take a `StructuredAgent`. */
export type StructuredAbility = 'structuredChat' | 'resumeSaved' | 'discoversModels'

/** Whether the agent has the ability, narrowing it to a `StructuredAgent` for the daemon call behind it. */
export function hasAbility(agent: ChatAgent, ability: StructuredAbility): agent is StructuredAgent {
  return abilitiesOf(agent)[ability]
}
