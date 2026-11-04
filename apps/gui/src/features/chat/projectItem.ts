import type { ChatEvent, ChatProvider, ItemFields, ItemStatus } from './chatEvents'
import { claudeToolName, mapClaudeTool } from './claudeTools'
import { fileNamedByLabel, parseCodexFileChanges } from './codexFileChange'
import type { HistoryEntry } from './historyInput'

type ItemContent = Exclude<HistoryEntry['content'], 'TurnStatus'>
type ToolContent = Extract<ItemContent, { Tool: unknown }>['Tool']

const COMMAND_ITEM = 'commandExecution'
const FILE_CHANGE_ITEM = 'fileChange'
// Codex item types the daemon keeps only by name (it drops the rest of an item it does not
// model). They still tell the view which card the item would have been.
const CODEX_PLAN_ITEM = 'plan'
const CODEX_HELPER_ITEM = 'collabAgentToolCall'

const STATUS_BY_STATE: Record<HistoryEntry['state'], ItemStatus> = {
  Running: 'running',
  Completed: 'completed',
  Failed: 'failed',
  Interrupted: 'interrupted',
}

export function statusOf(state: HistoryEntry['state']): ItemStatus {
  return STATUS_BY_STATE[state]
}

function codexTool(tool: ToolContent, base: ItemFields): ChatEvent[] {
  const label = tool.label ?? ''
  if (tool.kind === COMMAND_ITEM) return [{ ...base, kind: 'command', command: label, output: tool.output }]
  if (tool.kind !== FILE_CHANGE_ITEM) return [{ ...base, kind: 'tool', name: tool.kind, detail: tool.output }]

  const files = parseCodexFileChanges(label, tool.output)
  if (files.length === 0) {
    // The block could not be read: still show that a file changed, without inventing a diff.
    return [{ ...base, kind: 'edit', tool: FILE_CHANGE_ITEM, file: fileNamedByLabel(label), change: 'unknown', diff: { status: 'unavailable' } }]
  }
  return files.map((file, index) => ({
    ...base,
    id: files.length === 1 ? base.id : `${base.id}#${String(index)}`,
    kind: 'edit' as const,
    tool: FILE_CHANGE_ITEM,
    file: file.file,
    change: file.change,
    diff: file.diff,
  }))
}

function claudeTool(tool: ToolContent, base: ItemFields): ChatEvent[] {
  const label = tool.label ?? ''
  if (tool.kind !== COMMAND_ITEM) return [{ ...base, kind: 'tool', name: tool.kind, detail: tool.output }]
  const name = claudeToolName(label)
  // No tool name in the label means the label is the Bash command.
  if (name === null) return [{ ...base, kind: 'command', command: label, output: tool.output }]

  const mapping = mapClaudeTool(name, tool.input)
  switch (mapping.kind) {
    case 'edit':
      return [{ ...base, kind: 'edit', tool: mapping.tool, file: mapping.file, change: mapping.change, diff: mapping.diff }]
    case 'todo':
      return [{ ...base, kind: 'todo', items: mapping.items }]
    case 'subagent':
      return [{ ...base, kind: 'subagent', title: mapping.title, agentType: mapping.agentType, report: tool.output }]
    case 'tool':
      return [{ ...base, kind: 'tool', name: mapping.name, detail: tool.output }]
  }
}

function unsupportedItem(provider: ChatProvider, kind: string, base: ItemFields): ChatEvent[] {
  if (provider === 'codex' && kind === CODEX_PLAN_ITEM) return [{ ...base, kind: 'todo', items: null }]
  if (provider === 'codex' && kind === CODEX_HELPER_ITEM) {
    return [{ ...base, kind: 'subagent', title: null, agentType: null, report: '' }]
  }
  return [{ ...base, kind: 'tool', name: kind, detail: '' }]
}

/** Maps one history item (not a turn marker) to the events it shows as. Never throws. */
export function projectItem(provider: ChatProvider, content: ItemContent, base: ItemFields): ChatEvent[] {
  if ('Message' in content) {
    const { role, text } = content.Message
    return [{ ...base, kind: 'message', role: role === 'User' ? 'user' : 'agent', text }]
  }
  if ('Tool' in content) {
    return provider === 'claude' ? claudeTool(content.Tool, base) : codexTool(content.Tool, base)
  }
  return unsupportedItem(provider, content.Unsupported.kind, base)
}
