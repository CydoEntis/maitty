import { describe, expect, it, vi } from 'vitest'
import type { Conversation } from '@/data/preview'
import { displayName, folderNameOf, inCreationOrder, matchesFilter, processName, rowName, statusText, tildePath, toSessionView, truncatePrompt } from './session'
import { formatAgo, formatElapsed, markSeen, trackSessions } from './tracking'
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
    expect(toSessionView(repo(1, `${HOME}/maitty`, `${HOME}/maitty`), HOME)).toMatchObject({ title: 'main', promptPath: 'maitty', repoName: 'maitty', branch: 'main', process: 'shell' })
    expect(toSessionView(repo(1, `${HOME}/maitty`, `${HOME}/maitty/crates/maitty-core/src`), HOME)).toMatchObject({ title: 'maitty-core/src', promptPath: 'maitty/crates/maitty-core/src' })
    expect(toSessionView(conversation(1, `${HOME}/Downloads`, { agent: 'codex' }), HOME)).toMatchObject({ title: '~/Downloads', repoName: null, branch: null, process: 'codex' })
    // The worktree mark follows the pane the daemon says is in a linked worktree.
    expect(toSessionView(conversation(1, `${HOME}/wt`, { linked: true }), HOME).worktree).toBe(true)
    expect(toSessionView(conversation(1, `${HOME}/wt`), HOME).worktree).toBe(false)
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
    const view = (status: Conversation['status'], extra: Partial<Conversation> = {}) => statusText(toSessionView(conversation(1, HOME, { status, ...extra }), HOME))
    expect(view('working')).toBe('working')
    expect(view('needs-you')).toBe('needs you')
    expect(view('done')).toBe('finished')
    expect(view('idle')).toBe('idle')
    expect(view('unknown', { lastKnownStatus: 'working' })).toBe('last seen working')
  })

  it('lists sessions in creation order whatever their status, and a pin changes nothing', () => {
    const sessions = [conversation(3, HOME, { status: 'needs-you' }), conversation(1, HOME, { status: 'done', pinned: true }), conversation(2, HOME, { status: 'working' })].map((item) => toSessionView(item, HOME))
    expect(inCreationOrder(sessions).map((session) => session.id)).toEqual(['t1', 't2', 't3'])
  })

  it('names a row after the name you gave it, else the program', () => {
    expect(rowName(toSessionView(conversation(1, HOME, { agent: 'claude' }), HOME))).toBe('claude')
    expect(rowName(toSessionView(conversation(1, HOME, { title: 'Growth Tests', named: true }), HOME))).toBe('Growth Tests')
  })

  it('badges the folder by the repo root when there is one, else the shell folder', () => {
    expect(folderNameOf(toSessionView(repo(1, `${HOME}/maitty`, `${HOME}/maitty/crates`), HOME))).toBe('maitty')
    expect(folderNameOf(toSessionView(conversation(2, `${HOME}/personal/dynamic-ui`), HOME))).toBe('dynamic-ui')
  })

  it('carries the diff counts the daemon sends, zero when it sends none', () => {
    expect(toSessionView(conversation(1, HOME, { diffAdded: 104, diffRemoved: 61 }), HOME)).toMatchObject({ diffAdded: 104, diffRemoved: 61 })
    expect(toSessionView(conversation(1, HOME), HOME)).toMatchObject({ diffAdded: 0, diffRemoved: 0 })
  })

  it('filters by name, folder, process and machine, ignoring case', () => {
    const local = toSessionView(repo(1, `${HOME}/AeVoxGrowth`, `${HOME}/AeVoxGrowth`, { agent: 'claude' }), HOME)
    const remote = toSessionView(conversation(2, '/srv/app', { machine: 3, machineName: 'devbox' }), HOME)
    expect(matchesFilter(local, 'aevox')).toBe(true)
    expect(matchesFilter(local, 'CLAUDE')).toBe(true)
    expect(matchesFilter(local, 'devbox')).toBe(false)
    expect(matchesFilter(remote, 'devbox')).toBe(true)
    expect(matchesFilter(remote, '  ')).toBe(true)
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

  it('strips markup an agent quotes and collapses whitespace in a quoted line', () => {
    expect(truncatePrompt('<task-notification>\n  <task-id>bel0</task-id>   done </task-notification>')).toBe('bel0 done')
    expect(truncatePrompt('compare a < b and c > d')).toBe('compare a < b and c > d')
  })
})

describe('an exited session', () => {
  const failed = { code: 2, at: 10 }
  it('reads as exited when the daemon reports a failed last command and nothing is running', () => {
    expect(toSessionView(conversation(1, `${HOME}/a`, { status: 'idle', details: { asking: '', interrupted: null, exit: failed, timeline: [], usage: { context: null, cost: null, tokens: null } } }), HOME).status).toBe('error')
    expect(toSessionView(conversation(1, `${HOME}/a`, { status: 'done', details: { asking: '', interrupted: null, exit: failed, timeline: [], usage: { context: null, cost: null, tokens: null } } }), HOME).status).toBe('error')
  })
  it('shows working or needs you before a failure, and idle when nothing failed', () => {
    const details = { asking: '', interrupted: null, exit: failed, timeline: [], usage: { context: null, cost: null, tokens: null } }
    expect(toSessionView(conversation(1, `${HOME}/a`, { status: 'working', details }), HOME).status).toBe('working')
    expect(toSessionView(conversation(1, `${HOME}/a`, { status: 'needs-you', details }), HOME).status).toBe('needs')
    expect(toSessionView(conversation(1, `${HOME}/a`, { status: 'idle', details: { ...details, exit: null } }), HOME).status).toBe('idle')
  })
})
