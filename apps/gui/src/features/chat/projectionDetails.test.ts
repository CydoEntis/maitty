import { describe, expect, it } from 'vitest'
import type { ChatEvent, ChatProvider } from './chatEvents'
import { applyAnsweredApprovals, applyApprovals, applyHistoryEntries, createChatProjection, projectConversation, selectChatEvents } from './projection'

/*
 * Fixture provenance: the entry and approval shapes are the ones the daemon's read_conversation sends
 * (crates/maitty-shared/src/conversation.rs: `Tool` with `input`, `exit_code`, `duration_ms`; `Entry`
 * with `started_at_ms`/`completed_at_ms`; `AnsweredApproval { request, decision, answered_at_ms }`).
 */

const NOW = 50_000
type State = 'Running' | 'Completed' | 'Failed' | 'Interrupted'

function entry(item: string, content: unknown, state: State = 'Completed', times: { started_at_ms?: number; completed_at_ms?: number } = {}, turn = 'turn-1') {
  return { source: { conversation: 1, turn, item }, content, state, ...times }
}
const commandTool = (item: string, label: string, extra: Record<string, unknown> = {}, state: State = 'Completed', times = {}) =>
  entry(item, { Tool: { kind: 'commandExecution', label, output: 'out', ...extra } }, state, times)

function project(provider: ChatProvider, entries: unknown[], approvals: unknown[] = [], answered: unknown[] = []): ChatEvent[] {
  return projectConversation(provider, entries, approvals, NOW, answered)
}

describe('command timing and exit codes', () => {
  it.each([
    ['a Codex command with its own exit code and duration', 'codex', { exit_code: 2, duration_ms: 1_500 }, { exitCode: 2, durationMs: 1_500 }],
    ['a Codex command that exited 0', 'codex', { exit_code: 0, duration_ms: 40 }, { exitCode: 0, durationMs: 40 }],
    ['a Claude command, which reports neither', 'claude', {}, { exitCode: null, durationMs: null }],
  ] as const)('reads %s', (_name, provider, extra, expected) => {
    const [event] = project(provider, [commandTool('c', 'pnpm test', extra)])

    expect(event).toMatchObject({ kind: 'command', ...expected })
  })

  it('carries the daemon start and completion times, and null for an item read back from saved history', () => {
    const [timed, saved] = project('claude', [commandTool('a', 'ls', {}, 'Completed', { started_at_ms: 1_000, completed_at_ms: 3_500 }), commandTool('b', 'pwd')])

    expect(timed).toMatchObject({ timing: { startedAtMs: 1_000, completedAtMs: 3_500 } })
    expect(saved).toMatchObject({ timing: { startedAtMs: null, completedAtMs: null } })
  })
})

describe('tool chips', () => {
  it.each([
    ['Read', { file_path: '/repo/src/a.ts' }, '/repo/src/a.ts'],
    ['Grep', { pattern: 'TODO', path: 'src' }, 'src'],
    ['Grep', { pattern: 'TODO' }, 'TODO'],
    ['WebFetch', { url: 'https://example.com/docs' }, 'https://example.com/docs'],
    ['Glob', { unrelated: 1 }, null],
  ])('names what a Claude %s call acted on', (name, input, target) => {
    const [event] = project('claude', [commandTool('t', `${name} `, { input })])

    expect(event).toMatchObject({ kind: 'tool', name, target })
  })

  it('keeps the input as indented text for the expanded chip, and null when the history kept none', () => {
    const [withInput] = project('claude', [commandTool('t', 'Read ', { input: { file_path: 'a.ts' } })])
    const [without] = project('claude', [commandTool('u', 'Read ')])

    expect(withInput).toMatchObject({ input: '{\n  "file_path": "a.ts"\n}' })
    expect(without).toMatchObject({ input: null })
  })
})

describe('helper agents', () => {
  it('reads the prompt from a Claude Task and from a Codex helper call', () => {
    const [claude] = project('claude', [commandTool('t', 'Task ', { input: { description: 'Find call sites', prompt: 'Search the repo', subagent_type: 'Explore' } }, 'Running')])
    const [codex] = project('codex', [entry('h', { Tool: { kind: 'collabAgentToolCall', label: 'Audit the parser', output: '', input: { prompt: 'Read parser.rs' } } })])

    expect(claude).toMatchObject({ kind: 'subagent', title: 'Find call sites', agentType: 'Explore', prompt: 'Search the repo', status: 'running' })
    expect(codex).toMatchObject({ kind: 'subagent', title: 'Audit the parser', prompt: 'Read parser.rs' })
  })
})

describe('approvals', () => {
  // Recorded: the daemon's read_conversation (history/tests.rs approval fixtures); Claude's id is the tool-use id.
  const request = (id: number | string, over: Record<string, unknown> = {}) => ({
    request_id: id,
    kind: 'Command',
    turn_id: 'turn-1',
    item_id: 'toolu_1',
    command: 'rm -rf build',
    reason: null,
    changes: [],
    choices: ['Accept', 'AcceptForSession', 'Decline'],
    ...over,
  })

  it('names the tool step the request is about', () => {
    const [approval] = project('claude', [], [request('req-1')])

    expect(approval).toMatchObject({ kind: 'approval', stepId: 'turn-1:toolu_1' })
  })

  it('has no step when the provider named no item', () => {
    const [approval] = project('claude', [], [request('req-1', { item_id: null })])

    expect(approval).toMatchObject({ stepId: null })
  })

  it('shows an approval answered elsewhere with the person\'s real choice', () => {
    const pending = applyApprovals(createChatProjection('claude'), [request('req-1')])
    const gone = applyApprovals(pending, [])
    expect(selectChatEvents(gone)[0]).toMatchObject({ resolution: { status: 'answered', answer: 'unknown' } })

    const known = applyAnsweredApprovals(gone, [{ request: request('req-1'), decision: 'AcceptForSession', answered_at_ms: 9_000 }])

    expect(selectChatEvents(known)[0]).toMatchObject({ resolution: { status: 'answered', answer: 'AcceptForSession', answeredAtMs: 9_000 } })
  })

  it('adds an approval the chat never saw pending, as answered', () => {
    const events = project('claude', [commandTool('toolu_1', 'rm -rf build')], [], [{ request: request('req-9'), decision: 'Decline' }])

    expect(events.map((event) => event.kind)).toEqual(['command', 'approval'])
    expect(events[1]).toMatchObject({ requestId: 'req-9', resolution: { status: 'answered', answer: 'Decline', answeredAtMs: null } })
  })

  it('keeps the choice this app recorded, and never reopens an answered request', () => {
    let state = applyApprovals(createChatProjection('codex'), [request(7)])
    state = applyAnsweredApprovals(applyApprovals(state, []), [{ request: request(7), decision: 'Accept', answered_at_ms: 100 }])
    const again = applyAnsweredApprovals(applyApprovals(state, [request(7)]), [{ request: request(7), decision: 'Accept', answered_at_ms: 100 }])

    expect(selectChatEvents(again)[0]).toMatchObject({ resolution: { status: 'answered', answer: 'Accept' } })
  })

  it('counts an answered record it cannot read and ignores nothing else', () => {
    const state = applyAnsweredApprovals(createChatProjection('codex'), [{ nope: true }])

    expect(state.rejectedInputs).toBe(1)
    expect(state.approvals).toEqual([])
  })

  it('does not change the state when nothing was answered', () => {
    const state = applyHistoryEntries(createChatProjection('codex'), [], NOW)

    expect(applyAnsweredApprovals(state, [])).toBe(state)
  })
})
