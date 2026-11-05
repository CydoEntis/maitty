import { z } from 'zod'

/**
 * The daemon's `read_conversation` payload, as far as the chat projection needs it. It is
 * validated here, at the edge of this feature, one entry at a time: an entry this app cannot
 * read must not hide the rest of the conversation.
 */

const ItemStateSchema = z.enum(['Running', 'Completed', 'Failed', 'Interrupted'])

const ToolContentSchema = z.object({
  kind: z.string(),
  output: z.string(),
  label: z.string().optional(),
  // The tool's own input (bounded by the daemon): Claude's Edit, TodoWrite and Task calls become
  // full cards from it. Absent for Codex commands and for history cached by an older daemon.
  input: z.unknown().optional(),
  exit_code: z.number().int().optional(),
  duration_ms: z.number().int().nonnegative().optional(),
})

export const HistoryEntrySchema = z.object({
  source: z.object({ conversation: z.number().int().nonnegative(), turn: z.string(), item: z.string() }),
  content: z.union([
    z.literal('TurnStatus'),
    z.object({ Message: z.object({ role: z.enum(['User', 'Assistant']), text: z.string() }) }),
    z.object({ Tool: ToolContentSchema }),
    z.object({ Unsupported: z.object({ kind: z.string() }) }),
  ]),
  state: ItemStateSchema,
  // Unix milliseconds; absent for items read back from a provider's saved history.
  started_at_ms: z.number().int().nonnegative().optional(),
  completed_at_ms: z.number().int().nonnegative().optional(),
})
export type HistoryEntry = z.infer<typeof HistoryEntrySchema>

const ApprovalChoiceSchema = z.enum(['Accept', 'AcceptForSession', 'Decline', 'Cancel'])

export const ApprovalRequestSchema = z.object({
  request_id: z.union([z.number().int(), z.string()]),
  kind: z.enum(['Command', 'FileChange', 'Permission', 'UserInput']),
  turn_id: z.string().nullable().optional(),
  item_id: z.string().nullable().optional(),
  command: z.string().nullable().optional(),
  reason: z.string().nullable().optional(),
  changes: z.array(z.object({ path: z.string(), kind: z.string(), diff: z.string() })).default([]),
  choices: z.array(ApprovalChoiceSchema).default([]),
})
export type ApprovalRequest = z.infer<typeof ApprovalRequestSchema>

/** An approval a person already answered: the request, the choice, and when the daemon relayed it. */
export const AnsweredApprovalSchema = z.object({
  request: ApprovalRequestSchema,
  decision: ApprovalChoiceSchema,
  answered_at_ms: z.number().int().nonnegative().nullable().optional(),
})
export type AnsweredApproval = z.infer<typeof AnsweredApprovalSchema>

export function parseHistoryEntry(raw: unknown): HistoryEntry | null {
  const parsed = HistoryEntrySchema.safeParse(raw)
  return parsed.success ? parsed.data : null
}

export function parseApprovalRequest(raw: unknown): ApprovalRequest | null {
  const parsed = ApprovalRequestSchema.safeParse(raw)
  return parsed.success ? parsed.data : null
}

export function parseAnsweredApproval(raw: unknown): AnsweredApproval | null {
  const parsed = AnsweredApprovalSchema.safeParse(raw)
  return parsed.success ? parsed.data : null
}
