import type { CommandEvent, EditEvent } from './chatEvents'
import { isOmitted } from './redaction'
import type { WorkStep } from './workSummary'

/**
 * The stack under a folded run of steps: its latest commands and edits as cards, newest in front.
 * Reads, searches and fetches get no card, so a run of only those is the summary row alone.
 */
export type StackStep = CommandEvent | EditEvent
export type StackLine = { readonly mark: string; readonly text: string; readonly tone: 'add' | 'remove' | 'plain' }

export const STACK_MAX_CARDS = 3
const STACK_BODY_LINES = 2

const isStackStep = (step: WorkStep): step is StackStep => step.kind === 'command' || step.kind === 'edit'

/** The cards of the stack, newest first. */
export function stackCards(steps: readonly WorkStep[]): StackStep[] {
  return steps.filter(isStackStep).slice(-STACK_MAX_CARDS).reverse()
}

/** How many of the run's steps the stack leaves out, for the "+N more" pill; 0 while every command and edit has a card. */
export function stepsBeyondStack(steps: readonly WorkStep[]): number {
  return steps.filter(isStackStep).length > STACK_MAX_CARDS ? steps.length - STACK_MAX_CARDS : 0
}

/** What a card shows under its header: the end of a command's output, or the first changed lines of an edit. */
export function stackLines(step: StackStep): StackLine[] {
  if (step.kind === 'command') {
    if (isOmitted(step.output)) return []
    const lines = step.output.split('\n').filter((line) => line.trim() !== '')
    return lines.slice(-STACK_BODY_LINES).map((line) => ({ mark: '', text: line, tone: 'plain' }))
  }
  if (step.diff.status !== 'available') return []
  const lines = step.diff.hunks.flatMap((hunk) => hunk.lines)
  const changed = lines.filter((line) => line.kind !== 'context')
  return (changed.length > 0 ? changed : lines).slice(0, STACK_BODY_LINES).map((line) => ({
    mark: line.kind === 'add' ? '+' : line.kind === 'remove' ? '−' : '',
    text: line.text.trim(),
    tone: line.kind === 'add' ? 'add' : line.kind === 'remove' ? 'remove' : 'plain',
  }))
}
