import type { ApprovalEvent, CommandEvent, EditEvent, ItemFields, ItemStatus, MessageEvent, ReasoningEvent, SubagentEvent, TodoEvent, ToolEvent } from './chatEvents'

/** Builders for events in tests. Defaults describe a finished item with no daemon times. */

export function fields(id: string, status: ItemStatus = 'completed', turnId = 't1'): ItemFields {
  return { id, turnId, status, observed: { firstSeenAtMs: 0, settledAtMs: status === 'running' ? null : 0 }, timing: { startedAtMs: null, completedAtMs: null } }
}

export const messageEvent = (id: string, role: 'user' | 'agent', text: string, turnId = 't1'): MessageEvent => ({ ...fields(id, 'completed', turnId), kind: 'message', role, text })

export const commandEvent = (id: string, over: Partial<CommandEvent> = {}): CommandEvent => ({ ...fields(id), kind: 'command', command: 'ls', output: '', exitCode: null, durationMs: null, ...over })

export const toolEvent = (id: string, over: Partial<ToolEvent> = {}): ToolEvent => ({ ...fields(id), kind: 'tool', name: 'Read', target: null, input: null, detail: '', ...over })

export const subagentEvent = (id: string, over: Partial<SubagentEvent> = {}): SubagentEvent => ({ ...fields(id), kind: 'subagent', title: 'Find call sites', agentType: 'Explore', prompt: null, report: '', ...over })

export const todoEvent = (id: string, items: TodoEvent['items']): TodoEvent => ({ ...fields(id), kind: 'todo', items })

export const editEvent = (id: string, over: Partial<EditEvent> = {}): EditEvent => ({
  ...fields(id),
  kind: 'edit',
  tool: 'Edit',
  file: 'src/a.ts',
  change: 'update',
  diff: { status: 'available', hunks: [{ header: '@@ -1 +1 @@', lines: [{ kind: 'remove', text: 'old' }, { kind: 'add', text: 'new' }] }], added: 1, removed: 1, truncated: false },
  ...over,
})

export const approvalEvent = (requestId: number | string, over: Partial<ApprovalEvent> = {}): ApprovalEvent => ({
  kind: 'approval',
  id: `approval:${typeof requestId}:${String(requestId)}`,
  requestId,
  turnId: 't1',
  itemId: null,
  stepId: null,
  approvalKind: 'Command',
  command: 'rm -rf build',
  reason: null,
  changes: [],
  questions: [],
  choices: ['Accept', 'AcceptForSession', 'Decline'],
  canRemember: false,
  resolution: { status: 'pending' },
  ...over,
})

export const reasoningEvent = (id: string, text: string, turnId = 't1'): ReasoningEvent => ({ ...fields(id, 'completed', turnId), kind: 'reasoning', text })
