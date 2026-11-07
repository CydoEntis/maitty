import { useSyncExternalStore } from 'react'
import type { ChatAgent } from '@/features/workspace/model/chatView'

/** How much the model thinks before it answers. */
export type Reasoning = 'low' | 'medium' | 'high' | 'max'
export const REASONING_LEVELS: readonly Reasoning[] = ['low', 'medium', 'high', 'max']
export const DEFAULT_REASONING: Reasoning = 'medium'

export type ModelOption = { readonly id: string; readonly label: string; readonly blurb: string }

/**
 * PLACEHOLDER model lists. The host does not tell the window which models an agent offers, so these
 * are stand-ins until the adapters report their own; the first of each list is the agent's default.
 */
export const PLACEHOLDER_MODELS: Readonly<Record<ChatAgent, readonly ModelOption[]>> = {
  claude: [
    { id: 'sonnet', label: 'Sonnet', blurb: 'Fast and capable for everyday work' },
    { id: 'opus', label: 'Opus', blurb: 'The most capable, for hard problems' },
    { id: 'haiku', label: 'Haiku', blurb: 'The quickest, for small tasks' },
  ],
  codex: [
    { id: 'gpt-codex', label: 'GPT Codex', blurb: 'Tuned for coding agents' },
    { id: 'gpt', label: 'GPT', blurb: 'General purpose' },
    { id: 'gpt-mini', label: 'GPT mini', blurb: 'Small and quick' },
  ],
}

/** The reasoning levels' words for the picker. */
export const REASONING_LABEL: Readonly<Record<Reasoning, string>> = { low: 'Low', medium: 'Medium', high: 'High', max: 'Max' }

/** What a chat will use from its next message on. */
export type AgentChoice = { readonly model: string; readonly reasoning: Reasoning }

export function defaultChoice(agent: ChatAgent): AgentChoice {
  return { model: PLACEHOLDER_MODELS[agent][0]?.id ?? '', reasoning: DEFAULT_REASONING }
}

const STORAGE_KEY = 'maitty.gui.agentChoice.v1'
const listeners = new Set<() => void>()
type Stored = Record<string, { model?: string; reasoning?: Reasoning }>
let cache: Stored | null = null

function table(): Stored {
  if (cache !== null) return cache
  let loaded: Stored = {}
  try {
    const raw: unknown = JSON.parse(window.localStorage.getItem(STORAGE_KEY) ?? '{}')
    if (typeof raw === 'object' && raw !== null) loaded = raw as Stored
  } catch {
    // Unreadable storage is the same as nothing chosen.
  }
  cache = loaded
  return loaded
}

/** The choice stored for a chat, with the agent's defaults where nothing was chosen or the stored model is unknown. */
export function readChoice(conversation: number, agent: ChatAgent): AgentChoice {
  const fallback = defaultChoice(agent)
  const saved = table()[String(conversation)]
  const model = PLACEHOLDER_MODELS[agent].some((option) => option.id === saved?.model) ? (saved?.model ?? fallback.model) : fallback.model
  const reasoning = REASONING_LEVELS.find((level) => level === saved?.reasoning) ?? fallback.reasoning
  return { model, reasoning }
}

export function writeChoice(conversation: number, change: Partial<AgentChoice>): void {
  const all = table()
  all[String(conversation)] = { ...all[String(conversation)], ...change }
  cache = all
  try {
    window.localStorage.setItem(STORAGE_KEY, JSON.stringify(all))
  } catch {
    // The choice still holds for this run; only its memory across restarts is lost.
  }
  for (const listener of listeners) listener()
}

/** Drops the in-memory copy so the next read goes to storage again. For tests. */
export function forgetChoices(): void {
  cache = null
}

const subscribe = (listener: () => void) => {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

/** A chat's agent choice, kept per conversation in this window's storage, and the way to change it. */
export function useAgentChoice(conversation: number, agent: ChatAgent): [AgentChoice, (change: Partial<AgentChoice>) => void] {
  const model = useSyncExternalStore(subscribe, () => readChoice(conversation, agent).model)
  const reasoning = useSyncExternalStore(subscribe, () => readChoice(conversation, agent).reasoning)
  return [{ model, reasoning }, (change) => { writeChoice(conversation, change) }]
}
