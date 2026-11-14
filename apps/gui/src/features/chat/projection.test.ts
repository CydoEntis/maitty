import { describe, expect, it } from 'vitest'
import type { ChatEvent, ChatProvider } from './chatEvents'
import {
  applyApprovals,
  applyHistoryEntries,
  createChatProjection,
  projectConversation,
  recordApprovalAnswer,
  selectChatEvents,
} from './projection'

/*
 * Fixture provenance. "Recorded" means the same shape a Rust test in this repo asserts on or
 * builds; "derived" means computed by hand from the adapter code the file names; "synthetic"
 * means no recording exists yet (the daemon does not keep the data), so the shape follows the
 * provider's published tool schema and is labelled as such where it is used.
 */

const NOW = 1_000
const TURN = 'turn-1'

type State = 'Running' | 'Completed' | 'Failed' | 'Interrupted'

function item(turn: string, id: string, content: unknown, state: State = 'Completed') {
  return { source: { conversation: 1, turn, item: id }, content, state }
}
const message = (id: string, role: 'User' | 'Assistant', text: string, turn = TURN) => item(turn, id, { Message: { role, text } })
const tool = (id: string, label: string, output: string, state: State = 'Completed', extra: Record<string, unknown> = {}, turn = TURN) =>
  item(turn, id, { Tool: { kind: 'commandExecution', label, output, ...extra } }, state)
const turnStatus = (turn: string, state: State) => item(turn, '', 'TurnStatus', state)

function project(provider: ChatProvider, entries: unknown[], approvals: unknown[] = []): ChatEvent[] {
  return projectConversation(provider, entries, approvals, NOW)
}

function only(events: ChatEvent[]): ChatEvent {
  expect(events).toHaveLength(1)
  const [event] = events
  if (!event) throw new Error('expected one event')
  return event
}

// Recorded: crates/maitty-core/src/daemon/conversation/history/tests.rs,
// `a_codex_file_edit_keeps_its_files_and_diff_for_review` (two files, one update, one add), with the
// output text written by `file_change_output` in history_provider.rs.
const CODEX_TWO_FILE_OUTPUT = 'main.go (update)\n@@ -1 +1 @@\n-old\n+new\n\nnotes.txt (add)\nhello'

describe('messages', () => {
  it.each([
    ['User', 'user'],
    ['Assistant', 'agent'],
  ] as const)('shows a %s message as a %s message in the order received', (role, expected) => {
    const event = only(project('claude', [message('m1', role, 'hello')]))
    expect(event).toMatchObject({ kind: 'message', role: expected, text: 'hello', id: `${TURN}:m1`, turnId: TURN })
  })
})

describe('commands', () => {
  // Recorded: history/tests.rs `provider_tool_lifecycle_replaces_one_row_and_preserves_failed_status`.
  it('maps a Codex commandExecution to a command with its output and failed state', () => {
    const event = only(project('codex', [tool('1', 'pwsh -NoProfile -Command Write-Output smoke', 'synthetic output', 'Failed')]))
    expect(event).toMatchObject({
      kind: 'command',
      command: 'pwsh -NoProfile -Command Write-Output smoke',
      output: 'synthetic output',
      status: 'failed',
    })
  })

  // Recorded: maitty-provider-claude projection.rs `projects_tool_input_only_after_the_complete_json_arrives`
  // (Bash, input {"command":"pwd"}, result "done"); the history label is the command.
  it.each([
    ['Running', 'running'],
    ['Completed', 'completed'],
    ['Interrupted', 'interrupted'],
  ] as const)('maps a Claude Bash call that is %s to a command', (state, expected) => {
    const event = only(project('claude', [tool('t1', 'pwd', 'done', state)]))
    expect(event).toMatchObject({ kind: 'command', command: 'pwd', output: 'done', status: expected })
  })
})

describe('edits', () => {
  it('maps a Codex patch of two files to one edit per file with counts and hunks', () => {
    const entry = item(TURN, 'edit', { Tool: { kind: 'fileChange', label: 'Edited 2 files', output: CODEX_TWO_FILE_OUTPUT } })
    const events = project('codex', [entry])
    expect(events).toHaveLength(2)
    expect(events[0]).toMatchObject({
      kind: 'edit',
      id: `${TURN}:edit#0`,
      file: 'main.go',
      change: 'update',
      diff: {
        status: 'available',
        added: 1,
        removed: 1,
        truncated: false,
        hunks: [
          {
            header: '@@ -1 +1 @@',
            lines: [
              { kind: 'remove', text: 'old' },
              { kind: 'add', text: 'new' },
            ],
          },
        ],
      },
    })
    expect(events[1]).toMatchObject({
      kind: 'edit',
      id: `${TURN}:edit#1`,
      file: 'notes.txt',
      change: 'add',
      diff: { status: 'available', added: 1, removed: 0, hunks: [{ lines: [{ kind: 'add', text: 'hello' }] }] },
    })
  })

  it('keeps a single-file Codex patch on the item id', () => {
    const output = 'src/lib.rs (update)\n@@ -1,2 +1,2 @@\n keep\n-a\n+b\n'
    const event = only(project('codex', [item(TURN, 'e', { Tool: { kind: 'fileChange', label: 'Edited src/lib.rs', output } })]))
    expect(event).toMatchObject({ id: `${TURN}:e`, file: 'src/lib.rs', diff: { added: 1, removed: 1 } })
  })

  it('does not start a phantom file from a created file whose text looks like a header', () => {
    const output = 'a.txt (add)\nlooks like (add)\nsecond line'
    const event = only(project('codex', [item(TURN, 'e', { Tool: { kind: 'fileChange', label: 'Edited a.txt', output } })]))
    expect(event).toMatchObject({ file: 'a.txt', diff: { added: 2 } })
  })

  it('marks a diff the daemon cut off as truncated and drops the note', () => {
    const output = 'big.rs (update)\n@@ -1 +1 @@\n-a\n+b\n\n... diff cut off'
    const event = only(project('codex', [item(TURN, 'e', { Tool: { kind: 'fileChange', label: 'Edited big.rs', output } })]))
    expect(event).toMatchObject({ diff: { status: 'available', truncated: true, added: 1, removed: 1 } })
  })

  it('shows an unreadable patch as an edit without a diff instead of failing', () => {
    const event = only(project('codex', [item(TURN, 'e', { Tool: { kind: 'fileChange', label: 'Edited a.rs', output: 'not a header' } })]))
    expect(event).toMatchObject({ kind: 'edit', file: 'a.rs', diff: { status: 'unavailable' } })
  })

  // Derived: projection.rs builds a non-Bash label as `format!("{} {}", name, bounded_content_text(input))`,
  // and that text is empty for an object input, so the history holds only "<Tool> ".
  it.each(['Edit', 'MultiEdit', 'Write'])('shows today\'s Claude %s history as an edit with the file and diff unavailable', (name) => {
    const event = only(project('claude', [tool('t', `${name} `, 'The file has been updated.')]))
    expect(event).toMatchObject({ kind: 'edit', tool: name, file: null, diff: { status: 'unavailable' } })
  })

  // The daemon keeps these inputs as Claude sent them (tool_input/tests.rs `each_tool_keeps_the_input_fields_the_chat_view_reads`).
  it('builds Claude Edit, MultiEdit and Write diffs from the tool input', () => {
    const events = project('claude', [
      tool('e', 'Edit ', 'ok', 'Completed', { input: { file_path: '/p/a.ts', old_string: 'one\ntwo', new_string: 'three' } }),
      tool('m', 'MultiEdit ', 'ok', 'Completed', {
        input: { file_path: '/p/b.ts', edits: [{ old_string: 'x', new_string: 'y' }, { old_string: 'z', new_string: 'z2\nz3' }] },
      }),
      tool('w', 'Write ', 'ok', 'Completed', { input: { file_path: '/p/c.ts', content: 'l1\nl2\n' } }),
    ])
    expect(events.map((event) => event.kind === 'edit' && [event.tool, event.file, event.diff.status === 'available' && [event.diff.added, event.diff.removed]])).toEqual([
      ['Edit', '/p/a.ts', [1, 2]],
      ['MultiEdit', '/p/b.ts', [3, 2]],
      ['Write', '/p/c.ts', [2, 0]],
    ])
  })
})

describe('todo lists', () => {
  it('shows today\'s Claude TodoWrite history as a checklist whose items are unavailable', () => {
    const event = only(project('claude', [tool('t', 'TodoWrite ', 'Todos have been modified successfully.')]))
    expect(event).toMatchObject({ kind: 'todo', items: null })
  })

  // Input as the daemon keeps it (same test as above).
  it('reads the checklist from TodoWrite input when present', () => {
    const input = {
      todos: [
        { content: 'Read', status: 'completed', activeForm: 'Reading' },
        { content: 'Write', status: 'in_progress', activeForm: 'Writing' },
        { content: 'Test', status: 'pending', activeForm: 'Testing' },
      ],
    }
    const event = only(project('claude', [tool('t', 'TodoWrite ', 'ok', 'Completed', { input })]))
    expect(event).toMatchObject({
      kind: 'todo',
      items: [
        { text: 'Read', state: 'done' },
        { text: 'Write', state: 'current' },
        { text: 'Test', state: 'pending' },
      ],
    })
  })

  it('falls back to an unavailable checklist when the input is not a todo list', () => {
    const event = only(project('claude', [tool('t', 'TodoWrite ', 'ok', 'Completed', { input: { todos: 'nope' } })]))
    expect(event).toMatchObject({ kind: 'todo', items: null })
  })

  // History cached before the daemon kept plan text holds the item by kind only.
  it('shows a Codex plan item an older daemon cached as a checklist whose items are unavailable', () => {
    const event = only(project('codex', [item(TURN, 'p', { Unsupported: { kind: 'plan' } })]))
    expect(event).toMatchObject({ kind: 'todo', items: null })
  })

  // Recorded: history_provider/tests.rs `codex_plan_and_helper_items_keep_their_text_and_title`.
  it.each([
    ['1. Read\n2. Fix', ['Read', 'Fix']],
    ['Plan:\n- one\n* two\n3) three\nclosing remarks', ['one', 'two', 'three']],
    ['Just prose, no list.', null],
  ] as const)('reads the list entries of a Codex plan text %j', (output, expected) => {
    const event = only(project('codex', [item(TURN, 'p', { Tool: { kind: 'plan', label: 'Plan', output } })]))
    expect(event).toMatchObject({ kind: 'todo' })
    const items = event.kind === 'todo' ? event.items : undefined
    expect(items === null ? null : items?.map((entry) => entry.text)).toEqual(expected)
    if (items) expect(items.every((entry) => entry.state === 'pending')).toBe(true)
  })
})

describe('helper agents', () => {
  it('shows today\'s Claude Task history as a helper with its report and no title', () => {
    const event = only(project('claude', [tool('t', 'Task ', 'Found 3 call sites.')]))
    expect(event).toMatchObject({ kind: 'subagent', title: null, agentType: null, report: 'Found 3 call sites.' })
  })

  // Input as the daemon keeps it (same test as above).
  it('reads the title and type from Task input when present', () => {
    const input = { description: 'Find call sites', prompt: 'long prompt', subagent_type: 'Explore' }
    const event = only(project('claude', [tool('t', 'Task ', 'done', 'Running', { input })]))
    expect(event).toMatchObject({ kind: 'subagent', title: 'Find call sites', agentType: 'Explore', status: 'running' })
  })

  // History cached before the daemon kept helper titles holds the item by kind only.
  it('shows a Codex collabAgentToolCall an older daemon cached as a helper without a title', () => {
    const event = only(project('codex', [item(TURN, 'h', { Unsupported: { kind: 'collabAgentToolCall' } })]))
    expect(event).toMatchObject({ kind: 'subagent', title: null })
  })

  // Recorded: history_provider/tests.rs `codex_plan_and_helper_items_keep_their_text_and_title`.
  it('shows a Codex helper with the title the daemon kept', () => {
    const entry = item(TURN, 'h', { Tool: { kind: 'collabAgentToolCall', label: 'Audit the parser', output: '' } }, 'Running')
    expect(only(project('codex', [entry]))).toMatchObject({ kind: 'subagent', title: 'Audit the parser', status: 'running' })
  })
})

describe('unknown tools', () => {
  // Recorded: history/tests.rs `unknownTool` becomes `Unsupported { kind }`.
  it.each([
    ['codex', { Unsupported: { kind: 'unknownTool' } }, 'unknownTool'],
    ['claude', { Unsupported: { kind: 'unknownTool' } }, 'unknownTool'],
    ['codex', { Tool: { kind: 'mcpToolCall', output: 'x', label: 'x' } }, 'mcpToolCall'],
  ] as const)('shows an unrecognised %s item as a generic tool', (provider, content, name) => {
    const event = only(project(provider, [item(TURN, 'u', content)]))
    expect(event).toMatchObject({ kind: 'tool', name })
  })

  it.each(['Read ', 'Grep ', 'WebFetch ', 'mcp__docs__search ', 'BrandNewTool '])('shows the Claude tool "%s" as a generic tool named after it', (label) => {
    const event = only(project('claude', [tool('t', label, 'result text')]))
    expect(event).toMatchObject({ kind: 'tool', name: label.trim(), detail: 'result text' })
  })

  it('does not mistake a Bash command for a tool name', () => {
    expect(only(project('claude', [tool('t', 'Read the logs', 'x')]))).toMatchObject({ kind: 'command' })
    expect(only(project('claude', [tool('t', 'ls ', 'x')]))).toMatchObject({ kind: 'command' })
  })
})

describe('reasoning', () => {
  // Recorded: history/tests.rs `a_codex_reasoning_summary_is_kept_as_its_own_item_and_hidden_reasoning_is_not`.
  it('projects a reasoning item as a reasoning event and does not count it as unreadable', () => {
    const entries = [item(TURN, 'r', { Reasoning: { text: 'Weighing two fixes' } }), message('m1', 'Assistant', 'Done.')]
    const state = applyHistoryEntries(createChatProjection('codex'), entries, NOW)

    expect(selectChatEvents(state)).toMatchObject([{ kind: 'reasoning', text: 'Weighing two fixes' }, { kind: 'message', text: 'Done.' }])
    expect(state.rejectedInputs).toBe(0)
  })

  it('shows nothing for a reasoning item with no text', () => {
    const state = applyHistoryEntries(createChatProjection('claude'), [item(TURN, 'r', { Reasoning: { text: '  \n' } })], NOW)

    expect(selectChatEvents(state)).toEqual([])
    expect(state.rejectedInputs).toBe(0)
  })
})

describe('turns', () => {
  it('puts the divider after the turn\'s items once the turn has ended', () => {
    const events = project('claude', [
      turnStatus(TURN, 'Running'),
      message('u', 'User', 'go'),
      tool('t', 'pwd', '/p'),
      message('a', 'Assistant', 'done'),
      turnStatus(TURN, 'Completed'),
    ])
    expect(events.map((event) => event.kind)).toEqual(['message', 'command', 'message', 'turn'])
    expect(events.at(-1)).toMatchObject({ kind: 'turn', turnId: TURN, outcome: 'completed' })
  })

  it('takes a turn\'s duration from the daemon\'s times, and says when it has none', () => {
    const timed = { ...turnStatus(TURN, 'Completed'), started_at_ms: 1_000, completed_at_ms: 8_000 }
    expect(project('claude', [message('u', 'User', 'go'), timed]).at(-1)).toMatchObject({ kind: 'turn', startedAtMs: 1_000, endedAtMs: 8_000, timed: true })
    expect(project('claude', [message('u', 'User', 'go'), turnStatus(TURN, 'Completed')]).at(-1)).toMatchObject({ kind: 'turn', timed: false })
  })

  it('has no divider while the turn is still running', () => {
    const events = project('claude', [turnStatus(TURN, 'Running'), message('u', 'User', 'go')])
    expect(events.map((event) => event.kind)).toEqual(['message'])
  })

  it.each([
    ['Failed', 'failed'],
    ['Interrupted', 'interrupted'],
  ] as const)('ends a %s turn with a %s divider', (state, outcome) => {
    const events = project('codex', [message('u', 'User', 'go'), turnStatus(TURN, state)])
    expect(events.at(-1)).toMatchObject({ kind: 'turn', outcome })
  })

  it('keeps turns in the order they were first seen', () => {
    const events = project('claude', [
      message('a', 'User', 'one', 'turn-a'),
      turnStatus('turn-a', 'Completed'),
      message('b', 'User', 'two', 'turn-b'),
    ])
    expect(events.map((event) => event.kind === 'turn' ? `end:${event.turnId}` : event.id)).toEqual(['turn-a:a', 'end:turn-a', 'turn-b:b'])
  })
})

describe('approvals', () => {
  // Recorded: fake_claude.rs scenario "approval" (Bash `echo hello`, request id "tool-request") through
  // claude.rs, which sends choices accept/decline and the request id as the item id.
  const claudeApproval = { request_id: 'tool-request', kind: 'Command', item_id: 'tool-request', turn_id: TURN, command: 'echo hello', reason: null, cwd: null, changes: [], questions: [], choices: ['Accept', 'Decline'] }
  // Recorded: fake_provider.rs "conversation-file-approval" (patch preview plus the request).
  const codexApproval = {
    request_id: 'file-approval-private',
    kind: 'FileChange',
    item_id: 'file-change-1',
    turn_id: TURN,
    command: null,
    reason: 'Synthetic file change',
    changes: [{ path: 'src/synthetic.rs', kind: 'update', diff: '@@ -1 +1 @@\n-old\n+new\n' }],
    choices: ['Accept', 'AcceptForSession', 'Decline', 'Cancel'],
  }

  it('carries the Claude request id and shows the command while pending', () => {
    const events = project('claude', [turnStatus(TURN, 'Running'), message('u', 'User', 'go')], [claudeApproval])
    expect(events.at(-1)).toMatchObject({
      kind: 'approval',
      requestId: 'tool-request',
      approvalKind: 'Command',
      command: 'echo hello',
      choices: ['Accept', 'Decline'],
      resolution: { status: 'pending' },
    })
  })

  it('carries the Codex file preview as a diff', () => {
    const event = only(project('codex', [], [codexApproval]))
    expect(event).toMatchObject({
      kind: 'approval',
      itemId: 'file-change-1',
      changes: [{ path: 'src/synthetic.rs', diff: { status: 'available', added: 1, removed: 1 } }],
    })
  })

  it('marks a request answered when the daemon stops listing it, with the choice unknown', () => {
    const pending = applyApprovals(createChatProjection('claude'), [claudeApproval])
    const answered = applyApprovals(pending, [])
    expect(only(selectChatEvents(answered))).toMatchObject({ resolution: { status: 'answered', answer: 'unknown' } })
  })

  it('records the user\'s own answer and keeps it when the daemon later drops the request', () => {
    let state = applyApprovals(createChatProjection('claude'), [claudeApproval])
    state = recordApprovalAnswer(state, 'tool-request', 'AcceptForSession')
    expect(only(selectChatEvents(state))).toMatchObject({ resolution: { status: 'answered', answer: 'AcceptForSession' } })
    state = applyApprovals(state, [])
    expect(only(selectChatEvents(state))).toMatchObject({ resolution: { status: 'answered', answer: 'AcceptForSession' } })
  })

  it('never reopens an answered request that is listed again', () => {
    let state = applyApprovals(createChatProjection('claude'), [claudeApproval])
    state = recordApprovalAnswer(state, 'tool-request', 'Decline')
    state = applyApprovals(state, [claudeApproval])
    expect(only(selectChatEvents(state))).toMatchObject({ resolution: { status: 'answered', answer: 'Decline' } })
  })

  it('tells a numeric request id from the same digits as text', () => {
    const state = applyApprovals(createChatProjection('codex'), [
      { ...codexApproval, request_id: 7 },
      { ...codexApproval, request_id: '7' },
    ])
    expect(selectChatEvents(state)).toHaveLength(2)
    const answered = recordApprovalAnswer(state, 7, 'Accept')
    expect(selectChatEvents(answered).map((event) => event.kind === 'approval' && event.resolution.status)).toEqual(['answered', 'pending'])
  })

  it('sits after its turn\'s items and before the turn divider, and at the end when it names no turn', () => {
    const entries = [message('u', 'User', 'go'), turnStatus(TURN, 'Completed'), message('v', 'User', 'next', 'turn-2')]
    const events = project('claude', entries, [claudeApproval, { ...claudeApproval, request_id: 'orphan', turn_id: null }])
    expect(events.map((event) => event.kind)).toEqual(['message', 'approval', 'turn', 'message', 'approval'])
  })
})

describe('steps that never got a result', () => {
  it('stop spinning once their turn has completed', () => {
    const events = project('claude', [tool('t1', 'Read a.png', '', 'Running'), turnStatus(TURN, 'Completed')])
    expect(events.find((event) => event.kind === 'command')).toMatchObject({ status: 'completed' })
  })

  it('keep spinning while their turn is still running', () => {
    const events = project('claude', [tool('t1', 'Read a.png', '', 'Running'), turnStatus(TURN, 'Running')])
    expect(events.find((event) => event.kind === 'command')).toMatchObject({ status: 'running' })
  })
})

describe('malformed input', () => {
  it('skips entries and approvals that fail validation, counts them, and keeps the rest', () => {
    const entries = [
      message('ok', 'User', 'still here'),
      null,
      'text',
      { source: { conversation: 1, turn: TURN, item: 'x' }, content: { Message: { role: 'System', text: 'bad role' } }, state: 'Completed' },
      { source: { conversation: 1, turn: TURN }, content: 'TurnStatus', state: 'Completed' },
      item(TURN, 'bad-state', { Message: { role: 'User', text: 'x' } }, 'Weird' as State),
    ]
    const state = applyApprovals(applyHistoryEntries(createChatProjection('codex'), entries, NOW), [{ request_id: {} }, 42])
    expect(selectChatEvents(state)).toHaveLength(1)
    expect(state.rejectedInputs).toBe(7)
  })

  it('does not throw on a tool whose label and output are missing or odd', () => {
    const events = project('claude', [item(TURN, 't', { Tool: { kind: 'commandExecution', output: '' } }), tool('u', '', ''), tool('v', ' ', '')])
    expect(events.map((event) => event.kind)).toEqual(['command', 'command', 'command'])
  })
})

describe('incremental projection', () => {
  const entries = [
    turnStatus(TURN, 'Running'),
    message('u', 'User', 'edit it'),
    tool('t1', 'pwd', '', 'Running'),
    item(TURN, 'e', { Tool: { kind: 'fileChange', label: 'Edited 2 files', output: CODEX_TWO_FILE_OUTPUT } }),
    tool('t1', 'pwd', '/p', 'Completed'),
    message('a', 'Assistant', 'done'),
    turnStatus(TURN, 'Completed'),
  ]
  const approvals = [{ request_id: 5, kind: 'Command', turn_id: TURN, command: 'rm -r build', choices: ['Accept', 'Decline'], changes: [] }]

  function viaChunks(chunkSize: number): ChatEvent[] {
    let state = createChatProjection('codex')
    for (let start = 0; start < entries.length; start += chunkSize) {
      state = applyHistoryEntries(state, entries.slice(start, start + chunkSize), NOW)
    }
    return selectChatEvents(applyApprovals(state, approvals))
  }

  it.each([1, 2, 3, entries.length])('equals the full projection when appended in chunks of %i', (chunkSize) => {
    expect(viaChunks(chunkSize)).toEqual(project('codex', entries, approvals))
  })

  it('updates a running tool in place without moving it', () => {
    const events = project('codex', entries)
    expect(events.map((event) => event.id)).toEqual([`${TURN}:u`, `${TURN}:t1`, `${TURN}:e#0`, `${TURN}:e#1`, `${TURN}:a`, `turn:${TURN}`])
    expect(events[1]).toMatchObject({ kind: 'command', status: 'completed', output: '/p' })
  })

  it('is unchanged by reading the same page twice', () => {
    const once = applyHistoryEntries(createChatProjection('codex'), entries, NOW)
    expect(selectChatEvents(applyHistoryEntries(once, entries, NOW))).toEqual(selectChatEvents(once))
  })

  it('returns the same state object when nothing new arrived', () => {
    const state = applyHistoryEntries(createChatProjection('codex'), [], NOW)
    expect(applyHistoryEntries(state, [], NOW)).toBe(state)
  })

  it('does not change the state it was given', () => {
    const before = applyHistoryEntries(createChatProjection('codex'), entries.slice(0, 3), NOW)
    const snapshot = selectChatEvents(before)
    applyHistoryEntries(before, entries.slice(3), NOW + 1)
    expect(selectChatEvents(before)).toEqual(snapshot)
  })

  it('records when an item was first seen and when it first finished, from the injected clock', () => {
    let state = applyHistoryEntries(createChatProjection('claude'), [tool('t', 'pwd', '', 'Running')], 100)
    state = applyHistoryEntries(state, [tool('t', 'pwd', 'ok', 'Completed')], 250)
    state = applyHistoryEntries(state, [tool('t', 'pwd', 'ok', 'Completed')], 900)
    expect(only(selectChatEvents(state))).toMatchObject({ observed: { firstSeenAtMs: 100, settledAtMs: 250 } })
  })
})
