import { describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { displayName, groupInitials, groupSessions, processName, statusPhrase, tildePath, toSessionView, truncatePrompt, worstSession } from './session'
import { formatAgo, formatElapsed, markSeen, trackSessions } from './tracking'
import { hotkeyRows, matchShortcut } from './shortcuts'
import { folderExists, resolveStartFolder } from './startFolder'

vi.mock('@/api/daemon', () => ({
  terminalOf: (id: string) => Number(id.slice(1)),
  suggestProjectFolders: (typed: string) => Promise.resolve(typed === '/home/casey/code' ? ['/home/casey/code/', '/home/casey/codex/'] : typed === 'C:\\dev\\maitty' ? ['C:\\dev\\maitty\\'] : []),
}))

const HOME = '/home/casey'

function conversation(pane: number, cwd: string, extra: Partial<Conversation> = {}): Conversation {
  return { id: `t${String(pane)}`, generation: 0, kind: 'terminal', title: '', project: '', folder: cwd, cwd, repo: null, branch: 'working tree', provider: 'Codex', status: 'idle', updated: null, model: '', permission: '', context: null, nativePane: pane, hasDiff: false, agent: null, ...extra }
}

const repo = (pane: number, root: string, cwd: string, extra: Partial<Conversation> = {}) => conversation(pane, cwd, { folder: root, repo: root.split('/').at(-1) ?? null, branch: 'main', ...extra })

describe('session view', () => {
  it('writes the home folder as ~', () => {
    expect(tildePath(HOME, HOME)).toBe('~')
    expect(tildePath(`${HOME}/code/`, HOME)).toBe('~/code')
    expect(tildePath('/etc', HOME)).toBe('/etc')
    expect(tildePath('C:\\Users\\casey\\dev', 'C:\\Users\\casey')).toBe('~/dev')
  })

  it('names a shell by where it is within its repo, or by its ~ path outside one', () => {
    expect(toSessionView(repo(1, `${HOME}/maitty`, `${HOME}/maitty`), HOME)).toMatchObject({ title: '(root)', promptPath: 'maitty', repoName: 'maitty', branch: 'main', process: 'shell' })
    expect(toSessionView(repo(1, `${HOME}/maitty`, `${HOME}/maitty/crates/maitty-core/src`), HOME)).toMatchObject({ title: 'maitty-core/src', promptPath: 'maitty/crates/maitty-core/src' })
    expect(toSessionView(conversation(1, `${HOME}/Downloads`, { agent: 'codex' }), HOME)).toMatchObject({ title: '~/Downloads', repoName: null, branch: null, process: 'codex' })
  })

  it('calls a session by the name you gave it, else by where it is', () => {
    const unnamed = toSessionView(conversation(1, `${HOME}/maitty`), HOME)
    const named = toSessionView(conversation(2, `${HOME}/maitty`, { named: true, title: '  Login fix ' }), HOME)
    expect(unnamed.name).toBeNull()
    expect(displayName(unnamed)).toBe('~/maitty')
    expect(named.name).toBe('Login fix')
    expect(displayName(named)).toBe('Login fix')
    expect(toSessionView(conversation(3, HOME, { named: true, title: '   ' }), HOME).name).toBeNull()
  })

  it('shortens a prompt to one readable line', () => {
    expect(truncatePrompt('  Fix   the login\n redirect  ')).toBe('Fix the login redirect')
    expect(truncatePrompt('a'.repeat(300))).toHaveLength(160)
    expect(truncatePrompt('a'.repeat(300)).endsWith('…')).toBe(true)
    expect(truncatePrompt('short')).toBe('short')
    expect(toSessionView(conversation(1, HOME, { summary: '  Add tests ' }), HOME).lastPrompt).toBe('Add tests')
    expect(toSessionView(conversation(2, HOME), HOME).lastPrompt).toBe('')
  })

  it('names a session after what it is using, and a bare shell as shell', () => {
    expect(processName('deepseek', 'node')).toBe('deepseek')
    expect(processName(null, '/usr/bin/zsh')).toBe('shell')
    expect(processName(null, '-bash')).toBe('shell')
    expect(processName(null, 'pwsh.exe')).toBe('shell')
    expect(processName(null, 'cargo')).toBe('cargo')
    expect(processName(null, 'Node.exe')).toBe('node')
    expect(processName(null, undefined)).toBe('shell')
    expect(processName(null, '')).toBe('shell')
  })

  it('says each status in words on the row', () => {
    const view = (status: Conversation['status'], extra: Partial<Conversation> = {}) => statusPhrase(toSessionView(conversation(1, HOME, { status, ...extra }), HOME))
    expect(view('working')).toBe(' · working')
    expect(view('needs-you')).toBe(' · needs you')
    expect(view('done')).toBe(' · finished')
    expect(view('idle')).toBe(' · idle')
    expect(view('unknown', { lastKnownStatus: 'working' })).toBe(' · last seen working')
  })

  it('reports what needs you first, then what is working', () => {
    const sessions = [conversation(1, HOME, { status: 'done' }), conversation(2, HOME, { status: 'working' }), conversation(3, HOME, { status: 'needs-you' })].map((item) => toSessionView(item, HOME))
    expect(worstSession(sessions)?.id).toBe('t3')
    expect(worstSession(sessions.slice(0, 2))?.id).toBe('t2')
  })

  it('groups by folder when asked', () => {
    const sessions = [conversation(2, `${HOME}/b`), conversation(1, `${HOME}/a`), repo(3, `${HOME}/a`, `${HOME}/a`)].map((item) => toSessionView(item, HOME))
    const groups = groupSessions(sessions, 'folder', 't1', HOME)
    expect(groups.map((group) => [group.name, group.sessions.map((session) => session.id), group.containsFocused])).toEqual([
      ['a', ['t1', 't3'], true],
      ['b', ['t2'], false],
    ])
  })

  it('groups a worktree with its main checkout when git spells the repo path with forward slashes', () => {
    const worktree = repo(1, 'C:/dev/chameleon', 'C:\\dev\\chameleon-worktrees\\brave-wren')
    const main = repo(2, 'C:\\dev\\chameleon', 'C:\\dev\\chameleon', { repo: 'chameleon' })
    const groups = groupSessions([worktree, main].map((item) => toSessionView(item, HOME)), 'repo', null, HOME)
    expect(groups.map((group) => [group.name, group.sessions.map((session) => session.id)])).toEqual([['chameleon', ['t1', 't2']]])
  })

  it('groups a shell outside any repo by its own folder, not under a catch-all', () => {
    const sessions = [conversation(1, `${HOME}/personal/dynamic-ui`), repo(2, `${HOME}/maitty`, `${HOME}/maitty`)].map((item) => toSessionView(item, HOME))
    const groups = groupSessions(sessions, 'repo', null, HOME)
    expect(groups.map((group) => [group.name, group.kind, group.branch])).toEqual([
      ['dynamic-ui', 'folder', null],
      ['maitty', 'repo', 'main'],
    ])
  })

  it('shortens a group to initials for the rail', () => {
    expect(groupInitials({ kind: 'repo', name: 'maitty-docs' })).toBe('MD')
    expect(groupInitials({ kind: 'repo', name: 'maitty' })).toBe('MA')
    expect(groupInitials({ kind: 'folder', name: 'Downloads' })).toBe('DO')
  })
})

describe('session tracking', () => {
  const view = (status: Conversation['status'], extra: Partial<Conversation> = {}) => [toSessionView(conversation(1, HOME, { status, ...extra }), HOME)]

  it('starts the clock when this window sees a status change, and not before', () => {
    const first = trackSessions(new Map(), view('working'), new Set(), 1000)
    expect(first.tracks.get('t1')).toMatchObject({ workingSince: null, changedAt: null })
    const idle = trackSessions(first.tracks, view('idle'), new Set(), 2000)
    const working = trackSessions(idle.tracks, view('working'), new Set(), 3000)
    expect(working.tracks.get('t1')).toMatchObject({ workingSince: 3000, changedAt: 3000 })
  })

  it('marks a background session unread when it needs you or finishes, and read once shown', () => {
    const first = trackSessions(new Map(), view('working'), new Set(), 0)
    const done = trackSessions(first.tracks, view('done'), new Set(), 1)
    expect(done.tracks.get('t1')?.unread).toBe(true)
    expect(markSeen(done.tracks, new Set(['t1'])).get('t1')?.unread).toBe(false)
    const shown = trackSessions(first.tracks, view('needs-you'), new Set(['t1']), 1)
    expect(shown.tracks.get('t1')?.unread).toBe(false)
  })

  it('notices a shell moving to another repo', () => {
    const first = trackSessions(new Map(), view('idle'), new Set(), 0)
    const moved = trackSessions(first.tracks, [toSessionView(repo(1, `${HOME}/maitty`, `${HOME}/maitty`), HOME)], new Set(), 5)
    expect(moved.moves).toEqual([{ id: 't1', from: null, to: `${HOME}/maitty` }])
    expect(moved.tracks.get('t1')?.movedAt).toBe(5)
  })

  it('keeps what it last saw while the daemon is disconnected', () => {
    const first = trackSessions(new Map(), view('working'), new Set(), 0)
    const offline = trackSessions(first.tracks, view('unknown'), new Set(), 9)
    expect(offline.tracks.get('t1')).toBe(first.tracks.get('t1'))
  })

  it('formats elapsed and relative times as the rows show them', () => {
    expect(formatElapsed(102_000)).toBe('1:42')
    expect(formatAgo(30_000)).toBe('now')
    expect(formatAgo(42 * 60_000)).toBe('42m')
    expect(formatAgo(3 * 3_600_000)).toBe('3h')
    expect(formatAgo(2 * 86_400_000)).toBe('2d')
  })
})

describe('window shortcuts', () => {
  const key = (k: string, extra: Partial<KeyboardEvent> = {}) => ({ key: k, ctrlKey: false, metaKey: false, shiftKey: false, altKey: false, ...extra })

  it('uses Ctrl on Windows and Linux', () => {
    expect(matchShortcut(key('t', { ctrlKey: true }), false)).toBe('new-session')
    expect(matchShortcut(key('d', { ctrlKey: true }), false)).toBe('split-right')
    expect(matchShortcut(key('D', { ctrlKey: true, shiftKey: true }), false)).toBe('split-down')
    expect(matchShortcut(key('[', { ctrlKey: true }), false)).toBe('focus-previous')
    expect(matchShortcut(key(']', { ctrlKey: true }), false)).toBe('focus-next')
    expect(matchShortcut(key('w', { ctrlKey: true }), false)).toBe('close')
    expect(matchShortcut(key('b', { ctrlKey: true }), false)).toBe('toggle-sidebar')
    expect(matchShortcut(key(',', { ctrlKey: true }), false)).toBe('settings')
    expect(matchShortcut(key('Tab', { ctrlKey: true }), false)).toBe('next-tab')
    expect(matchShortcut(key('Tab', { ctrlKey: true, shiftKey: true }), false)).toBe('previous-tab')
  })

  it('passes every other key to the shell', () => {
    expect(matchShortcut(key('c', { ctrlKey: true }), false)).toBeNull()
    expect(matchShortcut(key('l', { ctrlKey: true }), false)).toBeNull()
    expect(matchShortcut(key('t'), false)).toBeNull()
    expect(matchShortcut(key('t', { ctrlKey: true, altKey: true }), false)).toBeNull()
  })

  it('uses Command on macOS and leaves Ctrl to the shell', () => {
    expect(matchShortcut(key('t', { metaKey: true }), true)).toBe('new-session')
    expect(matchShortcut(key('d', { ctrlKey: true }), true)).toBeNull()
    expect(matchShortcut(key('Tab', { ctrlKey: true }), true)).toBe('next-tab')
  })

  it('lists the hotkeys in the platform’s words', () => {
    expect(hotkeyRows(true)[2]).toEqual({ action: 'Split down', chords: [['⌘', '⇧', 'D']] })
    expect(hotkeyRows(false)[2]).toEqual({ action: 'Split down', chords: [['Ctrl', 'Shift', 'D']] })
  })
})

describe('start folder', () => {
  it('opens the home folder for an empty setting or ~, and expands ~/', () => {
    expect(resolveStartFolder('', HOME)).toBe(HOME)
    expect(resolveStartFolder('~', HOME)).toBe(HOME)
    expect(resolveStartFolder('~/code', HOME)).toBe(`${HOME}/code`)
    expect(resolveStartFolder('~\\dev', 'C:\\Users\\casey')).toBe('C:\\Users\\casey\\dev')
    expect(resolveStartFolder('/srv', HOME)).toBe('/srv')
  })

  it('accepts a folder only when the desktop shell finds it', async () => {
    await expect(folderExists('/home/casey/code')).resolves.toBe(true)
    await expect(folderExists('/home/casey/code/')).resolves.toBe(true)
    await expect(folderExists('/home/casey/cod')).resolves.toBe(false)
    await expect(folderExists('C:\\dev\\maitty')).resolves.toBe(true)
    await expect(folderExists('')).resolves.toBe(true)
    await expect(folderExists('/')).resolves.toBe(true)
  })
})
