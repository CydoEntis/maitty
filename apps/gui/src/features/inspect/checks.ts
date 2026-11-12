import { invoke, isTauri } from '@tauri-apps/api/core'
import { z } from 'zod'

const CheckStateSchema = z.enum(['Checking', 'Passed', 'Failed', 'Overridden', 'CouldNotRun', 'Pending', 'NeedsApproval', 'Stale', 'Never'])

/** One of a pane's checks as the daemon reports it (`CheckInfo` in the protocol); absent fields read as their defaults. */
export const CheckSchema = z.object({
  command: z.string().default(''),
  state: CheckStateSchema.default('CouldNotRun'),
  exit_code: z.number().int().nullable().default(null),
  started_at: z.number().default(0),
  duration_ms: z.number().default(0),
  first_failing_line: z.string().default(''),
  rerun_queued: z.boolean().default(false),
  override_reason: z.string().default(''),
  overridden_by: z.string().default(''),
  name: z.string().default(''),
  turn: z.number().default(0),
  line_count: z.number().default(0),
  changed_files: z.array(z.object({ kind: z.string(), path: z.string() })).default([]),
  // `"User"` for a command you set, or `{ Repo: { file, … } }` for one the repository brought.
  source: z.union([z.literal('User'), z.object({ Repo: z.object({ file: z.string(), uncommitted: z.boolean().default(false) }) })]).default('User'),
  previous_command: z.string().nullable().default(null),
})

export type Check = z.infer<typeof CheckSchema>
export type CheckState = z.infer<typeof CheckStateSchema>

const STATE_WORDS: Record<CheckState, string> = {
  Checking: 'Running',
  Passed: 'Passed',
  Failed: 'Failed',
  Overridden: 'Accepted as it is',
  CouldNotRun: 'Could not run',
  Pending: 'Waiting its turn',
  NeedsApproval: 'Needs your approval',
  Stale: 'Passed, then files changed',
  Never: 'Never run here',
}

/** The check's state in words. */
export function checkStateWords(check: Check): string {
  return check.state === 'Checking' && check.rerun_queued ? 'Running (again after this)' : STATE_WORDS[check.state]
}

/** `bad` needs the person, `waiting` is on its way, `good` is fine, `plain` is neither. */
export function checkTone(state: CheckState): 'bad' | 'waiting' | 'good' | 'plain' {
  switch (state) {
    case 'Failed':
    case 'CouldNotRun':
    case 'NeedsApproval':
      return 'bad'
    case 'Checking':
    case 'Pending':
    case 'Stale':
      return 'waiting'
    case 'Passed':
    case 'Overridden':
      return 'good'
    case 'Never':
      return 'plain'
  }
}

/** What the person can do about a check in this state. */
export function checkActions(check: Check): readonly ('rerun' | 'override' | 'undo' | 'approve')[] {
  switch (check.state) {
    case 'Failed':
      return ['rerun', 'override']
    case 'Overridden':
      return ['undo', 'rerun']
    case 'NeedsApproval':
      return ['approve']
    case 'Checking':
    case 'Pending':
    case 'Never':
      return []
    case 'Passed':
    case 'Stale':
    case 'CouldNotRun':
      return ['rerun']
  }
}

/** How many checks want the person: failed, unable to run, or waiting for an approval. */
export function checksNeedingYou(checks: readonly Check[]): number {
  return checks.filter((check) => checkTone(check.state) === 'bad').length
}

export type CheckAction = { action: 'rerun' | 'undo'; name: string } | { action: 'override'; name: string; reason: string } | { action: 'approve'; name: string; choice: 'once' | 'always' | 'never'; command: string }

/** Does something to a pane's check; the daemon (this computer's or the machine's) does it and the next snapshot shows it. */
export async function checkAction(term: number, machine: number, request: CheckAction): Promise<void> {
  if (!isTauri()) throw new Error('Checks are only available in the desktop app')
  await invoke('check_action', {
    machine,
    term,
    action: request.action,
    name: request.name,
    reason: request.action === 'override' ? request.reason : null,
    choice: request.action === 'approve' ? request.choice : null,
    command: request.action === 'approve' ? request.command : null,
  })
}

/** The tail of the check's last output. */
export async function checkOutput(term: number, machine: number, name: string): Promise<string> {
  if (!isTauri()) throw new Error('Checks are only available in the desktop app')
  return z.string().parse(await invoke<unknown>('check_output', { machine, term, name }))
}
