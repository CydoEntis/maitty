import { useEffect, useSyncExternalStore } from 'react'
import { loadAgentModels } from '@/api/daemon'
import type { ChatAgent } from '@/features/workspace/model/chatView'

/** How much the model thinks before it answers. */
export type Reasoning = 'low' | 'medium' | 'high' | 'max'
export const REASONING_LEVELS: readonly Reasoning[] = ['low', 'medium', 'high', 'max']
/** Nothing picked: the chat keeps the effort its agent already uses, and nothing is sent. */
export const DEFAULT_PICK = 'default'
export type ReasoningPick = Reasoning | typeof DEFAULT_PICK
export const REASONING_PICKS: readonly ReasoningPick[] = [DEFAULT_PICK, ...REASONING_LEVELS]

export type ModelOption = { readonly id: string; readonly label: string; readonly blurb: string }

/**
 * The models each agent's command line accepts by name, as a fixed list: the daemon does not report
 * them. Claude takes these aliases (`claude --model`). Codex's models depend on the account and the
 * installed version, so only the account's own default is offered until the daemon can list them.
 * The first of each list is "Default", which sends nothing.
 */
export const KNOWN_MODELS: Readonly<Record<ChatAgent, readonly ModelOption[]>> = {
  claude: [
    { id: DEFAULT_PICK, label: 'Default', blurb: 'What this chat started with' },
    { id: 'sonnet', label: 'Sonnet', blurb: 'Fast and capable for everyday work' },
    { id: 'opus', label: 'Opus', blurb: 'The most capable, for hard problems' },
    { id: 'haiku', label: 'Haiku', blurb: 'The quickest, for small tasks' },
  ],
  codex: [
    { id: DEFAULT_PICK, label: 'Default', blurb: "Your account's model. Picking another is not offered for Codex yet" },
  ],
  gemini: [
    { id: DEFAULT_PICK, label: 'Default', blurb: "Gemini CLI's own model. Its chat is read from the terminal, so nothing is picked here" },
  ],
}

const modelListeners = new Set<() => void>()
let discovered: Partial<Record<ChatAgent, readonly ModelOption[]>> = {}

/** The models to offer for an agent: the ones its own list gave, else the fixed list above. */
export function modelsFor(agent: ChatAgent): readonly ModelOption[] {
  return discovered[agent] ?? KNOWN_MODELS[agent]
}

/** Replaces an agent's models with the ones read from it (after "Default", which sends nothing). */
export function setDiscoveredModels(agent: ChatAgent, models: readonly ModelOption[]): void {
  if (models.length === 0) return
  discovered = { ...discovered, [agent]: [KNOWN_MODELS[agent][0] as ModelOption, ...models.filter((model) => model.id !== DEFAULT_PICK)] }
  for (const listener of modelListeners) listener()
}

/** For tests. */
export function forgetDiscoveredModels(): void {
  discovered = {}
}

export function useAgentModels(agent: ChatAgent): readonly ModelOption[] {
  return useSyncExternalStore(
    (listener) => { modelListeners.add(listener); return () => { modelListeners.delete(listener) } },
    () => modelsFor(agent),
  )
}

const asked = new Set<ChatAgent>()

/** Asks the host once per agent for the models it really offers; a failure leaves the fixed list. */
export function useDiscoveredModels(agent: ChatAgent) {
  useEffect(() => {
    // Gemini has no model list to read; its chat follows the terminal.
    if (agent === 'gemini' || asked.has(agent)) return
    asked.add(agent)
    loadAgentModels(agent).then((models) => { setDiscoveredModels(agent, models) }).catch((error: unknown) => { asked.delete(agent); console.warn(`Could not list ${agent}'s models`, error) })
  }, [agent])
}

/** The reasoning levels' words for the picker. */
export const REASONING_LABEL: Readonly<Record<ReasoningPick, string>> = { default: 'Default', low: 'Low', medium: 'Medium', high: 'High', max: 'Max' }

/** What a chat will use from its next message on. */
export type AgentChoice = { readonly model: string; readonly reasoning: ReasoningPick }

export function defaultChoice(): AgentChoice {
  return { model: DEFAULT_PICK, reasoning: DEFAULT_PICK }
}

const STORAGE_KEY = 'maitty.gui.agentChoice.v1'
const listeners = new Set<() => void>()
type Stored = Record<string, { model?: string; reasoning?: ReasoningPick }>
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
  const fallback = defaultChoice()
  const saved = table()[String(conversation)]
  const model = modelsFor(agent).some((option) => option.id === saved?.model) ? (saved?.model ?? fallback.model) : fallback.model
  const reasoning = REASONING_PICKS.find((level) => level === saved?.reasoning) ?? fallback.reasoning
  return { model, reasoning }
}

/** What the daemon is told with the next message: only what was picked, or `null` when nothing was. */
export type WireChoice = { readonly model?: string; readonly effort?: string }

export function wireChoice(conversation: number, agent: ChatAgent): WireChoice | null {
  const { model, reasoning } = readChoice(conversation, agent)
  const wire: WireChoice = {
    ...(model === DEFAULT_PICK ? {} : { model }),
    ...(reasoning === DEFAULT_PICK ? {} : { effort: reasoning }),
  }
  return Object.keys(wire).length === 0 ? null : wire
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
