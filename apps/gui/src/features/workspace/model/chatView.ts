/**
 * Which view a pane shows, the terminal or the chat (v18 B.3), and whether a session has a chat to
 * show at all. Pure: the window keeps the choices, this decides what they mean.
 *
 * The chat reads the daemon's structured events (ADR 0006), never the terminal's text. So a session
 * has a chat only when the daemon holds a conversation for it, and only for the agents that speak
 * an event stream or write a transcript the daemon follows: Claude Code, Codex and Gemini CLI. Any
 * other agent, and a plain shell, stay terminal-only.
 */

export type PaneView = 'terminal' | 'chat'
/** What a pane's switch can show: its view, or the Work board of a session that reported workflow runs. */
export type PaneShow = PaneView | 'work'
export type ChatAgent = 'claude' | 'codex' | 'gemini'

export const CHAT_AGENTS: readonly ChatAgent[] = ['claude', 'codex', 'gemini']

/** The agents the daemon can also run as a structured chat with no terminal (`startChatSession`); the rest are a terminal session shown as a chat. */
export type StructuredAgent = Exclude<ChatAgent, 'gemini'>

/** A per-agent choice of the view a session opens in. An agent opens as its chat unless the user changes it. */
export type ChatViewDefaults = Readonly<Record<ChatAgent, PaneView>>

export const DEFAULT_CHAT_VIEWS: ChatViewDefaults = { claude: 'chat', codex: 'chat', gemini: 'chat' }

/** The daemon conversation that carries a session's structured events. */
export type ChatLink = {
  /** The conversation's key: its number on this computer, or another machine's number kept apart from it (`api/conversationKey.ts`). */
  readonly conversation: number
  readonly generation: number
  /**
   * The conversation follows an agent the user started in the terminal (ADR 0003): the chat shows what
   * it did, but sending, answering and interrupting happen in the terminal.
   */
  readonly readOnly?: boolean
  /** The machine the agent runs on, so attachments are sent there; absent for this computer. */
  readonly machine?: string | null
}

export type ChatAvailability =
  /** The daemon holds this session's conversation: the chat can read and send. */
  | { readonly kind: 'ready'; readonly agent: ChatAgent; readonly link: ChatLink }
  /** A Claude, Codex or Gemini session the daemon has no conversation for, so there is no event stream to show. */
  | { readonly kind: 'no-events'; readonly agent: ChatAgent }
  /** An agent without a structured event stream. */
  | { readonly kind: 'unsupported'; readonly agent: string }
  /** A plain shell. */
  | { readonly kind: 'shell' }

export type ChatSource = {
  /** The agent the daemon reports in the session; null for a plain shell. */
  readonly agent: string | null | undefined
  readonly chat: ChatLink | null | undefined
}

export function chatAgentOf(agent: string | null | undefined): ChatAgent | null {
  const name = agent?.trim().toLowerCase()
  return CHAT_AGENTS.find((candidate) => candidate === name) ?? null
}

export function chatAvailability(source: ChatSource): ChatAvailability {
  const agent = chatAgentOf(source.agent)
  if (agent === null) {
    const other = source.agent?.trim()
    return other ? { kind: 'unsupported', agent: other } : { kind: 'shell' }
  }
  return source.chat ? { kind: 'ready', agent, link: source.chat } : { kind: 'no-events', agent }
}

/**
 * The view a pane shows. An explicit choice wins; otherwise a session whose chat is ready opens in
 * its agent's default, and everything else opens as the terminal it is.
 */
export function effectiveView(choice: PaneView | undefined, availability: ChatAvailability, defaults: ChatViewDefaults): PaneView {
  if (choice !== undefined) return choice
  return availability.kind === 'ready' ? defaults[availability.agent] : 'terminal'
}

/** The words the empty chat card gives for a session that has no chat. */
export function noChatReason(availability: Exclude<ChatAvailability, { kind: 'ready' }>): string {
  switch (availability.kind) {
    case 'shell':
      return 'Chat view is for agent sessions. Start claude or codex here to use it.'
    case 'unsupported':
      return `Chat view is for agent sessions. ${availability.agent} has no event stream to show, so it stays in the terminal.`
    case 'no-events':
      return `The chat fills in as soon as ${availability.agent} is ready. The terminal works as usual.`
  }
}

/** How long a new session may take to report in before the chat suggests its hooks are the problem. */
export const NO_EVENTS_HINT_AFTER_MS = 10_000

/** Shown once a session has stayed empty past `NO_EVENTS_HINT_AFTER_MS`: the usual cause is hooks from another version. */
export const noEventsHint = (agent: ChatAgent): string => `Still nothing. ${agent} may be waiting for you in the terminal, or its status hooks are not connected or are from another version. Check the terminal, connect them here, or run "maitty integrate ${agent}".`

/** Shown instead when the daemon already knows the agent is stopped for an answer (a trust-this-folder question, a login). */
export const waitingInTerminalHint = (agent: ChatAgent): string => `${agent} is waiting for you in the terminal, for example to trust this folder. Answer it there and the chat takes over.`

/** What a session tells the chat view about itself, from the fields every session view carries. */
export function chatSourceOf(session: { conversation: { agent?: string | null; chat?: ChatLink | undefined } }): ChatSource {
  return { agent: session.conversation.agent, chat: session.conversation.chat }
}
