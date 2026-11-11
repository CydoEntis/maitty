import { useEffect, useSyncExternalStore } from 'react'
import { loadAgentModels } from '@/api/daemon'
import type { ChatAgent } from '@/features/workspace/model/chatView'
import { modelLabel } from './chatMeta'

/** How much the model thinks before it answers. */
export type Reasoning = 'low' | 'medium' | 'high' | 'max'
export const REASONING_LEVELS: readonly Reasoning[] = ['low', 'medium', 'high', 'max']
/** Nothing picked: the chat keeps the effort its agent already uses, and nothing is sent. */
export const DEFAULT_PICK = 'default'
export type ReasoningPick = Reasoning | typeof DEFAULT_PICK
export const REASONING_PICKS: readonly ReasoningPick[] = [DEFAULT_PICK, ...REASONING_LEVELS]

export type ModelOption = {
  readonly id: string
  readonly label: string
  readonly blurb: string
  /** The model the agent runs when none is picked, where its own settings name one. */
  readonly isDefault?: boolean | undefined
  /** The effort levels the model takes, where the agent lists them; absent when it does not say. */
  readonly efforts?: readonly string[] | null | undefined
  /** An earlier model: the menu keeps it under "More models". */
  readonly more?: boolean | undefined
}

/**
 * The models each agent's command line accepts by name, as a fixed list: the daemon does not report
 * them. Claude takes these aliases or a full model id (`claude --model`); an alias follows the newest model of its family. The account's extra models (Fable) are read from Claude Code's config and added after these. Codex's models depend on the account and the
 * installed version, so only the account's own default is offered until the daemon can list them.
 * The first of each list is "Default", which sends nothing; the rest run strongest first.
 */
export const KNOWN_MODELS: Readonly<Record<ChatAgent, readonly ModelOption[]>> = {
  claude: [
    { id: DEFAULT_PICK, label: 'Default', blurb: 'Whatever Claude Code picks for your account' },
    { id: 'opus', label: 'Opus', blurb: 'The most capable, for hard problems' },
    { id: 'sonnet', label: 'Sonnet', blurb: 'Fast and capable for everyday work' },
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

/**
 * Replaces an agent's models with the ones read from it (after "Default", which sends nothing). Claude's read
 * models are only its extras, so they are added to the fixed aliases instead.
 */
export function setDiscoveredModels(agent: ChatAgent, models: readonly ModelOption[]): void {
  if (models.length === 0) return
  // The agent's own name and description for a model win ("Opus 5.5"). A model read by its id alone takes a fixed
  // alias's words, else is named as the chat names models.
  const read = models.filter((model) => model.id !== DEFAULT_PICK).map((model) => {
    const alias = KNOWN_MODELS[agent].find((known) => known.id === model.id)
    const label = model.label !== model.id ? model.label : (alias?.label ?? titleCase(modelLabel(model.id) ?? model.id))
    return { ...model, label, blurb: model.blurb !== '' ? model.blurb : (alias?.blurb ?? '') }
  })
  // A Default the agent described (the model it will run) says what it is, instead of the generic line.
  const described = models.find((model) => model.id === DEFAULT_PICK)
  const first = described === undefined ? KNOWN_MODELS[agent][0] as ModelOption : { ...(KNOWN_MODELS[agent][0] as ModelOption), blurb: described.blurb }
  const fixed = agent === 'claude' ? KNOWN_MODELS.claude.slice(1).filter((model) => !read.some((r) => r.id === model.id)) : []
  discovered = { ...discovered, [agent]: [first, ...read, ...fixed] }
  for (const listener of modelListeners) listener()
}

function titleCase(text: string): string {
  return text.replace(/\b[a-z]/g, (letter) => letter.toUpperCase())
}

/**
 * The rows the model menu shows. Where the agent's settings name the model it runs by default, that
 * model's row stands for "nothing picked" and there is no separate Default row; where they do not,
 * Default stays, because which model it means cannot be read.
 */
export function menuModels(models: readonly ModelOption[]): readonly ModelOption[] {
  return models.some((model) => model.isDefault === true) ? models.filter((model) => model.id !== DEFAULT_PICK) : models
}

/** The row that is ticked for a stored choice: the default's row while nothing is picked. */
export function pickedModel(models: readonly ModelOption[], choice: string): ModelOption | undefined {
  const rows = menuModels(models)
  return rows.find((model) => model.id === choice) ?? rows.find((model) => model.isDefault === true) ?? rows[0]
}

/** What picking a row stores: the default's row stores "nothing picked", so no model is sent for it. */
export function choiceFor(model: ModelOption): string {
  return model.isDefault === true ? DEFAULT_PICK : model.id
}

/**
 * Whether a model takes a reasoning level: what the agent listed for the model, where it did. Where it did
 * not, every level is offered except Max for Codex, which only some of its models take.
 */
export function effortAllowed(agent: ChatAgent, model: ModelOption | undefined, level: Reasoning): boolean {
  if (model?.efforts != null) return model.efforts.includes(level)
  return !(agent === 'codex' && level === 'max')
}

/** The models the menu shows before "More models" is opened: the newest, and the picked one wherever it is. */
export function leadingModels(rows: readonly ModelOption[], picked: string): readonly ModelOption[] {
  return rows.filter((row) => row.more !== true || row.id === picked)
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
