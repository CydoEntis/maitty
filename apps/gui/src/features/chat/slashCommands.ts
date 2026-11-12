import { useEffect, useSyncExternalStore } from 'react'
import { loadAgentCommands } from '@/api/daemon'

/** One command an agent takes after a slash, as the agent itself lists it. */
export type SlashCommand = {
  readonly name: string
  readonly description: string
  readonly hint: string
  readonly aliases: readonly string[]
  /** It opens a menu or a page in the agent's terminal, which the chat cannot show (`terminalCommands.ts`). */
  readonly terminal?: true
  /** A Codex skill: named in a message after a `$`, not run as a command. */
  readonly skill?: true
}

/** Codex lists only its skills (`agent_commands`); each is offered beside the window's list of its commands. */
export function asSkills(listed: readonly SlashCommand[]): SlashCommand[] {
  return listed.map((command) => ({ ...command, skill: true as const }))
}

/** A command as it is typed and shown: after a slash, or a skill after a `$`. */
export function commandLabel(command: SlashCommand): string {
  return `${command.skill === true ? '$' : '/'}${command.name}`
}

/** The agent's own list with the terminal's commands added, leaving out any the agent has started to list itself. */
export function withTerminalCommands(listed: readonly SlashCommand[], terminal: readonly SlashCommand[]): SlashCommand[] {
  const known = new Set(listed.flatMap((command) => [command.name, ...command.aliases]))
  return [...listed, ...terminal.filter((command) => !known.has(command.name))]
}

/**
 * The command being typed in the message box, without its slash; null when the box holds anything else. A command
 * is the first thing in the box, and the picker goes once a space ends its name: what follows is for the command.
 */
export function slashQuery(draft: string): string | null {
  return /^\/\S*$/.test(draft) ? draft.slice(1) : null
}

/** The skill being named at the start of the message box, without its `$`, as Codex's own box takes it; null otherwise. */
export function skillQuery(draft: string): string | null {
  return /^\$\S*$/.test(draft) ? draft.slice(1) : null
}

/** A message that is one line starting with a slash: the agent's own command, not words for it to read. */
/** A one-line shell command in the agent's `!` form (`!ls`, `! gh auth login`): Claude Code and Gemini run it themselves, in the terminal. */
export function isBangLine(text: string): boolean {
  return /^!\s*\S[^\n]*$/.test(text)
}

export function isSlashLine(text: string): boolean {
  return /^\/\S[^\n]*$/.test(text)
}

/** A plugin's command is written `plugin:command`; people type the part after the colon. */
const shortName = (name: string): string => name.slice(name.lastIndexOf(':') + 1)

const EXACT = 0
const NAME_STARTS = 1
const PART_STARTS = 2
const NAME_HOLDS = 3
const NO_MATCH = 4

function rank(command: SlashCommand, query: string): number {
  const name = command.name.toLowerCase()
  if (name === query) return EXACT
  if (name.startsWith(query)) return NAME_STARTS
  if (shortName(name).startsWith(query) || command.aliases.some((alias) => alias.toLowerCase().startsWith(query))) return PART_STARTS
  return name.includes(query) ? NAME_HOLDS : NO_MATCH
}

/**
 * The commands a typed name could mean, the closest first: the exact name, then names that start with it, then a
 * plugin command or another name of a command that does, then names that hold it. With nothing typed, all of
 * them by name.
 */
export function matchCommands(commands: readonly SlashCommand[], query: string): SlashCommand[] {
  const typed = query.toLowerCase()
  return commands
    .map((command) => ({ command, rank: typed === '' ? EXACT : rank(command, typed) }))
    .filter((entry) => entry.rank !== NO_MATCH)
    .sort((a, b) => a.rank - b.rank || a.command.name.localeCompare(b.command.name))
    .map((entry) => entry.command)
}

/** What picking a command leaves in the box: its name and a space, ready for what it takes. */
export function commandText(command: SlashCommand): string {
  return `${commandLabel(command)} `
}

export type SlashCommands = { readonly commands: readonly SlashCommand[]; readonly loading: boolean; readonly error: string | null }
type Listed = SlashCommands & { readonly at: number }

/** An agent's commands in a folder are asked for again after this long, so a command added meanwhile shows up. */
const STALE_MS = 300_000
const NONE: readonly SlashCommand[] = []
const NOT_ASKED: Listed = { at: 0, commands: NONE, loading: false, error: null }
const listed = new Map<string, Listed>()
const listeners = new Set<() => void>()

function put(key: string, entry: Listed): void {
  listed.set(key, entry)
  for (const listener of listeners) listener()
}

function subscribe(listener: () => void): () => void {
  listeners.add(listener)
  return () => { listeners.delete(listener) }
}

/** For tests: the lists belong to the window's lifetime. */
export function forgetSlashCommands(): void {
  listed.clear()
}

/** Asks the agent for its commands unless it is being asked, or its answer is recent; a failed ask is tried again. */
function refresh(key: string, agent: string, folder: string): void {
  const known = listed.get(key) ?? NOT_ASKED
  if (known.loading || (known.at > 0 && known.error === null && Date.now() - known.at < STALE_MS)) return
  put(key, { ...known, loading: true, error: null })
  loadAgentCommands(agent, folder).then(
    (commands) => { put(key, { at: Date.now(), commands, loading: false, error: null }) },
    (failure: unknown) => {
      console.warn("Could not list the agent's commands", failure)
      // The last list stays, with why there is no newer one.
      put(key, { at: known.at, commands: known.commands, loading: false, error: failure instanceof Error ? failure.message : String(failure) })
    },
  )
}

/**
 * The commands an agent takes in a folder, for the message box's picker. The agent is asked once the box is there
 * (it takes a few seconds), so the list is ready by the time a slash is typed; the last list shows meanwhile and is
 * asked for again when `wanted` (a slash was typed) finds it old. Null `from` asks for nothing.
 */
export function useSlashCommands(from: { readonly agent: string; readonly folder: string } | null, wanted: boolean): SlashCommands {
  const agent = from?.agent
  const folder = from?.folder
  const key = agent === undefined || folder === undefined ? null : `${agent}\n${folder}`
  useEffect(() => {
    if (key !== null && agent !== undefined && folder !== undefined) refresh(key, agent, folder)
  }, [key, agent, folder, wanted])
  return useSyncExternalStore(subscribe, () => (key === null ? NOT_ASKED : listed.get(key) ?? NOT_ASKED))
}
