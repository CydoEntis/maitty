import { type Dispatch, type SetStateAction, useCallback, useMemo } from 'react'
import { type Platform, type ShortcutAction, shortcutText } from './model/shortcuts'
import type { MachineHead } from './model/machines'
import { toast } from '@/lib/toast'
import type { DesktopSettingsSection } from '@/features/settings/components/DesktopSettings'
import type { SettingsPreferences } from '@/features/settings/preferences'
import { activeArrangement, type Layout, setColumnWidths, type SplitDirection } from './model/layout'
import { type SessionView, tildePath } from './model/session'
import { byUrgency, isShell, rowTitle } from './model/sidebar'
import { outputSnippet, type PaletteItem } from './model/palette'
import type { SearchHit } from '@/features/find/api'
import { THEMES } from './theme/themes'

/** The command palette's list: sessions, their projects, actions and themes. */
export function usePaletteItems({ paletteOpen, platform, layout, sessions, projectOf, showSessionById, home, newSession, focusedMachineNumber, newTerminal, focusedSession, addAgentTerminal, runShortcut, heads, online, preferences, setPreferences, split, setSettingsSection, inspectRefusal, setInspectOpen, setFindOpen, setFocusMode, setShortcutsOpen, setLayout, startAgent, setRecentOpen }: {
  paletteOpen: boolean
  platform: Platform
  layout: Layout
  sessions: SessionView[]
  projectOf: Map<string, { name: string; key: string }>
  showSessionById: (sessionId: string) => void
  home: string
  newSession: (machine?: number) => void
  focusedMachineNumber: number
  newTerminal: (machine?: number) => void
  focusedSession: SessionView | undefined
  addAgentTerminal: (agentId: string) => void
  runShortcut: (action: ShortcutAction) => void
  heads: MachineHead[]
  online: boolean
  preferences: SettingsPreferences
  setPreferences: Dispatch<SetStateAction<SettingsPreferences>>
  split: (paneId: string | null, direction: SplitDirection) => void
  setSettingsSection: Dispatch<SetStateAction<DesktopSettingsSection | null>>
  inspectRefusal: string | null
  setInspectOpen: Dispatch<SetStateAction<boolean>>
  setFindOpen: Dispatch<SetStateAction<boolean>>
  setFocusMode: Dispatch<SetStateAction<boolean>>
  setShortcutsOpen: Dispatch<SetStateAction<boolean>>
  setLayout: Dispatch<SetStateAction<Layout>>
  startAgent: (program: string) => void
  setRecentOpen: Dispatch<SetStateAction<boolean>>
}) {
  const paletteItems: PaletteItem[] = useMemo(() => {
    // Thirty-odd items with closures, built only while the palette can show them.
    if (!paletteOpen) return []
    const keys = (shortcut: ShortcutAction) => shortcutText(shortcut, platform)
    const action = (id: string, label: string, run: () => void, shortcut?: string): PaletteItem => ({ id, label, kind: 'Action', run, ...(shortcut ? { shortcut } : {}) })
    const showing = activeArrangement(layout)
    // Sessions first, most urgent first, then their projects, then actions: what the palette is mostly for is jumping.
    const urgent = byUrgency(sessions)
    const sessionItems: PaletteItem[] = urgent.map((member) => ({
      id: `session-${member.id}`,
      label: rowTitle(member),
      detail: `${member.process} · ${projectOf.get(member.id)?.name ?? member.cwdLabel}`,
      kind: 'Session',
      ...(member.status === 'needs' ? { badge: 'Needs you' } : {}),
      run: () => { showSessionById(member.id) },
    }))
    const projectItems: PaletteItem[] = [...new Map(urgent.map((member) => [projectOf.get(member.id)?.key ?? member.cwd, member])).entries()].map(([key, member]) => ({
      id: `project-${key}`,
      label: projectOf.get(member.id)?.name ?? member.cwdLabel,
      detail: tildePath(member.repoRoot ?? member.cwd, home),
      kind: 'Project',
      run: () => { showSessionById(member.id) },
    }))
    const items: PaletteItem[] = [
      ...sessionItems,
      ...projectItems,
      action('new-session', 'New agent chat…', () => { newSession(focusedMachineNumber) }, keys('new-session')),
      action('recent-sessions', 'Recent sessions…', () => { setRecentOpen(true) }),
      action('new-terminal', 'New shell', () => { newTerminal(focusedMachineNumber) }, keys('new-terminal')),
      ...(focusedSession && !isShell(focusedSession) ? [action('open-terminal', 'New terminal for this agent', () => { addAgentTerminal(focusedSession.id) })] : []),
      action('new-agent-chat', 'New agent chat', () => { runShortcut('new-agent-chat') }, keys('new-agent-chat')),
      ...heads.filter((head) => head.name !== null && head.chip?.tone === 'ok' && online).map((head) => action(`new-on-${head.key}`, `New shell on ${head.label}…`, () => { newTerminal(head.machine) })),
      action('sidebar-side', preferences.sidebarSide === 'left' ? 'Move sidebar to the right' : 'Move sidebar to the left', () => { setPreferences((current) => ({ ...current, sidebarSide: current.sidebarSide === 'left' ? 'right' : 'left' })) }),
      action('split-right', 'Split right', () => { split(null, 'right') }, keys('split-right')),
      action('split-down', 'Split down', () => { split(null, 'down') }, keys('split-down')),
      action('usage', 'Open usage and cost', () => { setSettingsSection('usage') }, keys('usage')),
      action('jump-needs', 'Jump to the next session that needs you', () => { runShortcut('jump-needs') }, keys('jump-needs')),
      action('inspect', 'Inspect the repository', () => { if (inspectRefusal !== null) toast.message(inspectRefusal); else setInspectOpen(true) }, keys('inspect')),
      ...(focusedSession ? [action('new-worktree', 'New worktree', () => { runShortcut('new-worktree') }, keys('new-worktree'))] : []),
      action('find', 'Find in the pane or every session', () => { setFindOpen(true) }, keys('find')),
      action('broadcast', showing?.broadcast ? 'Stop broadcasting' : 'Broadcast typing to this tab', () => { runShortcut('broadcast') }, keys('broadcast')),
      action('focus', 'Focus mode', () => { setFocusMode(true) }, keys('focus-mode')),
      action('settings', 'Open settings', () => { setSettingsSection('appearance') }, keys('settings')),
      action('performance', 'Open performance', () => { setSettingsSection('performance') }, keys('performance')),
      action('machines', 'Machines', () => { setSettingsSection('machines') }),
      action('shortcuts', 'Keyboard shortcuts', () => { setShortcutsOpen(true) }, keys('shortcuts')),
      ...(showing?.widths ? [action('reset-widths', 'Reset pane widths', () => { setLayout((current) => setColumnWidths(current, showing.id, null)) })] : []),
      action('start-codex', 'Start codex', () => { startAgent('codex') }),
    ]
    for (const candidate of THEMES) items.push({ id: `theme-${candidate.name}`, label: candidate.name, detail: candidate.family, kind: 'Theme', needsQuery: true, run: () => { setPreferences((current) => ({ ...current, themeMode: candidate.light ? 'light' : 'dark', ...(candidate.light ? { lightTheme: candidate.name } : { darkTheme: candidate.name }) })) } })
    return items
  }, [paletteOpen, layout, sessions, focusedSession, heads, preferences.sidebarSide, platform, projectOf, showSessionById, home, newSession, focusedMachineNumber, newTerminal, addAgentTerminal, runShortcut, online, setPreferences, split, setSettingsSection, inspectRefusal, setInspectOpen, setFindOpen, setFocusMode, setShortcutsOpen, setLayout, startAgent, setRecentOpen])
  /** A line some session printed, as a row that opens that session; null when the pane is not one of the window's sessions. */
  const paletteOutputItem = useCallback((hit: SearchHit, query: string): PaletteItem | null => {
    const owner = sessions.find((member) => member.machine === hit.machine && member.pane === hit.term)
    if (!owner) return null
    return { id: `output-${String(hit.machine)}/${String(hit.term)}:${String(hit.line)}`, label: rowTitle(owner), detail: outputSnippet(hit.text, query), kind: 'Output', run: () => { showSessionById(owner.id) } }
  }, [sessions, showSessionById])
  return { paletteItems, paletteOutputItem }
}
