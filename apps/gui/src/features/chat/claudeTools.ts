import { z } from 'zod'
import type { FileChangeKind, FileDiff, TodoItem, TodoItemState } from './chatEvents'
import { diffFromHunks, replacementHunk, wholeFileDiff } from './unifiedDiff'

// The Claude adapter reports every tool call as a `commandExecution` item whose label is the
// Bash command itself, or "<ToolName> " (the name and a space) for any other tool, because it
// builds the label from the text blocks of a tool's input, which is empty for an object. A label that is a bare tool name
// followed by one space is therefore a non-Bash tool. A Bash command that is exactly a
// capitalised word and a space would be misread; real commands start with a program name.
const NAMED_TOOL_LABEL = /^(mcp__[\w-]+|[A-Z][A-Za-z0-9_]*) $/

const EDIT_TOOL = 'Edit'
const MULTI_EDIT_TOOL = 'MultiEdit'
const WRITE_TOOL = 'Write'
const TODO_TOOL = 'TodoWrite'
// Claude Code calls its helper-agent tool `Task`; newer releases may report `Agent`.
const SUBAGENT_TOOLS: readonly string[] = ['Task', 'Agent']

/** The tool name behind a Claude history label, or `null` when the label is a Bash command. */
export function claudeToolName(label: string): string | null {
  return NAMED_TOOL_LABEL.exec(label)?.[1] ?? null
}

const EditInputSchema = z.object({ file_path: z.string(), old_string: z.string(), new_string: z.string() })
const MultiEditInputSchema = z.object({
  file_path: z.string(),
  edits: z.array(z.object({ old_string: z.string(), new_string: z.string() })),
})
const WriteInputSchema = z.object({ file_path: z.string(), content: z.string() })
const TodoInputSchema = z.object({
  todos: z.array(z.object({ content: z.string(), status: z.enum(['pending', 'in_progress', 'completed']) })),
})
const SubagentInputSchema = z.object({ description: z.string(), prompt: z.string().optional(), subagent_type: z.string().optional() })

const TODO_STATES: Record<'pending' | 'in_progress' | 'completed', TodoItemState> = {
  pending: 'pending',
  in_progress: 'current',
  completed: 'done',
}

export type ClaudeToolMapping =
  | { readonly kind: 'edit'; readonly tool: string; readonly file: string | null; readonly change: FileChangeKind; readonly diff: FileDiff }
  | { readonly kind: 'todo'; readonly items: readonly TodoItem[] | null }
  | { readonly kind: 'subagent'; readonly title: string | null; readonly agentType: string | null; readonly prompt: string | null }
  | { readonly kind: 'tool'; readonly name: string; readonly target: string | null; readonly input: string | null }

const UNAVAILABLE: FileDiff = { status: 'unavailable' }

function mapEdit(tool: string, input: unknown): ClaudeToolMapping {
  const none: ClaudeToolMapping = { kind: 'edit', tool, file: null, change: tool === WRITE_TOOL ? 'unknown' : 'update', diff: UNAVAILABLE }
  if (tool === EDIT_TOOL) {
    const parsed = EditInputSchema.safeParse(input)
    if (!parsed.success) return none
    const { file_path: file, old_string: oldText, new_string: newText } = parsed.data
    return { kind: 'edit', tool, file, change: 'update', diff: diffFromHunks([replacementHunk(oldText, newText)]) }
  }
  if (tool === MULTI_EDIT_TOOL) {
    const parsed = MultiEditInputSchema.safeParse(input)
    if (!parsed.success) return none
    const { file_path: file, edits } = parsed.data
    return {
      kind: 'edit',
      tool,
      file,
      change: 'update',
      diff: diffFromHunks(edits.map((edit) => replacementHunk(edit.old_string, edit.new_string))),
    }
  }
  const parsed = WriteInputSchema.safeParse(input)
  if (!parsed.success) return none
  // Write replaces the whole file and its input does not carry the old text, so an overwrite
  // shows every line as added.
  return { kind: 'edit', tool, file: parsed.data.file_path, change: 'unknown', diff: wholeFileDiff(parsed.data.content, 'add') }
}

/**
 * Chooses the event for one Claude tool call. `input` is the tool's input JSON, which the daemon
 * sends bounded (a history cached by an older daemon has none, so Edit, TodoWrite and Task map to cards with the detail
 * marked unavailable). An unknown tool name is a generic `tool`, never an error.
 */
export function mapClaudeTool(name: string, input: unknown): ClaudeToolMapping {
  if (name === EDIT_TOOL || name === MULTI_EDIT_TOOL || name === WRITE_TOOL) return mapEdit(name, input)
  if (name === TODO_TOOL) {
    const parsed = TodoInputSchema.safeParse(input)
    return {
      kind: 'todo',
      items: parsed.success ? parsed.data.todos.map((todo) => ({ text: todo.content, state: TODO_STATES[todo.status] })) : null,
    }
  }
  if (SUBAGENT_TOOLS.includes(name)) {
    const parsed = SubagentInputSchema.safeParse(input)
    return {
      kind: 'subagent',
      title: parsed.success ? parsed.data.description : null,
      agentType: parsed.success ? (parsed.data.subagent_type ?? null) : null,
      prompt: parsed.success ? (parsed.data.prompt ?? null) : null,
    }
  }
  return { kind: 'tool', name, target: toolTarget(input), input: inputText(input) }
}

// The input fields that name what a tool acted on, most specific first.
const TARGET_KEYS = ['file_path', 'notebook_path', 'path', 'pattern', 'command', 'url', 'query', 'description'] as const
const TARGET_LIMIT = 160
const INPUT_TEXT_LIMIT = 4000
const INPUT_INDENT = 2

/** What a tool call acted on, read from its input; `null` when the input names nothing. */
export function toolTarget(input: unknown): string | null {
  if (typeof input !== 'object' || input === null || Array.isArray(input)) return null
  const fields = input as Record<string, unknown>
  for (const key of TARGET_KEYS) {
    const value = fields[key]
    if (typeof value === 'string' && value.trim() !== '') return value.trim().split('\n')[0]?.slice(0, TARGET_LIMIT) ?? null
  }
  return null
}

/** The tool's input as indented JSON for the expanded chip, cut so one huge input never fills the list. */
export function inputText(input: unknown): string | null {
  if (input === undefined || input === null) return null
  const text = JSON.stringify(input, null, INPUT_INDENT)
  return text.length > INPUT_TEXT_LIMIT ? `${text.slice(0, INPUT_TEXT_LIMIT)}\n[cut]` : text
}
