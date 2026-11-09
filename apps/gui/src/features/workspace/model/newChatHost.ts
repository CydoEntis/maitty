import { useCallback, useEffect, useState } from 'react'
import { agentsInstalled, folderBranches, type FolderBranches } from '@/api/daemon'
import { DEFAULT_PICK, type ReasoningPick } from '@/features/chat/agentChoice'
import { PICKABLE_MODES } from '@/features/chat/chatMeta'
import type { ChatAgent } from './chatView'
import type { NewSessionProject } from './newSession'

/** The agents the new chat page knows by program name, in the order the list shows them. */
export const AGENT_CATALOG: readonly { readonly id: string; readonly label: string }[] = [
  { id: 'claude', label: 'Claude Code' },
  { id: 'codex', label: 'Codex' },
  { id: 'gemini', label: 'Gemini CLI' },
  { id: 'opencode', label: 'OpenCode' },
  { id: 'cursor-agent', label: 'Cursor Agent' },
]

/** Projects the list calls recent: the focused one and the newest. */
const RECENT_PROJECTS = 3

/** The projects split into the few used most recently and the rest. `projects` is already newest first. */
export function splitRecent(projects: readonly NewSessionProject[]): { recent: NewSessionProject[]; rest: NewSessionProject[] } {
  return { recent: projects.slice(0, RECENT_PROJECTS), rest: projects.slice(RECENT_PROJECTS) }
}

const MODEL_ID = /^[A-Za-z0-9][A-Za-z0-9._:/[\]-]*$/
const CLAUDE_EFFORT = new Set<ReasoningPick>(['low', 'medium', 'high', 'max'])
const CODEX_EFFORT = new Set<ReasoningPick>(['low', 'medium', 'high'])

/**
 * The flags that put a picked model and effort on an agent's launch line (the text after its name,
 * with a leading space, or nothing). Claude: `--model`, `--effort`. Codex: `-m`, and the effort as
 * the `model_reasoning_effort` setting, which has no Max. A name that is not plain model-id
 * characters is dropped rather than typed into a shell, and so is any level the agent lacks.
 */
export function launchFlags(agent: ChatAgent, model: string, effort: ReasoningPick, mode = 'default'): string {
  const flags: string[] = []
  if (agent === 'claude' && mode !== 'default' && PICKABLE_MODES.includes(mode)) flags.push('--permission-mode', mode)
  const named = model !== DEFAULT_PICK && MODEL_ID.test(model)
  if (agent === 'claude') {
    if (named) flags.push('--model', model)
    if (CLAUDE_EFFORT.has(effort)) flags.push('--effort', effort)
  } else if (agent === 'codex') {
    if (named) flags.push('-m', model)
    if (CODEX_EFFORT.has(effort)) flags.push('-c', `model_reasoning_effort=${effort}`)
  }
  return flags.length === 0 ? '' : ` ${flags.join(' ')}`
}

/** Each agent's own flag for yolo mode. Claude's only allows Bypass permissions (Shift+Tab reaches it); Codex and Gemini start with it on. */
const YOLO_FLAG: Readonly<Record<ChatAgent, string>> = {
  claude: '--allow-dangerously-skip-permissions',
  codex: '--dangerously-bypass-approvals-and-sandbox',
  gemini: '--yolo',
}

/** The launch flag of the agent's yolo mode, or nothing when that agent's yolo setting is off. */
export function yoloFlags(agent: ChatAgent, on: Readonly<Record<ChatAgent, boolean>>): string {
  return on[agent] ? ` ${YOLO_FLAG[agent]}` : ''
}

const MINUTE = 60
const HOUR = 3600
const DAY = 86_400
const WEEK = 604_800
const TWO_WEEKS = 1_209_600
const MS_PER_SECOND = 1000

/** Now as Unix seconds, to measure `ageOf` against. */
export const nowSeconds = (): number => Math.floor(Date.now() / MS_PER_SECOND)

/** "4m", "1h", "1d", "3w": how long ago a Unix time was; empty when unknown. */
export function ageOf(when: number | null, now: number): string {
  if (when === null) return ''
  const seconds = Math.max(0, now - when)
  if (seconds < HOUR) return `${String(Math.max(1, Math.floor(seconds / MINUTE)))}m`
  if (seconds < DAY) return `${String(Math.floor(seconds / HOUR))}h`
  if (seconds < TWO_WEEKS) return `${String(Math.floor(seconds / DAY))}d`
  return `${String(Math.floor(seconds / WEEK))}w`
}

/** Which agent programs are on this computer, or null while that is unknown (or could not be read). */
export function useInstalledAgents(): readonly string[] | null {
  const [installed, setInstalled] = useState<readonly string[] | null>(null)
  useEffect(() => {
    const stop = new AbortController()
    void (async () => {
      try {
        const found = await agentsInstalled()
        if (!stop.signal.aborted) setInstalled(found)
      } catch (error) {
        console.warn('Could not look for the installed agents', error)
      }
    })()
    return () => { stop.abort() }
  }, [])
  return installed
}

/** A folder's branches and worktrees (null when it is not a git repository or is still being read), and a way to read them again. */
export function useFolderBranches(folder: string): [FolderBranches | null, () => void] {
  const [read, setRead] = useState<{ folder: string; data: FolderBranches | null }>({ folder: '', data: null })
  const [again, setAgain] = useState(0)
  useEffect(() => {
    if (folder === '') return
    const stop = new AbortController()
    void (async () => {
      try {
        const data = await folderBranches(folder)
        if (!stop.signal.aborted) setRead({ folder, data })
      } catch (error) {
        if (!stop.signal.aborted) setRead({ folder, data: null })
        console.warn('Could not read the folder\'s branches', error)
      }
    })()
    return () => { stop.abort() }
  }, [folder, again])
  const reload = useCallback(() => { setAgain((count) => count + 1) }, [])
  return [read.folder === folder ? read.data : null, reload]
}
