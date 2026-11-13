import type { ChatEvent, ChatProvider, ItemFields, ItemStatus, TodoItem } from './chatEvents'
import { z } from 'zod'
import { claudeToolName, inputText, mapClaudeTool } from './claudeTools'
import { fileNamedByLabel, parseCodexFileChanges } from './codexFileChange'
import type { HistoryEntry } from './historyInput'

type ItemContent = Exclude<HistoryEntry['content'], 'TurnStatus'>
type ToolContent = Extract<ItemContent, { Tool: unknown }>['Tool']

const COMMAND_ITEM = 'commandExecution'
const FILE_CHANGE_ITEM = 'fileChange'
// Codex item types. The daemon now sends them as tool rows (the plan text, the helper's title);
// a history cached by an older daemon holds them by name only, so the fallback below stays.
const CODEX_PLAN_ITEM = 'plan'
const CODEX_HELPER_ITEM = 'collabAgentToolCall'
// The daemon's name for an agent's own summary of the earlier conversation; the text is the output.
const CONTEXT_SUMMARY_ITEM = 'contextSummary'

const STATUS_BY_STATE: Record<HistoryEntry['state'], ItemStatus> = {
  Running: 'running',
  Completed: 'completed',
  Failed: 'failed',
  Interrupted: 'interrupted',
}

export function statusOf(state: HistoryEntry['state']): ItemStatus {
  return STATUS_BY_STATE[state]
}

// A line of a plan that is a list entry: "- text", "* text", "1. text" or "1) text".
const PLAN_LIST_LINE = /^\s*(?:[-*]|\d+[.)])\s+(\S.*)$/

/** The list entries of a Codex plan's text; `null` when the plan has none (it is prose). */
function planItems(text: string): readonly TodoItem[] | null {
  const items = text.split('\n').flatMap((line): TodoItem[] => {
    const entry = PLAN_LIST_LINE.exec(line)?.[1]
    return entry === undefined ? [] : [{ text: entry.trim(), state: 'pending' }]
  })
  return items.length > 0 ? items : null
}

// The daemon sends a Codex helper call with the same input shape as Claude's Task.
const HelperInputSchema = z.object({ prompt: z.string().optional() })

function commandEvent(tool: ToolContent, command: string, base: ItemFields): ChatEvent {
  return { ...base, kind: 'command', command, output: tool.output, exitCode: tool.exit_code ?? null, durationMs: tool.duration_ms ?? null }
}

/** A tool the design has no card for: its name, what it acted on (its label), and its input. */
function genericTool(tool: ToolContent, name: string, base: ItemFields): ChatEvent {
  const label = (tool.label ?? '').trim()
  return { ...base, kind: 'tool', name, target: label === '' ? null : label, input: inputText(tool.input), detail: tool.output }
}

function codexTool(tool: ToolContent, base: ItemFields): ChatEvent[] {
  const label = tool.label ?? ''
  if (tool.kind === COMMAND_ITEM) return [commandEvent(tool, label, base)]
  if (tool.kind === CODEX_PLAN_ITEM) return [{ ...base, kind: 'todo', items: planItems(tool.output) }]
  if (tool.kind === CODEX_HELPER_ITEM) {
    const input = HelperInputSchema.safeParse(tool.input)
    return [{ ...base, kind: 'subagent', title: label === '' ? null : label, agentType: null, prompt: input.success ? (input.data.prompt ?? null) : null, report: tool.output }]
  }
  if (tool.kind !== FILE_CHANGE_ITEM) return [genericTool(tool, tool.kind, base)]

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
  if (tool.kind !== COMMAND_ITEM) return [genericTool(tool, tool.kind, base)]
  const name = claudeToolName(label)
  // No tool name in the label means the label is the Bash command.
  if (name === null) return [commandEvent(tool, label, base)]

  const mapping = mapClaudeTool(name, tool.input)
  switch (mapping.kind) {
    case 'edit':
      return [{ ...base, kind: 'edit', tool: mapping.tool, file: mapping.file, change: mapping.change, diff: mapping.diff }]
    case 'todo':
      return [{ ...base, kind: 'todo', items: mapping.items }]
    case 'subagent':
      return [{ ...base, kind: 'subagent', title: mapping.title, agentType: mapping.agentType, prompt: mapping.prompt, report: tool.output }]
    case 'tool':
      return [{ ...base, kind: 'tool', name: mapping.name, target: mapping.target, input: mapping.input, detail: tool.output }]
  }
}

function unsupportedItem(provider: ChatProvider, kind: string, base: ItemFields): ChatEvent[] {
  if (provider === 'codex' && kind === CODEX_PLAN_ITEM) return [{ ...base, kind: 'todo', items: null }]
  if (provider === 'codex' && kind === CODEX_HELPER_ITEM) {
    return [{ ...base, kind: 'subagent', title: null, agentType: null, prompt: null, report: '' }]
  }
  return [{ ...base, kind: 'tool', name: kind, target: null, input: null, detail: '' }]
}

/** Maps one history item (not a turn marker) to the events it shows as. Never throws. */
export function projectItem(provider: ChatProvider, content: ItemContent, base: ItemFields): ChatEvent[] {
  if ('Message' in content) {
    const { role, text } = content.Message
    return [{ ...base, kind: 'message', role: role === 'User' ? 'user' : 'agent', text }]
  }
  if ('Tool' in content) {
    if (content.Tool.kind === CONTEXT_SUMMARY_ITEM) return [{ ...base, kind: 'summary', text: content.Tool.output }]
    return provider === 'claude' ? claudeTool(content.Tool, base) : codexTool(content.Tool, base)
  }
  return unsupportedItem(provider, content.Unsupported.kind, base)
}
