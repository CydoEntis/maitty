/**
 * Which view a pane shows, the terminal or the chat (v18 B.3), and whether a session has a chat to
 * show at all. Pure: the window keeps the choices, this decides what they mean.
 *
 * The chat reads the daemon's structured events (ADR 0006), never the terminal's text. So a session
 * has a chat only when the daemon holds a conversation for it, and only for the agents that speak
 * an event stream: Claude Code and Codex. Any other agent, and a plain shell, stay terminal-only.
 */

export type PaneView = 'terminal' | 'chat'
export type ChatAgent = 'claude' | 'codex'

export const CHAT_AGENTS: readonly ChatAgent[] = ['claude', 'codex']

/** A per-agent choice of the view a session opens in. Terminal unless the user changes it. */
export type ChatViewDefaults = Readonly<Record<ChatAgent, PaneView>>

export const DEFAULT_CHAT_VIEWS: ChatViewDefaults = { claude: 'terminal', codex: 'terminal' }

/** The daemon conversation that carries a session's structured events. */
export type ChatLink = {
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
  /** A Claude or Codex session the daemon has no conversation for, so there is no event stream to show. */
  | { readonly kind: 'no-events'; readonly agent: ChatAgent }
  /** The session lives on another machine; its conversation cannot be read from here in v1. */
  | { readonly kind: 'remote'; readonly agent: ChatAgent | null; readonly machineName: string }
  /** An agent without a structured event stream. */
  | { readonly kind: 'unsupported'; readonly agent: string }
  /** A plain shell. */
  | { readonly kind: 'shell' }

export type ChatSource = {
  /** The agent the daemon reports in the session; null for a plain shell. */
  readonly agent: string | null | undefined
  /** The machine's name; null for this computer. */
  readonly machineName: string | null
  readonly chat: ChatLink | null | undefined
}

export function chatAgentOf(agent: string | null | undefined): ChatAgent | null {
  const name = agent?.trim().toLowerCase()
  return CHAT_AGENTS.find((candidate) => candidate === name) ?? null
}

export function chatAvailability(source: ChatSource): ChatAvailability {
  const agent = chatAgentOf(source.agent)
  if (source.machineName !== null) return { kind: 'remote', agent, machineName: source.machineName }
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
    case 'remote':
      return `Chat view is local-only in v1: this session runs on ${availability.machineName}, so its conversation is not read from this computer. The terminal works as usual.`
    case 'no-events':
      return `Chat view is for agent sessions. The daemon has no conversation for this ${availability.agent} session, so there is no event stream to show. The terminal works as usual.`
  }
}

/** What a session tells the chat view about itself, from the fields every session view carries. */
export function chatSourceOf(session: { machineName: string | null; conversation: { agent?: string | null; chat?: ChatLink | undefined } }): ChatSource {
  return { agent: session.conversation.agent, machineName: session.machineName, chat: session.conversation.chat }
}
