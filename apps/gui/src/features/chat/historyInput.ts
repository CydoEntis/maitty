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
  // The daemon does not send a tool's input today. The field is read when it appears so that
  // Claude's Edit, TodoWrite and Task calls can become full cards.
  input: z.unknown().optional(),
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

export function parseHistoryEntry(raw: unknown): HistoryEntry | null {
  const parsed = HistoryEntrySchema.safeParse(raw)
  return parsed.success ? parsed.data : null
}

export function parseApprovalRequest(raw: unknown): ApprovalRequest | null {
  const parsed = ApprovalRequestSchema.safeParse(raw)
  return parsed.success ? parsed.data : null
}
