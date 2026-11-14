import '@/features/work/work.css'
import type { WorkflowRun } from '@/data/preview'
import { WorkBoard } from '@/features/work/components/WorkBoard'
import { PaneViewSwitch } from '@/features/workspace/components/PaneViewSwitch'
import type { SubagentView } from '@/features/workspace/model/subagents'
import { toWorkflowView } from '@/features/workspace/model/workflows'

type Step = WorkflowRun['steps'][number]
const step = (name: string, state: Step['state'], note: string | null = null, helper: string | null = null): Step => ({ name, state, note, helper })
const run = (id: string, ticket: WorkflowRun['ticket'], steps: Step[]) => toWorkflowView({ workflow: 'kitchen', run: id, ticket, steps, reportedAt: 1 })
const ticket = (key: string, title: string) => ({ key, title, link: `https://plane.example/${key}` })

/** The brief's cases: a smooth run, one sent back, one needing the person, one with no ticket, and finished ones. */
const RUNS = [
  run('041', ticket('MAITTY-42', 'Add CSV export to the usage page'), [step('intake', 'passed'), step('spec', 'passed'), step('build', 'working', '3 of 5 files', 'MAITTY-42 builder'), step('review', 'waiting'), step('approve', 'waiting')]),
  run('042', ticket('MAITTY-43', 'Let a pane keep its scroll place when a chat reloads its history after a reconnect'), [step('intake', 'passed'), step('spec', 'needs_you', 'approve the spec'), step('build', 'waiting'), step('review', 'waiting'), step('approve', 'waiting')]),
  run('043', ticket('MAITTY-44', 'Fix the stale branch label'), [step('intake', 'passed'), step('spec', 'passed'), step('build', 'sent_back', '2 tests failed', 'MAITTY-44 builder'), step('review', 'failed', 'checks failed'), step('approve', 'waiting')]),
  run('007', null, [step('intake', 'working', null, 'kitchen intake'), step('spec', 'waiting'), step('build', 'waiting'), step('review', 'waiting'), step('approve', 'waiting')]),
  run('038', ticket('MAITTY-39', 'Rename the gear menu'), [step('intake', 'passed'), step('spec', 'passed'), step('build', 'passed'), step('review', 'passed'), step('approve', 'passed')]),
  run('039', ticket('MAITTY-40', 'Drop the old token file'), [step('intake', 'passed'), step('spec', 'skipped'), step('build', 'passed'), step('review', 'passed'), step('approve', 'passed')]),
]

const HELPERS: SubagentView[] = [
  { id: 'a1', name: 'Builder', kind: 'general-purpose', status: 'working', task: 'MAITTY-42 builder', latest: 'Editing src/usage/export.ts', result: null, startedAt: null, updatedAt: 1 },
]

const none = () => undefined

/** The Work tab: the switch with its count and needs square over the board; with `open`, a run's drawer open. */
export function WorkBoards({ open }: { open: boolean }) {
  return (
    <div className="flex w-full flex-col items-start gap-6">
      <PaneViewSwitch views={['chat', 'terminal', 'work']} view="work" work={{ going: 4, needsYou: true }} onChange={none} />
      <div className="relative h-[440px] w-full overflow-hidden rounded-(--r-panel) border border-(--line)">
        <WorkBoard runs={RUNS} helpers={HELPERS} onOpenHelper={none} {...(open ? { startOpen: 'kitchen:041' } : {})} />
      </div>
    </div>
  )
}
