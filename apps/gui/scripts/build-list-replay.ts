// Builds src/features/workspace/listReplay.fixture.json: the session lists a window hears over about seven minutes,
// in the shapes the host sends (`list_sessions`, `sessions-changed`). It is built, not recorded from a daemon; see
// docs/TESTING.md. Run `node scripts/build-list-replay.ts` from apps/gui.
import { writeFileSync } from 'node:fs'
import path from 'node:path'
import { fileURLToPath } from 'node:url'

type Status = 'None' | 'Idle' | 'Working' | 'Blocked' | 'Done'
type Pane = { id: number; agent: string | null; process: string; remote: string | null; status: Status; since: number }
type Step = { after: number; event: 'list_sessions' | 'sessions-changed'; note?: string; payload: unknown }

const OUTPUT = path.resolve(path.dirname(fileURLToPath(import.meta.url)), '../src/features/workspace/listReplay.fixture.json')
const STARTED = 1_767_605_400
const CHURN_STATUSES: readonly Status[] = ['Working', 'Done', 'Idle', 'Working', 'Blocked', 'Working', 'Done']

let elapsedMs = 0
let revision = 12
const panes = new Map<number, Pane>()
const steps: Step[] = []

function termInfo(pane: Pane) {
  return {
    id: pane.id, cols: 120, rows: 32, title: pane.agent ?? pane.process, process: pane.process, remote: pane.remote,
    agent: pane.agent, hooked: pane.agent !== null, status: pane.status, cwd: '/home/me/app', summary: '', name: '',
    model: '', said: '', branch: 'main', linked: false, subagents: [], root: '/home/me/app', top: '/home/me/app',
    since: pane.since, asleep: false, win32_input: false,
  }
}

function snapshot(removed: Record<string, string> = {}) {
  return {
    conversations: [], daemon_version: '0.76.0', daemon_id: 'fixture-daemon', workspaces: [], active_ws: null,
    terms: Object.fromEntries([...panes.values()].map((pane) => [String(pane.id), termInfo(pane)])),
    questions: [], secret_requests: [], limits: [], spent_today: 0, today: [], revision, removed,
  }
}

function emit(after: number, event: Step['event'], payload: unknown, note?: string): void {
  elapsedMs += after
  steps.push({ after, event, ...(note === undefined ? {} : { note }), payload })
}

function open(id: number, agent: string | null, process: string, remote: string | null = null): void {
  panes.set(id, { id, agent, process, remote, status: agent === null ? 'None' : 'Working', since: STARTED + elapsedMs / 1000 })
}

/** One status change per update, among the agents in turn, a second or two apart. */
function churn(updates: number): void {
  for (let update = 0; update < updates; update += 1) {
    const agents = [...panes.values()].filter((pane) => pane.agent !== null)
    const pane = agents[update % agents.length]
    if (pane === undefined) throw new Error('churn needs an agent pane')
    const after = 700 + ((update * 383) % 1800)
    pane.status = CHURN_STATUSES[(update + pane.id) % CHURN_STATUSES.length] ?? 'Working'
    pane.since = STARTED + Math.floor((elapsedMs + after) / 1000)
    revision += 1
    emit(after, 'sessions-changed', snapshot())
  }
}

function remove(id: number, reason: string, after: number, note: string): void {
  panes.delete(id)
  revision += 1
  emit(after, 'sessions-changed', snapshot({ [String(id)]: reason }), note)
}

open(1, 'claude', 'claude')
open(2, 'codex', 'codex')
open(3, null, 'ssh', 'build-box')
open(4, null, 'bash')
emit(0, 'list_sessions', snapshot(), 'the window opens and reads the list; pane 3 runs ssh')
churn(40)

open(5, 'claude', 'claude')
revision += 1
emit(1200, 'sessions-changed', snapshot(), 'pane 5 opens')
churn(20)

const closed = panes.get(5)
remove(5, 'Closed', 900, 'pane 5 is closed')
churn(2)
if (closed === undefined) throw new Error('pane 5 was not open')
panes.set(5, closed)
revision += 1
emit(1100, 'sessions-changed', snapshot(), 'the close of pane 5 is undone inside its undo window')
churn(30)

emit(1500, 'sessions-changed', null, 'the host lost the daemon, which is restarting')
const restored = [...panes.values()]
panes.clear()
revision = 1
for (const pane of restored.slice(0, 3)) panes.set(pane.id, pane)
emit(6000, 'list_sessions', snapshot(), 'the restarted daemon answers, numbering from 1 again, with three panes restored so far')
for (const pane of restored.slice(3)) {
  panes.set(pane.id, pane)
  revision += 1
  emit(1200, 'sessions-changed', snapshot(), `pane ${String(pane.id)} is restored`)
}
churn(40)

remove(4, 'Exited', 1300, 'the shell in pane 4 exits')
churn(20)

open(6, 'codex', 'codex')
revision += 2
emit(1600, 'sessions-changed', snapshot(), 'the update that opened pane 6 was lost; the host asked for the whole list and this is it')
churn(60)

writeFileSync(OUTPUT, `{\n"source": "built by apps/gui/scripts/build-list-replay.ts, not recorded from a daemon",\n"steps": [\n${steps.map((step) => JSON.stringify(step)).join(',\n')}\n]\n}\n`)
process.stdout.write(`Wrote ${String(steps.length)} steps to ${OUTPUT}\n`)
