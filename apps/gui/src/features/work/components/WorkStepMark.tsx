import type { WorkflowStepState } from '@/data/preview'
import { StatusMark, type MarkState } from '@/components/ui/status-mark'
import { Spinner } from '@/features/workspace/components/glyphs'
import { STEP_WORDS } from '@/features/workspace/model/workflows'

/** The step states the shared StatusMark draws. */
const MARK: Partial<Record<WorkflowStepState, MarkState>> = { waiting: 'waiting', working: 'running', passed: 'done', failed: 'failed', needs_you: 'needs' }

/** The states it lacks keep a shape of their own, so none reads by colour alone. */
const GLYPH: Partial<Record<WorkflowStepState, string>> = { sent_back: '↺', skipped: '–', unknown: '?' }

/** A workflow step's 14px mark, named by the state's word. */
export function WorkStepMark({ state }: { state: WorkflowStepState }) {
  const mark = MARK[state]
  if (mark !== undefined) return <StatusMark state={mark} spinner={<Spinner />} />
  return <span role="img" aria-label={STEP_WORDS[state]} className={`mx-board-glyph is-${state}`}>{GLYPH[state]}</span>
}
