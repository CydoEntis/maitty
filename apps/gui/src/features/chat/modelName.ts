import type { ChatAgent } from '@/features/workspace/model/chatView'
import { DEFAULT_PICK, type ModelOption } from './agentChoice'
import { modelLabel } from './chatMeta'

export const AGENT_LABEL: Readonly<Record<ChatAgent, string>> = { claude: 'Claude', codex: 'Codex', gemini: 'Gemini' }

/** "Sonnet 5.5" from the agent's own words for its model. */
function titleCase(text: string): string {
  return text.replace(/\b[a-z]/g, (letter) => letter.toUpperCase())
}

/**
 * The model the message box names: the one picked, else the one the agent's own file says it runs,
 * else just the agent, so it never claims a model the chat is not using.
 */
export function modelName(agent: ChatAgent, picked: ModelOption | undefined, current: string | null): string {
  if (picked !== undefined && picked.id !== DEFAULT_PICK) return `${AGENT_LABEL[agent]} ${picked.label}`
  const said = modelLabel(current)
  return said === null ? AGENT_LABEL[agent] : `${AGENT_LABEL[agent]} ${titleCase(said)}`
}
