import { useCallback, useEffect, useState } from 'react'
import { agentsInstalled, folderBranches, folderSyncState, pullFolder, savedConversations, type FolderBranches, type SavedConversation, type SyncState } from '@/api/daemon'
import { DEFAULT_PICK, type ReasoningPick } from '@/features/chat/agentChoice'
import { CODEX_FULL_MODE, PICKABLE_MODES } from '@/features/chat/chatMeta'
import type { ChatAgent } from './chatView'
import type { NewSessionProject } from './newSession'

/** Projects the list calls recent: the focused one and the newest. */
const RECENT_PROJECTS = 3

/** The projects split into the few used most recently and the rest. `projects` is already newest first. */
export function splitRecent(projects: readonly NewSessionProject[]): { recent: NewSessionProject[]; rest: NewSessionProject[] } {
  return { recent: projects.slice(0, RECENT_PROJECTS), rest: projects.slice(RECENT_PROJECTS) }
}

const MODEL_ID = /^[A-Za-z0-9][A-Za-z0-9._:/[\]-]*$/
const CLAUDE_EFFORT = new Set<ReasoningPick>(['low', 'medium', 'high', 'max'])
const CODEX_EFFORT = new Set<ReasoningPick>(['low', 'medium', 'high', 'max'])
/** What each Codex start mode puts on the launch line: its sandbox and when it asks. Auto is absent: it sends nothing, so the settings Codex has stand. */
const CODEX_MODE_FLAGS: Readonly<Record<string, readonly string[]>> = {
  codexReadOnly: ['--sandbox', 'read-only', '--ask-for-approval', 'on-request'],
  [CODEX_FULL_MODE]: ['--sandbox', 'danger-full-access', '--ask-for-approval', 'never'],
}

/**
 * The flags that put a picked permission mode, model and effort on an agent's launch line (the text after its name,
 * with a leading space, or nothing). Claude: `--model`, `--effort`. Codex: `-m`, and the effort as
 * the `model_reasoning_effort` setting. A name that is not plain model-id
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
    flags.push(...(CODEX_MODE_FLAGS[mode] ?? []))
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
/** One spelling of a path, to compare what git wrote with what the session has: forward slashes, no trailing one, and no case (Windows has none). */
const samePath = (path: string): string => path.replace(/\\/g, '/').replace(/\/+$/, '').toLowerCase()

/** Whether a folder is a worktree of its repository and not the main checkout; null when git has not said, or it is not a repository's folder. */
export function isWorktree(branches: FolderBranches | null, folder: string | null): boolean | null {
  if (branches === null || folder === null) return null
  const here = branches.worktrees.find((tree) => samePath(tree.path) === samePath(folder))
  return here === undefined ? null : !here.main
}

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

/** What the page knows of a folder's saved conversations: nothing yet, the list, or why there is none. */
export type SavedRead = { kind: 'none' } | { kind: 'listed'; list: readonly SavedConversation[] } | { kind: 'older' } | { kind: 'failed' }

const NOT_READ: SavedRead = { kind: 'none' }

/** The conversations `agent` saved for `folder`, read off the UI thread; a null agent asks nothing. An answer for another folder or agent is dropped. */
export function useSavedConversations(folder: string, agent: 'claude' | 'codex' | null): SavedRead {
  const key = agent === null || folder === '' ? '' : `${agent}\n${folder}`
  const [read, setRead] = useState<{ key: string; found: SavedRead }>({ key: '', found: NOT_READ })
  useEffect(() => {
    if (agent === null || folder === '') return undefined
    const asked = `${agent}\n${folder}`
    const stop = new AbortController()
    void (async () => {
      try {
        const list = await savedConversations(agent, folder)
        if (!stop.signal.aborted) setRead({ key: asked, found: list === null ? { kind: 'older' } : { kind: 'listed', list } })
      } catch (error) {
        if (!stop.signal.aborted) setRead({ key: asked, found: { kind: 'failed' } })
        console.warn('Could not read the saved conversations', error)
      }
    })()
    return () => { stop.abort() }
  }, [agent, folder])
  return key !== '' && read.key === key ? read.found : NOT_READ
}

/**
 * How a folder's checkout stands against its upstream. With `fetch` the remote is asked once when the folder is
 * picked (off the UI thread, in the host), so the page can say "fetched just now"; without it only what git already
 * knows is read. `pull` fast-forwards and reads again.
 */
export function useSyncState(folder: string, fetch: boolean): { state: SyncState | null; busy: boolean; pull: () => Promise<void> } {
  const [read, setRead] = useState<{ folder: string; state: SyncState | null }>({ folder: '', state: null })
  const [busy, setBusy] = useState(false)
  useEffect(() => {
    if (folder === '') return undefined
    const stop = new AbortController()
    void (async () => {
      try {
        const state = await folderSyncState(folder, fetch)
        if (!stop.signal.aborted) setRead({ folder, state })
      } catch (error) {
        if (!stop.signal.aborted) setRead({ folder, state: null })
        console.warn('Could not read how the folder stands against its upstream', error)
      }
    })()
    return () => { stop.abort() }
  }, [folder, fetch])
  const pull = useCallback(async () => {
    setBusy(true)
    try {
      await pullFolder(folder)
      setRead({ folder, state: await folderSyncState(folder, false) })
    } finally {
      setBusy(false)
    }
  }, [folder])
  return { state: read.folder === folder ? read.state : null, busy, pull }
}

/** The branch and how far behind its upstream each of several folders is, for the page's "switch to" chips. Nothing is fetched. */
export function useGlances(folders: readonly string[]): ReadonlyMap<string, SyncState> {
  const key = folders.join('\n')
  const [found, setFound] = useState<ReadonlyMap<string, SyncState>>(() => new Map())
  useEffect(() => {
    const stop = new AbortController()
    for (const folder of key === '' ? [] : key.split('\n')) {
      void (async () => {
        try {
          const state = await folderSyncState(folder, false)
          if (state !== null && !stop.signal.aborted) setFound((before) => new Map(before).set(folder, state))
        } catch (error) {
          console.warn('Could not read a project\'s branch', error)
        }
      })()
    }
    return () => { stop.abort() }
  }, [key])
  return found
}
